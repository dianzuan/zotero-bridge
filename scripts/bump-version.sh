#!/usr/bin/env bash
# Bump the zotron version across ALL locations.
#
# Usage:
#   scripts/bump-version.sh           # patch bump (default): X.Y.Z → X.Y.Z+1
#   scripts/bump-version.sh minor     # minor bump:           X.Y.Z → X.Y+1.0
#   scripts/bump-version.sh major     # major bump:           X.Y.Z → X+1.0.0
#
# Reads the current version from package.json, verifies all locations agree,
# computes the new version, and updates every file.

set -euo pipefail

# --- parse args --------------------------------------------------------

ROOT=""
BUMP_TYPE="patch"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root)
      ROOT="$2"
      shift 2
      ;;
    patch|minor|major)
      BUMP_TYPE="$1"
      shift
      ;;
    *)
      echo "Usage: $0 [--root <dir>] [patch|minor|major]" >&2
      exit 1
      ;;
  esac
done

if [ -z "${ROOT}" ]; then
  ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
fi

# --- read current version from package.json ---------------------------

CURRENT="$(sed -n 's/.*"version": "\([0-9]*\.[0-9]*\.[0-9]*\)".*/\1/p' "${ROOT}/package.json" | head -n 1)"
if [ -z "${CURRENT}" ]; then
  echo "ERROR: could not read version from package.json" >&2
  exit 1
fi

IFS='.' read -r MAJOR MINOR PATCH <<< "${CURRENT}"

# --- verify consistency across all locations ---------------------------

ERRORS=0

check() {
  local label="$1" file="$2" pattern="$3"
  if [ ! -f "${file}" ]; then
    echo "ERROR: ${label} — file not found: ${file}" >&2
    ERRORS=$((ERRORS + 1))
    return
  fi
  if ! grep -qF "${pattern}" "${file}"; then
    echo "ERROR: ${label} — expected '${pattern}' in ${file}" >&2
    ERRORS=$((ERRORS + 1))
  fi
}

check "package.json" \
  "${ROOT}/package.json" \
  "\"version\": \"${CURRENT}\""

check "package-lock.json (top)" \
  "${ROOT}/package-lock.json" \
  "\"version\": \"${CURRENT}\""

check "addon/manifest.json" \
  "${ROOT}/addon/manifest.json" \
  "\"version\": \"${CURRENT}\""

check "system.ts" \
  "${ROOT}/src/handlers/system.ts" \
  "plugin: \"${CURRENT}\""

check "update.json version" \
  "${ROOT}/update.json" \
  "\"version\": \"${CURRENT}\""

check "update.json link" \
  "${ROOT}/update.json" \
  "download/v${CURRENT}/zotron.xpi"

check "plugin .claude-plugin/plugin.json" \
  "${ROOT}/plugin/.claude-plugin/plugin.json" \
  "\"version\": \"${CURRENT}\""

check "codex-plugin plugin.json" \
  "${ROOT}/plugin/.codex-plugin/plugin.json" \
  "\"version\": \"${CURRENT}\""

check "setup-zotron.sh" \
  "${ROOT}/plugin/scripts/setup-zotron.sh" \
  "REQUIRED_VERSION:-${CURRENT}}"

check "zotron-cli Cargo.toml" \
  "${ROOT}/crates/zotron-cli/Cargo.toml" \
  "version = \"${CURRENT}\""

check "zotron-rpc Cargo.toml" \
  "${ROOT}/crates/zotron-rpc/Cargo.toml" \
  "version = \"${CURRENT}\""

check "zotron-types Cargo.toml" \
  "${ROOT}/crates/zotron-types/Cargo.toml" \
  "version = \"${CURRENT}\""

check "zotron-cli dep rpc" \
  "${ROOT}/crates/zotron-cli/Cargo.toml" \
  "zotron-rpc = { version = \"${CURRENT}\""

check "zotron-cli dep types" \
  "${ROOT}/crates/zotron-cli/Cargo.toml" \
  "zotron-types = { version = \"${CURRENT}\""

check "zotron-rpc dep types" \
  "${ROOT}/crates/zotron-rpc/Cargo.toml" \
  "zotron-types = { version = \"${CURRENT}\""

if [ "${ERRORS}" -gt 0 ]; then
  echo "ABORT: ${ERRORS} version location(s) are inconsistent — fix them before bumping." >&2
  exit 1
fi

# --- compute new version -----------------------------------------------

case "${BUMP_TYPE}" in
  patch) NEW="${MAJOR}.${MINOR}.$((PATCH + 1))" ;;
  minor) NEW="${MAJOR}.$((MINOR + 1)).0" ;;
  major) NEW="$((MAJOR + 1)).0.0" ;;
esac

echo "Bumping ${CURRENT} → ${NEW} (${BUMP_TYPE})"

# --- apply updates ------------------------------------------------------

# Helper: portable in-place sed (macOS vs GNU)
sedi() {
  if sed --version >/dev/null 2>&1; then
    sed -i "$@"
  else
    sed -i '' "$@"
  fi
}

# JSON files: replace version string
for f in package.json package-lock.json addon/manifest.json \
         plugin/.claude-plugin/plugin.json \
         plugin/.codex-plugin/plugin.json \
         .claude-plugin/marketplace.json; do
  sedi "s/\"version\": \"${CURRENT}\"/\"version\": \"${NEW}\"/g" "${ROOT}/${f}"
done

# system.ts
sedi "s/plugin: \"${CURRENT}\"/plugin: \"${NEW}\"/" "${ROOT}/src/handlers/system.ts"

# update.json: version AND update_link
for f in update.json; do
  sedi "s/\"version\": \"${CURRENT}\"/\"version\": \"${NEW}\"/g" "${ROOT}/${f}"
  sedi "s|download/v${CURRENT}/zotron.xpi|download/v${NEW}/zotron.xpi|g" "${ROOT}/${f}"
done

# setup-zotron.sh
sedi "s/REQUIRED_VERSION:-${CURRENT}}/REQUIRED_VERSION:-${NEW}}/" "${ROOT}/plugin/scripts/setup-zotron.sh"

# Cargo.toml: package versions AND dependency versions
for f in crates/zotron-cli/Cargo.toml crates/zotron-rpc/Cargo.toml crates/zotron-types/Cargo.toml; do
  sedi "s/version = \"${CURRENT}\"/version = \"${NEW}\"/g" "${ROOT}/${f}"
done

echo "Done. All locations updated to ${NEW}."
