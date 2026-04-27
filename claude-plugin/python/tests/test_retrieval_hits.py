import json
import sys
from unittest.mock import MagicMock, patch

from zotron.rag.search import VectorStore, format_retrieval_hit, write_hits_jsonl


def test_format_retrieval_hit_contract_contains_minimum_and_provenance():
    row = {
        "item_id": "ITEM1",
        "item_key": "ITEM1",
        "title": "产业贸易中心性、贸易外向度与金融风险",
        "authors": ["王姝黛", "杨子荣"],
        "year": 2022,
        "venue": "中国工业经济",
        "section_heading": "三、研究设计",
        "chunk_id": "ATT:c0",
        "block_ids": ["ATT:p1:b1"],
        "text": "本文利用世界投入产出表...",
        "score": 0.82,
    }
    hit = format_retrieval_hit(row, query="贸易中心性")
    assert hit["item_key"] == "ITEM1"
    assert hit["title"]
    assert hit["text"]
    assert hit["chunk_id"] == "ATT:c0"
    assert hit["block_ids"] == ["ATT:p1:b1"]
    assert hit["query"] == "贸易中心性"
    assert hit["zotero_uri"] == "zotero://select/library/items/ITEM1"


def test_hits_jsonl_writer_outputs_one_hit_per_line(tmp_path):
    path = tmp_path / "hits.jsonl"
    write_hits_jsonl(path, [{"item_key": "I", "title": "T", "text": "span"}])
    lines = path.read_text(encoding="utf-8").splitlines()
    assert len(lines) == 1
    assert json.loads(lines[0]) == {"item_key": "I", "title": "T", "text": "span"}


def test_zotron_rag_hits_jsonl_cli_keeps_search_store_compatible(tmp_path, capsys):
    from zotron.rag.cli import main as rag_main

    store = VectorStore(collection="test", collection_id=1, model="m")
    store.add_chunk(
        item_id="ITEM1",
        title="T1",
        authors="A1",
        section="S1",
        chunk_index=0,
        text="answer text",
        vector=[1.0, 0.0],
        attachment_id=10,
        chunk_id="ATT:c0",
        block_ids=["ATT:p1:b1"],
    )
    store_path = tmp_path / "test.json"
    store.save(store_path)

    with patch("zotron.rag.cli._store_path", return_value=store_path), patch("zotron.rag.cli._build_embedder") as mock_eb:
        mock_emb = MagicMock()
        mock_emb.embed.return_value = [1.0, 0.0]
        mock_eb.return_value = mock_emb
        argv = ["zotron-rag", "hits", "answer", "--collection", "test", "--output", "jsonl"]
        with patch.object(sys, "argv", argv):
            rag_main()

    line = capsys.readouterr().out.strip()
    hit = json.loads(line)
    assert hit["item_key"] == "ITEM1"
    assert hit["title"] == "T1"
    assert hit["text"] == "answer text"
    assert hit["chunk_id"] == "ATT:c0"
    assert hit["block_ids"] == ["ATT:p1:b1"]
