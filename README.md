<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

Let AI agents read, search, and annotate your Zotero library.

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[What it does](#what-it-does) · [Install](#install) · [For agents](#for-agents) · [CLI reference](#cli-reference) · [Development](#development)

</div>

## What it does

Zotron gives AI agents full access to your Zotero library. Once installed, your agent can:

- **Search** papers by title, author, year, tag, DOI, or full PDF text
- **Read** paper content, metadata, and notes
- **Annotate** PDFs — highlight, underline, or mark regions by quoting text (no need to open the PDF)
- **Export** citations as BibTeX, APA, or any CSL style
- **OCR** scanned PDFs and run hybrid semantic search (BM25 + vector + RRF)
- **Manage** collections, tags, and attachments

You talk to your agent in natural language. The agent uses Zotron under the hood.

## Install

```bash
cargo install zotron
```

Then install the [Zotero plugin](https://github.com/dianzuan/zotron/releases/latest) (Tools → Plugins → Install Add-on From File) and restart Zotero.

```bash
zotron ping   # should print {"status": "ok", ...}
```

## For agents

Zotron works as a plugin for [Claude Code](https://docs.claude.com/en/docs/claude-code/) and [Codex](https://github.com/openai/codex). Install once, then talk to your agent:

```bash
# Claude Code
/plugin marketplace add dianzuan/zotron && /zotron:setup

# Codex
codex plugin marketplace add dianzuan/zotron && $zotron-setup
```

After setup, just ask:

> "Search my Zotero for papers on attention mechanisms"
>
> "Read this paper and highlight the key findings in blue"
>
> "Export my ML collection as BibTeX"
>
> "Which of my papers discusses regression discontinuity?"

The agent calls `zotron` CLI commands internally — no MCP, no tool schema overhead.

### Source plugins

Zotron is extensible via source plugins — standalone binaries on `PATH` named `zotron-*`:

- **[zotron-scholar](https://github.com/dianzuan/zotron-scholar)** — OpenAlex, CrossRef, Semantic Scholar, Unpaywall, arXiv

Plugins output JSON to stdout, piped to `zotron push` to write into Zotero.

## How it works

Three layers:

1. **XPI plugin** (TypeScript) — runs inside Zotero 8, exposes 86 JSON-RPC 2.0 methods across 11 namespaces at `localhost:23119/zotron/rpc`
2. **Rust CLI** — noun-verb subcommands (`zotron items get`, `zotron search "query"`) published on [crates.io](https://crates.io/crates/zotron)
3. **Agent plugin** — skills for Claude Code and Codex that let AI agents drive Zotero through the CLI

The CLI talks to Zotero's internal JS API — the same surface plugins use. This covers things the official [Local API](https://www.zotero.org/support/dev/web_api/v3/start) doesn't: add by DOI/URL/ISBN, fulltext cache, CiteProc bibliography, duplicate merging, batch operations.

## CLI reference

All output is JSON. Pipe to `jq` for filtering.

```bash
zotron search "digital economy" --author "Zhang" --after 2020
zotron search "regression discontinuity" --fulltext --collection "Macro"
zotron items fulltext YR5BUGHG
zotron annotations create ITEM_KEY --quote "important finding" --color "#2ea8e5"
zotron export --collection "Macro"
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "Macro" "labor market effects"
```

Run `zotron --help` for the full command list, `zotron <command> --help` for flags. See also: [CLI reference (en)](docs/cli-reference.md) · [CLI 参考 (中文)](docs/cli-reference-zh.md)

## Development

```bash
npm install && npm test     # XPI unit tests
npm run build               # → .scaffold/build/zotron.xpi
cargo test                  # CLI + types tests
```

## Release

Pushing a `v*` tag triggers the [release workflow](.github/workflows/release.yml): builds the XPI, creates a GitHub Release, and publishes to crates.io.

See the [latest release](https://github.com/dianzuan/zotron/releases/latest).

## License

[AGPL-3.0-or-later](LICENSE)
