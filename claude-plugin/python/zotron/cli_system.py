"""CLI: system namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, rpc_or_die, emit_or_die

system_app = typer.Typer(
    help="Plugin version, trigger sync, list libraries.",
    no_args_is_help=True,
)


@system_app.command(
    "version",
    epilog="Examples:\n\n    zotron system version",
)
def system_version(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Show XPI version + the methods it exposes."""
    emit_or_die(rpc_or_die(new_rpc(url), "system.version"), jq_filter=jq_filter)


@system_app.command(
    "sync",
    epilog="Examples:\n\n    zotron system sync",
)
def system_sync(
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Trigger a Zotero sync (cloud <-> local)."""
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "system.sync")))


@system_app.command(
    "libraries",
    epilog="Examples:\n\n    zotron system libraries",
)
def system_libraries(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all libraries (user + groups)."""
    emit_or_die(rpc_or_die(new_rpc(url), "system.libraries"),
                 output=output, jq_filter=jq_filter)


@system_app.command(
    "switch-library",
    epilog="Examples:\n\n    zotron system switch-library 42",
)
def system_switch_library(
    library_id: int = typer.Argument(..., help="Library ID to switch to."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Switch the active library context."""
    typer.echo(json.dumps(
        rpc_or_die(new_rpc(url), "system.switchLibrary", {"id": library_id})
    ))


@system_app.command(
    "library-stats",
    epilog="Examples:\n\n    zotron system library-stats\n\n    zotron system library-stats --library 42",
)
def system_library_stats(
    library: int | None = typer.Option(None, "--library", help="Library ID (optional)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get statistics for the current (or specified) library."""
    params: dict = {}
    if library is not None:
        params["id"] = library
    emit_or_die(
        rpc_or_die(new_rpc(url), "system.libraryStats", params if params else None),
        jq_filter=jq_filter,
    )


@system_app.command(
    "item-types",
    epilog="Examples:\n\n    zotron system item-types",
)
def system_item_types(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all available Zotero item types."""
    emit_or_die(rpc_or_die(new_rpc(url), "system.itemTypes"), jq_filter=jq_filter)


@system_app.command(
    "item-fields",
    epilog="Examples:\n\n    zotron system item-fields --type journalArticle",
)
def system_item_fields(
    item_type: str = typer.Option(..., "--type", help="Item type name (e.g. journalArticle)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all fields for a given item type."""
    emit_or_die(
        rpc_or_die(new_rpc(url), "system.itemFields", {"itemType": item_type}),
        jq_filter=jq_filter,
    )


@system_app.command(
    "creator-types",
    epilog="Examples:\n\n    zotron system creator-types --type journalArticle",
)
def system_creator_types(
    item_type: str = typer.Option(..., "--type", help="Item type name (e.g. journalArticle)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List creator types for a given item type."""
    emit_or_die(
        rpc_or_die(new_rpc(url), "system.creatorTypes", {"itemType": item_type}),
        jq_filter=jq_filter,
    )


@system_app.command(
    "current-collection",
    epilog="Examples:\n\n    zotron system current-collection",
)
def system_current_collection(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get the currently selected Zotero collection (or null)."""
    emit_or_die(
        rpc_or_die(new_rpc(url), "system.currentCollection"),
        jq_filter=jq_filter,
    )


@system_app.command(
    "reload",
    epilog="Examples:\n\n    zotron system reload",
)
def system_reload(
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Reload the XPI plugin."""
    typer.echo(json.dumps(rpc_or_die(new_rpc(url), "system.reload")))


@system_app.command(
    "list-methods",
    epilog="Examples:\n\n    zotron system list-methods",
)
def system_list_methods(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all RPC methods exposed by the XPI."""
    emit_or_die(rpc_or_die(new_rpc(url), "system.listMethods"), jq_filter=jq_filter)


@system_app.command(
    "describe",
    epilog="Examples:\n\n    zotron system describe\n\n    zotron system describe items.get",
)
def system_describe(
    method: str | None = typer.Argument(None, help="Method name to describe (optional; omit for all)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Describe one or all RPC methods (schema / signatures)."""
    params = {"method": method} if method is not None else None
    emit_or_die(
        rpc_or_die(new_rpc(url), "system.describe", params),
        jq_filter=jq_filter,
    )
