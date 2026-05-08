use serde_json::json;
use zotron_types::{
    build_embedding_provider_request, parse_embedding_provider_response, EmbeddingChunkInput,
    EmbeddingRequestInput,
};

fn chunks() -> Vec<EmbeddingChunkInput> {
    vec![
        EmbeddingChunkInput {
            chunk_key: "ATT:c0".to_string(),
            text: "数字经济提升全要素生产率。".to_string(),
        },
        EmbeddingChunkInput {
            chunk_key: "ATT:c1".to_string(),
            text: "政策冲击影响企业创新。".to_string(),
        },
    ]
}

fn input() -> EmbeddingRequestInput {
    EmbeddingRequestInput {
        item_key: "ITEMKEY".to_string(),
        chunks: chunks(),
        model: None,
        url: None,
        input_type: None,
    }
}

#[test]
fn volcengine_embedding_request_is_openai_compatible_and_key_first() {
    let request = build_embedding_provider_request("volcengine", &input())
        .expect("volcengine request builds");

    assert_eq!(request.provider, "volcengine");
    assert_eq!(request.method, Some("POST"));
    assert_eq!(request.auth_header, Some("Authorization"));
    assert_eq!(
        request.url.as_deref(),
        Some("https://ark.cn-beijing.volces.com/api/v3/embeddings")
    );
    assert_eq!(request.body["model"], "doubao-embedding-text-240715");
    assert_eq!(
        request.body["input"],
        json!(["数字经济提升全要素生产率。", "政策冲击影响企业创新。"])
    );
    assert_eq!(request.body["chunk_keys"], json!(["ATT:c0", "ATT:c1"]));
    assert!(request.body.get("item_id").is_none());
}

#[test]
fn alibaba_dashscope_request_carries_input_type_for_query_and_document() {
    let query = build_embedding_provider_request(
        "dashscope",
        &EmbeddingRequestInput {
            input_type: Some("query".to_string()),
            ..input()
        },
    )
    .expect("dashscope alias builds");
    assert_eq!(
        query.url.as_deref(),
        Some("https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings")
    );
    assert_eq!(query.body["model"], "text-embedding-v4");
    assert_eq!(query.body["input_type"], "query");

    let document =
        build_embedding_provider_request("alibaba", &input()).expect("alibaba request builds");
    assert_eq!(document.body["input_type"], "document");
}

#[test]
fn custom_openai_compatible_request_requires_external_endpoint_and_model() {
    let err = build_embedding_provider_request(
        "custom",
        &EmbeddingRequestInput {
            model: Some("custom-embedding-model".to_string()),
            ..input()
        },
    )
    .expect_err("custom provider must not invent a URL");
    assert!(err.contains("custom embedding provider requires a base URL"));

    let request = build_embedding_provider_request(
        "custom",
        &EmbeddingRequestInput {
            model: Some("custom-embedding-model".to_string()),
            url: Some("http://127.0.0.1:8080/v1/embeddings".to_string()),
            ..input()
        },
    )
    .expect("custom request builds with explicit endpoint");
    assert_eq!(
        request.url.as_deref(),
        Some("http://127.0.0.1:8080/v1/embeddings")
    );
    assert_eq!(request.body["model"], "custom-embedding-model");
    assert_eq!(
        request.body["input"],
        json!(["数字经济提升全要素生产率。", "政策冲击影响企业创新。"])
    );
}

#[test]
fn embedding_response_parser_accepts_openai_and_dashscope_shapes() {
    let openai = json!({
        "model": "doubao-embedding-text-240715",
        "object": "list",
        "data": [
            {"object": "embedding", "index": 0, "embedding": [0.1, 0.2]},
            {"object": "embedding", "index": 1, "embedding": [0.3, 0.4]}
        ]
    });
    let parsed = parse_embedding_provider_response("volcengine", &openai, "ITEMKEY", &chunks())
        .expect("openai-compatible response parses");
    assert_eq!(parsed[0].item_key, "ITEMKEY");
    assert_eq!(parsed[0].chunk_key, "ATT:c0");
    assert_eq!(parsed[0].source_provider, "volcengine");
    assert_eq!(parsed[0].vector, vec![0.1, 0.2]);
    assert!(serde_json::to_value(&parsed[0])
        .unwrap()
        .get("item_id")
        .is_none());

    let dashscope = json!({
        "output": {
            "embeddings": [
                {"text_index": 0, "embedding": [1.0, 2.0, 3.0]},
                {"text_index": 1, "embedding": [4.0, 5.0, 6.0]}
            ]
        }
    });
    let parsed = parse_embedding_provider_response("alibaba", &dashscope, "ITEMKEY", &chunks())
        .expect("dashscope native response parses");
    assert_eq!(parsed[1].chunk_key, "ATT:c1");
    assert_eq!(parsed[1].vector, vec![4.0, 5.0, 6.0]);
}
