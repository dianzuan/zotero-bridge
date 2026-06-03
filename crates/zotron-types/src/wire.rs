//! JSON-RPC wire types and the academic-zh retrieval hit shape.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::evidence::PdfEvidenceRef;

/// Default Zotero Desktop endpoint exposed by the Zotron XPI.
pub const DEFAULT_RPC_URL: &str = "http://127.0.0.1:23119/zotron/rpc";

/// JSON-RPC 2.0 request payload used by the Python client today.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: u64,
}

impl JsonRpcRequest {
    pub fn new(id: u64, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: params.unwrap_or_else(|| Value::Object(Default::default())),
            id,
        }
    }
}

/// JSON-RPC error object returned by the XPI bridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC response envelope. `result` is intentionally untyped at this layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    #[serde(default)]
    pub jsonrpc: Option<String>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcErrorObject>,
    #[serde(default)]
    pub id: Option<Value>,
}

/// Typed contract for academic-zh retrieval hits emitted by `zotron rag hits`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcademicZhHit {
    pub item_key: String,
    pub title: String,
    pub text: String,
    pub authors: Vec<String>,
    pub zotero_uri: String,
    pub section_heading: String,
    #[serde(alias = "chunk_id")]
    pub chunk_key: String,
    pub query: String,
    pub score: f64,
    #[serde(default)]
    pub year: Option<i64>,
    #[serde(default)]
    pub venue: Option<String>,
    #[serde(default)]
    pub doi: Option<String>,
    #[serde(default, alias = "block_ids")]
    pub block_keys: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attachment_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_idx: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_range: Option<[u64; 2]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bbox: Option<[f64; 4]>,
    #[serde(default)]
    pub section_path: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<PdfEvidenceRef>,
}
