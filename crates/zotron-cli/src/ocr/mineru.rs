//! MinerU cloud/CLI ingestion: async submit/poll, result-zip extraction,
//! sidecar persistence, and the fs/http helpers they use.

use super::*;

pub(crate) struct MineruResultSource {
    pub(crate) task_id: Option<String>,
    pub(crate) state: String,
    result_dir: PathBuf,
    raw_zip_bytes: Option<Vec<u8>>,
    task_status: Option<Value>,
    payload: Value,
    content_list_file: Option<PathBuf>,
    markdown: Option<String>,
}

pub(crate) struct PersistedOcrArtifacts {
    pub(crate) block_count: usize,
    pub(crate) artifacts: Vec<Value>,
    pub(crate) chunks: Vec<zotron_types::StructureChunk>,
}

pub(crate) fn load_mineru_result_source(
    options: &OcrProcessOptions,
    attachment_path: &Path,
    file_name: &str,
) -> Result<MineruResultSource, String> {
    if let Some(result_dir) = options.result_dir.as_deref() {
        return mineru_result_source_from_dir(PathBuf::from(result_dir), None, None, None);
    }
    if let Some(result_zip) = options.result_zip.as_deref() {
        let zip_path = PathBuf::from(result_zip);
        let zip_bytes = fs::read(&zip_path)
            .map_err(|err| format!("read MinerU result zip {}: {err}", zip_path.display()))?;
        let result_dir = extract_zip_bytes_to_temp("zotron-mineru-result", &zip_bytes)?;
        return mineru_result_source_from_dir(result_dir, Some(zip_bytes), None, None);
    }

    let Some(source_url) = options
        .source_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return submit_mineru_local_file(options, attachment_path, file_name);
    };
    let input = OcrRequestInput {
        item_key: options.parent.clone(),
        attachment_key: options.attachment.clone().expect("attachment resolved"),
        file_name: file_name.to_string(),
        mime_type: "application/pdf".to_string(),
        content_base64: format!("url:{source_url}"),
        source_url: Some(source_url.to_string()),
        local_path: None,
        output_dir: None,
    };
    let task = submit_mineru_task(
        &options.provider,
        &input,
        options.provider_endpoint.clone(),
        &options.api_key_env,
    )?;
    let task_id = task
        .get("data")
        .and_then(|data| data.get("task_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU submit response missing data.task_id".to_string())?
        .to_string();
    let auth_header = provider_auth_header_value(&options.api_key_env, "bearer")?;
    let status = poll_mineru_task(
        options.provider_endpoint.as_deref(),
        &task_id,
        &auth_header,
        options.poll_interval_seconds,
        options.timeout_seconds,
    )?;
    let zip_url = status
        .pointer("/data/full_zip_url")
        .or_else(|| status.pointer("/data/result/full_zip_url"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU completed task missing data.full_zip_url".to_string())?;
    let zip_bytes = download_bytes(zip_url)?;
    let result_dir = extract_zip_bytes_to_temp("zotron-mineru-result", &zip_bytes)?;
    mineru_result_source_from_dir(result_dir, Some(zip_bytes), Some(status), Some(task_id))
}

pub(crate) fn submit_mineru_local_file(
    options: &OcrProcessOptions,
    attachment_path: &Path,
    file_name: &str,
) -> Result<MineruResultSource, String> {
    let auth_header = provider_auth_header_value(&options.api_key_env, "bearer")?;
    let upload_request = create_mineru_file_upload(
        options.provider_endpoint.as_deref(),
        file_name,
        options.attachment.as_deref().expect("attachment resolved"),
        &auth_header,
    )?;
    let upload_url = upload_request
        .pointer("/data/file_urls/0")
        .or_else(|| upload_request.pointer("/data/fileUrls/0"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU upload URL response missing data.file_urls[0]".to_string())?;
    let batch_id = upload_request
        .pointer("/data/batch_id")
        .or_else(|| upload_request.pointer("/data/batchId"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU upload URL response missing data.batch_id".to_string())?
        .to_string();
    let bytes = fs::read(attachment_path)
        .map_err(|err| format!("read attachment PDF {}: {err}", attachment_path.display()))?;
    put_bytes(upload_url, &bytes)?;
    let status = poll_mineru_batch(
        options.provider_endpoint.as_deref(),
        &batch_id,
        &auth_header,
        options.poll_interval_seconds,
        options.timeout_seconds,
    )?;
    let zip_url = mineru_batch_zip_url(&status)
        .ok_or_else(|| "MinerU completed batch missing full_zip_url".to_string())?;
    let zip_bytes = download_bytes(&zip_url)?;
    let result_dir = extract_zip_bytes_to_temp("zotron-mineru-result", &zip_bytes)?;
    mineru_result_source_from_dir(result_dir, Some(zip_bytes), Some(status), Some(batch_id))
}

pub(crate) fn create_mineru_file_upload(
    endpoint: Option<&str>,
    file_name: &str,
    data_id: &str,
    auth_header: &str,
) -> Result<Value, String> {
    let url = mineru_file_urls_url(endpoint);
    let body = serde_json::json!({
        "files": [{"name": file_name, "data_id": data_id}],
        "model_version": "vlm",
        "is_ocr": false,
        "enable_formula": true,
        "enable_table": true,
        "language": "ch",
        "page_ranges": "1-200",
    });
    ureq::post(&url)
        .set("Authorization", auth_header)
        .send_json(body)
        .map_err(|err| format!("POST {url} failed: {err}"))?
        .into_json::<Value>()
        .map_err(|err| format!("POST {url} returned invalid JSON: {err}"))
}

pub(crate) fn put_bytes(url: &str, bytes: &[u8]) -> Result<(), String> {
    ureq::put(url)
        .send_bytes(bytes)
        .map_err(|err| format!("PUT {url} failed: {err}"))?;
    Ok(())
}

pub(crate) fn submit_mineru_task(
    provider: &str,
    input: &OcrRequestInput,
    endpoint: Option<String>,
    api_key_env: &str,
) -> Result<Value, String> {
    let request = build_ocr_provider_request(provider, input)?;
    let method = request
        .method
        .ok_or_else(|| "MinerU provider missing HTTP method".to_string())?;
    let mut transport = provider_http_transport_with_auth(Some(api_key_env), "bearer")?;
    transport.post_json(&ProviderHttpInvocation {
        provider: request.provider.to_string(),
        style: request.style.to_string(),
        method: method.to_string(),
        url: endpoint.or_else(|| request.url.map(ToString::to_string)),
        auth_header_name: request.auth_header.map(ToString::to_string),
        auth_header_value: None,
        body: request.body,
    })
}

pub(crate) fn poll_mineru_task(
    endpoint: Option<&str>,
    task_id: &str,
    auth_header: &str,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let url = mineru_task_status_url(endpoint, task_id);
    let started = Instant::now();
    loop {
        let status = get_json_with_auth(&url, auth_header)?;
        let state = status
            .pointer("/data/state")
            .or_else(|| status.pointer("/data/status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        match state {
            "done" | "finished" | "success" => return Ok(status),
            "failed" | "error" => return Err(format!("MinerU task {task_id} failed: {status}")),
            _ => {
                if started.elapsed() >= Duration::from_secs(timeout_seconds) {
                    return Err(format!(
                        "MinerU task {task_id} timed out after {timeout_seconds}s with state {state}"
                    ));
                }
                thread::sleep(Duration::from_secs(poll_interval_seconds.max(1)));
            }
        }
    }
}

pub(crate) fn mineru_task_status_url(endpoint: Option<&str>, task_id: &str) -> String {
    let base = endpoint
        .unwrap_or("https://mineru.net/api/v4/extract/task")
        .trim_end_matches('/');
    if base.ends_with("/extract/task") {
        format!("{base}/{task_id}")
    } else {
        format!("{base}/extract/task/{task_id}")
    }
}

pub(crate) fn mineru_file_urls_url(endpoint: Option<&str>) -> String {
    let base = mineru_api_base(endpoint);
    format!("{base}/file-urls/batch")
}

pub(crate) fn mineru_batch_status_url(endpoint: Option<&str>, batch_id: &str) -> String {
    let base = mineru_api_base(endpoint);
    format!("{base}/extract-results/batch/{batch_id}")
}

pub(crate) fn mineru_api_base(endpoint: Option<&str>) -> String {
    let base = endpoint
        .unwrap_or("https://mineru.net/api/v4/extract/task")
        .trim_end_matches('/');
    if let Some(stripped) = base.strip_suffix("/extract/task") {
        return stripped.to_string();
    }
    if let Some(stripped) = base.strip_suffix("/extract") {
        return stripped.to_string();
    }
    base.to_string()
}

pub(crate) fn poll_mineru_batch(
    endpoint: Option<&str>,
    batch_id: &str,
    auth_header: &str,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let url = mineru_batch_status_url(endpoint, batch_id);
    let started = Instant::now();
    loop {
        let status = get_json_with_auth(&url, auth_header)?;
        let state = mineru_batch_state(&status).unwrap_or("unknown");
        match state {
            "done" | "finished" | "success" => return Ok(status),
            "failed" | "error" => return Err(format!("MinerU batch {batch_id} failed: {status}")),
            _ => {
                if started.elapsed() >= Duration::from_secs(timeout_seconds) {
                    return Err(format!(
                        "MinerU batch {batch_id} timed out after {timeout_seconds}s with state {state}"
                    ));
                }
                thread::sleep(Duration::from_secs(poll_interval_seconds.max(1)));
            }
        }
    }
}

pub(crate) fn mineru_batch_state(status: &Value) -> Option<&str> {
    status
        .pointer("/data/extract_result/0/state")
        .or_else(|| status.pointer("/data/extractResult/0/state"))
        .or_else(|| status.pointer("/data/state"))
        .and_then(Value::as_str)
}

pub(crate) fn mineru_batch_zip_url(status: &Value) -> Option<String> {
    status
        .pointer("/data/extract_result/0/full_zip_url")
        .or_else(|| status.pointer("/data/extractResult/0/full_zip_url"))
        .or_else(|| status.pointer("/data/full_zip_url"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

pub(crate) fn provider_auth_header_value(api_key_env: &str, auth_scheme: &str) -> Result<String, String> {
    let token = env::var(api_key_env)
        .map_err(|_| format!("missing provider credential env var {api_key_env}"))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(format!(
            "provider credential env var {api_key_env} is empty"
        ));
    }
    Ok(match auth_scheme {
        "bearer" if token.starts_with("Bearer ") => token.to_string(),
        "bearer" => format!("Bearer {token}"),
        "token" if token.starts_with("token ") => token.to_string(),
        "token" => format!("token {token}"),
        _ => token.to_string(),
    })
}

pub(crate) fn get_json_with_auth(url: &str, auth_header: &str) -> Result<Value, String> {
    ureq::get(url)
        .set("Authorization", auth_header)
        .call()
        .map_err(|err| format!("GET {url} failed: {err}"))?
        .into_json::<Value>()
        .map_err(|err| format!("GET {url} returned invalid JSON: {err}"))
}

pub(crate) fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|err| format!("download {url} failed: {err}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| format!("read download {url}: {err}"))?;
    Ok(bytes)
}

pub(crate) fn extract_zip_bytes_to_temp(prefix: &str, zip_bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = unique_temp_path(prefix);
    fs::create_dir_all(&dir).map_err(|err| format!("create temp dir {}: {err}", dir.display()))?;
    let zip_path = dir.with_extension("zip");
    fs::write(&zip_path, zip_bytes)
        .map_err(|err| format!("write temp zip {}: {err}", zip_path.display()))?;
    let output = ProcessCommand::new("unzip")
        .arg("-q")
        .arg("-o")
        .arg(&zip_path)
        .arg("-d")
        .arg(&dir)
        .output()
        .map_err(|err| format!("run unzip: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "unzip {} failed: {}",
            zip_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(dir)
}

pub(crate) fn unique_temp_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

pub(crate) fn mineru_result_source_from_dir(
    result_dir: PathBuf,
    raw_zip_bytes: Option<Vec<u8>>,
    task_status: Option<Value>,
    task_id: Option<String>,
) -> Result<MineruResultSource, String> {
    let (payload, content_list_file) = mineru_payload_from_result_dir(&result_dir)?;
    let markdown = find_first_file_by_name(&result_dir, "full.md")
        .map(|path| {
            fs::read_to_string(&path)
                .map_err(|err| format!("read native markdown {}: {err}", path.display()))
        })
        .transpose()?;
    Ok(MineruResultSource {
        task_id,
        state: "done".to_string(),
        result_dir,
        raw_zip_bytes,
        task_status,
        payload,
        content_list_file,
        markdown,
    })
}

pub(crate) fn mineru_payload_from_result_dir(result_dir: &Path) -> Result<(Value, Option<PathBuf>), String> {
    let v2 = find_first_file_with_suffix(result_dir, "_content_list_v2.json");
    if let Some(path) = v2 {
        let value = read_json_file(&path)?;
        return Ok((serde_json::json!({"content_list_v2": value}), Some(path)));
    }
    let content_list = find_first_file_with_suffix(result_dir, "_content_list.json");
    if let Some(path) = content_list {
        let value = read_json_file(&path)?;
        return Ok((serde_json::json!({"content_list": value}), Some(path)));
    }
    let layout = find_first_file_by_name(result_dir, "layout.json");
    if let Some(path) = layout {
        return Ok((read_json_file(&path)?, Some(path)));
    }
    let markdown = find_first_file_by_name(result_dir, "full.md");
    if let Some(path) = markdown {
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("read native markdown {}: {err}", path.display()))?;
        return Ok((serde_json::json!({"result": text}), Some(path)));
    }
    Err(format!(
        "MinerU result directory {} missing content_list_v2/content_list/layout/full.md",
        result_dir.display()
    ))
}

pub(crate) fn read_json_file(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse JSON {}: {err}", path.display()))
}

pub(crate) fn persist_mineru_result_sidecars(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    provider: &str,
    source: &MineruResultSource,
    chunk_chars: usize,
) -> Result<PersistedOcrArtifacts, String> {
    let blocks = parse_ocr_provider_response(provider, &source.payload, item_key, attachment_key)?;
    let chunks = zotron_types::chunks_from_blocks(&blocks, chunk_chars);
    let assets = copy_mineru_assets(&source.result_dir, storage_dir)?;
    let raw_bundle = serde_json::json!({
        "provider": provider,
        "item_key": item_key,
        "attachment_key": attachment_key,
        "task_id": source.task_id,
        "state": source.state,
        "task_status": source.task_status,
        "content_list_file": source.content_list_file,
        "payload": source.payload,
    });

    let mut artifacts = Vec::new();
    artifacts.push(write_sidecar_json(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::OcrRaw,
        &raw_bundle,
    )?);
    artifacts.push(write_sidecar_jsonl(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::Blocks,
        &blocks,
    )?);
    artifacts.push(write_chunks_sidecar(
        storage_dir,
        item_key,
        attachment_key,
        &chunks,
    )?);
    if let Some(markdown) = source.markdown.as_deref() {
        artifacts.push(write_sidecar_bytes(
            storage_dir,
            item_key,
            attachment_key,
            MachineArtifactKind::OcrNativeMarkdown,
            markdown.as_bytes(),
        )?);
    }
    artifacts.push(write_sidecar_json(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::OcrNativeAssets,
        &assets,
    )?);
    if let Some(bytes) = source.raw_zip_bytes.as_deref() {
        artifacts.push(write_extra_sidecar_bytes(
            storage_dir,
            ".zotron/ocr/latest.raw.zip",
            bytes,
        )?);
    }

    Ok(PersistedOcrArtifacts {
        block_count: blocks.len(),
        artifacts,
        chunks,
    })
}

pub(crate) fn copy_mineru_assets(result_dir: &Path, storage_dir: &Path) -> Result<Value, String> {
    let mut images = Vec::new();
    for file in collect_files(result_dir)? {
        if !is_image_file(&file) {
            continue;
        }
        let relative = file.strip_prefix(result_dir).unwrap_or(&file).to_path_buf();
        let destination = storage_dir.join(".zotron").join("ocr").join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::copy(&file, &destination).map_err(|err| {
            format!(
                "copy MinerU asset {} to {}: {err}",
                file.display(),
                destination.display()
            )
        })?;
        images.push(serde_json::json!({
            "sourceRelative": relative,
            "sidecarRelative": PathBuf::from(".zotron").join("ocr").join(&relative),
            "absolutePath": destination,
        }));
    }
    Ok(serde_json::json!({
        "provider": "mineru",
        "images": images,
    }))
}

pub(crate) fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif"
    )
}

pub(crate) fn find_first_file_with_suffix(root: &Path, suffix: &str) -> Option<PathBuf> {
    collect_files(root).ok()?.into_iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(suffix))
    })
}

pub(crate) fn find_first_file_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    collect_files(root).ok()?.into_iter().find(|path| {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name == name)
    })
}

pub(crate) fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files_into(root, &mut files)?;
    files.sort();
    Ok(files)
}

pub(crate) fn collect_files_into(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|err| format!("read dir {}: {err}", root.display()))? {
        let entry = entry.map_err(|err| format!("read dir entry {}: {err}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("stat {}: {err}", path.display()))?;
        if file_type.is_dir() {
            collect_files_into(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}
