"""Tests for OCRProcessor — collection traversal, note detection, HTML output."""

from __future__ import annotations

from unittest.mock import MagicMock, patch


from zotron.ocr.processor import OCRProcessor


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_processor() -> tuple[OCRProcessor, MagicMock, MagicMock]:
    rpc = MagicMock()
    engine = MagicMock()
    processor = OCRProcessor(rpc=rpc, engine=engine)
    return processor, rpc, engine


# ---------------------------------------------------------------------------
# find_collection_id
# ---------------------------------------------------------------------------


def test_find_collection_id():
    """Flat tree — finds the collection by name and returns its key."""
    processor, rpc, _ = _make_processor()
    rpc.call.return_value = [
        {"key": "COL1", "name": "Alpha", "children": []},
        {"key": "COL2", "name": "Beta", "children": []},
        {"key": "COL3", "name": "Gamma", "children": []},
    ]
    result = processor.find_collection_id("Beta")
    assert result == "COL2"
    rpc.call.assert_called_once_with("collections.tree")


def test_find_collection_id_nested():
    """Nested tree — finds the collection inside children."""
    processor, rpc, _ = _make_processor()
    rpc.call.return_value = [
        {
            "key": "COL10",
            "name": "Parent",
            "children": [
                {"key": "COL11", "name": "Child A", "children": []},
                {
                    "key": "COL12",
                    "name": "Child B",
                    "children": [
                        {"key": "COL13", "name": "Grandchild", "children": []}
                    ],
                },
            ],
        }
    ]
    result = processor.find_collection_id("Grandchild")
    assert result == "COL13"


def test_find_collection_not_found():
    """Empty tree — returns None when collection does not exist."""
    processor, rpc, _ = _make_processor()
    rpc.call.return_value = []
    result = processor.find_collection_id("Missing")
    assert result is None


# ---------------------------------------------------------------------------
# has_ocr_note
# ---------------------------------------------------------------------------


def test_has_ocr_note_true():
    """Note with 'ocr' tag exists — returns True."""
    processor, rpc, _ = _make_processor()
    rpc.call.return_value = [
        {"id": 100, "note": "<p>some content</p>", "tags": ["ocr", "imported"]},
    ]
    assert processor.has_ocr_note(42) is True
    rpc.call.assert_called_once_with("notes.get", {"parentId": 42})


def test_has_ocr_note_false():
    """Note exists but no 'ocr' tag — returns False."""
    processor, rpc, _ = _make_processor()
    rpc.call.return_value = [
        {"id": 101, "note": "<p>manual note</p>", "tags": ["manual"]},
    ]
    assert processor.has_ocr_note(42) is False


def test_has_ocr_note_no_notes():
    """No notes at all — returns False."""
    processor, rpc, _ = _make_processor()
    rpc.call.return_value = []
    assert processor.has_ocr_note(42) is False


def test_has_ocr_result_prefers_chunk_artifacts():
    processor, rpc, _ = _make_processor()

    def call(method, params=None):
        if method == "attachments.list":
            return [{"title": "ITEM.zotron-chunks.jsonl"}]
        raise AssertionError(f"unexpected RPC method {method}")

    rpc.call.side_effect = call

    assert processor.has_ocr_result(42) is True
    rpc.call.assert_called_once_with("attachments.list", {"parentId": 42})


def test_process_item_skips_when_artifact_exists_without_preview_note():
    processor, rpc, engine = _make_processor()
    processor.write_preview_note = False

    def call(method, params=None):
        if method == "attachments.list":
            return [{"title": "ITEM.zotron-chunks.jsonl"}]
        raise AssertionError(f"unexpected RPC method {method}")

    rpc.call.side_effect = call

    assert processor.process_item(42, "Done", force=False) == "skipped"
    engine.ocr_pdf.assert_not_called()


# ---------------------------------------------------------------------------
# format_note_html
# ---------------------------------------------------------------------------


def test_format_note_html():
    """HTML output contains title, provider, and markdown-converted content."""
    processor, _, _ = _make_processor()
    markdown_input = "## Section\n\nHello **world**."
    html = processor.format_note_html(
        title="My Paper",
        markdown=markdown_input,
        provider="GLMEngine",
        page_count=12,
    )
    assert "<h1>OCR: My Paper</h1>" in html
    assert "GLMEngine" in html
    assert "12 pages" in html
    # markdown library converts ## → <h2> and **word** → <strong>
    assert "<h2>" in html
    assert "<strong>world</strong>" in html
    assert "<hr/>" in html


