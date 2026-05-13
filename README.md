<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

A Rust CLI for Zotero. Search, manage, cite, OCR, and RAG your papers from the terminal.

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[Install](#install) · [Usage](#usage) · [Agent integration](#agent-integration) · [Development](#development)

</div>

## Install

```bash
cargo install zotron
```

Then install the [Zotero plugin](https://github.com/dianzuan/zotron/releases/latest) (Tools → Plugins → Install Add-on From File) and restart Zotero.

```bash
zotron ping   # should print {"status": "ok", ...}
```

## Usage

```bash
# Search — title/author/year by default, PDF content with --fulltext
zotron search "digital economy" --author "Zhang" --after 2020
zotron search "regression discontinuity" --fulltext --collection "Macro"

# Items
zotron items add --doi 10.1038/nature12373 --collection "ML Papers"
zotron items fulltext YR5BUGHG
zotron collections tree

# Export — bibtex by default
zotron export --collection "Macro"
zotron export --format bibliography YR5BUGHG BF4I9QX4

# OCR + semantic retrieval
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "Macro" "labor market effects"
```

Output is JSON. Pipe to `jq`:

```bash
zotron search "employment" | jq '.items[] | {key, title, year}'
```

Run `zotron --help` for the full command list, `zotron <command> --help` for flags.

## Agent integration

Zotron works as a plugin for [Claude Code](https://docs.claude.com/en/docs/claude-code/) and [Codex](https://github.com/openai/codex). The agent calls `zotron` subcommands directly — no MCP, no tool schema overhead.

```bash
# Claude Code
/plugin marketplace add dianzuan/zotron && /zotron:setup

# Codex
codex plugin marketplace add dianzuan/zotron && $zotron-setup
```

After setup, ask in natural language: "search my Zotero for papers on attention mechanisms", "export this collection as BibTeX", "OCR the PDFs in my ML folder".

## How it works

Zotron has two parts:

1. **XPI plugin** — runs inside Zotero, exposes 86 JSON-RPC methods over `localhost:23119`
2. **Rust CLI** — typed subcommands that call those methods, designed for shell pipelines and agents

The CLI talks to Zotero's internal JS API — the same surface plugins use. This covers things the official [Local API](https://www.zotero.org/support/dev/web_api/v3/start) doesn't: add by DOI/URL/ISBN, fulltext cache, CiteProc bibliography, duplicate merging, batch operations.

## Development

```bash
npm install && npm test     # 127 XPI unit tests
npm run build               # → .scaffold/build/zotron.xpi
cargo test                  # 44 CLI contract tests
```

## License

[AGPL-3.0-or-later](LICENSE)
