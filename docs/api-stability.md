# zotron API Stability

This document is the contract for external consumers of zotron.

## SDK (Python `zotron` package)

The following symbols, importable from `zotron`, are **stable** —
breaking changes require a major-version bump:

### Core RPC client
- `ZoteroRPC` — class, constructor takes `url: str`
- `ZoteroRPC.call(method: str, params: dict | None = None) -> Any`

### Push orchestration
- `push_item(rpc, item_json: dict, *, pdf_path=None, collection=None, on_duplicate="skip") -> PushResult`
- `resolve_collection(rpc, name_or_id: str | int) -> int`
- `find_duplicate(rpc, item_json: dict) -> int | None`
- `check_pdf_magic(path) -> bool`
- `PushResult` — dataclass with fields `status: Literal["created", "updated", "skipped_duplicate", "failed"]`, `zotero_item_key: str | None`, `pdf_attached: bool`, `pdf_size_bytes: int`, `error: dict | None`; convenience property `pdf_size_kb: int`

### Errors
- `ZotronError` — base
- `ZoteroUnavailable`
- `CollectionNotFound`
- `CollectionAmbiguous` (has `.candidates: list[dict]`)
- `InvalidPDF`

### Citation API (since 0.2.0)
- `Citation` — dataclass; fields `item_key: str`, `attachment_key: str | int | None`, `title: str`, `authors: str`, `section: str`, `chunk_index: int`, `text: str`, `score: float`; method `zotero_uri() -> str`
- `retrieve_with_citations(query: str, *, store_path, embedder, top_k=10) -> list[Citation]`
- `format_citation_markdown(c: Citation) -> str`
- `format_citation_json(c: Citation) -> dict`

Anything not in this list is internal — modules with leading underscore
(`_output`, `_paginate`) are explicitly private.

## CLI (`zotron` binary)

### Stable command surface

All subcommands listed in `zotron --help` are stable. Their flags
are stable. New flags may be added; existing flags will not be removed
or have their semantics changed without a major-version bump.

The `rpc` escape hatch is stable as a calling pattern; the set of XPI
methods reachable through it tracks the XPI version, not zotron.

### Stable JSON envelope

**Success:** stdout is JSON. Shape depends on the command — see
the per-command section below.

**Error envelope:**
```json
{
  "ok": false,
  "error": {
    "code": "UPPERCASE_TOKEN",
    "message": "human-readable string"
  }
}
```

Error codes (all stable; new codes may be added):
- `INVALID_JSON` — params didn't parse
- `INVALID_ARGS` — bad CLI flags / argument values
- `ZOTERO_UNAVAILABLE` — Zotero process not reachable
- `RPC_ERROR` — XPI returned a JSON-RPC error
- `COLLECTION_NOT_FOUND`
- `COLLECTION_AMBIGUOUS` — additionally has `candidates: [...]`
- `INVALID_PDF`
- `ZOTERO_ERROR` — generic XPI-side failure

**Dry-run envelope** (when `--dry-run` is passed to a write command):
```json
{
  "ok": true,
  "dryRun": true,
  "wouldCall": "items.addByDOI",
  "wouldCallParams": { ... }
}
```

`push --dry-run` uses `wouldPush` instead of `wouldCall` because it
chains multiple RPCs internally.

### Per-command stable fields

Refer to `docs/superpowers/specs/2026-04-23-xpi-api-prd.md` for the XPI
response shape; CLI commands forward those responses verbatim.

## Versioning

zotron follows semver:
- **major** — anything in this document changes
- **minor** — new methods / commands / flags
- **patch** — fixes that don't change observable contract

The XPI and Python SDK ship together from this monorepo; their version
numbers move in lockstep.
