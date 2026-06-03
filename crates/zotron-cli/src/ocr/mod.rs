//! OCR pipeline: provider dispatch (GLM/PaddleOCR/MinerU), the MinerU cloud and
//! CLI ingestion paths, sidecar writers, chunk auto-embedding glue, and OCR
//! status reporting.

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;
use zotron_rpc::{StdProviderCommandRunner, UreqProviderHttpTransport};
use zotron_types::{
    build_embedding_provider_request, build_ocr_provider_request,
    machine_artifact_exists_for_item, machine_artifact_exists_in_sidecar,
    machine_artifact_store_root, ocr_provider_spec as raw_ocr_provider_spec,
    parse_embedding_provider_response, parse_ocr_provider_response, write_machine_artifact_sidecar,
    EmbeddingChunkInput, EmbeddingRequestInput, EmbeddingVector, MachineArtifactKind,
    OcrRequestInput, ProviderCommandRunner, ProviderHttpInvocation, ProviderHttpTransport,
};

use crate::output::format_json;
use crate::rag::{
    embedding_vector_filename, fetch_embedding_settings, provider_http_transport_with_auth,
    read_json_input, resolve_sidecar_paths,
};
use crate::rpc::RpcCaller;
use crate::{
    find_collection_in_tree, is_pdf_attachment, local_path_from_zotero_path, ocr_provider_specs,
    paginate_rpc, OcrCommand,
};

mod mineru;
mod sidecar;
pub(crate) use mineru::*;
pub(crate) use sidecar::*;

pub(crate) fn run_ocr_command(command: OcrCommand, client: &mut impl RpcCaller) -> Result<String, String> {
    if let OcrCommand::Providers = &command {
        return format_json(&serde_json::json!({ "providers": ocr_provider_specs() }));
    }
    let value = match command {
        OcrCommand::Providers => unreachable!(),
        OcrCommand::Run {
            provider,
            input,
            file,
            item_key,
            attachment_key,
            mime_type,
            endpoint,
            api_key_env,
        } => run_ocr_run_command(OcrRunOptions {
            provider,
            input,
            file,
            item_key,
            attachment_key,
            mime_type,
            endpoint,
            api_key_env,
        })?,
        OcrCommand::Status { collection, .. } => run_ocr_status_command(client, collection)?,
        OcrCommand::Reindex { collection, key, stale_only, chunk_chars, .. } => {
            return run_ocr_reindex_command(client, collection, key, stale_only, chunk_chars);
        }
        OcrCommand::Process {
            provider,
            parent,
            collection,
            attachment,
            source_url,
            result_dir,
            result_zip,
            provider_endpoint,
            api_key_env,
            poll_interval_seconds,
            timeout_seconds,
            chunk_chars,
            ..
        } => {
            // Validate the target selection before any RPC so pure arg errors
            // short-circuit. --collection batch mode OCRs every item in the
            // collection; --result-dir/--result-zip are single-item replay
            // inputs and cannot fan out across a collection.
            if collection.is_some() && (result_dir.is_some() || result_zip.is_some()) {
                return Err("INVALID_ARGS: --collection cannot be combined with --result-dir/--result-zip".to_string());
            }
            if collection.is_none() && parent.is_none() {
                return Err(
                    "INVALID_ARGS: provide --parent <itemKey> or --collection <name>".to_string(),
                );
            }

            let resolved_provider = match provider {
                Some(p) => p,
                None => fetch_ocr_provider_from_settings(client)?,
            };
            let resolved_env = api_key_env.unwrap_or_else(|| "ZOTRON_OCR_API_KEY".to_string());
            let needs_auth = result_dir.is_none() && result_zip.is_none();
            if needs_auth && env::var(&resolved_env).ok().filter(|v| !v.is_empty()).is_none() {
                let key = fetch_ocr_api_key_from_settings(client);
                if !key.is_empty() {
                    unsafe { env::set_var(&resolved_env, &key); }
                }
            }

            if let Some(collection) = collection {
                return run_ocr_process_collection(
                    client,
                    &collection,
                    OcrProcessBatchOptions {
                        provider: resolved_provider,
                        source_url,
                        provider_endpoint,
                        api_key_env: resolved_env,
                        poll_interval_seconds,
                        timeout_seconds,
                        chunk_chars,
                    },
                );
            }

            let parent = parent.expect("parent presence validated above");

            run_ocr_process_command(
                client,
                OcrProcessOptions {
                    provider: resolved_provider,
                    parent,
                    attachment,
                    source_url,
                    result_dir,
                    result_zip,
                    provider_endpoint,
                    api_key_env: resolved_env,
                    poll_interval_seconds,
                    timeout_seconds,
                    chunk_chars,
                },
            )?
        }
    };
    format_json(&value)
}

