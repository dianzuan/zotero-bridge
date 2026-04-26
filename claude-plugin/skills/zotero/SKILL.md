---
name: zotero
description: Manage the user's Zotero library — search papers, add/organize items, export citations, OCR PDFs, and run semantic search (RAG). Use whenever the user mentions Zotero, "我的文献库", finding/adding/citing papers, "参考文献", "文献综述", or wants to read/extract content from their PDFs. Requires Zotero desktop running with the zotero-bridge plugin on localhost:23119.
---

# Zotero

Read-write bridge to the user's local Zotero library via the `zotero-bridge` CLI. Covers search, add/organize, citation export, PDF OCR, and semantic (RAG) search over OCR'd collections.

**Dependency:** Zotero desktop must be running with the `zotero-bridge` XPI plugin installed and listening on `localhost:23119`. A startup monitor warns if it isn't reachable; if so, ask the user to start Zotero before retrying.

## Pick a workflow

| User intent | Workflow | Sub-file |
|---|---|---|
| Find / read papers, browse collections, get fulltext or annotations | search | [search.md](search.md) |
| Add by DOI/URL/ISBN/file, update metadata, manage collections & tags, dedupe | manage | [manage.md](manage.md) |
| Generate references in GB/T 7714, BibTeX, RIS, CSL-JSON | export | [export.md](export.md) |
| OCR scanned/Chinese PDFs into Markdown notes (prep for RAG) | ocr | [ocr.md](ocr.md) |
| Semantic search across an OCR'd collection (literature review, "前人研究怎么说") | rag | [rag.md](rag.md) |

A typical session chains them: `search` to locate papers → `manage` to organize → `ocr` + `rag` for literature review → `export` for citations.

## CLI conventions

All commands invoke the unified `zotero-bridge` CLI.

**Typed subcommands** (preferred when available — clearer for AI, validated args):

```bash
zotero-bridge search quick "数字经济 就业" --limit 10
zotero-bridge collections tree
zotero-bridge export bibtex 12345 12346
```

See `zotero-bridge --help` for the full typed surface.

**`rpc` escape hatch** covers all 77 RPC methods across 9 namespaces — use when no typed subcommand exists:

```bash
zotero-bridge rpc <method.name> '<json-params>'
# e.g.
zotero-bridge rpc search.fulltext '{"query":"regression discontinuity","limit":10}'
zotero-bridge rpc items.addByDOI '{"doi":"10.1016/j.jfineco.2024.01.001"}'
```

**`--jq` filter** trims output to just the fields you need (cuts tokens):

```bash
zotero-bridge rpc items.getRecent '{"limit":50}' --jq '.[].title'
zotero-bridge rpc collections.tree --jq '.[] | {id, name}'
```

## RPC method reference

For the full 77-method list grouped by namespace, see [reference/rpc-methods.md](reference/rpc-methods.md). Read it on demand when you need a method that isn't documented in a workflow sub-file.
