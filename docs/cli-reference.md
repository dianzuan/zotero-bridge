# Zotron CLI Command Reference

Generated: 2026-06-03

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
  methods             List RPC methods, or describe a specific method
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
  add               Add an item by DOI, ISBN, URL, local file, or manual entry (--type + --field)
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
  fulltext          Retrieve the full-text content of an item's attachment. Prefers the clean OCR sidecar text, falling back to Zotero's built-in extraction
  related           List items related to the given item
  citation-key      Get the citation key for an item
  path              Get the local filesystem path of an item's PDF attachment
  attachments       List attachments belonging to an item
  find-pdfs         Batch find missing PDFs in a collection via Zotero's resolver chain
```

The `fulltext` command takes an optional `--ocr` flag that forces OCR-only output
(errors if the item has no OCR sidecar, with no Zotero fallback).

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
                                 [default: http://www.zotero.org/styles/apa]
      --html                     Output HTML instead of plain text (only for bibliography format)
      --url <URL>                [default: http://127.0.0.1:23119/zotron/rpc]
```

## zotron annotations
```
List, create, and delete PDF annotations

Usage: zotron annotations <COMMAND>

Commands:
  list          List annotations on a PDF. Accepts an item key (auto-resolves to PDF) or attachment key
  create        Create a new annotation on a PDF. Accepts an item key (auto-resolves to PDF) or attachment key
  create-batch  Batch-create annotations from a JSON array on stdin or --file
  locate        Locate a text quote in a PDF without creating an annotation. Returns page index and rects if found
  delete        Delete an annotation by key

Examples:
  zotron annotations list YR5BUGHG
  zotron annotations create YR5BUGHG --quote "text to highlight"  # locates text headlessly, no PDF viewer required
  zotron annotations create YR5BUGHG --type image --position '{"pageIndex":0,"rects":[[10,20,30,40]]}'
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
  reindex    Re-chunk and re-embed existing OCR results without re-running OCR
```

### zotron ocr process
```
Parse a Zotero PDF and write hidden sidecar OCR/RAG artifacts. Provider read
from Zotero settings unless --provider is given.

Usage: zotron ocr process [OPTIONS]

Options:
      --provider <PROVIDER>      Override OCR provider (default: Zotero settings ocr.provider)
      --parent <PARENT>          Parent item key. Required unless --collection is given
      --collection <COLLECTION>  Collection name (fuzzy) or key: OCR every item in the collection
      --attachment <ATTACHMENT>  PDF attachment key (auto-resolved from --parent; ignored with --collection)
      --source-url <SOURCE_URL>  Public URL for MinerU cloud parsing
      --result-dir <RESULT_DIR>  Already-extracted MinerU result directory (offline replay)
      --result-zip <RESULT_ZIP>  Already-downloaded MinerU result zip (offline replay)
      --chunk-chars <CHUNK_CHARS>  [default: 1200]
```

Pass `--parent <itemKey>` to OCR a single item, or `--collection <name|key>` to
OCR every item in a collection. In `--collection` mode the attachment is
auto-resolved per item, items with no PDF attachment are skipped (not errors),
and the output reports `processed` / `skipped` / `failed` counts plus a per-item
`items` array. `--collection` cannot be combined with `--result-dir` /
`--result-zip` (those are single-item replay inputs).

### zotron ocr reindex
```
Re-chunk and re-embed existing OCR results without re-running OCR

Usage: zotron ocr reindex [OPTIONS]

Options:
      --collection <COLLECTION>
      --key <KEY>
      --stale-only                 Only reindex items with stale schema version
      --chunk-chars <CHUNK_CHARS>  [default: 1200]
      --url <URL>                  [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                       Print help
```

