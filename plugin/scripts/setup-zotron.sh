#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_ROOT="${CODEX_PLUGIN_ROOT:-${CLAUDE_PLUGIN_ROOT:-}}"
if [ -z "${PLUGIN_ROOT}" ]; then
  PLUGIN_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"
fi

BIN_DIR="${HOME}/.local/bin"
mkdir -p "${BIN_DIR}"

# CLI check — this script only handles the XPI. CLI install is in the setup SKILL.md cascade.
if ! command -v zotron >/dev/null 2>&1; then
  echo "MISSING_ZOTRON"
  echo "The zotron CLI is not installed. Run /zotron:setup to install it, or download from:"
  echo "  https://github.com/dianzuan/zotron/releases/latest"
  exit 1
fi

case ":${PATH}:" in
  *":${BIN_DIR}:"*) ;;
  *)
    echo "PATH_HINT: add ${BIN_DIR} to PATH if zotron is not found in new shells."
    ;;
esac

REQUIRED_VERSION="${ZOTRON_REQUIRED_VERSION:-0.2.5}"
GITHUB_XPI_URL="https://github.com/dianzuan/zotron/releases/download/v${REQUIRED_VERSION}/zotron.xpi"
DEFAULT_XPI_URLS="${GITHUB_XPI_URL}
https://gh-proxy.com/${GITHUB_XPI_URL}
https://gh.llkk.cc/${GITHUB_XPI_URL}
https://hub.gitmirror.com/${GITHUB_XPI_URL}"

ZOTRON_CLI="$(command -v zotron)"
TMPDIR_SETUP="$(mktemp -d)"
trap 'rm -rf "${TMPDIR_SETUP}"' EXIT
if "${ZOTRON_CLI}" ping >"${TMPDIR_SETUP}/ping.json" 2>"${TMPDIR_SETUP}/ping.err"; then
  cat "${TMPDIR_SETUP}/ping.json"
  if "${ZOTRON_CLI}" system version >"${TMPDIR_SETUP}/version.json" 2>"${TMPDIR_SETUP}/version.err"; then
    INSTALLED_VERSION="$(sed -n 's/.*"plugin"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "${TMPDIR_SETUP}/version.json" | head -n 1)"
    if [ "${INSTALLED_VERSION}" = "${REQUIRED_VERSION}" ]; then
      echo "Zotron bridge is live at version ${INSTALLED_VERSION}."
      exit 0
    fi
    echo "Zotron bridge is live, but plugin version is ${INSTALLED_VERSION:-unknown}; expected ${REQUIRED_VERSION}."
    echo
    echo "Update inside Zotero instead of reinstalling:"
    echo "1. Tools -> Plugins"
    echo "2. Find Zotron"
    echo "3. Gear icon -> Check for Updates"
    echo "4. Restart Zotero after the update"
    exit 0
  fi
  echo "Zotron bridge is live, but version check failed. Run this manually:"
  echo "zotron system version"
  exit 0
fi

