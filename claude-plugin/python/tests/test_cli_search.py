"""Tests for zotron CLI search namespace — new subcommands."""
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
# search advanced
# ---------------------------------------------------------------------------

def test_search_advanced_single_condition(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"id": 1, "title": "张三的论文"}],
        "total": 1, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, [
        "search", "advanced",
        "--condition", "creator contains 张三",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["total"] == 1
    # Verify RPC call params
    mock_rpc.call.assert_called_once_with(
        "search.advanced",
        {
            "conditions": [{"field": "creator", "operator": "contains", "value": "张三"}],
            "operator": "and",
            "limit": 50,
            "offset": 0,
        },
    )


def test_search_advanced_condition_parsing_multi_word_value(mock_rpc):
    """Value with spaces (after first two tokens) is preserved intact."""
    mock_rpc.call.return_value = {
        "items": [], "total": 0, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, [
        "search", "advanced",
        "--condition", "title contains attention is all you need",
    ])
    assert result.exit_code == 0, result.stdout
    params = mock_rpc.call.call_args.args[1]
    cond = params["conditions"][0]
    assert cond["field"] == "title"
    assert cond["operator"] == "contains"
    assert cond["value"] == "attention is all you need"


def test_search_advanced_multiple_conditions_or(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [], "total": 0, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, [
        "search", "advanced",
        "--condition", "creator contains 张三",
        "--condition", "date isAfter 2020",
        "--operator", "or",
        "--limit", "20",
        "--offset", "5",
    ])
    assert result.exit_code == 0, result.stdout
    params = mock_rpc.call.call_args.args[1]
    assert params["operator"] == "or"
    assert params["limit"] == 20
    assert params["offset"] == 5
    assert len(params["conditions"]) == 2
    assert params["conditions"][1] == {"field": "date", "operator": "isAfter", "value": "2020"}


def test_search_advanced_invalid_operator(mock_rpc):
    result = runner.invoke(app, [
        "search", "advanced",
        "--condition", "creator contains 张三",
        "--operator", "not",
    ])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout


def test_search_advanced_bad_condition_too_few_tokens(mock_rpc):
    result = runner.invoke(app, [
        "search", "advanced",
        "--condition", "creator",
    ])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout


# ---------------------------------------------------------------------------
# search by-tag
# ---------------------------------------------------------------------------

def test_search_by_tag_basic(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"id": 5, "title": "A tagged item"}],
        "total": 1, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, ["search", "by-tag", "乡村振兴"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["total"] == 1
    mock_rpc.call.assert_called_once_with(
        "search.byTag",
        {"tag": "乡村振兴", "limit": 50, "offset": 0},
    )


def test_search_by_tag_with_limit_offset(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [], "total": 0, "limit": 10, "offset": 20, "hasMore": False,
    }
    result = runner.invoke(app, [
        "search", "by-tag", "AI", "--limit", "10", "--offset", "20",
    ])
    assert result.exit_code == 0, result.stdout
    params = mock_rpc.call.call_args.args[1]
    assert params["limit"] == 10
    assert params["offset"] == 20


# ---------------------------------------------------------------------------
# search saved-searches
# ---------------------------------------------------------------------------

def test_search_saved_searches(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"id": "abc", "name": "My Search"}],
        "total": 1, "limit": 50, "offset": 0, "hasMore": False,
    }
    result = runner.invoke(app, ["search", "saved-searches"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["total"] == 1
    mock_rpc.call.assert_called_once_with("search.savedSearches")


# ---------------------------------------------------------------------------
# search create-saved
# ---------------------------------------------------------------------------

def test_search_create_saved_basic(mock_rpc):
    mock_rpc.call.return_value = {"id": 42, "key": "ABCD1234", "name": "张三论文"}
    result = runner.invoke(app, [
        "search", "create-saved", "张三论文",
        "--condition", "creator contains 张三",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["name"] == "张三论文"
    assert data["id"] == 42
    mock_rpc.call.assert_called_once_with(
        "search.createSavedSearch",
        {
            "name": "张三论文",
            "conditions": [{"field": "creator", "operator": "contains", "value": "张三"}],
        },
    )


def test_search_create_saved_multiple_conditions(mock_rpc):
    mock_rpc.call.return_value = {"id": 7, "key": "K7", "name": "Multi"}
    result = runner.invoke(app, [
        "search", "create-saved", "Multi",
        "--condition", "tag contains AI",
        "--condition", "date isAfter 2022",
    ])
    assert result.exit_code == 0, result.stdout
    params = mock_rpc.call.call_args.args[1]
    assert len(params["conditions"]) == 2
    assert params["conditions"][0] == {"field": "tag", "operator": "contains", "value": "AI"}
    assert params["conditions"][1] == {"field": "date", "operator": "isAfter", "value": "2022"}


def test_search_create_saved_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "search", "create-saved", "DryTest",
        "--condition", "creator contains 李四",
        "--dry-run",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "search.createSavedSearch"
    assert data["wouldCallParams"]["name"] == "DryTest"
    assert data["wouldCallParams"]["conditions"] == [
        {"field": "creator", "operator": "contains", "value": "李四"}
    ]
    # Confirm no RPC call was made
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# search delete-saved
# ---------------------------------------------------------------------------

def test_search_delete_saved_basic(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "id": "abc123"}
    result = runner.invoke(app, ["search", "delete-saved", "abc123"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    mock_rpc.call.assert_called_once_with(
        "search.deleteSavedSearch",
        {"id": "abc123"},
    )


def test_search_delete_saved_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "search", "delete-saved", "xyz789",
        "--dry-run",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "search.deleteSavedSearch"
    assert data["wouldCallParams"] == {"id": "xyz789"}
    mock_rpc.call.assert_not_called()
