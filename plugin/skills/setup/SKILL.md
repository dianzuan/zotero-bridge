---
name: setup
description: Set up Zotron — verify `zotron ping` works; if missing, download the release XPI to Downloads and guide Zotero's native install dialog. Use when the user asks to install, configure, bootstrap, or verify Zotron.
---

# Zotron Setup

Run this when the user has just installed the Zotron plugin or when `zotron ping` cannot reach Zotero.

## Goal

End state: `zotron ping` succeeds and the agent can call the single `zotron` CLI. OCR and RAG are subcommands: `zotron ocr ...` and `zotron rag ...`.

## Distribution model

The repository does not track `zotron.xpi`. New installs download the release XPI. If GitHub is unavailable, set `ZOTRON_XPI_URLS` to one or more mirror URLs. If a local file is already available, set `ZOTRON_XPI_PATH=/path/to/zotron.xpi`.

If Zotron is already installed but the setup target version is newer, do not reinstall from setup. Tell the user to use Zotero's native update flow:

```text
Tools -> Plugins -> Zotron -> gear icon -> Check for Updates -> restart Zotero
```

## Procedure

1. Check that the `zotron` CLI is installed. If missing, run the cascading install script below. It tries four methods in order and stops at the first success.

```bash
#!/usr/bin/env bash
set -euo pipefail

if command -v zotron >/dev/null 2>&1; then
  echo "zotron already installed: $(command -v zotron)"
  exit 0
fi

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "$INSTALL_DIR"

# --- Detect platform ---
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in aarch64|arm64) ARCH=arm64 ;; x86_64|amd64) ARCH=amd64 ;; esac
case "$OS" in
  linux)  PLATFORM="linux-${ARCH}" ;;
  darwin) PLATFORM="macos-${ARCH}" ;;
  *)      PLATFORM="windows-amd64.exe" ;;
esac
# Release binaries are versioned (zotron-<version>-<platform>), so resolve the
# latest tag first, then build the download URL for that exact asset.
TAG=$(curl -fsSL https://api.github.com/repos/dianzuan/zotron/releases/latest \
        | grep -m1 '"tag_name"' | sed -E 's/.*"tag_name": *"([^"]+)".*/\1/')
GITHUB_URL="https://github.com/dianzuan/zotron/releases/download/${TAG}/zotron-${TAG#v}-${PLATFORM}"

installed=false

# --- Method 1: cargo binstall (prebuilt binary via cargo-binstall) ---
if ! $installed && command -v cargo-binstall >/dev/null 2>&1; then
  echo "[1/4] Trying cargo binstall zotron ..."
  if cargo binstall zotron --no-confirm 2>/dev/null; then
    installed=true
  fi
fi

# --- Method 2: Direct download from GitHub Releases ---
if ! $installed; then
  echo "[2/4] Trying direct download from GitHub Releases ..."
  if curl -fL --connect-timeout 15 --max-time 120 -o "${INSTALL_DIR}/zotron" "$GITHUB_URL" 2>/dev/null; then
    chmod +x "${INSTALL_DIR}/zotron"
    installed=true
  fi
fi

# --- Method 3: Mirror fallback (for users behind GFW) ---
if ! $installed; then
  MIRRORS=(
    "https://gh-proxy.com/${GITHUB_URL}"
    "https://gh.llkk.cc/${GITHUB_URL}"
    "https://hub.gitmirror.com/${GITHUB_URL}"
  )
  for mirror in "${MIRRORS[@]}"; do
    echo "[3/4] Trying mirror: ${mirror%/*} ..."
    if curl -fL --connect-timeout 15 --max-time 120 -o "${INSTALL_DIR}/zotron" "$mirror" 2>/dev/null; then
      chmod +x "${INSTALL_DIR}/zotron"
      installed=true
      break
    fi
  done
fi

# --- Method 4: cargo install (compile from source, last resort) ---
if ! $installed && command -v cargo >/dev/null 2>&1; then
  echo "[4/4] Trying cargo install zotron (compiling from source) ..."
  if cargo install zotron 2>/dev/null; then
    installed=true
  fi
fi

if $installed; then
  echo "zotron installed successfully: $(command -v zotron || echo "${INSTALL_DIR}/zotron")"
else
  echo "ERROR: All install methods failed. Please install manually from:"
  echo "  ${GITHUB_URL}"
  exit 1
fi
```

Make sure `~/.local/bin` is on PATH. If not, add it: `export PATH="$HOME/.local/bin:$PATH"`

2. Run the setup script to verify and bootstrap the XPI.

```bash
PLUGIN_ROOT="${CODEX_PLUGIN_ROOT:-${CLAUDE_PLUGIN_ROOT:-}}"
bash "$PLUGIN_ROOT/scripts/setup-zotron.sh"
```

3. If the bridge is live at the expected version, stop.

4. If the bridge is live but the plugin version is older, tell the user to update inside Zotero using the update flow above.

5. If the bridge is down, the script downloads or stages `zotron.xpi` into the user's real Downloads folder and prints the path as Zotero will see it.

6. Tell the user:

```text
In Zotero:
1. Tools -> Plugins
2. Gear icon -> Install Add-on From File
3. Choose zotron.xpi from the path printed by setup
4. Install, then restart Zotero
```

7. After restart, verify:

```bash
zotron ping
zotron system version
zotron --help  # should show all command groups
```

## Mirror Controls

```bash
ZOTRON_XPI_URLS='https://mirror.example/zotron.xpi https://github.com/dianzuan/zotron/releases/latest/download/zotron.xpi' \
  bash "$PLUGIN_ROOT/scripts/setup-zotron.sh"
```

Use `ZOTRON_XPI_PATH=/path/to/zotron.xpi` only when the file has already been downloaded through another channel.
