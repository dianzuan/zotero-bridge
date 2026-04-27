"""Zotero-native RAG/OCR artifact helpers.

The roadmap treats Zotero child attachments as the durable storage surface for
provider raw returns, normalized blocks/chunks, and embedding arrays.  This
module keeps the RPC-facing helpers small and the on-disk artifact formats
round-trippable in tests before they are imported into Zotero.
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import zipfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable

import numpy as np


@dataclass(frozen=True)
class ArtifactMetadata:
    schema_version: str
    source_sha256: str
    provider: str
    model: str
    dim: int
    config_sha256: str = ""
    created_at: str = ""

    def to_dict(self) -> dict[str, Any]:
        data = {
            "schema_version": self.schema_version,
            "source_sha256": self.source_sha256,
            "provider": self.provider,
            "model": self.model,
            "dim": self.dim,
            "config_sha256": self.config_sha256,
            "created_at": self.created_at or _dt.datetime.now(_dt.UTC).isoformat(),
        }
        return data


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_json(data: Any) -> str:
    encoded = json.dumps(data, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode("utf-8")
    return sha256_bytes(encoded)


def list_artifacts(rpc: Any, parent_id: int, suffix: str | None = None) -> list[dict[str, Any]]:
    artifacts = rpc.call("attachments.list", {"parentId": parent_id, "title": ""}) or []
    if isinstance(artifacts, list):
        try:
            setattr(rpc, "_zotron_last_artifacts", artifacts)
        except Exception:
            pass
    if suffix is None:
        return list(artifacts)
    return [
        a for a in artifacts
        if isinstance(a, dict)
        and str(a.get("title") or a.get("filename") or "").endswith(suffix)
    ]


def find_artifact_by_suffix(rpc: Any, parent_id: int, suffix: str) -> dict[str, Any] | None:
    cached = getattr(rpc, "_zotron_last_artifacts", None)
    if isinstance(cached, list):
        matches = [
            a for a in cached
            if isinstance(a, dict)
            and str(a.get("title") or a.get("filename") or "").endswith(suffix)
        ]
    else:
        matches = list_artifacts(rpc, parent_id=parent_id, suffix=suffix)
    return matches[0] if matches else None


def add_artifact_file(rpc: Any, parent_id: int, path: Path, title: str | None = None) -> dict[str, Any]:
    return rpc.call("attachments.add", {"parentId": parent_id, "path": str(path), "title": title or path.name})


def delete_artifact(rpc: Any, artifact_id: int) -> Any:
    return rpc.call("attachments.delete", {"id": artifact_id})


def _write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(row, ensure_ascii=False, separators=(",", ":")) + "\n")


def _read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def write_blocks_jsonl(path: Path, blocks: Iterable[dict[str, Any]]) -> None:
    _write_jsonl(path, blocks)


def read_blocks_jsonl(path: Path) -> list[dict[str, Any]]:
    return _read_jsonl(path)


def write_chunks_jsonl(path: Path, chunks: Iterable[dict[str, Any]]) -> None:
    _write_jsonl(path, chunks)


def read_chunks_jsonl(path: Path) -> list[dict[str, Any]]:
    return _read_jsonl(path)


class ZoteroArtifactStore:
    """Small RPC wrapper for Zotero child-attachment artifacts."""

    def __init__(self, rpc: Any) -> None:
        self.rpc = rpc

    def list_artifacts(self, parent_id: int, suffix: str | None = None) -> list[dict[str, Any]]:
        return list_artifacts(self.rpc, parent_id=parent_id, suffix=suffix)

    def find_artifact(self, parent_id: int, suffix: str) -> dict[str, Any] | None:
        return find_artifact_by_suffix(self.rpc, parent_id=parent_id, suffix=suffix)

    def add_artifact(self, parent_id: int, path: Path, title: str | None = None) -> dict[str, Any]:
        return add_artifact_file(self.rpc, parent_id=parent_id, path=path, title=title)

    def delete_artifact(self, artifact_id: int) -> Any:
        return delete_artifact(self.rpc, artifact_id=artifact_id)


def write_jsonl(path: Path, rows: Iterable[dict[str, Any]]) -> None:
    _write_jsonl(path, rows)


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return _read_jsonl(path)


def write_provider_raw_zip(
    path: Path,
    files: dict[str, Any] | None = None,
    *,
    provider: str | None = None,
    metadata: dict[str, Any] | None = None,
) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    files = files or {}
    provider_meta = {
        "provider": provider or "",
        "created_at": _dt.datetime.now(_dt.UTC).isoformat(),
        **(metadata or {}),
    }
    digest = hashlib.sha256()
    with zipfile.ZipFile(path, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        if provider is not None or metadata:
            payload = json.dumps(provider_meta, ensure_ascii=False, indent=2)
            digest.update(b"provider.json")
            digest.update(payload.encode("utf-8"))
            zf.writestr("provider.json", payload)
        for name, content in files.items():
            if isinstance(content, (dict, list)):
                payload = json.dumps(content, ensure_ascii=False, indent=2)
            elif isinstance(content, bytes):
                digest.update(name.encode("utf-8"))
                digest.update(content)
                zf.writestr(name, content)
                continue
            else:
                payload = str(content)
            digest.update(name.encode("utf-8"))
            digest.update(payload.encode("utf-8"))
            zf.writestr(name, payload)
    return digest.hexdigest()


def read_provider_raw_zip(path: Path) -> dict[str, Any]:
    files: dict[str, Any] = {}
    provider: dict[str, Any] = {}
    with zipfile.ZipFile(path) as zf:
        for name in zf.namelist():
            raw = zf.read(name)
            if name == "provider.json":
                provider = json.loads(raw.decode("utf-8"))
                continue
            try:
                files[name] = json.loads(raw.decode("utf-8"))
            except (UnicodeDecodeError, json.JSONDecodeError):
                try:
                    files[name] = raw.decode("utf-8")
                except UnicodeDecodeError:
                    files[name] = raw
    return {"provider": provider, "files": files}


def write_embedding_npz(path: Path, *, chunk_ids: list[str], vectors: np.ndarray, metadata: ArtifactMetadata | dict[str, Any]) -> None:
    meta = metadata.to_dict() if isinstance(metadata, ArtifactMetadata) else dict(metadata)
    path.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(
        path,
        schema_version=np.array(meta.get("schema_version", "1")),
        embedder_id=np.array(f"{meta.get('provider', '')}:{meta.get('model', '')}"),
        embedder_dim=np.array(int(meta.get("dim", vectors.shape[1] if vectors.ndim == 2 else 0))),
        source_chunks_sha256=np.array(meta.get("source_sha256", "")),
        created_at=np.array(meta.get("created_at") or _dt.datetime.now(_dt.UTC).isoformat()),
        chunk_ids=np.array(chunk_ids),
        vectors=vectors.astype(np.float32),
        metadata=np.array(json.dumps(meta, ensure_ascii=False)),
    )


def read_embedding_npz(path: Path) -> dict[str, Any]:
    with np.load(path, allow_pickle=False) as data:
        metadata = json.loads(str(data["metadata"])) if "metadata" in data else {}
        return {
            "chunk_ids": [str(x) for x in data["chunk_ids"].tolist()],
            "vectors": data["vectors"],
            "metadata": metadata,
        }


def is_artifact_stale(existing: dict[str, Any], expected: ArtifactMetadata | dict[str, Any]) -> bool:
    wanted = expected.to_dict() if isinstance(expected, ArtifactMetadata) else dict(expected)
    for key in ("schema_version", "source_sha256", "provider", "model", "dim", "config_sha256"):
        if str(existing.get(key, "")) != str(wanted.get(key, "")):
            return True
    return False


def find_stale_reasons(stored: dict[str, Any], current: dict[str, Any]) -> list[str]:
    reasons: list[str] = []
    for key, current_value in current.items():
        if stored.get(key) != current_value:
            reasons.append(f"{key} changed")
    return reasons
