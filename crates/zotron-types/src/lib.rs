//! Shared Zotron wire and CLI output types.

use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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

/// Typed contract for academic-zh retrieval hits emitted by `zotron-rag hits`.
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
}

/// OCR provider request families supported by the Rust evidence contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OcrRequestStyle {
    GlmLayoutParsing,
    PaddleocrVl,
    MineruCloudPrecise,
    MineruCli,
}

impl PartialEq<&str> for OcrRequestStyle {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl OcrRequestStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GlmLayoutParsing => "glm-layout-parsing",
            Self::PaddleocrVl => "paddleocr-vl",
            Self::MineruCloudPrecise => "mineru-cloud-precise",
            Self::MineruCli => "mineru-cli",
        }
    }
}

/// Static OCR provider contract. It describes transport and normalization
/// expectations; it does not perform live network calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcrProviderSpec {
    pub id: &'static str,
    pub provider_key: &'static str,
    pub request_style: OcrRequestStyle,
    pub auth: &'static str,
    pub auth_header: &'static str,
    pub supports_pdf_direct: bool,
    pub requires_api_key: bool,
    pub key_field: &'static str,
}

/// Provider-neutral OCR input used to build transport request payloads.
///
/// The Rust type contract intentionally accepts already-encoded PDF bytes so
/// this crate does not need to own IO, multipart encoding, or HTTP clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OcrRequestInput {
    pub item_key: String,
    pub attachment_key: String,
    pub file_name: String,
    pub mime_type: String,
    pub content_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_dir: Option<String>,
}

/// Static transport contract emitted for a provider.
///
/// Network callers can turn `method`/`url`/`auth_header`/`body` into an HTTP
/// request. Local providers such as MinerU use `command` instead.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrProviderRequest {
    pub provider: &'static str,
    pub style: &'static str,
    pub key_field: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<&'static str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<&'static str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header: Option<&'static str>,
    #[serde(default)]
    pub body: Value,
    #[serde(default)]
    pub command: Vec<String>,
}

/// Provider-neutral HTTP invocation passed to injected OCR/embedding transports.
///
/// This carries only the header name required by the provider contract. The
/// credential value is intentionally absent unless a higher layer explicitly
/// opts into live execution; contract tests keep it `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProviderHttpInvocation {
    pub provider: String,
    pub style: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header_value: Option<String>,
    pub body: Value,
}

/// HTTP execution seam for provider tests and future live adapters.
pub trait ProviderHttpTransport {
    fn post_json(&mut self, invocation: &ProviderHttpInvocation) -> Result<Value, String>;
}

/// CLI execution seam for local OCR providers such as MinerU.
pub trait ProviderCommandRunner {
    fn run_json(&mut self, command: &[String]) -> Result<Value, String>;
}

/// Embedding provider request families supported by the Rust evidence contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EmbeddingRequestStyle {
    OpenAiCompatible,
    Dashscope,
    Custom,
}

impl PartialEq<&str> for EmbeddingRequestStyle {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other || (*self == Self::OpenAiCompatible && *other == "dashscope")
    }
}

impl EmbeddingRequestStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai-compatible",
            Self::Dashscope => "dashscope",
            Self::Custom => "custom",
        }
    }
}

/// Static embedding provider contract. Provider outputs are machine artifacts
/// keyed by Zotero keys, not Zotero-visible literature records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingProviderSpec {
    pub id: &'static str,
    pub provider_key: &'static str,
    pub request_style: EmbeddingRequestStyle,
    pub default_url: Option<&'static str>,
    pub base_url: Option<&'static str>,
    pub default_model: &'static str,
    pub auth: &'static str,
    pub key_field: &'static str,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query_task: Option<&'static str>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_task: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingChunkInput {
    pub chunk_key: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddingRequestInput {
    pub item_key: String,
    pub chunks: Vec<EmbeddingChunkInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingProviderRequest {
    pub provider: &'static str,
    pub style: &'static str,
    pub key_field: &'static str,
    pub method: Option<&'static str>,
    pub url: Option<String>,
    pub auth_header: Option<&'static str>,
    pub body: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmbeddingVector {
    pub item_key: String,
    pub chunk_key: String,
    pub vector: Vec<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub source_provider: String,
}

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
        let text = blocks
            .iter()
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("\n\n");

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
        }
    }
}

