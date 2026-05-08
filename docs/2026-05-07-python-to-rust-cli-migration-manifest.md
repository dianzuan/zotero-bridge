# Python-to-Rust CLI Migration Manifest

Date: 2026-05-07

This manifest tracks the migration from the legacy Python CLI to the target Rust CLI + Zotero-side JS/XPI RPC stack. As of 2026-05-08, the Rust product surface is a single `zotron` binary; OCR and RAG are subcommands (`zotron ocr ...`, `zotron rag ...`). Legacy Python names `zotron-ocr` and `zotron-rag` are retained only as reference/migration labels in old material, not as current user-facing commands. Python is retained as reference code and parity evidence only; new product behavior should land in Rust and/or the JS XPI unless explicitly marked as a compatibility-only Python fix.

## Migration boundary

- Target stack: Rust CLI/crates plus Zotero-side JavaScript/XPI RPC.
- Legacy stack: Python remains readable reference code for expected behavior, fixtures, and migration comparison.
- Do not add new user-facing Python CLI behavior on this branch unless the change is explicitly a temporary compatibility shim.
- Rust may intentionally diverge from Python implementation details when the contract is clearer, for example external `jq` pipes instead of Python's built-in `--jq`.

## Status legend

- `rust+fixture`: implemented in `crates/zotron-cli` and covered by a JSON fixture under `fixtures/cli-parity/`.
- `python-reference`: not migrated yet; Python remains reference/legacy implementation until Rust+JS parity lands.
- `defer-write`: intentionally deferred because it mutates Zotero state or local files.
- `defer-namespace`: read-only or mixed namespace outside the current `system` / `collections` / `items` slice.

## Top-level commands

| Command | RPC / behavior | Status | Notes |
| --- | --- | --- | --- |
| `zotron ping` | `system.ping` | `rust+fixture` | Keeps Python compact `json.dumps` output spacing. |
| `zotron rpc` | arbitrary method | `rust+fixture` | Escape hatch; fixture covered by Rust contract test, not a separate parity JSON file. |
| `zotron push` | local import + item/attachment writes | `defer-write` | Requires collection resolution, PDF path handling, duplicate behavior. |
| `zotron find-pdfs` | `collections.getItems`, `attachments.list`, `attachments.findPDF` | `defer-write` | Triggers Zotero PDF resolver. |

## System namespace

| Command | RPC method | Status | Fixture |
| --- | --- | --- | --- |
| `zotron system version` | `system.version` | `rust+fixture` | `system-version.json` |
| `zotron system sync` | `system.sync` | `defer-write` | Triggers Zotero sync. |
| `zotron system libraries` | `system.libraries` | `rust+fixture` | `system-libraries.json` |
| `zotron system switch-library` | `system.switchLibrary` | `defer-write` | Changes active library context. |
| `zotron system library-stats` | `system.libraryStats` | `rust+fixture` | `system-library-stats.json` |
| `zotron system item-types` | `system.itemTypes` | `rust+fixture` | `system-item-types.json` |
| `zotron system item-fields` | `system.itemFields` | `rust+fixture` | `system-item-fields.json` |
| `zotron system creator-types` | `system.creatorTypes` | `rust+fixture` | `system-creator-types.json` |
| `zotron system current-collection` | `system.currentCollection` | `rust+fixture` | `system-current-collection.json` |
| `zotron system reload` | `system.reload` | `defer-write` | Reloads the XPI plugin. |
| `zotron system list-methods` | `system.listMethods` | `rust+fixture` | `system-list-methods.json` |
| `zotron system describe` | `system.describe` | `rust+fixture` | `system-describe.json` |

## Collections namespace

| Command | RPC method | Status | Fixture |
| --- | --- | --- | --- |
| `zotron collections list` | `collections.list` | `rust+fixture` | `collections-list.json` |
| `zotron collections tree` | `collections.tree` | `rust+fixture` | `collections-tree.json` |
| `zotron collections get` | `collections.list` then `collections.get` | `rust+fixture` | `collections-get.json` |
| `zotron collections get-items` | `collections.list` then `collections.getItems` | `rust+fixture` | `collections-get-items.json` |
| `zotron collections stats` | `collections.list` then `collections.stats` | `rust+fixture` | `collections-stats.json` |
| `zotron collections rename` | `collections.rename` | `defer-write` | Mutates collection metadata. |
| `zotron collections create` | `collections.create` | `defer-write` | Creates collection. |
| `zotron collections delete` | `collections.delete` | `defer-write` | Deletes collection link. |
| `zotron collections add-items` | `collections.addItems` | `defer-write` | Mutates collection membership. |
| `zotron collections remove-items` | `collections.removeItems` | `defer-write` | Mutates collection membership. |

