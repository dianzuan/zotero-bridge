# Zotron RAG/OCR TDD Tasklist

Date: 2026-04-27

Updated: 2026-05-09

This tasklist is now interpreted under the Rust migration branch. The current
goal is a structure-first evidence pipeline for agents, not a Python note
preview pipeline.

## Current Baseline

- Public command is `zotron`.
- OCR commands live under `zotron ocr ...`.
- RAG commands live under `zotron rag ...`.
- Python-era standalone names are historical references only.
- OCR/RAG artifacts are hidden per-PDF sidecars under the Zotero PDF attachment
  storage directory.
- `zotron ocr parse-pdf` has a MinerU path that writes raw provider output,
  normalized blocks, chunks, Markdown preview, and assets.
- `zotron rag hits --zotero` exists for XPI-backed retrieval.

## Done

- Provider/provider-json scaffolding for OCR and embeddings.
- MinerU parse-pdf pipeline from upload/source/result-dir inputs.
- Normalized block output with key-first schema.
- Structure-first chunk output with `chunk_key` and `block_keys`.
- Hidden sidecar storage under `storage/<attachment-key>/.zotron/`.
- Docs and skills updated away from `zotron-ocr` / `zotron-rag` as public
  commands.

## Next TDD Lanes

### Lane 1: Embedding Index Sidecar

Goal: turn `chunks.v1.jsonl` into a rebuildable vector sidecar.

Tests first:

- Missing `vectors.jsonl` reports not indexed.
- Rebuilding vectors preserves chunk provenance.
- Provider/model/dimension changes mark vectors stale.
- Removing vectors does not remove blocks or chunks.

Implementation:

- Write `embeddings/vectors.jsonl`.
- Store provider, model, dimension, source chunk hash, and created timestamp.
- Keep lexical retrieval available when embeddings are missing.

### Lane 2: GLM/Paddle Normalization

Goal: bring GLM and Paddle parser results into the same block/chunk sidecar
schema as MinerU.

Tests first:

- Mock GLM response normalizes into blocks/chunks.
- Mock Paddle sync response normalizes into blocks/chunks.
- Mock Paddle async JSONL result normalizes into blocks/chunks.
- Tables become table chunks; figure/image pixel content remains asset-backed
  metadata unless read by a VLM on demand.

Implementation:

- Add provider parsers without adding new public command names.
- Keep raw provider JSON for re-normalization.
- Preserve Markdown only as a preview/debug artifact.

### Lane 3: Retrieval Over Sidecars

Goal: retrieve original evidence spans directly from sidecar chunks.

Tests first:

- Query returns chunk text with `item_key`, `attachment_key`, `chunk_key`,
  `block_keys`, page range, section path, and score/source metadata.
- Exact/lexical matches work without embeddings.
- Embedding-backed ranking is optional and provider-gated.

Implementation:

- Read sidecar chunks across a collection.
- Add lexical-first retrieval.
- Use embeddings only when a valid vector sidecar exists.

### Lane 4: Locate and Annotate

Goal: convert retrieved evidence into Zotero-native annotations.

Tests first:

- Quote target can become text highlight when the PDF text layer supports it.
- Bbox target can become area annotation for OCR/layout blocks.
- Result schema uses `annotation_key`, not numeric IDs.
- Color/comment/provenance are preserved.

Implementation:

- Add locate target helper.
- Add batch apply helper over `annotations.create`.
- Keep dry-run support for automated agents.

## Explicit Non-Goals

- Built-in `ask-pdf` or agent reasoning inside Zotron.
- Treating provider Markdown as the source of truth.
- Storing OCR/RAG machine artifacts as visible Zotero notes or ordinary child
  attachments by default.
- Exporting annotated PDFs as a core Zotron command while Zotero already
  supports PDF export from the UI.

## Verification Contract

- Rust format/check/test/clippy for changed crates.
- Node/TypeScript tests for XPI changes.
- Live Zotero smoke only in the `test` collection.
- Record actual live commands; do not label a partial command smoke as full
  OCR/RAG/annotation validation.
