"""Tests for zotron CLI tags namespace."""
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
# tags add
# ---------------------------------------------------------------------------

def test_tags_add_single_tag(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0005"}
    result = runner.invoke(app, ["tags", "add", "12345", "--tag", "已读"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "KEY0005"
    call_args = mock_rpc.call.call_args
    assert call_args.args[0] == "tags.add"
    assert call_args.args[1] == {"key": "12345", "tags": ["已读"]}


def test_tags_add_multiple_tags(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0003"}
    result = runner.invoke(app, ["tags", "add", "abcd1234", "--tag", "foo", "--tag", "bar"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["tags"] == ["foo", "bar"]


def test_tags_add_dry_run(mock_rpc):
    result = runner.invoke(app, ["tags", "add", "12345", "--tag", "todo", "--dry-run"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "tags.add"
    assert data["wouldCallParams"] == {"key": "12345", "tags": ["todo"]}
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# tags remove
# ---------------------------------------------------------------------------

def test_tags_remove_single_tag(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0007"}
    result = runner.invoke(app, ["tags", "remove", "12345", "--tag", "已读"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "KEY0007"
    call_args = mock_rpc.call.call_args
    assert call_args.args[0] == "tags.remove"
    assert call_args.args[1] == {"key": "12345", "tags": ["已读"]}


def test_tags_remove_multiple_tags(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0002"}
    result = runner.invoke(app, ["tags", "remove", "99999", "--tag", "alpha", "--tag", "beta"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["tags"] == ["alpha", "beta"]


def test_tags_remove_dry_run(mock_rpc):
    result = runner.invoke(app, ["tags", "remove", "12345", "--tag", "stale", "--dry-run"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "tags.remove"
    assert data["wouldCallParams"] == {"key": "12345", "tags": ["stale"]}
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# tags batch-update
# ---------------------------------------------------------------------------

def test_tags_batch_update_add_and_remove(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "count": 3}
    result = runner.invoke(app, [
        "tags", "batch-update", "111", "222", "333",
        "--add", "已读", "--remove", "todo",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["count"] == 3
    call_args = mock_rpc.call.call_args
    assert call_args.args[0] == "tags.batchUpdate"
    params = call_args.args[1]
    assert params["keys"] == ["111", "222", "333"]
    assert params["add"] == ["已读"]
    assert params["remove"] == ["todo"]


def test_tags_batch_update_add_only(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "count": 2}
    result = runner.invoke(app, [
        "tags", "batch-update", "10", "20",
        "--add", "important",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    call_args = mock_rpc.call.call_args
    params = call_args.args[1]
    assert "add" in params
    assert "remove" not in params


def test_tags_batch_update_remove_only(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "count": 1}
    result = runner.invoke(app, [
        "tags", "batch-update", "42",
        "--remove", "stale",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    call_args = mock_rpc.call.call_args
    params = call_args.args[1]
    assert "remove" in params
    assert "add" not in params


def test_tags_batch_update_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "tags", "batch-update", "1", "2",
        "--add", "read", "--dry-run",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "tags.batchUpdate"
    mock_rpc.call.assert_not_called()


def test_tags_batch_update_requires_add_or_remove(mock_rpc):
    result = runner.invoke(app, ["tags", "batch-update", "12345"])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout
