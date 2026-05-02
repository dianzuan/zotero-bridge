"""Tests for zotron CLI items namespace — new subcommands."""
import json
from unittest.mock import patch, MagicMock

import pytest
from typer.testing import CliRunner

from zotron.cli import app

runner = CliRunner()


@pytest.fixture
def mock_rpc():
    """Patch ZoteroRPC so no real HTTP is made. Returns the mock instance."""
    with patch("zotron._cli_base.ZoteroRPC") as mock_cls:
        instance = MagicMock()
        mock_cls.return_value = instance
        yield instance


# ---------------------------------------------------------------------------
# items list
# ---------------------------------------------------------------------------

def test_items_list_ok(mock_rpc):
    mock_rpc.call.return_value = {"items": [{"key": "KEY0001", "title": "Paper A", "version": 1}], "total": 1}
    result = runner.invoke(app, ["items", "list"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["total"] == 1
    assert data["items"][0]["key"] == "KEY0001"


def test_items_list_passes_params(mock_rpc):
    mock_rpc.call.return_value = {"items": [], "total": 0}
    runner.invoke(app, ["items", "list", "--limit", "10", "--offset", "5",
                        "--sort", "title", "--direction", "desc"])
    mock_rpc.call.assert_called_once_with(
        "items.list",
        {"limit": 10, "offset": 5, "sort": "title", "direction": "desc"},
    )


def test_items_list_unavailable(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["items", "list"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout


# ---------------------------------------------------------------------------
# items create
# ---------------------------------------------------------------------------

def test_items_create_ok(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "ABCD1234", "version": 1}
    result = runner.invoke(app, [
        "items", "create", "--type", "journalArticle",
        "--field", "title=My Paper", "--field", "year=2026",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["key"] == "ABCD1234"


def test_items_create_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "items", "create", "--type", "journalArticle",
        "--field", "title=Test", "--dry-run",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.create"
    assert data["wouldCallParams"]["itemType"] == "journalArticle"
    assert data["wouldCallParams"]["fields"]["title"] == "Test"
    mock_rpc.call.assert_not_called()


def test_items_create_invalid_field_format(mock_rpc):
    result = runner.invoke(app, [
        "items", "create", "--type", "journalArticle",
        "--field", "badnoequals",
    ])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout


# ---------------------------------------------------------------------------
# items update
# ---------------------------------------------------------------------------

def test_items_update_ok(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0042", "updated": True}
    result = runner.invoke(app, [
        "items", "update", "42", "--field", "title=Updated Title",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["updated"] is True


def test_items_update_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "items", "update", "ABCD1234", "--field", "year=2027", "--dry-run",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.update"
    assert data["wouldCallParams"]["id"] == "ABCD1234"
    assert data["wouldCallParams"]["fields"]["year"] == "2027"
    mock_rpc.call.assert_not_called()


def test_items_update_accepts_string_id(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "ABCD1234", "updated": True}
    result = runner.invoke(app, ["items", "update", "ABCD1234", "--field", "title=X"])
    assert result.exit_code == 0
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["id"] == "ABCD1234"


# ---------------------------------------------------------------------------
# items delete
# ---------------------------------------------------------------------------

def test_items_delete_ok(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "deleted": True, "key": "KEY0042"}
    result = runner.invoke(app, ["items", "delete", "42"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["deleted"] is True


def test_items_delete_dry_run(mock_rpc):
    result = runner.invoke(app, ["items", "delete", "42", "--dry-run"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.delete"
    assert data["wouldCallParams"]["id"] == "42"
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# items list-trash
# ---------------------------------------------------------------------------

def test_items_list_trash_ok(mock_rpc):
    mock_rpc.call.return_value = {"items": [{"key": "KEY0099", "title": "Old Paper", "version": 1}], "total": 1}
    result = runner.invoke(app, ["items", "list-trash"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["total"] == 1


def test_items_list_trash_passes_params(mock_rpc):
    mock_rpc.call.return_value = {"items": [], "total": 0}
    runner.invoke(app, ["items", "list-trash", "--limit", "5", "--offset", "10"])
    mock_rpc.call.assert_called_once_with("items.getTrash", {"limit": 5, "offset": 10})


# ---------------------------------------------------------------------------
# items batch-trash
# ---------------------------------------------------------------------------

def test_items_batch_trash_ok(mock_rpc):
    mock_rpc.call.return_value = {"trashed": 3}
    result = runner.invoke(app, ["items", "batch-trash", "1", "2", "3"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["trashed"] == 3


def test_items_batch_trash_dry_run(mock_rpc):
    result = runner.invoke(app, ["items", "batch-trash", "10", "20", "--dry-run"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.batchTrash"
    assert "10" in data["wouldCallParams"]["ids"]
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# items recent
# ---------------------------------------------------------------------------

def test_items_recent_ok(mock_rpc):
    mock_rpc.call.return_value = {"items": [{"key": "KEY0005", "version": 1}], "total": 1}
    result = runner.invoke(app, ["items", "recent"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["total"] == 1


def test_items_recent_modified_type(mock_rpc):
    mock_rpc.call.return_value = {"items": [], "total": 0}
    runner.invoke(app, ["items", "recent", "--type", "modified", "--limit", "5"])
    mock_rpc.call.assert_called_once_with(
        "items.getRecent",
        {"limit": 5, "offset": 0, "type": "modified"},
    )


def test_items_recent_invalid_type(mock_rpc):
    result = runner.invoke(app, ["items", "recent", "--type", "newest"])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout


# ---------------------------------------------------------------------------
# items fulltext
# ---------------------------------------------------------------------------

def test_items_fulltext_ok(mock_rpc):
    mock_rpc.call.return_value = {"content": "The quick brown fox", "pages": 3}
    result = runner.invoke(app, ["items", "fulltext", "12345"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert "content" in data


def test_items_fulltext_unavailable(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["items", "fulltext", "12345"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout


# ---------------------------------------------------------------------------
# items add-from-file
# ---------------------------------------------------------------------------

def test_items_add_from_file_ok(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF-1.4 fake")
    mock_rpc.call.return_value = {"ok": True, "key": "FAKEKEY1", "version": 1}
    result = runner.invoke(app, ["items", "add-from-file", str(pdf)])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["key"] == "FAKEKEY1"


def test_items_add_from_file_dry_run(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF-1.4 fake")
    result = runner.invoke(app, ["items", "add-from-file", str(pdf), "--dry-run"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.addFromFile"
    assert str(pdf) in data["wouldCallParams"]["path"]
    mock_rpc.call.assert_not_called()


def test_items_add_from_file_with_collection(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF-1.4 fake")
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.list": [{"key": "COL5", "name": "MyCollection"}],
        "items.addFromFile": {"ok": True, "key": "ITEM88"},
    }.get(method)
    result = runner.invoke(app, [
        "items", "add-from-file", str(pdf), "--collection", "MyCollection",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["key"] == "ITEM88"


# ---------------------------------------------------------------------------
# items related
# ---------------------------------------------------------------------------

def test_items_related_ok(mock_rpc):
    mock_rpc.call.return_value = [{"key": "KEY0010", "title": "Related Paper", "version": 1}]
    result = runner.invoke(app, ["items", "related", "12345"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data[0]["key"] == "KEY0010"


def test_items_related_unavailable(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["items", "related", "12345"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout


# ---------------------------------------------------------------------------
# items add-related
# ---------------------------------------------------------------------------

def test_items_add_related_ok(mock_rpc):
    mock_rpc.call.return_value = {"added": True}
    result = runner.invoke(app, ["items", "add-related", "12345", "--target", "67890"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["added"] is True


def test_items_add_related_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "items", "add-related", "12345", "--target", "67890", "--dry-run",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.addRelated"
    assert data["wouldCallParams"]["id"] == "12345"
    assert data["wouldCallParams"]["targetId"] == "67890"
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# items remove-related
# ---------------------------------------------------------------------------

def test_items_remove_related_ok(mock_rpc):
    mock_rpc.call.return_value = {"removed": True}
    result = runner.invoke(app, ["items", "remove-related", "12345", "--target", "67890"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["removed"] is True


def test_items_remove_related_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "items", "remove-related", "12345", "--target", "67890", "--dry-run",
    ])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.removeRelated"
    assert data["wouldCallParams"]["targetId"] == "67890"
    mock_rpc.call.assert_not_called()


# ---------------------------------------------------------------------------
# items citation-key
# ---------------------------------------------------------------------------

def test_items_citation_key_ok(mock_rpc):
    mock_rpc.call.return_value = {"citationKey": "Smith2026", "key": "KEY12345"}
    result = runner.invoke(app, ["items", "citation-key", "12345"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["citationKey"] == "Smith2026"


def test_items_citation_key_unavailable(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["items", "citation-key", "12345"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


def test_items_citation_key_accepts_string_key(mock_rpc):
    mock_rpc.call.return_value = {"citationKey": "Jones2025"}
    result = runner.invoke(app, ["items", "citation-key", "ABCD1234"])
    assert result.exit_code == 0
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["id"] == "ABCD1234"
