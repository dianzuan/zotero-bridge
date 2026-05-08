# Python-to-Rust CLI surface manifest

Generated for the Rust migration slice on 2026-05-07. This manifest maps the incumbent `zotron` Python Typer CLI to Rust migration status without changing Python behavior, Rust implementation, or the XPI.

Status legend:
- `implemented`: already covered by the Rust CLI scaffold/parity fixtures in this branch lineage.
- `next-read-only`: recommended read-only migration slices.
- `defer-mutating`: side-effecting command; keep in Python until read-only parity and safety gates are complete.

| Python command | RPC method(s) | Side-effect class | Rust status | Recommended migration slice |
| --- | --- | --- | --- | --- |
| `zotron ping` | `system.ping` | read-only | implemented | baseline |
| `zotron rpc` | user supplied | depends on method | implemented escape hatch | baseline/manual |
| `zotron push` | `items.create`, `attachments.add` | mutating | Python only | defer-mutating |
| `zotron find-pdfs` | `collections.getItems`, `attachments.list`, `attachments.findPDF` | mutating | Python only | defer-mutating |
| `zotron system version` | `system.version` | read-only | not migrated | next-read-only-system |
| `zotron system sync` | `system.sync` | mutating/external sync | Python only | defer-mutating |
| `zotron system libraries` | `system.libraries` | read-only | not migrated | next-read-only-system |
| `zotron system switch-library` | `system.switchLibrary` | mutating context | Python only | defer-mutating |
| `zotron system library-stats` | `system.libraryStats` | read-only | not migrated | next-read-only-system |
| `zotron system item-types` | `system.itemTypes` | read-only | not migrated | next-read-only-system |
| `zotron system item-fields` | `system.itemFields` | read-only | not migrated | next-read-only-system |
| `zotron system creator-types` | `system.creatorTypes` | read-only | not migrated | next-read-only-system |
| `zotron system current-collection` | `system.currentCollection` | read-only | not migrated | next-read-only-system |
| `zotron system reload` | `system.reload` | mutating/plugin lifecycle | Python only | defer-mutating |
| `zotron system list-methods` | `system.listMethods` | read-only | implemented | baseline |
| `zotron system describe` | `system.describe` | read-only | not migrated | next-read-only-system |
| `zotron collections list` | `collections.list` | read-only | not migrated | next-read-only-collections |
| `zotron collections tree` | `collections.tree` | read-only | implemented | baseline |
| `zotron collections get` | `collections.list` for name resolution, `collections.get` | read-only | not migrated | next-read-only-collections |
| `zotron collections get-items` | `collections.list` for name resolution, `collections.getItems` | read-only | not migrated | next-read-only-collections |
| `zotron collections stats` | `collections.list` for name resolution, `collections.stats` | read-only | not migrated | next-read-only-collections |
| `zotron collections rename` | `collections.list`, `collections.rename` | mutating | Python only | defer-mutating |
| `zotron collections create` | `collections.create` | mutating | Python only | defer-mutating |
| `zotron collections delete` | `collections.delete` | mutating | Python only | defer-mutating |
| `zotron collections add-items` | `collections.addItems` | mutating | Python only | defer-mutating |
| `zotron collections remove-items` | `collections.removeItems` | mutating | Python only | defer-mutating |
| `zotron items get` | `items.get` | read-only | implemented | baseline |
| `zotron items add-by-doi` | `items.addByDOI` | mutating/import | Python only | defer-mutating |
| `zotron items add-by-isbn` | `items.addByISBN` | mutating/import | Python only | defer-mutating |
| `zotron items add-by-url` | `items.addByURL` | mutating/import | Python only | defer-mutating |
| `zotron items trash` | `items.trash` | mutating/reversible | Python only | defer-mutating |
| `zotron items restore` | `items.restore` | mutating | Python only | defer-mutating |
| `zotron items find-duplicates` | `items.findDuplicates` | read-only/library scan | not migrated | next-read-only-items |
| `zotron items merge-duplicates` | `items.mergeDuplicates` | mutating/destructive merge | Python only | defer-mutating |
| `zotron items list` | `items.list` | read-only | not migrated | next-read-only-items |
| `zotron items create` | `items.create` | mutating | Python only | defer-mutating |
| `zotron items update` | `items.update` | mutating | Python only | defer-mutating |
| `zotron items delete` | `items.delete` | destructive | Python only | defer-mutating |
| `zotron items list-trash` | `items.getTrash` | read-only | not migrated | next-read-only-items |
| `zotron items batch-trash` | `items.batchTrash` | mutating/reversible | Python only | defer-mutating |
| `zotron items recent` | `items.getRecent` | read-only | not migrated | next-read-only-items |
| `zotron items fulltext` | `items.getFullText` | read-only | not migrated | next-read-only-items |
| `zotron items add-from-file` | `items.addFromFile` | mutating/import | Python only | defer-mutating |
| `zotron items related` | `items.getRelated` | read-only | not migrated | next-read-only-items |
| `zotron items add-related` | `items.addRelated` | mutating | Python only | defer-mutating |
| `zotron items remove-related` | `items.removeRelated` | mutating | Python only | defer-mutating |
| `zotron items citation-key` | `items.citationKey` | read-only | not migrated | next-read-only-items |
| `zotron search quick` | `search.quick` | read-only | implemented | baseline |
| `zotron search fulltext` | `search.fulltext` | read-only | implemented | baseline |
| `zotron search by-identifier` | `search.byIdentifier` | read-only | implemented | baseline |
| `zotron search advanced` | `search.advanced` | read-only | implemented | baseline |
| `zotron search by-tag` | `search.byTag` | read-only | implemented | baseline |
| `zotron search saved-searches` | `search.savedSearches` | read-only | implemented | baseline |
| `zotron search create-saved` | `search.createSaved` | mutating | Python only | defer-mutating |
| `zotron search delete-saved` | `search.deleteSaved` | mutating | Python only | defer-mutating |
| `zotron tags list` | `tags.list` | read-only | migrated | tags-list |
| `zotron tags rename` | `tags.rename` | mutating | Python only | defer-mutating |
| `zotron tags delete` | `tags.delete` | mutating | Python only | defer-mutating |
| `zotron tags add` | `tags.add` | mutating | Python only | defer-mutating |
| `zotron tags remove` | `tags.remove` | mutating | Python only | defer-mutating |
| `zotron tags batch-update` | `tags.batchUpdate` | mutating | Python only | defer-mutating |
| `zotron export bibtex` | `export.bibtex` | read-only/export | not migrated | future-read-only-export |
| `zotron export ris` | `export.ris` | read-only/export | not migrated | future-read-only-export |
| `zotron export csl-json` | `export.cslJson` | read-only/export | not migrated | future-read-only-export |
| `zotron export bibliography` | `export.bibliography` | read-only/export | not migrated | future-read-only-export |
| `zotron notes list` | `notes.list` | read-only | implemented | baseline |
| `zotron notes get` | `notes.get` | read-only | implemented | baseline |
| `zotron notes create` | `notes.create` | mutating | Python only | defer-mutating |
| `zotron notes update` | `notes.update` | mutating | Python only | defer-mutating |
| `zotron notes delete` | `notes.delete` | mutating | Python only | defer-mutating |
| `zotron notes search` | `notes.search` | read-only | implemented | baseline |
| `zotron attachments list` | `attachments.list` | read-only | implemented | baseline |
| `zotron attachments get` | `attachments.get` | read-only | implemented | baseline |
| `zotron attachments fulltext` | `attachments.fulltext` | read-only | implemented | baseline |
| `zotron attachments add` | `attachments.add` | mutating/file attach | Python only | defer-mutating |
| `zotron attachments add-by-url` | `attachments.addByURL` | mutating/import | Python only; Rust compatibility shim covered by fixture | defer-mutating; prefer `--source-url` in Rust, keep `--url` alias for Python compatibility, and reserve `--endpoint` for the RPC endpoint |
| `zotron attachments path` | `attachments.path` | read-only/path lookup | implemented | baseline |
| `zotron attachments delete` | `attachments.delete` | destructive | Python only | defer-mutating |
| `zotron attachments find-pdf` | `attachments.findPDF` | mutating/import | Python only | defer-mutating |
| `zotron annotations list` | `annotations.list` | read-only | migrated | annotations-list |
| `zotron annotations create` | `annotations.create` | mutating | Python only | defer-mutating |
| `zotron annotations delete` | `annotations.delete` | destructive | Python only | defer-mutating |
| `zotron settings get` | `settings.get` | read-only | implemented | baseline |
| `zotron settings set` | `settings.set` | mutating | Python only | defer-mutating |
| `zotron settings list` | `settings.getAll` | read-only | implemented | baseline |
| `zotron settings set-all` | `settings.setAll` | mutating | Python only | defer-mutating |

## Recommended next slices

1. `next-read-only-system`: migrate `system version`, `libraries`, `library-stats`, `item-types`, `item-fields`, `creator-types`, `current-collection`, and `describe` with fixture parity.
2. `next-read-only-collections`: migrate `collections list`, `get`, `get-items`, and `stats`; include collection-name resolution fixtures because Python may call `collections.list` before the target RPC.
3. `next-read-only-items`: migrate `items list`, `find-duplicates`, `list-trash`, `recent`, `fulltext`, `related`, and `citation-key`.
4. Future read-only namespaces: export/notes/attachments/annotations/settings/tags after the system/collections/items/search read-only slices are stable.
5. Mutating commands: defer until read-only parity, dry-run behavior, destructive-operation UX, and rollback/error fixtures are explicitly specified.