pub fn builtin_ocr_provider_specs() -> Vec<OcrProviderSpec> {
    vec![
        OcrProviderSpec {
            id: "glm",
            provider_key: "glm",
            request_style: OcrRequestStyle::GlmLayoutParsing,
            auth: "bearer",
            auth_header: "Authorization",
            supports_pdf_direct: true,
            requires_api_key: true,
            key_field: "attachment_key",
        },
        OcrProviderSpec {
            id: "paddle",
            provider_key: "paddleocr-vl",
            request_style: OcrRequestStyle::PaddleocrVl,
            auth: "token",
            auth_header: "Authorization",
            supports_pdf_direct: true,
            requires_api_key: true,
            key_field: "attachment_key",
        },
        OcrProviderSpec {
            id: "mineru",
            provider_key: "mineru",
            request_style: OcrRequestStyle::MineruCloudPrecise,
            auth: "bearer",
            auth_header: "Authorization",
            supports_pdf_direct: true,
            requires_api_key: true,
            key_field: "attachment_key",
        },
        OcrProviderSpec {
            id: "mineru-cli",
            provider_key: "mineru-cli",
            request_style: OcrRequestStyle::MineruCli,
            auth: "none",
            auth_header: "",
            supports_pdf_direct: true,
            requires_api_key: false,
            key_field: "attachment_key",
        },
    ]
}

pub fn ocr_provider_spec(provider: &str) -> Result<OcrProviderSpec, String> {
    let normalized = provider.trim().to_ascii_lowercase();
    builtin_ocr_provider_specs()
        .into_iter()
        .find(|spec| {
            spec.id == normalized
                || spec.provider_key == normalized
                || (normalized == "paddle" && spec.provider_key == "paddleocr-vl")
        })
        .ok_or_else(|| format!("Unknown OCR provider: {provider}"))
}

pub fn build_ocr_provider_request(
    provider: &str,
    input: &OcrRequestInput,
) -> Result<OcrProviderRequest, String> {
    let spec = ocr_provider_spec(provider)?;

    match spec.request_style {
        OcrRequestStyle::GlmLayoutParsing => Ok(OcrProviderRequest {
            provider: spec.provider_key,
            style: spec.request_style.as_str(),
            key_field: spec.key_field,
            method: Some("POST"),
            url: Some("https://open.bigmodel.cn/api/paas/v4/layout_parsing"),
            auth_header: Some(spec.auth_header),
            body: json!({
                "model": "glm-ocr",
                "file": data_url_file_payload(input),
                "return_crop_images": false,
                "need_layout_visualization": false,
            }),
            command: Vec::new(),
        }),
        OcrRequestStyle::PaddleocrVl => Ok(OcrProviderRequest {
            provider: spec.provider_key,
            style: spec.request_style.as_str(),
            key_field: spec.key_field,
            method: Some("POST"),
            url: Some("<your-aistudio-layout-parsing-endpoint>"),
            auth_header: Some(spec.auth_header),
            body: json!({
                "file": input.content_base64,
                "fileType": if input.mime_type == "application/pdf" { 0 } else { 1 },
                "useDocOrientationClassify": false,
                "useDocUnwarping": false,
                "useChartRecognition": false,
            }),
            command: Vec::new(),
        }),
        OcrRequestStyle::MineruCloudPrecise => {
            let source_url = input
                .source_url
                .as_deref()
                .filter(|value| is_http_url(value))
                .or_else(|| input.content_base64.trim().strip_prefix("url:"))
                .or_else(|| {
                    is_http_url(input.content_base64.trim()).then(|| input.content_base64.trim())
                })
                .ok_or_else(|| {
                    "MinerU Cloud precise OCR request requires source_url or URL content_base64"
                        .to_string()
                })?;

            Ok(OcrProviderRequest {
                provider: spec.provider_key,
                style: spec.request_style.as_str(),
                key_field: spec.key_field,
                method: Some("POST"),
                url: Some("https://mineru.net/api/v4/extract/task"),
                auth_header: Some(spec.auth_header),
                body: json!({
                    "url": source_url,
                    "model_version": "vlm",
                    "is_ocr": false,
                    "enable_formula": true,
                    "enable_table": true,
                    "language": "ch",
                    "data_id": input.attachment_key,
                    "page_ranges": "1-200",
                }),
                command: Vec::new(),
            })
        }
        OcrRequestStyle::MineruCli => {
            let local_path = input
                .local_path
                .as_deref()
                .ok_or_else(|| "MinerU OCR request requires local_path".to_string())?;
            let output_dir = input
                .output_dir
                .as_deref()
                .ok_or_else(|| "MinerU OCR request requires output_dir".to_string())?;

            Ok(OcrProviderRequest {
                provider: spec.provider_key,
                style: spec.request_style.as_str(),
                key_field: spec.key_field,
                method: None,
                url: None,
                auth_header: None,
                body: Value::Null,
                command: vec![
                    "mineru".to_string(),
                    "-p".to_string(),
                    local_path.to_string(),
                    "-o".to_string(),
                    output_dir.to_string(),
                ],
            })
        }
    }
}

