<div align="center">

<img src="assets/logo.png" alt="Zotron logo" width="120" />

# Zotron

Read and write your Zotero library from the terminal.

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

</div>

---

Zotron is a Zotero plugin + Rust CLI. The plugin turns Zotero into a JSON-RPC server (86 methods); the CLI gives you a typed command surface over it.

```bash
cargo install zotron

zotron ping
zotron search "digital economy" --collection "Macro" --author "Zhang" --after 2020
zotron items add --doi 10.1038/nature12373 --collection "ML Papers"
zotron export --format bibliography --collection "Macro"
zotron rag search --collection "Macro" "employment elasticity"
```

## Install

1. `cargo install zotron`
2. Install the [XPI plugin](https://github.com/dianzuan/zotron/releases/latest) in Zotero (Tools → Plugins → Install Add-on From File)
3. `zotron ping` to verify

For **Claude Code** or **Codex**, also install the agent plugin:

```bash
# Claude Code
/plugin marketplace add dianzuan/zotron && /zotron:setup

# Codex
codex plugin marketplace add dianzuan/zotron && $zotron-setup
```

## What you can do

```bash
# Search across metadata or PDF full text
zotron search "regression discontinuity" --fulltext --limit 20

# Manage items
zotron items get YR5BUGHG
zotron items fulltext YR5BUGHG
zotron collections tree
zotron tags add YR5BUGHG BF4I9QX4 --tag "reviewed"

# Export citations (bibtex is the default)
zotron export --collection "Macro"
zotron export --format bibliography YR5BUGHG BF4I9QX4

# OCR and RAG
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "Macro" "labor market effects"

# Escape hatch — any of the 86 RPC methods
zotron rpc items.get '{"key":"YR5BUGHG"}'
```

Output is JSON. Pipe to `jq` for filtering:

```bash
zotron search "employment" | jq '.items[] | {key, title, year}'
```

## Why not Zotero's Local API?

Zotero 7+ has a built-in [Local API](https://www.zotero.org/support/dev/web_api/v3/start) at `localhost:23119/api/`. Use it if `pyzotero` already does what you need.

Zotron covers what the Local API doesn't: add by DOI/URL/ISBN, batch operations, fulltext cache access, CiteProc bibliography, duplicate detection, and a CLI designed for agents that pipe JSON.

## Development

```bash
npm install && npm test     # 127 XPI unit tests
npm run build               # XPI → .scaffold/build/zotron.xpi
cargo test                  # 44 CLI contract tests
```

## License

[AGPL-3.0-or-later](LICENSE)
