"""RAG CLI for Zotron — index, search, status subcommands."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from zotron.collections import find_by_name as _find_collection_by_name
from zotron.config import load_config
from zotron.rpc import ZoteroRPC
from zotron.rag.chunker import chunk_text
from zotron.rag.embedder import create_embedder
from zotron.rag.search import VectorStore, results_to_hits


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _store_path(collection_name: str) -> Path:
    return Path("~/.local/share/zotron/rag").expanduser() / f"{collection_name}.json"


def _find_collection_id(rpc: ZoteroRPC, name: str) -> int | None:
    """Search collections.tree recursively for a collection matching *name*."""
    return _find_collection_by_name(rpc, name)


def _get_item_text(rpc: ZoteroRPC, item_id: str) -> str | None:
    """Return OCR note (tag 'ocr') text first, then fulltext as fallback."""
    import re

    # Try OCR notes via notes.get
    try:
        notes = rpc.call("notes.get", {"parentId": int(item_id)}) or []
        for note in notes:
            tags = note.get("tags") or []
            # tags may be strings or dicts with "tag" key
            tag_values = [
                (t.get("tag") if isinstance(t, dict) else t) for t in tags
            ]
            if "ocr" in tag_values:
                note_html = note.get("content") or note.get("note") or ""
                text = re.sub(r"<[^>]+>", "", note_html)
                if text.strip():
                    return text
    except Exception:
        pass

    # Fallback: fulltext
    try:
        result = rpc.call("attachments.getFulltext", {"id": int(item_id)})
        if isinstance(result, str) and result.strip():
            return result
        if isinstance(result, dict):
            content = result.get("content", "")
            if content and content.strip():
                return content
    except Exception:
        pass

    return None


def _build_embedder(cfg: dict[str, Any]):
    embed_cfg = cfg.get("embedding", {})
    provider = embed_cfg.get("provider", "ollama")
    if provider == "ollama":
        model = embed_cfg.get("model", "nomic-embed-text")
        api_url = embed_cfg.get("ollama_base_url", "http://localhost:11434")
        return create_embedder(provider="ollama", model=model, api_url=api_url)
    else:
        model = embed_cfg.get("model") or embed_cfg.get("openai_model", "text-embedding-3-small")
        api_key = embed_cfg.get("api_key") or embed_cfg.get("openai_api_key", "")
        api_url = embed_cfg.get("api_url")
        return create_embedder(provider=provider, model=model, api_key=api_key, api_url=api_url)


# ---------------------------------------------------------------------------
# Subcommands
# ---------------------------------------------------------------------------

def cmd_index(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    rpc_url = cfg.get("zotero", {}).get("rpc_url", "http://localhost:23119/zotron/rpc")
    rpc = ZoteroRPC(rpc_url)

    collection_id = _find_collection_id(rpc, args.collection)
    if collection_id is None:
        print(json.dumps({"error": f"Collection not found: {args.collection!r}"}), file=sys.stderr)
        sys.exit(1)

    rag_cfg = cfg.get("rag", {})
    chunk_size = rag_cfg.get("chunk_size") or 512
    chunk_overlap = rag_cfg.get("chunk_overlap") or 64
    embed_cfg = cfg.get("embedding", {})
    model_name = (
        embed_cfg.get("model")
        if embed_cfg.get("provider") == "ollama"
        else embed_cfg.get("openai_model", "text-embedding-3-small")
    )

    store_path = _store_path(args.collection)

    if store_path.exists() and not args.rebuild:
        store = VectorStore.load(store_path)
    else:
        store = VectorStore(
            collection=args.collection,
            collection_id=collection_id,
            model=model_name,
        )

    embedder = _build_embedder(cfg)

    raw = rpc.call("collections.getItems", {"id": collection_id, "limit": 500}) or {}
    items = raw.get("items", []) if isinstance(raw, dict) else raw
    indexed = 0
    skipped = 0

    for item in items:
        item_id = str(item.get("id") or item.get("itemID", ""))
        if not item_id:
            continue

        if args.rebuild:
            store.clear_item(item_id)

        text = _get_item_text(rpc, item_id)
        if not text:
            skipped += 1
            continue

        title = item.get("title", "")
        creators = item.get("creators") or []
        if creators:
            authors = "; ".join(
                c.get("lastName", c.get("name", "")) for c in creators
            )
        else:
            authors = ""

        chunks = chunk_text(text, chunk_size=chunk_size, overlap=chunk_overlap)
        texts = [c["text"] for c in chunks]
        vectors = embedder.embed_batch(texts) if texts else []

        for chunk, vector in zip(chunks, vectors):
            store.add_chunk(
                item_id=item_id,
                title=title,
                authors=authors,
                section=chunk["section"],
                chunk_index=chunk["chunk_index"],
                text=chunk["text"],
                vector=vector,
            )
        indexed += 1

    store_path.parent.mkdir(parents=True, exist_ok=True)
    store.save(store_path)

    print(json.dumps({
        "status": "ok",
        "collection": args.collection,
        "indexed": indexed,
        "skipped": skipped,
        "total_chunks": len(store.chunks),
        "store_path": str(store_path),
    }))


def cmd_search(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    store_path = _store_path(args.collection)
    if not store_path.exists():
        print(json.dumps({"error": f"Collection not indexed: {args.collection!r}"}), file=sys.stderr)
        sys.exit(1)

    store = VectorStore.load(store_path)
    embedder = _build_embedder(cfg)
    top_k = cfg.get("rag", {}).get("top_k", 5)

    query_vec = embedder.embed(args.query)
    results = store.search(query_vec, top_k=top_k)

    print(json.dumps({"results": results}, ensure_ascii=False, indent=2))


def cmd_hits(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    store_path = _store_path(args.collection)
    if not store_path.exists():
        print(json.dumps({"error": f"Collection not indexed: {args.collection!r}"}), file=sys.stderr)
        sys.exit(1)

    store = VectorStore.load(store_path)
    embedder = _build_embedder(cfg)
    query_vec = embedder.embed(args.query)
    results = store.search(query_vec, top_k=args.limit)
    hits = results_to_hits(results, query=args.query)

    if args.output == "jsonl":
        for hit in hits:
            print(json.dumps(hit, ensure_ascii=False, separators=(",", ":")))
    else:
        print(json.dumps({"hits": hits, "total": len(hits)}, ensure_ascii=False, indent=2))


def cmd_status(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    store_path = _store_path(args.collection)
    if not store_path.exists():
        print(json.dumps({"status": "not indexed", "collection": args.collection}))
        return

    store = VectorStore.load(store_path)

    item_ids = {c["item_id"] for c in store.chunks}
    print(json.dumps({
        "status": "indexed",
        "collection": store.collection,
        "collection_id": store.collection_id,
        "model": store.model,
        "total_chunks": len(store.chunks),
        "total_items": len(item_ids),
        "store_path": str(store_path),
    }))


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> None:
    parser = argparse.ArgumentParser(
        prog="zotron-rag",
        description="RAG index and search for Zotero collections",
    )
    sub = parser.add_subparsers(dest="command", required=True)

    # index
    p_index = sub.add_parser(
        "index",
        help="Index a Zotero collection",
        epilog=(
            "Examples:\n"
            "  zotron-rag index --collection \"2026-AI\"\n"
            "  zotron-rag index --collection \"2026-AI\" --rebuild\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p_index.add_argument("--collection", required=True, help="Collection name")
    p_index.add_argument("--rebuild", action="store_true", help="Re-embed all items")

    # search
    p_search = sub.add_parser(
        "search",
        help="Search an indexed collection",
        epilog=(
            "Examples:\n"
            "  zotron-rag search --collection \"2026-AI\" \"transformer architecture\"\n"
            "  zotron-rag search --collection \"climate\" \"sea level rise\"\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p_search.add_argument("--collection", required=True, help="Collection name")
    p_search.add_argument("query", help="Query text")

    # hits
    p_hits = sub.add_parser(
        "hits",
        help="Emit academic-zh retrieval hits with span provenance",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p_hits.add_argument("query", help="Query text")
    p_hits.add_argument("--collection", required=True, help="Collection name")
    p_hits.add_argument("--limit", type=int, default=50, help="Maximum hits to return")
    p_hits.add_argument(
        "--output",
        choices=["json", "jsonl"],
        default="json",
        help="Output format (default: json)",
    )

    # status
    p_status = sub.add_parser("status", help="Show index status for a collection")
    p_status.add_argument("--collection", required=True, help="Collection name")

    # cite
    p_cite = sub.add_parser(
        "cite",
        help="Retrieve top-K chunks with full citation provenance.",
        epilog=(
            "Examples:\n"
            "  zotron-rag cite \"transformer\" --top-k 5\n"
            "  zotron-rag cite \"climate change\" --output markdown\n"
        ),
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    p_cite.add_argument("query", help="Search query text.")
    p_cite.add_argument("--collection", required=True, help="Collection name to search in.")
    p_cite.add_argument("--top-k", type=int, default=10, help="Number of citations to return (default 10).")
    p_cite.add_argument(
        "--output",
        choices=["json", "markdown"],
        default="json",
        help="Output format (default: json).",
    )

    args = parser.parse_args()
    cfg = load_config()

    if args.command == "cite":
        embedder = _build_embedder(cfg)
        store_path = _store_path(args.collection)
        if not store_path.exists():
            print(
                f"Error: no index for collection '{args.collection}'. "
                f"Run `zotron-rag index --collection {args.collection}` first.",
                file=sys.stderr,
            )
            sys.exit(2)

        from zotron.rag.citation import (
            retrieve_with_citations,
            format_citation_markdown,
            format_citation_json,
        )

        citations = retrieve_with_citations(
            query=args.query,
            store_path=store_path,
            embedder=embedder,
            top_k=args.top_k,
        )

        if args.output == "json":
            arr = [json.loads(format_citation_json(c)) for c in citations]
            print(json.dumps(arr, ensure_ascii=False, indent=2))
        else:  # markdown
            for c in citations:
                print(format_citation_markdown(c))
        return

    dispatch = {"index": cmd_index, "search": cmd_search, "hits": cmd_hits, "status": cmd_status}
    dispatch[args.command](args, cfg)


if __name__ == "__main__":
    main()
