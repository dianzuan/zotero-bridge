"""`zotron` command-line interface — thin wrapper over zotron XPI.

Each namespace lives in its own module (cli_items.py, cli_collections.py, etc.)
for parallel development. This file defines top-level commands and registers
all namespace sub-apps.
"""
from __future__ import annotations

import json
import sys
from dataclasses import asdict
from pathlib import Path
from typing import Literal, NoReturn, cast

import typer

from zotron._cli_base import (
    DEFAULT_URL,
    app,
    die,
    emit_or_die,
    new_rpc,
    resolve_or_die,
    rpc_or_die,
)
from zotron._output import emit
from zotron.errors import (
    CollectionAmbiguous,
    CollectionNotFound,
    InvalidPDF,
    ZotronError,
)
from zotron.push import push_item, resolve_collection
from zotron.rpc import ZoteroRPC

# Re-export for backward compat (test_cli.py imports `from zotron.cli import app`)
__all__ = ["app"]

# --- Register namespace sub-apps ---
from zotron.cli_collections import collections_app  # noqa: E402
from zotron.cli_items import items_app  # noqa: E402
from zotron.cli_search import search_app  # noqa: E402
from zotron.cli_tags import tags_app  # noqa: E402
from zotron.cli_export import export_app  # noqa: E402
from zotron.cli_system import system_app  # noqa: E402
from zotron.cli_notes import notes_app  # noqa: E402
from zotron.cli_attachments import attachments_app  # noqa: E402
from zotron.cli_annotations import annotations_app  # noqa: E402
from zotron.cli_settings import settings_app  # noqa: E402

app.add_typer(collections_app, name="collections")
app.add_typer(items_app, name="items")
app.add_typer(search_app, name="search")
app.add_typer(tags_app, name="tags")
app.add_typer(export_app, name="export")
app.add_typer(system_app, name="system")
app.add_typer(notes_app, name="notes")
app.add_typer(attachments_app, name="attachments")
app.add_typer(annotations_app, name="annotations")
app.add_typer(settings_app, name="settings")


# --- Backward compat aliases for helpers (used in test_cli.py) ---
_new_rpc = new_rpc
_die = die
_rpc_or_die = rpc_or_die
_emit_or_die = emit_or_die
_resolve_or_die = resolve_or_die


# ---------------------------------------------------------------------------
# Top-level commands (not in any namespace)
# ---------------------------------------------------------------------------

