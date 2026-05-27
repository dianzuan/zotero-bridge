use serde_json::json;
use zotron_types::{
    RerankProviderSpec, RerankScoreNorm, builtin_rerank_provider_specs,
    build_rerank_provider_request, parse_rerank_provider_response,
};

#[test]
fn builtin_rerank_providers_have_required_fields() {
    let specs: Vec<RerankProviderSpec> = builtin_rerank_provider_specs();
    assert!(specs.len() >= 6, "expect at least 6 providers");
    for spec in &specs {
        assert!(!spec.id.is_empty());
        assert!(!spec.default_model.is_empty() || spec.id == "openai-compatible");
        assert!(!spec.default_url.is_empty() || spec.id == "openai-compatible");
    }
}

#[test]
fn jina_is_first_and_default() {
    let specs = builtin_rerank_provider_specs();
    assert_eq!(specs[0].id, "jina");
    assert_eq!(specs[0].default_model, "jina-reranker-v2-base-multilingual");
    assert!(matches!(specs[0].score_norm, RerankScoreNorm::Identity));
}

#[test]
fn dashscope_uses_sigmoid_normalization() {
    let specs = builtin_rerank_provider_specs();
    let ds = specs.iter().find(|s| s.id == "dashscope").unwrap();
    assert!(matches!(ds.score_norm, RerankScoreNorm::Sigmoid));
}

#[test]
fn build_jina_rerank_request() {
    let req = build_rerank_provider_request(
        "jina-reranker-v2-base-multilingual",
        "what is BM25?",
        &["BM25 is a ranking function", "Apples are fruit"],
        10,
    );
    assert_eq!(req["model"], "jina-reranker-v2-base-multilingual");
    assert_eq!(req["query"], "what is BM25?");
    assert_eq!(req["documents"].as_array().unwrap().len(), 2);
    assert_eq!(req["top_n"], 10);
}

#[test]
fn parse_jina_response_returns_sorted_scores() {
    let spec = &builtin_rerank_provider_specs()[0]; // jina, Identity norm
    let payload = json!({
        "results": [
            {"index": 1, "relevance_score": 0.3},
            {"index": 0, "relevance_score": 0.9},
        ]
    });
    let results = parse_rerank_provider_response(spec, &payload).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].index, 0);
    assert!((results[0].score - 0.9).abs() < 1e-6);
    assert_eq!(results[1].index, 1);
    assert!((results[1].score - 0.3).abs() < 1e-6);
}

#[test]
fn parse_sigmoid_normalization_for_dashscope() {
    let spec = builtin_rerank_provider_specs()
        .into_iter()
        .find(|s| s.id == "dashscope")
        .unwrap();
    let payload = json!({
        "results": [
            {"index": 0, "relevance_score": 5.0},
            {"index": 1, "relevance_score": -5.0},
        ]
    });
    let results = parse_rerank_provider_response(&spec, &payload).unwrap();
    assert!(results[0].score > 0.99);
    assert!(results[1].score < 0.01);
}

#[test]
fn parse_cohere_response_shape() {
    let spec = builtin_rerank_provider_specs()
        .into_iter()
        .find(|s| s.id == "cohere")
        .unwrap();
    let payload = json!({
        "results": [
            {"index": 0, "relevance_score": 0.85},
            {"index": 1, "relevance_score": 0.42},
        ]
    });
    let results = parse_rerank_provider_response(&spec, &payload).unwrap();
    assert_eq!(results.len(), 2);
    assert!((results[0].score - 0.85).abs() < 1e-6);
}
