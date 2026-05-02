"""Tests for zotron CLI settings namespace."""
import json
import tempfile
from pathlib import Path
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


def test_settings_get(mock_rpc):
    """settings get <key> calls settings.get and prints {key, value}."""
    mock_rpc.call.return_value = {"key": "extensions.zotero.openURL.resolver", "value": "https://libgen.rs"}
    result = runner.invoke(app, ["settings", "get", "extensions.zotero.openURL.resolver"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["key"] == "extensions.zotero.openURL.resolver"
    assert data["value"] == "https://libgen.rs"


def test_settings_set_string_value(mock_rpc):
    """settings set <key> <value> with a plain string calls settings.set."""
    mock_rpc.call.return_value = {"ok": True, "key": "extensions.zotero.debug.log"}
    result = runner.invoke(app, ["settings", "set", "extensions.zotero.debug.log", "false"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["key"] == "extensions.zotero.debug.log"
    # Confirm the RPC was called with parsed JSON value (false → bool False)
    call_args = mock_rpc.call.call_args
    assert call_args.args[0] == "settings.set"
    assert call_args.args[1]["value"] is False


def test_settings_set_plain_string_fallback(mock_rpc):
    """settings set passes value as plain string when it is not valid JSON."""
    mock_rpc.call.return_value = {"ok": True, "key": "some.pref"}
    result = runner.invoke(app, ["settings", "set", "some.pref", "hello world"])
    assert result.exit_code == 0, result.stdout
    call_args = mock_rpc.call.call_args
    assert call_args.args[1]["value"] == "hello world"


def test_settings_set_dry_run(mock_rpc):
    """--dry-run prints the dry-run envelope and does not call the RPC."""
    result = runner.invoke(app, ["settings", "set", "foo.bar", "42", "--dry-run"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert data["ok"] is True
    assert data["dryRun"] is True
    assert data["wouldCall"] == "settings.set"
    mock_rpc.call.assert_not_called()


def test_settings_list(mock_rpc):
    """settings list calls settings.getAll and prints the dict."""
    mock_rpc.call.return_value = {
        "extensions.zotero.openURL.resolver": "https://libgen.rs",
        "extensions.zotero.debug.log": False,
    }
    result = runner.invoke(app, ["settings", "list"])
    assert result.exit_code == 0, result.stdout
    data = json.loads(result.stdout)
    assert "extensions.zotero.openURL.resolver" in data


def test_settings_set_all(mock_rpc):
    """settings set-all --file <path> reads JSON and calls settings.setAll."""
    mock_rpc.call.return_value = {"ok": True, "count": 2}
    prefs = {"extensions.zotero.openURL.resolver": "https://libgen.rs", "extensions.zotero.debug.log": False}
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        json.dump(prefs, f)
        tmp_path = f.name
    try:
        result = runner.invoke(app, ["settings", "set-all", "--file", tmp_path])
        assert result.exit_code == 0, result.stdout
        data = json.loads(result.stdout)
        assert data["ok"] is True
        assert data["count"] == 2
        call_args = mock_rpc.call.call_args
        assert call_args.args[0] == "settings.setAll"
        assert call_args.args[1] == {"settings": prefs}
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def test_settings_set_all_dry_run(mock_rpc):
    """settings set-all --dry-run prints dry-run envelope without calling RPC."""
    prefs = {"foo": "bar"}
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        json.dump(prefs, f)
        tmp_path = f.name
    try:
        result = runner.invoke(app, ["settings", "set-all", "--file", tmp_path, "--dry-run"])
        assert result.exit_code == 0, result.stdout
        data = json.loads(result.stdout)
        assert data["ok"] is True
        assert data["dryRun"] is True
        mock_rpc.call.assert_not_called()
    finally:
        Path(tmp_path).unlink(missing_ok=True)


def test_settings_set_all_invalid_json_file(mock_rpc):
    """settings set-all with a non-JSON file exits non-zero with INVALID_JSON."""
    with tempfile.NamedTemporaryFile(mode="w", suffix=".json", delete=False) as f:
        f.write("this is not json")
        tmp_path = f.name
    try:
        result = runner.invoke(app, ["settings", "set-all", "--file", tmp_path])
        assert result.exit_code != 0
        assert "INVALID_JSON" in result.stdout
    finally:
        Path(tmp_path).unlink(missing_ok=True)
