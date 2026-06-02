//! Per-noun command handlers (items / collections / tags / notes /
//! annotations / search / settings / export) and their shared helpers,
//! extracted verbatim from `lib.rs`. `run_command` dispatch and the clap
//! definitions remain in `lib.rs`.

use std::{
    env, fs,
    io::{self, Read},
    path::Path,
    process::Command as ProcessCommand,
};

use serde_json::Value;
use zotron_types::is_zotron_evidence_artifact;

use crate::ocr::{download_bytes, unique_temp_path};
use crate::output::{format_json, normalize_list_envelope, raw_value_output};
use crate::rpc::RpcCaller;
use crate::{
    AnnotationsCommand, CollectionsCommand, ExportArgs, ItemsCommand, NotesCommand, SearchArgs,
    SearchManagementCommand, SettingsCommand, SystemCommand, TagsCommand,
};

pub(crate) fn local_path_from_zotero_path(path: &str) -> String {
    if is_wsl() && path.as_bytes().get(1) == Some(&b':') {
        return ProcessCommand::new("wslpath")
            .arg("-u")
            .arg(path)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|converted| converted.trim().to_string())
            .filter(|converted| !converted.is_empty())
            .unwrap_or_else(|| path.to_string());
    }
    path.to_string()
}

pub(crate) fn find_collection_in_tree(
    client: &mut impl RpcCaller,
    collection: &str,
) -> Result<Option<Value>, String> {
    let tree = client.call("collections.tree", None)?;
    let nodes = tree
        .as_array()
        .ok_or_else(|| "collections.tree returned non-array result".to_string())?;
    Ok(search_collection_tree(nodes, collection).cloned())
}

fn search_collection_tree<'a>(nodes: &'a [Value], collection: &str) -> Option<&'a Value> {
    for node in nodes {
        if node.get("key").and_then(Value::as_str) == Some(collection)
            || node.get("name").and_then(Value::as_str) == Some(collection)
        {
            return Some(node);
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            if let Some(found) = search_collection_tree(children, collection) {
                return Some(found);
            }
        }
    }
    None
}

pub(crate) fn run_push_command(
    json_file: String,
    pdf: Option<String>,
    collection: Option<String>,
    on_duplicate: String,
    dry_run: bool,
    client: &mut impl RpcCaller,
) -> Result<String, String> {
    if !matches!(on_duplicate.as_str(), "skip" | "update" | "create") {
        return Err(format!(
            "INVALID_ARGS: --on-duplicate must be skip|update|create, got {on_duplicate:?}"
        ));
    }

    let payload = if json_file == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|err| format!("read stdin: {err}"))?;
        input
    } else {
        fs::read_to_string(&json_file).map_err(|err| format!("read {json_file}: {err}"))?
    };
    let mut item_json = serde_json::from_str::<Value>(&payload)
        .map_err(|err| format!("INVALID_JSON: Could not parse JSON: {err}"))?;

    // Validate required fields
    match item_json.get("itemType").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => {}
        _ => return Err("INVALID_ARGS: input JSON must include a non-empty \"itemType\" field".to_string()),
    }

    // Strip the embedded `_pdf` so it never leaks into the item's fields, and
    // let an explicit --pdf flag win over it. `_pdf` may be a URL or a local
    // path, enabling the documented `fetch | push` one-liner to auto-attach.
    let embedded_pdf = take_embedded_pdf(&mut item_json)?;
    let pdf_source = pdf.or(embedded_pdf);

    if dry_run {
        let collection_key = collection
            .as_deref()
            .map(|name| resolve_collection(client, name))
            .transpose()?;
        return format_json(
            &serde_json::json!({
                "ok": true,
                "dryRun": true,
                "wouldPush": {
                    "title": item_json.get("title").cloned().unwrap_or(Value::Null),
                    "itemType": item_json.get("itemType").cloned().unwrap_or(Value::Null),
                    "collectionKey": collection_key,
                    "pdfPath": pdf_source,
                    "onDuplicate": on_duplicate,
                }
            }));
    }

    // Resolve a URL `_pdf` to a local temp file so the rest of the push path
    // (magic-byte validation, size tracking) is uniform with --pdf. The guard
    // keeps the temp file alive until the attachment has been imported.
    let downloaded_pdf = pdf_source
        .as_deref()
        .filter(|source| is_remote_pdf(source))
        .map(download_pdf_to_temp)
        .transpose()?;
    let pdf_path = downloaded_pdf
        .as_deref()
        .map(|p| p.to_string_lossy().into_owned())
        .or(pdf_source);

    let result = push_item(
        client,
        &item_json,
        pdf_path.as_deref(),
        collection.as_deref(),
        &on_duplicate,
    )?;
    format_json(&result)
}

/// Remove and return the item's embedded `_pdf` value (URL or local path).
fn take_embedded_pdf(item_json: &mut Value) -> Result<Option<String>, String> {
    let Some(obj) = item_json.as_object_mut() else {
        return Ok(None);
    };
    match obj.remove("_pdf") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(pdf)) if pdf.trim().is_empty() => Ok(None),
        Some(Value::String(pdf)) => Ok(Some(pdf)),
        Some(other) => Err(format!(
            "INVALID_ARGS: \"_pdf\" must be a URL or local path string, got {other}"
        )),
    }
}

fn is_remote_pdf(source: &str) -> bool {
    let lower = source.trim_start().to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

/// Download a remote `_pdf` to a temp file and verify it is really a PDF.
fn download_pdf_to_temp(url: &str) -> Result<std::path::PathBuf, String> {
    let bytes = download_bytes(url)?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(format!(
            "INVALID_PDF: downloaded {url} does not start with %PDF- magic bytes"
        ));
    }
    let path = unique_temp_path("zotron-push-pdf").with_extension("pdf");
    fs::write(&path, &bytes).map_err(|err| format!("write {}: {err}", path.display()))?;
    Ok(path)
}

fn push_item(
    client: &mut impl RpcCaller,
    item_json: &Value,
    pdf_path: Option<&str>,
    collection: Option<&str>,
    on_duplicate: &str,
) -> Result<Value, String> {
    let pdf_size = if let Some(path) = pdf_path {
        validate_pdf_magic(path)?
    } else {
        0
    };

    let collection_key = match collection {
        Some(name) => resolve_collection(client, name)?,
        None => resolve_current_collection(client)?,
    };

    let dup_id = find_duplicate(client, item_json)?;
    if let Some(dup_id) = dup_id.as_deref().filter(|_| on_duplicate == "skip") {
        if !is_library_root(&collection_key) {
            client.call(
                "collections.addItems",
                Some(serde_json::json!({"key": collection_key, "keys": [dup_id]})),
            )?;
        }
        let mut pdf_attached = false;
        if let Some(path) = pdf_path {
            if !item_has_pdf_attachment(client, dup_id)? {
                attach_pdf(client, dup_id, path)?;
                pdf_attached = true;
            }
        }
        return Ok(push_result(
            "skipped_duplicate",
            Some(dup_id.to_string()),
            pdf_attached,
            if pdf_attached { pdf_size } else { 0 },
            Value::Null,
        ));
    }

    let xpi_payload = to_xpi_payload(item_json, Some(&collection_key));
    let (item_key, status) =
        if let Some(dup_id) = dup_id.as_deref().filter(|_| on_duplicate == "update") {
            let mut params = serde_json::Map::new();
            params.insert("key".to_string(), Value::String(dup_id.to_string()));
            params.insert(
                "fields".to_string(),
                xpi_payload
                    .get("fields")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            );
            if let Some(creators) = xpi_payload.get("creators") {
                params.insert("creators".to_string(), creators.clone());
            }
            if let Some(tags) = xpi_payload.get("tags") {
                params.insert("tags".to_string(), tags.clone());
            }
            client.call("items.update", Some(Value::Object(params)))?;
            (dup_id.to_string(), "updated")
        } else {
            let created = client.call("items.create", Some(xpi_payload))?;
            let key = created
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("items.create returned unexpected shape: {created:?}"))?;
            (key.to_string(), "created")
        };

    let mut pdf_attached = false;
    if let Some(path) = pdf_path {
        if status != "updated" || !item_has_pdf_attachment(client, &item_key)? {
            attach_pdf(client, &item_key, path)?;
            pdf_attached = true;
        }
    }

    if status == "updated" && !is_library_root(&collection_key) {
        client.call(
            "collections.addItems",
            Some(serde_json::json!({"key": collection_key, "keys": [item_key]})),
        )?;
    }

    Ok(push_result(
        status,
        Some(item_key),
        pdf_attached,
        if pdf_attached { pdf_size } else { 0 },
        Value::Null,
    ))
}

