from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Iterable, Mapping

import numpy as np

from zotron.artifacts import is_metadata_stale, read_chunks_jsonl, read_embedding_npz


class VectorStore:
    def __init__(self, collection: str, collection_id: int, model: str):
        self.collection = collection
        self.collection_id = collection_id
        self.model = model
        self.chunks: list[dict] = []

    def add_chunk(
        self,
        item_id: str,
        title: str,
        authors: str | list[str],
        section: str,
        chunk_index: int,
        text: str,
        vector: list[float],
        attachment_id: int | None = None,
        **provenance: object,
    ) -> None:
        row = {
            "item_id": item_id,
            "item_key": provenance.pop("item_key", item_id),
            "title": title,
            "authors": authors,
            "section": section,
            "chunk_index": chunk_index,
            "text": text,
            "vector": vector,
            "attachment_id": attachment_id,
        }
        row.update(provenance)
        self.chunks.append(row)

    def clear_item(self, item_id: str) -> None:
        self.chunks = [c for c in self.chunks if c["item_id"] != item_id]

    def search(self, query_vector: list[float], top_k: int = 10, query: str | None = None) -> list[dict]:
        if not self.chunks:
            return []

        q = np.array(query_vector, dtype=np.float32)
        q = q / np.linalg.norm(q)

        vectors = np.array([c["vector"] for c in self.chunks], dtype=np.float32)
        norms = np.linalg.norm(vectors, axis=1, keepdims=True)
        norms[norms == 0] = 1.0
        vectors = vectors / norms
        scores = vectors @ q

        top_indices = np.argsort(scores)[::-1][:top_k]

        results: list[dict] = []
        for i in top_indices:
            row = dict(self.chunks[i])
            row.pop("vector", None)
            row["score"] = float(scores[i])
            row["section_heading"] = row.get("section_heading") or row.get("section")
            if query is not None:
                row["query"] = query
            results.append(row)
        return results

    def search_hits(self, query_vector: list[float], *, query: str, top_k: int = 10) -> list[dict]:
        return results_to_hits(self.search(query_vector, top_k=top_k), query=query)

    def save(self, path: Path) -> None:
        data = {
            "collection": self.collection,
            "collection_id": self.collection_id,
            "model": self.model,
            "chunks": self.chunks,
        }
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")

    @classmethod
    def load(cls, path: Path) -> "VectorStore":
        data = json.loads(path.read_text(encoding="utf-8"))
        store = cls(
            collection=data["collection"],
            collection_id=data["collection_id"],
            model=data["model"],
        )
        store.chunks = data["chunks"]
        return store