pub fn builtin_embedding_provider_specs() -> Vec<EmbeddingProviderSpec> {
    vec![
        EmbeddingProviderSpec {
            id: "volcengine",
            provider_key: "volcengine",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://ark.cn-beijing.volces.com/api/v3/embeddings"),
            base_url: Some("https://ark.cn-beijing.volces.com/api/v3/embeddings"),
            default_model: "doubao-embedding-text-240715",
            auth: "bearer",
            key_field: "item_key",
            query_task: None,
            document_task: None,
        },
        EmbeddingProviderSpec {
            id: "alibaba",
            provider_key: "alibaba",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings"),
            base_url: Some("https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings"),
            default_model: "text-embedding-v4",
            auth: "bearer",
            key_field: "item_key",
            query_task: Some("query"),
            document_task: Some("document"),
        },
        EmbeddingProviderSpec {
            id: "custom",
            provider_key: "custom",
            request_style: EmbeddingRequestStyle::Custom,
            default_url: None,
            base_url: None,
            default_model: "",
            auth: "bearer",
            key_field: "item_key",
            query_task: None,
            document_task: None,
        },
    ]
}

pub fn embedding_provider_spec(provider: &str) -> Result<EmbeddingProviderSpec, String> {
    let normalized = provider.trim().to_ascii_lowercase();
    builtin_embedding_provider_specs()
        .into_iter()
        .find(|spec| {
            spec.id == normalized
                || spec.provider_key == normalized
                || (normalized == "dashscope" && spec.provider_key == "alibaba")
        })
        .ok_or_else(|| format!("Unknown embedding provider: {provider}"))
}