fn validate_pdf_magic(path: &str) -> Result<u64, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("INVALID_PDF: cannot read {path}: {e}"))?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(format!(
            "INVALID_PDF: {path} does not start with %PDF- magic bytes"
        ));
    }
    Ok(bytes.len() as u64)
}

fn resolve_current_collection(client: &mut impl RpcCaller) -> Result<Value, String> {
    let selected = client.call("system.currentCollection", None)?;
    Ok(selected
        .get("key")
        .cloned()
        .unwrap_or_else(|| Value::Number(0.into())))
}

fn find_duplicate(
    client: &mut impl RpcCaller,
    item_json: &Value,
) -> Result<Option<String>, String> {
    if let Some(doi) = item_json
        .get("DOI")
        .and_then(Value::as_str)
        .filter(|doi| !doi.is_empty())
    {
        let hits = client.call("search.byIdentifier", Some(serde_json::json!({"doi": doi})))?;
        if let Some(key) = first_hit_key(&hits) {
            return Ok(Some(key));
        }
    }

    if let Some(title) = item_json
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| title.len() >= 10)
    {
        let hits = client.call(
            "search.quick",
            Some(serde_json::json!({"query": title, "limit": 20})),
        )?;
        if let Some(items) = response_items(&hits) {
            for item in items {
                if item.get("title").and_then(Value::as_str) == Some(title) {
                    if let Some(key) = item.get("key").and_then(Value::as_str) {
                        return Ok(Some(key.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn first_hit_key(response: &Value) -> Option<String> {
    response_items(response)?
        .first()?
        .get("key")?
        .as_str()
        .map(ToString::to_string)
}

fn response_items(response: &Value) -> Option<&Vec<Value>> {
    response
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| response.as_array())
}

fn to_xpi_payload(item_json: &Value, collection_key: Option<&Value>) -> Value {
    const NON_FIELD_KEYS: &[&str] = &[
        "itemType",
        "creators",
        "tags",
        "collections",
        "attachments",
        "relations",
        "notes",
        "id",
        "key",
        "version",
        "_pdf",
    ];

    let mut fields = serde_json::Map::new();
    if let Some(item) = item_json.as_object() {
        for (key, value) in item {
            if !NON_FIELD_KEYS.contains(&key.as_str()) && !value.is_null() && value != "" {
                fields.insert(key.clone(), value.clone());
            }
        }
    }

    let mut payload = serde_json::Map::new();
    payload.insert(
        "itemType".to_string(),
        item_json
            .get("itemType")
            .cloned()
            .unwrap_or_else(|| Value::String("journalArticle".to_string())),
    );
    payload.insert("fields".to_string(), Value::Object(fields));

    if let Some(creators) = item_json.get("creators").and_then(Value::as_array) {
        if !creators.is_empty() {
            payload.insert(
                "creators".to_string(),
                Value::Array(
                    creators
                        .iter()
                        .map(|creator| {
                            let mut c = serde_json::json!({
                                "firstName": creator.get("firstName").and_then(Value::as_str).unwrap_or(""),
                                "lastName": creator.get("lastName").and_then(Value::as_str).unwrap_or(""),
                                "creatorType": creator.get("creatorType").and_then(Value::as_str).unwrap_or("author"),
                            });
                            if let Some(fm) = creator.get("fieldMode").and_then(Value::as_u64) {
                                c["fieldMode"] = Value::from(fm);
                            }
                            c
                        })
                        .collect(),
                ),
            );
        }
    }

    if let Some(tags) = item_json.get("tags").and_then(Value::as_array) {
        if !tags.is_empty() {
            payload.insert(
                "tags".to_string(),
                Value::Array(
                    tags.iter()
                        .map(|tag| tag.get("tag").cloned().unwrap_or_else(|| tag.clone()))
                        .collect(),
                ),
            );
        }
    }

    if let Some(collection_key) = collection_key.filter(|key| !is_library_root(key)) {
        payload.insert(
            "collections".to_string(),
            Value::Array(vec![collection_key.clone()]),
        );
    }

    Value::Object(payload)
}

fn item_has_pdf_attachment(client: &mut impl RpcCaller, item_key: &str) -> Result<bool, String> {
    let attachments = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": item_key})),
    )?;
    Ok(has_pdf_attachment(&attachments))
}

fn attach_pdf(client: &mut impl RpcCaller, item_key: &str, path: &str) -> Result<(), String> {
    client.call(
        "attachments.add",
        Some(serde_json::json!({
            "parentKey": item_key,
            "path": zotero_path(path),
            "title": "Full Text PDF",
        })),
    )?;
    Ok(())
}

fn zotero_path(path: &str) -> String {
    let path = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(path).to_path_buf())
        .to_string_lossy()
        .into_owned();
    if is_wsl() {
        return ProcessCommand::new("wslpath")
            .arg("-w")
            .arg(&path)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|converted| converted.trim().to_string())
            .filter(|converted| !converted.is_empty())
            .unwrap_or(path);
    }
    path
}

fn is_wsl() -> bool {
    if env::var_os("WSL_DISTRO_NAME").is_some() {
        return true;
    }
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|release| release.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

fn is_library_root(value: &Value) -> bool {
    value.as_i64() == Some(0) || value.as_u64() == Some(0)
}

fn push_result(
    status: &str,
    zotero_item_key: Option<String>,
    pdf_attached: bool,
    pdf_size_bytes: u64,
    error: Value,
) -> Value {
    serde_json::json!({
        "status": status,
        "zotero_item_key": zotero_item_key,
        "pdf_attached": pdf_attached,
        "pdf_size_bytes": pdf_size_bytes,
        "error": error,
    })
}

pub(crate) fn run_search(
    args: SearchArgs,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let SearchArgs {
        query, fulltext, author, after, before, journal, tag,
        doi, isbn, issn, collection, limit, offset, ..
    } = args;

    let has_identifier = doi.is_some() || isbn.is_some() || issn.is_some();
    if has_identifier {
        let mut params = serde_json::Map::new();
        if let Some(doi) = doi { params.insert("doi".into(), Value::String(doi)); }
        if let Some(isbn) = isbn { params.insert("isbn".into(), Value::String(isbn)); }
        if let Some(issn) = issn { params.insert("issn".into(), Value::String(issn)); }
        let value = client.call("search.byIdentifier", Some(Value::Object(params)))?;
        return Ok(normalize_list_envelope(value, "items", None, 0));
    }

    if fulltext {
        let query = query.ok_or("INVALID_ARGS: --fulltext requires a search query")?;
        let mut params = serde_json::json!({"query": query, "limit": limit});
        if let (Some(col), Some(map)) = (collection, params.as_object_mut()) {
            map.insert("collection".into(), resolve_collection(client, &col)?);
        }
        let value = client.call("search.fulltext", Some(params))?;
        return Ok(normalize_list_envelope(value, "items", Some(limit), 0));
    }

    let has_filters = author.is_some() || after.is_some() || before.is_some()
        || journal.is_some() || tag.is_some();
    if has_filters {
        let mut conditions: Vec<Value> = Vec::new();
        if let Some(query) = &query {
            conditions.push(serde_json::json!({
                "field": "quicksearch-titleCreatorYear",
                "operator": "contains",
                "value": query,
            }));
        }
        if let Some(author) = author {
            conditions.push(serde_json::json!({
                "field": "creator", "operator": "contains", "value": author,
            }));
        }
        if let Some(after) = after {
            conditions.push(serde_json::json!({
                "field": "date", "operator": "isAfter", "value": after,
            }));
        }
        if let Some(before) = before {
            conditions.push(serde_json::json!({
                "field": "date", "operator": "isBefore", "value": before,
            }));
        }
        if let Some(journal) = journal {
            conditions.push(serde_json::json!({
                "field": "publicationTitle", "operator": "contains", "value": journal,
            }));
        }
        if let Some(tag) = tag {
            conditions.push(serde_json::json!({
                "field": "tag", "operator": "is", "value": tag,
            }));
        }
        let value = client.call(
            "search.advanced",
            Some(serde_json::json!({
                "conditions": conditions,
                "operator": "and",
                "limit": limit,
                "offset": offset,
            })),
        )?;
        return Ok(normalize_list_envelope(value, "items", Some(limit), offset));
    }

    let query = query.ok_or(
        "INVALID_ARGS: provide a search query, or use --doi/--isbn/--issn for identifier lookup"
    )?;
    let value = if let Some(col) = collection {
        let key = resolve_collection(client, &col)?;
        let response = client.call(
            "collections.getItems",
            Some(serde_json::json!({"key": key})),
        )?;
        collection_quick_search_response(&response, &query, limit)
    } else {
        filter_search_artifacts(client.call(
            "search.quick",
            Some(serde_json::json!({"query": query, "limit": limit})),
        )?)
    };
    Ok(normalize_list_envelope(value, "items", Some(limit), 0))
}

pub(crate) fn run_search_management_command(
    command: SearchManagementCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    match command {
        SearchManagementCommand::SavedSearches => Ok(normalize_list_envelope(
            client.call("search.savedSearches", None)?,
            "items",
            None,
            0,
        )),
        SearchManagementCommand::CreateSaved {
            name, condition, dry_run, ..
        } => {
            let conditions = condition
                .iter()
                .map(|raw| parse_search_condition(raw))
                .collect::<Result<Vec<_>, _>>()?;
            let params = serde_json::json!({"name": name, "conditions": conditions});
            if dry_run {
                Ok(dry_run_value("search.createSavedSearch", params))
            } else {
                Ok(client.call("search.createSavedSearch", Some(params))?)
            }
        }
        SearchManagementCommand::DeleteSaved {
            search_key, dry_run, ..
        } => {
            let params = serde_json::json!({"key": search_key});
            if dry_run {
                Ok(dry_run_value("search.deleteSavedSearch", params))
            } else {
                Ok(client.call("search.deleteSavedSearch", Some(params))?)
            }
        }
    }
}

fn filter_search_artifacts(mut value: Value) -> Value {
    let Some(items) = value.get_mut("items").and_then(Value::as_array_mut) else {
        return value;
    };
    items.retain(|item| match item.get("title").and_then(Value::as_str) {
        Some(title) => !is_zotron_evidence_artifact(title),
        None => true,
    });
    let total_items = items.len() as u64;
    if let Some(total) = value.get_mut("total") {
        *total = Value::from(total_items);
    }
    value
}

fn collection_quick_search_response(response: &Value, query: &str, limit: u64) -> Value {
    let mut matched = collection_items(response)
        .into_iter()
        .filter(|item| !item_is_evidence_artifact(item))
        .filter(|item| quick_item_matches(item, query))
        .collect::<Vec<_>>();
    let total = matched.len() as u64;
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    if matched.len() > limit {
        matched.truncate(limit);
    }
    serde_json::json!({"items": matched, "total": total})
}

fn item_is_evidence_artifact(item: &Value) -> bool {
    item.get("title")
        .and_then(Value::as_str)
        .is_some_and(is_zotron_evidence_artifact)
}

fn quick_item_matches(item: &Value, query: &str) -> bool {
    let terms = query
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return true;
    }
    let mut haystack = String::new();
    append_search_text(item, &mut haystack);
    let haystack = haystack.to_lowercase();
    terms.iter().all(|term| haystack.contains(term))
}

fn append_search_text(value: &Value, out: &mut String) {
    match value {
        Value::String(text) => {
            out.push(' ');
            out.push_str(text);
        }
        Value::Number(number) => {
            out.push(' ');
            out.push_str(&number.to_string());
        }
        Value::Bool(value) => {
            out.push(' ');
            out.push_str(if *value { "true" } else { "false" });
        }
        Value::Array(items) => {
            for item in items {
                append_search_text(item, out);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                append_search_text(item, out);
            }
        }
        Value::Null => {}
    }
}

fn parse_search_condition(raw: &str) -> Result<Value, String> {
    let mut parts = raw.split_whitespace();
    let field = parts.next();
    let operator = parts.next();
    let value = parts.collect::<Vec<_>>().join(" ");
    match (field, operator, value.is_empty()) {
        (Some(field), Some(operator), false) => Ok(serde_json::json!({
            "field": field,
            "operator": operator,
            "value": value,
        })),
        _ => Err(format!(
            "INVALID_ARGS: --condition must be 'field operator value', got: {raw:?}"
        )),
    }
}

const RPC_PAGINATION_SAFETY_CAP: usize = 10_000;
const RPC_PAGE_LIST_KEYS: [&str; 4] = ["items", "tags", "results", "data"];

pub(crate) fn paginate_rpc(
    client: &mut impl RpcCaller,
    method: &str,
    params: Value,
    page_size: usize,
) -> Result<Value, String> {
    let base = params
        .as_object()
        .ok_or_else(|| "params must be a JSON object".to_string())?;
    let mut out = Vec::new();
    let mut prev_page: Option<Vec<Value>> = None;
    let mut offset = 0usize;

    loop {
        let mut page_params = base.clone();
        page_params.insert("offset".to_string(), Value::Number(offset.into()));
        page_params.insert("limit".to_string(), Value::Number(page_size.into()));
        let response = client.call(method, Some(Value::Object(page_params)))?;

        let page = match extract_page(&response) {
            Some(page) => page,
            None if out.is_empty() => return Ok(response),
            None if response.is_object() => {
                return Err(format!(
                    "paginate: {method:?} returned a non-paginated dict after {} accumulated rows; aborting",
                    out.len()
                ));
            }
            None => {
                return Err(format!(
                    "paginate: {method:?} returned non-list/non-dict shape after {} accumulated rows; aborting",
                    out.len()
                ));
            }
        };

        if prev_page.as_ref() == Some(&page) {
            return Err(format!(
                "paginate: {method:?} returned identical pages — method likely ignores offset; aborting after {} rows",
                out.len()
            ));
        }

        let page_len = page.len();
        out.extend(page.clone());
        if page_len < page_size {
            return Ok(Value::Array(out));
        }
        if out.len() >= RPC_PAGINATION_SAFETY_CAP {
            out.truncate(RPC_PAGINATION_SAFETY_CAP);
            return Ok(Value::Array(out));
        }
        prev_page = Some(page);
        offset += page_size;
    }
}

fn extract_page(response: &Value) -> Option<Vec<Value>> {
    if let Some(page) = response.as_array() {
        return Some(page.clone());
    }
    let object = response.as_object()?;
    for key in RPC_PAGE_LIST_KEYS {
        if let Some(page) = object.get(key).and_then(Value::as_array) {
            return Some(page.clone());
        }
    }
    None
}

fn run_find_pdfs_command(
    client: &mut impl RpcCaller,
    collection: String,
    limit: usize,
) -> Result<Value, String> {
    let collection_key = resolve_collection(client, &collection)?;
    let response = client.call(
        "collections.getItems",
        Some(serde_json::json!({"key": collection_key})),
    )?;
    let items = collection_items(&response);

    let mut missing = Vec::new();
    for item in &items {
        let Some(item_key) = item.get("key").and_then(Value::as_str) else {
            continue;
        };
        let attachments = client.call(
            "attachments.list",
            Some(serde_json::json!({"parentKey": item_key})),
        )?;
        if !has_pdf_attachment(&attachments) {
            missing.push(item.clone());
        }
        if limit > 0 && missing.len() >= limit {
            break;
        }
    }

    let mut results = Vec::new();
    for item in &missing {
        let item_key = item
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing item lacks key".to_string())?;
        let response = client.call(
            "attachments.findPDF",
            Some(serde_json::json!({"parentKey": item_key})),
        )?;
        let attachment = response.get("attachment").filter(|value| !value.is_null());
        results.push(serde_json::json!({
            "item_key": item_key,
            "title": item.get("title").cloned().unwrap_or(Value::Null),
            "found": attachment.is_some(),
            "attachment_key": attachment
                .and_then(|attachment| attachment.get("key"))
                .cloned()
                .unwrap_or(Value::Null),
        }));
    }

    Ok(serde_json::json!({
        "scanned": items.len(),
        "attempted": missing.len(),
        "results": results,
    }))
}

pub(crate) fn collection_items(response: &Value) -> Vec<Value> {
    if let Some(items) = response.get("items").and_then(Value::as_array) {
        return items.clone();
    }
    response.as_array().cloned().unwrap_or_default()
}

fn has_pdf_attachment(attachments: &Value) -> bool {
    attachments
        .as_array()
        .is_some_and(|attachments| attachments.iter().any(is_pdf_attachment))
}

pub(crate) fn is_pdf_attachment(attachment: &Value) -> bool {
    let content_type = attachment
        .get("contentType")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let path = attachment
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    matches!(
        content_type.as_str(),
        "application/pdf" | "application/x-pdf"
    ) || path.ends_with(".pdf")
}

pub(crate) fn run_system_command(
    command: SystemCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        SystemCommand::Version => client.call("system.version", None)?,
        SystemCommand::Libraries => client.call("system.libraries", None)?,
        SystemCommand::LibraryStats { library, .. } => {
            let params = library.map(|id| serde_json::json!({"id": id}));
            client.call("system.libraryStats", params)?
        }
        SystemCommand::Schema { item_type, .. } => {
            if let Some(item_type) = item_type {
                let fields = client.call("system.itemFields", Some(serde_json::json!({"itemType": item_type})))?;
                let creators = client.call("system.creatorTypes", Some(serde_json::json!({"itemType": item_type})))?;
                let field_names: Vec<Value> = fields.as_array().unwrap_or(&vec![])
                    .iter()
                    .filter_map(|f| f.get("field").cloned())
                    .collect();
                let creator_names: Vec<Value> = creators.as_array().unwrap_or(&vec![])
                    .iter()
                    .filter_map(|c| c.get("creatorType").cloned())
                    .collect();
                serde_json::json!({
                    "itemType": item_type,
                    "fields": field_names,
                    "creatorTypes": creator_names,
                })
            } else {
                let types = client.call("system.itemTypes", None)?;
                let type_names: Vec<Value> = types.as_array().unwrap_or(&vec![])
                    .iter()
                    .filter_map(|t| t.get("itemType").cloned())
                    .collect();
                Value::Array(type_names)
            }
        }
        SystemCommand::CurrentCollection => client.call("system.currentCollection", None)?,
        SystemCommand::Methods { method, .. } => {
            if let Some(method) = method {
                client.call("system.describe", Some(serde_json::json!({"method": method})))?
            } else {
                client.call("system.listMethods", None)?
            }
        }
    };
    Ok(value)
}

pub(crate) fn run_items_command(
    command: ItemsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        ItemsCommand::Add {
            doi,
            isbn,
            from_url,
            file,
            item_type,
            fields,
            collection,
            dry_run,
            ..
        } => {
            if let Some(doi) = doi {
                run_add_identifier_command(client, "items.addByDOI", "doi", doi, collection, dry_run)?
            } else if let Some(isbn) = isbn {
                run_add_identifier_command(client, "items.addByISBN", "isbn", isbn, collection, dry_run)?
            } else if let Some(from_url) = from_url {
                run_add_identifier_command(client, "items.addByURL", "url", from_url, collection, dry_run)?
            } else if let Some(file) = file {
                let mut params = serde_json::json!({"path": zotero_path(&file)});
                maybe_insert_collection(client, &mut params, collection)?;
                run_mutation_command(client, "items.addFromFile", params, dry_run)?
            } else if let Some(item_type) = item_type {
                let parsed_fields = parse_field_options(&fields)?;
                let mut params = serde_json::json!({"itemType": item_type});
                if !parsed_fields.is_empty() {
                    if let Some(map) = params.as_object_mut() {
                        map.insert("fields".to_string(), Value::Object(parsed_fields));
                    }
                }
                run_mutation_command(client, "items.create", params, dry_run)?
            } else {
                return Err("INVALID_ARGS: provide one of --doi, --isbn, --from-url, --file, or --type".into());
            }
        }
        ItemsCommand::Update {
            key,
            fields,
            dry_run,
            ..
        } => {
            let parsed_fields = parse_field_options(&fields)?;
            let mut params = serde_json::json!({"key": key});
            if !parsed_fields.is_empty() {
                if let Some(map) = params.as_object_mut() {
                    map.insert("fields".to_string(), Value::Object(parsed_fields));
                }
            }
            run_mutation_command(client, "items.update", params, dry_run)?
        }
        ItemsCommand::Delete { key, dry_run, .. } => run_mutation_command(
            client,
            "items.delete",
            serde_json::json!({"key": key}),
            dry_run,
        )?,
        ItemsCommand::Trash {
            items, dry_run, ..
        } => {
            if items.len() == 1 {
                run_mutation_command(
                    client,
                    "items.trash",
                    serde_json::json!({"key": items[0]}),
                    dry_run,
                )?
            } else {
                run_mutation_command(
                    client,
                    "items.batchTrash",
                    serde_json::json!({"keys": items}),
                    dry_run,
                )?
            }
        }
        ItemsCommand::Restore { item, dry_run, .. } => run_mutation_command(
            client,
            "items.restore",
            serde_json::json!({"key": item}),
            dry_run,
        )?,
        ItemsCommand::MergeDuplicates { keys, dry_run, .. } => {
            if keys.len() < 2 {
                return Err("INVALID_ARGS: need at least 2 keys to merge".to_string());
            }
            run_mutation_command(
                client,
                "items.mergeDuplicates",
                serde_json::json!({"keys": keys}),
                dry_run,
            )?
        }
        ItemsCommand::AddRelated {
            key,
            target,
            dry_run,
            ..
        } => run_mutation_command(
            client,
            "items.addRelated",
            serde_json::json!({"key": key, "targetKey": target}),
            dry_run,
        )?,
        ItemsCommand::RemoveRelated {
            key,
            target,
            dry_run,
            ..
        } => run_mutation_command(
            client,
            "items.removeRelated",
            serde_json::json!({"key": key, "targetKey": target}),
            dry_run,
        )?,
        ItemsCommand::Get { item, .. } => client.call("items.get", Some(serde_json::json!({"key": item})))?,
        ItemsCommand::List {
            limit,
            offset,
            sort,
            direction,
            trash,
            ..
        } => {
            if trash {
                let value = client.call(
                    "items.getTrash",
                    Some(serde_json::json!({"limit": limit, "offset": offset})),
                )?;
                normalize_list_envelope(value, "items", Some(limit), offset)
            } else {
                let mut params = serde_json::json!({
                    "limit": limit,
                    "offset": offset,
                    "direction": direction,
                });
                if let (Some(sort), Some(map)) = (sort, params.as_object_mut()) {
                    map.insert("sort".to_string(), Value::String(sort));
                }
                let value = client.call("items.list", Some(params))?;
                normalize_list_envelope(value, "items", Some(limit), offset)
            }
        }
        ItemsCommand::FindDuplicates => client.call("items.findDuplicates", None)?,
        ItemsCommand::Recent {
            limit,
            offset,
            recent_type,
            ..
        } => {
            if recent_type != "added" && recent_type != "modified" {
                return Err(format!(
                    "--type must be added or modified, got {recent_type:?}"
                ));
            }
            let value = client.call(
                "items.getRecent",
                Some(
                    serde_json::json!({"limit": limit, "offset": offset, "type": recent_type}),
                ),
            )?;
            normalize_list_envelope(value, "items", Some(limit), offset)
        }
        ItemsCommand::Fulltext { key, ocr, .. } => {
            // Default: prefer the clean OCR sidecar text, fall back to Zotero's
            // built-in PDF extraction when the item hasn't been OCR'd. `--ocr`
            // forces OCR-only and errors if no sidecar exists.
            match ocr_fulltext(client, &key)? {
                Some(value) => value,
                None if ocr => {
                    return Err(format!(
                        "NO_OCR_DATA: no OCR sidecar found for item {key} — run `zotron ocr process --parent {key}` first"
                    ));
                }
                None => {
                    let mut value = client
                        .call("items.getFullText", Some(serde_json::json!({"key": key})))?;
                    if let Some(map) = value.as_object_mut() {
                        map.insert("source".to_string(), Value::String("zotero".to_string()));
                    }
                    value
                }
            }
        }
        ItemsCommand::Related { key, .. } => normalize_list_envelope(
            client.call("items.getRelated", Some(serde_json::json!({"key": key})))?,
            "items",
            None,
            0,
        ),
        ItemsCommand::CitationKey { key, .. } => client.call("items.citationKey", Some(serde_json::json!({"key": key})))?,
        ItemsCommand::Path { key, .. } => localize_attachment_path_response(
            client.call("attachments.getPath", Some(serde_json::json!({"key": key})))?,
        ),
        ItemsCommand::Attachments { key, offset, .. } => {
            let value = client.call(
                "attachments.list",
                Some(serde_json::json!({"parentKey": key})),
            )?;
            let total = value
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len()) as u64;
            normalize_list_envelope(value, "items", Some(total), offset)
        }
        ItemsCommand::FindPdfs { collection, limit, .. } => {
            run_find_pdfs_command(client, collection, limit)?
        }
    };
    Ok(value)
}

