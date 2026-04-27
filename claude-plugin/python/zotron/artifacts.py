"""Zotero-native artifact helpers for OCR/RAG intermediate files.

The helpers in this module keep RAG source-of-truth data in auditable files
that can be attached back to the Zotero item: provider raw zips, normalized
blocks/chunks JSONL, and embedding NPZ files.
"""

from __future__ import annotations

import hashlib
import json
import zipfile
from collections.abc import Iterable, Mapping
from pathlib import Path
from typing import Any

import numpy as np


class ZoteroArtifactStore:
    """Small RPC wrapper for item-attached zotron artifacts."""

    def __init__(self, rpc: Any) -> None:
        self.rpc = rpc

    def list_artifacts(self, parent_id: int, suffix: str | None = None) -> list[dict[str, Any]]:
        attachments = self.rpc.call("attachments.list", {"parentId": parent_id}) or []
        if suffix is None:
            return list(attachments)
        return [a for a in attachments if str(a.get("title") or "").endswith(suffix)]

    def find_artifact(self, parent_id: int, suffix: str) -> dict[str, Any] | None:
        artifacts = self.list_artifacts(parent_id, suffix=suffix)
        return artifacts[0] if artifacts else None

    def add_artifact(self, parent_id: int, path: str | Path, title: str | None = None) -> dict[str, Any]:
        path = Path(path)
        return self.rpc.call(
            "attachments.add",
            {"parentId": parent_id, "path": str(path), "title": title or path.name},
        )

    def delete_artifact(self, attachment_id: int) -> Any:
        return self.rpc.call("items.delete", {"ids": [attachment_id]})


def _json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def write_provider_raw_zip(path: str | Path, entries: Mapping[str, Any]) -> str:
    """Write provider raw payloads to a zip and return its sha256 digest."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for name, value in entries.items():
            if isinstance(value, bytes):
                data = value
            elif isinstance(value, str):
                candidate = Path(value)
                data = candidate.read_bytes() if candidate.exists() else value.encode("utf-8")
            elif isinstance(value, Path):
                data = value.read_bytes()
            else:
                data = _json_bytes(value)
            zf.writestr(name, data)
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_jsonl(path: str | Path, rows: Iterable[Mapping[str, Any]]) -> str:
    """Write rows as UTF-8 JSONL and return the file sha256 digest."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(dict(row), ensure_ascii=False, sort_keys=True))
            fh.write("\n")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_jsonl(path: str | Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def write_embedding_npz(
    path: str | Path,
    *,
    vectors: np.ndarray,
    chunk_ids: list[str],
    metadata: Mapping[str, Any],
) -> str:
    """Write embedding vectors and metadata to the roadmap NPZ artifact."""
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(
        path,
        vectors=np.asarray(vectors, dtype=np.float32),
        chunk_ids=np.asarray(chunk_ids, dtype=object),
        metadata=np.asarray(json.dumps(dict(metadata), ensure_ascii=False, sort_keys=True), dtype=object),
    )
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_embedding_npz(path: str | Path) -> dict[str, Any]:
    with np.load(Path(path), allow_pickle=True) as data:
        metadata_raw = data["metadata"].item()
        return {
            "vectors": data["vectors"],
            "chunk_ids": [str(v) for v in data["chunk_ids"].tolist()],
            "metadata": json.loads(str(metadata_raw)),
        }


def find_stale_reasons(stored: Mapping[str, Any], current: Mapping[str, Any]) -> list[str]:
    """Compare persisted metadata against current inputs and explain staleness."""
    tracked = (
        "pdf_sha256",
        "provider_id",
        "ocr_model",
        "ocr_config_sha256",
        "blocks_schema_version",
        "chunking_config_sha256",
        "source_chunks_sha256",
        "embedder_id",
        "embedder_dim",
    )
    reasons: list[str] = []
    for key in tracked:
        if key in current and stored.get(key) != current.get(key):
            reasons.append(f"{key} changed")
    return reasons
