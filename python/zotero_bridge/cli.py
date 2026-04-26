"""`zotero-bridge` command-line interface — thin wrapper over zotero_bridge.

Exposes agent-friendly commands:
  zotero-bridge ping                       — health check
  zotero-bridge push <json> --pdf --collection
  zotero-bridge collections list|tree
  zotero-bridge find-pdfs --collection NAME
"""
from __future__ import annotations

import json
import sys
from dataclasses import asdict
from pathlib import Path

import typer

from zotero_bridge._output import emit
from zotero_bridge.errors import (
    CollectionAmbiguous,
    CollectionNotFound,
    InvalidPDF,
    ZoteroBridgeError,
)
from zotero_bridge.push import push_item, resolve_collection
from zotero_bridge.rpc import ZoteroRPC

DEFAULT_URL = "http://127.0.0.1:23119/zotero-bridge/rpc"

app = typer.Typer(
    name="zotero-bridge",
    help="Python client + CLI for the Zotero Bridge XPI.\n\n"
         "For fetching CNKI papers end-to-end, use `cnki export`.",
    no_args_is_help=True,
    context_settings={"help_option_names": ["-h", "--help"]},
)

collections_app = typer.Typer(
    help="Inspect Zotero collections.",
    no_args_is_help=True,
)
app.add_typer(collections_app, name="collections")


def _new_rpc(url: str) -> ZoteroRPC:
    return ZoteroRPC(url)


def _die(code: str, message: str, exit_code: int = 1) -> None:
    typer.echo(json.dumps({"ok": False, "error": {"code": code, "message": message}}, ensure_ascii=False))
    raise typer.Exit(code=exit_code)


def _rpc_or_die(rpc: ZoteroRPC, method: str, params: dict | None = None):
    """Call an RPC method, translating connection errors to the standard
    JSON error envelope. Returns the `result` field of the response, or {}
    if the XPI returned nothing."""
    try:
        resp = rpc.call(method, params) if params is not None else rpc.call(method)
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")
    except RuntimeError as e:
        # rpc.call raises RuntimeError for XPI-side JSON-RPC errors
        _die("RPC_ERROR", str(e))
    return resp if resp is not None else {}


def _dry_run(method: str, params: dict | None = None) -> None:
    """Emit the standard dry-run envelope and exit 0."""
    typer.echo(json.dumps({
        "ok": True,
        "dryRun": True,
        "wouldCall": method,
        "wouldCallParams": params or {},
    }, ensure_ascii=False))
    raise typer.Exit(code=0)


def _emit_or_die(data, *, output: str = "json", jq_filter: str | None = None) -> None:
    """Call emit() with the standard INVALID_JQ envelope translation.

    Use this anywhere a command surfaces --jq to the user; it routes any
    ValueError raised by emit() to the JSON envelope on stdout via _die().
    """
    try:
        emit(data, output=output, jq_filter=jq_filter)
    except ValueError as e:
        _die("INVALID_JQ", str(e))


def _resolve_or_die(rpc: ZoteroRPC, name_or_id: str) -> int:
    """Resolve a collection name/id to an integer id, dying with the
    standard envelope on ambiguity / not-found / connection failure."""
    try:
        return resolve_collection(rpc, name_or_id)
    except CollectionAmbiguous as e:
        typer.echo(json.dumps({
            "ok": False,
            "error": {
                "code": "COLLECTION_AMBIGUOUS",
                "message": str(e),
                "candidates": e.candidates,
            },
        }, ensure_ascii=False))
        raise typer.Exit(code=1) from None
    except CollectionNotFound as e:
        _die("COLLECTION_NOT_FOUND", str(e))
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")


