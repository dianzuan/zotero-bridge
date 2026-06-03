//! PDF evidence/chunk types and OCR-response-to-blocks parsing.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::chunking::{is_heading_type, is_table_type, normalize_block_type};
use crate::providers::ocr_provider_spec;

/// Normalized structure block emitted by PDF parsers/OCR providers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfEvidenceBlock {
    pub block_key: String,
    pub item_key: String,
    pub attachment_key: String,
    pub page_idx: u64,
    pub block_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(default)]
    pub section_path: Vec<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfEvidenceRef {
    pub block_key: String,
    pub page_idx: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
}

/// Structure-aware retrieval chunk. It preserves block provenance and avoids
/// legacy public `*_id` fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructureChunk {
    pub chunk_key: String,
    pub item_key: String,
    pub attachment_key: String,
    pub block_keys: Vec<String>,
    #[serde(default)]
    pub section_path: Vec<String>,
    pub text: String,
    pub page_range: [u64; 2],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_start: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_end: Option<u64>,
    #[serde(default)]
    pub evidence_refs: Vec<PdfEvidenceRef>,
}

impl StructureChunk {
    pub fn from_blocks(chunk_key: impl Into<String>, blocks: &[PdfEvidenceBlock]) -> Self {
        let chunk_key = chunk_key.into();
        let first = blocks.first();
        let item_key = first.map(|b| b.item_key.clone()).unwrap_or_default();
        let attachment_key = first.map(|b| b.attachment_key.clone()).unwrap_or_default();
        let page_start = blocks.iter().map(|b| b.page_idx).min().unwrap_or(0);
        let page_end = blocks
            .iter()
            .map(|b| b.page_idx)
            .max()
            .unwrap_or(page_start);
        let section_path = first.map(|b| b.section_path.clone()).unwrap_or_default();
        let body = blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");
        let section = &blocks[0].section_path;
        let text = if section.is_empty() {
            body
        } else {
            format!("{}: {}", section.join(" > "), body)
        };

        Self {
            chunk_key,
            item_key,
            attachment_key,
            block_keys: blocks.iter().map(|b| b.block_key.clone()).collect(),
            section_path,
            text,
            page_range: [page_start, page_end],
            page_start: Some(page_start),
            page_end: Some(page_end),
            evidence_refs: blocks
                .iter()
                .map(|block| PdfEvidenceRef {
                    block_key: block.block_key.clone(),
                    page_idx: block.page_idx,
                    bbox: block.bbox,
                })
                .collect(),
        }
    }
}

