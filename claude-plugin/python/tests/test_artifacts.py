from __future__ import annotations

import json
import zipfile

import numpy as np

from zotron.ocr.artifacts import (
    ProviderRawArtifact,
    is_metadata_stale,
    read_blocks_jsonl,
    read_chunks_jsonl,
    read_embedding_npz,
    write_blocks_jsonl,
    write_chunks_jsonl,
    write_embedding_npz,
    write_provider_raw_zip,
)


def test_provider_raw_artifact_zip_preserves_payload_and_manifest(tmp_path):
    artifact = ProviderRawArtifact(
        item_key="ITEM1",
        attachment_key="ATT1",
        provider="mineru",
        payload={"layout": [{"text": "hello", "page": 1}]},
        files={"markdown/page-1.md": "# Hello"},
        source_path="mineru-output.zip",
    )

    out = write_provider_raw_zip(tmp_path, artifact)

    assert out.name == "ITEM1.zotron-ocr.raw.zip"
    with zipfile.ZipFile(out) as zf:
        names = set(zf.namelist())
        assert names == {"manifest.json", "provider_raw.json", "markdown/page-1.md"}
        manifest = json.loads(zf.read("manifest.json"))
        assert manifest["item_key"] == "ITEM1"
        assert manifest["attachment_key"] == "ATT1"
        assert manifest["provider"] == "mineru"
        assert json.loads(zf.read("provider_raw.json"))["layout"][0]["text"] == "hello"


def test_blocks_and_chunks_jsonl_roundtrip(tmp_path):
    blocks = [
        {
            "block_id": "ATT1:p1:b1",
            "item_key": "ITEM1",
            "attachment_key": "ATT1",
            "type": "paragraph",
            "page": 1,
            "bbox": [1, 2, 3, 4],
            "reading_order": 1,
            "section_heading": "Intro",
            "text": "Alpha",
            "source_provider": "glm",
            "source_ref": "layout:0",
        }
    ]
    chunks = [
        {
            "chunk_id": "ATT1:c0",
            "item_key": "ITEM1",
            "attachment_key": "ATT1",
            "block_ids": ["ATT1:p1:b1"],
            "section_heading": "Intro",
            "page_start": 1,
            "page_end": 1,
            "text": "Alpha",
            "char_start": 0,
            "char_end": 5,
            "level": "chunk",
        }
    ]

    block_path = write_blocks_jsonl(tmp_path, "ITEM1", blocks)
    chunk_path = write_chunks_jsonl(tmp_path, "ITEM1", chunks)

    assert block_path.name == "ITEM1.zotron-blocks.jsonl"
    assert chunk_path.name == "ITEM1.zotron-chunks.jsonl"
    assert read_blocks_jsonl(block_path) == blocks
    assert read_chunks_jsonl(chunk_path) == chunks


def test_embedding_npz_roundtrip_and_stale_metadata(tmp_path):
    vectors = np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32)
    metadata = [
        {"chunk_id": "ATT1:c0", "item_key": "ITEM1", "text_sha256": "old"},
        {"chunk_id": "ATT1:c1", "item_key": "ITEM1", "text_sha256": "fresh"},
    ]

    path = write_embedding_npz(tmp_path, "ITEM1", vectors, metadata, model="test-emb")
    loaded_vectors, loaded_meta, model = read_embedding_npz(path)

    assert path.name == "ITEM1.zotron-embed.npz"
    np.testing.assert_array_equal(loaded_vectors, vectors)
    assert loaded_meta == metadata
    assert model == "test-emb"
    chunks = [
        {"chunk_id": "ATT1:c0", "text": "changed text"},
        {"chunk_id": "ATT1:c1", "text": "fresh"},
    ]
    assert is_metadata_stale(loaded_meta, chunks) is True
