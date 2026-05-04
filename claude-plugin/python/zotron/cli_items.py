"""CLI: items namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, die, rpc_or_die, dry_run, emit_or_die, resolve_or_die

items_app = typer.Typer(
    help="Add items by DOI/ISBN/URL; inspect; trash; find/merge duplicates.",
    no_args_is_help=True,
)


@items_app.command(
    "get",
    epilog="Examples:\n\n    zotron items get 12345",
)
def items_get(
    item: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Print the full serialization of an item by id."""
    emit_or_die(rpc_or_die(new_rpc(url), "items.get", {"key": item}),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "add-by-doi",
    epilog='Examples:\n\n    zotron items add-by-doi 10.1038/nature12373\n\n    zotron items add-by-doi 10.1038/nature12373 --collection "2026-AI"',
)
def items_add_by_doi(
    doi: str = typer.Argument(...),
    collection: str | None = typer.Option(None, "--collection"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a paper by DOI (uses Zotero's search translators)."""
    rpc = new_rpc(url)
    params: dict = {"doi": doi}
    if collection is not None:
        coll_id = resolve_or_die(rpc, collection) or None
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run_flag:
        dry_run("items.addByDOI", params)
    typer.echo(json.dumps(rpc_or_die(rpc, "items.addByDOI", params)))


@items_app.command(
    "add-by-isbn",
    epilog="Examples:\n\n    zotron items add-by-isbn 9780262035613",
)
def items_add_by_isbn(
    isbn: str = typer.Argument(...),
    collection: str | None = typer.Option(None, "--collection"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a book by ISBN."""
    rpc = new_rpc(url)
    params: dict = {"isbn": isbn}
    if collection is not None:
        coll_id = resolve_or_die(rpc, collection) or None
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run_flag:
        dry_run("items.addByISBN", params)
    typer.echo(json.dumps(rpc_or_die(rpc, "items.addByISBN", params)))


@items_app.command(
    "add-by-url",
    epilog="Examples:\n\n    zotron items add-by-url https://arxiv.org/abs/1706.03762",
)
def items_add_by_url(
    page_url: str = typer.Argument(...),
    collection: str | None = typer.Option(None, "--collection"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a web resource via Zotero's web translator."""
    rpc = new_rpc(url)
    params: dict = {"url": page_url}
    if collection is not None:
        coll_id = resolve_or_die(rpc, collection) or None
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run_flag:
        dry_run("items.addByURL", params)
    typer.echo(json.dumps(rpc_or_die(rpc, "items.addByURL", params)))


@items_app.command(
    "trash",
    epilog="Examples:\n\n    zotron items trash 12345",
)
def items_trash(
    item: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Move item to trash (reversible via `restore`)."""
    if dry_run_flag:
        dry_run("items.trash", {"key": item})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.trash",
                                      {"key": item})))


@items_app.command(
    "restore",
    epilog="Examples:\n\n    zotron items restore 12345",
)
def items_restore(
    item: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Restore a trashed item."""
    if dry_run_flag:
        dry_run("items.restore", {"key": item})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.restore",
                                      {"key": item})))


@items_app.command(
    "find-duplicates",
    epilog="Examples:\n\n    zotron items find-duplicates",
)
def items_find_duplicates(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Run Zotero's duplicate scan and print groups."""
    emit_or_die(rpc_or_die(new_rpc(url), "items.findDuplicates"),
                 jq_filter=jq_filter)


@items_app.command(
    "merge-duplicates",
    epilog="Examples:\n\n    zotron items merge-duplicates 12345 12346 12347",
)
def items_merge_duplicates(
    ids: list[str] = typer.Argument(..., help="First id is the master; rest merged into it."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Merge a group of duplicate items. Takes >= 2 ids."""
    if len(ids) < 2:
        die("INVALID_ARGS", "need at least 2 ids to merge", 2)
    if dry_run_flag:
        dry_run("items.mergeDuplicates", {"keys": ids})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.mergeDuplicates",
                                      {"keys": ids})))


@items_app.command(
    "list",
    epilog="Examples:\n\n    zotron items list\n\n    zotron items list --limit 20 --offset 0 --sort title --direction asc",
)
def items_list(
    limit: int = typer.Option(50, "--limit"),
    offset: int = typer.Option(0, "--offset"),
    sort: str | None = typer.Option(None, "--sort", help="Field to sort by."),
    direction: str = typer.Option("asc", "--direction", help="asc or desc."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List items in the library with optional sorting and pagination."""
    params: dict = {"limit": limit, "offset": offset, "direction": direction}
    if sort is not None:
        params["sort"] = sort
    emit_or_die(rpc_or_die(new_rpc(url), "items.list", params),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "create",
    epilog="Examples:\n\n    zotron items create --type journalArticle --field title=My Paper --field year=2026",
)
def items_create(
    item_type: str = typer.Option(..., "--type", help="Zotero item type, e.g. journalArticle."),
    fields: list[str] = typer.Option(None, "--field",
        help="Field as key=value (repeatable)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Create a new item of the given type."""
    field_dict: dict = {}
    for f in (fields or []):
        if "=" not in f:
            die("INVALID_ARGS", f"--field must be key=value, got: {f!r}", 2)
        k, v = f.split("=", 1)
        field_dict[k] = v
    params: dict = {"itemType": item_type}
    if field_dict:
        params["fields"] = field_dict
    if dry_run_flag:
        dry_run("items.create", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.create", params)))


@items_app.command(
    "update",
    epilog="Examples:\n\n    zotron items update 12345 --field title=New Title --field year=2026",
)
def items_update(
    item_id: str = typer.Argument(..., help="Item numeric ID or 8-char key."),
    fields: list[str] = typer.Option(None, "--field",
        help="Field as key=value (repeatable)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Update fields on an existing item."""
    field_dict: dict = {}
    for f in (fields or []):
        if "=" not in f:
            die("INVALID_ARGS", f"--field must be key=value, got: {f!r}", 2)
        k, v = f.split("=", 1)
        field_dict[k] = v
    params: dict = {"key": item_id}
    if field_dict:
        params["fields"] = field_dict
    if dry_run_flag:
        dry_run("items.update", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.update", params)))


@items_app.command(
    "delete",
    epilog="Examples:\n\n    zotron items delete 12345",
)
def items_delete(
    item_id: str = typer.Argument(..., help="Item numeric ID or 8-char key."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Permanently delete an item (bypasses trash)."""
    if dry_run_flag:
        dry_run("items.delete", {"key": item_id})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.delete", {"key": item_id})))


@items_app.command(
    "list-trash",
    epilog="Examples:\n\n    zotron items list-trash\n\n    zotron items list-trash --limit 10 --offset 0",
)
def items_list_trash(
    limit: int = typer.Option(50, "--limit"),
    offset: int = typer.Option(0, "--offset"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List items currently in the trash."""
    emit_or_die(rpc_or_die(new_rpc(url), "items.getTrash",
                              {"limit": limit, "offset": offset}),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "batch-trash",
    epilog="Examples:\n\n    zotron items batch-trash 12345 12346 12347",
)
def items_batch_trash(
    ids: list[str] = typer.Argument(..., help="Item IDs or 8-char keys to trash."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Move multiple items to trash in one call."""
    if dry_run_flag:
        dry_run("items.batchTrash", {"keys": ids})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.batchTrash", {"keys": ids})))


@items_app.command(
    "recent",
    epilog="Examples:\n\n    zotron items recent\n\n    zotron items recent --limit 10 --type modified",
)
def items_recent(
    limit: int = typer.Option(20, "--limit"),
    offset: int = typer.Option(0, "--offset"),
    recent_type: str = typer.Option("added", "--type",
        help="added or modified."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List recently added or modified items."""
    if recent_type not in ("added", "modified"):
        die("INVALID_ARGS", f"--type must be added or modified, got {recent_type!r}", 2)
    emit_or_die(rpc_or_die(new_rpc(url), "items.getRecent",
                              {"limit": limit, "offset": offset, "type": recent_type}),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "fulltext",
    epilog="Examples:\n\n    zotron items fulltext 12345",
)
def items_fulltext(
    item_id: str = typer.Argument(..., help="Item numeric ID or 8-char key."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Retrieve the full-text content of an item's attachment."""
    emit_or_die(rpc_or_die(new_rpc(url), "items.getFullText", {"key": item_id}),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "add-from-file",
    epilog="Examples:\n\n    zotron items add-from-file paper.pdf\n\n    zotron items add-from-file paper.pdf --collection \"2026-AI\"",
)
def items_add_from_file(
    path: str = typer.Argument(..., help="Local file path to import."),
    collection: str | None = typer.Option(None, "--collection",
        help="Optional collection name or ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add an item from a local file (e.g. a PDF)."""
    rpc = new_rpc(url)
    params: dict = {"path": path}
    if collection is not None:
        coll_id = resolve_or_die(rpc, collection)
        if coll_id and coll_id != 0:
            params["collection"] = coll_id
    if dry_run_flag:
        dry_run("items.addFromFile", params)
    typer.echo(json.dumps(rpc_or_die(rpc, "items.addFromFile", params)))


@items_app.command(
    "related",
    epilog="Examples:\n\n    zotron items related 12345",
)
def items_related(
    item_id: str = typer.Argument(..., help="Item numeric ID or 8-char key."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List items related to the given item."""
    emit_or_die(rpc_or_die(new_rpc(url), "items.getRelated", {"key": item_id}),
                 output=output, jq_filter=jq_filter)


@items_app.command(
    "add-related",
    epilog="Examples:\n\n    zotron items add-related 12345 --target 67890",
)
def items_add_related(
    item_id: str = typer.Argument(..., help="Source item numeric ID or 8-char key."),
    target: str = typer.Option(..., "--target", help="Target item ID or key to relate to."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add a related-item link between two items."""
    params: dict = {"key": item_id, "targetKey": target}
    if dry_run_flag:
        dry_run("items.addRelated", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.addRelated", params)))


@items_app.command(
    "remove-related",
    epilog="Examples:\n\n    zotron items remove-related 12345 --target 67890",
)
def items_remove_related(
    item_id: str = typer.Argument(..., help="Source item numeric ID or 8-char key."),
    target: str = typer.Option(..., "--target", help="Target item ID or key to unlink."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Remove a related-item link between two items."""
    params: dict = {"key": item_id, "targetKey": target}
    if dry_run_flag:
        dry_run("items.removeRelated", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "items.removeRelated", params)))


@items_app.command(
    "citation-key",
    epilog="Examples:\n\n    zotron items citation-key 12345",
)
def items_citation_key(
    item_id: str = typer.Argument(..., help="Item numeric ID or 8-char key."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get the citation key (Better BibTeX or Zotero auto key) for an item."""
    emit_or_die(rpc_or_die(new_rpc(url), "items.citationKey", {"key": item_id}),
                 jq_filter=jq_filter)
