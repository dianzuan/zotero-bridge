use serde_json::json;
use std::fs;
use zotron_types::{
    blocks_from_provider_payload, chunks_from_blocks, embedding_provider_spec,
    find_machine_artifact, is_zotron_artifact_title, machine_artifact_exists_for_item,
    machine_artifact_persist_plan, machine_artifact_relative_path, ocr_provider_spec,
    read_machine_artifact, write_machine_artifact, MachineArtifactKind, MachineArtifactStorage,
};

#[test]
fn ocr_registry_exposes_key_first_contract_providers() {
    let glm = ocr_provider_spec("glm").expect("glm provider exists");
    assert_eq!(glm.id, "glm");
    assert_eq!(glm.request_style, "glm-layout-parsing");
    assert_eq!(glm.auth, "bearer");

    let paddle = ocr_provider_spec("paddle").expect("paddle alias exists");
    assert_eq!(paddle.id, "paddle");
    assert_eq!(paddle.request_style, "paddleocr-vl");

    let mineru = ocr_provider_spec("mineru").expect("mineru provider exists");
    assert_eq!(mineru.auth, "none");

    let err = ocr_provider_spec("missing").expect_err("unknown provider is explicit");
    assert!(err.contains("Unknown OCR provider"));
}

#[test]
fn embedding_registry_exposes_domestic_and_custom_contracts() {
    let volcengine = embedding_provider_spec("volcengine").expect("volcengine provider exists");
    assert_eq!(volcengine.id, "volcengine");
    assert_eq!(volcengine.request_style, "openai-compatible");
    assert_eq!(volcengine.default_model, "doubao-embedding-text-240715");

    let alibaba = embedding_provider_spec("alibaba").expect("alibaba provider exists");
    assert_eq!(alibaba.id, "alibaba");
    assert_eq!(alibaba.request_style, "dashscope");

    let custom = embedding_provider_spec("custom").expect("custom provider exists");
    assert_eq!(custom.base_url, None);
}

#[test]
fn provider_payload_normalizes_to_key_first_blocks() {
    let payload = json!({
        "pages": [{
            "page": 16,
            "blocks": [
                {"type": "title", "text": "二、模型设定", "bbox": [10, 20, 300, 60]},
                {"type": "text", "text": "风险平价配置框架需要宏观因子。", "bbox": [10, 70, 500, 120]},
                {"type": "table", "caption": "表1 描述统计", "text": "变量 均值 标准差", "bbox": [10, 130, 500, 220]}
            ]
        }]
    });

    let blocks = blocks_from_provider_payload(&payload, "ITEMKEY", "ATTACHKEY", "mineru");
    assert_eq!(blocks.len(), 3);
    assert_eq!(blocks[0].block_key, "ATTACHKEY:p16:b0");
    assert_eq!(blocks[1].section_path, vec!["二、模型设定"]);
    assert_eq!(blocks[2].block_type, "table");

    let serialized = serde_json::to_value(&blocks[0]).expect("block serializes");
    assert!(serialized.get("block_key").is_some());
    assert!(
        serialized.get("block_id").is_none(),
        "new artifacts must not expose block_id"
    );
    assert!(
        serialized.get("item_id").is_none(),
        "new artifacts must not expose item_id"
    );
    assert!(
        serialized.get("attachment_id").is_none(),
        "new artifacts must not expose attachment_id"
    );
}

#[test]
fn chunks_preserve_structure_without_crossing_sections() {
    let payload = json!({"blocks": [
        {"page": 1, "type": "heading", "text": "第一节 引言"},
        {"page": 1, "type": "paragraph", "text": "数字经济提升全要素生产率。"},
        {"page": 1, "type": "heading", "text": "第二节 机制"},
        {"page": 2, "type": "paragraph", "text": "机制包括创新激励和资源配置。"}
    ]});
    let blocks = blocks_from_provider_payload(&payload, "ITEM", "ATT", "glm");

    let chunks = chunks_from_blocks(&blocks, 128);
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].chunk_key, "ATT:c0");
    assert_eq!(chunks[0].section_path, vec!["第一节 引言"]);
    assert_eq!(chunks[0].block_keys, vec!["ATT:p1:b1"]);
    assert_eq!(chunks[1].section_path, vec!["第二节 机制"]);
    assert_eq!(chunks[1].page_start, Some(2));

    let serialized = serde_json::to_value(&chunks[0]).expect("chunk serializes");
    assert!(serialized.get("chunk_key").is_some());
    assert!(
        serialized.get("chunk_id").is_none(),
        "new artifacts must not expose chunk_id"
    );
    assert!(serialized.get("block_keys").is_some());
    assert!(
        serialized.get("block_ids").is_none(),
        "new artifacts must not expose block_ids"
    );
}

