"""Config loading: defaults → Zotero RPC overlay → file overlay → env var overlay."""

from __future__ import annotations

import copy
import json
import os
import re
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------

DEFAULTS: dict[str, Any] = {
    "ocr": {
        "provider": "glm",          # default live OCR: GLM-OCR
        "glm_api_key": "",
        "api_key": "",
        "api_url": "https://open.bigmodel.cn/api/paas/v4/layout_parsing",
        "model": "glm-ocr",
        "output_dir": "~/zotron-ocr-output",
        "concurrency": 4,
    },
    "embedding": {
        "provider": "doubao",
        "model": "doubao-embedding-vision-251215",
        "api_key": "",
        "api_url": "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal",
        "ollama_base_url": "http://localhost:11434",
        "openai_api_key": "",
        "openai_model": "text-embedding-3-small",
    },
    "rag": {
        "index_dir": "~/.local/share/zotron/index",
        "chunk_size": 512,
        "chunk_overlap": 64,
        "top_k": 5,
    },
    "zotero": {
        "rpc_url": "http://localhost:23119/zotron/rpc",
        "timeout": 30,
    },
}

# ---------------------------------------------------------------------------
# Env-var → config-path mapping
# Each value is a tuple of (section, key) into the config dict.
# ---------------------------------------------------------------------------

ENV_MAP: dict[str, tuple[str, str]] = {
    "ZOTRON_OCR_PROVIDER":        ("ocr", "provider"),
    "ZOTRON_GLM_API_KEY":         ("ocr", "glm_api_key"),
    "ZOTRON_OCR_API_KEY":         ("ocr", "api_key"),
    "ZOTRON_OCR_API_URL":         ("ocr", "api_url"),
    "ZOTRON_OCR_MODEL":           ("ocr", "model"),
    "ZOTRON_OCR_OUTPUT_DIR":      ("ocr", "output_dir"),
    "ZOTRON_OCR_CONCURRENCY":     ("ocr", "concurrency"),
    "ZOTRON_EMBED_PROVIDER":      ("embedding", "provider"),
    "ZOTRON_EMBED_MODEL":         ("embedding", "model"),
    "ZOTRON_EMBED_API_KEY":       ("embedding", "api_key"),
    "ZOTRON_EMBED_API_URL":       ("embedding", "api_url"),
    "ZOTRON_OLLAMA_BASE_URL":     ("embedding", "ollama_base_url"),
    "ZOTRON_OPENAI_API_KEY":      ("embedding", "openai_api_key"),
    "ZOTRON_OPENAI_MODEL":        ("embedding", "openai_model"),
    "ZOTRON_INDEX_DIR":           ("rag", "index_dir"),
    "ZOTRON_CHUNK_SIZE":          ("rag", "chunk_size"),
    "ZOTRON_CHUNK_OVERLAP":       ("rag", "chunk_overlap"),
    "ZOTRON_TOP_K":               ("rag", "top_k"),
    "ZOTRON_RPC_URL":             ("zotero", "rpc_url"),
    "ZOTRON_TIMEOUT":             ("zotero", "timeout"),
}

_DEFAULT_CONFIG_PATH = Path("~/.config/zotron/config.json")


def _camel_to_snake(name: str) -> str:
    """Convert camelCase to snake_case (e.g. ``apiKey`` → ``api_key``)."""
    return re.sub(r"([A-Z])", lambda m: "_" + m.group(1).lower(), name)


def _load_from_zotero(rpc_url: str) -> dict[str, Any] | None:
    """Fetch settings from Zotero via JSON-RPC ``settings.getAll``.

    Returns a nested dict matching our config structure, or ``None`` if
    Zotero is not reachable (connection error, timeout, bad response).
    """
    try:
        import httpx  # already in dependencies; import here to keep module light

        payload = {"jsonrpc": "2.0", "method": "settings.getAll", "id": 1}
        response = httpx.post(rpc_url, json=payload, timeout=2.0)
        response.raise_for_status()
        data = response.json()
        flat: dict[str, Any] = data.get("result", {})

        cfg: dict[str, Any] = {}
        for dot_key, value in flat.items():
            parts = dot_key.split(".", 1)
            if len(parts) != 2:
                continue
            section, camel_key = parts
            snake_key = _camel_to_snake(camel_key)
            cfg.setdefault(section, {})[snake_key] = value
        return cfg
    except Exception:
        return None


def _coerce(value: str, default_value: Any) -> Any:
    """Coerce a string env var to the same type as the default value."""
    if isinstance(default_value, bool):
        return value.lower() in ("1", "true", "yes")
    if isinstance(default_value, int):
        return int(value)
    if isinstance(default_value, float):
        return float(value)
    return value


def load_config(config_path: str | Path | None = None) -> dict[str, Any]:
    """Return merged config: defaults → Zotero RPC → file overlay → env var overlay.

    Priority (highest wins):
    1. Environment variables
    2. Config file (``~/.config/zotron/config.json`` by default)
    3. Zotero RPC (``settings.getAll``) — primary source when Zotero is running
    4. Built-in defaults

    Parameters
    ----------
    config_path:
        Path to a JSON config file.  Defaults to
        ``~/.config/zotron/config.json``.  Missing file is silently
        ignored.

    Returns
    -------
    dict
        Fully-merged configuration dictionary.
    """
    cfg: dict[str, Any] = copy.deepcopy(DEFAULTS)

    # --- Zotero RPC overlay ---
    rpc_url: str = cfg["zotero"]["rpc_url"]
    zotero_cfg = _load_from_zotero(rpc_url)
    if zotero_cfg is not None:
        for section, values in zotero_cfg.items():
            if section in cfg and isinstance(values, dict):
                cfg[section].update(values)
            else:
                cfg[section] = values

    # --- file overlay ---
    path = Path(config_path or _DEFAULT_CONFIG_PATH).expanduser()
    if path.exists():
        with path.open(encoding="utf-8") as fh:
            file_cfg: dict[str, Any] = json.load(fh)
        for section, values in file_cfg.items():
            if section in cfg and isinstance(values, dict):
                cfg[section].update(values)
            else:
                cfg[section] = values

    # --- env var overlay ---
    for env_var, (section, key) in ENV_MAP.items():
        raw = os.environ.get(env_var)
        if raw is not None:
            default_val = DEFAULTS.get(section, {}).get(key, "")
            cfg.setdefault(section, {})[key] = _coerce(raw, default_val)

    return cfg
