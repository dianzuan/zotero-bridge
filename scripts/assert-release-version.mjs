#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const expected = process.argv[2] ?? null;
const versionPattern = /^\d+\.\d+\.\d+$/;
const tagPattern = /^v\d+\.\d+\.\d+$/;

function read(relativePath) {
  return fs.readFileSync(path.join(root, relativePath), "utf8");
}

function readJson(relativePath) {
  return JSON.parse(read(relativePath));
}

function fail(message) {
  console.error(`release version check failed: ${message}`);
  process.exitCode = 1;
}

function requireVersion(label, value) {
  if (!value) {
    fail(`${label} is missing`);
    return;
  }
  if (!versionPattern.test(value)) {
    fail(`${label} must be x.y.z only, got ${value}`);
  }
}

const versions = new Map();
versions.set("package.json", readJson("package.json").version);
versions.set("package-lock.json", readJson("package-lock.json").version);
versions.set("package-lock.json packages[\"\"]", readJson("package-lock.json").packages?.[""]?.version);
versions.set("addon/manifest.json", readJson("addon/manifest.json").version);

const pyproject = read("claude-plugin/python/pyproject.toml");
versions.set("claude-plugin/python/pyproject.toml", pyproject.match(/^version = "([^"]+)"$/m)?.[1]);

const systemHandler = read("src/handlers/system.ts");
versions.set("src/handlers/system.ts", systemHandler.match(/plugin:\s*"([^"]+)"/)?.[1]);

const setupScript = read("claude-plugin/scripts/setup-zotron.sh");
versions.set(
  "claude-plugin/scripts/setup-zotron.sh",
  setupScript.match(/REQUIRED_VERSION="\$\{ZOTRON_REQUIRED_VERSION:-([^}]+)\}"/)?.[1],
);

for (const [label, version] of versions) {
  requireVersion(label, version);
}

const uniqueVersions = new Set([...versions.values()]);
if (uniqueVersions.size !== 1) {
  fail(`version sources disagree: ${JSON.stringify(Object.fromEntries(versions))}`);
}

const releaseVersion = [...uniqueVersions][0];
if (expected !== null && expected !== releaseVersion) {
  fail(`expected ${expected}, got ${releaseVersion}`);
}

for (const tag of [`v${releaseVersion}`, process.env.GITHUB_REF_NAME].filter(Boolean)) {
  if (!tagPattern.test(tag)) {
    fail(`release tag must be vX.Y.Z only, got ${tag}`);
  }
}

for (const relativePath of ["update.json", "update-beta.json"]) {
  if (!fs.existsSync(path.join(root, relativePath))) {
    continue;
  }
  const updates = readJson(relativePath).addons?.["zotron@diamondrill"]?.updates;
  if (!Array.isArray(updates) || updates.length !== 1) {
    fail(`${relativePath} must contain exactly one update entry`);
    continue;
  }
  const update = updates[0];
  requireVersion(`${relativePath} update version`, update.version);
  if (update.version !== releaseVersion) {
    fail(`${relativePath} update version ${update.version} does not match ${releaseVersion}`);
  }
  const expectedLink = `https://github.com/dianzuan/zotron/releases/download/v${releaseVersion}/zotron.xpi`;
  if (update.update_link !== expectedLink) {
    fail(`${relativePath} update_link must be ${expectedLink}, got ${update.update_link}`);
  }
}

if (!process.exitCode) {
  console.log(`release version ok: ${releaseVersion}`);
}
