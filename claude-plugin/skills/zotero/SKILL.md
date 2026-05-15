---
name: zotero
description: Manage the user's Zotero library — search papers, add/organize items, export citations, OCR PDFs, and run semantic search (RAG). Use whenever the user mentions Zotero, "我的文献库", finding/adding/citing papers, "参考文献", "文献综述", or wants to read/extract content from their PDFs. Requires Zotero desktop running with the zotron XPI plugin. Check with `zotron ping`.
---

# Zotero

Read-write bridge to the user's local Zotero library via the Rust `zotron` CLI and Zotero-side JS/XPI RPC bridge. Python code in this repo is legacy reference material for migration parity, not the target implementation surface.

**Dependency:** Zotero desktop must be running with the `zotron` XPI plugin installed. Verify with `zotron ping`. If it fails, ask the user to start Zotero — or, if the XPI was never installed, run `/zotron:setup`. Do not curl HTTP endpoints directly; always use the `zotron` CLI.

## Pick a workflow

| User intent | Workflow | Sub-file |
|---|---|---|
| Find / read papers, browse collections, get fulltext or annotations | search | [search.md](search.md) |
| Add by DOI/URL/ISBN/file, update metadata, manage collections & tags, dedupe | manage | [manage.md](manage.md) |
| Generate references in GB/T 7714, BibTeX, RIS, CSL-JSON | export | [export.md](export.md) |
| OCR scanned/Chinese PDFs into hidden per-PDF raw/block/chunk sidecars | ocr | [ocr.md](ocr.md) |
| RAG retrieval hits for literature review / academic-zh span provenance | rag | [rag.md](rag.md) |

A typical session chains them: `search` to locate papers → `manage` to organize → `ocr` + `rag` for literature review → `export` for citations.

## CLI conventions

All commands use the `zotron` CLI with noun-verb structure:

```bash
zotron <namespace> <verb> [args] [--flags]
```

**Typed Rust subcommands** cover normal operations — always prefer these over raw RPC:

```bash
zotron ping                        # check connectivity
zotron search "数字经济" --limit 10
zotron search "数字经济" --collection "宏观因子" --limit 10
zotron collections get-items "宏观因子" --limit 20
zotron items get YR5BUGHG
zotron items fulltext YR5BUGHG
zotron notes list --parent YR5BUGHG
zotron attachments list --parent YR5BUGHG
zotron annotations list YR5BUGHG
zotron annotations create YR5BUGHG --quote "要高亮的文字"  # works headlessly, no PDF viewer required
zotron tags add YR5BUGHG --tag "已读"
zotron collections tree
zotron export YR5BUGHG
zotron settings list
zotron system schema                   # item types, fields, creator types
zotron system schema --type journalArticle  # fields for a specific type
zotron system list-methods
```

**Keys (primary):** All item-scoped commands accept an 8-char item key (`YR5BUGHG`) as the primary identifier. RPC params use `key`/`parentKey`/`keys` — never `id`/`parentId`. Collections accept 8-char key or name (`"数字经济"`).

**Search vs collection browsing:**
- `zotron search "关键词"` searches the whole library.
- `zotron search "关键词" --collection "集合名"` searches only items in that collection.
- `zotron search "关键词" --fulltext` searches inside PDF content, not just metadata.
- `zotron collections get-items "集合名"` lists items in a collection; use this when the user asks "这个集合里有什么".
- `zotron collections items "集合名"` is an alias for `get-items`.

**Filtering output:** Rust `zotron` stays JSON-first and does not embed libjq. Pipe to external `jq`. Many list/search commands return an envelope such as `{"items":[...],"total":N}`, so filter through `.items[]`:

```bash
zotron search "数字经济" | jq '.items[].title'
zotron collections get-items "宏观因子" | jq '.items[].title'
zotron collections tree | jq '.[] | {key, name}'
```

Do not assume Python-only conveniences such as built-in `--jq` exist in the Rust CLI.

**Discovery:**
- `zotron --help` — list all namespaces
- `zotron <namespace> --help` — list subcommands in a namespace
- `zotron system list-methods` — list all RPC methods
- `zotron system describe items.get` — describe a specific method's parameters