pub(crate) struct OcrProcessOptions {
    provider: String,
    parent: String,
    attachment: Option<String>,
    source_url: Option<String>,
    result_dir: Option<String>,
    result_zip: Option<String>,
    provider_endpoint: Option<String>,
    api_key_env: String,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
    chunk_chars: usize,
}

pub(crate) struct OcrRunOptions {
    provider: String,
    input: Option<String>,
    file: Option<String>,
    item_key: Option<String>,
    attachment_key: Option<String>,
    mime_type: Option<String>,
    endpoint: Option<String>,
    api_key_env: Option<String>,
}

/// Per-item OCR options shared across a `--collection` batch run. Each item's
/// PDF attachment is auto-resolved, so `attachment`/`result_dir`/`result_zip`
/// (single-item inputs) are intentionally absent.
pub(crate) struct OcrProcessBatchOptions {
    provider: String,
    source_url: Option<String>,
    provider_endpoint: Option<String>,
    api_key_env: String,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
    chunk_chars: usize,
}

/// Resolve a collection (by name or key) and OCR every item in it, skipping
/// items that have no PDF attachment. Mirrors `ocr status` / `ocr reindex`
/// collection resolution and iterates one item at a time.
pub(crate) fn run_ocr_process_collection(
    client: &mut impl RpcCaller,
    collection: &str,
    options: OcrProcessBatchOptions,
) -> Result<String, String> {
    let collection_key = find_collection_in_tree(client, collection)?
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

    let mut processed: Vec<Value> = Vec::new();
    let mut skipped = 0usize;
    let mut failed = 0usize;
    let mut results: Vec<Value> = Vec::new();

    for item in &items {
        let Some(item_key) = item.get("key").and_then(Value::as_str) else {
            continue;
        };

        match run_ocr_process_command(
            client,
            OcrProcessOptions {
                provider: options.provider.clone(),
                parent: item_key.to_string(),
                attachment: None,
                source_url: options.source_url.clone(),
                result_dir: None,
                result_zip: None,
                provider_endpoint: options.provider_endpoint.clone(),
                api_key_env: options.api_key_env.clone(),
                poll_interval_seconds: options.poll_interval_seconds,
                timeout_seconds: options.timeout_seconds,
                chunk_chars: options.chunk_chars,
            },
        ) {
            Ok(value) => {
                processed.push(Value::String(item_key.to_string()));
                results.push(serde_json::json!({
                    "parent": item_key,
                    "status": "ok",
                    "result": value,
                }));
            }
            // Items with no PDF attachment are expected in mixed collections;
            // skip them instead of aborting the whole batch.
            Err(err) if err.starts_with("NO_PDF_ATTACHMENT") => {
                skipped += 1;
                results.push(serde_json::json!({
                    "parent": item_key,
                    "status": "skipped",
                    "error": err,
                }));
            }
            Err(err) => {
                failed += 1;
                results.push(serde_json::json!({
                    "parent": item_key,
                    "status": "error",
                    "error": err,
                }));
            }
        }
    }

    format_json(&serde_json::json!({
        "collection": collection,
        "total": items.len(),
        "processed": processed.len(),
        "skipped": skipped,
        "failed": failed,
        "items": results,
    }))
}

