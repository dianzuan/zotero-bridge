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

- **77 typed RPC methods** across 9 namespaces — items, collections, attachments, notes, search, tags, export, settings, system
- **Python CLI + SDK** — typer-based, with `--jq` filtering, `--paginate` auto-loop, `--dry-run` preview, `--output {json,table}`, shell completion
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

### Install (users)

1. Download the latest XPI from [Releases](https://github.com/dianzuan/zotero-bridge/releases) (e.g. `zotero-bridge.xpi`).
2. In Zotero: **Tools → Plugins → ⚙ → Install Add-on From File…** → pick the `.xpi`.
3. Restart Zotero. The HTTP server is live on `localhost:23119/zotero-bridge/rpc`.

### Try it

```bash
# Health check
curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":1}'
# → {"jsonrpc":"2.0","result":{"status":"ok","timestamp":"..."}, "id":1}

# Library stats
curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.libraryStats","params":{},"id":1}'
# → {"libraryId":1, "items":5312, "collections":72}

# Most recent 3 items added
curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"items.getRecent","params":{"limit":3,"type":"added"},"id":1}'
```

### Method 2 — Use the Python CLI (one binary, all 77 methods)

```bash
pip install zotero-bridge   # or: uv tool install zotero-bridge

# Friendly typed subcommands:
zotero-bridge ping
zotero-bridge search quick "transformer attention" --limit 10
zotero-bridge collections tree

# Generic escape hatch — covers all 77 RPC methods:
zotero-bridge rpc <method> '<json-params>'
zotero-bridge rpc items.get '{"id":12345}'
zotero-bridge rpc tags.add '{"itemId":12345,"tags":["read"]}'
```

The `rpc` subcommand is the protocol-level escape hatch: any RPC method
that does not have a friendly typer subcommand can still be called directly.
This keeps the CLI surface stable as the XPI grows.

#### Shell completion

```bash
zotero-bridge --install-completion bash    # or zsh / fish / powershell
```

After restarting your shell, `zotero-bridge <Tab>` completes subcommands and flags.

#### Filter output with --jq

Borrowed from `gh api --jq`. Reduces AI token usage massively when you
only need a few fields:

```bash
zotero-bridge rpc items.getRecent '{"limit": 50}' --jq '.[].title'
zotero-bridge collections list --jq '.[] | select(.parentID == null) | .name'
```

## API surface

77 methods across 9 namespaces. The full conventions doc is at [docs/superpowers/specs/2026-04-23-xpi-api-prd.md](docs/superpowers/specs/2026-04-23-xpi-api-prd.md).

| Namespace | Methods | What it does |
|---|---|---|
| `items.*` | 17 | CRUD on Zotero items, add by DOI/URL/ISBN/file, recent, trash, duplicates, related |
| `collections.*` | 12 | List, create, rename, move, tree, items in collection |
| `attachments.*` | 6 | List attachments, get fulltext (cache-file backed), get path, find PDF |
| `notes.*` | 6 | Notes CRUD, annotations, search inside notes |
| `search.*` | 8 | Quick / fulltext / by-tag / by-identifier / advanced; saved searches |
| `tags.*` | 6 | List, add, remove, rename, delete (cross-library) |
| `export.*` | 5 | BibTeX / CSL-JSON / RIS / CSV / bibliography (CiteProc) |
| `settings.*` | 4 | Plugin-side preferences (e.g. OCR provider, embedding model) |
| `system.*` | 10 | Ping, version, libraries, switchLibrary, sync, currentCollection, **`system.reload`** (self-reload for dev) |

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

`v1.3.4` — production-ready for the 77 methods documented above. Validated against a real 5,000+-item / 70+-collection Zotero library. 99/99 mocha tests pass, TypeScript strict mode clean.

The codebase has gone through a 61-fix audit-and-fix campaign covering every method against Zotero 8 source code. See `docs/superpowers/specs/` for the audit deliverables.

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

This is the AI-facing contract: any agent that consumes citations from
zotero-bridge can rely on these field names. The `zoteroUri` field opens
the source item in Zotero desktop for one-click verification.

## API stability

Stable contract for SDK / CLI consumers: [docs/api-stability.md](docs/api-stability.md).

## Roadmap (not yet implemented)

The following features have **preference keys reserved** in `SETTINGS_KEYS` (so callers can already read/write `settings.set { key: "ocr.provider", value: ... }`) but **no consumer RPC methods exist yet**:

- **OCR**: `ocr.provider`, `ocr.apiKey`, `ocr.apiUrl`, `ocr.model` — intended for an `attachments.ocr` method that runs an attached PDF through a configurable OCR provider (e.g. GLM, OpenAI Vision). Not implemented.
- **Embeddings**: `embedding.provider`, `embedding.model`, `embedding.apiKey`, `embedding.apiUrl` — intended to back a future semantic-search or document-chunking pipeline. Not implemented.
- **RAG**: `rag.chunkSize`, `rag.chunkOverlap`, `rag.topK` — intended for a `search.semantic` method that searches across vector-embedded item content. Not implemented.

These are tracked as roadmap items, not bugs. Contributions welcome — see [`CONTRIBUTING.md`](CONTRIBUTING.md) (TBD).

## Contributing

Contributions welcome under AGPL-3.0-or-later. Run `npm test` before submitting; new methods need at minimum one mocha test using the existing `test/fixtures/zotero-mock.ts` harness.

## License

**[AGPL-3.0-or-later](LICENSE)** — same as Zotero itself.

This means: anyone can use, study, modify, and redistribute this plugin **including over a network**, as long as derivative work (forks, hosted services, modified binaries) ships with full source code under the same license.

If you want to use parts of Zotero Bridge in a closed-source product, this license is not for you — open an issue and we'll discuss commercial licensing options.

## Acknowledgments

- [Zotero](https://www.zotero.org/) by the Corporation for Digital Scholarship (AGPL-3.0)
- [`zotero-plugin-toolkit`](https://github.com/windingwind/zotero-plugin-toolkit) by windingwind (MIT)
- [`zotero-plugin-scaffold`](https://github.com/zotero-plugin-dev/zotero-plugin-scaffold) (AGPL-3.0)
- [`zotero-types`](https://github.com/windingwind/zotero-types) (MIT)
- Inspired by [`Jasminum`](https://github.com/l0o0/jasminum) (AGPL-3.0) — Chinese academic metadata for Zotero
- The Zotero plugin community (Knowledge4Zotero, zotero-pdf-translate, zotero-actions-tags, zotero-style — all AGPL-3.0)