#[test]
fn chunks_respect_max_chars_within_a_section() {
    let payload = json!({"blocks": [
        {"page": 1, "type": "heading", "text": "第一节 引言"},
        {"page": 1, "type": "paragraph", "text": "数字经济通过数据要素改善匹配效率。"},
        {"page": 1, "type": "paragraph", "text": "平台治理进一步降低企业交易成本。"},
        {"page": 2, "type": "paragraph", "text": "这些机制共同推动全要素生产率提升。"}
    ]});
    let blocks = blocks_from_provider_payload(&payload, "ITEM", "ATT", "glm");

    let chunks = chunks_from_blocks(&blocks, 34);

    assert_eq!(
        chunks.len(),
        3,
        "paragraphs that fit individually must be split instead of producing an oversized section chunk"
    );
    assert_eq!(chunks[0].block_keys, vec!["ATT:p1:b1"]);
    assert_eq!(chunks[1].block_keys, vec!["ATT:p1:b2"]);
    assert_eq!(chunks[2].block_keys, vec!["ATT:p2:b3"]);
    assert!(
        chunks.iter().all(|chunk| chunk.text.chars().count() <= 34),
        "no emitted chunk should exceed max_chars when individual blocks fit"
    );
}

#[test]
fn chunks_split_before_section_boundaries_even_when_max_chars_has_room() {
    let payload = json!({"blocks": [
        {"page": 1, "type": "heading", "text": "第一节 引言"},
        {"page": 1, "type": "paragraph", "text": "数字经济提升匹配效率。"},
        {"page": 1, "type": "heading", "text": "第二节 机制"},
        {"page": 2, "type": "paragraph", "text": "机制包括创新激励。"},
        {"page": 2, "type": "paragraph", "text": "也包括资源重新配置。"}
    ]});
    let blocks = blocks_from_provider_payload(&payload, "ITEM", "ATT", "glm");

    let chunks = chunks_from_blocks(&blocks, 128);

    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].section_path, vec!["第一节 引言"]);
    assert_eq!(chunks[0].block_keys, vec!["ATT:p1:b1"]);
    assert_eq!(chunks[1].section_path, vec!["第二节 机制"]);
    assert_eq!(chunks[1].block_keys, vec!["ATT:p2:b3", "ATT:p2:b4"]);
}

#[test]
fn artifact_titles_are_detected_for_search_pollution_guards() {
    assert!(is_zotron_artifact_title("ITEM.zotron-ocr.raw.zip"));
    assert!(is_zotron_artifact_title("ITEM.zotron-blocks.jsonl"));
    assert!(is_zotron_artifact_title("ITEM.zotron-chunks.jsonl"));
    assert!(is_zotron_artifact_title("ITEM.zotron-embed.npz"));
    assert!(!is_zotron_artifact_title("Normal paper title"));
}

#[test]
fn machine_artifacts_have_external_store_paths_not_zotero_titles() {
    let path = machine_artifact_relative_path(
        "ITEMKEY",
        "ATTACHKEY",
        MachineArtifactKind::EmbeddingVectors,
    );

    assert_eq!(
        path.to_string_lossy().replace('\\', "/"),
        "items/ITEMKEY/attachments/ATTACHKEY/zotron-embed.npz"
    );
    assert!(
        !path.to_string_lossy().contains(".zotero"),
        "machine artifacts should live in Zotron's external artifact store"
    );
    assert!(
        !path.to_string_lossy().contains("storage"),
        "machine artifacts must not target Zotero storage paths"
    );
}

