"""CLI: tags namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, rpc_or_die, dry_run, emit_or_die

tags_app = typer.Typer(
    help="Inspect / rename / delete tags across the library.",
    no_args_is_help=True,
)


@tags_app.command(
    "list",
    epilog="Examples:\n\n    zotron tags list --limit 100",
)
def tags_list(
    limit: int = typer.Option(200, "--limit"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all tags in the library (flat)."""
    emit_or_die(rpc_or_die(new_rpc(url), "tags.list", {"limit": limit}),
                 output=output, jq_filter=jq_filter)


@tags_app.command(
    "rename",
    epilog='Examples:\n\n    zotron tags rename "todo" "to-read"',
)
def tags_rename(
    old: str = typer.Argument(...),
    new: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Rename a tag across all items."""
    if dry_run_flag:
        dry_run("tags.rename", {"oldName": old, "newName": new})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "tags.rename",
                                      {"oldName": old, "newName": new})))


@tags_app.command(
    "delete",
    epilog='Examples:\n\n    zotron tags delete "outdated-tag"',
)
def tags_delete(
    tag: str = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete a tag library-wide (removes from every item that had it)."""
    if dry_run_flag:
        dry_run("tags.delete", {"tag": tag})
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "tags.delete",
                                      {"tag": tag})))


@tags_app.command(
    "add",
    epilog="Examples:\n\n    zotron tags add 12345 --tag 已读 --tag important",
)
def tags_add(
    id: str = typer.Argument(..., help="Item ID (numeric) or 8-char key."),
    tags: list[str] = typer.Option(..., "--tag", help="Tag to add (repeatable, required)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Add one or more tags to an item."""
    from zotron._cli_base import die
    if not tags:
        die("INVALID_ARGS", "--tag is required", 2)
    params = {"key": id, "tags": list(tags)}
    if dry_run_flag:
        dry_run("tags.add", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "tags.add", params)))


@tags_app.command(
    "remove",
    epilog="Examples:\n\n    zotron tags remove 12345 --tag 已读",
)
def tags_remove(
    id: str = typer.Argument(..., help="Item ID (numeric) or 8-char key."),
    tags: list[str] = typer.Option(..., "--tag", help="Tag to remove (repeatable, required)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Remove one or more tags from an item."""
    from zotron._cli_base import die
    if not tags:
        die("INVALID_ARGS", "--tag is required", 2)
    params = {"key": id, "tags": list(tags)}
    if dry_run_flag:
        dry_run("tags.remove", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "tags.remove", params)))


@tags_app.command(
    "batch-update",
    epilog="Examples:\n\n    zotron tags batch-update 12345 12346 --add 已读 --remove todo",
)
def tags_batch_update(
    ids: list[str] = typer.Argument(..., help="Item IDs (numeric or 8-char key)."),
    add_tags: list[str] = typer.Option(None, "--add", help="Tag to add (repeatable)."),
    remove_tags: list[str] = typer.Option(None, "--remove", help="Tag to remove (repeatable)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Batch add/remove tags across multiple items."""
    from zotron._cli_base import die
    add_list = list(add_tags) if add_tags else []
    remove_list = list(remove_tags) if remove_tags else []
    if not add_list and not remove_list:
        die("INVALID_ARGS", "at least one of --add or --remove is required", 2)
    params: dict = {"keys": list(ids)}
    if add_list:
        params["add"] = add_list
    if remove_list:
        params["remove"] = remove_list
    if dry_run_flag:
        dry_run("tags.batchUpdate", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "tags.batchUpdate", params)))
