"""CLI: export namespace."""
from __future__ import annotations

import json

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, rpc_or_die

export_app = typer.Typer(
    help="Export items as bibtex / ris / csl-json / csv / formatted bibliography.",
    no_args_is_help=True,
)


def _export_fmt(method: str, ids: list[str], url: str) -> None:
    rpc = new_rpc(url)
    resp = rpc_or_die(rpc, method, {"keys": ids})
    if isinstance(resp, dict) and "content" in resp:
        typer.echo(resp["content"])
    else:
        typer.echo(json.dumps(resp))


@export_app.command(
    "bibtex",
    epilog="Examples:\n\n    zotron export bibtex 12345 12346",
)
def export_bibtex(
    ids: list[str] = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print BibTeX for the given item ids."""
    _export_fmt("export.bibtex", ids, url)


@export_app.command(
    "ris",
    epilog="Examples:\n\n    zotron export ris 12345",
)
def export_ris(
    ids: list[str] = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print RIS for the given item ids."""
    _export_fmt("export.ris", ids, url)


@export_app.command(
    "csl-json",
    epilog="Examples:\n\n    zotron export csl-json 12345",
)
def export_csl_json(
    ids: list[str] = typer.Argument(...),
    url: str = typer.Option(DEFAULT_URL, "--url"),
) -> None:
    """Print CSL-JSON for the given item ids."""
    _export_fmt("export.cslJson", ids, url)


@export_app.command(
    "bibliography",
    epilog='Examples:\n\n    zotron export bibliography 12345 --style apa\n\n    zotron export bibliography 12345 --html',
)
def export_bibliography(
    ids: list[str] = typer.Argument(...),
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
    rpc = new_rpc(url)
    resp = rpc_or_die(rpc, "export.bibliography", {"keys": ids, "style": style})
    if isinstance(resp, dict) and ("html" in resp or "text" in resp):
        typer.echo(resp["html"] if html else resp.get("text", ""))
    else:
        typer.echo(json.dumps(resp))
