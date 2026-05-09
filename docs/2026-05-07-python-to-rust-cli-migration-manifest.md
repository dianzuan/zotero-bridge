# Python-to-Rust CLI Migration Manifest

Date: 2026-05-07

Updated: 2026-05-09

This manifest tracks the migration from the legacy Python CLI to the target
Rust CLI + Zotero-side JS/XPI RPC stack.

## Current Boundary

- Target stack: Rust CLI/crates plus Zotero-side JavaScript/XPI RPC.
- User-facing command: `zotron`.
- OCR/RAG surface: `zotron ocr ...` and `zotron rag ...`.
- Legacy Python names such as `zotron-ocr`, `zotron-rag`, `zotero-ocr`, and
  `zotero-rag` are historical labels only.
- Python remains readable reference code for behavior comparison, fixtures, and
  migration archaeology. New product behavior should land in Rust and/or the
  JS XPI.

## Migrated Rust Surface

The Rust binary currently exposes the broad Zotero command surface:

```text
zotron ping
zotron rpc
zotron push
zotron system ...
zotron search ...
zotron items ...
zotron collections ...
zotron notes ...
zotron attachments ...
zotron settings ...
zotron tags ...
zotron export ...
zotron annotations ...
zotron ocr ...
zotron rag ...
zotron find-pdfs
```

Important command details:

- `zotron search quick --collection NAME QUERY` supports collection-limited
  quick search.
- `zotron collections get-items NAME` is the direct collection-member listing
  command; `collections items` is not the product command.
- Rust output is JSON-first. Use external `jq` for filtering.
- Mutating commands should support clear dry-run or explicit side-effect
  behavior before they are documented as automation-safe.

## OCR/RAG Migration Status

Implemented on the Rust branch:

- `zotron ocr providers`
- `zotron ocr provider-json`
- `zotron ocr status`
- `zotron ocr parse-pdf`
- `zotron rag embedding-providers`
- `zotron rag embedding-json`
- `zotron rag status`
- `zotron rag hits`

`zotron ocr parse-pdf` now supports the MinerU parse pipeline and writes hidden
per-PDF sidecars under Zotero attachment storage:

```text
storage/<attachment-key>/.zotron/
  ocr/latest.raw.json
  ocr/latest.blocks.jsonl
  chunks/chunks.v1.jsonl
```

Live smoke evidence from the branch:

- Zotero item key: `2TGDLKDZ`
- PDF attachment key: `QI2YI74W`
- MinerU sidecar result: 187 normalized blocks and 43 chunks

## Remaining Migration Work

The Rust branch is not done. Remaining work is now about quality and product
completion, not basic command discovery:

- Build/write production embedding indexes (`vectors.jsonl`) from chunk
  sidecars.
- Add GLM/Paddle structured-output ingestion to the same sidecar contract.
- Add higher-level locate/apply commands for evidence-to-PDF annotation flows.
- Harden mutation safety with dry-run and regression tests.
- Validate cross-platform sidecar path behavior on Linux/macOS/Windows.
- Decide later whether to publish a single `zotron` crate to crates.io or keep
  internal crates private.

## Verification Expectations

For code changes touching this surface, run the relevant subset of:

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm test
npx tsc --noEmit
```

For live Zotero changes, use the Zotero `test` collection and record which
commands were actually run. Do not call a narrow command smoke test a full
feature test unless OCR, retrieval, and annotation paths were all exercised.