## Items namespace

| Command | RPC method | Status | Fixture |
| --- | --- | --- | --- |
| `zotron items get` | `items.get` | `rust+fixture` | `items-get.json` |
| `zotron items add-by-doi` | `items.addByDOI` | `defer-write` | Creates items. |
| `zotron items add-by-isbn` | `items.addByISBN` | `defer-write` | Creates items. |
| `zotron items add-by-url` | `items.addByURL` | `defer-write` | Creates items. |
| `zotron items trash` | `items.trash` | `defer-write` | Moves item to trash. |
| `zotron items restore` | `items.restore` | `defer-write` | Restores item. |
| `zotron items find-duplicates` | `items.findDuplicates` | `rust+fixture` | `items-find-duplicates.json` |
| `zotron items merge-duplicates` | `items.mergeDuplicates` | `defer-write` | Merges items. |
| `zotron items list` | `items.list` | `rust+fixture` | `items-list.json` |
| `zotron items create` | `items.create` | `defer-write` | Creates items. |
| `zotron items update` | `items.update` | `defer-write` | Mutates item fields. |
| `zotron items delete` | `items.delete` | `defer-write` | Permanent delete. |
| `zotron items list-trash` | `items.getTrash` | `rust+fixture` | `items-list-trash.json` |
| `zotron items batch-trash` | `items.batchTrash` | `defer-write` | Moves multiple items to trash. |
| `zotron items recent` | `items.getRecent` | `rust+fixture` | `items-recent.json` |
| `zotron items fulltext` | `items.getFullText` | `rust+fixture` | `items-fulltext.json` |
| `zotron items add-from-file` | `items.addFromFile` | `defer-write` | Imports local file. |
| `zotron items related` | `items.getRelated` | `rust+fixture` | `items-related.json` |
| `zotron items add-related` | `items.addRelated` | `defer-write` | Mutates relationship. |
| `zotron items remove-related` | `items.removeRelated` | `defer-write` | Mutates relationship. |
| `zotron items citation-key` | `items.citationKey` | `rust+fixture` | `items-citation-key.json` |

## Search namespace

| Command | RPC method | Status | Fixture |
| --- | --- | --- | --- |
| `zotron search quick` | `search.quick`; with `--collection`, `collections.getItems` then local metadata filter | `rust+fixture` | `search-quick.json`; `--collection` covered by Rust contract test |
| `zotron search fulltext` | `search.fulltext` | `rust+fixture` | `search-fulltext.json` |
| `zotron search by-identifier` | `search.byIdentifier` | `rust+fixture` | `search-by-identifier.json` |
| `zotron search advanced` | `search.advanced` | `rust+fixture` | `search-advanced.json` |
| `zotron search by-tag` | `search.byTag` | `rust+fixture` | `search-by-tag.json` |
| `zotron search saved-searches` | `search.savedSearches` | `rust+fixture` | `search-saved-searches.json` |
| `zotron search create-saved` | `search.createSavedSearch` | `defer-write` | Creates saved searches. |
| `zotron search delete-saved` | `search.deleteSavedSearch` | `defer-write` | Deletes saved searches. |

## Other namespaces

| Namespace | Commands | Status | Notes |
| --- | --- | --- | --- |
| `tags` | `list`, `rename`, `delete`, `add`, `remove`, `batch-update` | `defer-namespace` | Mixed read/write namespace. |
| `attachments` | `list`, `get`, `fulltext`, `path` are `rust+fixture`; `add`, `add-by-url`, `delete`, `find-pdf` are deferred | mixed | Read-only attachment inspection is migrated; writes/import/PDF lookup remain pending Rust+JS work. Rust `attachments add-by-url` keeps compatibility coverage for the deferred write path and normalizes the source-file option to `--source-url` with legacy `--url` accepted as an alias; RPC endpoint remains `--endpoint`. |
| `annotations` | `list`, `create`, `delete` | `defer-namespace` | Mixed read/write namespace. |
| `notes` | `list`, `get`, `search` are `rust+fixture`; `create`, `update`, `delete` are `defer-write` | mixed | Read-only note inspection is migrated; writes remain pending Rust+JS work. |
| `export` | `bibtex`, `ris`, `csl-json`, `bibliography` | `defer-namespace` | Read-only but outside current slice; needs output format parity. |
| `settings` | `get`, `list` are `rust+fixture`; `set`, `set-all` are `defer-write` | mixed | Read-only preference inspection is migrated; writes remain pending Rust+JS work. |

