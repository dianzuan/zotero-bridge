use serde_json::json;
use zotron_types::{
    builtin_embedding_provider_specs, builtin_ocr_provider_specs, is_zotron_evidence_artifact,
    EmbeddingRequestStyle, OcrRequestStyle, PdfEvidenceBlock, StructureChunk,
};

#[test]
fn ocr_provider_contracts_are_key_first_for_target_providers() {
    let specs = builtin_ocr_provider_specs();
    let ids = specs
        .iter()
        .map(|spec| spec.provider_key)
        .collect::<Vec<_>>();

    assert!(ids.contains(&"paddleocr-vl"));
    assert!(ids.contains(&"glm"));
    assert!(ids.contains(&"mineru"));

    let glm = specs
        .iter()
        .find(|spec| spec.provider_key == "glm")
        .unwrap();
    assert_eq!(glm.request_style, OcrRequestStyle::GlmLayoutParsing);
    assert!(glm.supports_pdf_direct);

    let mineru = specs
        .iter()
        .find(|spec| spec.provider_key == "mineru")
        .unwrap();
    assert_eq!(mineru.request_style, OcrRequestStyle::MineruCloudPrecise);
    assert!(mineru.requires_api_key);
}

#[test]
fn embedding_provider_contracts_cover_requested_cloud_and_custom_paths() {
    let specs = builtin_embedding_provider_specs();
    let ids = specs
        .iter()
        .map(|spec| spec.provider_key)
        .collect::<Vec<_>>();

    assert!(ids.contains(&"volcengine"));
    assert!(ids.contains(&"alibaba"));
    assert!(ids.contains(&"custom"));

    let alibaba = specs
        .iter()
        .find(|spec| spec.provider_key == "alibaba")
        .unwrap();
    assert_eq!(
        alibaba.request_style,
        EmbeddingRequestStyle::OpenAiCompatible
    );
    assert_eq!(alibaba.document_task, Some("document"));
    assert_eq!(alibaba.query_task, Some("query"));

    let custom = specs
        .iter()
        .find(|spec| spec.provider_key == "custom")
        .unwrap();
    assert!(custom.default_url.is_none());
}

#[test]
fn structure_chunk_contract_preserves_provenance_without_public_ids() {
    let block = PdfEvidenceBlock {
        block_key: "ATTACHKEY:p16:b004".to_string(),
        item_key: "ITEMKEY".to_string(),
        attachment_key: "ATTACHKEY".to_string(),
        page_idx: 15,
        block_type: "text".to_string(),
        bbox: Some([120.0, 300.0, 860.0, 420.0]),
        section_path: vec![
            "PART2".to_string(),
            "构建基于宏观因子的风险平价配置框架".to_string(),
        ],
        text: "Blyth(2016)提出了...".to_string(),
    };
    let chunk =
        StructureChunk::from_blocks("ITEMKEY:ATTACHKEY:c0001", std::slice::from_ref(&block));
    let value = serde_json::to_value(&chunk).unwrap();

    assert_eq!(value["chunk_key"], "ITEMKEY:ATTACHKEY:c0001");
    assert_eq!(value["item_key"], "ITEMKEY");
    assert_eq!(value["attachment_key"], "ATTACHKEY");
    assert_eq!(value["block_keys"], json!(["ATTACHKEY:p16:b004"]));
    assert_eq!(value["page_range"], json!([15, 15]));
    assert!(value.get("item_id").is_none());
    assert!(value.get("attachment_id").is_none());
    assert!(value.get("block_ids").is_none());
}

#[test]
fn chunks_from_blocks_drops_text_less_chunks() {
    // Figure/image blocks carry no text. They must NOT yield empty chunks:
    // strict embedding providers (volcengine) reject a whole batch if any input
    // is blank, and empty chunks pollute BM25 + counts. Regression for a real
    // failure found in production (Attention paper: 5/50 chunks were empty).
    let block = |key: &str, btype: &str, text: &str| PdfEvidenceBlock {
        block_key: format!("ATT:{key}"),
        item_key: "ITEM".to_string(),
        attachment_key: "ATT".to_string(),
        page_idx: 0,
        block_type: btype.to_string(),
        bbox: None,
        section_path: vec![],
        text: text.to_string(),
    };

    // A sole text-less block must produce no chunk at all.
    let only_figure = vec![block("b0", "image", "   ")];
    let chunks = zotron_types::chunks_from_blocks(&only_figure, 1200);
    assert!(
        chunks.iter().all(|chunk| !chunk.text.trim().is_empty()),
        "no chunk may have empty/whitespace text"
    );

    // Real text must still survive alongside text-less blocks.
    let mixed = vec![
        block("b0", "text", "Real paragraph about attention."),
        block("b1", "figure", ""),
        block("b2", "text", "Another real paragraph."),
    ];
    let chunks = zotron_types::chunks_from_blocks(&mixed, 1200);
    assert!(!chunks.is_empty(), "non-empty blocks must still produce chunks");
    assert!(
        chunks.iter().all(|chunk| !chunk.text.trim().is_empty()),
        "no chunk may have empty/whitespace text"
    );
}

#[test]
fn zotron_evidence_artifacts_are_detectable_for_search_filtering() {
    for title in [
        "ITEMKEY.zotron-ocr.raw.zip",
        "ITEMKEY.zotron-blocks.jsonl",
        "ITEMKEY.zotron-chunks.jsonl",
        "ITEMKEY.zotron-embed.npz",
    ] {
        assert!(
            is_zotron_evidence_artifact(title),
            "{title} should be filtered"
        );
    }

    assert!(!is_zotron_evidence_artifact("paper.pdf"));
    assert!(!is_zotron_evidence_artifact("notes-on-zotron.md"));
}
