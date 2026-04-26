<div align="center">

<img src="assets/logo.png" alt="Zotero Bridge logo" width="160" />

# Zotero Bridge

**Typed JSON-RPC 2.0 bridge for Zotero 8**

*77 internal API methods over HTTP — for AI agents, CLIs, and external tools.*

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![CI](https://github.com/dianzuan/zotero-bridge/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotero-bridge/actions/workflows/ci.yml)
[![Zotero](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)
[![GitHub release](https://img.shields.io/github/v/release/dianzuan/zotero-bridge?color=brightgreen)](https://github.com/dianzuan/zotero-bridge/releases/latest)

[**English**](README.md) · [**简体中文**](README.zh-CN.md)

</div>

---

## ✨ Highlights

- **1 umbrella Claude Code skill** — covers 5 workflows (search / manage / export / OCR / RAG) via progressive disclosure; AI reads and writes your library on your behalf
- **77 typed RPC methods** under the hood across 9 namespaces — any client (Python, curl, MCP server, …) can target the same API
- **Python CLI + SDK** — typer-based, with `--jq` filtering, `--paginate` auto-loop, `--dry-run` preview, shell completion
- **RAG with citation provenance** — every retrieved chunk carries a `zotero://` URI for one-click traceback
- **Tested on Zotero 8.0.4** — Zotero 7 not yet verified
- **AGPL-3.0** — fully open source

## 📑 Contents

- [What is this?](#what-is-this)
- [Why?](#why)
- [Quick start](#quick-start)
- [API surface](#api-surface)
- [Development](#development)
- [RAG with Citations](#rag-with-citations-the-ai-reads-pdfs-like-a-human-surface)
- [API stability](#api-stability)
- [Roadmap](#roadmap-not-yet-implemented)
- [Contributing](#contributing)
- [License](#license)

---

## What is this?

Zotero Bridge is a [bootstrap-extension](https://www.zotero.org/support/dev/zotero_7_for_developers) plugin that turns your running Zotero instance into a JSON-RPC 2.0 server. External tools — research agents, citation pipelines, scrapers, MCP servers, custom CLIs — can read from and write to your library over plain HTTP without poking at SQLite directly.

```
┌──────────────────────────┐         ┌─────────────────────────────┐
│  Your tool / agent       │         │  Zotero (with this plugin)  │
│                          │         │                             │
│  curl /zotero-bridge/rpc │ ──HTTP─▶│  77 typed RPC methods       │
│  cnki-plugin push        │         │  • items.* (17)             │
│  research agent          │         │  • collections.* (12)       │
│  Better-BibTeX consumer  │         │  • attachments.* (6)        │
│  …                       │         │  • notes.* (6)              │
│                          │         │  • search.* (8)             │
│                          │         │  • tags.* (6)               │
│                          │         │  • export.* (5)             │
│                          │         │  • settings.* (4)           │
│                          │         │  • system.* (10)            │
└──────────────────────────┘         └─────────────────────────────┘
```

## Why?

Zotero ships an HTTP connector at `localhost:23119` that's hardcoded for the browser-extension use case (a handful of endpoints like `/connector/getSelectedCollection`). It is not a general-purpose API. If you want to ask "give me the 5 most recent journal articles tagged X", you have to either:

- Vendor a SQLite reader and parse the `.sqlite` directly (fragile, schema versions, write-locks)
- Eval arbitrary JS via a debug-server backdoor (insecure, unsupported)
- Write your own bootstrap plugin from scratch every project (rebuilds wheel, no shared conventions)

Zotero Bridge fills that gap with a **single, stable, typed API surface** that any tool can target.

## Quick start

### 1. Install the Zotero plugin (everyone does this)

1. Download the latest `zotero-bridge.xpi` from [Releases](https://github.com/dianzuan/zotero-bridge/releases).
2. In Zotero: **Tools → Plugins → ⚙ → Install Add-on From File…** → pick the `.xpi`.
3. Restart Zotero. The HTTP server is live on `localhost:23119/zotero-bridge/rpc`.

### 2. Pick a client

#### A. Claude Code (recommended)

The repo ships 1 umbrella skill (5 workflows via progressive disclosure) so Claude can search, add, export, OCR, and RAG over your library directly.

In Claude Code:

```
/plugin marketplace add dianzuan/zotero-bridge
/plugin install zotero-bridge@zotero-bridge
```

Now ask Claude things like *"find papers I have on transformer attention"* or *"add DOI 10.1038/nature12373 to my ML collection"* — it routes to the right workflow, which calls the RPC.

(Or for local dev: clone the repo and `/plugin marketplace add ~/zotero-bridge`.)

#### B. Python CLI / SDK (scripts and other AI agents)

```bash
uv tool install zotero-bridge        # or: pip install zotero-bridge

zotero-bridge ping
zotero-bridge search quick "transformer attention" --limit 10
zotero-bridge rpc items.get '{"id":12345}'    # escape hatch — covers all 77 methods
```

The `rpc` subcommand is the protocol-level escape hatch: any RPC method that doesn't have a friendly typer subcommand can still be called directly. `--jq` filters output (`gh api --jq` style), `--install-completion {bash|zsh|fish|powershell}` enables shell completion. See [`docs/api-stability.md`](docs/api-stability.md) for the SDK contract.

#### C. Raw HTTP (any tool, any language)

```bash
curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":1}'
```

## API surface

77 methods across 9 namespaces. The full conventions doc is at [docs/superpowers/specs/2026-04-23-xpi-api-prd.md](docs/superpowers/specs/2026-04-23-xpi-api-prd.md).

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

### Conventions

- **All return shapes follow PRD §2** — `serializeItem(item)` for item-bearing returns, paginated `{items, total, offset?, limit?}` envelope where pagination applies, `libraryId` (lowercase) at the wire.
- **Errors are JSON-RPC 2.0 structured `{code, message}`** — `-32602` for caller error (missing/wrong field), `-32603` for server error (Zotero internal failure).
- **Chinese name handling** — `items.create` automatically splits Chinese full names like `欧阳修` → `{lastName: "欧阳", firstName: "修"}` for proper Zotero creator records, including 70+ compound surnames.

## Development

### Prerequisites

- Node.js 18+
- Zotero 8 installed locally
- (Optional but recommended) WSL on Windows for the dev workflow

### Build & test

```bash
npm install
npm test           # 99 mocha unit tests
npm run build      # type-check + bundle + emit XPI to .scaffold/build/
```

### Hot-reload dev workflow

Set `ZOTERO_PLUGIN_ZOTERO_BIN_PATH` to your Zotero binary path, then:

```bash
ZOTERO_PLUGIN_ZOTERO_BIN_PATH=/path/to/zotero npm start
```

This launches Zotero with the plugin loaded as a proxy file. Source changes auto-rebuild and reload.

**WSL → Windows note**: `npm start`'s built-in RDP-based reload doesn't work cross-OS (the profile path issue). Use the bundled `system.reload` RPC instead:

```bash
npm run build && \
  rsync -a --delete .scaffold/build/addon/ "$DEV_ADDON_DIR" && \
  curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"system.reload","id":1}'
```

This invalidates Gecko's startup cache and reloads the plugin in-place.

## Project structure

```
src/
├── handlers/         # 9 handler files, one per namespace
├── utils/            # Pure helpers: errors, guards, serialize, etc.
├── server.ts         # JSON-RPC 2.0 dispatcher + endpoint registration
├── hooks.ts          # Bootstrap-time setup (preference defaults)
└── index.ts          # Plugin entry point
test/
├── handlers/         # Per-handler tests (sinon + mocked Zotero globals)
├── utils/            # Pure-helper unit tests
├── fixtures/         # Zotero mock harness (installZotero/resetZotero)
└── chinese-name.test.ts
addon/
└── manifest.json     # Plugin metadata (name, version, target Zotero versions)
```

## Status

`v1.3.4` — production-ready. Validated against a 5,000+-item / 70+-collection library. 99/99 mocha tests pass.

## RAG with Citations (the "AI reads PDFs like a human" surface)

Zotero Bridge's RAG layer (`python/zotero_bridge/rag/`) lets AI agents
retrieve text from a user's Zotero library **with structured provenance**.
Every chunk comes back as a `Citation` object carrying the Zotero item
key, attachment id, section heading, chunk index, similarity score, the
verbatim text, and a `zotero://` URI for one-click verification.

### Python API

```python
from zotero_bridge import retrieve_with_citations
from zotero_bridge.rag.embedder import create_embedder
from pathlib import Path

embedder = create_embedder(
    provider="ollama", model="nomic-embed-text",
    api_url="http://localhost:11434",
)
citations = retrieve_with_citations(
    query="how do transformers attend to long-range context?",
    store_path=Path("~/.local/share/zotero-bridge/rag/ml-papers.json").expanduser(),
    embedder=embedder,
    top_k=10,
)
for c in citations:
    print(f"{c.title} [{c.zotero_uri()}] section={c.section} score={c.score:.2f}")
    print(c.text)
```

### CLI

```bash
# 1) Build an index for a Zotero collection (existing command)
zotero-rag index --collection "ML Papers"

# 2) Retrieve citations for a query
zotero-rag cite "how do transformers attend to long-range context?" --collection "ML Papers" --output markdown
zotero-rag cite "how do transformers attend to long-range context?" --collection "ML Papers" --output json --top-k 5
```

### Stable JSON schema

The `--output json` form returns a list of objects with this stable schema:

```json
[
  {
    "itemKey": "ABC123",
    "attachmentId": 42,
    "title": "Attention Is All You Need",
    "authors": "Vaswani, Ashish; Shazeer, Noam",
    "section": "Section 3 — The Model",
    "chunkIndex": 7,
    "text": "...",
    "score": 0.87,
    "zoteroUri": "zotero://select/library/items/ABC123"
  }
]
```

This is the AI-facing contract — any agent consuming citations from zotero-bridge can rely on these field names.

## API stability

Stable contract for SDK / CLI consumers: [docs/api-stability.md](docs/api-stability.md).

## Roadmap (not yet implemented)

Preference keys are reserved in `SETTINGS_KEYS` (callable via `settings.set`), but the consumer RPC methods don't exist yet:

- `ocr.*` — for a future `attachments.ocr` method
- `embedding.*` — for future semantic search / chunking
- `rag.*` — for a future `search.semantic` method

PRs welcome.

## Contributing

PRs welcome. Run `npm test` before submitting; new methods need a mocha test using `test/fixtures/zotero-mock.ts`.

## License

[AGPL-3.0-or-later](LICENSE). For closed-source use, open an issue to discuss commercial licensing.

## Acknowledgments

- [Zotero](https://www.zotero.org/) by the Corporation for Digital Scholarship (AGPL-3.0)
- [`zotero-plugin-toolkit`](https://github.com/windingwind/zotero-plugin-toolkit) by windingwind (MIT)
- [`zotero-plugin-scaffold`](https://github.com/zotero-plugin-dev/zotero-plugin-scaffold) (AGPL-3.0)
- [`zotero-types`](https://github.com/windingwind/zotero-types) (MIT)
- Inspired by [`Jasminum`](https://github.com/l0o0/jasminum) (AGPL-3.0) — Chinese academic metadata for Zotero
- The Zotero plugin community (Knowledge4Zotero, zotero-pdf-translate, zotero-actions-tags, zotero-style — all AGPL-3.0)