pub fn build_embedding_provider_request(
    provider: &str,
    input: &EmbeddingRequestInput,
) -> Result<EmbeddingProviderRequest, String> {
    let spec = embedding_provider_spec(provider)?;
    let url = input
        .url
        .as_deref()
        .or(spec.default_url)
        .or(spec.base_url)
        .ok_or_else(|| "custom embedding provider requires a base URL".to_string())?
        .to_string();
    let model = input
        .model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .unwrap_or(spec.default_model);
    if model.trim().is_empty() {
        return Err(format!(
            "embedding provider {} requires a model",
            spec.provider_key
        ));
    }

    let mut body = serde_json::Map::new();
    body.insert("model".to_string(), Value::String(model.to_string()));
    body.insert(
        "input".to_string(),
        Value::Array(
            input
                .chunks
                .iter()
                .map(|chunk| Value::String(chunk.text.clone()))
                .collect(),
        ),
    );
    body.insert(
        "chunk_keys".to_string(),
        Value::Array(
            input
                .chunks
                .iter()
                .map(|chunk| Value::String(chunk.chunk_key.clone()))
                .collect(),
        ),
    );
    if spec.provider_key == "alibaba" {
        let input_type = input
            .input_type
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("document");
        body.insert(
            "input_type".to_string(),
            Value::String(input_type.to_string()),
        );
    }

    let style = match spec.provider_key {
        "alibaba" => EmbeddingRequestStyle::Dashscope.as_str(),
        "custom" => EmbeddingRequestStyle::OpenAiCompatible.as_str(),
        _ => spec.request_style.as_str(),
    };

    Ok(EmbeddingProviderRequest {
        provider: spec.provider_key,
        style,
        key_field: spec.key_field,
        method: Some("POST"),
        url: Some(url),
        auth_header: Some("Authorization"),
        body: Value::Object(body),
    })
}

pub fn parse_embedding_provider_response(
    provider: &str,
    payload: &Value,
    item_key: &str,
    chunks: &[EmbeddingChunkInput],
) -> Result<Vec<EmbeddingVector>, String> {
    fn parse_required_index(item: &Value, field: &str, context: &str) -> Result<usize, String> {
        let raw = item
            .get(field)
            .ok_or_else(|| format!("{context} missing {field}"))?;
        let index = raw
            .as_u64()
            .ok_or_else(|| format!("{context} field {field} must be a non-negative integer"))?;
        Ok(index as usize)
    }

    let spec = embedding_provider_spec(provider)?;
    let embeddings = if let Some(data) = payload.get("data").and_then(Value::as_array) {
        data.iter()
            .map(|item| {
                let index = parse_required_index(item, "index", "embedding response item")?;
                let vector = item
                    .get("embedding")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "embedding response item missing embedding".to_string())?;
                Ok((index, vector_to_f64(vector)?))
            })
            .collect::<Result<Vec<_>, String>>()?
    } else if let Some(items) = payload
        .pointer("/output/embeddings")
        .and_then(Value::as_array)
    {
        items
            .iter()
            .map(|item| {
                let index = parse_required_index(item, "text_index", "dashscope embedding item")?;
                let vector = item
                    .get("embedding")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "dashscope embedding item missing embedding".to_string())?;
                Ok((index, vector_to_f64(vector)?))
            })
            .collect::<Result<Vec<_>, String>>()?
    } else {
        return Err("embedding response missing data or output.embeddings".to_string());
    };
    if embeddings.is_empty() {
        return Err(format!(
            "embedding provider {} response did not contain parseable embeddings",
            spec.provider_key
        ));
    }

    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    embeddings
        .into_iter()
        .map(|(index, vector)| {
            let chunk = chunks
                .get(index)
                .ok_or_else(|| format!("embedding response index {index} has no chunk key"))?;
            Ok(EmbeddingVector {
                item_key: item_key.to_string(),
                chunk_key: chunk.chunk_key.clone(),
                vector,
                model: model.clone(),
                source_provider: spec.provider_key.to_string(),
            })
        })
        .collect()
}

pub fn execute_ocr_provider_request(
    provider: &str,
    input: &OcrRequestInput,
    http: &mut impl ProviderHttpTransport,
    command_runner: &mut impl ProviderCommandRunner,
) -> Result<Vec<PdfEvidenceBlock>, String> {
    let request = build_ocr_provider_request(provider, input)?;
    let payload = if request.command.is_empty() {
        let method = request
            .method
            .ok_or_else(|| format!("OCR provider {} missing HTTP method", request.provider))?;
        http.post_json(&ProviderHttpInvocation {
            provider: request.provider.to_string(),
            style: request.style.to_string(),
            method: method.to_string(),
            url: request.url.map(ToString::to_string),
            auth_header_name: request.auth_header.map(ToString::to_string),
            auth_header_value: None,
            body: request.body,
        })?
    } else {
        command_runner.run_json(&request.command)?
    };

    parse_ocr_provider_response(
        request.provider,
        &payload,
        &input.item_key,
        &input.attachment_key,
    )
}

