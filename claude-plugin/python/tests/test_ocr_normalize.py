from zotron.ocr.normalize import build_chunks_from_blocks, normalize_provider_raw


def test_normalize_structured_provider_blocks_preserves_provenance():
    raw = {
        "blocks": [
            {
                "type": "paragraph",
                "page": 2,
                "bbox": [1, 2, 3, 4],
                "text": "结构化正文",
                "confidence": 0.9,
            }
        ]
    }
    blocks = normalize_provider_raw(
        provider="mineru",
        raw=raw,
        item_key="ITEM1",
        attachment_key="ATT1",
    )
    assert blocks[0]["block_id"] == "ATT1:p2:b0"
    assert blocks[0]["item_key"] == "ITEM1"
    assert blocks[0]["source_provider"] == "mineru"
    assert blocks[0]["bbox"] == [1, 2, 3, 4]
    assert blocks[0]["confidence"] == 0.9


def test_normalize_markdown_fallback_is_not_required_for_structured_raw():
    raw = {"pages": [{"page": 1, "markdown": "# 方法\n\n第一段。\n\n第二段。"}]}
    blocks = normalize_provider_raw(
        provider="vlm",
        raw=raw,
        item_key="ITEM1",
        attachment_key="ATT1",
    )
    assert [b["type"] for b in blocks] == ["heading", "paragraph", "paragraph"]
    assert blocks[0]["section_heading"] == "方法"
    assert blocks[1]["section_heading"] == "方法"
    assert blocks[1]["source_ref"] == "pages[0].markdown"


def test_build_chunks_from_blocks_does_not_cross_sections_and_keeps_block_ids():
    blocks = [
        {"block_id": "b1", "item_key": "I", "attachment_key": "A", "type": "heading", "page": 1, "section_heading": "一、引言", "text": "一、引言"},
        {"block_id": "b2", "item_key": "I", "attachment_key": "A", "type": "paragraph", "page": 1, "section_heading": "一、引言", "text": "引言内容"},
        {"block_id": "b3", "item_key": "I", "attachment_key": "A", "type": "heading", "page": 2, "section_heading": "二、方法", "text": "二、方法"},
        {"block_id": "b4", "item_key": "I", "attachment_key": "A", "type": "paragraph", "page": 2, "section_heading": "二、方法", "text": "方法内容"},
    ]
    chunks = build_chunks_from_blocks(blocks, max_chars=100)
    assert [c["section_heading"] for c in chunks] == ["一、引言", "二、方法"]
    assert chunks[0]["block_ids"] == ["b2"]
    assert chunks[1]["block_ids"] == ["b4"]
    assert chunks[0]["page_start"] == 1
    assert chunks[1]["page_start"] == 2