/// Extract provider-native Markdown from known OCR response shapes.
///
/// Markdown is persisted as an audit/convenience artifact only; normalized
/// blocks remain the retrieval source of truth when structured payloads exist.
pub fn provider_native_markdown(payload: &Value) -> Option<String> {
    if let Some(markdown) = payload.get("md_results").and_then(Value::as_str) {
        return non_empty_string(markdown);
    }
    if let Some(markdown) = payload.get("markdown").and_then(Value::as_str) {
        return non_empty_string(markdown);
    }
    if let Some(markdown) = payload.pointer("/markdown/text").and_then(Value::as_str) {
        return non_empty_string(markdown);
    }
    if let Some(results) = payload
        .pointer("/result/layoutParsingResults")
        .and_then(Value::as_array)
    {
        let parts = results
            .iter()
            .filter_map(|result| result.pointer("/markdown/text").and_then(Value::as_str))
            .filter(|text| !text.trim().is_empty())
            .collect::<Vec<_>>();
        if !parts.is_empty() {
            return Some(parts.join("\n\n"));
        }
    }
    if let Some(result) = payload.get("result").and_then(Value::as_str) {
        return non_empty_string(result);
    }
    None
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub fn blocks_from_provider_payload(
    payload: &Value,
    item_key: &str,
    attachment_key: &str,
    source_provider: &str,
) -> Vec<PdfEvidenceBlock> {
    let mut blocks = Vec::new();
    let mut section_path = Vec::<String>::new();

    if let Some(pages) = payload.get("pages").and_then(Value::as_array) {
        for page in pages {
            let page_idx = page
                .get("page")
                .and_then(Value::as_u64)
                .or_else(|| page.get("page_idx").and_then(Value::as_u64))
                .unwrap_or(1);
            if let Some(page_blocks) = page.get("blocks").and_then(Value::as_array) {
                push_blocks(
                    &mut blocks,
                    page_blocks,
                    item_key,
                    attachment_key,
                    page_idx,
                    source_provider,
                    &mut section_path,
                );
            }
        }
    } else if let Some(payload_blocks) = payload.get("blocks").and_then(Value::as_array) {
        push_blocks(
            &mut blocks,
            payload_blocks,
            item_key,
            attachment_key,
            1,
            source_provider,
            &mut section_path,
        );
    }

    blocks
}

pub fn parse_ocr_provider_response(
    provider: &str,
    payload: &Value,
    item_key: &str,
    attachment_key: &str,
) -> Result<Vec<PdfEvidenceBlock>, String> {
    let spec = ocr_provider_spec(provider)?;
    let normalized = normalize_ocr_payload(payload);
    let blocks =
        blocks_from_provider_payload(&normalized, item_key, attachment_key, spec.provider_key);

    if blocks.is_empty() {
        Err(format!(
            "OCR provider {} response did not contain parseable blocks",
            spec.provider_key
        ))
    } else {
        Ok(blocks)
    }
}

fn normalize_ocr_payload(payload: &Value) -> Value {
    if payload.get("pages").is_some() || payload.get("blocks").is_some() {
        return payload.clone();
    }

    if let Some(content_list) = payload.get("content_list").and_then(Value::as_array) {
        return json!({ "blocks": content_list });
    }

    if let Some(content_list) = payload.get("content_list_v2").and_then(Value::as_array) {
        return normalize_mineru_content_list_v2(content_list);
    }

    if let Some(data) = payload.get("data") {
        if data.is_object() {
            return normalize_ocr_payload(data);
        }
    }

    if let Some(layout_details) = payload.get("layout_details").and_then(Value::as_array) {
        let pages = layout_details
            .iter()
            .enumerate()
            .filter_map(|(index, page)| {
                let blocks = page.as_array()?.clone();
                Some(json!({
                    "page": index as u64 + 1,
                    "blocks": blocks,
                }))
            })
            .collect::<Vec<_>>();
        return json!({ "pages": pages });
    }

    if let Some(results) = payload
        .pointer("/result/layoutParsingResults")
        .and_then(Value::as_array)
    {
        let pages = results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| {
                let mut blocks = result
                    .pointer("/prunedResult/parsing_res_list")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                if blocks.is_empty() {
                    if let Some(markdown) = result.pointer("/markdown/text").and_then(Value::as_str)
                    {
                        blocks = blocks_from_markdown_text(markdown);
                    }
                }
                (!blocks.is_empty()).then(|| {
                    json!({
                        "page": index as u64 + 1,
                        "blocks": blocks,
                    })
                })
            })
            .collect::<Vec<_>>();
        return json!({ "pages": pages });
    }

    for candidate in [
        payload.pointer("/choices/0/message/content"),
        payload.pointer("/choices/0/delta/content"),
        payload.get("output_text"),
        payload.get("content"),
        payload.get("result"),
    ]
    .into_iter()
    .flatten()
    {
        if let Some(value) = parse_json_string(candidate) {
            return normalize_ocr_payload(&value);
        }
        if let Some(markdown) = candidate.as_str() {
            let blocks = blocks_from_markdown_text(markdown);
            if !blocks.is_empty() {
                return json!({
                    "pages": [{
                        "page": 1,
                        "blocks": blocks,
                    }],
                });
            }
        }
        if candidate.is_object() || candidate.is_array() {
            return normalize_ocr_payload(candidate);
        }
    }

    payload.clone()
}

fn normalize_mineru_content_list_v2(content_list: &[Value]) -> Value {
    let pages = content_list
        .iter()
        .enumerate()
        .filter_map(|(index, page)| {
            let page_idx = page
                .get("page_idx")
                .or_else(|| page.get("page"))
                .and_then(Value::as_u64)
                .unwrap_or(index as u64 + 1);
            let blocks = if let Some(blocks) = page.as_array() {
                blocks.clone()
            } else {
                page.get("blocks")
                    .or_else(|| page.get("items"))
                    .or_else(|| page.get("content"))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            };
            (!blocks.is_empty()).then(|| {
                json!({
                    "page": page_idx,
                    "blocks": blocks,
                })
            })
        })
        .collect::<Vec<_>>();
    json!({ "pages": pages })
}

