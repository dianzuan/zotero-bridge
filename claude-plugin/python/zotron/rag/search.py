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
        item_key = str(provenance.pop("item_key", item_id))
        chunk_id = str(provenance.pop("chunk_id", f"{item_key}:c{chunk_index}"))
        block_ids = provenance.pop("block_ids", [])
        self.chunks.append(
            {
                "item_id": item_id,
                "item_key": item_key,
                "title": title,
                "authors": authors,
                "section": section,
                "section_heading": provenance.pop("section_heading", section),
                "chunk_index": chunk_index,
                "chunk_id": chunk_id,
                "block_ids": list(block_ids) if isinstance(block_ids, list) else [],
                "text": text,
                "vector": vector,
                "attachment_id": attachment_id,
                **provenance,
            }
        )

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

        results = [
            {
                **{k: v for k, v in self.chunks[i].items() if k != "vector"},
                "score": float(scores[i]),
            }
            for i in top_indices
        ]
        if query is not None:
            for row in results:
                row["query"] = query
                row["zotero_uri"] = f"zotero://select/library/items/{row['item_key']}"
        return results

    def search_hits(self, query_vector: list[float], query: str, top_k: int = 10) -> list[dict]:
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


def format_retrieval_hit(row: dict, *, query: str = "") -> dict:
    """Format an internal search row as the academic-zh retrieval hit contract."""
    item_key = str(row.get("item_key") or row.get("item_id") or "")
    hit = {
        "item_key": item_key,
        "title": row.get("title", ""),
        "text": row.get("text", ""),
    }
    optional_map = {
        "authors": row.get("authors"),
        "year": row.get("year"),
        "venue": row.get("venue"),
        "doi": row.get("doi"),
        "section_heading": row.get("section_heading") or row.get("section"),
        "chunk_id": row.get("chunk_id"),
        "block_ids": row.get("block_ids"),
        "score": row.get("score"),
    }
    if query:
        optional_map["query"] = query
    if item_key:
        optional_map["zotero_uri"] = row.get("zotero_uri") or f"zotero://select/library/items/{item_key}"
    for key, value in optional_map.items():
        if value not in (None, "", []):
            hit[key] = value
    return hit


def write_hits_jsonl(path: Path, hits: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for hit in hits:
            fh.write(json.dumps(hit, ensure_ascii=False, separators=(",", ":")) + "\n")