pub(crate) fn run_ocr_run_command(options: OcrRunOptions) -> Result<Value, String> {
    let input: OcrRequestInput = match (options.input, options.file) {
        (Some(input), None) => read_json_input(&input)?,
        (None, Some(file)) => ocr_input_from_file(
            file,
            options.item_key,
            options.attachment_key,
            options.mime_type,
        )?,
        (Some(_), Some(_)) => {
            return Err("INVALID_ARGS: use either --input or --file, not both".to_string())
        }
        (None, None) => return Err("INVALID_ARGS: provide --input JSON or --file".to_string()),
    };
    let request = build_ocr_provider_request(&options.provider, &input)?;
    let payload = if request.command.is_empty() {
        let method = request
            .method
            .ok_or_else(|| format!("OCR provider {} missing HTTP method", request.provider))?;
        let auth_scheme = raw_ocr_provider_spec(&options.provider)?.auth;
        let mut transport =
            provider_http_transport_with_auth(options.api_key_env.as_deref(), auth_scheme)?;
        transport.post_json(&ProviderHttpInvocation {
            provider: request.provider.to_string(),
            style: request.style.to_string(),
            method: method.to_string(),
            url: options
                .endpoint
                .or_else(|| request.url.map(ToString::to_string)),
            auth_header_name: request.auth_header.map(ToString::to_string),
            auth_header_value: None,
            body: request.body,
        })?
    } else {
        let mut command_runner = StdProviderCommandRunner;
        command_runner.run_json(&request.command)?
    };
    let blocks = match parse_ocr_provider_response(
        request.provider,
        &payload,
        &input.item_key,
        &input.attachment_key,
    ) {
        Ok(blocks) => blocks,
        Err(err) => {
            if let Some(task) = ocr_async_task_result(request.provider, &payload) {
                return Ok(task);
            }
            return Err(err);
        }
    };

    Ok(serde_json::json!({
        "provider": request.provider,
        "blocks": blocks,
    }))
}

pub(crate) fn fetch_ocr_provider_from_settings(client: &mut impl RpcCaller) -> Result<String, String> {
    let settings = client.call("settings.getAll", None)?;
    let provider = settings
        .get("ocr.provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if provider.is_empty() {
        return Err("MISSING_CONFIG: ocr.provider not configured — set it in Zotero → Settings → Zotron → OCR Settings".to_string());
    }
    Ok(provider)
}

pub(crate) fn fetch_ocr_api_key_from_settings(client: &mut impl RpcCaller) -> String {
    client
        .call("settings.getRaw", Some(serde_json::json!({"key": "ocr.apiKey"})))
        .ok()
        .and_then(|raw| raw.get("ocr.apiKey").and_then(Value::as_str).map(String::from))
        .unwrap_or_default()
}

