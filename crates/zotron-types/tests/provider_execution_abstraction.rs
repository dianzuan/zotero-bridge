use serde_json::{json, Value};
use zotron_types::{
    execute_embedding_provider_request, execute_ocr_provider_request, EmbeddingChunkInput,
    EmbeddingRequestInput, OcrRequestInput, ProviderCommandRunner, ProviderHttpInvocation,
    ProviderHttpTransport,
};

#[derive(Default)]
struct MockHttpTransport {
    seen: Vec<ProviderHttpInvocation>,
    replies: Vec<Value>,
}

impl ProviderHttpTransport for MockHttpTransport {
    fn post_json(&mut self, invocation: &ProviderHttpInvocation) -> Result<Value, String> {
        self.seen.push(invocation.clone());
        if invocation.auth_header_value.is_some() {
            return Err("test transport must not receive live credential values".to_string());
        }
        Ok(self.replies.remove(0))
    }
}

#[derive(Default)]
struct MockCommandRunner {
    seen: Vec<Vec<String>>,
    replies: Vec<Value>,
}

impl ProviderCommandRunner for MockCommandRunner {
    fn run_json(&mut self, command: &[String]) -> Result<Value, String> {
        self.seen.push(command.to_vec());
        Ok(self.replies.remove(0))
    }
}

fn assert_no_legacy_id_fields(value: &Value) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                assert!(
                    !matches!(
                        key.as_str(),
                        "item_id" | "attachment_id" | "block_id" | "chunk_id" | "block_ids"
                    ),
                    "provider execution outputs must stay key-first; found legacy field {key}"
                );
                assert_no_legacy_id_fields(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_no_legacy_id_fields(item);
            }
        }
        _ => {}
    }
}

fn assert_no_zotero_write_surface(value: &Value) {
    let serialized = serde_json::to_string(value).expect("value serializes");
    for forbidden in [
        "/zotron/rpc",
        "attachments.add",
        "items.update",
        "storage",
        ".zotero",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "provider execution must not expose Zotero RPC/storage write surface: {forbidden}"
        );
    }
}

fn ocr_input() -> OcrRequestInput {
    OcrRequestInput {
        item_key: "ITEMKEY".to_string(),
        attachment_key: "ATTACHKEY".to_string(),
        file_name: "paper.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        content_base64: "JVBERi0x".to_string(),
        local_path: Some("/tmp/paper.pdf".to_string()),
        output_dir: Some("/tmp/mineru-out".to_string()),
    }
}

#[test]
fn ocr_executor_routes_glm_and_paddle_through_mock_http_without_credentials() {
    let mut http = MockHttpTransport {
        replies: vec![
            json!({"choices":[{"message":{"content":"{\"pages\":[{\"page\":1,\"blocks\":[{\"type\":\"text\",\"text\":\"GLM 正文\"}]}]}"}}]}),
            json!({"pages":[{"page_idx":2,"blocks":[{"block_label":"paragraph","block_content":"Paddle 正文"}]}]}),
        ],
        ..Default::default()
    };
    let mut cli = MockCommandRunner::default();

    let glm = execute_ocr_provider_request("glm", &ocr_input(), &mut http, &mut cli)
        .expect("glm executes through injected HTTP transport");
    assert_eq!(glm[0].block_key, "ATTACHKEY:p1:b0");
    assert_eq!(glm[0].text, "GLM 正文");

    let paddle = execute_ocr_provider_request("paddleocr-vl", &ocr_input(), &mut http, &mut cli)
        .expect("paddle executes through injected HTTP transport");
    assert_eq!(paddle[0].block_key, "ATTACHKEY:p2:b0");
    assert_eq!(paddle[0].text, "Paddle 正文");

    assert!(
        cli.seen.is_empty(),
        "HTTP providers must not invoke CLI runner"
    );
    assert_eq!(http.seen.len(), 2);
    assert_eq!(http.seen[0].provider, "glm");
    assert_eq!(
        http.seen[0].auth_header_name.as_deref(),
        Some("Authorization")
    );
    assert!(http.seen[0].auth_header_value.is_none());
    assert_eq!(http.seen[1].provider, "paddleocr-vl");
    assert_eq!(http.seen[1].body["attachment_key"], "ATTACHKEY");
}

#[test]
fn ocr_executor_outputs_key_first_blocks_without_zotero_write_surfaces() {
    let mut http = MockHttpTransport {
        replies: vec![json!({"pages":[{"page":7,"blocks":[
            {"type":"title","text":"三、实证结果"},
            {"type":"text","text":"关键结论保持可追溯。"}
        ]}]})],
        ..Default::default()
    };
    let mut cli = MockCommandRunner::default();

    let blocks = execute_ocr_provider_request("glm", &ocr_input(), &mut http, &mut cli)
        .expect("OCR executor emits normalized blocks");
    let value = serde_json::to_value(&blocks).expect("blocks serialize");

    assert_eq!(value[0]["item_key"], "ITEMKEY");
    assert_eq!(value[0]["attachment_key"], "ATTACHKEY");
    assert_eq!(value[0]["block_key"], "ATTACHKEY:p7:b0");
    assert_no_legacy_id_fields(&value);
    assert_no_zotero_write_surface(&value);
    for invocation in &http.seen {
        assert_no_zotero_write_surface(&invocation.body);
    }
    assert!(cli.seen.is_empty());
}

