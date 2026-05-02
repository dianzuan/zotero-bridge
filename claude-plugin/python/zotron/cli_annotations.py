"""CLI: annotations namespace."""
from __future__ import annotations

import json
from typing import Optional

import typer

from zotron._cli_base import DEFAULT_URL, new_rpc, rpc_or_die, dry_run, emit_or_die

annotations_app = typer.Typer(
    help="List, create, delete PDF annotations.",
    no_args_is_help=True,
)


@annotations_app.command(
    "list",
    epilog="Examples:\n\n    zotron annotations list --parent 12345",
)
def annotations_list(
    parent: str = typer.Option(..., "--parent", help="Parent item ID (attachment)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    output: str = typer.Option(
        "json", "--output", "-o", help="Output format: json (default) or table."
    ),
    jq_filter: Optional[str] = typer.Option(None, "--jq", help="jq filter expression"),
) -> None:
    """List annotations on a PDF attachment."""
    rpc = new_rpc(url)
    data = rpc_or_die(rpc, "annotations.list", {"parentId": parent})
    emit_or_die(data, output=output, jq_filter=jq_filter)


@annotations_app.command(
    "create",
    epilog="Examples:\n\n    zotron annotations create --parent 12345 --type highlight\n\n    zotron annotations create --parent 12345 --type note --text \"key finding\" --comment \"revisit this\"",
)
def annotations_create(
    parent: str = typer.Option(..., "--parent", help="Parent attachment item ID."),
    annotation_type: str = typer.Option(
        ...,
        "--type",
        help="Annotation type: highlight | note | underline",
    ),
    text: Optional[str] = typer.Option(None, "--text", help="Selected text to annotate."),
    comment: Optional[str] = typer.Option(None, "--comment", help="Annotation comment."),
    color: str = typer.Option("#ffd400", "--color", help="Highlight color (hex, default #ffd400)."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(
        False, "--dry-run", help="Print intended RPC call as JSON; do not execute."
    ),
) -> None:
    """Create a new annotation on a PDF attachment."""
    if annotation_type not in ("highlight", "note", "underline"):
        from zotron._cli_base import die
        die("INVALID_ARGS", f"--type must be highlight|note|underline, got {annotation_type!r}", 2)

    params: dict = {"parentId": parent, "type": annotation_type, "color": color}
    if text is not None:
        params["text"] = text
    if comment is not None:
        params["comment"] = comment

    if dry_run_flag:
        dry_run("annotations.create", params)

    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "annotations.create", params), ensure_ascii=False))


@annotations_app.command(
    "delete",
    epilog="Examples:\n\n    zotron annotations delete 55",
)
def annotations_delete(
    annotation_id: str = typer.Argument(..., help="Annotation item ID to delete."),
    url: str = typer.Option(DEFAULT_URL, "--url"),
    dry_run_flag: bool = typer.Option(
        False, "--dry-run", help="Print intended RPC call as JSON; do not execute."
    ),
) -> None:
    """Delete an annotation by ID."""
    params = {"id": annotation_id}

    if dry_run_flag:
        dry_run("annotations.delete", params)

    rpc = new_rpc(url)
    typer.echo(json.dumps(rpc_or_die(rpc, "annotations.delete", params), ensure_ascii=False))