fn run_add_identifier_command(
    client: &mut impl RpcCaller,
    method: &str,
    param_name: &str,
    param_value: String,
    collection: Option<String>,
    dry_run: bool,
) -> Result<Value, String> {
    let mut params = Value::Object(serde_json::Map::from_iter([(
        param_name.to_string(),
        Value::String(param_value),
    )]));
    maybe_insert_collection(client, &mut params, collection)?;
    run_mutation_command(client, method, params, dry_run)
}

fn run_mutation_command(
    client: &mut impl RpcCaller,
    method: &str,
    params: Value,
    dry_run: bool,
) -> Result<Value, String> {
    if dry_run {
        Ok(dry_run_value(method, params))
    } else {
        client.call(method, Some(params))
    }
}

fn parse_field_options(fields: &[String]) -> Result<serde_json::Map<String, Value>, String> {
    let mut parsed = serde_json::Map::new();
    for field in fields {
        let (key, value) = field
            .split_once('=')
            .ok_or_else(|| format!("INVALID_ARGS: --field must be key=value, got: {field:?}"))?;
        parsed.insert(key.to_string(), Value::String(value.to_string()));
    }
    Ok(parsed)
}

fn maybe_insert_collection(
    client: &mut impl RpcCaller,
    params: &mut Value,
    collection: Option<String>,
) -> Result<(), String> {
    let Some(collection) = collection else {
        return Ok(());
    };
    let collection = resolve_collection(client, &collection)?;
    let include = match &collection {
        Value::Null => false,
        Value::Number(number) => number.as_i64() != Some(0),
        _ => true,
    };
    if include {
        params
            .as_object_mut()
            .expect("mutation params are always objects")
            .insert("collection".to_string(), collection);
    }
    Ok(())
}

