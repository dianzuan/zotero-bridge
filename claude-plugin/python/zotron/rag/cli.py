"""RAG CLI for Zotron — index, search, status subcommands."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast

from zotron.collections import find_by_name as _find_collection_by_name
from zotron.config import load_config
from zotron.rpc import ZoteroRPC
from zotron._paginate import paginate
from zotron.artifacts import (
    CHUNKS_SUFFIX,
    EMBEDDING_SUFFIX,
    artifact_path,
    metadata_for_chunks,
    read_chunks_jsonl,
    read_embedding_npz,
    write_embedding_npz,
)
from zotron.rag.chunker import chunk_text
from zotron.rag.embedder import create_embedder
from zotron.rag.search import VectorStore, results_to_hits
from zotron.paths import linux_path, zotero_path


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _store_path(collection_name: str) -> Path:
    return Path("~/.local/share/zotron/rag").expanduser() / f"{collection_name}.json"


def _find_collection_id(rpc: ZoteroRPC, name: str) -> str | None:
    """Search collections.tree recursively for a collection matching *name*."""
    return _find_collection_by_name(rpc, name)


def _get_item_text(rpc: ZoteroRPC, item_id: str) -> str | None:
    """Return OCR note (tag 'ocr') text first, then fulltext as fallback."""
    import re

    # Try OCR notes via notes.get
    try:
        notes = cast(
            list[dict[str, Any]],
            rpc.call("notes.get", {"parentId": item_id}) or [],
        )
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
        result = rpc.call("attachments.getFulltext", {"id": item_id})
        if isinstance(result, str) and result.strip():
            return result
        if isinstance(result, dict):
            content = result.get("content", "")
            if content and content.strip():
                return content
    except Exception:
        pass

    return None




def _artifact_item_keys(artifacts_dir: Path, item_key: str | None = None) -> list[str]:
    if item_key:
        return [item_key]
    suffix = f".{CHUNKS_SUFFIX}"
    return sorted(
        path.name[: -len(suffix)]
        for path in artifacts_dir.glob(f"*{suffix}")
        if path.name.endswith(suffix)
    )


def _embedding_model_from_cfg(cfg: dict[str, Any]) -> str:
    embed_cfg = cfg.get("embedding", {})
    return str(
        embed_cfg.get("model")
        or embed_cfg.get("openai_model")
        or "unknown"
    )


def _read_item_embedding(path: Path) -> tuple[list[list[float]], list[dict[str, Any]], str]:
    loaded = read_embedding_npz(path)
    if isinstance(loaded, tuple):
        vectors, metadata, model = loaded
        return vectors.astype(float).tolist(), list(metadata), model

    vectors = loaded["vectors"].astype(float).tolist()
    raw_metadata = loaded.get("metadata") or {}
    chunk_ids = loaded.get("chunk_ids") or []
    if isinstance(raw_metadata, list):
        metadata = [dict(row) for row in raw_metadata]
    else:
        metadata = [{"chunk_id": chunk_id} for chunk_id in chunk_ids]
    model = str(raw_metadata.get("model", "unknown")) if isinstance(raw_metadata, dict) else "unknown"
    return vectors, metadata, model


def _artifact_vector_store(artifacts_dir: str | Path, item_key: str | None = None) -> VectorStore:
    directory = Path(artifacts_dir).expanduser()
    store = VectorStore(collection="artifacts", collection_id="0", model="artifact")

    for key in _artifact_item_keys(directory, item_key):
        chunks_path = artifact_path(directory, key, CHUNKS_SUFFIX)
        embed_path = artifact_path(directory, key, EMBEDDING_SUFFIX)
        if not chunks_path.exists():
            raise FileNotFoundError(f"Missing chunks artifact: {chunks_path}")
        if not embed_path.exists():
            raise FileNotFoundError(f"Missing embedding artifact: {embed_path}")

        chunks = read_chunks_jsonl(chunks_path)
        vectors, metadata, model = _read_item_embedding(embed_path)
        if len(chunks) != len(vectors):
            raise ValueError(f"Artifact length mismatch for {key}: {len(chunks)} chunks, {len(vectors)} vectors")
        store.model = model

        for index, (chunk, vector) in enumerate(zip(chunks, vectors)):
            meta = dict(metadata[index]) if index < len(metadata) else {}
            row = {**meta, **dict(chunk)}
            row.setdefault("item_key", key)
            row.setdefault("item_id", row["item_key"])
            row.setdefault("title", "")
            row.setdefault("authors", [])
            row.setdefault("section", row.get("section_heading") or "")
            row.setdefault("section_heading", row.get("section") or "")
            row.setdefault("chunk_index", index)
            row.setdefault("chunk_id", f"{row['item_key']}:c{index}")
            row["vector"] = vector
            store.chunks.append(row)
    return store

def _build_embedder(cfg: dict[str, Any]):
    embed_cfg = cfg.get("embedding", {})
    provider = embed_cfg.get("provider", "doubao")
    if provider == "ollama":
        model = embed_cfg.get("model", "nomic-embed-text")
        api_url = embed_cfg.get("ollama_base_url", "http://localhost:11434")
        return create_embedder(provider="ollama", model=model, api_url=api_url)
    else:
        model = embed_cfg.get("model") or embed_cfg.get("openai_model", "doubao-embedding-vision-251215")
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
        else embed_cfg.get("model") or embed_cfg.get("openai_model", "doubao-embedding-vision-251215")
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

    items = paginate(rpc, "collections.getItems", {"id": collection_id}, page_size=500)
    indexed = 0
    skipped = 0

    for item in items:
        item_id = str(item.get("key", ""))
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


def _load_index_or_exit(collection: str) -> VectorStore:
    store_path = _store_path(collection)
    if not store_path.exists():
        print(json.dumps({"error": f"Collection not indexed: {collection!r}"}), file=sys.stderr)
        sys.exit(1)
    return VectorStore.load(store_path)


def cmd_search(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    if getattr(args, "artifacts_dir", None):
        results = _run_artifact_search(args, cfg)
        print(json.dumps({"results": results}, ensure_ascii=False, indent=2))
        return
    if not args.collection:
        print(json.dumps({"error": "--collection is required unless --artifacts-dir is used"}), file=sys.stderr)
        sys.exit(2)
    store = _load_index_or_exit(args.collection)
    embedder = _build_embedder(cfg)
    top_k = cfg.get("rag", {}).get("top_k", 5)

    query_vec = embedder.embed(args.query)
    results = store.search(query_vec, top_k=top_k)

    print(json.dumps({"results": results}, ensure_ascii=False, indent=2))


def cmd_hits(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    _run_hits(
        args.query,
        args.collection,
        args.output,
        args.top_k,
        cfg,
        artifacts_dir=getattr(args, "artifacts_dir", None),
        item_key=getattr(args, "item_key", None),
        zotero=getattr(args, "zotero", False),
        top_spans_per_item=getattr(args, "top_spans_per_item", None),
        include_fulltext_spans=getattr(args, "include_fulltext_spans", False),
    )


def cmd_index_artifacts(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    if getattr(args, "zotero", False):
        _run_zotero_index_artifacts(args, cfg)
        return

    if not args.artifacts_dir:
        print(json.dumps({"error": "--artifacts-dir is required unless --zotero is used"}), file=sys.stderr)
        sys.exit(2)

    artifacts_dir = Path(args.artifacts_dir).expanduser()
    item_keys = _artifact_item_keys(artifacts_dir, args.item_key)
    if not item_keys:
        print(json.dumps({"error": f"No chunk artifacts found in {str(artifacts_dir)!r}"}), file=sys.stderr)
        sys.exit(2)

    embedder = _build_embedder(cfg)
    model = args.model or _embedding_model_from_cfg(cfg)
    indexed = 0
    total_chunks = 0
    written_paths: list[str] = []

    for key in item_keys:
        chunks_path = artifact_path(artifacts_dir, key, CHUNKS_SUFFIX)
        chunks = read_chunks_jsonl(chunks_path)
        texts = [str(chunk.get("text", "")) for chunk in chunks]
        vectors = embedder.embed_batch(texts) if texts else []
        embedding_path = write_embedding_npz(
            artifacts_dir,
            key,
            vectors=vectors,
            metadata=metadata_for_chunks(chunks),
            model=model,
        )
        indexed += 1
        total_chunks += len(chunks)
        written_paths.append(str(embedding_path))

    print(json.dumps({
        "status": "ok",
        "indexed": indexed,
        "total_chunks": total_chunks,
        "embedding_path": written_paths[0] if len(written_paths) == 1 else None,
        "embedding_paths": written_paths,
    }, ensure_ascii=False))


def _item_key_from_info(item_id: str, item_info: dict[str, Any], chunks: list[dict[str, Any]]) -> str:
    if chunks:
        first_key = chunks[0].get("item_key")
        if first_key:
            return str(first_key)
    return str(item_info.get("key") or item_info.get("itemKey") or item_id)


def _attachment_path(rpc: ZoteroRPC, attachment: dict[str, Any]) -> Path:
    path = attachment.get("path")
    if not path:
        result = rpc.call("attachments.getPath", {"id": attachment.get("key")})
        path = result.get("path") if isinstance(result, dict) else result
    if not path:
        raise FileNotFoundError(f"Attachment path unavailable for {attachment.get('title')!r}")
    return Path(linux_path(str(path))).expanduser()


def _find_chunks_attachment(rpc: ZoteroRPC, item_id: str) -> dict[str, Any] | None:
    attachments = cast(list[dict[str, Any]], rpc.call("attachments.list", {"parentId": item_id}) or [])
    return _find_chunks_attachment_in(attachments)


def _find_chunks_attachment_in(attachments: list[dict[str, Any]]) -> dict[str, Any] | None:
    suffix = f".{CHUNKS_SUFFIX}"
    return next(
        (attachment for attachment in attachments if str(attachment.get("title") or "").endswith(suffix)),
        None,
    )


def _zotero_item_ids_for_index(rpc: ZoteroRPC, args: argparse.Namespace) -> list[Any]:
    if args.item is not None:
        return [args.item]
    if args.collection:
        collection_id = _find_collection_id(rpc, args.collection)
        if collection_id is None:
            print(json.dumps({"error": f"Collection not found: {args.collection!r}"}), file=sys.stderr)
            sys.exit(1)
        items = paginate(rpc, "collections.getItems", {"id": collection_id}, page_size=500)
        return [item["key"] for item in items if item.get("key") is not None]
    print(json.dumps({"error": "--item or --collection is required when --zotero is used"}), file=sys.stderr)
    sys.exit(2)


def _index_zotero_item_artifact(
    *,
    rpc: ZoteroRPC,
    item_id: str,
    embedder: Any,
    model: str,
    output_dir: Path | None,
) -> dict[str, Any]:
    item_info = cast(dict[str, Any], rpc.call("items.get", {"id": item_id}) or {})
    attachments = cast(list[dict[str, Any]], rpc.call("attachments.list", {"parentId": item_id}) or [])
    chunks_attachment = _find_chunks_attachment_in(attachments)
    if chunks_attachment is None:
        return {"item_key": item_id, "status": "skipped", "reason": "missing chunks artifact"}

    chunks_path = _attachment_path(rpc, chunks_attachment)
    chunks = read_chunks_jsonl(chunks_path)
    item_key = _item_key_from_info(item_id, item_info, chunks)
    texts = [str(chunk.get("text", "")) for chunk in chunks]
    vectors = embedder.embed_batch(texts) if texts else []

    target_dir = output_dir or chunks_path.parent
    embedding_path = Path(
        write_embedding_npz(
            target_dir,
            item_key,
            vectors=vectors,
            metadata=metadata_for_chunks(chunks),
            model=model,
        ),
    )
    replaced = 0
    for attachment in attachments:
        if str(attachment.get("title") or "") != embedding_path.name:
            continue
        attachment_key = attachment.get("key")
        if attachment_key is None:
            continue
        rpc.call("attachments.delete", {"id": attachment_key})
        replaced += 1

    attachment = rpc.call(
        "attachments.add",
        {
            "parentId": item_id,
            "path": zotero_path(embedding_path),
            "title": embedding_path.name,
        },
    )

    return {
        "item_key": item_key,
        "status": "ok",
        "chunks": len(chunks),
        "chunks_attachment_key": chunks_attachment.get("key"),
        "embedding_title": embedding_path.name,
        "embedding_path": str(embedding_path),
        "embedding_attachment_key": attachment.get("key") if isinstance(attachment, dict) else None,
        "replaced": replaced,
    }


def _run_zotero_index_artifacts(args: argparse.Namespace, cfg: dict[str, Any]) -> None:
    rpc_url = cfg.get("zotero", {}).get("rpc_url", "http://localhost:23119/zotron/rpc")
    rpc = ZoteroRPC(rpc_url)
    item_ids = _zotero_item_ids_for_index(rpc, args)
    output_dir = Path(args.artifacts_dir).expanduser() if args.artifacts_dir else None
    if output_dir is not None:
        output_dir.mkdir(parents=True, exist_ok=True)

    embedder = _build_embedder(cfg)
    model = args.model or _embedding_model_from_cfg(cfg)
    items = [
        _index_zotero_item_artifact(
            rpc=rpc,
            item_id=item_id,
            embedder=embedder,
            model=model,
            output_dir=output_dir,
        )
        for item_id in item_ids
    ]
    indexed = sum(1 for item in items if item["status"] == "ok")
    print(json.dumps({
        "status": "ok",
        "indexed": indexed,
        "attached": indexed,
        "skipped": sum(1 for item in items if item["status"] == "skipped"),
        "total_chunks": sum(int(item.get("chunks", 0)) for item in items),
        "items": items,
    }, ensure_ascii=False))


def _run_artifact_search(args: argparse.Namespace, cfg: dict[str, Any]) -> list[dict[str, Any]]:
    embedder = _build_embedder(cfg)
    store = _artifact_vector_store(args.artifacts_dir, args.item_key)
    return store.search(embedder.embed(args.query), top_k=args.top_k, query=args.query)


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



def _emit_hits(hits: list[dict[str, Any]], output: str) -> None:
    if output == "jsonl":
        for hit in hits:
            print(json.dumps(hit, ensure_ascii=False, separators=(",", ":")))
    else:
        print(json.dumps({"hits": hits, "total": len(hits)}, ensure_ascii=False, indent=2))


def _run_zotero_hits(
    query: str,
    collection: str | None,
    output: str,
    top_k: int,
    cfg: dict[str, Any],
    *,
    top_spans_per_item: int | None = None,
    include_fulltext_spans: bool = False,
) -> None:
    if not collection:
        print(json.dumps({"error": "--collection is required when --zotero is used"}), file=sys.stderr)
        sys.exit(2)
    rpc_url = cfg.get("zotero", {}).get("rpc_url", "http://localhost:23119/zotron/rpc")
    rpc = ZoteroRPC(rpc_url)
    payload = rpc.call(
        "rag.searchHits",
        {
            "query": query,
            "collection": collection,
            "limit": top_k,
            "top_spans_per_item": top_spans_per_item or 3,
            "include_fulltext_spans": include_fulltext_spans,
        },
    )
    hits = payload.get("hits", []) if isinstance(payload, dict) else []
    _emit_hits(list(hits), output)


def _run_hits(
    query: str,
    collection: str | None,
    output: str,
    top_k: int,
    cfg: dict[str, Any],
    *,
    artifacts_dir: str | None = None,
    item_key: str | None = None,
    zotero: bool = False,
    top_spans_per_item: int | None = None,
    include_fulltext_spans: bool = False,
) -> None:
    if zotero:
        _run_zotero_hits(
            query,
            collection,
            output,
            top_k,
            cfg,
            top_spans_per_item=top_spans_per_item,
            include_fulltext_spans=include_fulltext_spans,
        )
        return

    if artifacts_dir:
        store = _artifact_vector_store(artifacts_dir, item_key)
        embedder = _build_embedder(cfg)
        rows = store.search(embedder.embed(query), top_k=top_k, query=query)
        _emit_hits(results_to_hits(rows, query=query), output)
        return

    if not collection:
        print(json.dumps({"error": "--collection is required unless --artifacts-dir is used"}), file=sys.stderr)
        sys.exit(2)
    store_path = _store_path(collection)
    if not store_path.exists():
        print(json.dumps({"error": f"Collection not indexed: {collection!r}"}), file=sys.stderr)
        sys.exit(2)
    store = VectorStore.load(store_path)
    embedder = _build_embedder(cfg)
    rows = store.search(embedder.embed(query), top_k=top_k)
    _emit_hits(results_to_hits(rows, query=query), output)

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

    # index-artifacts
    p_index_artifacts = sub.add_parser(
        "index-artifacts",
        help="Embed Zotero-native OCR chunk artifacts into <item-key>.zotron-embed.npz",
    )
    p_index_artifacts.add_argument("--artifacts-dir", help="Directory containing <item-key>.zotron-chunks.jsonl files, or an output directory with --zotero")
    p_index_artifacts.add_argument("--item-key", help="Limit indexing to one Zotero item key")
    p_index_artifacts.add_argument("--zotero", action="store_true", help="Read chunk artifacts from Zotero item attachments and attach embedding artifacts")
    p_index_artifacts.add_argument("--item", help="Zotero item key to index when --zotero is used")
    p_index_artifacts.add_argument("--collection", help="Zotero collection name to index when --zotero is used")
    p_index_artifacts.add_argument("--model", help="Embedding model label to store in the artifact")

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
    p_search.add_argument("--collection", help="Collection name")
    p_search.add_argument("--artifacts-dir", help="Directory containing chunk and embedding artifacts")
    p_search.add_argument("--item-key", help="Limit artifact search to one Zotero item key")
    p_search.add_argument("--limit", "--top-k", dest="top_k", type=int, default=5, help="Maximum results to emit")
    p_search.add_argument("query", help="Query text")

    # status
    p_status = sub.add_parser("status", help="Show index status for a collection")
    p_status.add_argument("--collection", required=True, help="Collection name")

    # hits
    p_hits = sub.add_parser(
        "hits",
        help="Emit academic-zh retrieval hits with item_key/title/text provenance.",
    )
    p_hits.add_argument("query", help="Query text")
    p_hits.add_argument("--collection", help="Collection name")
    p_hits.add_argument("--artifacts-dir", help="Directory containing chunk and embedding artifacts")
    p_hits.add_argument("--item-key", help="Limit artifact search to one Zotero item key")
    p_hits.add_argument("--zotero", action="store_true", help="Use Zotero XPI rag.searchHits JSON-RPC backend")
    p_hits.add_argument("--top-spans-per-item", type=int, default=3, help="Maximum Zotero-backed hits per item")
    p_hits.add_argument("--include-fulltext-spans", action="store_true", help="Ask Zotero backend to include fulltext fallback spans")
    p_hits.add_argument("--limit", "--top-k", dest="top_k", type=int, default=50, help="Maximum hits to emit")
    p_hits.add_argument("--output", choices=["json", "jsonl"], default="json", help="Output format")

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

    if args.command == "hits":
        _run_hits(
            args.query,
            args.collection,
            args.output,
            args.top_k,
            cfg,
            artifacts_dir=args.artifacts_dir,
            item_key=args.item_key,
            zotero=args.zotero,
            top_spans_per_item=args.top_spans_per_item,
            include_fulltext_spans=args.include_fulltext_spans,
        )
        return

    if args.command == "index-artifacts":
        cmd_index_artifacts(args, cfg)
        return

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

    dispatch = {"index": cmd_index, "search": cmd_search, "status": cmd_status, "hits": cmd_hits}
    dispatch[args.command](args, cfg)


if __name__ == "__main__":
    main()
