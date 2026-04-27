"""CLI tests for Zotero-native artifact-backed RAG hits."""

from __future__ import annotations

import json
import sys
from unittest.mock import MagicMock, patch

import numpy as np

from zotron.artifacts import read_embedding_npz, write_chunks_jsonl, write_embedding_npz
from zotron.rag.cli import main as rag_main


def test_rag_index_artifacts_writes_item_embedding_npz(tmp_path, capsys):
    chunks = [
        {
            "item_key": "ITEM1",
            "title": "Artifact Paper",
            "text": "first span",
            "section_heading": "Intro",
            "chunk_id": "ITEM1:c1",
            "block_ids": ["ITEM1:b1"],
        },
        {
            "item_key": "ITEM1",
            "title": "Artifact Paper",
            "text": "second span",
            "section_heading": "Methods",
            "chunk_id": "ITEM1:c2",
            "block_ids": ["ITEM1:b2"],
        },
    ]
    write_chunks_jsonl(tmp_path, "ITEM1", chunks)

    mock_embedder = MagicMock()
    mock_embedder.embed_batch.return_value = [[1.0, 0.0], [0.0, 1.0]]

    with patch("zotron.rag.cli._build_embedder", return_value=mock_embedder), patch.object(
        sys,
        "argv",
        [
            "zotron-rag",
            "index-artifacts",
            "--artifacts-dir",
            str(tmp_path),
            "--item-key",
            "ITEM1",
            "--model",
            "test-embedding",
        ],
    ):
        rag_main()

    out = json.loads(capsys.readouterr().out)
    assert out["status"] == "ok"
    assert out["indexed"] == 1
    assert out["total_chunks"] == 2
    assert out["embedding_path"].endswith("ITEM1.zotron-embed.npz")

    vectors, metadata, model = read_embedding_npz(tmp_path / "ITEM1.zotron-embed.npz")
    np.testing.assert_allclose(vectors, np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32))
    assert model == "test-embedding"
    assert [row["chunk_id"] for row in metadata] == ["ITEM1:c1", "ITEM1:c2"]
    assert "text" not in metadata[0]


def test_rag_hits_reads_artifact_chunks_and_embeddings_as_jsonl(tmp_path, capsys):
    chunks = [
        {
            "item_key": "ITEM1",
            "title": "Artifact Paper",
            "authors": ["Author A"],
            "year": 2026,
            "text": "matched span",
            "section_heading": "Findings",
            "chunk_id": "ITEM1:c1",
            "block_ids": ["ITEM1:p1:b1"],
        },
        {
            "item_key": "ITEM1",
            "title": "Artifact Paper",
            "text": "unmatched span",
            "section_heading": "Appendix",
            "chunk_id": "ITEM1:c2",
            "block_ids": ["ITEM1:p9:b1"],
        },
    ]
    write_chunks_jsonl(tmp_path, "ITEM1", chunks)
    write_embedding_npz(
        tmp_path,
        "ITEM1",
        vectors=np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32),
        metadata=[{k: v for k, v in row.items() if k != "text"} for row in chunks],
        model="test-embedding",
    )

    mock_embedder = MagicMock()
    mock_embedder.embed.return_value = [1.0, 0.0]

    with patch("zotron.rag.cli._build_embedder", return_value=mock_embedder), patch.object(
        sys,
        "argv",
        [
            "zotron-rag",
            "hits",
            "artifact query",
            "--artifacts-dir",
            str(tmp_path),
            "--item-key",
            "ITEM1",
            "--limit",
            "1",
            "--output",
            "jsonl",
        ],
    ):
        rag_main()

    rows = [json.loads(line) for line in capsys.readouterr().out.splitlines()]
    assert rows == [
        {
            "item_key": "ITEM1",
            "title": "Artifact Paper",
            "text": "matched span",
            "authors": ["Author A"],
            "year": 2026,
            "zotero_uri": "zotero://select/library/items/ITEM1",
            "section_heading": "Findings",
            "chunk_id": "ITEM1:c1",
            "block_ids": ["ITEM1:p1:b1"],
            "query": "artifact query",
            "score": 1.0,
        }
    ]