pub fn execute_embedding_provider_request(
    provider: &str,
    input: &EmbeddingRequestInput,
    http: &mut impl ProviderHttpTransport,
) -> Result<Vec<EmbeddingVector>, String> {
    let request = build_embedding_provider_request(provider, input)?;
    let method = request.method.ok_or_else(|| {
        format!(
            "embedding provider {} missing HTTP method",
            request.provider
        )
    })?;
    let payload = http.post_json(&ProviderHttpInvocation {
        provider: request.provider.to_string(),
        style: request.style.to_string(),
        method: method.to_string(),
        url: request.url.clone(),
        auth_header_name: request.auth_header.map(ToString::to_string),
        auth_header_value: None,
        body: request.body,
    })?;

    parse_embedding_provider_response(request.provider, &payload, &input.item_key, &input.chunks)
}

pub fn is_zotron_evidence_artifact(title: &str) -> bool {
    const SUFFIXES: [&str; 6] = [
        ".zotron-ocr.raw.zip",
        ".zotron-blocks.jsonl",
        ".zotron-chunks.jsonl",
        ".zotron-embed.npz",
        ".zotron-ocr.native.md",
        ".zotron-ocr.assets.json",
    ];
    SUFFIXES.iter().any(|suffix| title.ends_with(suffix))
}

