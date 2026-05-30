//! Output formatting helpers: JSON serialization, list-envelope normalization,
//! raw value passthrough, and structured error JSON.

use serde_json::Value;

pub fn format_error_json(message: &str) -> String {
    let message = message.trim_end();
    let (code, message) = split_error_code(message).unwrap_or(("RUNTIME_ERROR", message));
    serde_json::json!({"error": {"code": code, "message": message}}).to_string()
}

pub(crate) fn split_error_code(message: &str) -> Option<(&str, &str)> {
    let (code, rest) = message.split_once(':')?;
    if !code.is_empty()
        && code
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_')
    {
        Some((code, rest.trim_start()))
    } else {
        None
    }
}

pub(crate) fn normalize_list_envelope(
    value: Value,
    list_key: &str,
    limit: Option<u64>,
    offset: u64,
) -> Value {
    if let Value::Array(arr) = value {
        let total = arr.len() as u64;
        let mut obj = serde_json::Map::new();
        obj.insert(list_key.to_string(), Value::Array(arr));
        obj.insert("total".to_string(), Value::from(total));
        if let Some(limit) = limit {
            obj.insert("limit".to_string(), Value::from(limit));
        }
        obj.insert("offset".to_string(), Value::from(offset));
        obj.insert("hasMore".to_string(), Value::Bool(false));
        return Value::Object(obj);
    }

    let mut obj = match value {
        Value::Object(obj) if obj.contains_key(list_key) => obj,
        other => return other,
    };

    let items_len = obj
        .get(list_key)
        .and_then(Value::as_array)
        .map_or(0, |a| a.len()) as u64;
    let total = obj
        .get("total")
        .and_then(Value::as_u64)
        .unwrap_or(items_len);

    obj.insert("total".to_string(), Value::from(total));
    if let Some(limit) = limit {
        obj.insert("limit".to_string(), Value::from(limit));
    }
    obj.insert("offset".to_string(), Value::from(offset));
    obj.insert(
        "hasMore".to_string(),
        Value::Bool(offset + items_len < total),
    );

    Value::Object(obj)
}

pub(crate) fn raw_value_output(value: &Value) -> Result<String, String> {
    // Raw export content (bibtex/ris) arrives as a JSON string and must stay
    // verbatim. Any other value type is serialized as compact JSON.
    let mut out = match value {
        Value::Null => String::new(),
        Value::String(content) => content.clone(),
        other => serde_json::to_string(other).map_err(|e| e.to_string())?,
    };
    out.push('\n');
    Ok(out)
}

pub(crate) fn format_json(value: &Value) -> Result<String, String> {
    let mut out = serde_json::to_string(value).map_err(|e| e.to_string())?;
    out.push('\n');
    Ok(out)
}
