"""Zotero-native artifact helpers for OCR/RAG intermediate files.

The helpers in this module keep RAG source-of-truth data in auditable files
that can be attached back to the Zotero item: provider raw zips, normalized
blocks/chunks JSONL, and embedding NPZ files.
"""

from __future__ import annotations

import hashlib
import json
import zipfile
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import asdict, dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

import numpy as np


OCR_RAW_SUFFIX = "zotron-ocr.raw.zip"
BLOCKS_SUFFIX = "zotron-blocks.jsonl"
CHUNKS_SUFFIX = "zotron-chunks.jsonl"
EMBEDDING_SUFFIX = "zotron-embed.npz"


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


@dataclass(frozen=True)
class ProviderRawArtifact:
    """Provider raw OCR payload plus sidecar files for zip storage."""

    item_key: str
    attachment_key: str
    provider: str
    payload: Any
    files: dict[str, str | bytes | Path | Mapping[str, Any]] = field(default_factory=dict)
    source_path: str | None = None
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())

    def manifest(self) -> dict[str, Any]:
        return {
            "item_key": self.item_key,
            "attachment_key": self.attachment_key,
            "provider": self.provider,
            "source_path": self.source_path,
            "created_at": self.created_at,
            "files": sorted(self.files),
        }


class ZoteroArtifactStore:
    """Small RPC wrapper for item-attached zotron artifacts."""

    def __init__(self, rpc: Any) -> None:
        self.rpc = rpc

    def list_artifacts(self, parent_id: str, suffix: str | None = None) -> list[dict[str, Any]]:
        attachments = self.rpc.call("attachments.list", {"parentKey": parent_id}) or []
        if suffix is None:
            return list(attachments)
        return [a for a in attachments if str(a.get("title") or "").endswith(suffix)]

    def find_artifact(self, parent_id: str, suffix: str) -> dict[str, Any] | None:
        artifacts = self.list_artifacts(parent_id, suffix=suffix)
        return artifacts[0] if artifacts else None

    def add_artifact(self, parent_id: str, path: str | Path, title: str | None = None) -> dict[str, Any]:
        path = Path(path)
        return self.rpc.call(
            "attachments.add",
            {"parentKey": parent_id, "path": str(path), "title": title or path.name},
        )

    def delete_artifact(self, attachment_id: str) -> Any:
        return self.rpc.call("attachments.delete", {"key": attachment_id})


def _json_bytes(value: Any, *, pretty: bool = False) -> bytes:
    kwargs: dict[str, Any] = {"ensure_ascii": False}
    if pretty:
        kwargs["indent"] = 2
    else:
        kwargs.update({"sort_keys": True, "separators": (",", ":")})
    return json.dumps(value, **kwargs).encode("utf-8")


def _metadata_dict(metadata: ArtifactMetadata | Mapping[str, Any]) -> dict[str, Any]:
    if isinstance(metadata, ArtifactMetadata):
        return metadata.to_dict()
    return dict(metadata)


def _zip_entry_bytes(value: Any, *, pretty_json: bool = False) -> bytes:
    if isinstance(value, bytes):
        return value
    if isinstance(value, str):
        candidate = Path(value)
        return candidate.read_bytes() if candidate.exists() else value.encode("utf-8")
    if isinstance(value, Path):
        return value.read_bytes()
    return _json_bytes(value, pretty=pretty_json)


def _safe_item_key(item_key: str) -> str:
    return item_key.replace("/", "_").replace("\\", "_")


def artifact_path(directory: str | Path, item_key: str, suffix: str) -> Path:
    """Return the canonical filesystem path for an item-scoped artifact."""
    return Path(directory).expanduser() / f"{_safe_item_key(item_key)}.{suffix}"


def _assert_safe_zip_member(name: str) -> None:
    if name.startswith("/") or ".." in Path(name).parts:
        raise ValueError(f"unsafe artifact member path: {name!r}")


def list_artifacts(rpc: Any, *, parent_id: str, suffix: str | None = None) -> list[dict[str, Any]]:
    artifacts = rpc.call("attachments.list", {"parentKey": parent_id}) or []
    artifacts = list(artifacts)
    setattr(rpc, "_zotron_last_artifacts", artifacts)
    if suffix is None:
        return artifacts
    return [artifact for artifact in artifacts if str(artifact.get("title") or "").endswith(suffix)]


def find_artifact_by_suffix(rpc: Any, *, parent_id: str, suffix: str) -> dict[str, Any] | None:
    cached = getattr(rpc, "_zotron_last_artifacts", None)
    artifacts = cached if cached is not None else list_artifacts(rpc, parent_id=parent_id)
    return next((artifact for artifact in artifacts if str(artifact.get("title") or "").endswith(suffix)), None)


def add_artifact_file(rpc: Any, *, parent_id: str, path: str | Path, title: str | None = None) -> dict[str, Any]:
    path = Path(path)
    return rpc.call("attachments.add", {"parentKey": parent_id, "path": str(path), "title": title or path.name})


def delete_artifact(rpc: Any, *, artifact_id: str) -> Any:
    return rpc.call("attachments.delete", {"key": artifact_id})



