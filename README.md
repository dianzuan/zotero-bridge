<div align="center">

<img src="assets/logo.png" alt="Zotron logo" width="160" />

# Zotron

**Typed JSON-RPC 2.0 bridge for Zotero 8**

*77 internal API methods over HTTP — for AI agents, CLIs, and external tools.*

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
│  curl /zotron/rpc        │ ──HTTP─▶│  77 typed RPC methods       │
│  cnki-plugin push        │         │  • items.* (19)             │
│  research agent          │         │  • collections.* (12)       │
│  Better-BibTeX consumer  │         │  • attachments.* (6)        │
│  …                       │         │  • notes.* (6)              │
│                          │         │  • search.* (8)             │
│                          │         │  • tags.* (6)               │
│                          │         │  • export.* (5)             │
│                          │         │  • settings.* (4)           │
│                          │         │  • system.* (11)            │
└──────────────────────────┘         └─────────────────────────────┘
```

Validated on Zotero 8.0.4 against a 5000+-item / 70+-collection library. Zotero 7 not yet verified.

## Why?

Zotero's built-in `localhost:23119` HTTP service is hardcoded for the browser-extension use case (a handful of endpoints like `/connector/getSelectedCollection`) — not a general-purpose API. The workarounds — vendor a SQLite reader (fragile, schema-versioned, write-locked), `eval` JS through a debug-server backdoor (insecure, unsupported), or hand-roll a bootstrap plugin per project (rebuilds the wheel) — are all bad. Zotron fills that gap with a single stable typed surface any tool can target.

## Quick start

### Path A — Claude Code (recommended)

**Prerequisites:** [Claude Code](https://docs.claude.com/en/docs/claude-code/), [`uv`](https://docs.astral.sh/uv/getting-started/installation/), Zotero 8 desktop.

```
/plugin marketplace add dianzuan/zotron
/plugin install zotron@zotron
/zotron:setup
```

`/zotron:setup` pings the bridge; if Zotero is missing the XPI, it fetches the latest release, downloads it, and walks you through **Tools → Plugins → ⚙ → Install From File → restart**. Then talk to Claude in plain English — *"find papers on transformer attention"*, *"add DOI 10.1038/nature12373 to my ML collection"*, *"export APA references for items 10, 13, 16"*. Claude routes to the right sub-workflow (search / manage / export / OCR / RAG), which calls the RPC.

### Path B — Python CLI / SDK

```bash
# 1) Install the XPI manually from https://github.com/dianzuan/zotron/releases/latest
# 2) Install the CLI from git (not yet on PyPI):
uv tool install "git+https://github.com/dianzuan/zotron.git#subdirectory=claude-plugin/python"

zotron ping
zotron search quick "transformer attention" --limit 10
zotron rpc items.get '{"id":12345}'    # escape hatch — covers all 77 methods
```

`--jq` filters output (`gh api --jq` style); `--install-completion {bash|zsh|fish|powershell}` enables shell completion. SDK contract: [`docs/api-stability.md`](docs/api-stability.md).

### Path C — Raw HTTP

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

77 methods across 9 namespaces. Full conventions: [docs/superpowers/specs/2026-04-23-xpi-api-prd.md](docs/superpowers/specs/2026-04-23-xpi-api-prd.md).

| Namespace | Methods | What it does |
|---|---|---|
| `items.*` | 19 | CRUD on Zotero items, add by DOI/URL/ISBN/file, recent, trash, duplicates, related |
| `collections.*` | 12 | List, create, rename, move, tree, items in collection |
| `attachments.*` | 6 | List attachments, get fulltext (cache-file backed), get path, find PDF |
| `notes.*` | 6 | Notes CRUD, annotations, search inside notes |
| `search.*` | 8 | Quick / fulltext / by-tag / by-identifier / advanced; saved searches |
| `tags.*` | 6 | List, add, remove, rename, delete (cross-library) |
| `export.*` | 5 | BibTeX / CSL-JSON / RIS / CSV / bibliography (CiteProc) |
| `settings.*` | 4 | Plugin-side preferences (e.g. OCR provider, embedding model) |
| `system.*` | 11 | Ping, version, libraries, switchLibrary, sync, currentCollection, **`system.reload`** (self-reload for dev) |

**Conventions:** return shapes follow PRD §2 — `serializeItem(item)` for item-bearing returns, `{items, total, offset?, limit?}` envelope for pagination, lowercase `libraryId` on the wire. Errors are JSON-RPC 2.0 `{code, message}` (`-32602` caller error, `-32603` server error). `items.create` auto-splits Chinese full names — `欧阳修` → `{lastName: "欧阳", firstName: "修"}` — covering 70+ compound surnames.

## RAG with citations

The RAG layer (`claude-plugin/python/zotron/rag/`) returns each retrieved chunk as a `Citation` carrying the Zotero item key, attachment id, section heading, chunk index, similarity score, verbatim text, and a `zotero://` URI for one-click verification.

```bash
zotron-rag index --collection "ML Papers"
zotron-rag cite "how do transformers attend to long-range context?" --collection "ML Papers" --output json
```

`--output json` is the AI-facing stable contract:

```json
{ "itemKey": "ABC123", "attachmentId": 42, "title": "...", "authors": "...",
  "section": "Section 3 — The Model", "chunkIndex": 7, "text": "...",
  "score": 0.87, "zoteroUri": "zotero://select/library/items/ABC123" }
```

## Development

Node 18+, Zotero 8 installed locally. (WSL recommended on Windows.)

```bash
npm install
npm test           # 99 mocha unit tests
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
- `rag.*` — for `search.semantic`

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
