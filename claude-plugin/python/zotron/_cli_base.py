"""Shared CLI infrastructure: app instance, helpers, constants.

Every namespace module (cli_items.py, cli_notes.py, etc.) imports from here.
"""
from __future__ import annotations

import json
from typing import NoReturn

import typer

from zotron._output import emit
from zotron.errors import CollectionAmbiguous, CollectionNotFound
from zotron.push import resolve_collection
from zotron.rpc import ZoteroRPC

DEFAULT_URL = "http://127.0.0.1:23119/zotron/rpc"

app = typer.Typer(
    name="zotron",
    help="Python client + CLI for the Zotron XPI.\n\n"
         "For fetching CNKI papers end-to-end, use `cnki export`.",
    no_args_is_help=True,
    context_settings={"help_option_names": ["-h", "--help"]},
)


def new_rpc(url: str) -> ZoteroRPC:
    return ZoteroRPC(url)


def die(code: str, message: str, exit_code: int = 1) -> NoReturn:
    typer.echo(json.dumps({"ok": False, "error": {"code": code, "message": message}}, ensure_ascii=False))
    raise typer.Exit(code=exit_code)


def rpc_or_die(rpc: ZoteroRPC, method: str, params: dict | None = None):
    try:
        resp = rpc.call(method, params) if params is not None else rpc.call(method)
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
    except RuntimeError as e:
        die("RPC_ERROR", str(e))
    return resp if resp is not None else {}


def dry_run(method: str, params: dict | None = None) -> None:
    typer.echo(json.dumps({
        "ok": True,
        "dryRun": True,
        "wouldCall": method,
        "wouldCallParams": params or {},
    }, ensure_ascii=False))
    raise typer.Exit(code=0)


def emit_or_die(data, *, output: str = "json", jq_filter: str | None = None) -> None:
    try:
        emit(data, output=output, jq_filter=jq_filter)
    except ValueError as e:
        die("INVALID_JQ", str(e))


def resolve_or_die(rpc: ZoteroRPC, name_or_id: str) -> int:
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
        die("COLLECTION_NOT_FOUND", str(e))
    except ConnectionError:
        die("ZOTERO_UNAVAILABLE", "Cannot connect to zotron")
