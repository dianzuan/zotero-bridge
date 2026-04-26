# RPC method reference

All RPC methods exposed by the `zotero-bridge` XPI, callable via:

```bash
zotero-bridge rpc <namespace>.<method> '<json-params>'
```

Or directly over HTTP at `POST http://localhost:23119/zotero-bridge/rpc` with a JSON-RPC 2.0 envelope.

## Namespace summary

| Namespace | Methods | What it does |
|---|---|---|
| `items.*` | 17 | CRUD on Zotero items, add by DOI/URL/ISBN/file, recent, trash, duplicates, related |
| `collections.*` | 12 | List, create, rename, move, tree, items in collection |
| `attachments.*` | 6 | List attachments, get fulltext (cache-file backed), get path, find PDF |
| `notes.*` | 6 | Notes CRUD, annotations, search inside notes |
| `search.*` | 8 | Quick / fulltext / by-tag / by-identifier / advanced; saved searches |
| `tags.*` | 6 | List, add, remove, rename, delete (cross-library) |
| `export.*` | 5 | BibTeX / CSL-JSON / RIS / CSV / bibliography (CiteProc) |
| `settings.*` | 4 | Plugin-side preferences (e.g. OCR provider, embedding model) |
| `system.*` | 10 | Ping, version, libraries, switchLibrary, sync, currentCollection, `system.reload` (self-reload for dev) |

Total: 77 methods.

## items.*

| Method | Purpose |
|---|---|
| `items.get` | Fetch a single item's full metadata by ID |
| `items.create` | Create an item with fields, creators, tags |
| `items.update` | Update fields, creators, or tags on an existing item |
| `items.delete` | Permanently delete an item |
| `items.trash` | Move item to trash |
| `items.restore` | Restore item from trash |
| `items.getTrash` | List items in trash |
| `items.batchTrash` | Move many items to trash in one call |
| `items.getRecent` | Recently added or modified items |
| `items.addByDOI` | Add via DOI (Zotero's translator pipeline) |
| `items.addByURL` | Add via web URL |
| `items.addByISBN` | Add a book via ISBN |
| `items.addFromFile` | Import a local file (PDF, etc.) as an item |
| `items.findDuplicates` | List likely-duplicate item groups |
| `items.mergeDuplicates` | Merge a duplicate group, keeping one master |
| `items.getRelated` | List items related to this one |
| `items.addRelated` | Link two items as related |
| `items.removeRelated` | Unlink related items |
| `items.citationKey` | Better-BibTeX citation key for this item |

## collections.*

| Method | Purpose |
|---|---|
| `collections.list` | Flat list of all collections in the active library |
| `collections.get` | Fetch a single collection by ID |
| `collections.getItems` | List items in a collection |
| `collections.getSubcollections` | Direct children of a collection |
| `collections.tree` | Full collection tree for the current library |
| `collections.create` | Create a collection (optional `parentId`) |
| `collections.rename` | Rename a collection |
| `collections.delete` | Delete a collection (does not delete its items) |
| `collections.move` | Re-parent a collection |
| `collections.addItems` | Add items into a collection |
| `collections.removeItems` | Remove items from a collection (does not delete them) |
| `collections.stats` | Item counts and basic stats per collection |

## attachments.*

| Method | Purpose |
|---|---|
| `attachments.list` | List attachments for a parent item |
| `attachments.getFulltext` | Extracted PDF fulltext (cache-file backed) |
| `attachments.add` | Attach a local file to an item |
| `attachments.addByURL` | Attach a remote URL as a linked attachment |
| `attachments.getPath` | Resolve the on-disk path of an attachment |
| `attachments.findPDF` | Find the primary PDF attachment for an item |

## notes.*

| Method | Purpose |
|---|---|
| `notes.get` | Get notes for a parent item |
| `notes.create` | Create a note (HTML body, optional parent) |
| `notes.update` | Update a note's body |
| `notes.search` | Search inside note bodies |
| `notes.getAnnotations` | PDF annotations / highlights for an item |
| `notes.createAnnotation` | Create an annotation on a PDF attachment |

## search.*

| Method | Purpose |
|---|---|
| `search.quick` | Title/creator/year keyword search (the default) |
| `search.advanced` | Multi-condition search (`field`/`op`/`value` tuples) |
| `search.fulltext` | Search inside indexed PDF text |
| `search.byTag` | Items carrying a given tag |
| `search.byIdentifier` | Look up by DOI / ISBN / arXiv / PMID |
| `search.savedSearches` | List the user's saved searches |
| `search.createSavedSearch` | Persist a saved search |
| `search.deleteSavedSearch` | Delete a saved search |

## tags.*

| Method | Purpose |
|---|---|
| `tags.list` | All tags in the active library |
| `tags.add` | Add tags to an item |
| `tags.remove` | Remove tags from an item |
| `tags.rename` | Rename a tag (library-wide) |
| `tags.delete` | Delete a tag (library-wide) |
| `tags.batchUpdate` | Add/remove tags across many items in one call |

## export.*

| Method | Purpose |
|---|---|
| `export.bibtex` | BibTeX for given item IDs |
| `export.cslJson` | CSL-JSON (programmatic citation data) |
| `export.ris` | RIS (EndNote etc.) |
| `export.csv` | CSV table of items |
| `export.bibliography` | Formatted bibliography via CiteProc (default GB/T 7714 for zh) |

## settings.*

| Method | Purpose |
|---|---|
| `settings.get` | Read one plugin-side preference |
| `settings.set` | Write one plugin-side preference |
| `settings.getAll` | Read all plugin-side preferences |
| `settings.setAll` | Bulk-write plugin-side preferences |

## system.*

| Method | Purpose |
|---|---|
| `system.ping` | Liveness check (`{status, timestamp}`) |
| `system.version` | Zotero version + plugin version + method count |
| `system.libraries` | List all available libraries |
| `system.switchLibrary` | Set the active library |
| `system.libraryStats` | Item / collection / tag counts for a library |
| `system.itemTypes` | Available Zotero item types |
| `system.itemFields` | Valid fields for a given item type |
| `system.creatorTypes` | Valid creator types for an item type |
| `system.sync` | Trigger a Zotero sync |
| `system.currentCollection` | The collection currently selected in the Zotero UI |
| `system.reload` | Hot-reload the plugin (dev-only) |

---

Full PRD with return shapes, error codes, and conventions: `docs/superpowers/specs/2026-04-23-xpi-api-prd.md`
