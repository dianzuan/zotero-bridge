---
name: zotero
description: Manage the user's Zotero library — search papers, add/organize items, export citations, OCR PDFs, and run semantic search (RAG). Use whenever the user mentions Zotero, "我的文献库", finding/adding/citing papers, "参考文献", "文献综述", or wants to read/extract content from their PDFs. Requires Zotero desktop running with the zotron plugin on localhost:23119.
---

# Zotero

Read-write bridge to the user's local Zotero library via the `zotron` CLI. Covers search, add/organize, citation export, PDF OCR, and semantic (RAG) search over OCR'd collections.

**Dependency:** Zotero desktop must be running with the `zotron` XPI plugin installed and listening on `localhost:23119`. If a CLI call fails with a connection error, ask the user to start Zotero — or, if the XPI was never installed, run the bundled `/zotron:setup` slash command to bootstrap it.

## Pick a workflow

| User intent | Workflow | Sub-file |
|---|---|---|
| Find / read papers, browse collections, get fulltext or annotations | search | [search.md](search.md) |
| Add by DOI/URL/ISBN/file, update metadata, manage collections & tags, dedupe | manage | [manage.md](manage.md) |
| Generate references in GB/T 7714, BibTeX, RIS, CSL-JSON | export | [export.md](export.md) |
| OCR scanned/Chinese PDFs into Zotero-attached raw/block/chunk artifacts | ocr | [ocr.md](ocr.md) |
| RAG retrieval hits for literature review / academic-zh span provenance | rag | [rag.md](rag.md) |

A typical session chains them: `search` to locate papers → `manage` to organize → `ocr` + `rag` for literature review → `export` for citations.

## CLI conventions

All commands use the `zotron` CLI with noun-verb structure:

```bash
zotron <namespace> <verb> [args] [--flags]
```

**Typed subcommands** cover all operations — always prefer these over raw RPC:

```bash
zotron search quick "数字经济" --limit 10
zotron items get 12345
zotron items fulltext 12345
zotron notes list --parent 12345
zotron attachments list --parent 12345
zotron annotations list --parent 12345
zotron tags add 12345 --tag "已读"
zotron collections tree
zotron export bibtex 12345
zotron settings list
zotron system list-methods
```

**IDs:** All item-scoped commands accept either a numeric ID (`12345`) or an 8-char item key (`YR5BUGHG`). Collections accept numeric ID or name (`"数字经济"`).

**`--jq` filter** trims output to cut tokens:

```bash
zotron items list --limit 50 --jq '.[].title'
zotron collections tree --jq '.[] | {id, name}'
```

**`rpc` escape hatch** for edge cases without a typed subcommand:

```bash
zotron rpc <method.name> '<json-params>'
```

**Discovery:**
- `zotron --help` — list all namespaces
- `zotron <namespace> --help` — list subcommands in a namespace
- `zotron system list-methods` — list all RPC methods
- `zotron system describe items.get` — describe a specific method's parameters