pub(crate) fn run_settings_command(
    command: SettingsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        SettingsCommand::Get { key, .. } => client.call("settings.get", Some(serde_json::json!({"key": key})))?,
        SettingsCommand::List => client.call("settings.getAll", None)?,
        SettingsCommand::Set {
            pairs,
            file,
            dry_run,
            ..
        } => {
            if let Some(file) = file {
                // --file mode: read JSON and call settings.setAll
                let raw = fs::read_to_string(&file)
                    .map_err(|err| format!("INVALID_JSON: Could not read JSON: {err}"))?;
                let settings: Value = serde_json::from_str(&raw)
                    .map_err(|err| format!("INVALID_JSON: Could not parse JSON: {err}"))?;
                if dry_run {

                        dry_run_value("settings.setAll", settings)
                } else {

                        client.call("settings.setAll", Some(settings))?
                }
            } else if pairs.len() == 2 {
                // Single key=value: settings.set
                let key = &pairs[0];
                let value = &pairs[1];
                let parsed_value = serde_json::from_str::<Value>(value)
                    .unwrap_or(Value::String(value.clone()));
                let params = serde_json::json!({"key": key, "value": parsed_value});
                if dry_run {

                        dry_run_value("settings.set", params)
                } else {

                        client.call("settings.set", Some(params))?
                }
            } else if pairs.len() > 2 && pairs.len() % 2 == 0 {
                // Multiple pairs: build a map and call settings.setAll
                let mut map = serde_json::Map::new();
                for chunk in pairs.chunks(2) {
                    let parsed = serde_json::from_str::<Value>(&chunk[1])
                        .unwrap_or(Value::String(chunk[1].clone()));
                    map.insert(chunk[0].clone(), parsed);
                }
                let settings = Value::Object(map);
                if dry_run {

                        dry_run_value("settings.setAll", settings)
                } else {

                        client.call("settings.setAll", Some(settings))?
                }
            } else {
                return Err(
                    "INVALID_ARGS: provide key value pairs (even number of args) or --file".into(),
                );
            }
        }
    };
    Ok(value)
}