stage_xpi() {
  local target="$1"

  if [ -n "${ZOTRON_XPI_PATH:-}" ]; then
    if [ ! -f "${ZOTRON_XPI_PATH}" ]; then
      echo "LOCAL_XPI_MISSING: ${ZOTRON_XPI_PATH}"
      exit 1
    fi
    cp "${ZOTRON_XPI_PATH}" "${target}"
    echo "XPI_SOURCE: local ${ZOTRON_XPI_PATH}"
    return 0
  fi

  local normalized
  normalized="$(printf '%s\n' "${ZOTRON_XPI_URLS:-${DEFAULT_XPI_URLS}}" | tr ',; ' '\n')"
  while IFS= read -r url; do
    url="$(printf '%s' "${url}" | xargs)"
    [ -z "${url}" ] && continue
    echo "Downloading Zotron XPI from ${url}"
    if command -v curl >/dev/null 2>&1; then
      if curl -fL --retry 2 --connect-timeout 10 -o "${target}.tmp" "${url}"; then
        mv "${target}.tmp" "${target}"
        echo "XPI_SOURCE: ${url}"
        return 0
      fi
    elif command -v wget >/dev/null 2>&1; then
      if wget -O "${target}.tmp" "${url}"; then
        mv "${target}.tmp" "${target}"
        echo "XPI_SOURCE: ${url}"
        return 0
      fi
    else
      echo "MISSING_DOWNLOADER: install curl or wget, or set ZOTRON_XPI_PATH=/path/to/zotron.xpi"
      exit 1
    fi
    rm -f "${target}.tmp"
  done <<< "${normalized}"

  echo "XPI_DOWNLOAD_FAILED"
  echo "Tried URLs:"
  printf '%s\n' "${normalized}" | sed '/^[[:space:]]*$/d'
  echo
  echo "Use one of:"
  echo "  ZOTRON_XPI_PATH=/path/to/zotron.xpi bash ${SCRIPT_DIR}/setup-zotron.sh"
  echo "  ZOTRON_XPI_URLS='https://your-mirror.example/zotron.xpi ${GITHUB_XPI_URL}' bash ${SCRIPT_DIR}/setup-zotron.sh"
  exit 1
}

case "$(uname -s)" in
  Darwin*)
    DOWNLOADS="${HOME}/Downloads"
    DISPLAY_PATH="${DOWNLOADS}/zotron.xpi"
    ;;
  Linux*)
    if grep -qi microsoft /proc/version 2>/dev/null && ! command -v zotero >/dev/null 2>&1 && command -v powershell.exe >/dev/null 2>&1; then
      WIN_DL="$(powershell.exe -NoProfile -Command "(New-Object -ComObject Shell.Application).NameSpace('shell:Downloads').Self.Path" 2>/dev/null | tr -d '\r\n')"
      if [ -n "${WIN_DL}" ] && command -v wslpath >/dev/null 2>&1; then
        DOWNLOADS="$(wslpath -u "${WIN_DL}")"
        DISPLAY_PATH="${WIN_DL}\\zotron.xpi"
      else
        DOWNLOADS="${HOME}/Downloads"
        DISPLAY_PATH="${DOWNLOADS}/zotron.xpi"
      fi
    else
      DOWNLOADS="$(xdg-user-dir DOWNLOAD 2>/dev/null || echo "${HOME}/Downloads")"
      DISPLAY_PATH="${DOWNLOADS}/zotron.xpi"
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    WIN_DL="$(powershell.exe -NoProfile -Command "(New-Object -ComObject Shell.Application).NameSpace('shell:Downloads').Self.Path" 2>/dev/null | tr -d '\r\n')"
    DOWNLOADS="$(cygpath -u "${WIN_DL}")"
    DISPLAY_PATH="${WIN_DL}\\zotron.xpi"
    ;;
  *)
    DOWNLOADS="${HOME}/Downloads"
    DISPLAY_PATH="${DOWNLOADS}/zotron.xpi"
    ;;
esac

mkdir -p "${DOWNLOADS}"
stage_xpi "${DOWNLOADS}/zotron.xpi"

echo "Zotron bridge is not live yet."
if [ -s "${TMPDIR_SETUP}/ping.err" ]; then
  echo "Last ping error:"
  sed -n '1,12p' "${TMPDIR_SETUP}/ping.err"
fi
echo
echo "XPI placed at:"
echo "${DISPLAY_PATH}"
echo
echo "XPI source controls:"
echo "  ZOTRON_XPI_URLS='https://mirror.example/zotron.xpi ${GITHUB_XPI_URL}'"
echo "  ZOTRON_XPI_PATH='/path/to/zotron.xpi'  # predownloaded local file"
echo
echo "In Zotero:"
echo "1. Tools -> Plugins"
echo "2. Gear icon -> Install Add-on From File"
echo "3. Choose zotron.xpi from the path above"
echo "4. Install, then restart Zotero"
echo
echo "After restart, verify with:"
echo "zotron ping"
