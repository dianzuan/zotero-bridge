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

| Namespace | Methods | CLI | What it does |
|---|---|---|---|
| `items` | 21 | `zotron items <verb>` | Get, list, create, update, delete, trash, fulltext, add by DOI/URL/ISBN/file, duplicates, related |
| `collections` | 12 | `zotron collections <verb>` | List, tree, create, rename, delete, add/remove items, getItems |
| `attachments` | 8 | `zotron attachments <verb>` | List, get, fulltext, add, add-by-url, path, delete, find-pdf |
| `notes` | 5 | `zotron notes <verb>` | Get, list, create, update, search (delete goes through items.delete) |
| `annotations` | 3 | `zotron annotations <verb>` | List, create, delete PDF annotations |
| `search` | 8 | `zotron search <verb>` | Quick, fulltext, advanced, by-tag, by-identifier, saved searches |
| `tags` | 6 | `zotron tags <verb>` | List, add, remove, rename, delete, batch-update |
| `export` | 5 | `zotron export <format>` | BibTeX, CSL-JSON, RIS, bibliography, CSV |
| `settings` | 4 | `zotron settings <verb>` | Get, set, list, reset preferences |
| `system` | 13 | `zotron system <verb>` | Version, sync, ping, libraries, item-types, item-fields, creator-types, list-methods, describe, and more |
| `rag` | 1 | `zotron rpc rag.*` | Semantic search over OCR'd collection chunks |

**Total: 86 RPC methods**

## RPC escape hatch

For methods without a typed CLI subcommand:

```bash
zotron rpc <namespace>.<method> '<json-params>'
```

**Parameters**: All item/collection/attachment identifiers use `key` (8-char alphanumeric string like `"YR5BUGHG"`). Parent references use `parentKey`. Batch operations use `keys: [...]`. Numeric IDs are also accepted.