pub(crate) fn run_tags_command(
    command: TagsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        TagsCommand::List { limit, .. } => {
            let value = client.call("tags.list", Some(serde_json::json!({"limit": limit})))?;
            normalize_list_envelope(value, "items", Some(limit), 0)
        }
        TagsCommand::Rename {
            old, new, dry_run, ..
        } => run_mutation_command(
            client,
            "tags.rename",
            serde_json::json!({"oldName": old, "newName": new}),
            dry_run,
        )?,
        TagsCommand::Delete { tag, dry_run, .. } => run_mutation_command(
            client,
            "tags.delete",
            serde_json::json!({"tag": tag}),
            dry_run,
        )?,
        TagsCommand::Add {
            keys, tags, dry_run, ..
        } => {
            if keys.len() == 1 {
                run_mutation_command(
                    client,
                    "tags.add",
                    serde_json::json!({"key": keys[0], "tags": tags}),
                    dry_run,
                )?
            } else {
                run_mutation_command(
                    client,
                    "tags.batchUpdate",
                    serde_json::json!({"keys": keys, "add": tags}),
                    dry_run,
                )?
            }
        }
        TagsCommand::Remove {
            keys, tags, dry_run, ..
        } => {
            if keys.len() == 1 {
                run_mutation_command(
                    client,
                    "tags.remove",
                    serde_json::json!({"key": keys[0], "tags": tags}),
                    dry_run,
                )?
            } else {
                run_mutation_command(
                    client,
                    "tags.batchUpdate",
                    serde_json::json!({"keys": keys, "remove": tags}),
                    dry_run,
                )?
            }
        }
    };
    Ok(value)
}

