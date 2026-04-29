"""Filesystem path helpers for Zotero RPC calls."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


def is_wsl() -> bool:
    """Return True when running under Windows Subsystem for Linux."""
    if os.environ.get("WSL_DISTRO_NAME"):
        return True
    try:
        release = Path("/proc/sys/kernel/osrelease").read_text(encoding="utf-8").lower()
    except OSError:
        return False
    return "microsoft" in release or "wsl" in release


def zotero_path(local_path: str | Path) -> str:
    """Translate a local path to a string Zotero Desktop can open.

    Under WSL, Zotero usually runs as a Windows app and cannot read POSIX
    paths like ``/home/user/file``. Convert them to Windows/UNC paths through
    ``wslpath -w``. Native Linux/macOS paths pass through unchanged.
    """
    path_str = str(local_path)
    if is_wsl():
        try:
            result = subprocess.run(
                ["wslpath", "-w", path_str],
                capture_output=True,
                text=True,
                timeout=5,
                check=True,
            )
            return result.stdout.strip()
        except (subprocess.SubprocessError, FileNotFoundError):
            return path_str
    return path_str


def linux_path(path: str) -> str:
    """Convert a Windows path to a WSL Linux path when possible."""
    try:
        result = subprocess.run(
            ["wslpath", "-u", path],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return result.stdout.strip()
    except (FileNotFoundError, subprocess.TimeoutExpired):
        pass
    return path
