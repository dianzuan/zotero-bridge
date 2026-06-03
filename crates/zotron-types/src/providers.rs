//! OCR / embedding / rerank provider specs, request building, and response parsing.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::evidence::{parse_ocr_provider_response, PdfEvidenceBlock};

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
    Cohere,
    OllamaLocal,
    Custom,
}

impl PartialEq<&str> for EmbeddingRequestStyle {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
            || (*self == Self::OpenAiCompatible && *other == "dashscope")
            || (*self == Self::OllamaLocal && *other == "openai-compatible")
    }
}

impl EmbeddingRequestStyle {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai-compatible",
            Self::Dashscope => "dashscope",
            Self::Cohere => "cohere",
            Self::OllamaLocal => "ollama-local",
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

/// Score normalization strategy for reranking providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerankScoreNorm {
    /// Scores are already in [0, 1] — use as-is.
    Identity,
    /// Provider returns logits — apply sigmoid to normalize to [0, 1].
    Sigmoid,
}

/// Static reranking provider contract, parallel to [`EmbeddingProviderSpec`].
#[derive(Debug, Clone)]
pub struct RerankProviderSpec {
    pub id: &'static str,
    pub default_url: &'static str,
    pub default_model: &'static str,
    pub score_norm: RerankScoreNorm,
}