fn dry_run_value(method: &str, params: Value) -> Value {
    serde_json::json!({
        "ok": true,
        "dryRun": true,
        "wouldCall": method,
        "wouldCallParams": params,
    })
}

pub(crate) fn run_annotations_command(
    command: AnnotationsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        AnnotationsCommand::List { parent, attachment, context, .. } => {
            let mut params = serde_json::json!({"parentKey": parent});
            if let Some(att) = attachment {
                params["attachmentKey"] = Value::String(att);
            }
            if let Some(ctx) = context {
                params["context"] = Value::Number(ctx.into());
            }
            let value = client.call("annotations.list", Some(params))?;
            let total = value
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len()) as u64;
            normalize_list_envelope(value, "items", Some(total), 0)
        }
        AnnotationsCommand::Create {
            parent,
            attachment,
            annotation_type,
            position,
            quote,
            page,
            sort_index,
            text,
            comment,
            color,
            dry_run,
            ..
        } => {
            let annotation_type = annotation_type.unwrap_or_else(|| "highlight".to_string());
            if !matches!(
                annotation_type.as_str(),
                "highlight" | "note" | "underline" | "image" | "ink"
            ) {
                return Err(format!(
                    "INVALID_ARGS: --type must be highlight|note|underline|image|ink, got {annotation_type:?}"
                ));
            }
            let mut params = serde_json::Map::new();
            params.insert("parentKey".to_string(), Value::String(parent));
            if let Some(att) = attachment {
                params.insert("attachmentKey".to_string(), Value::String(att));
            }
            params.insert("type".to_string(), Value::String(annotation_type.clone()));
            params.insert("color".to_string(), Value::String(color));

            if let Some(ref quote_text) = quote {
                if !matches!(annotation_type.as_str(), "highlight" | "underline") {
                    return Err(format!(
                        "INVALID_ARGS: --quote is only valid for highlight|underline, got {annotation_type:?}"
                    ));
                }
                params.insert("quote".to_string(), Value::String(quote_text.clone()));
                if let Some(page_idx) = page {
                    params.insert(
                        "pageIndex".to_string(),
                        Value::Number(page_idx.into()),
                    );
                }
                // When --quote is given, --position is optional
                if let Some(raw) = position {
                    let pos = serde_json::from_str::<Value>(&raw)
                        .map_err(|err| format!("INVALID_JSON: Could not parse --position: {err}"))?;
                    validate_annotation_position(annotation_type.as_str(), &pos)?;
                    params.insert("position".to_string(), pos);
                }
            } else {
                let position = position
                    .ok_or_else(|| "INVALID_ARGS: --position JSON is required (or use --quote)".to_string())
                    .and_then(|raw| {
                        serde_json::from_str::<Value>(&raw)
                            .map_err(|err| format!("INVALID_JSON: Could not parse --position: {err}"))
                    })?;
                validate_annotation_position(annotation_type.as_str(), &position)?;
                params.insert("position".to_string(), position);
            }

            if let Some(sort_index) = sort_index {
                params.insert(
                    "sortIndex".to_string(),
                    parse_annotation_sort_index(sort_index)?,
                );
            }
            if let Some(text) = text {
                params.insert("text".to_string(), Value::String(text));
            }
            if let Some(comment) = comment {
                params.insert("comment".to_string(), Value::String(comment));
            }
            run_mutation_command(client, "annotations.create", Value::Object(params), dry_run)?
        }
        AnnotationsCommand::CreateBatch {
            parent,
            attachment,
            file,
            dry_run,
            ..
        } => {
            let input = if let Some(ref path) = file {
                std::fs::read_to_string(path)
                    .map_err(|e| format!("INVALID_ARGS: cannot read file {path}: {e}"))?
            } else {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                    .map_err(|e| format!("INVALID_ARGS: cannot read stdin: {e}"))?;
                buf
            };
            let annotations: Value = serde_json::from_str(&input)
                .map_err(|e| format!("INVALID_JSON: {e}"))?;
            if !annotations.is_array() {
                return Err("INVALID_ARGS: input must be a JSON array".to_string());
            }
            let mut params = serde_json::json!({
                "parentKey": parent,
                "annotations": annotations,
            });
            if let Some(att) = attachment {
                params["attachmentKey"] = Value::String(att);
            }
            run_mutation_command(client, "annotations.createBatch", params, dry_run)?
        }
        AnnotationsCommand::Locate {
            parent,
            attachment,
            quote,
            page,
            ..
        } => {
            let mut params = serde_json::json!({
                "parentKey": parent,
                "quote": quote,
            });
            if let Some(att) = attachment {
                params["attachmentKey"] = Value::String(att);
            }
            if let Some(page_idx) = page {
                params["pageIndex"] = Value::Number(page_idx.into());
            }
            client.call("annotations.locate", Some(params))?
        }
        AnnotationsCommand::Delete {
            annotation_key,
            dry_run,
            ..
        } => run_mutation_command(
            client,
            "annotations.delete",
            serde_json::json!({"key": annotation_key}),
            dry_run,
        )?,
    };
    Ok(value)
}

