"""CLI entry point for zotron-ocr command."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, cast

from zotron.config import load_config
from zotron.rpc import ZoteroRPC
from zotron.collections import find_by_name as _find_collection_by_name
from zotron.ocr.artifacts import CHUNKS_SUFFIX
from zotron.ocr.engine import create_engine
from zotron.ocr.processor import OCRProcessor


def _make_processor(cfg: dict) -> OCRProcessor:
    rpc = ZoteroRPC(url=cfg["zotero"]["rpc_url"])
    engine = create_engine(
        provider=cfg["ocr"]["provider"],
        api_key=cfg["ocr"].get("api_key") or cfg["ocr"].get("glm_api_key") or None,
        api_url=cfg["ocr"].get("api_url"),
    )
    output_dir = cfg.get("ocr", {}).get("output_dir")
    return OCRProcessor(
        rpc=rpc,
        engine=engine,
        artifact_dir=Path(output_dir).expanduser() if output_dir else None,
    )


def cmd_status(args: argparse.Namespace, cfg: dict) -> None:
    """Show OCR stats for a collection (JSON to stdout)."""
    rpc = ZoteroRPC(url=cfg["zotero"]["rpc_url"])
    collection_id = _find_collection_by_name(rpc, args.collection)
    if collection_id is None:
        error_result = {"error": f"Collection not found: {args.collection!r}"}
        print(json.dumps(error_result, ensure_ascii=False))
        sys.exit(1)

    raw = rpc.call("collections.getItems", {"id": collection_id, "limit": 500}) or {}
    items = raw.get("items", []) if isinstance(raw, dict) else raw
    total = len(items)
    has_ocr = sum(1 for item in items if _has_ocr_result(rpc, item.get("id")))
    status_result: dict[str, Any] = {
        "collection": args.collection,
        "total": total,
        "has_ocr": has_ocr,
        "missing_ocr": total - has_ocr,
    }
    print(json.dumps(status_result, ensure_ascii=False))


def _has_ocr_note(rpc: ZoteroRPC, item_id: int) -> bool:
    notes = cast(list[dict[str, Any]], rpc.call("notes.get", {"parentId": item_id}) or [])
    return any("ocr" in (note.get("tags") or []) for note in notes)


def _has_ocr_artifact(rpc: ZoteroRPC, item_id: int) -> bool:
    attachments = cast(list[dict[str, Any]], rpc.call("attachments.list", {"parentId": item_id}) or [])
    return any(str(att.get("title") or "").endswith(CHUNKS_SUFFIX) for att in attachments)


def _has_ocr_result(rpc: ZoteroRPC, item_id: int) -> bool:
    return _has_ocr_artifact(rpc, item_id) or _has_ocr_note(rpc, item_id)


def _process_item(proc: OCRProcessor, item_id: int, *, force: bool) -> dict:
    try:
        item_info = proc.rpc.call("items.get", {"id": item_id}) or {}
        title = item_info.get("title", "(untitled)")
    except Exception:
        title = "(untitled)"
    status = proc.process_item(item_id, title, force=force)
    return {"item_id": item_id, "status": status}


def cmd_process(args: argparse.Namespace, cfg: dict) -> None:
    """Process a collection or single item (JSON result to stdout)."""
    proc = _make_processor(cfg)

    if args.item is not None:
        result = _process_item(proc, args.item, force=args.force)
    elif args.collection is not None:
        result = proc.process_collection(args.collection, force=args.force)
    else:
        print(
            json.dumps({"error": "Specify --collection or --item"}),
            file=sys.stderr,
        )
        sys.exit(1)

    print(json.dumps(result, ensure_ascii=False))


def _add_process_flags(parser: argparse.ArgumentParser, *, collection: bool, item: bool) -> None:
    if collection:
        parser.add_argument("--collection", help="Collection name to process")
    if item:
        parser.add_argument("--item", type=int, help="Single item ID to process")
    parser.add_argument(
        "--force",
        action="store_true",
        help="Re-OCR even if an OCR Note already exists",
    )


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="zotron-ocr",
        description="OCR PDFs in Zotero and write raw/block/chunk artifacts.",
    )
    _add_process_flags(parser, collection=True, item=True)

    sub = parser.add_subparsers(dest="command")
    status_p = sub.add_parser("status", help="Show OCR statistics for a collection")
    status_p.add_argument(
        "--collection", required=True, help="Collection name"
    )
    run_p = sub.add_parser("run", help="Process a collection and attach OCR artifacts")
    _add_process_flags(run_p, collection=True, item=False)
    run_p.set_defaults(item=None)

    rebuild_p = sub.add_parser("rebuild", help="Force rebuild artifacts for one item")
    _add_process_flags(rebuild_p, collection=False, item=True)
    rebuild_p.set_defaults(collection=None, force=True)

    args = parser.parse_args()
    cfg = load_config()

    try:
        if args.command == "status":
            cmd_status(args, cfg)
        elif args.command == "run":
            cmd_process(args, cfg)
        elif args.command == "rebuild":
            args.force = True
            cmd_process(args, cfg)
        else:
            if args.command == "rebuild":
                args.force = True
            cmd_process(args, cfg)
    except Exception as exc:
        print(json.dumps({"error": str(exc)}), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
