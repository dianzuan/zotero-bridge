import zipfile
from unittest.mock import MagicMock

import numpy as np

from zotron.artifacts import (
    ArtifactMetadata,
    add_artifact_file,
    delete_artifact,
    find_artifact_by_suffix,
    is_artifact_stale,
    list_artifacts,
    read_blocks_jsonl,
    read_chunks_jsonl,
    read_embedding_npz,
    read_provider_raw_zip,
    write_blocks_jsonl,
    write_chunks_jsonl,
    write_embedding_npz,
    write_provider_raw_zip,
)


def test_zotero_artifact_helpers_find_add_delete_by_suffix(tmp_path):
    rpc = MagicMock()
    rpc.call.side_effect = [
        [
            {"id": 10, "title": "Paper.zotron-blocks.jsonl"},
            {"id": 11, "title": "Paper.pdf"},
        ],
        {"id": 12, "title": "Paper.zotron-chunks.jsonl"},
        {"ok": True},
    ]

    artifacts = list_artifacts(rpc, parent_id=1, suffix=".zotron-blocks.jsonl")
    assert [a["id"] for a in artifacts] == [10]
    assert find_artifact_by_suffix(rpc, parent_id=1, suffix=".zotron-blocks.jsonl")["id"] == 10

    path = tmp_path / "Paper.zotron-chunks.jsonl"
    path.write_text("{}\n", encoding="utf-8")
    created = add_artifact_file(rpc, parent_id=1, path=path, title=path.name)
    assert created["id"] == 12
    assert delete_artifact(rpc, artifact_id=12) == {"ok": True}
    assert rpc.call.call_args_list[-2].args == ("attachments.add", {"parentKey": 1, "path": str(path), "title": path.name})
    assert rpc.call.call_args_list[-1].args == ("attachments.delete", {"key": 12})


def test_provider_raw_zip_and_blocks_chunks_jsonl_roundtrip(tmp_path):
    raw_zip = tmp_path / "Paper.zotron-ocr.raw.zip"
    write_provider_raw_zip(
        raw_zip,
        provider="mineru",
        files={
            "content.json": {"pages": [{"page": 1, "text": "正文"}]},
            "nested/table.txt": "table text",
        },
    )
    with zipfile.ZipFile(raw_zip) as zf:
        assert "provider.json" in zf.namelist()
        assert "nested/table.txt" in zf.namelist()
    raw = read_provider_raw_zip(raw_zip)
    assert raw["provider"]["provider"] == "mineru"
    assert raw["files"]["content.json"] == {"pages": [{"page": 1, "text": "正文"}]}

    blocks = [{"block_id": "att:p1:b1", "type": "paragraph", "page": 1, "text": "正文"}]
    chunks = [{"chunk_id": "att:c0", "block_ids": ["att:p1:b1"], "text": "正文"}]
    blocks_path = tmp_path / "Paper.zotron-blocks.jsonl"
    chunks_path = tmp_path / "Paper.zotron-chunks.jsonl"
    write_blocks_jsonl(blocks_path, blocks)
    write_chunks_jsonl(chunks_path, chunks)
    assert read_blocks_jsonl(blocks_path) == blocks
    assert read_chunks_jsonl(chunks_path) == chunks


def test_embedding_npz_roundtrip_and_stale_metadata(tmp_path):
    path = tmp_path / "Paper.zotron-embed.npz"
    metadata = ArtifactMetadata(
        schema_version="1",
        source_sha256="chunks-sha",
        provider="openai",
        model="text-embedding-3-small",
        dim=2,
        config_sha256="cfg-sha",
    )
    vectors = np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32)
    write_embedding_npz(path, chunk_ids=["c1", "c2"], vectors=vectors, metadata=metadata)
    loaded = read_embedding_npz(path)
    assert loaded["chunk_ids"] == ["c1", "c2"]
    assert loaded["metadata"]["source_sha256"] == "chunks-sha"
    np.testing.assert_array_equal(loaded["vectors"], vectors)

    assert is_artifact_stale(loaded["metadata"], metadata) is False
    changed_model = ArtifactMetadata(**{**metadata.to_dict(), "model": "different"})
    assert is_artifact_stale(loaded["metadata"], changed_model) is True
