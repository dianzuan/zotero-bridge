"""Tests for zotron-ocr command routing."""

from __future__ import annotations

import json
import sys
from unittest.mock import MagicMock, patch

from zotron.ocr import cli


def _run_main(argv):
    with patch.object(sys, "argv", ["zotron-ocr", *argv]):
        cli.main()


def test_run_subcommand_processes_collection(capsys):
    proc = MagicMock()
    proc.process_collection.return_value = {"ok": 1, "skipped": 0, "errors": []}
    with patch("zotron.ocr.cli.load_config", return_value={}), patch("zotron.ocr.cli._make_processor", return_value=proc):
        _run_main(["run", "--collection", "案例库"])

    proc.process_collection.assert_called_once_with("案例库", force=False)
    assert json.loads(capsys.readouterr().out)["ok"] == 1


def test_run_subcommand_processes_single_item(capsys):
    proc = MagicMock()
    proc.rpc.call.return_value = {"title": "Paper"}
    proc.process_item.return_value = "ok"
    with patch("zotron.ocr.cli.load_config", return_value={}), patch("zotron.ocr.cli._make_processor", return_value=proc):
        _run_main(["run", "--item", "5443"])

    proc.process_item.assert_called_once_with(5443, "Paper", force=False)
    assert json.loads(capsys.readouterr().out) == {"item_id": 5443, "status": "ok"}


def test_rebuild_subcommand_forces_single_item(capsys):
    proc = MagicMock()
    proc.rpc.call.return_value = {"title": "Paper"}
    proc.process_item.return_value = "ok"
    with patch("zotron.ocr.cli.load_config", return_value={}), patch("zotron.ocr.cli._make_processor", return_value=proc):
        _run_main(["rebuild", "--item", "7"])

    proc.process_item.assert_called_once_with(7, "Paper", force=True)
    assert json.loads(capsys.readouterr().out) == {"item_id": 7, "status": "ok"}


def test_legacy_collection_flags_still_process_collection(capsys):
    proc = MagicMock()
    proc.process_collection.return_value = {"ok": 0, "skipped": 1, "errors": []}
    with patch("zotron.ocr.cli.load_config", return_value={}), patch("zotron.ocr.cli._make_processor", return_value=proc):
        _run_main(["--collection", "旧接口"] )

    proc.process_collection.assert_called_once_with("旧接口", force=False)
    assert json.loads(capsys.readouterr().out)["skipped"] == 1


def test_status_does_not_require_ocr_api_key(capsys):
    rpc = MagicMock()
    rpc.call.side_effect = [
        [{"key": "COL3", "name": "测试集", "children": []}],
        {"items": [{"key": "ITEM10"}, {"key": "ITEM11"}]},
        [],
        [{"tags": ["ocr"]}],
        [{"title": "ITEM.zotron-chunks.jsonl"}],
    ]

    with patch("zotron.ocr.cli.load_config", return_value={"zotero": {"rpc_url": "http://rpc"}}), patch(
        "zotron.ocr.cli.ZoteroRPC", return_value=rpc
    ), patch("zotron.ocr.cli._make_processor") as make_processor:
        _run_main(["status", "--collection", "测试集"])

    make_processor.assert_not_called()
    assert json.loads(capsys.readouterr().out) == {
        "collection": "测试集",
        "total": 2,
        "has_ocr": 2,
        "missing_ocr": 0,
    }


def test_status_counts_missing_when_no_note_or_artifact(capsys):
    rpc = MagicMock()
    rpc.call.side_effect = [
        [{"key": "COL3", "name": "测试集", "children": []}],
        {"items": [{"key": "ITEM10"}]},
        [{"tags": []}],
        [],
    ]

    with patch("zotron.ocr.cli.load_config", return_value={"zotero": {"rpc_url": "http://rpc"}}), patch(
        "zotron.ocr.cli.ZoteroRPC", return_value=rpc
    ), patch("zotron.ocr.cli._make_processor") as make_processor:
        _run_main(["status", "--collection", "测试集"])

    make_processor.assert_not_called()
    assert json.loads(capsys.readouterr().out) == {
        "collection": "测试集",
        "total": 1,
        "has_ocr": 0,
        "missing_ocr": 1,
    }