#[test]
fn ocr_executor_routes_mineru_through_mock_cli_runner() {
    let mut http = MockHttpTransport::default();
    let mut cli = MockCommandRunner {
        replies: vec![json!({"content_list":[
            {"page_idx":4,"type":"title","text":"二、模型"},
            {"page_idx":4,"type":"text","text":"MinerU 正文"}
        ]})],
        ..Default::default()
    };

    let blocks = execute_ocr_provider_request("mineru", &ocr_input(), &mut http, &mut cli)
        .expect("mineru executes through injected command runner");

    assert!(
        http.seen.is_empty(),
        "CLI providers must not invoke HTTP transport"
    );
    assert_eq!(
        cli.seen,
        vec![vec![
            "mineru",
            "-p",
            "/tmp/paper.pdf",
            "-o",
            "/tmp/mineru-out"
        ]]
    );
    assert_eq!(blocks[1].block_key, "ATTACHKEY:p4:b1");
    assert_eq!(blocks[1].section_path, vec!["二、模型"]);
}

#[test]
fn mineru_executor_uses_local_provider_cli_without_zotero_rpc_or_storage_paths() {
    let mut http = MockHttpTransport::default();
    let mut cli = MockCommandRunner {
        replies: vec![json!({"content_list":[
            {"page_idx":1,"type":"text","text":"MinerU 本地解析结果"}
        ]})],
        ..Default::default()
    };

    let blocks = execute_ocr_provider_request("mineru", &ocr_input(), &mut http, &mut cli)
        .expect("MinerU executor emits normalized blocks");
    let value = serde_json::to_value(&blocks).expect("blocks serialize");

    assert_no_legacy_id_fields(&value);
    assert_no_zotero_write_surface(&value);
    assert!(http.seen.is_empty());
    let command_value = serde_json::to_value(&cli.seen).expect("commands serialize");
    assert_no_zotero_write_surface(&command_value);
}

#[test]
fn embedding_executor_routes_cloud_and_custom_providers_through_mock_http() {
    let input = EmbeddingRequestInput {
        item_key: "ITEMKEY".to_string(),
        chunks: vec![EmbeddingChunkInput {
            chunk_key: "ATTACHKEY:c0".to_string(),
            text: "数字经济提升全要素生产率。".to_string(),
        }],
        model: None,
        url: None,
        input_type: None,
    };
    let mut http = MockHttpTransport {
        replies: vec![
            json!({"model":"doubao-embedding-text-240715","data":[{"index":0,"embedding":[0.1,0.2]}]}),
            json!({"model":"text-embedding-v4","output":{"embeddings":[{"text_index":0,"embedding":[1.0,2.0]}]}}),
            json!({"model":"local-bge","data":[{"index":0,"embedding":[3.0,4.0]}]}),
        ],
        ..Default::default()
    };

    let volcengine = execute_embedding_provider_request("volcengine", &input, &mut http)
        .expect("volcengine executes through injected HTTP transport");
    assert_eq!(volcengine[0].chunk_key, "ATTACHKEY:c0");
    assert_eq!(volcengine[0].source_provider, "volcengine");

    let alibaba = execute_embedding_provider_request("alibaba", &input, &mut http)
        .expect("alibaba executes through injected HTTP transport");
    assert_eq!(alibaba[0].vector, vec![1.0, 2.0]);

    let custom = execute_embedding_provider_request(
        "custom",
        &EmbeddingRequestInput {
            model: Some("local-bge".to_string()),
            url: Some("http://127.0.0.1:8080/v1/embeddings".to_string()),
            ..input
        },
        &mut http,
    )
    .expect("custom executes through injected HTTP transport");
    assert_eq!(custom[0].model.as_deref(), Some("local-bge"));

    assert_eq!(http.seen.len(), 3);
    assert_eq!(http.seen[0].provider, "volcengine");
    assert_eq!(http.seen[1].provider, "alibaba");
    assert_eq!(http.seen[1].body["input_type"], "document");
    assert_eq!(http.seen[2].provider, "custom");
    assert_eq!(
        http.seen[2].url.as_deref(),
        Some("http://127.0.0.1:8080/v1/embeddings")
    );
    assert!(http
        .seen
        .iter()
        .all(|invocation| invocation.auth_header_value.is_none()));
}

#[test]
fn embedding_executor_outputs_key_first_vectors_without_zotero_write_surfaces() {
    let input = EmbeddingRequestInput {
        item_key: "ITEMKEY".to_string(),
        chunks: vec![EmbeddingChunkInput {
            chunk_key: "ATTACHKEY:c0".to_string(),
            text: "结构化证据进入向量化。".to_string(),
        }],
        model: None,
        url: None,
        input_type: None,
    };
    let mut http = MockHttpTransport {
        replies: vec![json!({"model":"doubao-embedding-text-240715","data":[
            {"index":0,"embedding":[0.1,0.2,0.3]}
        ]})],
        ..Default::default()
    };

    let vectors = execute_embedding_provider_request("volcengine", &input, &mut http)
        .expect("embedding executor emits vectors");
    let value = serde_json::to_value(&vectors).expect("vectors serialize");

    assert_eq!(value[0]["item_key"], "ITEMKEY");
    assert_eq!(value[0]["chunk_key"], "ATTACHKEY:c0");
    assert_no_legacy_id_fields(&value);
    assert_no_zotero_write_surface(&value);
    assert_eq!(http.seen.len(), 1);
    assert_no_zotero_write_surface(&http.seen[0].body);
}
