"""CLI: attachments namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import (
    DEFAULT_URL,
    new_rpc,
    rpc_or_die,
    dry_run,
    emit_or_die,
)

attachments_app = typer.Typer(
    help="List, add, delete attachments; get fulltext and file paths.",
    no_args_is_help=True,
)


@attachments_app.command(
    "list",
    epilog="Examples:\n\n    zotron attachments list --parent 42\n\n    zotron attachments list --parent 42 --limit 10 --offset 20",
)
def attachments_list(
    parent: str = typer.Option(..., "--parent", help="Parent item ID."),
    limit: int = typer.Option(50, "--limit", help="Maximum items to return."),
    offset: int = typer.Option(0, "--offset", help="Number of items to skip."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List attachments belonging to a parent item."""
    rpc = new_rpc(url)
    params: dict = {"parentId": parent, "limit": limit, "offset": offset}
    result = rpc_or_die(rpc, "attachments.list", params)
    emit_or_die(result, output=output, jq_filter=jq_filter)


@attachments_app.command(
    "get",
    epilog="Examples:\n\n    zotron attachments get 10",
)
def attachments_get(
    id: str = typer.Argument(..., help="Attachment item ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get a single attachment by ID."""
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "attachments.get", {"id": id})
    emit_or_die(result, output=output, jq_filter=jq_filter)


@attachments_app.command(
    "fulltext",
    epilog="Examples:\n\n    zotron attachments fulltext 10",
)
def attachments_fulltext(
    id: str = typer.Argument(..., help="Attachment item ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get full-text content of an attachment (returns key, content, indexedChars, totalChars)."""
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "attachments.getFullText", {"id": id})
    emit_or_die(result, jq_filter=jq_filter)


@attachments_app.command(
    "add",
    epilog="Examples:\n\n    zotron attachments add --parent 42 --path /path/to/paper.pdf\n\n    zotron attachments add --parent 42 --path paper.pdf --title \"My Paper\"",
)
def attachments_add(
    parent: str = typer.Option(..., "--parent", help="Parent item ID."),
    path: str = typer.Option(..., "--path", help="Path to the file to attach."),
    title: str | None = typer.Option(None, "--title", help="Optional attachment title."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Attach a local file to an item."""
    params: dict = {"parentId": parent, "path": path}
    if title is not None:
        params["title"] = title
    if dry_run_flag:
        dry_run("attachments.add", params)
    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "attachments.add", params), ensure_ascii=False))


@attachments_app.command(
    "add-by-url",
    epilog="Examples:\n\n    zotron attachments add-by-url --parent 42 --url https://example.com/paper.pdf\n\n    zotron attachments add-by-url --parent 42 --url https://example.com/paper.pdf --title \"Remote PDF\"",
)
def attachments_add_by_url(
    parent: str = typer.Option(..., "--parent", help="Parent item ID."),
    url: str = typer.Option(..., "--url", help="URL of the file to attach."),
    title: str | None = typer.Option(None, "--title", help="Optional attachment title."),
    endpoint: str = typer.Option(DEFAULT_URL, "--endpoint", help="zotron RPC endpoint URL."),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Attach a remote file (by URL) to an item."""
    params: dict = {"parentId": parent, "url": url}
    if title is not None:
        params["title"] = title
    if dry_run_flag:
        dry_run("attachments.addByURL", params)
    rpc = new_rpc(endpoint)
    typer.echo(json.dumps(rpc_or_die(rpc, "attachments.addByURL", params), ensure_ascii=False))


@attachments_app.command(
    "path",
    epilog="Examples:\n\n    zotron attachments path 10",
)
def attachments_path(
    id: str = typer.Argument(..., help="Attachment item ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get the local filesystem path of an attachment (returns key, path)."""
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "attachments.getPath", {"id": id})
    emit_or_die(result, jq_filter=jq_filter)


@attachments_app.command(
    "delete",
    epilog="Examples:\n\n    zotron attachments delete 10",
)
def attachments_delete(
    id: str = typer.Argument(..., help="Attachment item ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete an attachment (returns {ok: true, key: ...})."""
    if dry_run_flag:
        dry_run("attachments.delete", {"id": id})
    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "attachments.delete", {"id": id}), ensure_ascii=False))


@attachments_app.command(
    "find-pdf",
    epilog="Examples:\n\n    zotron attachments find-pdf --parent 42",
)
def attachments_find_pdf(
    parent: str = typer.Option(..., "--parent", help="Parent item ID."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Trigger Zotero's Find Available PDF for a parent item (returns {attachment: item | null})."""
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "attachments.findPDF", {"parentId": parent})
    emit_or_die(result, jq_filter=jq_filter)
