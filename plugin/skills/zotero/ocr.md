# OCR

Use Zotron's OCR provider utilities and sidecar status checks for PDFs that Zotero cannot parse cleanly. Use `zotron ocr ...`; do not call standalone `zotron-ocr`.

## When to use OCR

Before running OCR, check if Zotero's built-in text extraction is sufficient:

```bash
zotron items fulltext YR5BUGHG
```

If the content is non-empty and readable, **skip OCR**. OCR is needed when:
- The fulltext is empty (scanned PDF with no text layer)
- The fulltext is garbled (common with Chinese/CJK PDFs)
- You need structured blocks/chunks for RAG with page and bbox provenance

## Core usage

```bash
# Check status
zotron ocr status --collection "数字经济"

# Inspect configured providers
zotron ocr providers

# Parse a Zotero PDF and write hidden sidecar blocks/chunks (provider from Zotero settings)
zotron ocr process --parent ITEMKEY

# Attachment key auto-resolved from --parent; specify explicitly if multiple PDFs
zotron ocr process --parent ITEMKEY --attachment ATTACHKEY

# Batch: OCR every item in a collection (PDF auto-resolved per item; items without a PDF are skipped)
zotron ocr process --collection "数字经济"

# Replay an already-downloaded MinerU result without another provider call
zotron ocr process --parent ITEMKEY --result-dir /tmp/mineru-result
```

Use `process` for the real pipeline. `run` is only a low-level transport/debug command.

Pass `--parent` for a single item or `--collection` to process the whole collection. Batch output reports `processed` / `skipped` / `failed` counts plus a per-item `items` array; `--collection` cannot be combined with `--result-dir` / `--result-zip`.

## Reindex (re-chunk + re-embed without re-OCR)

`ocr reindex` rebuilds the chunk sidecars and embedding vectors from the already-extracted blocks — no OCR provider call, so it is free. Use it after upgrading or after changing chunk settings.

```bash
# Rebuild every out-of-date sidecar in a collection (recommended after upgrade)
zotron ocr reindex --collection "数字经济" --stale-only

# Rebuild specific items
zotron ocr reindex --key ITEMKEY --stale-only

# Force a full rebuild regardless of schema version
zotron ocr reindex --collection "数字经济"
```

Chunk sidecars are schema-versioned: a `schema_version` header is written as the first line of `chunks/chunks.v1.jsonl`. `--stale-only` reads that header and **skips sidecars already at the current schema**, so it only rebuilds what is out of date — safe and cheap to run repeatedly.

**Run after upgrading.** Sidecars produced before schema versioning have no header and are treated as stale. Running `zotron ocr reindex --stale-only` once after an upgrade rebuilds them to the current schema; otherwise stale chunks get mixed into retrieval. Reindex also (re)generates embedding vectors, so semantic retrieval becomes available for documents that were only chunked before.

## When to use

OCR is not mandatory for every PDF. Zotero's built-in text extraction is the first choice for normal text-layer PDFs. Use cloud OCR/layout parsing when Zotero fulltext is empty, garbled, or lacks the block/table provenance needed for evidence-aware RAG.

## Configuration

OCR provider, API key, model, and URL are configured in **Zotero → Settings → Zotron → OCR Settings**. The CLI reads these automatically via RPC — no flags or environment variables needed.

## Output

Machine outputs should be written as hidden sidecar files next to the PDF, not as normal Zotero notes by default:

```text
storage/<attachment-key>/.zotron/
├── ocr/latest.raw.json
├── ocr/latest.blocks.jsonl
├── ocr/latest.native.md
├── ocr/latest.assets.json
└── chunks/chunks.v1.jsonl
```
