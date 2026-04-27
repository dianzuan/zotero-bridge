from __future__ import annotations

from zotron.ocr.normalize import blocks_from_provider_payload, chunks_from_blocks


def test_blocks_from_structured_provider_payload_keeps_page_bbox_and_source_ref():
    payload = {
        "pages": [
            {
                "page": 2,
                "blocks": [
                    {
                        "type": "paragraph",
                        "text": "研究设计内容",
                        "bbox": [72, 210, 510, 286],
                        "confidence": 0.94,
                    }
                ],
            }
        ]
    }

    blocks = blocks_from_provider_payload(
        payload,
        item_key="ITEM1",
        attachment_key="ATT1",
        provider="mineru",
    )

    assert blocks == [
        {
            "block_id": "ATT1:p2:b0",
            "attachment_key": "ATT1",
            "item_key": "ITEM1",
            "type": "paragraph",
            "page": 2,
            "bbox": [72, 210, 510, 286],
            "reading_order": 0,
            "section_heading": "",
            "text": "研究设计内容",
            "caption": "",
            "image_ref": "",
            "source_provider": "mineru",
            "source_ref": "pages[0].blocks[0]",
            "confidence": 0.94,
        }
    ]


def test_blocks_from_markdown_payload_is_fallback_not_sole_truth():
    payload = {"markdown": "# 方法\n\n第一段。\n\n第二段。"}

    blocks = blocks_from_provider_payload(
        payload,
        item_key="ITEM1",
        attachment_key="ATT1",
        provider="glm",
    )

    assert [b["type"] for b in blocks] == ["heading", "paragraph", "paragraph"]
    assert blocks[0]["text"] == "方法"
    assert blocks[1]["section_heading"] == "方法"
    assert blocks[1]["source_ref"] == "markdown:1"


def test_chunks_from_blocks_preserves_block_ids_pages_and_section():
    blocks = [
        {"block_id": "ATT1:p1:b0", "item_key": "ITEM1", "attachment_key": "ATT1", "type": "paragraph", "page": 1, "section_heading": "Intro", "text": "Alpha"},
        {"block_id": "ATT1:p1:b1", "item_key": "ITEM1", "attachment_key": "ATT1", "type": "paragraph", "page": 1, "section_heading": "Intro", "text": "Beta"},
        {"block_id": "ATT1:p2:b0", "item_key": "ITEM1", "attachment_key": "ATT1", "type": "paragraph", "page": 2, "section_heading": "Methods", "text": "Gamma"},
    ]

    chunks = chunks_from_blocks(blocks, max_chars=20)

    assert chunks[0]["chunk_id"] == "ATT1:c0"
    assert chunks[0]["block_ids"] == ["ATT1:p1:b0", "ATT1:p1:b1"]
    assert chunks[0]["section_heading"] == "Intro"
    assert chunks[0]["page_start"] == 1
    assert chunks[0]["page_end"] == 1
    assert chunks[0]["text"] == "Alpha\n\nBeta"
    assert chunks[1]["section_heading"] == "Methods"
