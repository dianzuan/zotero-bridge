"""Tests for zotron CLI notes namespace."""
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


# ---------------------------------------------------------------------------
# notes list
# ---------------------------------------------------------------------------

def test_notes_list_returns_envelope(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"key": "KEY0001", "content": "Note A", "version": 1}, {"key": "KEY0002", "content": "Note B", "version": 1}],
        "total": 2, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, ["notes", "list", "--parent", "12345"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["total"] == 2
    assert len(data["items"]) == 2
    mock_rpc.call.assert_called_once_with(
        "notes.list", {"parentKey": "12345", "limit": 50, "offset": 0}
    )


def test_notes_list_with_pagination(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"key": "KEY0005", "content": "Note", "version": 1}],
        "total": 100, "limit": 10, "offset": 20, "hasMore": True,
    }
    result = runner.invoke(
        app, ["notes", "list", "--parent", "abc8", "--limit", "10", "--offset", "20"]
    )
    assert result.exit_code == 0
    mock_rpc.call.assert_called_once_with(
        "notes.list", {"parentKey": "abc8", "limit": 10, "offset": 20}
    )


def test_notes_list_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["notes", "list", "--parent", "99"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


def test_notes_list_rpc_error(mock_rpc):
    mock_rpc.call.side_effect = RuntimeError("[-32601] Method not found")
    result = runner.invoke(app, ["notes", "list", "--parent", "99"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "RPC_ERROR"


# ---------------------------------------------------------------------------
# notes get
# ---------------------------------------------------------------------------

def test_notes_get_returns_note(mock_rpc):
    mock_rpc.call.return_value = {"key": "KEY0042", "content": "<p>My note</p>", "parentKey": "KEY0010", "version": 1}
    result = runner.invoke(app, ["notes", "get", "42"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0042"
    assert data["content"] == "<p>My note</p>"
    mock_rpc.call.assert_called_once_with("notes.get", {"key": "42"})


def test_notes_get_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["notes", "get", "1"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


# ---------------------------------------------------------------------------
# notes create
# ---------------------------------------------------------------------------

def test_notes_create_returns_note(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0055"}
    result = runner.invoke(
        app, ["notes", "create", "--parent", "12345", "--content", "Hello world"]
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0055"
    mock_rpc.call.assert_called_once_with(
        "notes.create", {"parentKey": "12345", "content": "Hello world"}
    )


def test_notes_create_with_tags(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0056"}
    result = runner.invoke(
        app,
        ["notes", "create", "--parent", "99", "--content", "Tagged note",
         "--tag", "research", "--tag", "important"],
    )
    assert result.exit_code == 0
    mock_rpc.call.assert_called_once_with(
        "notes.create",
        {"parentKey": "99", "content": "Tagged note", "tags": ["research", "important"]},
    )


def test_notes_create_dry_run(mock_rpc):
    result = runner.invoke(
        app,
        ["notes", "create", "--parent", "12345", "--content", "Draft note", "--dry-run"],
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "notes.create"
    assert data["wouldCallParams"]["parentKey"] == "12345"
    assert data["wouldCallParams"]["content"] == "Draft note"
    # Must NOT have called the real RPC
    mock_rpc.call.assert_not_called()


def test_notes_create_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(
        app, ["notes", "create", "--parent", "1", "--content", "x"]
    )
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


# ---------------------------------------------------------------------------
# notes update
# ---------------------------------------------------------------------------

def test_notes_update_returns_note(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0042", "content": "Updated content", "version": 1}
    result = runner.invoke(
        app, ["notes", "update", "42", "--content", "Updated content"]
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["content"] == "Updated content"
    mock_rpc.call.assert_called_once_with(
        "notes.update", {"key": "42", "content": "Updated content"}
    )


def test_notes_update_dry_run(mock_rpc):
    result = runner.invoke(
        app, ["notes", "update", "77", "--content", "New text", "--dry-run"]
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "notes.update"
    assert data["wouldCallParams"] == {"key": "77", "content": "New text"}
    mock_rpc.call.assert_not_called()


def test_notes_update_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["notes", "update", "1", "--content", "x"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


# ---------------------------------------------------------------------------
# notes delete
# ---------------------------------------------------------------------------

def test_notes_delete_returns_ok(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0042"}
    result = runner.invoke(app, ["notes", "delete", "42"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "KEY0042"
    mock_rpc.call.assert_called_once_with("items.delete", {"key": "42"})


def test_notes_delete_dry_run(mock_rpc):
    result = runner.invoke(app, ["notes", "delete", "99", "--dry-run"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.delete"
    assert data["wouldCallParams"] == {"key": "99"}
    mock_rpc.call.assert_not_called()


def test_notes_delete_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["notes", "delete", "1"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


# ---------------------------------------------------------------------------
# notes search
# ---------------------------------------------------------------------------

def test_notes_search_returns_envelope(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"key": "KEY0003", "content": "Found note", "version": 1}],
        "total": 1, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, ["notes", "search", "quantum entanglement"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["total"] == 1
    mock_rpc.call.assert_called_once_with(
        "notes.search", {"query": "quantum entanglement", "limit": 50}
    )


def test_notes_search_with_limit(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [], "total": 0, "limit": 10, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, ["notes", "search", "term", "--limit", "10"])
    assert result.exit_code == 0
    mock_rpc.call.assert_called_once_with(
        "notes.search", {"query": "term", "limit": 10}
    )


def test_notes_search_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["notes", "search", "test"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


def test_notes_search_rpc_error(mock_rpc):
    mock_rpc.call.side_effect = RuntimeError("[-32601] Method not found")
    result = runner.invoke(app, ["notes", "search", "test"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "RPC_ERROR"