@app.command(
    "ping",
    epilog="Examples:\n\n    zotero-bridge ping\n\n    zotero-bridge ping --url http://localhost:23119/zotero-bridge/rpc",
)
def ping(
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Check that Zotero is running with the zotero-bridge XPI enabled."""
    rpc = _new_rpc(url)
    try:
        resp = rpc.call("system.ping")
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE",
             f"Cannot connect to zotero-bridge at {url}. Is Zotero running?")
    typer.echo(json.dumps(resp))


@app.command(
    "rpc",
    epilog="Examples:\n\n    zotero-bridge rpc system.ping\n\n    zotero-bridge rpc items.get '{\"id\":12345}'\n\n    zotero-bridge rpc tags.add '{\"itemId\":12345,\"tags\":[\"已读\"]}'",
)
def rpc_command(
    method: str = typer.Argument(..., help="JSON-RPC method name, e.g. 'items.get'"),
    params_json: str = typer.Argument(
        "{}",
        help="JSON-encoded params object, e.g. '{\"id\":123}'. Defaults to '{}'.",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url", help="zotero-bridge RPC endpoint URL."),
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
        _die("INVALID_JSON", f"params must be a JSON object: {exc}")

    rpc = _new_rpc(url)
    if paginate_flag:
        from zotero_bridge._paginate import paginate
        try:
            result = paginate(rpc, method, params, page_size=page_size)
        except ConnectionError:
            _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")
        except RuntimeError as e:
            _die("RPC_ERROR", str(e))
    else:
        result = _rpc_or_die(rpc, method, params)

    _emit_or_die(result, jq_filter=jq_filter)


@collections_app.command(
    "list",
    epilog="Examples:\n\n    zotero-bridge collections list",
)
def collections_list(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all collections in the user library (flat)."""
    rpc = _new_rpc(url)
    try:
        resp = rpc.call("collections.list")
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")
    _emit_or_die(resp or [], output=output, jq_filter=jq_filter)


@collections_app.command(
    "tree",
    epilog="Examples:\n\n    zotero-bridge collections tree",
)
def collections_tree(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Print the collection hierarchy as a tree."""
    rpc = _new_rpc(url)
    try:
        resp = rpc.call("collections.tree")
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")
    _emit_or_die(resp or {}, jq_filter=jq_filter)


@collections_app.command(
    "rename",
    epilog="Examples:\n\n    zotero-bridge collections rename \"typo-案例库\" \"案例库\"",
)
def collections_rename(
    old_name: str = typer.Argument(..., help="Current collection name (or numeric ID)."),
    new_name: str = typer.Argument(..., help="New name."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Rename a collection. Recovery path for typo-named auto-created collections."""
    rpc = _new_rpc(url)
    coll_id = _resolve_or_die(rpc, old_name)
    if coll_id == 0:
        _die("COLLECTION_NOT_FOUND",
             f"{old_name!r} resolved to library root (no collection to rename)")
    if dry_run:
        _dry_run("collections.rename", {"id": coll_id, "name": new_name})
    typer.echo(json.dumps(
        _rpc_or_die(rpc, "collections.rename", {"id": coll_id, "name": new_name})
    ))


@collections_app.command(
    "create",
    epilog="Examples:\n\n    zotero-bridge collections create \"2026-AI\"\n\n    zotero-bridge collections create \"Reading List\" --parent \"2026-AI\"",
)
def collections_create(
    name: str = typer.Argument(..., help="Collection name."),
    parent: str | None = typer.Option(
        None, "--parent",
        help="Optional parent collection (name or numeric ID).",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Create a collection, optionally nested under a parent."""
    rpc = _new_rpc(url)
    params: dict = {"name": name}
    if parent is not None:
        parent_id = _resolve_or_die(rpc, parent)
        if parent_id == 0:
            _die("COLLECTION_NOT_FOUND",
                 f"parent {parent!r} resolved to library root")
        params["parentId"] = parent_id
    if dry_run:
        _dry_run("collections.create", params)
    typer.echo(json.dumps(_rpc_or_die(rpc, "collections.create", params)))


@collections_app.command(
    "delete",
    epilog="Examples:\n\n    zotero-bridge collections delete \"TempCollection\"",
)
def collections_delete(
    name_or_id: str = typer.Argument(..., help="Collection name (fuzzy) or numeric ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete a collection (its items are not deleted — just un-linked)."""
    rpc = _new_rpc(url)
    coll_id = _resolve_or_die(rpc, name_or_id)
    if coll_id == 0:
        _die("COLLECTION_NOT_FOUND",
             f"{name_or_id!r} resolved to library root (can't delete)")
    if dry_run:
        _dry_run("collections.delete", {"id": coll_id})
    typer.echo(json.dumps(_rpc_or_die(rpc, "collections.delete", {"id": coll_id})))


@collections_app.command(
    "add-items",
    epilog="Examples:\n\n    zotero-bridge collections add-items \"2026-AI\" 12345 12346",
)
def collections_add_items(
    collection: str = typer.Argument(..., help="Target collection name or ID."),
    item_ids: list[int] = typer.Argument(..., help="Item IDs to add."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add existing items to a collection (idempotent; dup already-in works)."""
    rpc = _new_rpc(url)
    coll_id = _resolve_or_die(rpc, collection)
    if coll_id == 0:
        _die("COLLECTION_NOT_FOUND", "can't add to library root")
    if dry_run:
        _dry_run("collections.addItems", {"id": coll_id, "itemIds": item_ids})
    typer.echo(json.dumps(_rpc_or_die(rpc, "collections.addItems",
                                      {"id": coll_id, "itemIds": item_ids})))


@collections_app.command(
    "remove-items",
    epilog="Examples:\n\n    zotero-bridge collections remove-items \"2026-AI\" 12345",
)
def collections_remove_items(
    collection: str = typer.Argument(..., help="Collection name or ID."),
    item_ids: list[int] = typer.Argument(..., help="Item IDs to remove from collection."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Remove items from a collection (items themselves are kept in library)."""
    rpc = _new_rpc(url)
    coll_id = _resolve_or_die(rpc, collection)
    if coll_id == 0:
        _die("COLLECTION_NOT_FOUND", "can't operate on library root")
    if dry_run:
        _dry_run("collections.removeItems", {"id": coll_id, "itemIds": item_ids})
    typer.echo(json.dumps(_rpc_or_die(rpc, "collections.removeItems",
                                      {"id": coll_id, "itemIds": item_ids})))


@app.command(
    "push",
    epilog="Examples:\n\n    cat item.json | zotero-bridge push - --pdf paper.pdf --collection \"Reading List\"\n\n    zotero-bridge push paper.json --on-duplicate update",
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
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Parse input + resolve collection only; do not push to Zotero."),
) -> None:
    """Push prepared Zotero JSON (from file or stdin) to Zotero.

    For fetching CNKI papers end-to-end, use `cnki export`.
    """
    if on_duplicate not in ("skip", "update", "create"):
        _die("INVALID_ARGS", f"--on-duplicate must be skip|update|create, got {on_duplicate!r}", 2)

    # Read payload
    if json_file == "-":
        payload = sys.stdin.read()
    else:
        payload = Path(json_file).read_text(encoding="utf-8")
    try:
        item_json = json.loads(payload)
    except json.JSONDecodeError as e:
        _die("INVALID_JSON", f"Could not parse JSON: {e}", 2)

    if dry_run:
        rpc = _new_rpc(url)
        coll_id = None
        if collection is not None:
            coll_id = _resolve_or_die(rpc, collection)
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

    rpc = _new_rpc(url)
    try:
        result = push_item(
            rpc, item_json,
            pdf_path=pdf,
            collection=collection,
            on_duplicate=on_duplicate,
        )
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")
    except InvalidPDF as e:
        _die("INVALID_PDF", str(e))
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
        _die("COLLECTION_NOT_FOUND", str(e))
    except ZoteroBridgeError as e:
        _die("ZOTERO_ERROR", str(e))

    typer.echo(json.dumps(asdict(result)))


@app.command(
    "find-pdfs",
    epilog="Examples:\n\n    zotero-bridge find-pdfs --collection \"2026-AI\"\n\n    zotero-bridge find-pdfs --collection \"2026-AI\" --limit 20",
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
    rpc = _new_rpc(url)
    try:
        coll_id = resolve_collection(rpc, collection)
    except ConnectionError:
        _die("ZOTERO_UNAVAILABLE", "Cannot connect to zotero-bridge")
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
        _die("COLLECTION_NOT_FOUND", str(e))

    # collections.getItems returns {items: [...], total: N}
    resp = rpc.call("collections.getItems", {"id": coll_id}) or {}
    items = resp.get("items", []) if isinstance(resp, dict) else (resp or [])

    # Filter to items missing PDFs. Ask the XPI per-item via attachments.list.
    missing = []
    for it in items:
        item_id = it.get("id")
        if item_id is None:
            continue
        attachments = rpc.call("attachments.list", {"parentId": item_id}) or []
        # Match on MIME type OR .pdf filename suffix. Zotero may store some
        # attachments with non-standard contentType; a MIME-only check would
        # false-negative and we'd try to find another PDF the item already has.
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
        resp = rpc.call("attachments.findPDF", {"parentId": it["id"]}) or {}
        found = bool(resp.get("found"))
        attachment_data = resp.get("attachment") if found else None
        results.append({
            "item_id": it["id"],
            "title": it.get("title"),
            "found": found,
            "attachment_id": attachment_data.get("id") if attachment_data else None,
        })

    _emit_or_die(
        {
            "scanned": len(items),
            "attempted": len(missing),
            "results": results,
        },
        jq_filter=jq_filter,
    )


# ---------------------------------------------------------------------------
# items sub-app — standalone add-by-identifier + maintenance ops
# ---------------------------------------------------------------------------

items_app = typer.Typer(
    help="Add items by DOI/ISBN/URL; inspect; trash; find/merge duplicates.",
    no_args_is_help=True,
)
app.add_typer(items_app, name="items")


@items_app.command(
    "get",
    epilog="Examples:\n\n    zotero-bridge items get 12345",
)
def items_get(
    item_id: int = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Print the full serialization of an item by id."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "items.get", {"id": item_id}),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "add-by-doi",
    epilog="Examples:\n\n    zotero-bridge items add-by-doi 10.1038/nature12373\n\n    zotero-bridge items add-by-doi 10.1038/nature12373 --collection \"2026-AI\"",
)
def items_add_by_doi(
    doi: str = typer.Argument(...),
    collection: str | None = typer.Option(None, "--collection"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a paper by DOI (uses Zotero's search translators)."""
    rpc = _new_rpc(url)
    params: dict = {"doi": doi}
    if collection is not None:
        coll_id = _resolve_or_die(rpc, collection) or None
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run:
        _dry_run("items.addByDOI", params)
    typer.echo(json.dumps(_rpc_or_die(rpc, "items.addByDOI", params)))


@items_app.command(
    "add-by-isbn",
    epilog="Examples:\n\n    zotero-bridge items add-by-isbn 9780262035613",
)
def items_add_by_isbn(
    isbn: str = typer.Argument(...),
    collection: str | None = typer.Option(None, "--collection"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a book by ISBN."""
    rpc = _new_rpc(url)
    params: dict = {"isbn": isbn}
    if collection is not None:
        coll_id = _resolve_or_die(rpc, collection) or None
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run:
        _dry_run("items.addByISBN", params)
    typer.echo(json.dumps(_rpc_or_die(rpc, "items.addByISBN", params)))


@items_app.command(
    "add-by-url",
    epilog="Examples:\n\n    zotero-bridge items add-by-url https://arxiv.org/abs/1706.03762",
)
def items_add_by_url(
    page_url: str = typer.Argument(...),
    collection: str | None = typer.Option(None, "--collection"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a web resource via Zotero's web translator."""
    rpc = _new_rpc(url)
    params: dict = {"url": page_url}
    if collection is not None:
        coll_id = _resolve_or_die(rpc, collection) or None
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run:
        _dry_run("items.addByURL", params)
    typer.echo(json.dumps(_rpc_or_die(rpc, "items.addByURL", params)))


@items_app.command(
    "trash",
    epilog="Examples:\n\n    zotero-bridge items trash 12345",
)
def items_trash(
    item_id: int = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Move item to trash (reversible via `restore`)."""
    if dry_run:
        _dry_run("items.trash", {"id": item_id})
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url), "items.trash",
                                      {"id": item_id})))


@items_app.command(
    "restore",
    epilog="Examples:\n\n    zotero-bridge items restore 12345",
)
def items_restore(
    item_id: int = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Restore a trashed item."""
    if dry_run:
        _dry_run("items.restore", {"id": item_id})
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url), "items.restore",
                                      {"id": item_id})))


@items_app.command(
    "find-duplicates",
    epilog="Examples:\n\n    zotero-bridge items find-duplicates",
)
def items_find_duplicates(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Run Zotero's duplicate scan and print groups."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "items.findDuplicates"),
                 jq_filter=jq_filter)


@items_app.command(
    "merge-duplicates",
    epilog="Examples:\n\n    zotero-bridge items merge-duplicates 12345 12346 12347",
)
def items_merge_duplicates(
    ids: list[int] = typer.Argument(..., help="First id is the master; rest merged into it."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Merge a group of duplicate items. Takes ≥2 ids."""
    if len(ids) < 2:
        _die("INVALID_ARGS", "need at least 2 ids to merge", 2)
    if dry_run:
        _dry_run("items.mergeDuplicates", {"ids": ids})
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url), "items.mergeDuplicates",
                                      {"ids": ids})))


# ---------------------------------------------------------------------------
# search sub-app
# ---------------------------------------------------------------------------

search_app = typer.Typer(
    help="Search items by text / tag / identifier.",
    no_args_is_help=True,
)
app.add_typer(search_app, name="search")


@search_app.command(
    "quick",
    epilog="Examples:\n\n    zotero-bridge search quick \"transformer\" --limit 10",
)
def search_quick(
    query: str = typer.Argument(...),
    limit: int = typer.Option(50, "--limit"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Zotero quick-search (title, creator, year, tags)."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "search.quick",
                             {"query": query, "limit": limit}),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "fulltext",
    epilog="Examples:\n\n    zotero-bridge search fulltext \"attention is all you need\"",
)
def search_fulltext(
    query: str = typer.Argument(...),
    limit: int = typer.Option(50, "--limit"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Full-text search across PDF contents."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "search.fulltext",
                             {"query": query, "limit": limit}),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "by-identifier",
    epilog="Examples:\n\n    zotero-bridge search by-identifier --doi 10.1038/nature12373\n\n    zotero-bridge search by-identifier --isbn 9780262035613",
)
def search_by_identifier(
    doi: str | None = typer.Option(None, "--doi"),
    isbn: str | None = typer.Option(None, "--isbn"),
    issn: str | None = typer.Option(None, "--issn"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Find an item by DOI / ISBN / ISSN.

    PMID is intentionally not supported here — Zotero stores PMIDs in the
    free-form `Extra` field rather than a structured identifier column,
    so a typed search isn't reliable. Use `search quick <pmid>` instead.
    """
    params = {k: v for k, v in
              {"doi": doi, "isbn": isbn, "issn": issn}.items()
              if v}
    if not params:
        _die("INVALID_ARGS", "give at least one of --doi/--isbn/--issn", 2)
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url),
                                      "search.byIdentifier", params)))


# ---------------------------------------------------------------------------
# tags sub-app
# ---------------------------------------------------------------------------

tags_app = typer.Typer(
    help="Inspect / rename / delete tags across the library.",
    no_args_is_help=True,
)
app.add_typer(tags_app, name="tags")


@tags_app.command(
    "list",
    epilog="Examples:\n\n    zotero-bridge tags list --limit 100",
)
def tags_list(
    limit: int = typer.Option(200, "--limit"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all tags in the library (flat)."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "tags.list", {"limit": limit}),
                 output=output, jq_filter=jq_filter)


@tags_app.command(
    "rename",
    epilog="Examples:\n\n    zotero-bridge tags rename \"todo\" \"to-read\"",
)
def tags_rename(
    old: str = typer.Argument(...),
    new: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Rename a tag across all items."""
    if dry_run:
        _dry_run("tags.rename", {"oldName": old, "newName": new})
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url), "tags.rename",
                                      {"oldName": old, "newName": new})))


@tags_app.command(
    "delete",
    epilog="Examples:\n\n    zotero-bridge tags delete \"outdated-tag\"",
)
def tags_delete(
    tag: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete a tag library-wide (removes from every item that had it)."""
    if dry_run:
        _dry_run("tags.delete", {"tag": tag})
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url), "tags.delete",
                                      {"tag": tag})))


# ---------------------------------------------------------------------------
# export sub-app — citation export formats
# ---------------------------------------------------------------------------

export_app = typer.Typer(
    help="Export items as bibtex / ris / csl-json / csv / formatted bibliography.",
    no_args_is_help=True,
)
app.add_typer(export_app, name="export")


def _export_fmt(method: str, ids: list[int], url: str) -> None:
    rpc = _new_rpc(url)
    resp = _rpc_or_die(rpc, method, {"ids": ids})
    # export.* returns {content: "..."} — emit raw content, not envelope
    if isinstance(resp, dict) and "content" in resp:
        typer.echo(resp["content"])
    else:
        typer.echo(json.dumps(resp))


@export_app.command(
    "bibtex",
    epilog="Examples:\n\n    zotero-bridge export bibtex 12345 12346",
)
def export_bibtex(
    ids: list[int] = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print BibTeX for the given item ids."""
    _export_fmt("export.bibtex", ids, url)


@export_app.command(
    "ris",
    epilog="Examples:\n\n    zotero-bridge export ris 12345",
)
def export_ris(
    ids: list[int] = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print RIS for the given item ids."""
    _export_fmt("export.ris", ids, url)


@export_app.command(
    "csl-json",
    epilog="Examples:\n\n    zotero-bridge export csl-json 12345",
)
def export_csl_json(
    ids: list[int] = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print CSL-JSON for the given item ids."""
    _export_fmt("export.cslJson", ids, url)


@export_app.command(
    "bibliography",
    epilog="Examples:\n\n    zotero-bridge export bibliography 12345 --style apa\n\n    zotero-bridge export bibliography 12345 --html",
)
def export_bibliography(
    ids: list[int] = typer.Argument(...),
    style: str = typer.Option(
        "http://www.zotero.org/styles/gb-t-7714-2015-numeric", "--style",
        help="CSL style URL or short name (e.g. apa, chicago-author-date).",
    ),
    html: bool = typer.Option(
        False, "--html",
        help="Emit HTML bibliography instead of plain text.",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print a formatted bibliography (default GB/T 7714 numeric, plain text)."""
    rpc = _new_rpc(url)
    resp = _rpc_or_die(rpc, "export.bibliography", {"ids": ids, "style": style})
    # XPI returns {format, style, html, text, count}. Emit the chosen variant raw;
    # fall back to full JSON only if the XPI response shape is unexpected.
    if isinstance(resp, dict) and ("html" in resp or "text" in resp):
        typer.echo(resp["html"] if html else resp.get("text", ""))
    else:
        typer.echo(json.dumps(resp))


# ---------------------------------------------------------------------------
# system sub-app — version / sync / libraries
# ---------------------------------------------------------------------------

system_app = typer.Typer(
    help="Plugin version, trigger sync, list libraries.",
    no_args_is_help=True,
)
app.add_typer(system_app, name="system")


@system_app.command(
    "version",
    epilog="Examples:\n\n    zotero-bridge system version",
)
def system_version(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Show XPI version + the methods it exposes."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "system.version"), jq_filter=jq_filter)


@system_app.command(
    "sync",
    epilog="Examples:\n\n    zotero-bridge system sync",
)
def system_sync(
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Trigger a Zotero sync (cloud ↔ local)."""
    typer.echo(json.dumps(_rpc_or_die(_new_rpc(url), "system.sync")))


@system_app.command(
    "libraries",
    epilog="Examples:\n\n    zotero-bridge system libraries",
)
def system_libraries(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all libraries (user + groups)."""
    _emit_or_die(_rpc_or_die(_new_rpc(url), "system.libraries"),
                 output=output, jq_filter=jq_filter)
