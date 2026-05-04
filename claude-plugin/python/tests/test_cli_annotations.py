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
        {"key": "KEY0001", "type": "highlight", "text": "important point", "version": 1},
        {"key": "KEY0002", "type": "note", "comment": "my note", "version": 1},
    ]
    result = runner.invoke(app, ["annotations", "list", "--parent", "42"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert len(data) == 2
    assert data[0]["type"] == "highlight"
    mock_rpc.call.assert_called_once_with("annotations.list", {"parentKey": "42"})


def test_annotations_create(mock_rpc):
    """annotations create --parent <id> --type highlight returns {id, key}."""
    mock_rpc.call.return_value = {"ok": True, "key": "ABCDEF12", "version": 1}
    result = runner.invoke(
        app,
        ["annotations", "create", "--parent", "42", "--type", "highlight"],
    )
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "ABCDEF12"
    call_args = mock_rpc.call.call_args
    assert call_args.args[0] == "annotations.create"
    params = call_args.args[1]
    assert params["parentKey"] == "42"
    assert params["type"] == "highlight"
    assert params["color"] == "#ffd400"


def test_annotations_create_with_optional_fields(mock_rpc):
    """create with --text, --comment, --color passes all fields."""
    mock_rpc.call.return_value = {"ok": True, "key": "XYZ", "version": 1}
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
    assert data["wouldCallParams"]["parentKey"] == "42"
    assert data["wouldCallParams"]["type"] == "underline"
    mock_rpc.call.assert_not_called()


def test_annotations_delete(mock_rpc):
    """annotations delete <id> calls annotations.delete and returns {ok, id}."""
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0055"}
    result = runner.invoke(app, ["annotations", "delete", "55"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "KEY0055"
    mock_rpc.call.assert_called_once_with("annotations.delete", {"key": "55"})


def test_annotations_delete_dry_run(mock_rpc):
    """annotations delete --dry-run emits envelope without calling RPC."""
    result = runner.invoke(app, ["annotations", "delete", "55", "--dry-run"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "annotations.delete"
    assert data["wouldCallParams"]["key"] == "55"
    mock_rpc.call.assert_not_called()


def test_annotations_list_connection_error(mock_rpc):
    """Connection failure surfaces as ZOTERO_UNAVAILABLE envelope."""
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["annotations", "list", "--parent", "42"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["ok"] is False
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"
