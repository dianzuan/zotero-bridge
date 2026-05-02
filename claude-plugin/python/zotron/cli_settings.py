"""CLI: settings namespace."""
from __future__ import annotations

import json
from pathlib import Path

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, rpc_or_die, dry_run, emit_or_die, die

settings_app = typer.Typer(help="Get and set Zotero preferences.", no_args_is_help=True)


@settings_app.command(
    "get",
    epilog="Examples:\n\n    zotron settings get extensions.zotero.openURL.resolver",
)
def settings_get(
    key: str = typer.Argument(..., help="Preference key."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """Get a single Zotero preference value."""
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "settings.get", {"key": key})
    emit_or_die(result, jq_filter=jq_filter)


@settings_app.command(
    "set",
    epilog="Examples:\n\n    zotron settings set extensions.zotero.openURL.resolver https://libgen.rs\n\n    zotron settings set extensions.zotero.debug.log false",
)
def settings_set(
    key: str = typer.Argument(..., help="Preference key."),
    value: str = typer.Argument(..., help="Value (JSON-decoded if valid JSON, else plain string)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Set a single Zotero preference. Value is JSON-parsed when valid."""
    try:
        parsed_value = json.loads(value)
    except json.JSONDecodeError:
        parsed_value = value
    params = {"key": key, "value": parsed_value}
    if dry_run_flag:
        dry_run("settings.set", params)
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "settings.set", params)
    typer.echo(json.dumps(result, ensure_ascii=False))


@settings_app.command(
    "list",
    epilog="Examples:\n\n    zotron settings list",
)
def settings_list(
    url: str = typer.Option(DEFAULT_URL, "--url"),
    jq_filter: str | None = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List all Zotero preferences as a key->value dict."""
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "settings.getAll")
    emit_or_die(result, jq_filter=jq_filter)


@settings_app.command(
    "set-all",
    epilog="Examples:\n\n    zotron settings set-all --file prefs.json",
)
def settings_set_all(
    file: Path = typer.Option(..., "--file", help="Path to a JSON file of key->value pairs."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(False, "--dry-run",
        help="Print intended RPC call as JSON; do not execute."),
) -> None:
    """Bulk-set Zotero preferences from a JSON file."""
    try:
        raw = file.read_text(encoding="utf-8")
        settings_dict = json.loads(raw)
    except json.JSONDecodeError as e:
        die("INVALID_JSON", f"Could not parse JSON: {e}", 2)
    params = {"settings": settings_dict}
    if dry_run_flag:
        dry_run("settings.setAll", params)
    rpc = new_rpc(url)
    result = rpc_or_die(rpc, "settings.setAll", params)
    typer.echo(json.dumps(result, ensure_ascii=False))
