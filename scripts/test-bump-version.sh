#!/usr/bin/env bash
# Test suite for scripts/bump-version.sh
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
BUMP_SCRIPT="${SCRIPT_DIR}/bump-version.sh"
REPO_ROOT="$(cd -- "${SCRIPT_DIR}/.." && pwd)"

PASS=0
FAIL=0
ERRORS=""

# --- helpers -----------------------------------------------------------

setup_fixture() {
  local tmpdir
  tmpdir="$(mktemp -d)"

  # Recreate just the files the bump script touches, seeded at 0.1.1
  mkdir -p "${tmpdir}/addon"
  mkdir -p "${tmpdir}/src/handlers"
  mkdir -p "${tmpdir}/.claude-plugin"
  mkdir -p "${tmpdir}/plugin/.claude-plugin"
  mkdir -p "${tmpdir}/plugin/.codex-plugin"
  mkdir -p "${tmpdir}/plugin/scripts"
  mkdir -p "${tmpdir}/crates/zotron-cli"
  mkdir -p "${tmpdir}/crates/zotron-rpc"
  mkdir -p "${tmpdir}/crates/zotron-types"
  mkdir -p "${tmpdir}/scripts"

  # Copy real files from repo, then normalize to 0.1.1 via sed
  for f in package.json package-lock.json addon/manifest.json \
           src/handlers/system.ts update.json \
           .claude-plugin/marketplace.json \
           plugin/.claude-plugin/plugin.json \
           plugin/.codex-plugin/plugin.json \
           plugin/scripts/setup-zotron.sh \
           crates/zotron-cli/Cargo.toml \
           crates/zotron-rpc/Cargo.toml \
           crates/zotron-types/Cargo.toml; do
    cp "${REPO_ROOT}/${f}" "${tmpdir}/${f}"
  done

  # Copy the bump script itself into the fixture so it can find scripts/
  cp "${BUMP_SCRIPT}" "${tmpdir}/scripts/bump-version.sh"

  echo "${tmpdir}"
}

teardown_fixture() {
  rm -rf "$1"
}

assert_version_in() {
  local file="$1" expected="$2" label="$3"
  if grep -q "${expected}" "${file}"; then
    return 0
  else
    echo "  FAIL: ${label} — expected '${expected}' in ${file}"
    return 1
  fi
}

assert_no_version_in() {
  local file="$1" unwanted="$2" label="$3"
  if grep -q "${unwanted}" "${file}"; then
    echo "  FAIL: ${label} — found unwanted '${unwanted}' in ${file}"
    return 1
  else
    return 0
  fi
}

check_all_locations() {
  local dir="$1" ver="$2"
  local ok=true

  assert_version_in "${dir}/package.json" "\"version\": \"${ver}\"" "package.json" || ok=false
  assert_version_in "${dir}/package-lock.json" "\"version\": \"${ver}\"" "package-lock.json" || ok=false
  assert_version_in "${dir}/addon/manifest.json" "\"version\": \"${ver}\"" "addon/manifest.json" || ok=false
  assert_version_in "${dir}/src/handlers/system.ts" "plugin: \"${ver}\"" "system.ts" || ok=false
  assert_version_in "${dir}/update.json" "\"version\": \"${ver}\"" "update.json version" || ok=false
  assert_version_in "${dir}/update.json" "download/v${ver}/zotron.xpi" "update.json link" || ok=false
  assert_version_in "${dir}/plugin/.claude-plugin/plugin.json" "\"version\": \"${ver}\"" "claude-plugin" || ok=false
  assert_version_in "${dir}/plugin/.codex-plugin/plugin.json" "\"version\": \"${ver}\"" "codex-plugin" || ok=false
  assert_version_in "${dir}/plugin/scripts/setup-zotron.sh" "REQUIRED_VERSION:-${ver}}" "setup-zotron.sh" || ok=false

  # Cargo.toml: package version
  assert_version_in "${dir}/crates/zotron-cli/Cargo.toml" "version = \"${ver}\"" "zotron-cli version" || ok=false
  assert_version_in "${dir}/crates/zotron-rpc/Cargo.toml" "version = \"${ver}\"" "zotron-rpc version" || ok=false
  assert_version_in "${dir}/crates/zotron-types/Cargo.toml" "version = \"${ver}\"" "zotron-types version" || ok=false

  # Cargo.toml: dependency versions
  assert_version_in "${dir}/crates/zotron-cli/Cargo.toml" "zotron-rpc = { version = \"${ver}\"" "zotron-cli dep rpc" || ok=false
  assert_version_in "${dir}/crates/zotron-cli/Cargo.toml" "zotron-types = { version = \"${ver}\"" "zotron-cli dep types" || ok=false
  assert_version_in "${dir}/crates/zotron-rpc/Cargo.toml" "zotron-types = { version = \"${ver}\"" "zotron-rpc dep types" || ok=false

  $ok
}

