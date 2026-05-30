//! Retrieval-augmented generation: embedding glue, sidecar chunk/vector loading,
//! rerank orchestration, BM25/cosine/RRF/MMR hybrid search, and rag status.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;
use zotron_rpc::UreqProviderHttpTransport;
use zotron_types::{
    bm25_score_chunks, build_embedding_provider_request, cosine_similarity,
    diversity_filter, execute_embedding_provider_request, gap_cutoff, max_k_truncate,
    parse_embedding_provider_response, read_machine_artifact_sidecar, rrf_merge,
    score_floor_filter, token_budget_filter, ArtifactStorePlatform, EmbeddingChunkInput,
    EmbeddingRequestInput, EmbeddingVector, MachineArtifactKind, StructureChunk,
};

use crate::output::{format_json, normalize_list_envelope};
use crate::rpc::RpcCaller;
use crate::{
    collection_items, embedding_provider_spec, find_collection_in_tree, local_path_from_zotero_path,
    paginate_rpc, resolve_collection, RagCommand, RagSearchOptions,
};

pub(crate) fn run_rag_command(command: RagCommand, client: &mut impl RpcCaller) -> Result<String, String> {
    match command {
        RagCommand::Providers => format_json(
            &serde_json::json!({
                "providers": [
                    embedding_provider_spec("volcengine")?,
                    embedding_provider_spec("alibaba")?,
                    embedding_provider_spec("custom")?,
                ],
            })),
        RagCommand::Embed {
            provider,
            input,
            endpoint,
            model,
            input_type,
            api_key_env,
        } => {
            let value = run_embedding_provider_json_command(
                provider,
                input,
                endpoint,
                model,
                input_type,
                api_key_env,
            )?;
            format_json(&value)
        }
        RagCommand::Status { collection, .. } => {
            let value = rag_status_value(client, &collection)?;
            format_json(&value)
        }
        RagCommand::Search {
            query,
            collection,
            keys,
            zotero,
            top_spans_per_item,
            include_fulltext_spans,
            top_k,
            output,
            ..
        } => run_rag_search_command(
            client,
            RagSearchOptions {
                query,
                collection,
                keys,
                zotero,
                top_spans_per_item,
                include_fulltext_spans,
                top_k,
                output,
            },
        ),
    }
}

pub(crate) fn run_embedding_provider_json_command(
    provider: String,
    input: String,
    endpoint: Option<String>,
    model: Option<String>,
    input_type: Option<String>,
    api_key_env: Option<String>,
) -> Result<Value, String> {
    let mut input: EmbeddingRequestInput = read_json_input(&input)?;
    if endpoint.is_some() {
        input.url = endpoint;
    }
    if model.is_some() {
        input.model = model;
    }
    if input_type.is_some() {
        input.input_type = input_type;
    }
    let mut transport = provider_http_transport(api_key_env.as_deref())?;
    let vectors = execute_embedding_provider_request(&provider, &input, &mut transport)?;

    Ok(serde_json::json!({
        "provider": provider,
        "vectors": vectors,
    }))
}

pub(crate) fn provider_http_transport(api_key_env: Option<&str>) -> Result<UreqProviderHttpTransport, String> {
    provider_http_transport_with_auth(api_key_env, "bearer")
}

pub(crate) fn provider_http_transport_with_auth(
    api_key_env: Option<&str>,
    auth_scheme: &str,
) -> Result<UreqProviderHttpTransport, String> {
    let Some(env_name) = api_key_env else {
        return Ok(UreqProviderHttpTransport::new());
    };
    let token = env::var(env_name)
        .map_err(|_| format!("missing provider credential env var {env_name}"))?;
    if token.trim().is_empty() {
        return Err(format!("provider credential env var {env_name} is empty"));
    }
    let token = token.trim();
    match auth_scheme {
        "token" if token.starts_with("token ") => {
            Ok(UreqProviderHttpTransport::with_api_key(token.to_string()))
        }
        "token" => Ok(UreqProviderHttpTransport::with_api_key(format!(
            "token {token}"
        ))),
        "bearer" if token.starts_with("Bearer ") => {
            Ok(UreqProviderHttpTransport::with_api_key(token.to_string()))
        }
        "bearer" => Ok(UreqProviderHttpTransport::with_bearer_token(token)),
        "none" => Ok(UreqProviderHttpTransport::new()),
        other => Err(format!("unsupported provider auth scheme {other}")),
    }
}

pub(crate) fn read_json_input<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let payload = if path == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|err| format!("read stdin: {err}"))?;
        input
    } else {
        fs::read_to_string(path).map_err(|err| format!("read {path}: {err}"))?
    };
    serde_json::from_str::<T>(&payload)
        .map_err(|err| format!("INVALID_JSON: Could not parse JSON: {err}"))
}

