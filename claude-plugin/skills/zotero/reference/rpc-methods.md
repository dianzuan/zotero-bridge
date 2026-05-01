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
| `items` | `zotron items <verb>` | Get, list, create, update, delete, trash, fulltext, add by DOI/URL/ISBN/file, duplicates, related |
| `collections` | `zotron collections <verb>` | List, tree, create, rename, delete, add/remove items |
| `notes` | `zotron notes <verb>` | List, get, create, update, delete, search |
| `attachments` | `zotron attachments <verb>` | List, get, fulltext, add, path, delete, find-pdf |
| `annotations` | `zotron annotations <verb>` | List, create, delete PDF annotations |
| `search` | `zotron search <verb>` | Quick, fulltext, advanced, by-tag, saved searches |
| `tags` | `zotron tags <verb>` | List, add, remove, rename, delete, batch-update |
| `export` | `zotron export <format>` | BibTeX, CSL-JSON, RIS, bibliography |
| `settings` | `zotron settings <verb>` | Get, set, list preferences |
| `system` | `zotron system <verb>` | Version, sync, libraries, item-types, list-methods, describe |
| `rag` | `zotron rpc rag.*` | Semantic search over OCR'd collection chunks |

## RPC escape hatch

For methods without a typed CLI subcommand:

```bash
zotron rpc <namespace>.<method> '<json-params>'
```

**ID or Key**: All methods that accept `id` or `parentId` also accept an 8-char alphanumeric item key string (e.g. `"YR5BUGHG"`).
