<div align="center">

<img src="assets/logo.png" alt="Zotron logo" width="160" />

# Zotron

**Typed JSON-RPC 2.0 bridge for Zotero 8**

*81 internal API methods over HTTP — for AI agents, CLIs, and external tools.*

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![Zotero](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)
[![GitHub release](https://img.shields.io/github/v/release/dianzuan/zotron?color=brightgreen)](https://github.com/dianzuan/zotron/releases/latest)

[**English**](README.md) · [**简体中文**](README.zh-CN.md)

</div>

---

## What is this?

Zotron is a [bootstrap-extension](https://www.zotero.org/support/dev/zotero_7_for_developers) plugin that turns your running Zotero into a JSON-RPC 2.0 server. External tools — research agents, citation pipelines, scrapers, MCP servers, custom CLIs — read and write your library over plain HTTP without touching SQLite.

```
┌──────────────────────────┐         ┌─────────────────────────────┐
│  Your tool / agent       │         │  Zotero (with this plugin)  │
│                          │         │                             │
│  curl /zotron/rpc        │ ──HTTP─▶│  86 typed RPC methods       │
│  cnki-plugin push        │         │  • items.* (21)             │
│  research agent          │         │  • collections.* (12)       │
│  Better-BibTeX consumer  │         │  • attachments.* (8)        │
│  …                       │         │  • notes.* (5)              │
│                          │         │  • annotations.* (3)        │
│                          │         │  • search.* (8)             │
│                          │         │  • tags.* (6)               │
│                          │         │  • export.* (5)             │
│                          │         │  • settings.* (4)           │
│                          │         │  • system.* (13)            │
└──────────────────────────┘         └─────────────────────────────┘
```

Validated on Zotero 8.0.4 against a 5000+-item / 70+-collection library. Zotero 7 not yet verified.

## Why not the official Zotero Local API?

Zotero 7 shipped its own [Local API](https://www.zotero.org/support/dev/web_api/v3/start) at `localhost:23119/api/` — a local port of the cloud Web API. If your client already speaks `api.zotero.org` (`pyzotero`, web-API-compatible plugins), point it at `/api/` and you're done. Zotron isn't trying to replace that.

But the Local API is read-heavy and schema-locked to what `api.zotero.org` exposes. For agents and tooling, the gap shows up fast:

| | Zotero Local API (`/api/`) | Zotron (`/zotron/rpc`) |
|---|---|---|
| Read items, collections, tags, annotations | ✅ | ✅ |
| **Add by DOI / URL / ISBN / file (translator-backed)** | ❌ | ✅ |
| **Dedupe, hierarchical collection ops, batch retag** | partial | ✅ |
| **Fulltext cache (`getCachedFile`), embedded relations** | ❌ | ✅ |
| **Current selection, switch library, trigger sync, plugin reload** | ❌ | ✅ |
| **CSL bibliography in arbitrary installed style (full CiteProc)** | partial | ✅ |
| Compatible with `pyzotero` / Web-API clients out-of-box | ✅ | ❌ (custom RPC) |
| Requires the "Allow other apps" checkbox | yes | **no** (plugin endpoints bypass that gate) |

Zotron is a typed JSON-RPC bridge to Zotero's **internal JS API** — the same surface plugins themselves use, with no Web-API schema translation layer between you and the data. 86 methods across 11 namespaces (CRUD + search + export + tags + sync + RAG + system).

Pre-Zotero-7 alternatives — vendoring a SQLite reader (fragile, write-locked, schema-versioned), `eval`-ing JS through the debug-server backdoor (insecure, unsupported), or hand-rolling a one-off bootstrap plugin per project (rebuilds the wheel) — are all bad. Zotron replaces them with one stable typed surface.

## Quick start

### Path A — Claude Code (recommended)

**Prerequisites:** [Claude Code](https://docs.claude.com/en/docs/claude-code/), [`uv`](https://docs.astral.sh/uv/getting-started/installation/), Zotero 8 desktop.

```
/plugin marketplace add dianzuan/zotron
/plugin install zotron@zotron
/zotron:setup
```

`/zotron:setup` pings the bridge. If the XPI is missing, it downloads the release `zotron.xpi` to your real Downloads folder (auto-detected, handles drive relocation like `E:\Downloads` on Windows, OneDrive redirect, and POSIX defaults), trying GitHub first and then configured mirror URLs. If the XPI is installed but older than the setup target, it tells you to use Zotero's built-in add-on update flow instead of reinstalling. Then talk to Claude in plain English — *"find papers on transformer attention"*, *"add DOI 10.1038/nature12373 to my ML collection"*, *"export APA references for items 10, 13, 16"*. Claude routes to the right sub-workflow (search / manage / export / OCR / RAG), which calls the RPC.

### Path B — OpenAI Codex CLI / code-cli

Use this path when you work from Codex instead of Claude Code. The same `claude-plugin/` package also ships a native Codex plugin manifest, so Codex and Claude Code use the same bridge, Python CLI, XPI, and skills.

**Prerequisites:** OpenAI Codex CLI (`codex`; some environments label it `code-cli`), [`uv`](https://docs.astral.sh/uv/getting-started/installation/), Zotero 8 desktop.

```bash
# 1) Install Codex CLI if it is not already available.
npm install -g @openai/codex

# 2) Add the Zotron plugin marketplace.
codex plugin marketplace add dianzuan/zotron

# Local checkout alternative:
# codex plugin marketplace add .
```

Then install **Zotron** from Codex's plugin UI and invoke the setup skill:

```text
$zotron-setup
```

The setup skill exposes the bundled `zotron`, `zotron-rag`, and `zotron-ocr` shims, downloads release `zotron.xpi` into your Downloads folder when needed, and walks you through Zotero's native **Tools → Plugins → ⚙ → Install Add-on From File → restart** flow. The repository does not track generated XPI files; releases are the install source. Set `ZOTRON_XPI_URLS` to a whitespace/comma/semicolon-separated mirror list when GitHub is not reachable.

After Zotero restarts:

```bash
zotron ping
zotron search quick "transformer attention" --limit 10
```

After `zotron ping` succeeds, Codex can call `zotron`, `zotron-rag`, `zotron-ocr`, or raw HTTP directly through the installed plugin skill.

### Path C — Python CLI / SDK

```bash
# 1) Install the XPI manually from https://github.com/dianzuan/zotron/releases/latest
# 2) Install the CLI from git (not yet on PyPI):
uv tool install "git+https://github.com/dianzuan/zotron.git#subdirectory=claude-plugin/python"

zotron ping
zotron search quick "transformer attention" --limit 10
zotron rpc items.get '{"key":"YR5BUGHG"}'  # escape hatch — covers all 86 methods
```

`--jq` filters output (`gh api --jq` style); `--install-completion {bash|zsh|fish|powershell}` enables shell completion. SDK contract: [`docs/api-stability.md`](docs/api-stability.md).

### Path D — Raw HTTP

```bash
curl -s -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":1}'
```

### Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `/zotron:setup` says `MISSING_UV` | `uv` not on PATH | `curl -LsSf https://astral.sh/uv/install.sh \| sh` |
| Skill startup banner: *"Zotron not detected"* | Zotero not running or XPI not installed | Start Zotero, then re-run `/zotron:setup` |
| `connection refused` on port 23119 | Zotero's built-in HTTP server is off | Edit → Settings → Advanced → Config Editor → `extensions.zotero.httpServer.enabled = true` |
| Skill doesn't auto-trigger after install | Plugin not loaded into the session | `/reload-plugins`, or restart Claude Code |
| `zotron: command not found` from Bash tool | Plugin's `bin/` not on PATH | Plugin must be enabled — check the **Installed** tab in `/plugin` |

## API surface

86 methods across 11 namespaces. Full conventions: [docs/superpowers/specs/2026-04-23-xpi-api-prd.md](docs/superpowers/specs/2026-04-23-xpi-api-prd.md).

| Namespace | Methods | What it does |
|---|---|---|
| `items.*` | 21 | CRUD, list, fulltext, add by DOI/URL/ISBN/file, recent, trash, duplicates, related, citation key |
| `collections.*` | 12 | List, get, tree, create, rename, move, delete, items, subcollections, stats |
| `attachments.*` | 8 | List, get, fulltext, add, add-by-URL, path, delete, find PDF |
| `notes.*` | 5 | List by parent, get single, create, update, search |
| `annotations.*` | 3 | List, create, delete PDF annotations |
| `search.*` | 8 | Quick / fulltext / by-tag / by-identifier / advanced; saved searches |
| `tags.*` | 6 | List, add, remove, rename, delete, batch update |
| `export.*` | 5 | BibTeX / CSL-JSON / RIS / CSV / bibliography (CiteProc) |
| `settings.*` | 4 | Plugin-side preferences (OCR provider, embedding model) |
| `system.*` | 13 | Ping, version, libraries, switchLibrary, sync, currentCollection, listMethods, describe, reload |

**Conventions:** Responses are **key-first** — item and collection objects use `key` (8-char alphanumeric, Zotero Web API v3 aligned) as the primary identifier; numeric `id` is not exposed. Items include a `version` field for sync. Mutation returns use `{ok: true, key}`. Pagination uses `{items, total, offset?, limit?}` envelope. Lowercase `libraryId` on the wire. All parameters that accept item/collection identifiers take either a numeric ID or a key string. Unknown method calls get fuzzy "Did you mean?" suggestions. Errors are JSON-RPC 2.0 `{code, message}` (`-32602` caller error, `-32603` server error). `items.create` auto-splits Chinese full names — `欧阳修` → `{lastName: "欧阳", firstName: "修"}` — covering 70+ compound surnames.

## RAG with citations

The RAG layer (`claude-plugin/python/zotron/rag/`) returns each retrieved chunk as a `Citation` carrying the Zotero item key, attachment id, section heading, chunk index, similarity score, verbatim text, and a `zotero://` URI for one-click verification.

```bash
zotron-rag index --collection "ML Papers"
zotron-rag cite "how do transformers attend to long-range context?" --collection "ML Papers" --output json
```

`--output json` is the AI-facing stable contract:

```json
{ "itemKey": "ABC123", "attachmentKey": "ATT42XY", "title": "...", "authors": "...",
  "section": "Section 3 — The Model", "chunkIndex": 7, "text": "...",
  "score": 0.87, "zoteroUri": "zotero://select/library/items/ABC123" }
```

The 2026 RAG/OCR roadmap extends this toward Zotero-native artifacts and an academic-zh friendly JSONL hit stream. The stable target is:

- provider raw evidence in `<item-key>.zotron-ocr.raw.zip`;
- normalized OCR/parser blocks in `<item-key>.zotron-blocks.jsonl`;
- retrieval chunks in `<item-key>.zotron-chunks.jsonl`;
- vectors plus index metadata in `<item-key>.zotron-embed.npz`;
- retrieval hits as one JSON object per line with required `item_key`, `title`, and `text`, plus provenance fields such as `zotero_uri`, `chunk_id`, `block_ids`, `section_heading`, `query`, and `score`. The XPI exposes `rag.searchHits` / `rag.searchCards` for Zotero-native chunk artifact lookup; `zotron-rag hits --zotero` calls that JSON-RPC backend.

Markdown is allowed as a derived convenience output, but it is not the source of truth for OCR/RAG because it loses page, bbox, table, figure, provider, and reading-order provenance.

## Development

Node 18+, Zotero 8 installed locally. (WSL recommended on Windows.)

```bash
npm install
npm test           # 127 mocha unit tests
npm run build      # type-check + bundle + emit XPI to .scaffold/build/
```

Hot-reload: `ZOTERO_PLUGIN_ZOTERO_BIN_PATH=/path/to/zotero npm start`. On WSL, scaffold's RDP reload is broken across OS boundaries — use the bundled `system.reload` RPC after `rsync`-ing the built addon to your dev profile:

```bash
npm run build && \
  rsync -a --delete .scaffold/build/addon/ "$DEV_ADDON_DIR" && \
  curl -s -X POST http://localhost:23119/zotron/rpc \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"system.reload","id":1}'
```

## Roadmap

Preference keys reserved in `SETTINGS_KEYS` (callable via `settings.set`); consumer methods not yet implemented:

- `ocr.*` — for a future `attachments.ocr` method
- `embedding.*` — semantic search / chunking
- `rag.searchHits` / `rag.searchCards` — Zotero-native retrieval hits over attached chunk artifacts

See [`docs/2026-04-27-rag-ocr-roadmap.md`](docs/2026-04-27-rag-ocr-roadmap.md) for the current storage and retrieval contract. First-class RAG/OCR work should preserve provider raw outputs, normalize to blocks/chunks, and expose academic-zh compatible retrieval hits without treating markdown as the only truth.

PRs welcome. New RPC methods need a mocha test using `test/fixtures/zotero-mock.ts`.

## License

[AGPL-3.0-or-later](LICENSE). For closed-source use, open an issue to discuss commercial licensing.

## Acknowledgments

- [Zotero](https://www.zotero.org/) by the Corporation for Digital Scholarship (AGPL-3.0)
- [`zotero-plugin-toolkit`](https://github.com/windingwind/zotero-plugin-toolkit) by windingwind (MIT)
- [`zotero-plugin-scaffold`](https://github.com/zotero-plugin-dev/zotero-plugin-scaffold) (AGPL-3.0)
- [`zotero-types`](https://github.com/windingwind/zotero-types) (MIT)
- Inspired by [`Jasminum`](https://github.com/l0o0/jasminum) (AGPL-3.0) — Chinese academic metadata for Zotero
- The Zotero plugin community (Knowledge4Zotero, zotero-pdf-translate, zotero-actions-tags, zotero-style — all AGPL-3.0)
