"""Tests for zotron.cli."""
import json
from unittest.mock import patch, MagicMock

import pytest
from typer.testing import CliRunner

from zotron.cli import app


runner = CliRunner()


@pytest.fixture
def mock_rpc():
    """Patch ZoteroRPC at the module level. Returns the MagicMock instance
    so individual tests can configure .call responses."""
    with patch("zotron._cli_base.ZoteroRPC") as mock_cls:
        instance = MagicMock()
        mock_cls.return_value = instance
        yield instance


def test_ping_ok(mock_rpc):
    mock_rpc.call.return_value = {"status": "ok", "timestamp": "2026-04-22T12:00:00Z"}
    result = runner.invoke(app, ["ping"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["status"] == "ok"


def test_ping_unavailable(mock_rpc):
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["ping"])
    assert result.exit_code != 0
    assert "ZOTERO_UNAVAILABLE" in result.stdout or "ZOTERO_UNAVAILABLE" in result.stderr


def test_collections_list(mock_rpc):
    mock_rpc.call.return_value = [
        {"key": "COL1", "name": "Research", "parentKey": None},
        {"key": "COL2", "name": "Teaching", "parentKey": None},
    ]
    result = runner.invoke(app, ["collections", "list"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert len(data) == 2
    assert data[0]["name"] == "Research"


def test_collections_tree(mock_rpc):
    mock_rpc.call.return_value = {
        "key": "COL1", "name": "root",
        "children": [{"key": "COL2", "name": "child", "children": []}],
    }
    result = runner.invoke(app, ["collections", "tree"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["name"] == "root"


def test_collections_rename_happy_path(mock_rpc):
    """rename by existing name → resolve key → call rename RPC."""
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.list": [{"key": "COL42", "name": "typo-案例库"}],
        "collections.rename": {"ok": True, "key": "COL42"},
    }.get(method)
    result = runner.invoke(app, ["collections", "rename",
                                 "typo-案例库", "案例库"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    rename_calls = [c for c in mock_rpc.call.call_args_list
                    if c.args[0] == "collections.rename"]
    assert len(rename_calls) == 1
    assert rename_calls[0].args[1] == {"key": "COL42", "name": "案例库"}


def test_collections_rename_not_found(mock_rpc):
    """Nonexistent name → COLLECTION_NOT_FOUND exit."""
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.list": [],
    }.get(method)
    result = runner.invoke(app, ["collections", "rename",
                                 "ghost", "whatever"])
    assert result.exit_code != 0
    assert "COLLECTION_NOT_FOUND" in (result.stdout + (result.stderr or ""))



def test_collections_create(mock_rpc):
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.create": {"ok": True, "key": "COL100", "name": params["name"]},
    }.get(method)
    result = runner.invoke(app, ["collections", "create", "NewColl"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)["name"] == "NewColl"


def test_collections_delete(mock_rpc):
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.list": [{"key": "COL200", "name": "Old"}],
        "collections.delete": {"ok": True, "key": "COL200"},
    }.get(method)
    result = runner.invoke(app, ["collections", "delete", "Old"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)["ok"] is True


def test_items_add_by_doi(mock_rpc):
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.list": [{"key": "COL7", "name": "Refs"}],
        "items.addByDOI": [{"key": "ITEM999", "title": "Found by DOI"}],
    }.get(method)
    result = runner.invoke(app, ["items", "add-by-doi", "10.x/y",
                                 "--collection", "Refs"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data[0]["key"] == "ITEM999"


def test_items_find_duplicates(mock_rpc):
    mock_rpc.call.return_value = {"groups": [["K1", "K2"], ["K3", "K4", "K5"]], "totalGroups": 2}
    result = runner.invoke(app, ["items", "find-duplicates"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)["totalGroups"] == 2


def test_items_merge_requires_two(mock_rpc):
    result = runner.invoke(app, ["items", "merge-duplicates", "42"])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout


def test_search_quick(mock_rpc):
    mock_rpc.call.return_value = {"items": [{"key": "ITEM1"}], "total": 1}
    result = runner.invoke(app, ["search", "quick", "乡村振兴", "--limit", "10"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)["total"] == 1


def test_search_by_identifier_requires_one(mock_rpc):
    result = runner.invoke(app, ["search", "by-identifier"])
    assert result.exit_code != 0
    assert "INVALID_ARGS" in result.stdout


def test_tags_list(mock_rpc):
    mock_rpc.call.return_value = [{"tag": "foo", "type": 1}]
    result = runner.invoke(app, ["tags", "list", "--limit", "5"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)[0]["tag"] == "foo"


def test_tags_rename(mock_rpc):
    mock_rpc.call.return_value = {"renamed": True, "from": "old", "to": "new"}
    result = runner.invoke(app, ["tags", "rename", "old", "new"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)["renamed"] is True


def test_export_bibtex_prints_raw_content(mock_rpc):
    """export.* returns {content: "..."}. CLI prints content, not JSON envelope."""
    mock_rpc.call.return_value = {"content": "@article{foo, ...}"}
    result = runner.invoke(app, ["export", "bibtex", "1", "2"])
    assert result.exit_code == 0
    assert result.stdout.strip() == "@article{foo, ...}"


def test_export_bibliography_emits_text_by_default(mock_rpc):
    """XPI returns {format, style, html, text, count}. Default: emit .text."""
    mock_rpc.call.return_value = {
        "format": "bibliography", "style": "apa",
        "html": "<div>Citation HTML</div>",
        "text": "Citation plain text",
        "count": 1,
    }
    result = runner.invoke(app, ["export", "bibliography", "42"])
    assert result.exit_code == 0
    assert result.stdout.strip() == "Citation plain text"


def test_export_bibliography_html_flag(mock_rpc):
    """--html emits the HTML variant instead."""
    mock_rpc.call.return_value = {
        "format": "bibliography", "style": "apa",
        "html": "<div>Citation HTML</div>",
        "text": "Citation plain text",
        "count": 1,
    }
    result = runner.invoke(app, ["export", "bibliography", "42", "--html"])
    assert result.exit_code == 0
    assert result.stdout.strip() == "<div>Citation HTML</div>"


def test_system_version(mock_rpc):
    mock_rpc.call.return_value = {"version": "0.3.0", "methods": ["system.ping"]}
    result = runner.invoke(app, ["system", "version"])
    assert result.exit_code == 0
    assert json.loads(result.stdout)["version"] == "0.3.0"


def test_system_libraries(mock_rpc):
    mock_rpc.call.return_value = [
        {"id": 1, "type": "user", "name": "My Library"},
        {"id": 42, "type": "group", "name": "Research Group"},
    ]
    result = runner.invoke(app, ["system", "libraries"])
    assert result.exit_code == 0
    assert len(json.loads(result.stdout)) == 2


def test_push_from_file(mock_rpc, tmp_path):
    item = {"itemType": "journalArticle", "title": "From File", "DOI": "10.x/a"}
    item_file = tmp_path / "item.json"
    item_file.write_text(json.dumps(item), encoding="utf-8")
    # Make find_duplicate return nothing and items.create succeed.
    mock_rpc.call.side_effect = lambda method, params=None: {
        "search.byIdentifier": [],
        "search.quick": [],
        "collections.list": [],
        "system.currentCollection": {"key": None},
        "items.create": {"ok": True, "key": "K77", "version": 1},
    }.get(method)
    result = runner.invoke(app, ["push", str(item_file)])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["status"] == "created"
    assert data["zotero_item_id"] == "K77"


def test_push_from_stdin(mock_rpc):
    item = {"itemType": "journalArticle", "title": "FromStdin", "DOI": "10.x/b"}
    mock_rpc.call.side_effect = lambda method, params=None: {
        "search.byIdentifier": [],
        "search.quick": [],
        "collections.list": [],
        "system.currentCollection": {"key": None},
        "items.create": {"ok": True, "key": "K88"},
    }.get(method)
    result = runner.invoke(app, ["push", "-"], input=json.dumps(item))
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["status"] == "created"
    assert data["zotero_item_id"] == "K88"


def test_push_with_invalid_pdf(mock_rpc, tmp_path):
    item = {"itemType": "journalArticle", "title": "X", "DOI": "10.x/c"}
    item_file = tmp_path / "item.json"
    item_file.write_text(json.dumps(item), encoding="utf-8")
    bad_pdf = tmp_path / "bad.pdf"
    bad_pdf.write_bytes(b"<html></html>")
    result = runner.invoke(app, [
        "push", str(item_file), "--pdf", str(bad_pdf),
    ])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["ok"] is False
    assert data["error"]["code"] == "INVALID_PDF"


def test_push_collection_ambiguous(mock_rpc, tmp_path):
    item = {"itemType": "journalArticle", "title": "Y", "DOI": "10.x/d"}
    item_file = tmp_path / "item.json"
    item_file.write_text(json.dumps(item), encoding="utf-8")
    mock_rpc.call.side_effect = lambda method, params=None: {
        "collections.list": [
            {"key": "COL1", "name": "Papers 2024"},
            {"key": "COL2", "name": "Papers 2025"},
        ],
    }.get(method, [])
    result = runner.invoke(app, [
        "push", str(item_file), "--collection", "papers",
    ])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "COLLECTION_AMBIGUOUS"


def test_rpc_calls_method_with_params(mock_rpc):
    """zotron rpc <method> <json> forwards to ZoteroRPC.call()."""
    mock_rpc.call.return_value = {"itemKey": "ABC123"}
    result = runner.invoke(
        app,
        ["rpc", "items.get", '{"id": 12345}'],
    )
    assert result.exit_code == 0
    mock_rpc.call.assert_called_once_with("items.get", {"id": 12345})
    data = json.loads(result.stdout)
    assert data["itemKey"] == "ABC123"


def test_rpc_defaults_params_to_empty_object(mock_rpc):
    """rpc <method> with no params arg defaults params to {}."""
    mock_rpc.call.return_value = {"status": "ok"}
    result = runner.invoke(app, ["rpc", "system.ping"])
    assert result.exit_code == 0
    mock_rpc.call.assert_called_once_with("system.ping", {})


def test_rpc_rejects_invalid_json(mock_rpc):
    """Invalid JSON params yields non-zero exit with INVALID_JSON error envelope."""
    result = runner.invoke(app, ["rpc", "items.get", "not-json"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "INVALID_JSON"


def test_rpc_propagates_zotero_unavailable(mock_rpc):
    """Connection errors surface as ZOTERO_UNAVAILABLE JSON envelope on stdout."""
    mock_rpc.call.side_effect = ConnectionError("refused")
    result = runner.invoke(app, ["rpc", "system.ping"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "ZOTERO_UNAVAILABLE"


def test_rpc_propagates_rpc_error(mock_rpc):
    """XPI-side errors (RuntimeError) surface as RPC_ERROR JSON envelope on stdout."""
    mock_rpc.call.side_effect = RuntimeError("[-32601] Method not found")
    result = runner.invoke(app, ["rpc", "system.bogus"])
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert "error" in data
    assert "[-32601] Method not found" in data["error"]["message"]


def test_find_pdfs_lists_items_missing_pdf(mock_rpc):
    """find-pdfs should enumerate items in collection, check attachments.list per item,
    and call attachments.findPDF for items lacking a PDF."""
    items_in_collection = [
        {"key": "ITEM10", "title": "A"},   # no attachments → will be findPDF'd
        {"key": "ITEM11", "title": "B"},   # has a PDF → skipped
    ]
    attachments_per_item = {
        "ITEM10": [],
        "ITEM11": [{"key": "ATT20", "contentType": "application/pdf", "title": "Full Text PDF"}],
    }

    def call(method, params=None):
        params = params or {}
        if method == "collections.list":
            return [{"key": "COL1", "name": "Research"}]
        if method == "collections.getItems":
            return {"items": items_in_collection, "total": len(items_in_collection)}
        if method == "attachments.list":
            return attachments_per_item.get(params["parentKey"], [])
        if method == "attachments.findPDF":
            if params["parentKey"] == "ITEM10":
                return {"attachment": {"key": "ATT99", "title": "Full Text PDF"}}
            return {"attachment": None}
        return None

    mock_rpc.call.side_effect = call
    result = runner.invoke(app, ["find-pdfs", "--collection", "Research"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["scanned"] == 2
    assert data["attempted"] == 1   # only ITEM10 needed a PDF
    assert data["results"][0]["item_key"] == "ITEM10"
    assert data["results"][0]["found"] is True
    assert data["results"][0]["attachment_key"] == "ATT99"


def test_help_epilog_contains_examples():
    """Every command's --help must contain an 'Examples:' section."""
    cmds_to_check = [
        ["ping", "--help"],
        ["rpc", "--help"],
        ["search", "quick", "--help"],
        ["collections", "create", "--help"],
        ["items", "add-by-doi", "--help"],
        ["push", "--help"],
        ["export", "bibtex", "--help"],
        ["system", "version", "--help"],
    ]
    for argv in cmds_to_check:
        result = runner.invoke(app, argv)
        assert result.exit_code == 0, f"{argv} --help failed"
        assert "Examples:" in result.stdout, f"{argv} missing Examples section"
        assert "zotron" in result.stdout, f"{argv} epilog missing example"


def test_collections_list_table_output(mock_rpc):
    """--output table renders headers + rows for list-of-dicts."""
    mock_rpc.call.return_value = [
        {"key": "COL1", "name": "Research", "parentKey": None},
        {"key": "COL2", "name": "Teaching", "parentKey": "COL1"},
    ]
    result = runner.invoke(app, ["collections", "list", "--output", "table"])
    assert result.exit_code == 0
    # Headers
    assert "key" in result.stdout and "name" in result.stdout
    # Rows
    assert "Research" in result.stdout and "Teaching" in result.stdout
    # Should NOT be JSON
    assert not result.stdout.lstrip().startswith("[")


def test_collections_list_json_default(mock_rpc):
    """Default output remains json — no behavior change for existing callers."""
    mock_rpc.call.return_value = [{"key": "COL1", "name": "Research"}]
    result = runner.invoke(app, ["collections", "list"])
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data == [{"key": "COL1", "name": "Research"}]


def test_rpc_with_jq_filter(mock_rpc):
    """rpc --jq filters output server-side(ish), reducing tokens."""
    mock_rpc.call.return_value = [
        {"key": "K1", "title": "Paper A", "year": 2024},
        {"key": "K2", "title": "Paper B", "year": 2025},
    ]
    result = runner.invoke(
        app,
        ["rpc", "items.list", "{}", "--jq", ".[].title"],
    )
    assert result.exit_code == 0
    assert "Paper A" in result.stdout and "Paper B" in result.stdout
    # `key` and `year` keys should not appear in filtered output
    assert '"key"' not in result.stdout
    assert '"year"' not in result.stdout


def test_rpc_invalid_jq_emits_envelope(mock_rpc):
    """Invalid jq expression yields INVALID_JQ envelope."""
    mock_rpc.call.return_value = [{"key": "K1"}]
    result = runner.invoke(
        app,
        ["rpc", "items.list", "{}", "--jq", "[[[unbalanced"],
    )
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "INVALID_JQ"


def test_items_add_by_doi_dry_run_does_not_call_write(mock_rpc):
    """--dry-run prints intended call envelope, does NOT invoke RPC."""
    result = runner.invoke(
        app,
        ["items", "add-by-doi", "10.1038/nature12373", "--dry-run"],
    )
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "items.addByDOI"
    assert data["wouldCallParams"]["doi"] == "10.1038/nature12373"
    # The write RPC must not have been called
    write_calls = [c for c in mock_rpc.call.call_args_list
                   if c.args[0] == "items.addByDOI"]
    assert write_calls == []


def test_collections_create_dry_run(mock_rpc):
    result = runner.invoke(
        app,
        ["collections", "create", "TestColl", "--dry-run"],
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldCall"] == "collections.create"
    assert data["wouldCallParams"] == {"name": "TestColl"}
    create_calls = [c for c in mock_rpc.call.call_args_list
                    if c.args[0] == "collections.create"]
    assert create_calls == []


def test_items_trash_dry_run(mock_rpc):
    result = runner.invoke(
        app,
        ["items", "trash", "12345", "--dry-run"],
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["wouldCall"] == "items.trash"
    assert data["wouldCallParams"] == {"key": "12345"}


def test_push_dry_run_does_not_call_push_item(mock_rpc, tmp_path):
    paper_json = tmp_path / "paper.json"
    paper_json.write_text(json.dumps({
        "itemType": "journalArticle",
        "title": "Attention is all you need",
    }))
    with patch("zotron.cli.push_item") as mock_push:
        result = runner.invoke(
            app,
            ["push", str(paper_json), "--dry-run"],
        )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert data["dryRun"] is True
    assert data["wouldPush"]["title"] == "Attention is all you need"
    mock_push.assert_not_called()


def test_search_quick_invalid_jq_emits_envelope(mock_rpc):
    """A jq runtime error on a non-rpc command must surface via INVALID_JQ envelope."""
    mock_rpc.call.return_value = {"items": [{"key": "K1", "title": "x"}], "total": 1}
    result = runner.invoke(
        app,
        ["search", "quick", "test", "--jq", '[.[] | {key}]'],
    )
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["ok"] is False
    assert data["error"]["code"] == "INVALID_JQ"


def test_collections_list_invalid_jq_emits_envelope(mock_rpc):
    mock_rpc.call.return_value = [{"key": "COL1", "name": "x"}]
    result = runner.invoke(
        app,
        ["collections", "list", "--jq", "[[[broken"],
    )
    assert result.exit_code != 0
    data = json.loads(result.stdout)
    assert data["error"]["code"] == "INVALID_JQ"


def test_rpc_paginate_loops_until_short_page(mock_rpc):
    mock_rpc.call.side_effect = [
        [{"key": "K1"}, {"key": "K2"}],      # full
        [{"key": "K3"}, {"key": "K4"}],      # full
        [{"key": "K5"}],                      # short — stop
    ]
    result = runner.invoke(
        app,
        ["rpc", "items.list", "{}", "--paginate", "--page-size", "2"],
    )
    assert result.exit_code == 0
    data = json.loads(result.stdout)
    assert [r["key"] for r in data] == ["K1", "K2", "K3", "K4", "K5"]
    assert mock_rpc.call.call_count == 3


def test_collection_delete_chinese_name_unresolvable_envelope_unescaped(mock_rpc):
    """Error messages must surface Chinese unescaped in the envelope."""
    # collections.list returns empty so the resolve fails
    mock_rpc.call.return_value = []
    result = runner.invoke(app, ["collections", "delete", "测试集合"])
    assert result.exit_code != 0
    # Raw stdout should contain the Chinese characters, not \uXXXX escapes
    assert "测试集合" in result.stdout
    assert "\\u" not in result.stdout
