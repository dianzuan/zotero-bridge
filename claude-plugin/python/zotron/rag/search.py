from __future__ import annotations

import json
from pathlib import Path

import numpy as np


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