pub(crate) fn run_ocr_process_command(
    client: &mut impl RpcCaller,
    mut options: OcrProcessOptions,
) -> Result<Value, String> {
    let spec = raw_ocr_provider_spec(&options.provider)?;

    let attachment = match options.attachment.take() {
        Some(key) => key,
        None => resolve_first_pdf_attachment_key(client, &options.parent)?,
    };
    options.attachment = Some(attachment.clone());

    let attachment_path = resolve_attachment_path(client, &attachment)?;
    let storage_dir = attachment_path
        .parent()
        .ok_or_else(|| {
            format!(
                "ATTACHMENT_PATH_INVALID: attachment path has no parent directory: {}",
                attachment_path.display()
            )
        })?
        .to_path_buf();

    match spec.provider_key {
        "mineru" | "mineru-cli" => {
            if options.result_dir.is_some() && options.result_zip.is_some() {
                return Err("INVALID_ARGS: use either --result-dir or --result-zip, not both".to_string());
            }
            if options.source_url.is_some()
                && (options.result_dir.is_some() || options.result_zip.is_some())
            {
                return Err(
                    "INVALID_ARGS: --source-url cannot be combined with --result-dir/--result-zip"
                        .to_string(),
                );
            }
            let file_name = attachment_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("document.pdf")
                .to_string();
            let source = load_mineru_result_source(&options, &attachment_path, &file_name)?;
            let artifacts = persist_mineru_result_sidecars(
                &storage_dir, &options.parent, &attachment,
                &options.provider, &source, options.chunk_chars,
            )?;
            let embedding_count = embed_sidecar_chunks(
                client, &storage_dir, &options.parent, &attachment, &artifacts.chunks,
            );
            Ok(serde_json::json!({
                "provider": spec.provider_key,
                "status": "indexed",
                "itemKey": options.parent,
                "attachmentKey": attachment,
                "embeddings": embedding_count,
                "attachmentPath": attachment_path,
                "storageDir": storage_dir,
                "taskId": source.task_id,
                "state": source.state,
                "blocks": artifacts.block_count,
                "chunks": artifacts.chunk_count,
                "artifacts": artifacts.artifacts,
            }))
        }
        _ => {
            run_ocr_process_sync(
                client, &options, spec.provider_key,
                &attachment, &attachment_path, &storage_dir,
            )
        }
    }
}

