"""Tests for zotron.config — load_config."""

import json
import os
from unittest.mock import MagicMock, patch


from zotron.config import DEFAULTS, load_config


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _strip_zb_env(monkeypatch):
    """Remove any real ZOTRON_* env vars that could pollute results."""
    for key in list(os.environ):
        if key.startswith("ZOTRON_"):
            monkeypatch.delenv(key, raising=False)


def _no_zotero(monkeypatch):
    """Patch out _load_from_zotero so tests don't make real HTTP calls."""
    monkeypatch.setattr("zotron.config._load_from_zotero", lambda url: None)


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

def test_load_defaults_when_no_file(tmp_path, monkeypatch):
    """No config file and no env vars → pure defaults are returned."""
    _strip_zb_env(monkeypatch)
    _no_zotero(monkeypatch)
    nonexistent = tmp_path / "does_not_exist.json"
    cfg = load_config(config_path=nonexistent)

    assert cfg["ocr"]["provider"] == DEFAULTS["ocr"]["provider"]
    assert cfg["ocr"]["provider"] == "glm"
    assert cfg["ocr"]["model"] == "glm-ocr"
    assert cfg["ocr"]["api_url"] == "https://open.bigmodel.cn/api/paas/v4/layout_parsing"
    assert cfg["zotero"]["rpc_url"] == DEFAULTS["zotero"]["rpc_url"]
    assert cfg["rag"]["top_k"] == DEFAULTS["rag"]["top_k"]
    assert cfg["embedding"]["model"] == DEFAULTS["embedding"]["model"]


def test_load_from_file(tmp_path, monkeypatch):
    """Values in config file override defaults."""
    _strip_zb_env(monkeypatch)
    _no_zotero(monkeypatch)

    config_file = tmp_path / "config.json"
    config_file.write_text(
        json.dumps({
            "ocr": {"provider": "paddleocr", "concurrency": 8},
            "zotero": {"rpc_url": "http://localhost:9999/rpc"},
        }),
        encoding="utf-8",
    )

    cfg = load_config(config_path=config_file)

    # overridden values
    assert cfg["ocr"]["provider"] == "paddleocr"
    assert cfg["ocr"]["concurrency"] == 8
    assert cfg["zotero"]["rpc_url"] == "http://localhost:9999/rpc"

    # non-overridden values stay as defaults
    assert cfg["ocr"]["output_dir"] == DEFAULTS["ocr"]["output_dir"]
    assert cfg["zotero"]["timeout"] == DEFAULTS["zotero"]["timeout"]


def test_env_overrides_file(tmp_path, monkeypatch):
    """Env vars take precedence over file values (and defaults)."""
    _strip_zb_env(monkeypatch)
    _no_zotero(monkeypatch)

    config_file = tmp_path / "config.json"
    config_file.write_text(
        json.dumps({"ocr": {"provider": "paddleocr"}}),
        encoding="utf-8",
    )

    monkeypatch.setenv("ZOTRON_OCR_PROVIDER", "mineru")
    monkeypatch.setenv("ZOTRON_OCR_MODEL", "custom-ocr-model")
    monkeypatch.setenv("ZOTRON_TIMEOUT", "60")

    cfg = load_config(config_path=config_file)

    # env var wins over file
    assert cfg["ocr"]["provider"] == "mineru"
    assert cfg["ocr"]["model"] == "custom-ocr-model"
    # env var wins over default (note: ZOTRON_TIMEOUT → zotero.timeout)
    assert cfg["zotero"]["timeout"] == 60


def test_load_from_zotero_rpc(tmp_path, monkeypatch):
    """Zotero RPC settings override defaults; env vars still win over RPC."""
    _strip_zb_env(monkeypatch)

    zotero_settings = {
        "ocr.provider": "paddleocr",
        "ocr.apiKey": "zotero-key",
        "ocr.apiUrl": "http://ocr.example.com",
        "embedding.provider": "openai",
        "embedding.model": "text-embedding-3-large",
        "embedding.apiKey": "emb-key",
        "embedding.apiUrl": "http://emb.example.com",
        "rag.chunkSize": 256,
        "rag.chunkOverlap": 32,
        "rag.topK": 10,
    }
    rpc_response = {"jsonrpc": "2.0", "result": zotero_settings, "id": 1}

    mock_response = MagicMock()
    mock_response.json.return_value = rpc_response
    mock_response.raise_for_status.return_value = None

    nonexistent = tmp_path / "no_config.json"

    with patch("httpx.post", return_value=mock_response):
        # Without env var override — Zotero wins over defaults
        cfg = load_config(config_path=nonexistent)

    assert cfg["ocr"]["provider"] == "paddleocr"
    assert cfg["ocr"]["api_key"] == "zotero-key"
    assert cfg["ocr"]["api_url"] == "http://ocr.example.com"
    assert cfg["embedding"]["provider"] == "openai"
    assert cfg["embedding"]["model"] == "text-embedding-3-large"
    assert cfg["embedding"]["api_key"] == "emb-key"
    assert cfg["embedding"]["api_url"] == "http://emb.example.com"
    assert cfg["rag"]["chunk_size"] == 256
    assert cfg["rag"]["chunk_overlap"] == 32
    assert cfg["rag"]["top_k"] == 10

    # Non-mapped keys stay as defaults
    assert cfg["ocr"]["output_dir"] == DEFAULTS["ocr"]["output_dir"]

    # Env var still wins over Zotero RPC
    monkeypatch.setenv("ZOTRON_OCR_PROVIDER", "mineru")
    with patch("httpx.post", return_value=mock_response):
        cfg2 = load_config(config_path=nonexistent)
    assert cfg2["ocr"]["provider"] == "mineru"


def test_load_from_zotero_unreachable(tmp_path, monkeypatch):
    """When Zotero is not running, _load_from_zotero returns None and defaults hold."""
    _strip_zb_env(monkeypatch)
    import httpx

    nonexistent = tmp_path / "no_config.json"
    with patch("httpx.post", side_effect=httpx.ConnectError("refused")):
        cfg = load_config(config_path=nonexistent)

    assert cfg["ocr"]["provider"] == DEFAULTS["ocr"]["provider"]
    assert cfg["rag"]["top_k"] == DEFAULTS["rag"]["top_k"]
