"""Tests for `zotron-rag cite` subcommand."""
import json
import sys
from unittest.mock import patch, MagicMock

import pytest


def test_cite_prints_json_array_of_citations(tmp_path, capsys):
    """`zotron-rag cite <query> --collection X --output json` prints JSON array."""
    from zotron.rag.cli import main as rag_main
    from zotron.rag.search import VectorStore

    store_dir = tmp_path / "store"
    store_dir.mkdir()
    store = VectorStore(collection="test", collection_id="1", model="m")
    store.add_chunk(
        item_id="KEY1", title="T1", authors="A1", section="S1",
        chunk_index=0, text="answer text", vector=[1.0, 0.0], attachment_id=10,
    )
    store_path = store_dir / "test.json"
    store.save(store_path)

    with patch("zotron.rag.cli._store_path") as mock_path, \
         patch("zotron.rag.cli._build_embedder") as mock_eb:
        mock_path.return_value = store_path
        mock_emb = MagicMock()
        mock_emb.embed.return_value = [1.0, 0.0]
        mock_eb.return_value = mock_emb

        argv = ["zotron-rag", "cite", "answer", "--collection", "test", "--output", "json"]
        with patch.object(sys, "argv", argv):
            try:
                rag_main()
            except SystemExit as e:
                assert e.code in (0, None), f"main() exited with {e.code}"

    captured = capsys.readouterr()
    parsed = json.loads(captured.out)
    assert isinstance(parsed, list)
    assert len(parsed) == 1
    assert parsed[0]["itemKey"] == "KEY1"
    assert parsed[0]["attachmentId"] == 10
    assert parsed[0]["zoteroUri"] == "zotero://select/library/items/KEY1"


def test_cite_missing_index_errors_clearly(tmp_path, capsys):
    """If no index file exists for the collection, command exits non-zero with hint."""
    from zotron.rag.cli import main as rag_main

    nonexistent = tmp_path / "nope.json"
    with patch("zotron.rag.cli._store_path") as mock_path, \
         patch("zotron.rag.cli._build_embedder") as mock_eb:
        mock_path.return_value = nonexistent
        mock_eb.return_value = MagicMock()

        argv = ["zotron-rag", "cite", "q", "--collection", "missing"]
        with patch.object(sys, "argv", argv):
            with pytest.raises(SystemExit) as e:
                rag_main()
            assert e.value.code == 2

    captured = capsys.readouterr()
    combined = (captured.out or "") + (captured.err or "")
    assert "missing" in combined or "no index" in combined.lower()