fn validate_annotation_position(annotation_type: &str, position: &Value) -> Result<(), String> {
    position
        .get("pageIndex")
        .and_then(Value::as_i64)
        .filter(|value| *value >= 0)
        .ok_or_else(|| {
            "INVALID_ARGS: --position must include a non-negative integer pageIndex".to_string()
        })?;

    if annotation_type == "ink" {
        let has_paths = position
            .get("paths")
            .and_then(Value::as_array)
            .is_some_and(|paths| !paths.is_empty());
        if !has_paths {
            return Err("INVALID_ARGS: ink --position must include non-empty paths".to_string());
        }
        return Ok(());
    }

    let valid_rects = position
        .get("rects")
        .and_then(Value::as_array)
        .is_some_and(|rects| !rects.is_empty() && rects.iter().all(is_annotation_rect));
    if !valid_rects {
        return Err(
            "INVALID_ARGS: --position must include non-empty rects of [x1, y1, x2, y2]".to_string(),
        );
    }
    Ok(())
}

fn is_annotation_rect(value: &Value) -> bool {
    value.as_array().is_some_and(|coords| {
        coords.len() == 4
            && coords
                .iter()
                .all(|coord| coord.as_f64().is_some_and(f64::is_finite))
    })
}

fn parse_annotation_sort_index(raw: String) -> Result<Value, String> {
    let parsed = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| Value::String(raw));
    let valid = match &parsed {
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        Value::String(value) => {
            is_zotero_pdf_sort_index(value.trim())
                || (!value.trim().is_empty()
                    && value.trim().parse::<f64>().is_ok_and(f64::is_finite))
        }
        _ => false,
    };
    if valid {
        Ok(parsed)
    } else {
        Err(format!(
            "INVALID_ARGS: --sort-index must be a finite number or numeric string, got {parsed}"
        ))
    }
}

fn is_zotero_pdf_sort_index(value: &str) -> bool {
    let mut parts = value.split('|');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(page), Some(offset), Some(y), None)
            if page.len() == 5
                && offset.len() == 6
                && y.len() == 5
                && page.chars().all(|ch| ch.is_ascii_digit())
                && offset.chars().all(|ch| ch.is_ascii_digit())
                && y.chars().all(|ch| ch.is_ascii_digit())
    )
}


/// Read the clean OCR sidecar text for an item's PDF, preferring native
/// markdown and falling back to assembled blocks. Returns `Ok(None)` when the
/// item simply has no PDF/OCR data yet (the caller may fall back to Zotero's
/// built-in extraction); `Err` is reserved for hard RPC/IO failures.
fn ocr_fulltext(client: &mut impl RpcCaller, key: &str) -> Result<Option<Value>, String> {
    use crate::ocr::{resolve_attachment_path, resolve_first_pdf_attachment_key};
    use zotron_types::{machine_artifact_sidecar_absolute_path, MachineArtifactKind, PdfEvidenceBlock};

    let att_key = match resolve_first_pdf_attachment_key(client, key) {
        Ok(att_key) => att_key,
        // No PDF attachment → no OCR possible; signal fall-back, not an error.
        Err(err) if err.starts_with("NO_PDF_ATTACHMENT") => return Ok(None),
        Err(err) => return Err(err),
    };
    let att_path = resolve_attachment_path(client, &att_key)?;
    let storage_dir = att_path.parent().ok_or_else(|| {
        format!("ATTACHMENT_PATH_INVALID: no parent dir: {}", att_path.display())
    })?;

    let md_path = machine_artifact_sidecar_absolute_path(storage_dir, MachineArtifactKind::OcrNativeMarkdown);
    if md_path.exists() {
        let content = fs::read_to_string(&md_path)
            .map_err(|e| format!("READ_FAILED: {}: {e}", md_path.display()))?;
        return Ok(Some(serde_json::json!({
            "key": key,
            "source": "ocr_markdown",
            "content": content,
            "totalChars": content.len(),
        })));
    }

    let blocks_path = machine_artifact_sidecar_absolute_path(storage_dir, MachineArtifactKind::Blocks);
    if blocks_path.exists() {
        let raw = fs::read_to_string(&blocks_path)
            .map_err(|e| format!("READ_FAILED: {}: {e}", blocks_path.display()))?;
        let content: String = raw.lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str::<PdfEvidenceBlock>(line).ok())
            .map(|b| b.text)
            .filter(|t| !t.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");
        return Ok(Some(serde_json::json!({
            "key": key,
            "source": "ocr_blocks",
            "content": content,
            "totalChars": content.len(),
        })));
    }

    Ok(None)
}

fn localize_attachment_path_response(mut value: Value) -> Value {
    if let Some(path) = value.get("path").and_then(Value::as_str) {
        let local = local_path_from_zotero_path(path);
        if let Some(map) = value.as_object_mut() {
            map.insert("path".to_string(), Value::String(local));
        }
    }
    value
}

pub(crate) fn run_notes_command(
    command: NotesCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        NotesCommand::List {
            parent,
            limit,
            offset,
            ..
        } => {
            let value = client.call(
                "notes.list",
                Some(serde_json::json!({"parentKey": parent})),
            )?;
            normalize_list_envelope(value, "items", Some(limit), offset)
        }
        NotesCommand::Get { note_key, .. } => {
            client.call("notes.get", Some(serde_json::json!({"key": note_key})))?
        }
        NotesCommand::Create {
            parent,
            content,
            tags,
            dry_run,
            ..
        } => {
            let mut params = serde_json::Map::new();
            params.insert("parentKey".to_string(), Value::String(parent));
            params.insert("content".to_string(), Value::String(content));
            if !tags.is_empty() {
                params.insert(
                    "tags".to_string(),
                    Value::Array(tags.into_iter().map(Value::String).collect()),
                );
            }
            run_mutation_command(client, "notes.create", Value::Object(params), dry_run)?
        }
        NotesCommand::Update {
            note_key,
            content,
            dry_run,
            ..
        } => run_mutation_command(
            client,
            "notes.update",
            serde_json::json!({"key": note_key, "content": content}),
            dry_run,
        )?,
        NotesCommand::Delete {
            note_key, dry_run, ..
        } => {
            // Python CLI intentionally routes note deletion through items.delete.
            run_mutation_command(
                client,
                "items.delete",
                serde_json::json!({"key": note_key}),
                dry_run,
            )?
        }
        NotesCommand::Search { query, limit, .. } => {
            let value = client.call(
                "notes.search",
                Some(serde_json::json!({"query": query, "limit": limit})),
            )?;
            normalize_list_envelope(value, "items", Some(limit), 0)
        }
    };
    Ok(value)
}

