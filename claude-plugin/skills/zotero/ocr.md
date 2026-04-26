# OCR

Convert PDFs in a Zotero collection to high-quality Markdown via cloud OCR, stored as Zotero Notes. Always run OCR before building a RAG index.

## Core usage

```bash
# OCR all PDFs in a collection (most common)
zotero-ocr --collection "数字经济"

# OCR a single paper
zotero-ocr --item 12345

# Force re-OCR (even if already done)
zotero-ocr --collection "数字经济" --force

# Check status
zotero-ocr status --collection "数字经济"
```

## When to use

OCR is the first step before building a RAG index. Zotero's built-in text extraction produces poor results for Chinese PDFs. Cloud OCR (GLM-4V) produces structured Markdown with headings, tables, and formulas preserved.

**Always OCR before RAG:** `zotero-ocr` → `zotero-rag index`

## Configuration

Needs OCR API key. Set via environment:
```bash
export OCR_PROVIDER=glm
export OCR_API_KEY=your-key
```
Or config file: `~/.config/zotero-bridge/config.json`

## Output

OCR results are saved as Zotero Notes with the "ocr" tag.
