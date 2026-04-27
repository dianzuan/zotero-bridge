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
        authors: str,
        section: str,
        chunk_index: int,
        text: str,
        vector: list[float],
        attachment_id: int | None = None,
        **metadata,
    ) -> None:
        row = {
            "item_id": item_id,
            "title": title,
            "authors": authors,
            "section": section,
            "chunk_index": chunk_index,
            "text": text,
            "vector": vector,
            "attachment_id": attachment_id,
        }
        row.update(metadata)
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

        results = [
            {
                "item_id": self.chunks[i]["item_id"],
                "item_key": self.chunks[i].get("item_key") or self.chunks[i]["item_id"],
                "title": self.chunks[i]["title"],
                "authors": self.chunks[i]["authors"],
                "year": self.chunks[i].get("year"),
                "venue": self.chunks[i].get("venue"),
                "doi": self.chunks[i].get("doi", ""),
                "section": self.chunks[i]["section"],
                "section_heading": self.chunks[i].get("section_heading") or self.chunks[i]["section"],
                "chunk_index": self.chunks[i]["chunk_index"],
                "chunk_id": self.chunks[i].get("chunk_id"),
                "block_ids": self.chunks[i].get("block_ids", []),
                "text": self.chunks[i]["text"],
                "score": float(scores[i]),
                "attachment_id": self.chunks[i].get("attachment_id"),
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


def _authors_list(value) -> list[str]:
    if value is None:
        return []
    if isinstance(value, list):
        return [str(v) for v in value if str(v)]
    if isinstance(value, str):
        return [part.strip() for part in value.split(";") if part.strip()]
    return [str(value)]


def results_to_hits(results: list[dict], query: str) -> list[dict]:
    hits: list[dict] = []
    for row in results:
        item_key = row.get("item_key") or row.get("item_id") or ""
        hit = {
            "item_key": item_key,
            "title": row.get("title", ""),
            "text": row.get("text", ""),
            "authors": _authors_list(row.get("authors")),
            "year": row.get("year"),
            "venue": row.get("venue", ""),
            "doi": row.get("doi", ""),
            "zotero_uri": f"zotero://select/library/items/{item_key}",
            "section_heading": row.get("section_heading") or row.get("section", ""),
            "chunk_id": row.get("chunk_id") or (
                f"{row.get('attachment_id')}:c{row.get('chunk_index')}"
                if row.get("attachment_id") is not None
                else str(row.get("chunk_index", ""))
            ),
            "block_ids": row.get("block_ids", []),
            "query": query,
            "score": row.get("score", 0.0),
        }
        hits.append(hit)
    return hits


def format_retrieval_hit(row: dict, *, query: str = "") -> dict:
    return results_to_hits([row], query=query)[0]


def write_hits_jsonl(path: Path, hits: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as fh:
        for hit in hits:
            fh.write(json.dumps(hit, ensure_ascii=False, separators=(",", ":")) + "\n")