pub(crate) fn run_collections_command(
    command: CollectionsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        CollectionsCommand::List => normalize_list_envelope(
            client.call("collections.list", None)?,
            "items",
            None,
            0,
        ),
        CollectionsCommand::Tree => client.call("collections.tree", None)?,
        CollectionsCommand::Get { name_or_id, .. } => {
            let key = resolve_collection(client, &name_or_id)?;
            client.call("collections.get", Some(serde_json::json!({"key": key})))?
        }
        CollectionsCommand::GetItems {
            name_or_id,
            limit,
            offset,
            ..
        } => {
            let key = resolve_collection(client, &name_or_id)?;
            let mut params = serde_json::json!({"key": key});
            if let Some(map) = params.as_object_mut() {
                if let Some(limit) = limit {
                    map.insert("limit".to_string(), Value::Number(limit.into()));
                }
                if offset > 0 {
                    map.insert("offset".to_string(), Value::Number(offset.into()));
                }
            }
            normalize_list_envelope(
                client.call("collections.getItems", Some(params))?,
                "items",
                limit,
                offset,
            )
        }
        CollectionsCommand::Stats { name_or_id, .. } => {
            let key = resolve_collection(client, &name_or_id)?;
            client.call("collections.stats", Some(serde_json::json!({"key": key})))?
        }
        CollectionsCommand::Rename {
            old_name,
            new_name,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &old_name, "rename")?;
            let params = serde_json::json!({"key": key, "name": new_name});
            if dry_run {
                return Ok(dry_run_value("collections.rename", params));
            }
            return client.call("collections.rename", Some(params));
        }
        CollectionsCommand::Create {
            name,
            parent,
            dry_run,
            ..
        } => {
            let mut params = serde_json::json!({"name": name});
            if let Some(parent) = parent {
                let parent_key = resolve_mutable_collection(client, &parent, "use as parent")?;
                if let Some(map) = params.as_object_mut() {
                    map.insert("parentKey".to_string(), parent_key);
                }
            }
            if dry_run {
                return Ok(dry_run_value("collections.create", params));
            }
            return client.call("collections.create", Some(params));
        }
        CollectionsCommand::Delete {
            name_or_id,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &name_or_id, "delete")?;
            let params = serde_json::json!({"key": key});
            if dry_run {
                return Ok(dry_run_value("collections.delete", params));
            }
            return client.call("collections.delete", Some(params));
        }
        CollectionsCommand::AddItems {
            collection,
            item_keys,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &collection, "add to")?;
            let params = serde_json::json!({"key": key, "keys": item_keys});
            if dry_run {
                return Ok(dry_run_value("collections.addItems", params));
            }
            return client.call("collections.addItems", Some(params));
        }
        CollectionsCommand::RemoveItems {
            collection,
            item_keys,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &collection, "operate on")?;
            let params = serde_json::json!({"key": key, "keys": item_keys});
            if dry_run {
                return Ok(dry_run_value("collections.removeItems", params));
            }
            return client.call("collections.removeItems", Some(params));
        }
    };
    Ok(value)
}

fn resolve_export_keys(
    client: &mut impl RpcCaller,
    mut keys: Vec<String>,
    collection: Option<String>,
) -> Result<Vec<String>, String> {
    if let Some(name) = collection {
        let col_key = resolve_collection(client, &name)?;
        let response = client.call(
            "collections.getItems",
            Some(serde_json::json!({"key": col_key})),
        )?;
        let items = collection_items(&response);
        for item in items {
            if let Some(key) = item.get("key").and_then(Value::as_str) {
                if !keys.contains(&key.to_string()) {
                    keys.push(key.to_string());
                }
            }
        }
    }
    if keys.is_empty() {
        return Err("No item keys provided. Pass positional keys and/or --collection.".to_string());
    }
    Ok(keys)
}

pub(crate) fn run_export(args: ExportArgs, client: &mut impl RpcCaller) -> Result<String, String> {
    let keys = resolve_export_keys(client, args.keys, args.collection)?;
    match args.format.as_str() {
        "bibtex" => run_export_content_command(client, "export.bibtex", keys),
        "ris" => run_export_content_command(client, "export.ris", keys),
        "csl-json" => {
            let response =
                client.call("export.cslJson", Some(serde_json::json!({"keys": keys})))?;
            if let Some(content) = response.get("content") {
                format_json(content)
            } else {
                format_json(&response)
            }
        }
        "bibliography" => {
            let response = client.call(
                "export.bibliography",
                Some(serde_json::json!({"keys": keys, "style": args.style})),
            )?;
            if let Some(object) = response.as_object() {
                let field = if args.html { "html" } else { "text" };
                if object.contains_key("html") || object.contains_key("text") {
                    return raw_value_output(
                        object.get(field).unwrap_or(&Value::String(String::new())),
                    );
                }
            }
            format_json(&response)
        }
        other => Err(format!(
            "INVALID_ARGS: unknown format {other:?}, expected bibtex/ris/csl-json/bibliography"
        )),
    }
}

fn run_export_content_command(
    client: &mut impl RpcCaller,
    method: &str,
    keys: Vec<String>,
) -> Result<String, String> {
    let response = client.call(method, Some(serde_json::json!({"keys": keys})))?;
    if let Some(content) = response.get("content") {
        raw_value_output(content)
    } else {
        format_json(&response)
    }
}

pub(crate) fn resolve_collection(
    client: &mut impl RpcCaller,
    name_or_id: &str,
) -> Result<Value, String> {
    let trimmed = name_or_id.trim();
    if let Ok(id) = trimmed.parse::<i64>() {
        return Ok(Value::Number(id.into()));
    }

    let collections = client.call("collections.list", None)?;
    let items = collections
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| collections.as_array())
        .ok_or_else(|| "collections.list returned non-array result".to_string())?;

    if let Some(collection) = items
        .iter()
        .find(|collection| collection.get("key").and_then(Value::as_str) == Some(trimmed))
    {
        return collection_key(collection);
    }

    let exact = items
        .iter()
        .filter(|collection| collection.get("name").and_then(Value::as_str) == Some(trimmed))
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return collection_key(exact[0]);
    }

    let needle = normalize_collection_name(trimmed);
    let fuzzy = items
        .iter()
        .filter(|collection| {
            collection
                .get("name")
                .and_then(Value::as_str)
                .map(normalize_collection_name)
                .is_some_and(|name| name.contains(&needle))
        })
        .collect::<Vec<_>>();

    match fuzzy.len() {
        1 => collection_key(fuzzy[0]),
        0 => Err(format!(
            "COLLECTION_NOT_FOUND: No collection named {trimmed:?}"
        )),
        _ => Err(format!(
            "COLLECTION_AMBIGUOUS: Multiple collections match {trimmed:?}"
        )),
    }
}

fn collection_key(collection: &Value) -> Result<Value, String> {
    collection
        .get("key")
        .cloned()
        .ok_or_else(|| "collection result is missing key".to_string())
}

fn resolve_mutable_collection(
    client: &mut impl RpcCaller,
    name_or_id: &str,
    operation: &str,
) -> Result<Value, String> {
    let key = resolve_collection(client, name_or_id)?;
    if key.as_i64() == Some(0) {
        return Err(format!(
            "COLLECTION_NOT_FOUND: {name_or_id:?} resolved to library root (cannot {operation})"
        ));
    }
    Ok(key)
}

fn normalize_collection_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
