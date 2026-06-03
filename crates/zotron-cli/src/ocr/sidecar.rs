//! Provider-neutral hidden-sidecar artifact writers.

use super::*;

pub(crate) fn write_sidecar_json(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    value: &Value,
) -> Result<Value, String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    write_sidecar_bytes(storage_dir, item_key, attachment_key, kind, &bytes)
}

pub(crate) fn write_sidecar_jsonl<T: serde::Serialize>(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    values: &[T],
) -> Result<Value, String> {
    let mut out = String::new();
    for value in values {
        out.push_str(&serde_json::to_string(value).map_err(|err| err.to_string())?);
        out.push('\n');
    }
    write_sidecar_bytes(storage_dir, item_key, attachment_key, kind, out.as_bytes())
}

/// Write the Chunks sidecar with a `{"schema_version":N}` header line followed
/// by one chunk per line. The header lets `ocr reindex --stale-only` detect
/// freshly-produced (current-schema) sidecars and skip re-embedding them.
/// This is the single writer for the Chunks artifact — `ocr process` (sync +
/// MinerU) and `ocr reindex` all go through here so the on-disk format stays
/// consistent.
pub(crate) fn write_chunks_sidecar(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    chunks: &[zotron_types::StructureChunk],
) -> Result<Value, String> {
    let mut out = String::new();
    out.push_str(&format!("{{\"schema_version\":{CHUNK_SCHEMA_VERSION}}}\n"));
    for chunk in chunks {
        out.push_str(&serde_json::to_string(chunk).map_err(|err| err.to_string())?);
        out.push('\n');
    }
    write_sidecar_bytes(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::Chunks,
        out.as_bytes(),
    )
}

pub(crate) fn write_sidecar_bytes(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    bytes: &[u8],
) -> Result<Value, String> {
    let record = write_machine_artifact_sidecar(storage_dir, item_key, attachment_key, kind, bytes)
        .map_err(|err| format!("write sidecar {:?}: {err}", kind))?;
    Ok(serde_json::json!({
        "kind": kind,
        "relativePath": record.relative_path,
        "absolutePath": record.absolute_path,
    }))
}

pub(crate) fn write_extra_sidecar_bytes(
    storage_dir: &Path,
    relative_path: &str,
    bytes: &[u8],
) -> Result<Value, String> {
    let absolute_path = storage_dir.join(relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    fs::write(&absolute_path, bytes)
        .map_err(|err| format!("write sidecar {}: {err}", absolute_path.display()))?;
    Ok(serde_json::json!({
        "kind": "ocr_raw_zip",
        "relativePath": relative_path,
        "absolutePath": absolute_path,
    }))
}
