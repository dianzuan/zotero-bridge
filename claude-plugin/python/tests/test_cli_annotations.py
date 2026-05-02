"""Tests for zotron CLI annotations namespace."""
import json
from unittest.mock import patch, MagicMock

import pytest
from typer.testing import CliRunner

from zotron.cli import app

runner = CliRunner()


@pytest.fixture
def mock_rpc():
    with patch("zotron._cli_base.ZoteroRPC") as mock_cls:
        instance = MagicMock()
        mock_cls.return_value = instance
        yield instance


def test_annotations_list(mock_rpc):
    """annotations list --parent <id> returns annotation array."""
    mock_rpc.call.return_value = [
        {"id": 1, "type": "highlight", "text": "important point"},
        {"id": 2, "type": "note", "comment": "my note"},
    ]
    result = runner.invoke(app, ["annotations", "list", "--parent", "42"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert len(data) == 2
    assert data[0]["type"] == "highlight"
    mock_rpc.call.assert_called_once_with("annotations.list", {"parentId": "42"})


def test_annotations_create(mock_rpc):
    """annotations create --parent <id> --type highlight returns {id, key}."""
    mock_rpc.call.return_value = {"id": 99, "key": "ABCDEF12"}
    result = runner.invoke(
        app,
        ["annotations", "create", "--parent", "42", "--type", "highlight"],
    )
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["id"] == 99
    assert data["key"] == "ABCDEF12"
    call_args = mock_rpc.call.call_args
    assert call_args.args[0] == "annotations.create"
    params = call_args.args[1]
    assert params["parentId"] == "42"
    assert params["type"] == "highlight"
    assert params["color"] == "#ffd400"


def test_annotations_create_with_optional_fields(mock_rpc):
    """create with --text, --comment, --color passes all fields."""
    mock_rpc.call.return_value = {"id": 100, "key": "XYZ"}
    result = runner.invoke(
        app,
        [
            "annotations", "create",
            "--parent", "42",
            "--type", "note",
            "--text", "selected text",
            "--comment", "my comment",
            "--color", "#ff0000",
        ],
    )
    assert result.exit_code == 0, result.stdout
    params = mock_rpc.call.call_args.args[1]
    assert params["text"] == "selected text"
    assert params["comment"] == "my comment"
    assert params["color"] == "#ff0000"


def test_annotations_create_dry_run(mock_rpc):
    """--dry-run emits envelope and does not call RPC."""
    result = runner.invoke(
        app,
        [
            "annotations", "create",
            "--parent", "42",
            "--type", "underline",
            "--dry-run",
        ],
    )
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "annotations.create"
    assert data["wouldCallParams"]["parentId"] == "42"
    assert data["wouldCallParams"]["type"] == "underline"
    mock_rpc.call.assert_not_called()


def test_annotations_delete(mock_rpc):
    """annotations delete <id> calls annotations.delete and returns {ok, id}."""
    mock_rpc.call.return_value = {"ok": True, "id": 55}
    result = runner.invoke(app, ["annotations", "delete", "55"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["id"] == 55
    mock_rpc.call.assert_called_once_with("annotations.delete", {"id": "55"})


def test_annotations_delete_dry_run(mock_rpc):
    """annotations delete --dry-run emits envelope without calling RPC."""
    result = runner.invoke(app, ["annotations", "delete", "55", "--dry-run"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "annotations.delete"
    assert data["wouldCallParams"]["id"] == "55"
    mock_rpc.call.assert_not_called()


def test_annotations_list_connection_error(mock_rpc):
    """Connection failure surfaces as ZOTERO_UNAVAILABLE envelope."""
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["annotations", "list", "--parent", "42"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["ok"] is False
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"
