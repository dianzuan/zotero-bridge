# CLI & RPC Reference

## Discovering commands

The `zotron` CLI is self-documenting. Use these instead of reading this file:

```bash
# List all namespaces
zotron --help

# List subcommands in a namespace
zotron items --help
zotron search --help
zotron notes --help

# Describe a specific RPC method's parameters
zotron system describe items.get

# List all RPC methods
zotron system list-methods
```

## Namespace summary

| Namespace | CLI | What it does |
|---|---|---|
| `search` | `zotron search "query" --author --tag --fulltext --doi` | Unified search with ergonomic flags |
| `items` | `zotron items <verb>` | Get, list, create, update, delete, trash/restore, fulltext, recent, citation-key, add (--doi/--isbn/--from-url/--file), find/merge duplicates, related |
| `collections` | `zotron collections <verb>` | List, tree, get, create, rename, delete, add/remove items, get-items, stats |
| `attachments` | `zotron attachments <verb>` | List, get, fulltext, add (--path/--from-url), path, delete, find-pdf |
| `notes` | `zotron notes <verb>` | Get, list, create, update, delete, search |
| `annotations` | `zotron annotations <verb>` | List, create, delete PDF annotations |
| `tags` | `zotron tags <verb>` | List, add, remove, rename, delete (add/remove accept multiple keys) |
| `export` | `zotron export --format bibtex/ris/csl-json/bibliography` | Export citations in various formats |
| `settings` | `zotron settings <verb>` | Get, set, list preferences |
| `system` | `zotron system <verb>` | Version, libraries, library-stats, schema, current-collection, list-methods, describe |
| `ocr` | `zotron ocr <verb>` | providers, run, status, process |
| `rag` | `zotron rag <verb>` | providers, embed, status, search |

## Identifiers

All item/collection/attachment identifiers use `key` (8-char alphanumeric string like `"YR5BUGHG"`). Parent references use `parentKey`. Batch operations use `keys: [...]`. Collections also accept name strings.
