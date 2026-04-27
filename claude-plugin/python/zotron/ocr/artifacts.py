"""Zotero-native artifact helpers for OCR and RAG storage.

The roadmap treats Zotero as the durable evidence store.  These helpers keep
provider raw payloads, normalized blocks/chunks, and embedding vectors in
separate artifacts so raw OCR output remains auditable and normalized data can
be rebuilt without re-running OCR.
"""
from __future__ import annotations

import hashlib
import json
import zipfile
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Iterable

import numpy as np


@dataclass(frozen=True)
class ProviderRawArtifact:
    """Provider raw OCR payload plus sidecar files for zip storage."""

    item_key: str
    attachment_key: str
    provider: str
    payload: Any
    files: dict[str, str | bytes] = field(default_factory=dict)
    source_path: str | None = None
    created_at: str = field(
        default_factory=lambda: datetime.now(timezone.utc).isoformat()
    )

    def manifest(self) -> dict[str, Any]:
        return {
            "item_key": self.item_key,
            "attachment_key": self.attachment_key,
            "provider": self.provider,
            "source_path": self.source_path,
            "created_at": self.created_at,
            "files": sorted(self.files),
        }


def _artifact_path(directory: Path, item_key: str, suffix: str) -> Path:
    safe_key = item_key.replace("/", "_").replace("\\", "_")
    return Path(directory).expanduser() / f"{safe_key}.{suffix}"


def write_provider_raw_zip(directory: Path, artifact: ProviderRawArtifact) -> Path:
    """Write ``<item-key>.zotron-ocr.raw.zip`` and return its path."""
    path = _artifact_path(directory, artifact.item_key, "zotron-ocr.raw.zip")
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        zf.writestr(
            "manifest.json",
            json.dumps(artifact.manifest(), ensure_ascii=False, indent=2),
        )
        zf.writestr(
            "provider_raw.json",
            json.dumps(artifact.payload, ensure_ascii=False, indent=2),
        )
        for name, content in artifact.files.items():
            if name.startswith("/") or ".." in Path(name).parts:
                raise ValueError(f"unsafe artifact member path: {name!r}")
            zf.writestr(name, content)
    return path


def _write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, sort_keys=True))
            fh.write("\n")
    return path


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open(encoding="utf-8") as fh:
        for line in fh:
            if line.strip():
                rows.append(json.loads(line))
    return rows


def write_blocks_jsonl(directory: Path, item_key: str, blocks: Iterable[dict[str, Any]]) -> Path:
    return _write_jsonl(_artifact_path(directory, item_key, "zotron-blocks.jsonl"), blocks)


def read_blocks_jsonl(path: Path) -> list[dict[str, Any]]:
    return _read_jsonl(path)


def write_chunks_jsonl(directory: Path, item_key: str, chunks: Iterable[dict[str, Any]]) -> Path:
    return _write_jsonl(_artifact_path(directory, item_key, "zotron-chunks.jsonl"), chunks)


def read_chunks_jsonl(path: Path) -> list[dict[str, Any]]:
    return _read_jsonl(path)


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def metadata_for_chunks(chunks: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    metadata: list[dict[str, Any]] = []
    for chunk in chunks:
        row = {k: v for k, v in chunk.items() if k != "text"}
        row["text_sha256"] = text_sha256(str(chunk.get("text", "")))
        metadata.append(row)
    return metadata


def write_embedding_npz(
    directory: Path,
    item_key: str,
    vectors: np.ndarray,
    metadata: list[dict[str, Any]],
    *,
    model: str,
) -> Path:
    """Write ``<item-key>.zotron-embed.npz`` with vectors and JSON metadata."""
    path = _artifact_path(directory, item_key, "zotron-embed.npz")
    path.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(
        path,
        vectors=np.asarray(vectors, dtype=np.float32),
        metadata_json=json.dumps(metadata, ensure_ascii=False),
        model=model,
    )
    return path


def read_embedding_npz(path: Path) -> tuple[np.ndarray, list[dict[str, Any]], str]:
    with np.load(path, allow_pickle=False) as data:
        vectors = data["vectors"]
        metadata = json.loads(str(data["metadata_json"]))
        model = str(data["model"])
    return vectors, metadata, model


def is_metadata_stale(metadata: list[dict[str, Any]], chunks: list[dict[str, Any]]) -> bool:
    """Return True when embedding metadata no longer matches chunk text/order."""
    if len(metadata) != len(chunks):
        return True
    for meta, chunk in zip(metadata, chunks):
        if meta.get("chunk_id") != chunk.get("chunk_id"):
            return True
        expected = meta.get("text_sha256")
        if expected and expected != text_sha256(str(chunk.get("text", ""))):
            return True
    return False
