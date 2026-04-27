"""Tests for Zotero-native RAG/OCR artifact helpers."""

from __future__ import annotations

import json
import zipfile
from unittest.mock import MagicMock

import numpy as np

from zotron.artifacts import (
    ZoteroArtifactStore,
    find_stale_reasons,
    read_embedding_npz,
    read_jsonl,
    write_embedding_npz,
    write_jsonl,
    write_provider_raw_zip,
)


def test_zotero_artifact_store_filters_by_title_suffix_and_adds_attachment(tmp_path):
    rpc = MagicMock()

    def call(method, params=None):
        if method == "attachments.list":
            return [
                {"id": 1, "title": "ITEM.zotron-ocr.raw.zip"},
                {"id": 2, "title": "ITEM.zotron-blocks.jsonl"},
                {"id": 3, "title": "unrelated.pdf"},
            ]
        if method == "attachments.add":
            return {"id": 99, "title": params["title"]}
        raise AssertionError(method)

    rpc.call.side_effect = call
    store = ZoteroArtifactStore(rpc)

    assert store.list_artifacts(parent_id=42, suffix=".zotron-blocks.jsonl") == [
        {"id": 2, "title": "ITEM.zotron-blocks.jsonl"}
    ]
    assert store.find_artifact(parent_id=42, suffix=".zotron-ocr.raw.zip")["id"] == 1

    artifact = tmp_path / "ITEM.zotron-blocks.jsonl"
    artifact.write_text("{}\n", encoding="utf-8")
    added = store.add_artifact(parent_id=42, path=artifact, title=artifact.name)
    assert added["id"] == 99
    rpc.call.assert_any_call(
        "attachments.add",
        {"parentId": 42, "path": str(artifact), "title": "ITEM.zotron-blocks.jsonl"},
    )


def test_provider_raw_zip_preserves_named_json_and_binary_entries(tmp_path):
    target = tmp_path / "ITEM.zotron-ocr.raw.zip"
    digest = write_provider_raw_zip(
        target,
        {
            "glm-response.json": {"md_results": "# Title", "layout": [{"page": 1}]},
            "images/page-1.png": b"png-bytes",
        },
    )

    assert len(digest) == 64
    with zipfile.ZipFile(target) as zf:
        assert sorted(zf.namelist()) == ["glm-response.json", "images/page-1.png"]
        assert json.loads(zf.read("glm-response.json"))["layout"][0]["page"] == 1
        assert zf.read("images/page-1.png") == b"png-bytes"


def test_blocks_and_chunks_jsonl_roundtrip(tmp_path):
    rows = [
        {"block_id": "att:p1:b1", "item_key": "ITEM", "page": 1, "text": "正文"},
        {"block_id": "att:p1:b2", "item_key": "ITEM", "page": 1, "text": "续文"},
    ]
    path = tmp_path / "ITEM.zotron-blocks.jsonl"

    write_jsonl(path, rows)

    assert path.read_text(encoding="utf-8").count("\n") == 2
    assert read_jsonl(path) == rows


def test_embedding_npz_roundtrip_includes_metadata_and_vectors(tmp_path):
    path = tmp_path / "ITEM.zotron-embed.npz"
    vectors = np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32)
    metadata = {
        "schema_version": "1",
        "embedder_id": "openai:text-embedding-3-small",
        "embedder_dim": 2,
        "source_chunks_sha256": "abc123",
        "created_at": "2026-04-27T00:00:00Z",
    }

    write_embedding_npz(path, vectors=vectors, chunk_ids=["c1", "c2"], metadata=metadata)
    loaded = read_embedding_npz(path)

    assert loaded["metadata"] == metadata
    assert loaded["chunk_ids"] == ["c1", "c2"]
    np.testing.assert_allclose(loaded["vectors"], vectors)


def test_stale_metadata_reports_changed_inputs():
    current = {
        "pdf_sha256": "new-pdf",
        "provider_id": "glm",
        "ocr_model": "glm-ocr",
        "blocks_schema_version": "1",
        "chunking_config_sha256": "chunk-v2",
        "embedder_id": "voyage:v3",
        "embedder_dim": 1024,
    }
    stored = dict(current)
    stored["pdf_sha256"] = "old-pdf"
    stored["chunking_config_sha256"] = "chunk-v1"

    assert find_stale_reasons(stored, current) == [
        "pdf_sha256 changed",
        "chunking_config_sha256 changed",
    ]