def write_provider_raw_zip(
    path: str | Path,
    entries: Mapping[str, Any] | ProviderRawArtifact | None = None,
    *,
    provider: str | None = None,
    files: Mapping[str, Any] | None = None,
) -> str | Path:
    """Write provider raw payloads to a zip.

    Canonical path mode writes exactly the requested zip path and returns its
    sha256 digest. Item-key mode accepts :class:`ProviderRawArtifact`, writes
    ``<item-key>.zotron-ocr.raw.zip`` under ``path``, and returns that path for
    compatibility with the previous OCR helper API.
    """
    if isinstance(entries, ProviderRawArtifact):
        artifact = entries
        target = artifact_path(path, artifact.item_key, OCR_RAW_SUFFIX)
        target.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as zf:
            zf.writestr("manifest.json", _json_bytes(artifact.manifest(), pretty=True))
            zf.writestr("provider_raw.json", _json_bytes(artifact.payload, pretty=True))
            for name, content in artifact.files.items():
                _assert_safe_zip_member(name)
                zf.writestr(name, _zip_entry_bytes(content, pretty_json=True))
        return target

    payloads = files if files is not None else entries
    if payloads is None:
        raise ValueError("write_provider_raw_zip requires entries or files")
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(target, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        if provider is not None:
            zf.writestr("provider.json", _json_bytes({"provider": provider}))
        for name, value in payloads.items():
            _assert_safe_zip_member(name)
            zf.writestr(name, _zip_entry_bytes(value))
    return hashlib.sha256(target.read_bytes()).hexdigest()


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
            elif name == "manifest.json":
                provider = {**provider, **dict(value)}
            else:
                files[name] = value
    return {"provider": provider, "files": files}


def _write_jsonl(path: Path, rows: Iterable[Mapping[str, Any]]) -> str:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for row in rows:
            fh.write(json.dumps(dict(row), ensure_ascii=False, sort_keys=True))
            fh.write("\n")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_jsonl(path: str | Path, rows: Iterable[Mapping[str, Any]]) -> str:
    """Write rows as UTF-8 JSONL and return the file sha256 digest."""
    return _write_jsonl(Path(path), rows)


def read_jsonl(path: str | Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in Path(path).read_text(encoding="utf-8").splitlines() if line.strip()]


def _write_item_jsonl(directory: str | Path, item_key: str, suffix: str, rows: Iterable[Mapping[str, Any]]) -> Path:
    path = artifact_path(directory, item_key, suffix)
    _write_jsonl(path, rows)
    return path


def write_blocks_jsonl(
    path_or_directory: str | Path,
    rows_or_item_key: Iterable[Mapping[str, Any]] | str,
    rows: Iterable[Mapping[str, Any]] | None = None,
) -> str | Path:
    if rows is None:
        return write_jsonl(path_or_directory, rows_or_item_key)  # type: ignore[arg-type]
    return _write_item_jsonl(path_or_directory, str(rows_or_item_key), BLOCKS_SUFFIX, rows)


def read_blocks_jsonl(path: str | Path) -> list[dict[str, Any]]:
    return read_jsonl(path)


def write_chunks_jsonl(
    path_or_directory: str | Path,
    rows_or_item_key: Iterable[Mapping[str, Any]] | str,
    rows: Iterable[Mapping[str, Any]] | None = None,
) -> str | Path:
    if rows is None:
        return write_jsonl(path_or_directory, rows_or_item_key)  # type: ignore[arg-type]
    return _write_item_jsonl(path_or_directory, str(rows_or_item_key), CHUNKS_SUFFIX, rows)


def read_chunks_jsonl(path: str | Path) -> list[dict[str, Any]]:
    return read_jsonl(path)


def text_sha256(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def metadata_for_chunks(chunks: Iterable[Mapping[str, Any]]) -> list[dict[str, Any]]:
    metadata: list[dict[str, Any]] = []
    for chunk in chunks:
        row = {k: v for k, v in dict(chunk).items() if k != "text"}
        row["text_sha256"] = text_sha256(str(chunk.get("text", "")))
        metadata.append(row)
    return metadata



def write_embedding_npz(
    path: str | Path,
    item_key: str | None = None,
    vectors: Any = None,
    metadata: Any = None,
    *,
    chunk_ids: list[str] | None = None,
    model: str | None = None,
) -> str | Path:
    """Write embedding vectors and metadata to an NPZ artifact.

    Path mode returns a sha256 digest and stores ``chunk_ids`` plus metadata.
    Item-key compatibility mode writes ``<item-key>.zotron-embed.npz`` and
    returns the path with legacy ``metadata_json``/``model`` fields.
    """
    if vectors is None or metadata is None:
        raise ValueError("write_embedding_npz requires vectors and metadata")

    if item_key is not None:
        if model is None:
            raise ValueError("item-key embedding artifacts require model")
        target = artifact_path(path, item_key, EMBEDDING_SUFFIX)
        target.parent.mkdir(parents=True, exist_ok=True)
        np.savez_compressed(
            target,
            vectors=np.asarray(vectors, dtype=np.float32),
            metadata_json=json.dumps(list(metadata), ensure_ascii=False),
            model=model,
        )
        return target

    if chunk_ids is None:
        raise ValueError("path embedding artifacts require chunk_ids")
    target = Path(path)
    target.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(
        target,
        vectors=np.asarray(vectors, dtype=np.float32),
        chunk_ids=np.asarray(chunk_ids, dtype=object),
        metadata=np.asarray(json.dumps(_metadata_dict(metadata), ensure_ascii=False, sort_keys=True), dtype=object),
    )
    return hashlib.sha256(target.read_bytes()).hexdigest()


def read_embedding_npz(path: str | Path) -> dict[str, Any] | tuple[np.ndarray, list[dict[str, Any]], str]:
    with np.load(Path(path), allow_pickle=True) as data:
        if "metadata_json" in data.files:
            return data["vectors"], json.loads(str(data["metadata_json"])), str(data["model"])
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


def is_metadata_stale(metadata: Sequence[Mapping[str, Any]], chunks: Sequence[Mapping[str, Any]]) -> bool:
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
