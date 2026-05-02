"""Tests for zotron CLI attachments namespace."""
import json
from unittest.mock import patch, MagicMock

import pytest
from typer.testing import CliRunner

import zotron._cli_base  # ensure the module is importable before patching  # noqa: F401
from zotron.cli import app

runner = CliRunner()


@pytest.fixture
def mock_rpc():
    instance = MagicMock()
    with patch("zotron._cli_base.ZoteroRPC", return_value=instance):
        yield instance


# ---------------------------------------------------------------------------
# attachments list
# ---------------------------------------------------------------------------

def test_attachments_list(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [{"key": "KEY0010", "title": "paper.pdf", "version": 1}],
        "total": 1,
        "limit": 50,
        "offset": 0,
        "hasMore": False,
    }
    result = runner.invoke(app, ["attachments", "list", "--parent", "42"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["total"] == 1
    assert data["items"][0]["key"] == "KEY0010"


def test_attachments_list_with_limit_offset(mock_rpc):
    mock_rpc.call.return_value = {
        "items": [],
        "total": 0,
        "limit": 10,
        "offset": 20,
        "hasMore": False,
    }
    result = runner.invoke(app, [
        "attachments", "list", "--parent", "42",
        "--limit", "10", "--offset", "20",
    ])
    assert result.exit_code == 0, result.stdout
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["parentId"] == "42"
    assert call_args.args[1]["limit"] == 10
    assert call_args.args[1]["offset"] == 20


def test_attachments_list_missing_parent(mock_rpc):
    result = runner.invoke(app, ["attachments", "list"])
    assert result.exit_code != 0


def test_attachments_list_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["attachments", "list", "--parent", "1"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout


# ---------------------------------------------------------------------------
# attachments get
# ---------------------------------------------------------------------------

def test_attachments_get(mock_rpc):
    mock_rpc.call.return_value = {
        "key": "KEY0010",
        "title": "paper.pdf",
        "contentType": "application/pdf",
        "version": 1,
    }
    result = runner.invoke(app, ["attachments", "get", "10"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0010"
    assert data["contentType"] == "application/pdf"


def test_attachments_get_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["attachments", "get", "10"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout


# ---------------------------------------------------------------------------
# attachments fulltext
# ---------------------------------------------------------------------------

def test_attachments_fulltext(mock_rpc):
    mock_rpc.call.return_value = {
        "key": "KEY0010",
        "content": "This is the full text of the PDF.",
        "indexedChars": 5000,
        "totalChars": 5000,
        "version": 1,
    }
    result = runner.invoke(app, ["attachments", "fulltext", "10"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0010"
    assert "content" in data
    assert data["indexedChars"] == 5000


def test_attachments_fulltext_rpc_error(mock_rpc):
    mock_rpc.call.side_effect = RuntimeError("[-32601] Method not found")
    result = runner.invoke(app, ["attachments", "fulltext", "99"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "RPC_ERROR"


# ---------------------------------------------------------------------------
# attachments add
# ---------------------------------------------------------------------------

def test_attachments_add(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF-1.4 fake content")
    mock_rpc.call.return_value = {
        "ok": True,
        "key": "KEY0020",
        "title": "paper.pdf",
        "contentType": "application/pdf",
        "version": 1,
    }
    result = runner.invoke(app, [
        "attachments", "add",
        "--parent", "42",
        "--path", str(pdf),
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0020"


def test_attachments_add_with_title(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF-1.4 fake content")
    mock_rpc.call.return_value = {
        "ok": True,
        "key": "KEY0021",
        "title": "My Custom Title",
        "contentType": "application/pdf",
        "version": 1,
    }
    result = runner.invoke(app, [
        "attachments", "add",
        "--parent", "42",
        "--path", str(pdf),
        "--title", "My Custom Title",
    ])
    assert result.exit_code == 0, result.stdout
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["title"] == "My Custom Title"


def test_attachments_add_dry_run(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF-1.4 fake")
    result = runner.invoke(app, [
        "attachments", "add",
        "--parent", "42",
        "--path", str(pdf),
        "--dry-run",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "attachments.add"
    mock_rpc.call.assert_not_called()


def test_attachments_add_missing_parent(mock_rpc, tmp_path):
    pdf = tmp_path / "paper.pdf"
    pdf.write_bytes(b"%PDF")
    result = runner.invoke(app, ["attachments", "add", "--path", str(pdf)])
    assert result.exit_code != 0


# ---------------------------------------------------------------------------
# attachments add-by-url
# ---------------------------------------------------------------------------

def test_attachments_add_by_url(mock_rpc):
    mock_rpc.call.return_value = {
        "ok": True,
        "key": "KEY0030",
        "title": "Remote PDF",
        "url": "https://example.com/paper.pdf",
        "version": 1,
    }
    result = runner.invoke(app, [
        "attachments", "add-by-url",
        "--parent", "42",
        "--url", "https://example.com/paper.pdf",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0030"


def test_attachments_add_by_url_with_title(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0031", "title": "Custom Title", "version": 1}
    result = runner.invoke(app, [
        "attachments", "add-by-url",
        "--parent", "42",
        "--url", "https://example.com/paper.pdf",
        "--title", "Custom Title",
    ])
    assert result.exit_code == 0, result.stdout
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["title"] == "Custom Title"


def test_attachments_add_by_url_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "attachments", "add-by-url",
        "--parent", "42",
        "--url", "https://example.com/paper.pdf",
        "--dry-run",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "attachments.addByURL"
    assert data["wouldCallParams"]["url"] == "https://example.com/paper.pdf"
    mock_rpc.call.assert_not_called()


def test_attachments_add_by_url_missing_url(mock_rpc):
    result = runner.invoke(app, [
        "attachments", "add-by-url",
        "--parent", "42",
    ])
    assert result.exit_code != 0


# ---------------------------------------------------------------------------
# attachments path
# ---------------------------------------------------------------------------

def test_attachments_path(mock_rpc):
    mock_rpc.call.return_value = {
        "key": "KEY0010",
        "path": "/home/user/Zotero/storage/ABC123/paper.pdf",
        "version": 1,
    }
    result = runner.invoke(app, ["attachments", "path", "10"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["key"] == "KEY0010"
    assert "path" in data


def test_attachments_path_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["attachments", "path", "10"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout


# ---------------------------------------------------------------------------
# attachments delete
# ---------------------------------------------------------------------------

def test_attachments_delete(mock_rpc):
    mock_rpc.call.return_value = {"ok": True, "key": "KEY0010"}
    result = runner.invoke(app, ["attachments", "delete", "10"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "KEY0010"


def test_attachments_delete_dry_run(mock_rpc):
    result = runner.invoke(app, [
        "attachments", "delete", "10", "--dry-run",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "attachments.delete"
    assert data["wouldCallParams"]["id"] == "10"
    mock_rpc.call.assert_not_called()


def test_attachments_delete_rpc_error(mock_rpc):
    mock_rpc.call.side_effect = RuntimeError("not found")
    result = runner.invoke(app, ["attachments", "delete", "999"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "RPC_ERROR"


# ---------------------------------------------------------------------------
# attachments find-pdf
# ---------------------------------------------------------------------------

def test_attachments_find_pdf_found(mock_rpc):
    mock_rpc.call.return_value = {
        "attachment": {"key": "KEY0050", "title": "Full Text PDF", "version": 1},
    }
    result = runner.invoke(app, [
        "attachments", "find-pdf", "--parent", "42",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["attachment"]["key"] == "KEY0050"


def test_attachments_find_pdf_not_found(mock_rpc):
    mock_rpc.call.return_value = {"attachment": None}
    result = runner.invoke(app, [
        "attachments", "find-pdf", "--parent", "42",
    ])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["attachment"] is None


def test_attachments_find_pdf_missing_parent(mock_rpc):
    result = runner.invoke(app, ["attachments", "find-pdf"])
    assert result.exit_code != 0


def test_attachments_find_pdf_connection_error(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, [
        "attachments", "find-pdf", "--parent", "42",
    ])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout
