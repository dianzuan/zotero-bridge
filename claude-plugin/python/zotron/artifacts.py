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
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import numpy as np


@dataclass(frozen=True)
class ArtifactMetadata:
    """Versioned metadata used to decide whether derived artifacts are stale."""

    schema_version: str
    source_sha256: str
    provider: str
    model: str
    dim: int
    config_sha256: str

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


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
        return self.rpc.call("attachments.delete", {"id": attachment_id})


def _json_bytes(value: Any) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")


def _metadata_dict(metadata: ArtifactMetadata | Mapping[str, Any]) -> dict[str, Any]:
    if isinstance(metadata, ArtifactMetadata):
        return metadata.to_dict()
    return dict(metadata)


def _zip_entry_bytes(value: Any) -> bytes:
    if isinstance(value, bytes):
        return value
    if isinstance(value, str):
        candidate = Path(value)
        return candidate.read_bytes() if candidate.exists() else value.encode("utf-8")
    if isinstance(value, Path):
        return value.read_bytes()
    return _json_bytes(value)


def list_artifacts(rpc: Any, *, parent_id: int, suffix: str | None = None) -> list[dict[str, Any]]:
    artifacts = rpc.call("attachments.list", {"parentId": parent_id}) or []
    artifacts = list(artifacts)
    setattr(rpc, "_zotron_last_artifacts", artifacts)
    if suffix is None:
        return artifacts
    return [artifact for artifact in artifacts if str(artifact.get("title") or "").endswith(suffix)]


def find_artifact_by_suffix(rpc: Any, *, parent_id: int, suffix: str) -> dict[str, Any] | None:
    cached = getattr(rpc, "_zotron_last_artifacts", None)
    artifacts = cached if cached is not None else list_artifacts(rpc, parent_id=parent_id)
    return next((artifact for artifact in artifacts if str(artifact.get("title") or "").endswith(suffix)), None)


def add_artifact_file(rpc: Any, *, parent_id: int, path: str | Path, title: str | None = None) -> dict[str, Any]:
    path = Path(path)
    return rpc.call("attachments.add", {"parentId": parent_id, "path": str(path), "title": title or path.name})


def delete_artifact(rpc: Any, *, artifact_id: int) -> Any:
    return rpc.call("attachments.delete", {"id": artifact_id})


def write_provider_raw_zip(
    path: str | Path,
    entries: Mapping[str, Any] | None = None,
    *,
    provider: str | None = None,
    files: Mapping[str, Any] | None = None,
) -> str:
    """Write provider raw payloads to a zip and return its sha256 digest."""
    payloads = files if files is not None else entries
    if payloads is None:
        raise ValueError("write_provider_raw_zip requires entries or files")
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        if provider is not None:
            zf.writestr("provider.json", _json_bytes({"provider": provider}))
        for name, value in payloads.items():
            zf.writestr(name, _zip_entry_bytes(value))
    return hashlib.sha256(path.read_bytes()).hexdigest()


def read_provider_raw_zip(path: str | Path) -> dict[str, Any]:
    files: dict[str, Any] = {}
    provider: dict[str, Any] = {}
    with zipfile.ZipFile(Path(path)) as zf:
        for name in zf.namelist():
            data = zf.read(name)
            if name.endswith(".json"):
                value: Any = json.loads(data.decode("utf-8"))
            else:
                try:
                    value = data.decode("utf-8")
                except UnicodeDecodeError:
                    value = data
            if name == "provider.json":
                provider = dict(value)
            else:
                files[name] = value
    return {"provider": provider, "files": files}


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


write_blocks_jsonl = write_jsonl
read_blocks_jsonl = read_jsonl
write_chunks_jsonl = write_jsonl
read_chunks_jsonl = read_jsonl


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
        metadata=np.asarray(json.dumps(_metadata_dict(metadata), ensure_ascii=False, sort_keys=True), dtype=object),
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
        "source_sha256",
        "embedder_id",
        "embedder_dim",
        "provider",
        "model",
        "dim",
        "config_sha256",
        "schema_version",
    )
    reasons: list[str] = []
    for key in tracked:
        if key in current and stored.get(key) != current.get(key):
            reasons.append(f"{key} changed")
    return reasons


def is_artifact_stale(stored: Mapping[str, Any], current: ArtifactMetadata | Mapping[str, Any]) -> bool:
    return bool(find_stale_reasons(stored, _metadata_dict(current)))