run_test() {
  local name="$1"
  shift
  echo "--- ${name}"
  if "$@"; then
    PASS=$((PASS + 1))
    echo "  ok"
  else
    FAIL=$((FAIL + 1))
    ERRORS="${ERRORS}\n  - ${name}"
  fi
}

# --- tests -------------------------------------------------------------

test_patch_bump() {
  local dir
  dir="$(setup_fixture)"
  # Verify fixture starts at 0.1.1
  check_all_locations "${dir}" "0.1.1" || { teardown_fixture "${dir}"; return 1; }

  # Run patch bump (default, no arg)
  bash "${dir}/scripts/bump-version.sh" --root "${dir}" || { teardown_fixture "${dir}"; return 1; }
  check_all_locations "${dir}" "0.1.2" || { teardown_fixture "${dir}"; return 1; }

  # Old version must be gone
  assert_no_version_in "${dir}/package.json" "\"0.1.1\"" "old version gone" || { teardown_fixture "${dir}"; return 1; }

  teardown_fixture "${dir}"
}

test_minor_bump() {
  local dir
  dir="$(setup_fixture)"
  bash "${dir}/scripts/bump-version.sh" --root "${dir}" minor || { teardown_fixture "${dir}"; return 1; }
  check_all_locations "${dir}" "0.2.0" || { teardown_fixture "${dir}"; return 1; }
  teardown_fixture "${dir}"
}

test_major_bump() {
  local dir
  dir="$(setup_fixture)"
  bash "${dir}/scripts/bump-version.sh" --root "${dir}" major || { teardown_fixture "${dir}"; return 1; }
  check_all_locations "${dir}" "1.0.0" || { teardown_fixture "${dir}"; return 1; }
  teardown_fixture "${dir}"
}

test_double_patch_bump() {
  local dir
  dir="$(setup_fixture)"
  bash "${dir}/scripts/bump-version.sh" --root "${dir}" || { teardown_fixture "${dir}"; return 1; }
  bash "${dir}/scripts/bump-version.sh" --root "${dir}" || { teardown_fixture "${dir}"; return 1; }
  check_all_locations "${dir}" "0.1.3" || { teardown_fixture "${dir}"; return 1; }
  teardown_fixture "${dir}"
}

test_inconsistent_versions_fail() {
  local dir
  dir="$(setup_fixture)"
  # Introduce a version mismatch
  sed -i 's/"version": "0.1.1"/"version": "0.9.9"/' "${dir}/addon/manifest.json"
  if bash "${dir}/scripts/bump-version.sh" --root "${dir}" 2>/dev/null; then
    echo "  FAIL: should have failed on inconsistent versions"
    teardown_fixture "${dir}"
    return 1
  fi
  teardown_fixture "${dir}"
}

# --- run ---------------------------------------------------------------

echo "=== bump-version.sh test suite ==="
echo ""
run_test "patch bump (default)" test_patch_bump
run_test "minor bump" test_minor_bump
run_test "major bump" test_major_bump
run_test "double patch bump" test_double_patch_bump
run_test "inconsistent versions are rejected" test_inconsistent_versions_fail

echo ""
echo "=== Results: ${PASS} passed, ${FAIL} failed ==="
if [ "${FAIL}" -gt 0 ]; then
  echo -e "Failures:${ERRORS}"
  exit 1
fi
