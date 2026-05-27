use zotron_types::{RerankProviderSpec, RerankScoreNorm, builtin_rerank_provider_specs};

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