pub fn builtin_rerank_provider_specs() -> Vec<RerankProviderSpec> {
    vec![
        RerankProviderSpec {
            id: "jina",
            default_url: "https://api.jina.ai/v1/rerank",
            default_model: "jina-reranker-v2-base-multilingual",
            score_norm: RerankScoreNorm::Identity,
        },
        RerankProviderSpec {
            id: "cohere",
            default_url: "https://api.cohere.com/v2/rerank",
            default_model: "rerank-v3.5",
            score_norm: RerankScoreNorm::Identity,
        },
        RerankProviderSpec {
            id: "voyage",
            default_url: "https://api.voyageai.com/v1/rerank",
            default_model: "rerank-2",
            score_norm: RerankScoreNorm::Identity,
        },
        RerankProviderSpec {
            id: "dashscope",
            default_url: "https://dashscope.aliyuncs.com/compatible-api/v1/reranks",
            default_model: "qwen3-rerank",
            score_norm: RerankScoreNorm::Identity,
        },
        RerankProviderSpec {
            id: "siliconflow",
            default_url: "https://api.siliconflow.cn/v1/rerank",
            default_model: "BAAI/bge-reranker-v2-m3",
            score_norm: RerankScoreNorm::Sigmoid,
        },
        RerankProviderSpec {
            id: "openai-compatible",
            default_url: "",
            default_model: "",
            score_norm: RerankScoreNorm::Identity,
        },
    ]
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
            url: None,
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
            id: "ollama",
            provider_key: "ollama",
            request_style: EmbeddingRequestStyle::OllamaLocal,
            default_url: Some("http://localhost:11434/v1/embeddings"),
            base_url: Some("http://localhost:11434/v1/embeddings"),
            default_model: "nomic-embed-text",
            auth: "none",
            key_field: "item_key",
            query_task: None,
            document_task: None,
        },
        EmbeddingProviderSpec {
            id: "openai",
            provider_key: "openai",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://api.openai.com/v1/embeddings"),
            base_url: Some("https://api.openai.com/v1/embeddings"),
            default_model: "text-embedding-3-small",
            auth: "bearer",
            key_field: "item_key",
            query_task: None,
            document_task: None,
        },
        EmbeddingProviderSpec {
            id: "zhipu",
            provider_key: "zhipu",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://open.bigmodel.cn/api/paas/v4/embeddings"),
            base_url: Some("https://open.bigmodel.cn/api/paas/v4/embeddings"),
            default_model: "embedding-3",
            auth: "bearer",
            key_field: "item_key",
            query_task: None,
            document_task: None,
        },
        EmbeddingProviderSpec {
            id: "jina",
            provider_key: "jina",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://api.jina.ai/v1/embeddings"),
            base_url: Some("https://api.jina.ai/v1/embeddings"),
            default_model: "jina-embeddings-v3",
            auth: "bearer",
            key_field: "item_key",
            query_task: Some("retrieval.query"),
            document_task: Some("retrieval.passage"),
        },
        EmbeddingProviderSpec {
            id: "siliconflow",
            provider_key: "siliconflow",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://api.siliconflow.cn/v1/embeddings"),
            base_url: Some("https://api.siliconflow.cn/v1/embeddings"),
            default_model: "BAAI/bge-m3",
            auth: "bearer",
            key_field: "item_key",
            query_task: None,
            document_task: None,
        },
        EmbeddingProviderSpec {
            id: "voyage",
            provider_key: "voyage",
            request_style: EmbeddingRequestStyle::OpenAiCompatible,
            default_url: Some("https://api.voyageai.com/v1/embeddings"),
            base_url: Some("https://api.voyageai.com/v1/embeddings"),
            default_model: "voyage-4",
            auth: "bearer",
            key_field: "item_key",
            query_task: Some("query"),
            document_task: Some("document"),
        },
        EmbeddingProviderSpec {
            id: "cohere",
            provider_key: "cohere",
            request_style: EmbeddingRequestStyle::Cohere,
            default_url: Some("https://api.cohere.com/v2/embed"),
            base_url: Some("https://api.cohere.com/v2/embed"),
            default_model: "embed-multilingual-v3.0",
            auth: "bearer",
            key_field: "item_key",
            query_task: Some("search_query"),
            document_task: Some("search_document"),
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
    if spec.provider_key == "cohere" {
        let input_type = input
            .input_type
            .as_deref()
            .filter(|v| !v.trim().is_empty())
            .unwrap_or("search_document");
        body.insert(
            "input_type".to_string(),
            Value::String(input_type.to_string()),
        );
        body.insert(
            "embedding_types".to_string(),
            serde_json::json!(["float"]),
        );
    }
    if spec.provider_key == "jina" || spec.provider_key == "voyage" {
        if let Some(task) = input
            .input_type
            .as_deref()
            .filter(|v| !v.trim().is_empty())
        {
            body.insert(
                "input_type".to_string(),
                Value::String(task.to_string()),
            );
        }
    }

    let style = match spec.provider_key {
        "alibaba" => EmbeddingRequestStyle::Dashscope.as_str(),
        "cohere" => EmbeddingRequestStyle::Cohere.as_str(),
        "custom" => EmbeddingRequestStyle::OpenAiCompatible.as_str(),
        _ => spec.request_style.as_str(),
    };

    Ok(EmbeddingProviderRequest {
        provider: spec.provider_key,
        style,
        key_field: spec.key_field,
        method: Some("POST"),
        url: Some(url),
        auth_header: if spec.auth == "none" { None } else { Some("Authorization") },
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
    } else if let Some(float_list) = payload
        .pointer("/embeddings/float")
        .and_then(Value::as_array)
    {
        float_list
            .iter()
            .enumerate()
            .map(|(index, vec_val)| {
                let vector = vec_val
                    .as_array()
                    .ok_or_else(|| "cohere embedding item not an array".to_string())?;
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

#[derive(Debug, Clone)]
pub struct RerankResult {
    pub index: usize,
    pub score: f64,
}

pub fn build_rerank_provider_request(
    model: &str,
    query: &str,
    documents: &[&str],
    top_n: usize,
) -> serde_json::Value {
    serde_json::json!({
        "model": model,
        "query": query,
        "documents": documents,
        "top_n": top_n,
    })
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

pub fn parse_rerank_provider_response(
    spec: &RerankProviderSpec,
    payload: &serde_json::Value,
) -> Result<Vec<RerankResult>, String> {
    let results = payload
        .get("results")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "missing 'results' array in rerank response".to_string())?;

    let mut parsed: Vec<RerankResult> = results
        .iter()
        .filter_map(|r| {
            let index = r.get("index")?.as_u64()? as usize;
            let raw_score = r.get("relevance_score")?.as_f64()?;
            let score = match spec.score_norm {
                RerankScoreNorm::Identity => raw_score,
                RerankScoreNorm::Sigmoid => sigmoid(raw_score),
            };
            Some(RerankResult { index, score })
        })
        .collect();

    parsed.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    Ok(parsed)
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