fn blocks_from_markdown_text(markdown: &str) -> Vec<Value> {
    markdown
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            let heading = trimmed.starts_with('#');
            let text = if heading {
                trimmed.trim_start_matches('#').trim()
            } else {
                trimmed
            };
            if text.is_empty() {
                return None;
            }
            Some(json!({
                "type": if heading { "heading" } else { "text" },
                "text": text,
            }))
        })
        .collect()
}

fn parse_json_string(value: &Value) -> Option<Value> {
    let text = value.as_str()?.trim();
    serde_json::from_str(text).ok()
}

fn push_blocks(
    blocks: &mut Vec<PdfEvidenceBlock>,
    raw_blocks: &[Value],
    item_key: &str,
    attachment_key: &str,
    default_page_idx: u64,
    _source_provider: &str,
    section_path: &mut Vec<String>,
) {
    for raw in raw_blocks {
        let page_idx = raw
            .get("page_idx")
            .and_then(Value::as_u64)
            .or_else(|| raw.get("page").and_then(Value::as_u64))
            .unwrap_or(default_page_idx);
        // `native_label` (GLM) carries the fine-grained element type
        // (doc_title/paragraph_title/display_formula/chart/...) while the
        // coarse `label` collapses everything to text/image/table/formula.
        // Prefer it so headings survive — they drive section-aware chunking.
        let block_type = raw
            .get("native_label")
            .or_else(|| raw.get("type"))
            .or_else(|| raw.get("block_label"))
            .or_else(|| raw.get("label"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let normalized_type = normalize_block_type(&block_type);
        let text = if is_table_type(normalized_type) {
            table_text_from_raw(raw)
        } else {
            ["text", "block_content", "content", "caption"]
                .into_iter()
                .find_map(|key| evidence_text_part(raw.get(key)))
                .unwrap_or_default()
        };
        if is_heading_type(&block_type) {
            *section_path = vec![text.clone()];
        }
        let bbox = raw
            .get("bbox")
            .or_else(|| raw.get("bbox_2d"))
            .or_else(|| raw.get("block_bbox"))
            .and_then(bbox4);
        let block_key = format!("{attachment_key}:p{page_idx}:b{}", blocks.len());
        blocks.push(PdfEvidenceBlock {
            block_key,
            item_key: item_key.to_string(),
            attachment_key: attachment_key.to_string(),
            page_idx,
            block_type: normalized_type.to_string(),
            bbox,
            section_path: section_path.clone(),
            text,
        });
    }
}

fn table_text_from_raw(raw: &Value) -> String {
    let mut parts = Vec::<String>::new();
    for key in [
        "title",
        "caption",
        "headers",
        "columns",
        "row_headers",
        "row_labels",
        "units",
        "text",
        "block_content",
        "content",
        "markdown",
        "html",
        "rows",
        "cells",
        "table",
    ] {
        if let Some(text) = evidence_text_part(raw.get(key)) {
            if !parts.iter().any(|part| part == &text) {
                parts.push(text);
            }
        }
    }
    parts.join("\n\n")
}

fn evidence_text_part(value: Option<&Value>) -> Option<String> {
    let text = ocr_text_from_value(value?)?;
    (!text.is_empty()).then_some(text)
}

fn ocr_text_from_value(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty_string(text),
        Value::Array(items) => join_ocr_text(items.iter().filter_map(ocr_text_from_value)),
        Value::Object(map) => {
            for key in [
                "text",
                "block_content",
                "markdown",
                "html",
                "latex",
                "title",
                "caption",
                "content",
                "title_content",
                "paragraph_content",
                "table_body",
                "table_caption",
                "table_footnote",
                "image_caption",
                "list_content",
                "children",
                "items",
                "rows",
                "cells",
                "table",
            ] {
                if let Some(text) = map.get(key).and_then(ocr_text_from_value) {
                    return Some(text);
                }
            }
            join_ocr_text(map.values().filter_map(ocr_text_from_value))
        }
        Value::Number(_) | Value::Bool(_) | Value::Null => None,
    }
}

fn join_ocr_text(parts: impl Iterator<Item = String>) -> Option<String> {
    let text = parts
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (!text.is_empty()).then_some(text)
}

fn bbox4(value: &Value) -> Option<[f64; 4]> {
    let arr = value.as_array()?;
    if arr.len() != 4 {
        return None;
    }
    Some([
        arr[0].as_f64()?,
        arr[1].as_f64()?,
        arr[2].as_f64()?,
        arr[3].as_f64()?,
    ])
}