pub(crate) fn run_ocr_process_sync(
    client: &mut impl RpcCaller,
    options: &OcrProcessOptions,
    provider: &str,
    attachment_key: &str,
    attachment_path: &Path,
    storage_dir: &Path,
) -> Result<Value, String> {
    let api_url = if let Some(endpoint) = &options.provider_endpoint {
        endpoint.clone()
    } else {
        let settings = client.call("settings.getAll", None)?;
        settings.get("ocr.apiUrl")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    if api_url.is_empty() {
        return Err(format!("MISSING_CONFIG: ocr.apiUrl not configured for provider {provider}"));
    }

    let api_key = {
        let from_env = if !options.api_key_env.is_empty() {
            env::var(&options.api_key_env).ok().filter(|v| !v.is_empty())
        } else {
            None
        };
        from_env.unwrap_or_else(|| {
            client.call("settings.getRaw", Some(serde_json::json!({"key": "ocr.apiKey"})))
                .ok()
                .and_then(|raw| raw.get("ocr.apiKey").and_then(Value::as_str).map(String::from))
                .unwrap_or_default()
        })
    };

    let pdf_bytes = fs::read(attachment_path)
        .map_err(|e| format!("READ_PDF_FAILED: {}: {e}", attachment_path.display()))?;

    const MAX_PDF_SIZE: usize = 100 * 1024 * 1024; // 100 MB
    if pdf_bytes.len() > MAX_PDF_SIZE {
        return Err(format!(
            "PDF_TOO_LARGE: {} is {} MB, max {} MB",
            attachment_path.display(),
            pdf_bytes.len() / (1024 * 1024),
            MAX_PDF_SIZE / (1024 * 1024),
        ));
    }

    let base64_pdf = format!("data:application/pdf;base64,{}", base64_encode(&pdf_bytes));

    let input = OcrRequestInput {
        content_base64: base64_pdf,
        file_name: attachment_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string(),
        mime_type: "application/pdf".to_string(),
        item_key: options.parent.clone(),
        attachment_key: attachment_key.to_string(),
        source_url: None,
        local_path: Some(attachment_path.to_string_lossy().to_string()),
        output_dir: None,
    };
    let request = build_ocr_provider_request(provider, &input)?;

    let payload = if request.command.is_empty() {
        let method = request
            .method
            .ok_or_else(|| format!("OCR provider {provider} missing HTTP method"))?;
        let spec = raw_ocr_provider_spec(provider)?;

        let mut transport = if !api_key.is_empty() {
            match spec.auth {
                "bearer" => UreqProviderHttpTransport::with_bearer_token(&api_key),
                "token" => UreqProviderHttpTransport::with_api_key(format!("token {api_key}")),
                _ => UreqProviderHttpTransport::new(),
            }
        } else {
            UreqProviderHttpTransport::new()
        };

        transport.post_json(&ProviderHttpInvocation {
            provider: request.provider.to_string(),
            style: request.style.to_string(),
            method: method.to_string(),
            url: Some(api_url),
            auth_header_name: request.auth_header.map(ToString::to_string),
            auth_header_value: None,
            body: request.body,
        })?
    } else {
        let mut runner = StdProviderCommandRunner;
        runner.run_json(&request.command)?
    };

    let blocks = parse_ocr_provider_response(provider, &payload, &options.parent, attachment_key)?;
    let chunks = zotron_types::chunks_from_blocks(&blocks, options.chunk_chars);

    let mut artifacts = vec![
        write_sidecar_json(
            storage_dir, &options.parent, attachment_key,
            MachineArtifactKind::OcrRaw, &payload,
        )?,
        write_sidecar_jsonl(
            storage_dir, &options.parent, attachment_key,
            MachineArtifactKind::Blocks, &blocks,
        )?,
        write_chunks_sidecar(
            storage_dir, &options.parent, attachment_key, &chunks,
        )?,
    ];
    if let Some(markdown) = zotron_types::provider_native_markdown(&payload) {
        artifacts.push(write_sidecar_bytes(
            storage_dir, &options.parent, attachment_key,
            MachineArtifactKind::OcrNativeMarkdown, markdown.as_bytes(),
        )?);
    }

    let embedding_count = embed_sidecar_chunks(client, storage_dir, &options.parent, attachment_key, &chunks);

    Ok(serde_json::json!({
        "provider": provider,
        "status": "indexed",
        "itemKey": options.parent,
        "attachmentKey": attachment_key,
        "embeddings": embedding_count,
        "attachmentPath": attachment_path,
        "storageDir": storage_dir,
        "blocks": blocks.len(),
        "chunks": chunks.len(),
        "artifacts": artifacts,
    }))
}

/// Schema version written as the first line of chunks.v1.jsonl by reindex.
pub(crate) const CHUNK_SCHEMA_VERSION: u32 = 2;

pub(crate) fn run_ocr_reindex_command(
    client: &mut impl RpcCaller,
    collection: Option<String>,
    key: Option<String>,
    stale_only: bool,
    chunk_chars: usize,
) -> Result<String, String> {
    // Resolve sidecar paths using the same logic as RAG search.
    let keys: Vec<String> = key.into_iter().collect();
    let sidecars = resolve_sidecar_paths(
        client,
        collection.as_deref(),
        &keys,
    )?;

    if sidecars.is_empty() {
        return format_json(
            &serde_json::json!({
                "reindexed": 0,
                "skipped": 0,
                "message": "no sidecars found"
            }));
    }

    let mut reindexed: Vec<Value> = Vec::new();
    let mut skipped = 0usize;

    for (item_key, att_key, sidecar_root) in &sidecars {
        // sidecar_root is storage_dir/.zotron
        // storage_dir is sidecar_root's parent
        let storage_dir = match sidecar_root.parent() {
            Some(p) => p,
            None => {
                skipped += 1;
                continue;
            }
        };

        let chunks_path = sidecar_root.join(zotron_types::MachineArtifactKind::Chunks.sidecar_relative_path());
        if stale_only {
            if let Ok(f) = fs::File::open(&chunks_path) {
                use std::io::BufRead;
                let mut reader = std::io::BufReader::new(f);
                let mut first_line = String::new();
                if reader.read_line(&mut first_line).is_ok() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&first_line) {
                        if v.get("schema_version").and_then(|v| v.as_u64()) == Some(CHUNK_SCHEMA_VERSION as u64) {
                            skipped += 1;
                            continue;
                        }
                    }
                }
            }
        }

        let blocks_path = sidecar_root.join(zotron_types::MachineArtifactKind::Blocks.sidecar_relative_path());
        let blocks_content = match fs::read_to_string(&blocks_path) {
            Ok(c) => c,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let blocks: Vec<zotron_types::PdfEvidenceBlock> = blocks_content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        if blocks.is_empty() {
            skipped += 1;
            continue;
        }

        // Re-chunk (same chunk-size handling as `ocr process`).
        let chunks = zotron_types::chunks_from_blocks(&blocks, chunk_chars);

        // Write chunks via the unified writer (adds the schema-version header).
        write_chunks_sidecar(storage_dir, item_key, att_key, &chunks)?;

        // Re-embed.
        let embedding_count = embed_sidecar_chunks(client, storage_dir, item_key, att_key, &chunks);

        reindexed.push(serde_json::json!({
            "itemKey": item_key,
            "attachmentKey": att_key,
            "chunks": chunks.len(),
            "embeddings": embedding_count,
        }));
    }

    format_json(
        &serde_json::json!({
            "reindexed": reindexed.len(),
            "skipped": skipped,
            "items": reindexed,
        }))
}

