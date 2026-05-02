"""Tests for zotron.rag.citation."""
import json as _json

from zotron.rag.citation import (
    Citation,
    format_citation_markdown,
    format_citation_json,
)


def make_sample() -> Citation:
    return Citation(
        item_key="ABC123",
        attachment_id=42,
        title="数字经济与就业",
        authors="张三; 李四",
        section="第三章 实证分析",
        chunk_index=7,
        text="数字经济通过提升劳动生产率促进就业。",
        score=0.87,
    )


def test_citation_construction():
    c = make_sample()
    assert c.item_key == "ABC123"
    assert c.attachment_id == 42
    assert c.chunk_index == 7
    assert c.score == 0.87


def test_citation_zotero_uri():
    c = make_sample()
    assert c.zotero_uri() == "zotero://select/library/items/ABC123"


def test_format_citation_markdown_includes_provenance():
    c = make_sample()
    out = format_citation_markdown(c)
    assert "数字经济与就业" in out
    assert "张三" in out
    assert "ABC123" in out
    assert "第三章" in out
    assert "数字经济通过提升劳动生产率促进就业" in out
    assert "0.87" in out


def test_format_citation_json_round_trip():
    c = make_sample()
    out = format_citation_json(c)
    parsed = _json.loads(out)
    assert parsed["itemKey"] == "ABC123"
    assert parsed["attachmentId"] == 42
    assert parsed["chunkIndex"] == 7
    assert parsed["score"] == 0.87
    assert parsed["zoteroUri"] == "zotero://select/library/items/ABC123"
    assert parsed["text"].startswith("数字经济")


def test_retrieve_with_citations_returns_citation_list(tmp_path):
    """retrieve_with_citations() loads a VectorStore and returns Citation objects."""
    from unittest.mock import MagicMock
    from zotron.rag.citation import Citation, retrieve_with_citations
    from zotron.rag.search import VectorStore

    store = VectorStore(collection="test", collection_id="1", model="m")
    store.add_chunk(
        item_id="K1", title="T1", authors="A1", section="S1",
        chunk_index=0, text="hello world", vector=[1.0, 0.0], attachment_id=10,
    )
    store_path = tmp_path / "test.json"
    store.save(store_path)

    mock_embedder = MagicMock()
    mock_embedder.embed.return_value = [1.0, 0.0]

    citations = retrieve_with_citations(
        query="hello",
        store_path=store_path,
        embedder=mock_embedder,
        top_k=1,
    )
    assert len(citations) == 1
    c = citations[0]
    assert isinstance(c, Citation)
    assert c.item_key == "K1"
    assert c.attachment_id == 10
    assert c.chunk_index == 0
    assert c.text == "hello world"
    assert c.score > 0