Rebuilds chunk sidecars and embedding vectors from already-extracted blocks — no
OCR provider call, so it is free. Chunk sidecars carry a `schema_version` header
line; `--stale-only` reads it and skips sidecars already at the current schema,
so it only rebuilds what is out of date. **Run `zotron ocr reindex --stale-only`
once after upgrading** so pre-schema-versioning v1 sidecars are rebuilt to the
current schema (otherwise stale chunks get mixed into retrieval). Reindex also
(re)generates embedding vectors, enabling semantic retrieval for documents that
were only chunked before.

## zotron rag
```
Build and search retrieval artifacts

Usage: zotron rag <COMMAND>

Commands:
  providers  Print supported embedding provider contracts
  embed      Execute an embedding provider request from JSON and emit vectors
  status     Show index status for a collection
  search     Emit academic-zh retrieval hits with item_key/title/text provenance
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### zotron rag search
```
Emit academic-zh retrieval hits with item_key/title/text provenance

Usage: zotron rag search [OPTIONS] <QUERY>

Arguments:
  <QUERY>

Options:
      --collection <COLLECTION>
      --key <KEYS>                               Limit retrieval to one or more Zotero item keys
      --top-spans-per-item <TOP_SPANS_PER_ITEM>  [default: 3]
      --include-fulltext-spans
      --limit <TOP_K>                            [default: 50]
      --output <OUTPUT>                          [default: json] [possible values: json, jsonl]
      --url <URL>                                [default: http://127.0.0.1:23119/zotron/rpc]
  -h, --help                                     Print help
```

Hybrid retrieval (BM25 + vector + RRF fusion) is the default. Falls back to
keyword matching when no vector index exists. Embedding provider is configured
in Zotero → Settings → Zotron panel. 10 providers supported: Ollama (default),
OpenAI, Volcengine, DashScope, Zhipu, Jina, SiliconFlow, Voyage, Cohere, Custom.

After fusion the results pass through a quality pipeline: an optional cross-encoder
**rerank**; a **dynamic cutoff** (score floor + largest-gap trim, active when a
reranker is configured) that returns only as many hits as are relevant instead of
a fixed count; **MMR diversity** that drops near-duplicate spans (relevance scores
are min-max normalized to 0..1 first, so diversity works in every mode); and a
**token budget** bounded by min/max K.

Output fields:
- `mode` (top-level) — the retrieval path actually used: `hybrid`, `dense`, or
  `lexical`. If embedding vectors or the query embedding are unavailable the
  search falls back to lexical (BM25) and reports `lexical` here instead of
  silently returning nothing.
- `score_kind` (per hit) — origin/scale of the hit's `score`: `rerank` (0..1
  reranker score), `rrf` (fused rank score), `cosine` (vector similarity), or
  `bm25` (keyword score).

`zotron rag status` reports `embeddings_available` / `total_vectors` so you can
tell whether semantic (dense) retrieval is possible before searching.

Retrieval pipeline settings (Zotero → Settings → Zotron panel):
- `rag.retrievalMode` — `hybrid` (default) | `dense` | `lexical`
- `rag.minK` (default 3) / `rag.maxK` (default 20) — result-count bounds
- `rag.tokenBudget` (default 6000) — total token cap for returned spans
- `rag.mmrLambda` (default 0.7) — diversity trade-off (higher favors relevance)
- `rerank.provider` / `rerank.apiKey` / `rerank.model` / `rerank.apiUrl` — reranker config
- `rerank.candidateCount` (default 30) — how many fused candidates to rerank
- `rerank.scoreFloor` (default 0.1) — drop reranked hits below this score
- `rerank.gapThreshold` (default 0.15) — trim the tail at the largest score gap

## zotron sources
```
Discover and manage source plugins (`zotron-*` on PATH)

Usage: zotron sources <COMMAND>

Commands:
  list  List all discovered source plugins on PATH (the default action)
  sync  Symlink plugin skills into the Claude Code plugin's skills directory
```

External academic sources are standalone `zotron-*` binaries on PATH. Core
discovers them via `zotron sources list`; each implements a `manifest`
subcommand. Plugins emit Zotero JSON to stdout, piped to `zotron push`.
