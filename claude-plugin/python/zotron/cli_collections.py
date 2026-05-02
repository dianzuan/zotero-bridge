"""CLI: collections namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, die, rpc_or_die, dry_run, emit_or_die, resolve_or_die

collections_app = typer.Typer(
    help="Inspect Zotero collections.",
    no_args_is_help=True,
)


@collections_app.command(
    "list",
    epilog="Examples:\n\n    zotron collections list",
)
def collections_list(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all collections in the user library (flat)."""
    rpc = new_rpc(url)
    try:
        resp = rpc.call("collections.list")
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
    emit_or_die(resp or [], output=output, jq_filter=jq_filter)


@collections_app.command(
    "tree",
    epilog="Examples:\n\n    zotron collections tree",
)
def collections_tree(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Print the collection hierarchy as a tree."""
    rpc = new_rpc(url)
    try:
        resp = rpc.call("collections.tree")
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
    emit_or_die(resp or {}, jq_filter=jq_filter)


@collections_app.command(
    "rename",
    epilog='Examples:\n\n    zotron collections rename "typo-案例库" "案例库"',
)
def collections_rename(
    old_name: str = typer.Argument(..., help="Current collection name (or numeric ID)."),
    new_name: str = typer.Argument(..., help="New name."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Rename a collection. Recovery path for typo-named auto-created collections."""
    rpc = new_rpc(url)
    coll_id = resolve_or_die(rpc, old_name)
    if coll_id == 0:
        die("COLLECTION_NOT_FOUND",
             f"{old_name!r} resolved to library root (no collection to rename)")
    if dry_run_flag:
        dry_run("collections.rename", {"id": coll_id, "name": new_name})
    typer.echo(json.dumps(
        rpc_or_die(rpc, "collections.rename", {"id": coll_id, "name": new_name})
    ))


@collections_app.command(
    "create",
    epilog='Examples:\n\n    zotron collections create "2026-AI"\n\n    zotron collections create "Reading List" --parent "2026-AI"',
)
def collections_create(
    name: str = typer.Argument(..., help="Collection name."),
    parent: str | None = typer.Option(
        None, "--parent",
        help="Optional parent collection (name or numeric ID).",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Create a collection, optionally nested under a parent."""
    rpc = new_rpc(url)
    params: dict = {"name": name}
    if parent is not None:
        parent_id = resolve_or_die(rpc, parent)
        if parent_id == 0:
            die("COLLECTION_NOT_FOUND",
                 f"parent {parent!r} resolved to library root")
        params["parentId"] = parent_id
    if dry_run_flag:
        dry_run("collections.create", params)
    typer.echo(json.dumps(rpc_or_die(rpc, "collections.create", params)))


@collections_app.command(
    "delete",
    epilog='Examples:\n\n    zotron collections delete "TempCollection"',
)
def collections_delete(
    name_or_id: str = typer.Argument(..., help="Collection name (fuzzy) or numeric ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete a collection (its items are not deleted -- just un-linked)."""
    rpc = new_rpc(url)
    coll_id = resolve_or_die(rpc, name_or_id)
    if coll_id == 0:
        die("COLLECTION_NOT_FOUND",
             f"{name_or_id!r} resolved to library root (can't delete)")
    if dry_run_flag:
        dry_run("collections.delete", {"id": coll_id})
    typer.echo(json.dumps(rpc_or_die(rpc, "collections.delete", {"id": coll_id})))


@collections_app.command(
    "add-items",
    epilog='Examples:\n\n    zotron collections add-items "2026-AI" 12345 12346',
)
def collections_add_items(
    collection: str = typer.Argument(..., help="Target collection name or ID."),
    item_ids: list[str] = typer.Argument(..., help="Item IDs to add."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add existing items to a collection (idempotent; dup already-in works)."""
    rpc = new_rpc(url)
    coll_id = resolve_or_die(rpc, collection)
    if coll_id == 0:
        die("COLLECTION_NOT_FOUND", "can't add to library root")
    if dry_run_flag:
        dry_run("collections.addItems", {"id": coll_id, "itemIds": item_ids})
    typer.echo(json.dumps(rpc_or_die(rpc, "collections.addItems",
                                      {"id": coll_id, "itemIds": item_ids})))


@collections_app.command(
    "remove-items",
    epilog='Examples:\n\n    zotron collections remove-items "2026-AI" 12345',
)
def collections_remove_items(
    collection: str = typer.Argument(..., help="Collection name or ID."),
    item_ids: list[str] = typer.Argument(..., help="Item IDs to remove from collection."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Remove items from a collection (items themselves are kept in library)."""
    rpc = new_rpc(url)
    coll_id = resolve_or_die(rpc, collection)
    if coll_id == 0:
        die("COLLECTION_NOT_FOUND", "can't operate on library root")
    if dry_run_flag:
        dry_run("collections.removeItems", {"id": coll_id, "itemIds": item_ids})
    typer.echo(json.dumps(rpc_or_die(rpc, "collections.removeItems",
                                      {"id": coll_id, "itemIds": item_ids})))