## RAG entrypoint (`zotron-rag`)

| Command | RPC / behavior | Status | Fixture | Notes |
| --- | --- | --- | --- | --- |
| `zotron-rag index` | collection scan + local vector index writes | `python-reference` | — | Pending Rust artifact/vector-store parity. |
| `zotron-rag index-artifacts` | chunk artifact embedding + optional Zotero attachment writes | `python-reference` | — | Requires Rust artifact writing and embedding providers. |
| `zotron-rag search` | local/artifact vector search | `python-reference` | — | Requires Rust embedder/vector-store parity. |
| `zotron-rag status` | local `~/.local/share/zotron/rag/<collection>.json` status | `rust+fixture` | `rag-parity/status-not-indexed.json` | Rust covers the low-risk status command, including not-indexed output and JSON store summary. |
| `zotron-rag hits --zotero` | `rag.searchHits` | `rust+fixture` | `rag-parity/hits-zotero-json.json` | Rust covers the XPI-backed path only; local artifact/vector hits remain pending Rust work. |
| `zotron-rag cite` | local vector citations | `python-reference` | — | Citation formatting and retrieval internals pending Rust parity. |

## Current Rust parity surface

The Rust CLI now supports parity fixtures for:

- Top-level: `ping`, `rpc` contract, `search quick`.
- Tags read-only commands: `list`.
- Annotations read-only commands: `list`.
- Settings read-only commands: `get`, `list`.
- RAG entrypoint first slice: `zotron-rag status`, `zotron-rag hits --zotero`.
- System read-only commands: `version`, `libraries`, `library-stats`, `item-types`, `item-fields`, `creator-types`, `current-collection`, `list-methods`, `describe`.
- Collections read-only commands: `list`, `tree`, `get`, `get-items`, `stats`.
- Items read-only commands: `get`, `find-duplicates`, `list`, `list-trash`, `recent`, `fulltext`, `related`, `citation-key`.
- Search read-only commands: `quick`, `fulltext`, `by-identifier`, `advanced`, `by-tag`, `saved-searches`.
- Notes read-only commands: `list`, `get`, `search`.
- Attachments read-only commands: `list`, `get`, `fulltext`, `path`.
- Settings read-only commands: `get`, `list`.
- RAG entrypoint first slice: `zotron-rag status`, `zotron-rag hits --zotero`.

Known gaps for this slice:

- Rust basic `zotron` does not expose Python `--jq` or `--output table`; parity fixtures exercise default JSON output only and callers should pipe JSON to external `jq` when filtering is needed.
- `zotron-rag hits` Rust parity is limited to the RPC-backed `--zotero` path; local artifact/vector search, `index`, `index-artifacts`, `search`, and `cite` remain pending Rust work with Python as reference.
- Collection name resolution matches Python's exact and normalized fuzzy success path, but current Rust errors are plain CLI errors rather than Python's structured JSON envelopes.

## Basic CLI normalization checklist

- [x] `attachments add-by-url` naming audited: source URL is exposed as `--source-url` in the Rust CLI while legacy `--url` continues to parse for Python/fixture compatibility.
- [x] RPC endpoint naming audited for `attachments add-by-url`: endpoint selection remains isolated behind `--endpoint`, so `--url`/`--source-url` cannot be confused with the JSON-RPC endpoint.
- [x] Compatibility is locked without breaking Python parity: the shared parity fixture keeps legacy `--url`, while focused Rust contract tests cover both normalized `--source-url` and legacy `--url`.
- [ ] Final team integration must re-run the full basic normalization gate after Worker 1/2/3 changes are merged: `cargo fmt --check`, `cargo check -p zotron-cli`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, Python fixture parity pytest, `npx tsc --noEmit`, and help/error smoke checks.
