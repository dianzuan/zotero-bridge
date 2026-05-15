<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

Let AI agents read, search, and annotate your Zotero library.

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[What it does](#what-it-does) · [Install](#install) · [For agents](#for-agents) · [CLI reference](#cli-reference) · [中文](README.zh-CN.md)

<a href="https://star-history.com/#dianzuan/zotron&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date" />
    <img alt="Star History" src="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date" width="500" />
  </picture>
</a>

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

### 1. Rust CLI

```bash
cargo install zotron
```

### 2. Zotero plugin

Download the latest [zotron.xpi](https://github.com/dianzuan/zotron/releases/latest), then in Zotero: Tools → Plugins → Install Add-on From File. Restart Zotero.

### 3. Verify

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

## CLI reference

All output is JSON. Pipe to `jq` for filtering.

```bash
# Search
zotron search "digital economy" --author "Zhang" --after 2020
zotron search "regression discontinuity" --fulltext --collection "Macro"

# Read
zotron items get YR5BUGHG
zotron items fulltext YR5BUGHG
zotron collections tree

# Annotate
zotron annotations create YR5BUGHG --quote "important finding" --color "#2ea8e5"
zotron annotations list YR5BUGHG

# Export
zotron export --collection "Macro"
zotron export --format bibliography YR5BUGHG BF4I9QX4

# OCR + RAG
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "Macro" "labor market effects"

# Pipe to jq
zotron search "employment" | jq '.items[] | {key, title, year}'
```

Run `zotron --help` for the full command list, `zotron <command> --help` for flags.

Full reference: [CLI reference (en)](docs/cli-reference.md) · [CLI 参考 (中文)](docs/cli-reference-zh.md)

## License

[AGPL-3.0-or-later](LICENSE)