pub fn is_zotron_artifact_title(title: &str) -> bool {
    is_zotron_evidence_artifact(title)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineArtifactKind {
    OcrRaw,
    Blocks,
    Chunks,
    EmbeddingVectors,
    OcrNativeMarkdown,
    OcrNativeAssets,
}

impl MachineArtifactKind {
    pub fn file_name(self) -> &'static str {
        match self {
            Self::OcrRaw => "zotron-ocr.raw.zip",
            Self::Blocks => "zotron-blocks.jsonl",
            Self::Chunks => "zotron-chunks.jsonl",
            Self::EmbeddingVectors => "zotron-embed.npz",
            Self::OcrNativeMarkdown => "zotron-ocr.native.md",
            Self::OcrNativeAssets => "zotron-ocr.assets.json",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineArtifactStorage {
    ExternalStore,
    ZoteroAttachment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineArtifactPersistPlan {
    pub storage: MachineArtifactStorage,
    pub relative_path: PathBuf,
    pub file_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zotero_attachment_title: Option<String>,
    pub should_call_zotero_attachments_add: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineArtifactRecord {
    pub item_key: String,
    pub attachment_key: String,
    pub kind: MachineArtifactKind,
    pub relative_path: PathBuf,
    pub absolute_path: PathBuf,
}

pub fn machine_artifact_relative_path(
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
) -> PathBuf {
    PathBuf::from("items")
        .join(item_key)
        .join("attachments")
        .join(attachment_key)
        .join(kind.file_name())
}

pub fn machine_artifact_persist_plan(
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    attach_to_zotero: bool,
) -> MachineArtifactPersistPlan {
    let relative_path = machine_artifact_relative_path(item_key, attachment_key, kind);
    let file_name = kind.file_name().to_string();
    MachineArtifactPersistPlan {
        storage: if attach_to_zotero {
            MachineArtifactStorage::ZoteroAttachment
        } else {
            MachineArtifactStorage::ExternalStore
        },
        relative_path,
        file_name,
        zotero_attachment_title: attach_to_zotero
            .then(|| format!("{item_key}.{}", kind.file_name())),
        should_call_zotero_attachments_add: attach_to_zotero,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactStorePlatform {
    Linux,
    Macos,
    Windows,
    Other,
}

impl ArtifactStorePlatform {
    pub fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }
}

pub fn machine_artifact_store_root_for_platform(
    platform: ArtifactStorePlatform,
    zotron_artifact_store: Option<&Path>,
    xdg_data_home: Option<&Path>,
    appdata: Option<&Path>,
    userprofile: Option<&Path>,
    home: Option<&Path>,
) -> PathBuf {
    if let Some(path) = zotron_artifact_store {
        return path.to_path_buf();
    }

    match platform {
        ArtifactStorePlatform::Windows => {
            if let Some(path) = appdata {
                return path.join("Zotron").join("artifacts");
            }
            if let Some(path) = userprofile {
                return path
                    .join("AppData")
                    .join("Roaming")
                    .join("Zotron")
                    .join("artifacts");
            }
            if let Some(path) = home {
                return path
                    .join("AppData")
                    .join("Roaming")
                    .join("Zotron")
                    .join("artifacts");
            }
            PathBuf::from(".zotron").join("artifacts")
        }
        ArtifactStorePlatform::Macos => {
            if let Some(path) = home {
                return path
                    .join("Library")
                    .join("Application Support")
                    .join("Zotron")
                    .join("artifacts");
            }
            if let Some(path) = xdg_data_home {
                return path.join("zotron").join("artifacts");
            }
            PathBuf::from(".zotron").join("artifacts")
        }
        ArtifactStorePlatform::Linux | ArtifactStorePlatform::Other => xdg_data_home
            .map(|path| path.join("zotron").join("artifacts"))
            .or_else(|| {
                home.map(|path| {
                    path.join(".local")
                        .join("share")
                        .join("zotron")
                        .join("artifacts")
                })
            })
            .unwrap_or_else(|| PathBuf::from(".zotron").join("artifacts")),
    }
}

pub fn machine_artifact_store_root() -> PathBuf {
    let zotron_artifact_store = env::var_os("ZOTRON_ARTIFACT_STORE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);
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

    machine_artifact_store_root_for_platform(
        ArtifactStorePlatform::current(),
        zotron_artifact_store.as_deref(),
        xdg_data_home.as_deref(),
        appdata.as_deref(),
        userprofile.as_deref(),
        home.as_deref(),
    )
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

/// Extract provider-native image/layout asset references from known OCR shapes.
///
/// Values are preserved without interpretation so downstream tooling can fetch
/// or decode provider-specific URLs/base64 payloads without re-running OCR.
pub fn provider_native_image_assets(payload: &Value) -> Option<Value> {
    let mut assets = serde_json::Map::new();
    if let Some(images) = payload.get("layout_visualization") {
        assets.insert("layout_visualization".to_string(), images.clone());
    }
    if let Some(images) = payload.get("outputImages") {
        assets.insert("outputImages".to_string(), images.clone());
    }
    if let Some(images) = payload.pointer("/markdown/images") {
        assets.insert("markdown_images".to_string(), images.clone());
    }
    if let Some(results) = payload
        .pointer("/result/layoutParsingResults")
        .and_then(Value::as_array)
    {
        let page_assets = results
            .iter()
            .enumerate()
            .filter_map(|(index, result)| {
                let mut page = serde_json::Map::new();
                if let Some(images) = result.pointer("/markdown/images") {
                    page.insert("markdown_images".to_string(), images.clone());
                }
                if let Some(images) = result.get("outputImages") {
                    page.insert("outputImages".to_string(), images.clone());
                }
                (!page.is_empty()).then(|| {
                    json!({
                        "page": index as u64 + 1,
                        "assets": Value::Object(page),
                    })
                })
            })
            .collect::<Vec<_>>();
        if !page_assets.is_empty() {
            assets.insert(
                "layout_parsing_results".to_string(),
                Value::Array(page_assets),
            );
        }
    }

    (!assets.is_empty()).then(|| Value::Object(assets))
}

/// Persist raw OCR provider JSON plus native Markdown/image asset sidecars.
///
/// The raw payload is always written as `zotron-ocr.raw.zip` for compatibility
/// with the existing artifact vocabulary, while extracted native Markdown and
/// image references are written only when the provider returned them.
pub fn write_ocr_provider_artifacts(
    store_root: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    provider: &str,
    payload: &Value,
) -> io::Result<Vec<MachineArtifactRecord>> {
    let raw_bundle = json!({
        "provider": provider,
        "payload": payload,
    });
    let raw_bytes = serde_json::to_vec_pretty(&raw_bundle).map_err(json_to_io_error)?;
    let mut records = vec![write_machine_artifact(
        store_root.as_ref(),
        item_key,
        attachment_key,
        MachineArtifactKind::OcrRaw,
        &raw_bytes,
    )?];

    if let Some(markdown) = provider_native_markdown(payload) {
        records.push(write_machine_artifact(
            store_root.as_ref(),
            item_key,
            attachment_key,
            MachineArtifactKind::OcrNativeMarkdown,
            markdown.as_bytes(),
        )?);
    }

    if let Some(assets) = provider_native_image_assets(payload) {
        let bytes = serde_json::to_vec_pretty(&json!({
            "provider": provider,
            "assets": assets,
        }))
        .map_err(json_to_io_error)?;
        records.push(write_machine_artifact(
            store_root.as_ref(),
            item_key,
            attachment_key,
            MachineArtifactKind::OcrNativeAssets,
            &bytes,
        )?);
    }

    Ok(records)
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn json_to_io_error(err: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err)
}

pub fn machine_artifact_absolute_path(
    store_root: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
) -> PathBuf {
    store_root.as_ref().join(machine_artifact_relative_path(
        item_key,
        attachment_key,
        kind,
    ))
}

pub fn write_machine_artifact(
    store_root: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    bytes: &[u8],
) -> io::Result<MachineArtifactRecord> {
    let relative_path = machine_artifact_relative_path(item_key, attachment_key, kind);
    let absolute_path = store_root.as_ref().join(&relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&absolute_path, bytes)?;
    Ok(MachineArtifactRecord {
        item_key: item_key.to_string(),
        attachment_key: attachment_key.to_string(),
        kind,
        relative_path,
        absolute_path,
    })
}

pub fn read_machine_artifact(
    store_root: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
) -> io::Result<Vec<u8>> {
    fs::read(machine_artifact_absolute_path(
        store_root,
        item_key,
        attachment_key,
        kind,
    ))
}

pub fn delete_machine_artifact(
    store_root: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
) -> io::Result<bool> {
    let path = machine_artifact_absolute_path(store_root, item_key, attachment_key, kind);
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

pub fn find_machine_artifact(
    store_root: impl AsRef<Path>,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
) -> Option<MachineArtifactRecord> {
    let absolute_path = machine_artifact_absolute_path(&store_root, item_key, attachment_key, kind);
    absolute_path.exists().then(|| MachineArtifactRecord {
        item_key: item_key.to_string(),
        attachment_key: attachment_key.to_string(),
        kind,
        relative_path: machine_artifact_relative_path(item_key, attachment_key, kind),
        absolute_path,
    })
}

pub fn machine_artifact_exists_for_item(
    store_root: impl AsRef<Path>,
    item_key: &str,
    kind: MachineArtifactKind,
) -> bool {
    let attachments_dir = store_root
        .as_ref()
        .join("items")
        .join(item_key)
        .join("attachments");
    let Ok(entries) = fs::read_dir(attachments_dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .path()
            .join(kind.file_name())
            .try_exists()
            .unwrap_or(false)
    })
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
            let blocks = page
                .get("blocks")
                .or_else(|| page.get("items"))
                .or_else(|| page.get("content"))
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
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

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn data_url_file_payload(input: &OcrRequestInput) -> String {
    let content = input.content_base64.trim();
    if content.starts_with("data:")
        || content.starts_with("http://")
        || content.starts_with("https://")
    {
        return content.to_string();
    }
    format!("data:{};base64,{content}", input.mime_type)
}

fn parse_json_string(value: &Value) -> Option<Value> {
    let text = value.as_str()?.trim();
    serde_json::from_str(text).ok()
}

pub fn chunks_from_blocks(blocks: &[PdfEvidenceBlock], max_chars: usize) -> Vec<StructureChunk> {
    let mut chunks = Vec::<StructureChunk>::new();
    let mut current = Vec::<PdfEvidenceBlock>::new();
    let mut current_chars = 0usize;
    let mut current_section = Vec::<String>::new();
    let attachment_key = blocks
        .first()
        .map(|block| block.attachment_key.clone())
        .unwrap_or_default();

    for block in blocks {
        if is_heading_type(&block.block_type) {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
            current_section = vec![block.text.clone()];
            continue;
        }

        let block_section = if block.section_path.is_empty() {
            current_section.clone()
        } else {
            block.section_path.clone()
        };

        if !current.is_empty() && !same_section(&current[0].section_path, &block_section) {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
        }

        let mut block = block.clone();
        block.section_path = block_section;

        if is_table_type(&block.block_type) {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
            chunks.push(StructureChunk::from_blocks(
                format!("{attachment_key}:c{}", chunks.len()),
                std::slice::from_ref(&block),
            ));
            continue;
        }

        let block_chars = block.text.chars().count();
        let joined_chars = current_chars + if current.is_empty() { 0 } else { 2 } + block_chars;
        if max_chars > 0 && !current.is_empty() && joined_chars > max_chars {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
        }

        current_chars += if current.is_empty() { 0 } else { 2 } + block_chars;
        current.push(block);
    }

    flush_chunk(&mut chunks, &mut current, &attachment_key);

    chunks
}

fn flush_chunk(
    chunks: &mut Vec<StructureChunk>,
    current: &mut Vec<PdfEvidenceBlock>,
    attachment_key: &str,
) {
    if current.is_empty() {
        return;
    }
    let chunk_key = format!("{attachment_key}:c{}", chunks.len());
    chunks.push(StructureChunk::from_blocks(chunk_key, current));
    current.clear();
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
        let block_type = raw
            .get("type")
            .or_else(|| raw.get("block_label"))
            .or_else(|| raw.get("label"))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let normalized_type = normalize_block_type(&block_type);
        let text = if is_table_type(normalized_type) {
            table_text_from_raw(raw)
        } else {
            raw.get("text")
                .or_else(|| raw.get("block_content"))
                .or_else(|| raw.get("content"))
                .or_else(|| raw.get("caption"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
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
    let value = value?;
    let text = match value {
        Value::String(text) => text.trim().to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).ok()?,
        Value::Null => return None,
        other => other.to_string(),
    };
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

fn is_heading_type(block_type: &str) -> bool {
    matches!(
        block_type,
        "heading" | "title" | "doc_title" | "paragraph_title" | "section"
    )
}

fn normalize_block_type(block_type: &str) -> &str {
    match block_type {
        "title" | "doc_title" | "paragraph_title" | "heading" | "section" => "heading",
        "text" => "paragraph",
        "table" | "table_body" | "table_caption" | "table_footnote" => "table",
        other => other,
    }
}

fn is_table_type(block_type: &str) -> bool {
    normalize_block_type(block_type) == "table"
}

fn same_section(a: &[String], b: &[String]) -> bool {
    a == b
}

fn vector_to_f64(values: &[Value]) -> Result<Vec<f64>, String> {
    values
        .iter()
        .map(|value| {
            value
                .as_f64()
                .ok_or_else(|| "embedding vector contains non-number".to_string())
        })
        .collect()
}
