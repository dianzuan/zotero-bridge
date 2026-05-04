"""CLI: search namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, die, rpc_or_die, emit_or_die, dry_run

search_app = typer.Typer(
    help="Search items by text / tag / identifier.",
    no_args_is_help=True,
)


@search_app.command(
    "quick",
    epilog='Examples:\n\n    zotron search quick "transformer" --limit 10',
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
    emit_or_die(rpc_or_die(new_rpc(url), "search.quick",
                             {"query": query, "limit": limit}),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "fulltext",
    epilog='Examples:\n\n    zotron search fulltext "attention is all you need"',
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
    emit_or_die(rpc_or_die(new_rpc(url), "search.fulltext",
                             {"query": query, "limit": limit}),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "by-identifier",
    epilog="Examples:\n\n    zotron search by-identifier --doi 10.1038/nature12373\n\n    zotron search by-identifier --isbn 9780262035613",
)
def search_by_identifier(
    doi: str | None = typer.Option(None, "--doi"),
    isbn: str | None = typer.Option(None, "--isbn"),
    issn: str | None = typer.Option(None, "--issn"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Find an item by DOI / ISBN / ISSN."""
    params = {k: v for k, v in
              {"doi": doi, "isbn": isbn, "issn": issn}.items()
              if v}
    if not params:
        die("INVALID_ARGS", "give at least one of --doi/--isbn/--issn", 2)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url),
                                      "search.byIdentifier", params)))


def _parse_condition(raw: str) -> dict:
    """Parse 'field operator value' string into {field, operator, value}.

    Splits on the first two whitespace tokens; the remainder is the value
    (allowing spaces in values like multi-word names).
    """
    parts = raw.split(None, 2)
    if len(parts) < 3:
        die("INVALID_ARGS",
             f"--condition must be 'field operator value', got: {raw!r}", 2)
    return {"field": parts[0], "operator": parts[1], "value": parts[2]}


@search_app.command(
    "advanced",
    epilog=(
        "Examples:\n\n"
        '    zotron search advanced --condition "creator contains 张三"\n\n'
        '    zotron search advanced --condition "creator contains 张三" '
        '--condition "date isAfter 2020" --operator and'
    ),
)
def search_advanced(
    condition: list[str] = typer.Option(
        ..., "--condition",
        help="Condition string 'field operator value'. Repeatable.",
    ),
    operator: str = typer.Option(
        "and", "--operator",
        help="Logical operator joining conditions: and | or.",
    ),
    limit: int = typer.Option(50, "--limit"),
    offset: int = typer.Option(0, "--offset"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Advanced search using structured field/operator/value conditions."""
    if operator not in ("and", "or"):
        die("INVALID_ARGS", f"--operator must be 'and' or 'or', got {operator!r}", 2)
    conditions = [_parse_condition(c) for c in condition]
    params: dict = {
        "conditions": conditions,
        "operator": operator,
        "limit": limit,
        "offset": offset,
    }
    emit_or_die(rpc_or_die(new_rpc(url), "search.advanced", params),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "by-tag",
    epilog=(
        "Examples:\n\n"
        '    zotron search by-tag "乡村振兴"\n\n'
        '    zotron search by-tag "AI" --limit 20 --offset 0'
    ),
)
def search_by_tag(
    tag: str = typer.Argument(..., help="Tag to search for."),
    limit: int = typer.Option(50, "--limit"),
    offset: int = typer.Option(0, "--offset"),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List items that have the given tag."""
    params: dict = {"tag": tag, "limit": limit, "offset": offset}
    emit_or_die(rpc_or_die(new_rpc(url), "search.byTag", params),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "saved-searches",
    epilog="Examples:\n\n    zotron search saved-searches",
)
def search_saved_searches(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option("json", "--output", "-o",
        help="Output format: json (default) or table."),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all saved searches in the library."""
    emit_or_die(rpc_or_die(new_rpc(url), "search.savedSearches"),
                 output=output, jq_filter=jq_filter)


@search_app.command(
    "create-saved",
    epilog=(
        "Examples:\n\n"
        '    zotron search create-saved "张三论文" --condition "creator contains 张三"\n\n'
        '    zotron search create-saved "Recent AI" '
        '--condition "tag contains AI" --condition "date isAfter 2022"'
    ),
)
def search_create_saved(
    name: str = typer.Argument(..., help="Name of the saved search."),
    condition: list[str] = typer.Option(
        ..., "--condition",
        help="Condition string 'field operator value'. Repeatable.",
    ),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Create a saved search with one or more conditions."""
    conditions = [_parse_condition(c) for c in condition]
    params: dict = {"name": name, "conditions": conditions}
    if dry_run_flag:
        dry_run("search.createSavedSearch", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url),
                                      "search.createSavedSearch", params),
                          ensure_ascii=False))


@search_app.command(
    "delete-saved",
    epilog="Examples:\n\n    zotron search delete-saved abc123",
)
def search_delete_saved(
    search_id: str = typer.Argument(..., help="ID of the saved search to delete."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Delete a saved search by ID."""
    params: dict = {"key": search_id}
    if dry_run_flag:
        dry_run("search.deleteSavedSearch", params)
    typer.echo(json.dumps(rpc_or_die(new_rpc(url),
                                      "search.deleteSavedSearch", params)))
