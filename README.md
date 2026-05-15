<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

Let AI agents read, search, and annotate your Zotero library.

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[What it does](#what-it-does) · [Install](#install) · [For agents](#for-agents) · [CLI reference](#cli-reference) · [中文](README.zh-CN.md)

</div>

<!-- TODO: add demo GIF here — record a terminal session showing search → annotate → export -->

## Why Zotron?

Zotero's official API is read-only and HTTP-based. MCP servers add latency and token overhead per tool call. Zotron is a local Rust CLI that talks directly to Zotero's internal JS API — full read-write access, structured JSON output, pipes to `jq`.

## What it does

Once installed, your agent can:

- **Search** papers by title, author, year, tag, DOI, or full PDF text
- **Read** paper content, metadata, and notes
- **Annotate** PDFs — highlight, underline, or mark regions by quoting text (no need to open the PDF)
- **Export** citations as BibTeX, APA, or any CSL style
- **OCR** scanned PDFs and run hybrid semantic search (BM25 + vector + RRF)
- **Manage** collections, tags, and attachments

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

The agent calls `zotron` CLI commands directly.

### Source plugins

External sources are added through plugins — standalone binaries on `PATH` named `zotron-*`:

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

## FAQ

**Q: Does Zotero need to be running?**
Yes. Zotron talks to a live Zotero instance via its XPI plugin. Run `zotron ping` to check.

**Q: Does it work with Zotero 6?**
No. Zotron requires Zotero 7+ (tested on Zotero 8).

**Q: Can I use it without Claude Code or Codex?**
Yes. The CLI works standalone — any shell-capable agent or script can call `zotron` commands.

## Contributing

PRs welcome. Fork, branch, and open a pull request — CI must pass before merging.

## Star History

<a href="https://star-history.com/#dianzuan/zotron&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date" />
    <img alt="Star History" src="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date" width="500" />
  </picture>
</a>

## License

[AGPL-3.0-or-later](LICENSE)
