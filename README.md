<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

Let AI agents read, search, and annotate your Zotero library.

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[Why Zotron?](#why-zotron) · [What it does](#what-it-does) · [Install](#install) · [For agents](#for-agents) · [CLI reference](#cli-reference) · [FAQ](#faq) · [中文](README.zh-CN.md)

</div>

## Why Zotron?

Zotron is a local Rust CLI that talks directly to Zotero's internal JS API — full read-write access, structured JSON output, pipes to `jq`. Zotero's official API is read-only and HTTP-based; MCP servers add latency and token overhead per tool call.

## What it does

Once installed, your agent can:

- **Search** papers by title, author, year, tag, DOI, or full PDF text. Combine filters in one call: `--author "Li" --after 2020 --tag "core" --collection "Macro"`. Fulltext search (`--fulltext`) looks inside PDFs, not just metadata.

- **Read** paper content, metadata, and notes. `items fulltext` returns the cached text of a PDF attachment. `items get` returns structured metadata (title, authors, date, journal, DOI, tags, collections). `notes list` includes OCR markdown when available.

- **Annotate** PDFs by quoting text. Pass `--quote "the sentence you want"` and Zotron locates it in the PDF and creates a highlight at the correct position — without opening the PDF in Zotero's reader. Works for highlight and underline types. Supports Zotero's 8 built-in colors.

- **Export** citations as BibTeX, APA, Chicago, or any CSL style. Export a single item, multiple items, or an entire collection. Output goes to stdout, redirect to a file as needed.

- **OCR** scanned PDFs with pluggable providers (MinerU, GLM, PaddleOCR). OCR results are stored as sidecar files per attachment. After OCR, `rag search` runs hybrid retrieval: BM25 lexical matching + cosine vector similarity + Reciprocal Rank Fusion, all local.

- **Manage** collections, tags, and attachments. Create collections, move items between them, add/remove tags in batch, attach files by URL or local path, find missing PDFs.

## Install

### 1. CLI (recommended)

Download the prebuilt binary for your platform from the [latest release](https://github.com/dianzuan/zotron/releases/latest), put it in `~/.local/bin/` (or anywhere on your `PATH`), and make it executable:

| Platform | File |
|----------|------|
| Linux x86_64 | `zotron-linux-amd64` |
| Linux ARM64 | `zotron-linux-arm64` |
| macOS Intel | `zotron-macos-amd64` |
| macOS Apple Silicon | `zotron-macos-arm64` |
| Windows x86_64 | `zotron-windows-amd64.exe` |

```bash
chmod +x zotron-linux-amd64
mv zotron-linux-amd64 ~/.local/bin/zotron
```

Or build from source: `cargo install zotron`

### 2. Zotero plugin

Download [zotron.xpi](https://github.com/dianzuan/zotron/releases/latest) from the same release page. In Zotero: Tools → Plugins → Install Add-on From File. Restart Zotero.

### Verify

```bash
zotron ping   # should print {"status": "ok", ...}
```

If `ping` fails, make sure Zotero is running and the Zotron plugin is enabled (Tools → Plugins).

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

A typical agent workflow looks like this: search for papers → read fulltext → highlight relevant passages with `--quote` → export citations. Each step is one CLI call, and the agent chains them based on the JSON output of the previous step.

### Source plugins

External sources are added through plugins — standalone binaries on `PATH` named `zotron-*`:

- **[zotron-scholar](https://github.com/dianzuan/zotron-scholar)** — OpenAlex, CrossRef, Semantic Scholar, Unpaywall, arXiv

Plugins output JSON to stdout, piped to `zotron push` to write into Zotero.

## CLI reference

All output is JSON. Pipe to `jq` for filtering.

### Search

```bash
# Quick search by title/author/year
zotron search "digital economy"

# Combine filters
zotron search "digital economy" --author "Zhang" --after 2020 --collection "Macro"

# Search inside PDF text
zotron search "regression discontinuity" --fulltext

# Search by DOI
zotron search --doi 10.1038/nature12373
```

### Read

```bash
# Full metadata
zotron items get YR5BUGHG

# PDF fulltext (cached by Zotero)
zotron items fulltext YR5BUGHG

# List attachments
zotron attachments list --parent YR5BUGHG

# Collection tree
zotron collections tree
```

### Annotate

```bash
# Highlight by quoting text (locates automatically, no PDF viewer needed)
zotron annotations create YR5BUGHG --quote "important finding" --color "#2ea8e5"

# List existing annotations
zotron annotations list YR5BUGHG
```

### Export

```bash
# BibTeX (default)
zotron export --collection "Macro"

# APA bibliography
zotron export --format bibliography YR5BUGHG BF4I9QX4

# Redirect to file
zotron export --collection "Macro" > refs.bib
```

### OCR + RAG

```bash
# OCR a scanned PDF
zotron ocr process --parent YR5BUGHG --provider mineru

# Hybrid semantic search (BM25 + vector + RRF)
zotron rag search --collection "Macro" "labor market effects"
```

### Pipe to jq

```bash
# Extract key fields
zotron search "employment" | jq '.items[] | {key, title, year}'

# Count results
zotron search "climate" | jq '.total'
```

Run `zotron --help` for the full command list, `zotron <command> --help` for flags.

## FAQ

**Q: Does Zotero need to be running?**
Yes. Zotron communicates with Zotero through the XPI plugin on `localhost:23119`. Run `zotron ping` to check the connection.

**Q: Does it work with Zotero 6?**
No. Zotron requires Zotero 7+ (tested on Zotero 8).

**Q: Can I use it without Claude Code or Codex?**
Yes. The CLI works standalone. Any shell-capable agent, script, or human can call `zotron` commands.

**Q: What platforms are supported?**
Windows, macOS, and Linux. The CLI is a single Rust binary. The XPI plugin runs inside Zotero on all platforms Zotero supports.

**Q: Can I use it with multiple Zotero libraries?**
Yes. `zotron system libraries` lists available libraries. `zotron system switchLibrary --id 2` switches the active library.

**Q: `zotron ping` fails — what do I check?**
1. Is Zotero running?
2. Is the Zotron plugin enabled? (Tools → Plugins)
3. Is something else using port 23119?
4. On Windows, check that your firewall allows localhost connections.

**Q: How does `--quote` highlighting work without opening the PDF?**
Zotron opens the PDF in a background reader tab (invisible to the user), extracts per-character position data, locates the quoted text, creates the annotation, and closes the background tab.

**Q: Downloaded the binary but "permission denied"?**
Run `chmod +x zotron-*` and make sure it's in a directory on your `PATH` (e.g., `~/.local/bin/`).

**Q: Can't download from GitHub (network issues)?**
The `/zotron:setup` skill tries mirror sites automatically. You can also manually download from a mirror: replace `https://github.com/` with `https://gh-proxy.com/https://github.com/` in the download URL.

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
