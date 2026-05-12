# Zotron CLI Command Reference

Generated: 2026-05-12

## zotron ping
```
Check that Zotero is running with the Zotron XPI enabled

Usage: zotron ping [OPTIONS]

Options:
      --url <URL>  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help       Print help
```

## zotron rpc
```
Generic RPC escape hatch

Usage: zotron rpc [OPTIONS] <METHOD> [PARAMS_JSON]

Arguments:
  <METHOD>
  [PARAMS_JSON]  [default: {}]

Options:
      --url <URL>              [default: http://127.0.0.1:23119/zotron/rpc]
      --paginate
      --page-size <PAGE_SIZE>  [default: 100]
  -h, --help                   Print help
```

## zotron push
```
Push prepared Zotero JSON (from file or stdin) to Zotero

Usage: zotron push [OPTIONS] <JSON_FILE>

Arguments:
  <JSON_FILE>  Path to a JSON file, or "-" to read from stdin

Options:
      --pdf <PDF>                    Optional PDF attachment path
      --collection <COLLECTION>      Collection name (fuzzy) or key
      --on-duplicate <ON_DUPLICATE>  Duplicate handling: skip | update | create [default: skip]
      --url <URL>                    [default: http://127.0.0.1:23119/zotron/rpc]
      --dry-run                      Parse input + resolve collection only; do not push to Zotero
  -h, --help                         Print help
```

## zotron system
```
System and plugin introspection commands

Usage: zotron system <COMMAND>

Commands:
  version             Show XPI version and exposed method metadata
  libraries           List all libraries (user + groups)
  library-stats       Get statistics for the current (or specified) library
  schema              Show item type schema. Without --type, lists all types.
                      With --type, shows fields and creator types
  current-collection  Get the currently selected Zotero collection (or null)
  list-methods        List all RPC methods exposed by the XPI
  describe            Describe one or all RPC methods (schema / signatures)
```

## zotron search
```
Search items by text, tag, identifier, or structured conditions

Usage: zotron search [OPTIONS] [QUERY] [COMMAND]

Commands:
  saved-searches  List all saved searches in the library
  create-saved    Create a saved search with one or more conditions
  delete-saved    Delete a saved search by key

Arguments:
  [QUERY]  Search query (title/creator/year by default; PDF content with --fulltext)

Options:
      --fulltext                 Search inside PDF full-text content instead of metadata
      --author <AUTHOR>          Filter by author/creator name (contains match)
      --after <AFTER>            Filter by date after (YYYY or YYYY-MM-DD)
      --before <BEFORE>          Filter by date before (YYYY or YYYY-MM-DD)
      --journal <JOURNAL>        Filter by journal/publication title (contains match)
      --tag <TAG>                Filter by tag (exact match)
      --doi <DOI>                Find by DOI
      --isbn <ISBN>              Find by ISBN
      --issn <ISSN>              Find by ISSN
      --collection <COLLECTION>  Limit results to a collection name or key
      --limit <LIMIT>            [default: 50]
      --offset <OFFSET>          [default: 0]
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
```

## zotron items
```
Inspect and manage Zotero items

Usage: zotron items <COMMAND>

Commands:
  add               Add an item by DOI, ISBN, URL, or local file
  create            Create a new item of the given type
  update            Update fields on an existing item
  delete            Permanently delete an item
  trash             Move one or more items to trash
  restore           Restore a trashed item
  merge-duplicates  Merge a group of duplicate items
  add-related       Add a related-item link between two items
  remove-related    Remove a related-item link between two items
  get               Print the full serialization of an item by key
  list              List items in the library with optional sorting and pagination
  find-duplicates   Run Zotero's duplicate scan and print groups
  recent            List recently added or modified items
  fulltext          Retrieve the full-text content of an item's attachment
  related           List items related to the given item
  citation-key      Get the citation key for an item
```

## zotron collections
```
Inspect Zotero collections

Usage: zotron collections <COMMAND>

Commands:
  list          List all collections in the user library (flat)
  tree          Print the collection hierarchy as a tree
  get           Get a single collection's metadata
  get-items     List all items in a collection [aliases: items]
  stats         Show item/attachment/note/subcollection counts for a collection
  rename        Rename a collection
  create        Create a collection, optionally nested under a parent
  delete        Delete a collection
  add-items     Add existing items to a collection
  remove-items  Remove items from a collection
```

## zotron notes
```
Inspect Zotero notes

Usage: zotron notes <COMMAND>

Commands:
  list    List notes attached to a parent item
  get     Get a single note by key
  create  Create a note attached to a parent item
  update  Update the content of an existing note
  delete  Delete a note by key
  search  Search notes by text content
```

## zotron attachments
```
Inspect Zotero attachments

Usage: zotron attachments <COMMAND>

Commands:
  list      List attachments belonging to a parent item
  get       Get a single attachment by key
  fulltext  Get full-text content of an attachment
  path      Get the local filesystem path of an attachment
  add       Attach a local file or remote URL to an item
  delete    Delete an attachment
  find-pdf  Trigger Zotero's Find Available PDF for a parent item
```

## zotron settings
```
Inspect Zotero preferences

Usage: zotron settings <COMMAND>

Commands:
  get   Get a single Zotero preference value
  list  List all Zotero preferences as a key->value dict [aliases: get-all]
  set   Set one or more Zotero preferences (key value pairs), or bulk-set from a JSON file
```

## zotron tags
```
Inspect and manage Zotero tags

Usage: zotron tags <COMMAND>

Commands:
  list    List all tags in the library (flat)
  rename  Rename a tag across all items
  delete  Delete a tag library-wide
  add     Add tags to one or more items
  remove  Remove tags from one or more items
```

## zotron export
```
Export items as BibTeX, RIS, CSL-JSON, or formatted bibliography

Usage: zotron export [OPTIONS] [KEYS]...

Arguments:
  [KEYS]...  Item keys to export

Options:
      --format <FORMAT>          Output format: bibtex, ris, csl-json, bibliography [default: bibtex]
      --collection <COLLECTION>  Export all items from this collection (name or key)
      --style <STYLE>            Citation style URL (only for bibliography format)
                                 [default: http://www.zotero.org/styles/gb-t-7714-2015-numeric]
      --html                     Output HTML instead of plain text (only for bibliography format)
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
```

## zotron annotations
```
List, create, and delete PDF annotations

Usage: zotron annotations <COMMAND>

Commands:
  list    List annotations on a PDF attachment
  create  Create a new annotation on a PDF attachment
  delete  Delete an annotation by key
```

## zotron ocr
```
OCR PDFs and manage raw/block/chunk evidence artifacts

Usage: zotron ocr <COMMAND>

Commands:
  providers  Print supported OCR provider contracts
  run        Execute an OCR provider request from JSON and emit normalized blocks
  status     Show OCR statistics for a collection
  process    Parse a Zotero PDF through MinerU and write hidden sidecar OCR/RAG artifacts
```

## zotron rag
```
Build and search retrieval artifacts

Usage: zotron rag <COMMAND>

Commands:
  providers  Print supported embedding provider contracts
  embed      Execute an embedding provider request from JSON and emit vectors
  status     Show index status for a collection
  search     Emit academic-zh retrieval hits with item_key/title/text provenance
```

## zotron find-pdfs
```
Batch fill missing PDFs in a collection via Zotero's resolver chain

Usage: zotron find-pdfs [OPTIONS] --collection <COLLECTION>

Options:
      --collection <COLLECTION>
      --limit <LIMIT>            [default: 0]
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
```
