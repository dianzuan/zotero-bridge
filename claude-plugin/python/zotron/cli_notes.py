"""CLI: notes namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, rpc_or_die, dry_run, emit_or_die

notes_app = typer.Typer(
    help="List, create, update, delete notes on items.",
    no_args_is_help=True,
)


@notes_app.command(
    "list",
    epilog="Examples:\n\n    zotron notes list --parent 12345\n\n    zotron notes list --parent AB12CD34 --limit 20 --offset 40",
)
def notes_list(
    parent: str = typer.Option(..., "--parent", help="Parent item ID (numeric or 8-char key)."),
    limit: int = typer.Option(50, "--limit", help="Maximum number of notes to return."),
    offset: int = typer.Option(0, "--offset", help="Pagination offset."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List notes attached to a parent item."""
    rpc = new_rpc(url)
    data = rpc_or_die(rpc, "notes.list", {"parentId": parent, "limit": limit, "offset": offset})
    emit_or_die(data, output=output, jq_filter=jq_filter)


@notes_app.command(
    "get",
    epilog="Examples:\n\n    zotron notes get 42\n\n    zotron notes get AB12CD34",
)
def notes_get(
    note_id: str = typer.Argument(..., help="Note ID (numeric or 8-char key)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get a single note by ID."""
    rpc = new_rpc(url)
    data = rpc_or_die(rpc, "notes.get", {"id": note_id})
    emit_or_die(data, output=output, jq_filter=jq_filter)


@notes_app.command(
    "create",
    epilog='Examples:\n\n    zotron notes create --parent 12345 --content "My annotation"\n\n    zotron notes create --parent AB12CD34 --content "See also §3" --tag research --tag todo',
)
def notes_create(
    parent: str = typer.Option(..., "--parent", help="Parent item ID (numeric or 8-char key)."),
    content: str = typer.Option(..., "--content", help="Note HTML content."),
    tags: list[str] = typer.Option(None, "--tag", help="Tag to attach (repeatable)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Create a note attached to a parent item."""
    params: dict = {"parentId": parent, "content": content}
    if tags:
        params["tags"] = list(tags)
    if dry_run_flag:
        dry_run("notes.create", params)
    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "notes.create", params), ensure_ascii=False))


@notes_app.command(
    "update",
    epilog='Examples:\n\n    zotron notes update 42 --content "Revised annotation"',
)
def notes_update(
    note_id: str = typer.Argument(..., help="Note ID (numeric or 8-char key)."),
    content: str = typer.Option(..., "--content", help="Replacement note HTML content."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Update the content of an existing note."""
    params: dict = {"id": note_id, "content": content}
    if dry_run_flag:
        dry_run("notes.update", params)
    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "notes.update", params), ensure_ascii=False))


@notes_app.command(
    "delete",
    epilog="Examples:\n\n    zotron notes delete 42\n\n    zotron notes delete AB12CD34 --dry-run",
)
def notes_delete(
    note_id: str = typer.Argument(..., help="Note ID (numeric or 8-char key)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete a note by ID."""
    params: dict = {"id": note_id}
    if dry_run_flag:
        dry_run("notes.delete", params)
    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "notes.delete", params), ensure_ascii=False))


@notes_app.command(
    "search",
    epilog='Examples:\n\n    zotron notes search "quantum entanglement"\n\n    zotron notes search "chapter 3" --limit 20',
)
def notes_search(
    query: str = typer.Argument(..., help="Search query string."),
    limit: int = typer.Option(50, "--limit", help="Maximum number of results."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Search notes by text content."""
    rpc = new_rpc(url)
    data = rpc_or_die(rpc, "notes.search", {"query": query, "limit": limit})
    emit_or_die(data, output=output, jq_filter=jq_filter)
