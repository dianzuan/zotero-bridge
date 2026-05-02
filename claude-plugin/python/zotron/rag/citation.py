"""Citation dataclass + formatters for RAG results.

A Citation captures the provenance an AI agent needs to quote text from a
user's Zotero library without hallucination — the Zotero item key (stable
identifier), the attachment id, section heading, chunk index, the verbatim
text, and a similarity score. Each Citation round-trips to JSON and renders
to markdown for prompt injection.
"""
from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class Citation:
    item_key: str
    attachment_id: str | None
    title: str
    authors: str
    section: str
    chunk_index: int
    text: str
    score: float

    def zotero_uri(self) -> str:
        """Return a zotero:// URI that opens the item in Zotero desktop."""
        return f"zotero://select/library/items/{self.item_key}"


def format_citation_markdown(c: Citation) -> str:
    """Render Citation as a markdown block suitable for AI prompt injection."""
    return (
        f"**{c.title}** — {c.authors} "
        f"[zotero:{c.item_key}] (section: {c.section}, score: {c.score:.2f})\n\n"
        f"> {c.text}\n"
    )


def format_citation_json(c: Citation) -> str:
    """Render Citation as a JSON object with stable field names."""
    return json.dumps(
        {
            "itemKey": c.item_key,
            "attachmentId": c.attachment_id,
            "title": c.title,
            "authors": c.authors,
            "section": c.section,
            "chunkIndex": c.chunk_index,
            "text": c.text,
            "score": c.score,
            "zoteroUri": c.zotero_uri(),
        },
        ensure_ascii=False,
        indent=2,
    )


def retrieve_with_citations(
    query: str,
    *,
    store_path: Path,
    embedder: Any,
    top_k: int = 10,
) -> list[Citation]:
    """Embed *query*, search the VectorStore at *store_path*, return Citation list.

    Parameters
    ----------
    query : str
        Natural-language search query.
    store_path : Path
        Path to a VectorStore JSON file (saved via VectorStore.save()).
    embedder : Embedder
        Object with `.embed(text) -> list[float]` (e.g. OllamaEmbedder).
    top_k : int
        Maximum number of citations to return.
    """
    from zotron.rag.search import VectorStore  # local import avoids cycles

    store = VectorStore.load(store_path)
    qvec = embedder.embed(query)
    rows = store.search(qvec, top_k=top_k)
    return [
        Citation(
            item_key=r["item_id"],
            attachment_id=r.get("attachment_id"),
            title=r["title"],
            authors=r["authors"],
            section=r["section"],
            chunk_index=r["chunk_index"],
            text=r["text"],
            score=r["score"],
        )
        for r in rows
    ]