class ArtifactBackedVectorStore(VectorStore):
    """VectorStore populated directly from Zotero-native chunk/embedding artifacts.

    OCR/RAG artifacts are item-scoped files attached to Zotero items:
    ``<item-key>.zotron-chunks.jsonl`` plus ``<item-key>.zotron-embed.npz``.
    This adapter keeps the legacy in-memory ``VectorStore`` search behavior but
    makes artifacts the source of truth and validates per-chunk embedding
    metadata when the NPZ includes it.
    """

    @classmethod
    def from_item_artifacts(
        cls,
        *,
        collection: str,
        collection_id: int,
        item_key: str,
        chunks_path: str | Path,
        embeddings_path: str | Path,
        item_metadata: Mapping[str, Any] | None = None,
    ) -> "ArtifactBackedVectorStore":
        store = cls(collection=collection, collection_id=collection_id, model="")
        store.add_item_artifacts(
            item_key=item_key,
            chunks_path=chunks_path,
            embeddings_path=embeddings_path,
            item_metadata=item_metadata,
        )
        return store

    @classmethod
    def from_artifacts(
        cls,
        *,
        collection: str,
        collection_id: int,
        items: Iterable[Mapping[str, Any]],
    ) -> "ArtifactBackedVectorStore":
        """Build one searchable store from multiple item artifact descriptors.

        Each item descriptor must include ``item_key``, ``chunks_path``, and
        ``embeddings_path``; additional keys become item-level provenance.
        """
        store = cls(collection=collection, collection_id=collection_id, model="")
        for item in items:
            item_key = str(item.get("item_key") or item.get("key") or "")
            if not item_key:
                raise ValueError("artifact item descriptor requires item_key")
            chunks_path = item.get("chunks_path")
            embeddings_path = item.get("embeddings_path")
            if chunks_path is None or embeddings_path is None:
                raise ValueError(f"artifact item {item_key!r} requires chunks_path and embeddings_path")
            metadata = {k: v for k, v in dict(item).items() if k not in {"chunks_path", "embeddings_path"}}
            store.add_item_artifacts(
                item_key=item_key,
                chunks_path=chunks_path,
                embeddings_path=embeddings_path,
                item_metadata=metadata,
            )
        return store

    def add_item_artifacts(
        self,
        *,
        item_key: str,
        chunks_path: str | Path,
        embeddings_path: str | Path,
        item_metadata: Mapping[str, Any] | None = None,
    ) -> None:
        metadata = dict(item_metadata or {})
        chunks = read_chunks_jsonl(chunks_path)
        vectors, embedding_metadata, model = self._read_embeddings(embeddings_path)

        if len(vectors) != len(chunks):
            raise ValueError(
                f"embedding vector count ({len(vectors)}) does not match chunk count ({len(chunks)}) for {item_key}"
            )
        if embedding_metadata is not None and is_metadata_stale(embedding_metadata, chunks):
            raise ValueError(f"stale embedding metadata for {item_key}")

        if self.model and model and self.model != model:
            raise ValueError(f"mixed embedding models are not supported: {self.model!r} != {model!r}")
        if model:
            self.model = model

        item_id = str(metadata.get("item_id") or metadata.get("id") or item_key)
        title = str(metadata.get("title") or "")
        authors = metadata.get("authors") or metadata.get("creators") or []
        item_provenance = {
            key: metadata[key]
            for key in (
                "attachment_id",
                "attachment_key",
                "doi",
                "venue",
                "year",
                "zotero_uri",
            )
            if key in metadata
        }

        for idx, (chunk, vector) in enumerate(zip(chunks, vectors, strict=True)):
            row = dict(chunk)
            text = str(row.pop("text", ""))
            section_heading = row.get("section_heading") or row.get("section") or ""
            provenance = {**item_provenance, **row}
            attachment_id = metadata.get("attachment_id") or row.get("attachment_id")
            provenance.pop("attachment_id", None)
            self.add_chunk(
                item_id=item_id,
                item_key=item_key,
                title=str(row.get("title") or title),
                authors=row.get("authors") or authors,
                section=str(section_heading),
                chunk_index=int(row.get("chunk_index", idx)),
                text=text,
                vector=np.asarray(vector, dtype=np.float32).tolist(),
                attachment_id=attachment_id,
                **provenance,
            )

    @staticmethod
    def _read_embeddings(embeddings_path: str | Path) -> tuple[np.ndarray, list[dict[str, Any]] | None, str]:
        loaded = read_embedding_npz(embeddings_path)
        if isinstance(loaded, tuple):
            vectors, metadata, model = loaded
            return np.asarray(vectors, dtype=np.float32), list(metadata), model

        vectors = np.asarray(loaded["vectors"], dtype=np.float32)
        chunk_ids = loaded.get("chunk_ids") or []
        embedding_metadata: list[dict[str, Any]] | None
        if chunk_ids:
            embedding_metadata = [{"chunk_id": chunk_id} for chunk_id in chunk_ids]
        else:
            embedding_metadata = None
        model = str(loaded.get("metadata", {}).get("model") or loaded.get("metadata", {}).get("embedder_model") or "")
        return vectors, embedding_metadata, model


def _authors_list(authors: object) -> list[str]:
    if authors is None:
        return []
    if isinstance(authors, list):
        return [str(a) for a in authors if str(a)]
    return [part.strip() for part in str(authors).split(";") if part.strip()]


def results_to_hits(rows: list[dict], *, query: str) -> list[dict]:
    """Convert internal search rows to the academic-zh retrieval hit contract."""
    hits: list[dict] = []
    for row in rows:
        item_key = str(row.get("item_key") or row.get("item_id") or "")
        hit = {
            "item_key": item_key,
            "title": row.get("title") or "",
            "text": row.get("text") or "",
        }
        optional = {
            "authors": _authors_list(row.get("authors")),
            "year": row.get("year"),
            "venue": row.get("venue"),
            "doi": row.get("doi"),
            "zotero_uri": row.get("zotero_uri") or (f"zotero://select/library/items/{item_key}" if item_key else ""),
            "section_heading": row.get("section_heading") or row.get("section"),
            "chunk_id": row.get("chunk_id") or (f"{item_key}:c{row.get('chunk_index')}" if item_key and row.get("chunk_index") is not None else None),
            "block_ids": row.get("block_ids"),
            "query": query,
            "score": row.get("score"),
        }
        for key, value in optional.items():
            if value is None or value == []:
                continue
            if value == "" and key not in {"doi", "venue"}:
                continue
            hit[key] = value
        hits.append(hit)
    return hits