@app.command(
    "ping",
    epilog="Examples:\n\n    zotron ping\n\n    zotron ping --url http://localhost:23119/zotron/rpc",
)
def ping(
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Check that Zotero is running with the zotron XPI enabled."""
    rpc = new_rpc(url)
    try:
        resp = rpc.call("system.ping")
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE",
             f"Cannot connect to zotron at {url}. Is Zotero running?")
    typer.echo(json.dumps(resp))


@app.command(
    "rpc",
    epilog="Examples:\n\n    zotron rpc system.ping\n\n    zotron rpc items.get '{\"id\":12345}'\n\n    zotron rpc tags.add '{\"itemId\":12345,\"tags\":[\"已读\"]}'",
)
def rpc_command(
    method: str = typer.Argument(..., help="JSON-RPC method name, e.g. 'items.get'"),
    params_json: str = typer.Argument(
        "{}",
        help="JSON-encoded params object, e.g. '{\"id\":123}'. Defaults to '{}'.",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url", help="zotron RPC endpoint URL."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
    paginate_flag: bool = typer.Option(
        False, "--paginate",
        help="Auto-loop offset/limit until exhausted (or 10k cap).",
    ),
    page_size: int = typer.Option(
        100, "--page-size",
        help="Items per page when --paginate is set.",
    ),
) -> None:
    """Generic RPC escape hatch — call any of the 76 XPI methods directly."""
    try:
        params = json.loads(params_json)
    except json.JSONDecodeError as exc:
        die("INVALID_JSON", f"params must be a JSON object: {exc}")

    rpc = new_rpc(url)
    if paginate_flag:
        from zotron._paginate import paginate
        try:
            result = paginate(rpc, method, params, page_size=page_size)
        except ConnectionError:
            die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
        except RuntimeError as e:
            die("RPC_ERROR", str(e))
    else:
        result = rpc_or_die(rpc, method, params)

    emit_or_die(result, jq_filter=jq_filter)


@app.command(
    "push",
    epilog='Examples:\n\n    cat item.json | zotron push - --pdf paper.pdf --collection "Reading List"\n\n    zotron push paper.json --on-duplicate update',
)
def push(
    json_file: str = typer.Argument(
        ...,
        help='Path to a JSON file, or "-" to read from stdin.',
    ),
    pdf: Path | None = typer.Option(None, "--pdf", help="Optional PDF attachment path."),
    collection: str = typer.Option(
        None, "--collection",
        help="Collection name (fuzzy) or numeric ID.",
    ),
    on_duplicate: str = typer.Option(
        "skip", "--on-duplicate",
        help="skip | update | create",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Parse input + resolve collection only; do not push to Zotero."),
) -> None:
    """Push prepared Zotero JSON (from file or stdin) to Zotero.

    For fetching CNKI papers end-to-end, use `cnki export`.
    """
    if on_duplicate not in ("skip", "update", "create"):
        die("INVALID_ARGS", f"--on-duplicate must be skip|update|create, got {on_duplicate!r}", 2)
    on_duplicate = cast(Literal["skip", "update", "create"], on_duplicate)

    if json_file == "-":
        payload = sys.stdin.read()
    else:
        payload = Path(json_file).read_text(encoding="utf-8")
    try:
        item_json = json.loads(payload)
    except json.JSONDecodeError as e:
        die("INVALID_JSON", f"Could not parse JSON: {e}", 2)

    if dry_run_flag:
        rpc = new_rpc(url)
        coll_id = None
        if collection is not None:
            coll_id = resolve_or_die(rpc, collection)
        typer.echo(json.dumps({
            "ok": True,
            "dryRun": True,
            "wouldPush": {
                "title": item_json.get("title"),
                "itemType": item_json.get("itemType"),
                "collectionId": coll_id,
                "pdfPath": str(pdf) if pdf else None,
                "onDuplicate": on_duplicate,
            },
        }, ensure_ascii=False))
        raise typer.Exit(code=0)

    rpc = new_rpc(url)
    try:
        result = push_item(
            rpc, item_json,
            pdf_path=pdf,
            collection=collection,
            on_duplicate=on_duplicate,
        )
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
    except InvalidPDF as e:
        die("INVALID_PDF", str(e))
    except CollectionAmbiguous as e:
        typer.echo(json.dumps({
            "ok": False,
            "error": {
                "code": "COLLECTION_AMBIGUOUS",
                "message": str(e),
                "candidates": e.candidates,
            },
        }, ensure_ascii=False))
        raise typer.Exit(code=1)
    except CollectionNotFound as e:
        die("COLLECTION_NOT_FOUND", str(e))
    except ZotronError as e:
        die("ZOTERO_ERROR", str(e))

    typer.echo(json.dumps(asdict(result)))


@app.command(
    "find-pdfs",
    epilog='Examples:\n\n    zotron find-pdfs --collection "2026-AI"\n\n    zotron find-pdfs --collection "2026-AI" --limit 20',
)
def find_pdfs(
    collection: str = typer.Option(..., "--collection", help="Collection name or ID."),
    limit: int = typer.Option(0, "--limit", help="Stop after N attempts (0 = unlimited)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Batch fill PDFs for items in a collection via Zotero's Find Available PDF.

    For each item in the collection, checks whether it already has an attachment
    via attachments.list(parentId), and if not, invokes attachments.findPDF which
    uses Zotero's PDF-resolver chain (Unpaywall, OA repos, etc.).
    """
    rpc = new_rpc(url)
    try:
        coll_id = resolve_collection(rpc, collection)
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
    except CollectionAmbiguous as e:
        typer.echo(json.dumps({
            "ok": False,
            "error": {
                "code": "COLLECTION_AMBIGUOUS",
                "message": str(e),
                "candidates": e.candidates,
            },
        }, ensure_ascii=False))
        raise typer.Exit(code=1)
    except CollectionNotFound as e:
        die("COLLECTION_NOT_FOUND", str(e))

    resp = rpc.call("collections.getItems", {"id": coll_id}) or {}
    items = cast(list[dict], resp.get("items", []) if isinstance(resp, dict) else (resp or []))

    missing = []
    for it in items:
        item_key = it.get("key")
        if item_key is None:
            continue
        attachments = cast(list[dict], rpc.call("attachments.list", {"parentId": item_key}) or [])
        has_pdf = any(
            (a.get("contentType") or "").lower() == "application/pdf"
            or (a.get("path") or "").lower().endswith(".pdf")
            for a in attachments
        )
        if not has_pdf:
            missing.append(it)
        if limit > 0 and len(missing) >= limit:
            break

    results = []
    for it in missing:
        resp = rpc.call("attachments.findPDF", {"parentId": it["key"]}) or {}
        attachment_data = resp.get("attachment")
        found = attachment_data is not None
        results.append({
            "item_key": it["key"],
            "title": it.get("title"),
            "found": found,
            "attachment_key": attachment_data.get("key") if attachment_data else None,
        })

    emit_or_die(
        {
            "scanned": len(items),
            "attempted": len(missing),
            "results": results,
        },
        jq_filter=jq_filter,
    )