# ---------------------------------------------------------------------------
# Artifact pipeline
# ---------------------------------------------------------------------------


class _FakeOCRResult:
    provider = "mock-ocr"
    model = "mock-layout"
    raw_payload = {
        "pages": [
            {
                "page": 1,
                "blocks": [
                    {"type": "heading", "text": "Intro"},
                    {"type": "paragraph", "text": "Alpha", "bbox": [1, 2, 3, 4]},
                ],
            }
        ]
    }
    markdown = "# Intro\n\nAlpha"
    text = ""
    files = {"provider/page-1.md": "# Intro\n\nAlpha"}


def test_process_item_writes_zotero_artifact_pipeline(tmp_path):
    processor, rpc, engine = _make_processor()
    processor.artifact_dir = tmp_path
    pdf_path = tmp_path / "source.pdf"
    pdf_path.write_bytes(b"%PDF-1.4 fake")
    engine.ocr_pdf.return_value = _FakeOCRResult()

    def call(method, params=None):
        if method == "notes.get":
            return []
        if method == "items.get":
            return {"id": 42, "key": "ITEM1", "title": "My Paper"}
        if method == "attachments.list":
            return [{"id": 9, "key": "ATT1", "contentType": "application/pdf"}]
        if method == "attachments.getPath":
            return {"path": str(pdf_path)}
        if method == "attachments.add":
            return {"id": 100 + len(rpc.call.call_args_list), "title": params["title"]}
        if method == "notes.create":
            return {"id": 501}
        raise AssertionError(f"unexpected RPC method {method}")

    rpc.call.side_effect = call

    assert processor.process_item(42, "My Paper", force=True) == "ok"

    add_calls = [c.args[1] for c in rpc.call.call_args_list if c.args[0] == "attachments.add"]
    assert [c["title"] for c in add_calls] == [
        "ITEM1.zotron-ocr.raw.zip",
        "ITEM1.zotron-blocks.jsonl",
        "ITEM1.zotron-chunks.jsonl",
    ]
    assert all((tmp_path / title).exists() for title in [
        "ITEM1.zotron-ocr.raw.zip",
        "ITEM1.zotron-blocks.jsonl",
        "ITEM1.zotron-chunks.jsonl",
    ])
    blocks = (tmp_path / "ITEM1.zotron-blocks.jsonl").read_text(encoding="utf-8")
    chunks = (tmp_path / "ITEM1.zotron-chunks.jsonl").read_text(encoding="utf-8")
    assert '"block_id": "ATT1:p1:b1"' in blocks
    assert '"section_heading": "Intro"' in chunks
    assert any(c.args[0] == "notes.create" for c in rpc.call.call_args_list)


def test_process_item_can_skip_preview_note_while_writing_artifacts(tmp_path):
    processor, rpc, engine = _make_processor()
    processor.artifact_dir = tmp_path
    processor.write_preview_note = False
    pdf_path = tmp_path / "source.pdf"
    pdf_path.write_bytes(b"pdf")
    engine.ocr_pdf.return_value = "Plain OCR text"

    def call(method, params=None):
        if method == "items.get":
            return {"key": "ITEM2"}
        if method == "attachments.list":
            return [{"id": 10, "key": "ATT2", "contentType": "application/pdf"}]
        if method == "attachments.getPath":
            return str(pdf_path)
        if method == "attachments.add":
            return {"id": 200, "title": params["title"]}
        raise AssertionError(f"unexpected RPC method {method}")

    rpc.call.side_effect = call

    assert processor.process_item(43, "Plain", force=True) == "ok"
    assert not any(c.args[0] == "notes.create" for c in rpc.call.call_args_list)
    assert (tmp_path / "ITEM2.zotron-ocr.raw.zip").exists()


def test_attach_artifact_converts_path_for_zotero(tmp_path):
    processor, rpc, _ = _make_processor()
    artifact = tmp_path / "ITEM.zotron-chunks.jsonl"
    artifact.write_text("{}", encoding="utf-8")

    with patch("zotron.ocr.processor.zotero_path", return_value="\\\\wsl.localhost\\Ubuntu\\tmp\\ITEM.zotron-chunks.jsonl"):
        processor._attach_artifact(42, artifact)

    rpc.call.assert_called_once_with(
        "attachments.add",
        {
            "parentId": 42,
            "path": "\\\\wsl.localhost\\Ubuntu\\tmp\\ITEM.zotron-chunks.jsonl",
            "title": "ITEM.zotron-chunks.jsonl",
        },
    )