pub(crate) fn embed_sidecar_chunks(
    client: &mut impl RpcCaller,
    storage_dir: &Path,
    item_key: &str,
    _attachment_key: &str,
    chunks: &[zotron_types::StructureChunk],
) -> usize {
    let Ok((provider, model, api_url, api_key)) = fetch_embedding_settings(client) else {
        return 0;
    };
    if provider.is_empty() || (api_key.is_empty() && provider != "ollama") {
        return 0;
    }
    // Skip empty/whitespace-only chunks: some embedding providers (e.g.
    // volcengine/doubao) reject the ENTIRE batch with HTTP 400 "Input string is
    // empty" if any single input is blank. Blank chunks carry no signal anyway.
    let emb_chunks: Vec<EmbeddingChunkInput> = chunks
        .iter()
        .filter(|c| !c.text.trim().is_empty())
        .map(|c| EmbeddingChunkInput {
            chunk_key: c.chunk_key.clone(),
            text: c.text.clone(),
        })
        .collect();
    if emb_chunks.is_empty() {
        return 0;
    }
    // Batch in groups of 20 to avoid API limits. On ANY batch failure we must
    // NOT clobber a complete existing vectors file with a truncated partial
    // result — that silently degrades future retrieval while the caller reports
    // a normal count. We collect into a temp file and atomically rename only if
    // every batch succeeded; on partial failure we warn and leave the existing
    // (complete) file untouched.
    let batch_size = 20;
    let total_batches = emb_chunks.chunks(batch_size).count();
    let mut all_vectors: Vec<EmbeddingVector> = Vec::new();
    let mut failed = false;
    for (i, batch) in emb_chunks.chunks(batch_size).enumerate() {
        let input = EmbeddingRequestInput {
            item_key: item_key.to_string(),
            chunks: batch.to_vec(),
            model: if model.is_empty() { None } else { Some(model.clone()) },
            url: if api_url.is_empty() { None } else { Some(api_url.clone()) },
            input_type: Some("document".to_string()),
        };
        let request = match build_embedding_provider_request(&provider, &input) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} request build failed: {e}", i + 1);
                failed = true;
                break;
            }
        };
        let Some(url) = request.url.as_deref() else {
            eprintln!("warning: embedding batch {}/{total_batches} has no provider URL configured", i + 1);
            failed = true;
            break;
        };
        let mut http = ureq::post(url).set("Content-Type", "application/json");
        if let Some(auth) = request.auth_header {
            if !api_key.is_empty() {
                http = http.set(auth, &format!("Bearer {api_key}"));
            }
        }
        let resp = match http.send_json(&request.body) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} request failed: {e}", i + 1);
                failed = true;
                break;
            }
        };
        let payload: Value = match resp.into_json() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} response parse failed: {e}", i + 1);
                failed = true;
                break;
            }
        };
        match parse_embedding_provider_response(&provider, &payload, item_key, batch) {
            Ok(vectors) => all_vectors.extend(vectors),
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} parse failed: {e}", i + 1);
                failed = true;
                break;
            }
        }
    }

    if failed {
        let filename = embedding_vector_filename(&provider, &model);
        let vectors_path = storage_dir.join(".zotron").join("embeddings").join(&filename);
        let preserved = if vectors_path.exists() {
            " — existing vectors left untouched"
        } else {
            ""
        };
        eprintln!(
            "warning: embedding incomplete ({} of {} chunks); not writing partial vectors{preserved}",
            all_vectors.len(),
            emb_chunks.len(),
        );
        // Signal partial/failed state to the caller (0 == nothing reliably written).
        return 0;
    }

    let count = all_vectors.len();
    if count > 0 {
        let filename = embedding_vector_filename(&provider, &model);
        let vectors_dir = storage_dir.join(".zotron").join("embeddings");
        if let Err(e) = fs::create_dir_all(&vectors_dir) {
            eprintln!("warning: cannot create embeddings dir {}: {e}", vectors_dir.display());
            return 0;
        }
        let vectors_path = vectors_dir.join(&filename);
        let mut out = String::new();
        for v in &all_vectors {
            if let Ok(line) = serde_json::to_string(v) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        // Atomic write: temp file in the same dir, then rename over the target.
        let tmp_path = vectors_dir.join(format!("{filename}.tmp"));
        if let Err(e) = fs::write(&tmp_path, &out) {
            eprintln!("warning: failed to write temp embeddings {}: {e}", tmp_path.display());
            return 0;
        }
        if let Err(e) = fs::rename(&tmp_path, &vectors_path) {
            eprintln!("warning: failed to persist embeddings to {}: {e}", vectors_path.display());
            let _ = fs::remove_file(&tmp_path);
            return 0;
        }
    }
    count
}

