"""CLI tests for Zotero XPI-backed retrieval hits."""

from __future__ import annotations

import json
import sys
from unittest.mock import MagicMock, patch

import numpy as np

from zotron.artifacts import read_embedding_npz, write_chunks_jsonl
from zotron.rag.cli import main as rag_main


def test_rag_hits_zotero_backend_calls_search_hits_rpc(capsys):
    rpc = MagicMock()
    rpc.call.return_value = {
        "hits": [
            {
                "item_key": "ITEM1",
                "title": "Paper",
                "text": "matched span",
                "chunk_id": "ITEM1:c1",
                "query": "trade risk",
                "score": 1.0,
            }
        ],
        "total": 1,
    }

    cfg = {"zotero": {"rpc_url": "http://zotero.test/rpc"}}
    with patch("zotron.rag.cli.load_config", return_value=cfg), \
         patch("zotron.rag.cli.ZoteroRPC", return_value=rpc) as rpc_cls, \
         patch.object(sys, "argv", [
             "zotron-rag",
             "hits",
             "trade risk",
             "--collection",
             "中国工业经济",
             "--zotero",
             "--limit",
             "5",
             "--top-spans-per-item",
             "2",
             "--include-fulltext-spans",
             "--output",
             "jsonl",
         ]):
        rag_main()

    rpc_cls.assert_called_once_with("http://zotero.test/rpc")
    rpc.call.assert_called_once_with(
        "rag.searchHits",
        {
            "query": "trade risk",
            "collection": "中国工业经济",
            "limit": 5,
            "top_spans_per_item": 2,
            "include_fulltext_spans": True,
        },
    )
    rows = [json.loads(line) for line in capsys.readouterr().out.splitlines()]
    assert rows == [
        {
            "item_key": "ITEM1",
            "title": "Paper",
            "text": "matched span",
            "chunk_id": "ITEM1:c1",
            "query": "trade risk",
            "score": 1.0,
        }
    ]


def test_rag_hits_zotero_backend_requires_collection(capsys):
    with patch("zotron.rag.cli.load_config", return_value={}), \
         patch("zotron.rag.cli.ZoteroRPC") as rpc_cls, \
         patch.object(sys, "argv", [
             "zotron-rag",
             "hits",
             "trade risk",
             "--zotero",
         ]):
        try:
            rag_main()
        except SystemExit as exc:
            assert exc.code == 2
        else:  # pragma: no cover - documents expected CLI exit behavior
            raise AssertionError("--zotero without collection should exit non-zero")

    rpc_cls.assert_not_called()
    assert "--collection is required" in capsys.readouterr().err


def test_rag_index_artifacts_zotero_item_embeds_attached_chunks(tmp_path, capsys):
    chunks = [
        {
            "item_key": "ITEM1",
            "title": "Paper",
            "text": "first Zotero span",
            "chunk_id": "ITEM1:c1",
            "block_ids": ["b1"],
        },
        {
            "item_key": "ITEM1",
            "title": "Paper",
            "text": "second Zotero span",
            "chunk_id": "ITEM1:c2",
            "block_ids": ["b2"],
        },
    ]
    chunks_path = write_chunks_jsonl(tmp_path, "ITEM1", chunks)

    rpc = MagicMock()
    rpc.call.side_effect = [
        {"id": 5443, "key": "ITEM1", "title": "Paper"},
        [{"id": 9001, "title": "ITEM1.zotron-chunks.jsonl", "path": str(chunks_path)}],
        {"id": 9100, "title": "ITEM1.zotron-embed.npz"},
    ]
    mock_embedder = MagicMock()
    mock_embedder.embed_batch.return_value = [[1.0, 0.0], [0.0, 1.0]]

    with patch("zotron.rag.cli.load_config", return_value={"zotero": {"rpc_url": "http://rpc"}}), \
         patch("zotron.rag.cli.ZoteroRPC", return_value=rpc), \
         patch("zotron.rag.cli._build_embedder", return_value=mock_embedder), \
         patch("zotron.rag.cli.zotero_path", side_effect=lambda path: str(path)), \
         patch.object(sys, "argv", [
             "zotron-rag",
             "index-artifacts",
             "--zotero",
             "--item",
             "5443",
             "--model",
             "test-embedding",
         ]):
        rag_main()

    mock_embedder.embed_batch.assert_called_once_with(["first Zotero span", "second Zotero span"])
    add_call = rpc.call.call_args_list[-1]
    assert add_call.args[0] == "attachments.add"
    add_params = add_call.args[1]
    assert add_params["parentId"] == 5443
    assert add_params["title"] == "ITEM1.zotron-embed.npz"

    vectors, metadata, model = read_embedding_npz(add_params["path"])
    np.testing.assert_allclose(vectors, np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32))
    assert [row["chunk_id"] for row in metadata] == ["ITEM1:c1", "ITEM1:c2"]
    assert model == "test-embedding"

    out = json.loads(capsys.readouterr().out)
    assert out["status"] == "ok"
    assert out["indexed"] == 1
    assert out["attached"] == 1
    assert out["total_chunks"] == 2
    assert out["items"][0]["item_id"] == 5443
    assert out["items"][0]["embedding_title"] == "ITEM1.zotron-embed.npz"
