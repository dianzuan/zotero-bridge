"""JSON-RPC client for Zotron XPI."""
from __future__ import annotations

import os
import subprocess
from pathlib import Path

import httpx


def _is_wsl() -> bool:
    if os.environ.get("WSL_DISTRO_NAME"):
        return True
    try:
        return "microsoft" in Path("/proc/sys/kernel/osrelease").read_text().lower()
    except OSError:
        return False


class ZoteroRPC:
    def __init__(self, url: str, client: httpx.Client | None = None):
        self.url = url
        self._client = client or httpx.Client(timeout=30.0)
        self._id = 0

    @staticmethod
    def zotero_path(local_path: str | Path) -> str:
        """Translate a local path so Zotero Desktop can open it.

        On WSL, Zotero runs as a Windows app and cannot read POSIX paths.
        This converts them via ``wslpath -w``. On native platforms, passes through.
        """
        p = str(local_path)
        if not _is_wsl():
            return p
        try:
            result = subprocess.run(
                ["wslpath", "-w", p],
                capture_output=True, text=True, timeout=5, check=True,
            )
            return result.stdout.strip()
        except (subprocess.SubprocessError, FileNotFoundError):
            return p

    def call(self, method: str, params: dict | None = None) -> dict:
        self._id += 1
        payload = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {},
            "id": self._id,
        }
        try:
            resp = self._client.post(self.url, json=payload)
        except (httpx.ConnectError, httpx.ConnectTimeout):
            raise ConnectionError(
                "Cannot connect to Zotero. Is it running with zotron plugin?"
            )
        data = resp.json()
        if "error" in data:
            err = data["error"]
            raise RuntimeError(f"[{err['code']}] {err['message']}")
        return data.get("result")