pub(crate) fn resolve_attachment_path(
    client: &mut impl RpcCaller,
    attachment_key: &str,
) -> Result<PathBuf, String> {
    let payload = client.call(
        "attachments.getPath",
        Some(serde_json::json!({"key": attachment_key})),
    )?;
    let raw_path = payload
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| {
            format!("ATTACHMENT_PATH_NOT_FOUND: attachment {attachment_key} has no local PDF path")
        })?;
    Ok(PathBuf::from(local_path_from_zotero_path(raw_path)))
}

/// Resolve the first PDF attachment key for a parent item via `attachments.list`.
pub(crate) fn resolve_first_pdf_attachment_key(
    client: &mut impl RpcCaller,
    parent_key: &str,
) -> Result<String, String> {
    let response = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": parent_key})),
    )?;
    // The XPI returns {items: [...], total: N}.
    let attachments = response
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| response.as_array())
        .ok_or_else(|| {
            format!("NO_PDF_ATTACHMENT: no attachments found for item {parent_key}")
        })?;
    for attachment in attachments {
        if is_pdf_attachment(attachment) {
            if let Some(key) = attachment.get("key").and_then(Value::as_str) {
                return Ok(key.to_string());
            }
        }
    }
    Err(format!(
        "NO_PDF_ATTACHMENT: no PDF attachment found for item {parent_key}"
    ))
}

pub(crate) fn ocr_async_task_result(provider: &str, payload: &Value) -> Option<Value> {
    let data = payload.get("data")?;
    let task_id = data.get("task_id").and_then(Value::as_str)?;
    Some(serde_json::json!({
        "provider": provider,
        "status": "submitted",
        "taskId": task_id,
        "state": data.get("state").and_then(Value::as_str).unwrap_or("submitted"),
        "resultUrl": data.get("full_zip_url").or_else(|| data.get("markdown_url")).cloned(),
        "raw": payload,
    }))
}