pub(crate) fn fetch_embedding_settings(
    client: &mut impl RpcCaller,
) -> Result<(String, String, String, String), String> {
    let settings = client.call("settings.getAll", None)?;
    let provider = settings
        .get("embedding.provider")
        .and_then(Value::as_str)
        .unwrap_or("ollama")
        .to_string();
    let model = settings
        .get("embedding.model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let api_url = settings
        .get("embedding.apiUrl")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let raw = client.call("settings.getRaw", Some(serde_json::json!({"key": "embedding.apiKey"})))?;
    let api_key = raw
        .get("embedding.apiKey")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok((provider, model, api_url, api_key))
}

#[derive(Debug)]
pub struct RerankSettings {
    pub provider: String,
    pub model: String,
    pub api_url: String,
    pub api_key: String,
    pub candidate_count: usize,
}

pub fn fetch_rerank_settings(
    client: &mut impl RpcCaller,
) -> Result<RerankSettings, String> {
    let settings = client.call("settings.getAll", None)?;
    let provider = settings
        .get("rerank.provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let model = settings
        .get("rerank.model")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let api_url = settings
        .get("rerank.apiUrl")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let candidate_count = settings
        .get("rerank.candidateCount")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    let raw = client.call(
        "settings.getRaw",
        Some(serde_json::json!({"key": "rerank.apiKey"})),
    )?;
    let api_key = raw
        .get("rerank.apiKey")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let specs = zotron_types::builtin_rerank_provider_specs();
    let spec = specs.iter().find(|s| s.id == provider);

    let api_url = if api_url.is_empty() {
        spec.map(|s| s.default_url.to_string()).unwrap_or_default()
    } else {
        api_url
    };

    let model = if model.is_empty() {
        spec.map(|s| s.default_model.to_string()).unwrap_or_default()
    } else {
        model
    };

    Ok(RerankSettings {
        provider,
        model,
        api_url,
        api_key,
        candidate_count,
    })
}

pub(crate) struct RagCutoffSettings {
    min_k: usize,
    max_k: usize,
    token_budget: usize,
    mmr_lambda: f64,
    score_floor: f64,
    gap_threshold: f64,
}

pub(crate) fn fetch_rag_cutoff_settings(client: &mut impl RpcCaller) -> RagCutoffSettings {
    let settings = client.call("settings.getAll", None).unwrap_or_default();
    let get = |key: &str, default: &str| -> String {
        settings
            .get(key)
            .and_then(|v| v.as_str())
            .unwrap_or(default)
            .to_string()
    };
    let legacy_top_k: Option<usize> = settings
        .get("rag.topK")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse().ok());
    let max_k = get("rag.maxK", "")
        .parse()
        .ok()
        .or(legacy_top_k)
        .unwrap_or(20);
    if legacy_top_k.is_some()
        && settings
            .get("rag.maxK")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .is_empty()
    {
        eprintln!("warning: rag.topK is deprecated, use rag.maxK instead");
    }
    RagCutoffSettings {
        min_k: get("rag.minK", "3").parse().unwrap_or(3),
        max_k,
        token_budget: get("rag.tokenBudget", "6000").parse().unwrap_or(6000),
        mmr_lambda: get("rag.mmrLambda", "0.7").parse().unwrap_or(0.7),
        score_floor: get("rerank.scoreFloor", "0.1").parse().unwrap_or(0.1),
        gap_threshold: get("rerank.gapThreshold", "0.15").parse().unwrap_or(0.15),
    }
}

pub(crate) fn rerank_chunks(
    query: &str,
    chunks: &[StructureChunk],
    ranked: &[(usize, f64)],
    settings: &RerankSettings,
) -> Result<Vec<(usize, f64)>, String> {
    let specs = zotron_types::builtin_rerank_provider_specs();
    let spec = specs
        .iter()
        .find(|s| s.id == settings.provider)
        .ok_or_else(|| format!("unknown rerank provider: {}", settings.provider))?;

    let candidate_count = settings.candidate_count.min(ranked.len());
    let candidates: Vec<(usize, f64)> = ranked.iter().take(candidate_count).copied().collect();
    let documents: Vec<&str> = candidates
        .iter()
        .map(|(idx, _)| chunks[*idx].text.as_str())
        .collect();

    let request_body = zotron_types::build_rerank_provider_request(
        &settings.model,
        query,
        &documents,
        candidate_count,
    );

    let body_str = serde_json::to_string(&request_body)
        .map_err(|e| format!("rerank request serialize error: {e}"))?;

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();

    let send = |agent: &ureq::Agent| -> Result<ureq::Response, (bool, String)> {
        agent
            .post(&settings.api_url)
            .set("Content-Type", "application/json")
            .set("Authorization", &format!("Bearer {}", settings.api_key))
            .send_string(&body_str)
            .map_err(|e| {
                let transient = matches!(&e, ureq::Error::Status(code, _) if *code == 429 || *code >= 500);
                (transient, e.to_string())
            })
    };

    let response = match send(&agent) {
        Ok(r) => r,
        Err((true, _)) => {
            std::thread::sleep(Duration::from_secs(1));
            send(&agent).map_err(|(_, msg)| format!("rerank API retry failed: {msg}"))?
        }
        Err((_, msg)) => return Err(format!("rerank API failed: {msg}")),
    };

    let payload: serde_json::Value = response
        .into_json()
        .map_err(|e| format!("rerank response parse error: {e}"))?;

    let reranked = zotron_types::parse_rerank_provider_response(spec, &payload)?;

    Ok(map_reranked_to_candidates(reranked, &candidates))
}

/// Map reranker results back onto the original candidate list.
///
/// The reranker API returns indices into the documents we sent, but a
/// misbehaving (or malicious) provider can return an `index` that is out of
/// range for `candidates`. Bounds-check every result and silently drop any
/// out-of-range entry instead of panicking.
pub(crate) fn map_reranked_to_candidates(
    reranked: Vec<zotron_types::RerankResult>,
    candidates: &[(usize, f64)],
) -> Vec<(usize, f64)> {
    reranked
        .into_iter()
        .filter_map(|r| candidates.get(r.index).map(|c| (c.0, r.score)))
        .collect()
}

pub(crate) fn fetch_retrieval_mode(client: &mut impl RpcCaller) -> String {
    client
        .call(
            "settings.get",
            Some(serde_json::json!({"key": "rag.retrievalMode"})),
        )
        .ok()
        .and_then(|v| {
            v.get("rag.retrievalMode")
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| "hybrid".to_string())
}

pub(crate) fn resolve_sidecar_paths(
    client: &mut impl RpcCaller,
    collection: Option<&str>,
    keys: &[String],
) -> Result<Vec<(String, String, PathBuf)>, String> {
    let items = if !keys.is_empty() {
        let mut items = Vec::new();
        for key in keys {
            let item = client.call("items.get", Some(serde_json::json!({"key": key})))?;
            items.push(item);
        }
        items
    } else if let Some(col) = collection {
        let col_key = resolve_collection(client, col)?;
        let response = client.call(
            "collections.getItems",
            Some(serde_json::json!({"key": col_key})),
        )?;
        collection_items(&response)
    } else {
        return Err("INVALID_ARGS: --collection or --key required".into());
    };

    let mut results = Vec::new();
    for item in &items {
        let item_key = item.get("key").and_then(Value::as_str).unwrap_or_default();
        let attachments = client.call(
            "attachments.list",
            Some(serde_json::json!({"parentKey": item_key})),
        )?;
        let att_list = attachments
            .get("items")
            .and_then(Value::as_array)
            .or_else(|| attachments.as_array())
            .cloned()
            .unwrap_or_default();
        for att in &att_list {
            let content_type = att
                .get("contentType")
                .and_then(Value::as_str)
                .unwrap_or("");
            if content_type != "application/pdf" {
                continue;
            }
            let att_key = att.get("key").and_then(Value::as_str).unwrap_or_default();
            let path = att.get("path").and_then(Value::as_str).unwrap_or_default();
            if path.is_empty() {
                continue;
            }
            let local_path = local_path_from_zotero_path(path);
            let pdf_path = PathBuf::from(&local_path);
            if let Some(parent) = pdf_path.parent() {
                let sidecar_root = parent.join(".zotron");
                if sidecar_root.exists() {
                    results.push((item_key.to_string(), att_key.to_string(), sidecar_root));
                }
            }
        }
    }
    Ok(results)
}

/// True if a sidecar line is the `{"schema_version":N}` header line written by
/// `write_chunks_sidecar`. Detected by PARSING the line and checking for a
/// top-level numeric `schema_version` key — not by substring-matching the
/// token, which would drop a legitimate chunk whose text contains it.
pub(crate) fn is_chunk_schema_header(line: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(line)
        .ok()
        .and_then(|v| v.get("schema_version").map(serde_json::Value::is_number))
        .unwrap_or(false)
}

pub(crate) fn load_sidecar_chunks(sidecar_root: &Path) -> Vec<StructureChunk> {
    let chunks_path =
        sidecar_root.join(MachineArtifactKind::Chunks.sidecar_relative_path());
    let Ok(content) = fs::read_to_string(&chunks_path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| !is_chunk_schema_header(line))
        .filter_map(|line| serde_json::from_str::<StructureChunk>(line).ok())
        .collect()
}

pub(crate) fn embedding_vector_filename(provider: &str, model: &str) -> String {
    let p = provider.trim().to_lowercase().replace('/', "-");
    let m = model.trim().to_lowercase().replace('/', "-");
    if p.is_empty() && m.is_empty() {
        return "vectors.jsonl".to_string();
    }
    format!("{p}--{m}.jsonl")
}

pub(crate) fn load_sidecar_vectors(sidecar_root: &Path, provider: &str, model: &str) -> Vec<EmbeddingVector> {
    let embeddings_dir = sidecar_root.join("embeddings");
    let target = embedding_vector_filename(provider, model);
    let target_path = embeddings_dir.join(&target);
    if let Ok(content) = fs::read_to_string(&target_path) {
        let vecs: Vec<EmbeddingVector> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        if !vecs.is_empty() {
            return vecs;
        }
    }
    // Fallback: try legacy vectors.v1.jsonl / vectors.jsonl with provider match
    for legacy in &["vectors.v1.jsonl", "vectors.jsonl"] {
        let path = embeddings_dir.join(legacy);
        if let Ok(content) = fs::read_to_string(&path) {
            let vecs: Vec<EmbeddingVector> = content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter_map(|line| serde_json::from_str::<EmbeddingVector>(line).ok())
                .filter(|v| v.source_provider == provider || provider.is_empty())
                .collect();
            if !vecs.is_empty() {
                return vecs;
            }
        }
    }
    Vec::new()
}

pub(crate) fn embed_query_text(
    query: &str,
    provider: &str,
    model: &str,
    api_url: &str,
    api_key: &str,
) -> Result<Vec<f64>, String> {
    let input = EmbeddingRequestInput {
        item_key: "query".to_string(),
        chunks: vec![EmbeddingChunkInput {
            chunk_key: "q0".to_string(),
            text: query.to_string(),
        }],
        model: if model.is_empty() {
            None
        } else {
            Some(model.to_string())
        },
        url: if api_url.is_empty() {
            None
        } else {
            Some(api_url.to_string())
        },
        input_type: Some("query".to_string()),
    };
    let request = build_embedding_provider_request(provider, &input)?;
    let url = request
        .url
        .as_deref()
        .ok_or("no embedding URL configured")?;
    let mut http = ureq::post(url).set("Content-Type", "application/json");
    if let Some(auth) = request.auth_header {
        if !api_key.is_empty() {
            http = http.set(auth, &format!("Bearer {api_key}"));
        }
    }
    let resp = http
        .send_json(&request.body)
        .map_err(|e| format!("embedding request failed: {e}"))?;
    let payload: Value = resp
        .into_json()
        .map_err(|e| format!("embedding response parse: {e}"))?;
    let vectors =
        parse_embedding_provider_response(provider, &payload, "query", &input.chunks)?;
    vectors
        .into_iter()
        .next()
        .map(|v| v.vector)
        .ok_or_else(|| "no embedding vector returned".to_string())
}

pub(crate) fn run_rag_search_xpi_fallback(
    client: &mut impl RpcCaller,
    options: &RagSearchOptions,
) -> Result<String, String> {
    let mut params = serde_json::json!({
        "query": options.query,
        "limit": options.top_k,
        "top_spans_per_item": options.top_spans_per_item,
        "include_fulltext_spans": options.include_fulltext_spans,
    });
    if let Some(map) = params.as_object_mut() {
        if let Some(col) = &options.collection {
            map.insert("collection".into(), Value::String(col.clone()));
        }
        if !options.keys.is_empty() {
            map.insert(
                "keys".into(),
                Value::Array(options.keys.iter().map(|k| Value::String(k.clone())).collect()),
            );
        }
    }
    let payload = client.call("rag.searchHits", Some(params))?;
    let hits = payload
        .get("hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if options.output == "jsonl" {
        let mut out = String::new();
        for hit in &hits {
            out.push_str(&serde_json::to_string(hit).map_err(|e| e.to_string())?);
            out.push('\n');
        }
        Ok(out)
    } else {
        let total = hits.len() as u64;
        format_json(
            &normalize_list_envelope(
                serde_json::json!({"items": hits, "total": total}),
                "items",
                Some(options.top_k),
                0,
            ))
    }
}

pub(crate) fn run_rag_search_command(
    client: &mut impl RpcCaller,
    options: RagSearchOptions,
) -> Result<String, String> {
    // When --zotero is explicitly passed, use XPI fallback directly (backward compat).
    if options.zotero {
        if options.collection.is_none() && options.keys.is_empty() {
            return Err(
                "INVALID_ARGS: --collection or --key is required".to_string(),
            );
        }
        return run_rag_search_xpi_fallback(client, &options);
    }

    // Hybrid path: require scope.
    if options.collection.is_none() && options.keys.is_empty() {
        return Err("INVALID_ARGS: --collection or --key required".to_string());
    }

    // Step 1: resolve sidecar paths from collection/keys.
    let sidecars = resolve_sidecar_paths(
        client,
        options.collection.as_deref(),
        &options.keys,
    );

    // If sidecar resolution fails or returns empty, fall back to XPI.
    // But propagate COLLECTION_NOT_FOUND errors directly instead of masking them.
    let sidecars = match sidecars {
        Ok(ref s) if !s.is_empty() => s,
        Err(ref e) if e.contains("COLLECTION_NOT_FOUND") => return Err(e.clone()),
        _ => return run_rag_search_xpi_fallback(client, &options),
    };

    // Step 2: load all chunks and vectors from sidecars.
    let (emb_provider, emb_model, emb_url, emb_key) = fetch_embedding_settings(client)?;
    let mut all_chunks: Vec<StructureChunk> = Vec::new();
    let mut all_vectors: Vec<EmbeddingVector> = Vec::new();
    for (_item_key, _att_key, sidecar_root) in sidecars {
        all_chunks.extend(load_sidecar_chunks(sidecar_root));
        all_vectors.extend(load_sidecar_vectors(sidecar_root, &emb_provider, &emb_model));
    }

    if all_chunks.is_empty() {
        return run_rag_search_xpi_fallback(client, &options);
    }

    // Step 3: determine requested retrieval mode.
    let requested_mode = fetch_retrieval_mode(client);

    // Step 4: BM25 scoring (unless mode is "dense").
    let mut bm25_ranked = if requested_mode != "dense" {
        bm25_score_chunks(&all_chunks, &options.query, 1.2, 0.75)
    } else {
        Vec::new()
    };

    // Step 5: dense vector scoring (unless mode is "lexical" or no vectors).
    let dense_ranked = if requested_mode != "lexical" && !all_vectors.is_empty() {
        match embed_query_text(&options.query, &emb_provider, &emb_model, &emb_url, &emb_key) {
            Ok(query_vec) => {
                let vec_map: std::collections::HashMap<&str, &[f64]> = all_vectors
                    .iter()
                    .map(|v| (v.chunk_key.as_str(), v.vector.as_slice()))
                    .collect();
                let mut scores: Vec<(usize, f64)> = all_chunks
                    .iter()
                    .enumerate()
                    .filter_map(|(i, chunk)| {
                        vec_map.get(chunk.chunk_key.as_str()).map(|stored| {
                            (i, cosine_similarity(&query_vec, stored))
                        })
                    })
                    .filter(|(_, s)| *s > 0.0)
                    .collect();
                scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                scores
            }
            Err(e) => {
                // Query embedding failed: don't silently swallow it.
                eprintln!("warning: dense retrieval unavailable (query embedding failed): {e}");
                Vec::new()
            }
        }
    } else {
        if requested_mode == "dense" && all_vectors.is_empty() {
            eprintln!("warning: dense retrieval requested but no embedding vectors found for this scope");
        }
        Vec::new()
    };

    // Step 6: merge results. Determine the ACTUAL retrieval path used so the
    // output can report it (the requested mode may not be achievable).
    let limit = options.top_k as usize;
    let actual_mode: &str;
    let rrf_ranked = if !bm25_ranked.is_empty() && !dense_ranked.is_empty() {
        actual_mode = "hybrid";
        rrf_merge(&bm25_ranked, &dense_ranked, 60.0, limit)
    } else if !dense_ranked.is_empty() {
        actual_mode = "dense";
        dense_ranked.into_iter().take(limit).collect()
    } else if !bm25_ranked.is_empty() {
        actual_mode = "lexical";
        bm25_ranked.clone().into_iter().take(limit).collect()
    } else {
        // Both BM25 and dense produced nothing. In "dense" mode BM25 was never
        // run, so do a lexical BM25 fallback pass rather than returning a silent
        // empty set — this covers dense-unavailable (no vectors / query-embed
        // failed) AND dense-ran-but-matched-nothing. In lexical/hybrid modes
        // BM25 already ran and was genuinely empty, so we just report the
        // lexical path with no results.
        if requested_mode == "dense" {
            eprintln!("warning: falling back to lexical (BM25) retrieval");
            bm25_ranked = bm25_score_chunks(&all_chunks, &options.query, 1.2, 0.75);
        }
        actual_mode = "lexical";
        bm25_ranked.clone().into_iter().take(limit).collect()
    };

    // --- Rerank + Dynamic Cutoff Pipeline ---

    let rerank_result = fetch_rerank_settings(client);
    let rag_cutoff = fetch_rag_cutoff_settings(client);

    let mut pipeline_ranked = rrf_ranked.clone();
    let mut full_reranked: Option<Vec<(usize, f64)>> = None;

    // Step 6a: Rerank (if configured)
    if let Ok(ref rs) = rerank_result {
        if !rs.provider.is_empty() && !rs.api_key.is_empty() {
            match rerank_chunks(&options.query, &all_chunks, &pipeline_ranked, rs) {
                Ok(reranked) => {
                    full_reranked = Some(reranked.clone());
                    pipeline_ranked = reranked;
                }
                Err(e) => {
                    eprintln!("warning: reranker skipped: {e}");
                }
            }
        }
    }

    // Step 6b: Score floor + Gap cutoff (only with reranker scores)
    if full_reranked.is_some() {
        pipeline_ranked = score_floor_filter(&pipeline_ranked, rag_cutoff.score_floor);
        pipeline_ranked = gap_cutoff(&pipeline_ranked, rag_cutoff.gap_threshold);
    }

    // Origin/scale of each hit's final score, so consumers know the score scale
    // (which varies by path). Reranking, when it succeeds, dominates ordering.
    let score_kind: &str = if full_reranked.is_some() {
        "rerank"
    } else {
        match actual_mode {
            "hybrid" => "rrf",
            "dense" => "cosine",
            _ => "bm25",
        }
    };

    // Step 6c: MMR dedup.
    //
    // The 0.05 MMR threshold assumes relevance in [0,1]. Reranker scores are
    // already sigmoid-normalized to ~[0,1], so they feed MMR directly. Raw RRF
    // (~0.016) / BM25 are NOT in [0,1]; without rescaling the diversity test
    // (lambda*rel - (1-lambda)*sim > threshold) nukes almost every candidate on
    // the no-reranker path and min_k silently re-expands — killing both
    // diversity and result quality. So min-max normalize ONLY the non-reranked
    // path; normalizing the reranked path would stretch a uniformly-high set to
    // [0,1] and make its lowest (still-relevant) member look droppable.
    let mmr_input: Vec<(usize, f64)> = if full_reranked.is_some() {
        pipeline_ranked.clone()
    } else {
        let normalized_rel = zotron_types::min_max_normalize(
            &pipeline_ranked.iter().map(|(_, s)| *s as f32).collect::<Vec<_>>(),
        );
        pipeline_ranked
            .iter()
            .zip(normalized_rel.iter())
            .map(|((idx, _), norm)| (*idx, *norm as f64))
            .collect()
    };
    let chunk_key_index: std::collections::HashMap<&str, usize> = all_chunks
        .iter()
        .enumerate()
        .map(|(i, c)| (c.chunk_key.as_str(), i))
        .collect();
    let vector_map: std::collections::HashMap<usize, &[f64]> = all_vectors
        .iter()
        .filter_map(|v| {
            let &idx = chunk_key_index.get(v.chunk_key.as_str())?;
            Some((idx, v.vector.as_slice()))
        })
        .collect();
    // Diversity filter selects on normalized relevance; map the survivors back
    // to their original scores so downstream stages and hit output keep the true scale.
    let diversity_kept = diversity_filter(
        &mmr_input,
        &vector_map,
        rag_cutoff.mmr_lambda,
        0.05,
    );
    let original_score: std::collections::HashMap<usize, f64> =
        pipeline_ranked.iter().map(|(idx, s)| (*idx, *s)).collect();
    pipeline_ranked = diversity_kept
        .into_iter()
        .map(|(idx, _norm)| (idx, *original_score.get(&idx).unwrap_or(&0.0)))
        .collect();

    // Step 6d: Token budget. Pass character counts (matching the chunker's
    // sizing unit), not UTF-8 byte lengths.
    let char_lens: Vec<usize> = all_chunks.iter().map(|c| c.text.chars().count()).collect();
    pipeline_ranked = token_budget_filter(
        &pipeline_ranked,
        &char_lens,
        rag_cutoff.token_budget,
    );

    // Step 6e: Min/Max K with re-expansion from cached results
    if pipeline_ranked.len() < rag_cutoff.min_k {
        let source = full_reranked.as_deref().unwrap_or(&rrf_ranked);
        for &(idx, score) in source {
            if pipeline_ranked.len() >= rag_cutoff.min_k {
                break;
            }
            if !pipeline_ranked.iter().any(|(i, _)| *i == idx) {
                pipeline_ranked.push((idx, score));
            }
        }
    }
    pipeline_ranked = max_k_truncate(pipeline_ranked, rag_cutoff.max_k);

    let ranked = pipeline_ranked;

    // Step 7: apply per-item span limit.
    let mut per_item_count: std::collections::HashMap<&str, u64> =
        std::collections::HashMap::new();
    let mut selected: Vec<(usize, f64)> = Vec::new();
    for (idx, score) in &ranked {
        let item_key = all_chunks[*idx].item_key.as_str();
        let count = per_item_count.entry(item_key).or_insert(0);
        if *count < options.top_spans_per_item {
            *count += 1;
            selected.push((*idx, *score));
        }
    }

    // Step 8: enrich hits with item metadata.
    let mut meta_cache: std::collections::HashMap<String, Value> =
        std::collections::HashMap::new();
    let mut hits: Vec<Value> = Vec::new();
    for (idx, score) in &selected {
        let chunk = &all_chunks[*idx];
        let meta = if let Some(cached) = meta_cache.get(&chunk.item_key) {
            cached.clone()
        } else {
            let fetched = client
                .call(
                    "items.get",
                    Some(serde_json::json!({"key": chunk.item_key})),
                )
                .unwrap_or(Value::Null);
            meta_cache.insert(chunk.item_key.clone(), fetched.clone());
            fetched
        };
        let title = meta
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let authors = meta
            .get("creators")
            .and_then(Value::as_array)
            .map(|creators| {
                creators
                    .iter()
                    .filter_map(|c| {
                        let last = c.get("lastName").and_then(Value::as_str).unwrap_or("");
                        let first = c.get("firstName").and_then(Value::as_str).unwrap_or("");
                        if last.is_empty() && first.is_empty() {
                            None
                        } else {
                            Some(format!("{last}{first}"))
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default();
        let year = meta.get("date").and_then(Value::as_str).unwrap_or("");
        let mut hit = serde_json::json!({
            "item_key": chunk.item_key,
            "chunk_key": chunk.chunk_key,
            "title": title,
            "authors": authors,
            "year": year,
            "text": chunk.text,
            "page_range": chunk.page_range,
            "section_path": chunk.section_path,
            "score": score,
            "score_kind": score_kind,
        });
        if options.include_fulltext_spans {
            hit.as_object_mut().unwrap().insert(
                "attachment_key".to_string(),
                Value::String(chunk.attachment_key.clone()),
            );
        }
        hits.push(hit);
    }

    // Step 9: format output. `mode` reports the ACTUAL retrieval path used
    // (hybrid / dense / lexical), which may differ from the requested mode when
    // dense vectors or query embeddings were unavailable.
    if options.output == "jsonl" {
        let mut out = String::new();
        for hit in &hits {
            out.push_str(&serde_json::to_string(hit).map_err(|e| e.to_string())?);
            out.push('\n');
        }
        Ok(out)
    } else {
        let total = hits.len() as u64;
        format_json(
            &normalize_list_envelope(
                serde_json::json!({"items": hits, "total": total, "mode": actual_mode}),
                "items",
                Some(options.top_k),
                0,
            ))
    }
}

pub(crate) fn rag_status_value(client: &mut impl RpcCaller, collection: &str) -> Result<Value, String> {
    let raw_store_path = rag_store_path(collection);
    if raw_store_path.exists() {
        return rag_status_from_store(collection, &raw_store_path);
    }

    let mut store_candidates = Vec::new();
    let collection_match = find_collection_in_tree(client, collection)?;
    if let Some(collection_node) = collection_match.as_ref() {
        if let Some(name) = collection_node.get("name").and_then(Value::as_str) {
            store_candidates.push(rag_store_path(name));
        }
        if let Some(key) = collection_node.get("key").and_then(Value::as_str) {
            store_candidates.push(rag_store_path(key));
        }
    }
    for store_path in unique_paths(store_candidates) {
        if store_path.exists() {
            return rag_status_from_store(collection, &store_path);
        }
    }

    rag_status_from_zotero_sidecars(client, collection, collection_match)
}

pub(crate) fn unique_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.iter().any(|seen| seen == &path) {
            unique.push(path);
        }
    }
    unique
}

pub(crate) fn rag_status_from_store(collection: &str, store_path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(store_path)
        .map_err(|err| format!("read RAG store {}: {err}", store_path.display()))?;
    let store: Value = serde_json::from_str(&raw)
        .map_err(|err| format!("parse RAG store {}: {err}", store_path.display()))?;
    let chunks = store
        .get("chunks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut item_keys = Vec::<Value>::new();
    for chunk in &chunks {
        let Some(item_key) = chunk.get("item_key") else {
            continue;
        };
        if !item_keys.iter().any(|seen| seen == item_key) {
            item_keys.push(item_key.clone());
        }
    }
    Ok(serde_json::json!({
        "status": "indexed",
        "collection": store.get("collection").and_then(Value::as_str).unwrap_or(collection),
        "collection_key": store.get("collection_key").cloned().unwrap_or(Value::Null),
        "model": store.get("model").cloned().unwrap_or(Value::String("unknown".to_string())),
        "total_chunks": chunks.len(),
        "total_items": item_keys.len(),
        "store_path": store_path.to_string_lossy(),
    }))
}

pub(crate) fn rag_status_from_zotero_sidecars(
    client: &mut impl RpcCaller,
    collection: &str,
    collection_match: Option<Value>,
) -> Result<Value, String> {
    let collection_key = collection_match
        .as_ref()
        .and_then(|node| node.get("key").cloned())
        .ok_or_else(|| format!("COLLECTION_NOT_FOUND: Collection not found: {collection:?}"))?;
    let raw = paginate_rpc(
        client,
        "collections.getItems",
        serde_json::json!({"key": collection_key}),
        500,
    )?;
    let items = raw
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| raw.as_array())
        .ok_or_else(|| "collections.getItems returned non-array/non-items result".to_string())?
        .clone();

    // Embedding provider/model decide which vector file counts as "available".
    // Only fetched when there is at least one item to inspect (avoids an RPC
    // round-trip when the collection is empty / not indexed). A failure (no
    // settings) just yields empty provider/model and a zero vector count.
    let (emb_provider, emb_model) = if items.is_empty() {
        (String::new(), String::new())
    } else {
        fetch_embedding_settings(client)
            .map(|(p, m, _, _)| (p, m))
            .unwrap_or_default()
    };

    let mut indexed_items = 0usize;
    let mut total_chunks = 0usize;
    let mut total_vectors = 0usize;
    for item in &items {
        let item_key = item.get("key").cloned().unwrap_or(Value::Null);
        // One attachments.list call yields both chunk and vector counts.
        let (chunk_count, vector_count) =
            sidecar_counts_for_item(client, &item_key, &emb_provider, &emb_model)?;
        if chunk_count > 0 {
            indexed_items += 1;
            total_chunks += chunk_count;
            total_vectors += vector_count;
        }
    }

    if indexed_items == 0 {
        return Ok(serde_json::json!({
            "status": "not indexed",
            "collection": collection,
            "total_items": items.len(),
            "indexed_items": 0,
        }));
    }

    Ok(serde_json::json!({
        "status": "indexed",
        "collection": collection,
        "total_chunks": total_chunks,
        "total_items": indexed_items,
        "collection_items": items.len(),
        // Whether semantic (dense) retrieval is actually available for this
        // scope+provider — lets a user tell before searching whether they'll get
        // hybrid retrieval or just lexical BM25.
        "total_vectors": total_vectors,
        "embeddings_available": total_vectors > 0,
        "embedding_provider": emb_provider,
        "embedding_model": emb_model,
        "source": "zotero-sidecar",
    }))
}

/// Count both chunk lines and stored embedding vectors for an item, using a
/// single `attachments.list` call. Returns `(chunks, vectors)`.
pub(crate) fn sidecar_counts_for_item(
    client: &mut impl RpcCaller,
    item_key: &Value,
    emb_provider: &str,
    emb_model: &str,
) -> Result<(usize, usize), String> {
    let attachments = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": item_key.clone()})),
    )?;
    let Some(attachments) = attachments.as_array() else {
        return Ok((0, 0));
    };

    let mut chunk_count = 0usize;
    let mut vector_count = 0usize;
    for attachment in attachments {
        let Some(path) = attachment.get("path").and_then(Value::as_str) else {
            continue;
        };
        let local = local_path_from_zotero_path(path);
        let Some(dir) = Path::new(&local).parent() else {
            continue;
        };
        if let Ok(bytes) = read_machine_artifact_sidecar(dir, MachineArtifactKind::Chunks) {
            let text = String::from_utf8_lossy(&bytes);
            // Subtract the {"schema_version":N} header line if present.
            chunk_count += text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .filter(|line| !is_chunk_schema_header(line))
                .count();
        }
        let sidecar_root = dir.join(".zotron");
        vector_count += load_sidecar_vectors(&sidecar_root, emb_provider, emb_model).len();
    }
    Ok((chunk_count, vector_count))
}

pub(crate) fn rag_store_path(collection: &str) -> PathBuf {
    rag_store_root().join(format!("{collection}.json"))
}

pub(crate) fn rag_store_root() -> PathBuf {
    let xdg_data_home = env::var_os("XDG_DATA_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let appdata = env::var_os("APPDATA")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let userprofile = env::var_os("USERPROFILE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
    let home = env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);

    rag_store_root_for_platform(
        ArtifactStorePlatform::current(),
        xdg_data_home.as_deref(),
        appdata.as_deref(),
        userprofile.as_deref(),
        home.as_deref(),
    )
}

pub(crate) fn rag_store_root_for_platform(
    platform: ArtifactStorePlatform,
    xdg_data_home: Option<&Path>,
    appdata: Option<&Path>,
    userprofile: Option<&Path>,
    home: Option<&Path>,
) -> PathBuf {
    match platform {
        ArtifactStorePlatform::Windows => {
            if let Some(path) = appdata {
                return path.join("Zotron").join("rag");
            }
            if let Some(path) = userprofile {
                return path
                    .join("AppData")
                    .join("Roaming")
                    .join("Zotron")
                    .join("rag");
            }
            if let Some(path) = home {
                return path
                    .join("AppData")
                    .join("Roaming")
                    .join("Zotron")
                    .join("rag");
            }
            PathBuf::from(".zotron").join("rag")
        }
        ArtifactStorePlatform::Macos => {
            if let Some(path) = home {
                return path
                    .join("Library")
                    .join("Application Support")
                    .join("Zotron")
                    .join("rag");
            }
            if let Some(path) = xdg_data_home {
                return path.join("zotron").join("rag");
            }
            PathBuf::from(".zotron").join("rag")
        }
        ArtifactStorePlatform::Linux | ArtifactStorePlatform::Other => xdg_data_home
            .map(|path| path.join("zotron").join("rag"))
            .or_else(|| {
                home.map(|path| path.join(".local").join("share").join("zotron").join("rag"))
            })
            .unwrap_or_else(|| PathBuf::from(".zotron").join("rag")),
    }
}

#[cfg(test)]
mod rerank_bounds_tests {
    use super::map_reranked_to_candidates;
    use zotron_types::RerankResult;

    #[test]
    fn drops_out_of_range_indices_without_panicking() {
        // candidates[i].0 is the original chunk index; .1 the prior score.
        let candidates = vec![(10_usize, 0.1_f64), (20, 0.2), (30, 0.3)];
        let reranked = vec![
            RerankResult { index: 1, score: 0.9 },   // in range -> maps to candidates[1].0 == 20
            RerankResult { index: 5, score: 0.8 },   // out of range -> dropped
            RerankResult { index: 0, score: 0.7 },   // in range -> maps to candidates[0].0 == 10
        ];

        let mapped = map_reranked_to_candidates(reranked, &candidates);

        // (b) the out-of-range entry was dropped
        assert_eq!(mapped.len(), 2);
        // (c) in-range entries map to the correct candidates[i].0 value and score
        assert_eq!(mapped[0], (20, 0.9));
        assert_eq!(mapped[1], (10, 0.7));
    }

    #[test]
    fn all_out_of_range_yields_empty() {
        let candidates = vec![(10_usize, 0.1_f64)];
        let reranked = vec![
            RerankResult { index: 1, score: 0.9 },
            RerankResult { index: 99, score: 0.8 },
        ];
        let mapped = map_reranked_to_candidates(reranked, &candidates);
        assert!(mapped.is_empty());
    }
}

#[cfg(test)]
mod sidecar_header_tests {
    use super::is_chunk_schema_header;

    #[test]
    fn detects_schema_version_header_line() {
        assert!(is_chunk_schema_header("{\"schema_version\":2}"));
        assert!(is_chunk_schema_header("{\"schema_version\": 1}"));
    }

    #[test]
    fn does_not_flag_a_chunk_whose_text_is_the_token() {
        // Regression: the old substring filter dropped any line containing the
        // quoted token "schema_version" — including a legitimate chunk whose
        // text value is exactly that. Parsing for a top-level key avoids this.
        let chunk_line = "{\"chunk_key\":\"ATT1:c0\",\"item_key\":\"ITEM1\",\"attachment_key\":\"ATT1\",\"block_keys\":[],\"section_path\":[],\"text\":\"schema_version\",\"page_range\":[0,0],\"evidence_refs\":[]}";
        assert!(!is_chunk_schema_header(chunk_line));
    }

    #[test]
    fn does_not_flag_a_plain_chunk_line() {
        assert!(!is_chunk_schema_header("{\"chunk_key\":\"x\",\"text\":\"hello world\"}"));
    }
}
