"""Tests for zotron CLI system namespace (new subcommands)."""
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


def test_system_switch_library(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "id": 42}
    result = runner.invoke(app, ["system", "switch-library", "42"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["id"] == 42
    mock_rpc.call.assert_called_once_with("system.switchLibrary", {"id": 42})


def test_system_library_stats_no_library(mock_rpc):
    mock_rpc.call.return_value = {"items": 1234, "collections": 42}
    result = runner.invoke(app, ["system", "library-stats"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["items"] == 1234


def test_system_library_stats_with_library(mock_rpc):
    mock_rpc.call.return_value = {"items": 99, "collections": 5}
    result = runner.invoke(app, ["system", "library-stats", "--library", "7"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["items"] == 99
    mock_rpc.call.assert_called_once_with("system.libraryStats", {"id": 7})


def test_system_item_types(mock_rpc):
    mock_rpc.call.return_value = [
        {"itemType": "journalArticle", "localized": "Journal Article"},
        {"itemType": "book", "localized": "Book"},
    ]
    result = runner.invoke(app, ["system", "item-types"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert len(data) == 2
    assert data[0]["itemType"] == "journalArticle"


def test_system_item_fields(mock_rpc):
    mock_rpc.call.return_value = [
        {"field": "title", "localized": "Title"},
        {"field": "DOI", "localized": "DOI"},
    ]
    result = runner.invoke(app, ["system", "item-fields", "--type", "journalArticle"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert any(f["field"] == "title" for f in data)
    mock_rpc.call.assert_called_once_with(
        "system.itemFields", {"itemType": "journalArticle"}
    )


def test_system_creator_types(mock_rpc):
    mock_rpc.call.return_value = [
        {"creatorType": "author", "localized": "Author"},
        {"creatorType": "editor", "localized": "Editor"},
    ]
    result = runner.invoke(app, ["system", "creator-types", "--type", "book"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data[0]["creatorType"] == "author"
    mock_rpc.call.assert_called_once_with("system.creatorTypes", {"itemType": "book"})


def test_system_current_collection_returns_object(mock_rpc):
    mock_rpc.call.return_value = {"key": "COL5", "name": "AI Papers", "libraryId": 1}
    result = runner.invoke(app, ["system", "current-collection"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["name"] == "AI Papers"
    assert data["key"] == "COL5"


def test_system_current_collection_returns_null(mock_rpc):
    mock_rpc.call.return_value = None
    result = runner.invoke(app, ["system", "current-collection"])
    assert result.exit_code == 0, result.stdout
    # _rpc_or_die converts None → {}; emit_or_die outputs {}
    data = json.loads(result.stdout)
    assert data == {}


def test_system_reload(mock_rpc):
    mock_rpc.call.return_value = {"ok": True}
    result = runner.invoke(app, ["system", "reload"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    mock_rpc.call.assert_called_once_with("system.reload")


def test_system_list_methods(mock_rpc):
    mock_rpc.call.return_value = ["system.ping", "system.version", "items.get"]
    result = runner.invoke(app, ["system", "list-methods"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert "system.ping" in data


def test_system_describe_no_method(mock_rpc):
    """Without argument → call system.describe with no params → returns all schemas."""
    mock_rpc.call.return_value = {"methods": {"system.ping": {"params": []}}}
    result = runner.invoke(app, ["system", "describe"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert "methods" in data
    mock_rpc.call.assert_called_once_with("system.describe")


def test_system_describe_with_method(mock_rpc):
    """With method argument → call system.describe with {method: ...}."""
    mock_rpc.call.return_value = {"method": "items.get", "params": [{"name": "id"}]}
    result = runner.invoke(app, ["system", "describe", "items.get"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["method"] == "items.get"
    mock_rpc.call.assert_called_once_with("system.describe", {"method": "items.get"})