#[test]
fn machine_artifact_persistence_defaults_to_external_store_without_zotero_add() {
    let plan =
        machine_artifact_persist_plan("ITEMKEY", "ATTACHKEY", MachineArtifactKind::Chunks, false);

    assert_eq!(plan.storage, MachineArtifactStorage::ExternalStore);
    assert_eq!(
        plan.relative_path.to_string_lossy().replace('\\', "/"),
        "items/ITEMKEY/attachments/ATTACHKEY/zotron-chunks.jsonl"
    );
    assert_eq!(plan.file_name, "zotron-chunks.jsonl");
    assert_eq!(plan.zotero_attachment_title, None);
    assert!(
        !plan.should_call_zotero_attachments_add,
        "machine outputs must not call Zotero attachments.add by default"
    );

    let explicit =
        machine_artifact_persist_plan("ITEMKEY", "ATTACHKEY", MachineArtifactKind::Chunks, true);
    assert_eq!(explicit.storage, MachineArtifactStorage::ZoteroAttachment);
    assert_eq!(
        explicit.zotero_attachment_title.as_deref(),
        Some("ITEMKEY.zotron-chunks.jsonl")
    );
    assert!(explicit.should_call_zotero_attachments_add);
}

#[test]
fn external_machine_artifact_store_round_trips_without_zotero_state() {
    let root = std::env::temp_dir().join(format!(
        "zotron-artifact-store-{}-{}",
        std::process::id(),
        "roundtrip"
    ));
    let _ = fs::remove_dir_all(&root);

    let record = write_machine_artifact(
        &root,
        "ITEMKEY",
        "ATTACHKEY",
        MachineArtifactKind::Blocks,
        br#"{"block_key":"ATTACHKEY:p1:b0"}\n"#,
    )
    .expect("artifact writes");

    assert_eq!(
        record.relative_path.to_string_lossy().replace('\\', "/"),
        "items/ITEMKEY/attachments/ATTACHKEY/zotron-blocks.jsonl"
    );
    assert_eq!(
        read_machine_artifact(&root, "ITEMKEY", "ATTACHKEY", MachineArtifactKind::Blocks)
            .expect("artifact reads"),
        br#"{"block_key":"ATTACHKEY:p1:b0"}\n"#
    );
    assert!(
        find_machine_artifact(&root, "ITEMKEY", "ATTACHKEY", MachineArtifactKind::Blocks).is_some()
    );
    assert!(machine_artifact_exists_for_item(
        &root,
        "ITEMKEY",
        MachineArtifactKind::Blocks
    ));
    assert!(!machine_artifact_exists_for_item(
        &root,
        "ITEMKEY",
        MachineArtifactKind::EmbeddingVectors
    ));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn ocr_request_builders_emit_provider_specific_transport_contracts() {
    use zotron_types::{build_ocr_provider_request, OcrRequestInput};

    let input = OcrRequestInput {
        item_key: "ITEMKEY".to_string(),
        attachment_key: "ATTACHKEY".to_string(),
        file_name: "paper.pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        content_base64: "JVBERi0x".to_string(),
        local_path: Some("/tmp/paper.pdf".to_string()),
        output_dir: Some("/tmp/mineru-out".to_string()),
    };

    let glm = build_ocr_provider_request("glm", &input).expect("glm request builds");
    assert_eq!(glm.provider, "glm");
    assert_eq!(glm.style, "glm-layout-parsing");
    assert_eq!(glm.method, Some("POST"));
    assert_eq!(glm.auth_header, Some("Authorization"));
    assert_eq!(glm.key_field, "attachment_key");
    assert_eq!(glm.body["model"], "glm-ocr");
    assert_eq!(glm.body["file"], "data:application/pdf;base64,JVBERi0x");
    assert!(glm.body.get("attachment_key").is_none());

    let paddle = build_ocr_provider_request("paddleocr-vl", &input).expect("paddle request builds");
    assert_eq!(paddle.provider, "paddleocr-vl");
    assert_eq!(paddle.style, "paddleocr-vl");
    assert_eq!(paddle.method, Some("POST"));
    assert_eq!(paddle.body["file"], "JVBERi0x");
    assert_eq!(paddle.body["fileType"], 0);
    assert_eq!(paddle.body["useDocOrientationClassify"], false);
    assert!(paddle.body.get("messages").is_none());

    let mineru = build_ocr_provider_request("mineru", &input).expect("mineru request builds");
    assert_eq!(mineru.provider, "mineru");
    assert_eq!(mineru.style, "mineru-cli");
    assert_eq!(mineru.method, None);
    assert_eq!(mineru.auth_header, None);
    assert_eq!(
        mineru.command,
        vec!["mineru", "-p", "/tmp/paper.pdf", "-o", "/tmp/mineru-out"]
    );
    assert!(mineru.body.is_null());
}

#[test]
fn ocr_response_parsers_normalize_glm_paddle_and_mineru_outputs() {
    use zotron_types::parse_ocr_provider_response;

    let glm_payload = json!({
        "choices": [{"message": {"content": "{\"pages\":[{\"page\":2,\"blocks\":[{\"type\":\"title\",\"text\":\"一、引言\",\"bbox\":[1,2,3,4]},{\"type\":\"text\",\"text\":\"正文\"}]}]}"}}]
    });
    let glm_blocks = parse_ocr_provider_response("glm", &glm_payload, "ITEM", "ATT")
        .expect("glm response parses");
    assert_eq!(glm_blocks.len(), 2);
    assert_eq!(glm_blocks[0].block_key, "ATT:p2:b0");
    assert_eq!(glm_blocks[1].section_path, vec!["一、引言"]);

    let paddle_payload = json!({
        "pages": [{"page_idx": 3, "blocks": [{
            "block_label": "paragraph",
            "block_content": "Paddle 正文",
            "block_bbox": [10, 20, 30, 40]
        }]}]
    });
    let paddle_blocks = parse_ocr_provider_response("paddleocr-vl", &paddle_payload, "ITEM", "ATT")
        .expect("paddle response parses");
    assert_eq!(paddle_blocks[0].page_idx, 3);
    assert_eq!(paddle_blocks[0].block_type, "paragraph");
    assert_eq!(paddle_blocks[0].text, "Paddle 正文");
    assert_eq!(paddle_blocks[0].bbox, Some([10.0, 20.0, 30.0, 40.0]));

    let mineru_payload = json!({
        "content_list": [
            {"page_idx": 4, "type": "title", "text": "二、模型"},
            {"page_idx": 4, "type": "text", "text": "MinerU 正文", "bbox": [5, 6, 7, 8]}
        ]
    });
    let mineru_blocks = parse_ocr_provider_response("mineru", &mineru_payload, "ITEM", "ATT")
        .expect("mineru response parses");
    assert_eq!(mineru_blocks.len(), 2);
    assert_eq!(mineru_blocks[1].section_path, vec!["二、模型"]);
    assert_eq!(mineru_blocks[1].block_key, "ATT:p4:b1");
}

#[test]
fn embedding_request_builders_emit_cloud_and_custom_transport_contracts() {
    use zotron_types::{
        build_embedding_provider_request, EmbeddingChunkInput, EmbeddingRequestInput,
    };

    let input = EmbeddingRequestInput {
        item_key: "ITEMKEY".to_string(),
        chunks: vec![
            EmbeddingChunkInput {
                chunk_key: "ATT:c0".to_string(),
                text: "第一段".to_string(),
            },
            EmbeddingChunkInput {
                chunk_key: "ATT:c1".to_string(),
                text: "第二段".to_string(),
            },
        ],
        model: None,
        url: None,
        input_type: None,
    };

    let volcengine =
        build_embedding_provider_request("volcengine", &input).expect("volcengine request builds");
    assert_eq!(volcengine.provider, "volcengine");
    assert_eq!(volcengine.style, "openai-compatible");
    assert_eq!(volcengine.method, Some("POST"));
    assert!(volcengine.url.unwrap().contains("volces.com"));
    assert_eq!(volcengine.body["model"], "doubao-embedding-text-240715");
    assert_eq!(volcengine.body["input"], json!(["第一段", "第二段"]));
    assert_eq!(volcengine.body["chunk_keys"], json!(["ATT:c0", "ATT:c1"]));

    let alibaba =
        build_embedding_provider_request("alibaba", &input).expect("alibaba request builds");
    assert_eq!(alibaba.provider, "alibaba");
    assert_eq!(alibaba.style, "dashscope");
    assert!(alibaba.url.unwrap().contains("dashscope.aliyuncs.com"));
    assert_eq!(alibaba.body["input_type"], "document");

    let custom = build_embedding_provider_request(
        "custom",
        &EmbeddingRequestInput {
            model: Some("bge-m3".to_string()),
            url: Some("https://example.test/v1/embeddings".to_string()),
            ..input
        },
    )
    .expect("custom request builds");
    assert_eq!(custom.provider, "custom");
    assert_eq!(custom.style, "openai-compatible");
    assert_eq!(
        custom.url.as_deref(),
        Some("https://example.test/v1/embeddings")
    );
    assert_eq!(custom.body["model"], "bge-m3");
}

#[test]
fn embedding_response_parsers_preserve_chunk_keys_for_openai_and_dashscope_shapes() {
    use zotron_types::{parse_embedding_provider_response, EmbeddingChunkInput};

    let chunks = vec![
        EmbeddingChunkInput {
            chunk_key: "ATT:c0".to_string(),
            text: "第一段".to_string(),
        },
        EmbeddingChunkInput {
            chunk_key: "ATT:c1".to_string(),
            text: "第二段".to_string(),
        },
    ];

    let openai_payload = json!({
        "model": "doubao-embedding-text-240715",
        "data": [
            {"index": 1, "embedding": [0.3, 0.4]},
            {"index": 0, "embedding": [0.1, 0.2]}
        ]
    });
    let volcengine =
        parse_embedding_provider_response("volcengine", &openai_payload, "ITEMKEY", &chunks)
            .expect("volcengine embeddings parse");
    assert_eq!(volcengine[0].chunk_key, "ATT:c1");
    assert_eq!(volcengine[0].item_key, "ITEMKEY");
    assert_eq!(volcengine[0].source_provider, "volcengine");
    assert_eq!(volcengine[0].vector, vec![0.3, 0.4]);
    assert_eq!(
        volcengine[0].model.as_deref(),
        Some("doubao-embedding-text-240715")
    );

    let dashscope_payload = json!({
        "output": {"embeddings": [
            {"text_index": 0, "embedding": [0.5, 0.6]},
            {"text_index": 1, "embedding": [0.7, 0.8]}
        ]},
        "model": "text-embedding-v4"
    });
    let alibaba =
        parse_embedding_provider_response("alibaba", &dashscope_payload, "ITEMKEY", &chunks)
            .expect("dashscope embeddings parse");
    assert_eq!(alibaba[0].chunk_key, "ATT:c0");
    assert_eq!(alibaba[1].vector, vec![0.7, 0.8]);

    let err = parse_embedding_provider_response("custom", &json!({"data": []}), "ITEMKEY", &chunks)
        .expect_err("empty embeddings are explicit");
    assert!(err.contains("did not contain parseable embeddings"));
}

#[test]
fn academic_zh_hits_accept_legacy_id_fields_but_serialize_key_first() {
    use zotron_types::AcademicZhHit;

    let hit: AcademicZhHit = serde_json::from_value(json!({
        "item_key": "ITEMKEY",
        "title": "Title",
        "text": "Text",
        "authors": ["Author"],
        "zotero_uri": "zotero://select/library/items/ITEMKEY",
        "section_heading": "Section",
        "chunk_id": "ATT:c0",
        "query": "query",
        "score": 1.0,
        "block_ids": ["ATT:p1:b1"]
    }))
    .expect("legacy id fields deserialize through aliases");

    assert_eq!(hit.chunk_key, "ATT:c0");
    assert_eq!(hit.block_keys, vec!["ATT:p1:b1"]);

    let serialized = serde_json::to_value(&hit).expect("hit serializes");
    assert_eq!(serialized["chunk_key"], "ATT:c0");
    assert_eq!(serialized["block_keys"], json!(["ATT:p1:b1"]));
    assert!(serialized.get("chunk_id").is_none());
    assert!(serialized.get("block_ids").is_none());
}
