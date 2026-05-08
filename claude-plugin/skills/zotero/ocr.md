# OCR

Use Zotron's OCR provider utilities and sidecar status checks for PDFs that Zotero cannot parse cleanly. The supported Rust CLI surface is `zotron ocr ...`; do not call standalone `zotron-ocr`.

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

# Run one provider request from an explicit JSON payload
zotron ocr provider-json --provider mineru --input /tmp/request.json --output /tmp/mineru-result.json
```

## When to use

OCR is not mandatory for every PDF. Zotero's built-in text extraction is the first choice for normal text-layer PDFs. Use cloud OCR/layout parsing when Zotero fulltext is empty, garbled, or lacks the block/table provenance needed for evidence-aware RAG.

## Configuration

Needs provider API keys. Keep them in ignored environment files or shell env, not in commands or docs:

```bash
export ZOTRON_MINERU_API_KEY=...
export ZOTRON_PADDLE_OCR_TOKEN=...
export ZOTRON_GLM_API_KEY=...
```

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