pub(crate) fn ocr_input_from_file(
    file: String,
    item_key: Option<String>,
    attachment_key: Option<String>,
    mime_type: Option<String>,
) -> Result<OcrRequestInput, String> {
    let item_key = item_key
        .ok_or_else(|| "INVALID_ARGS: --item-key is required when using --file".to_string())?;
    let attachment_key = attachment_key.ok_or_else(|| {
        "INVALID_ARGS: --attachment-key is required when using --file".to_string()
    })?;
    let path = PathBuf::from(&file);
    let bytes = fs::read(&path).map_err(|err| format!("read {file}: {err}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf")
        .to_string();
    let mime_type = mime_type.unwrap_or_else(|| guess_mime_type(&path).to_string());
    Ok(OcrRequestInput {
        item_key,
        attachment_key,
        file_name,
        mime_type,
        content_base64: base64_encode(&bytes),
        source_url: None,
        local_path: Some(file),
        output_dir: None,
    })
}

pub(crate) fn guess_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/pdf",
    }
}

pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub(crate) fn run_ocr_status_command(
    client: &mut impl RpcCaller,
    collection: String,
) -> Result<Value, String> {
    let collection_key = find_collection_in_tree(client, &collection)?
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

    let mut has_ocr = 0usize;
    for item in &items {
        let item_key = item.get("key").cloned().unwrap_or(Value::Null);
        if has_ocr_artifact(client, &item_key)? || has_ocr_note(client, &item_key)? {
            has_ocr += 1;
        }
    }

    Ok(serde_json::json!({
        "collection": collection,
        "total": items.len(),
        "hasOcr": has_ocr,
        "missingOcr": items.len() - has_ocr,
    }))
}

pub(crate) fn has_ocr_artifact(client: &mut impl RpcCaller, item_key: &Value) -> Result<bool, String> {
    if let Some(item_key) = item_key.as_str() {
        // Legacy external store lookup for artifacts produced before the
        // per-attachment hidden sidecar became the default.
        if machine_artifact_exists_for_item(
            machine_artifact_store_root(),
            item_key,
            MachineArtifactKind::Chunks,
        ) {
            return Ok(true);
        }
    }

    let attachments = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": item_key.clone()})),
    )?;
    Ok(attachments.as_array().is_some_and(|attachments| {
        attachments.iter().any(|attachment| {
            let has_sidecar_chunks = attachment
                .get("path")
                .and_then(Value::as_str)
                .map(local_path_from_zotero_path)
                .as_deref()
                .map(Path::new)
                .and_then(Path::parent)
                .is_some_and(|dir| {
                    machine_artifact_exists_in_sidecar(dir, MachineArtifactKind::Chunks)
                });
            if has_sidecar_chunks {
                return true;
            }

            // Read-only fallback for old Zotero-visible artifact attachments.
            attachment
                .get("title")
                .and_then(Value::as_str)
                .is_some_and(|title| title.ends_with("zotron-chunks.jsonl"))
        })
    }))
}

pub(crate) fn has_ocr_note(client: &mut impl RpcCaller, item_key: &Value) -> Result<bool, String> {
    let notes = client.call(
        "notes.list",
        Some(serde_json::json!({"parentKey": item_key.clone()})),
    )?;
    Ok(notes.as_array().is_some_and(|notes| {
        notes.iter().any(|note| {
            note.get("tags")
                .and_then(Value::as_array)
                .is_some_and(|tags| tags.iter().any(tag_is_ocr))
        })
    }))
}

pub(crate) fn tag_is_ocr(tag: &Value) -> bool {
    tag.as_str() == Some("ocr")
        || tag
            .get("tag")
            .and_then(Value::as_str)
            .is_some_and(|tag| tag == "ocr")
}
