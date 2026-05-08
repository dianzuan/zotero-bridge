# PDF Evidence and Annotation Milestones

Date: 2026-05-07

This milestone plan turns the revised PDF strategy into executable delivery
stages. Zotron is not an LLM question-answering service. It is the evidence and
annotation substrate for Codex / Claude Code: parse, index, retrieve, locate,
then apply Zotero-native annotations.

## Scope Decisions

- Canonical identifiers are keys: `item_key`, `attachment_key`, `block_key`,
  `annotation_key`, `collection_key`, and `keys`.
- New RAG/OCR/PDF evidence outputs must not introduce public `*_id`,
  `collectionId`, `attachmentId`, or `itemId` fields.
- MinerU-style structured JSON is the primary path for layout-aware parsing.
- OCR, block, chunk, and embedding outputs are machine artifacts. They must not
  be Zotero notes or ordinary child attachments by default, so Zotero's library
  UI/search continues to show real literature.
- Sync-worthy evidence artifacts may live as hidden sidecars under the existing
  PDF attachment storage directory. They do not create extra Zotero items:
  `storage/<attachment-key>/.zotron/manifest.json`,
  `storage/<attachment-key>/.zotron/zotron-blocks.jsonl`, and
  `storage/<attachment-key>/.zotron/zotron-chunks.jsonl`.
- Embedding vectors are rebuildable machine cache. They default to the
  cross-platform Zotron artifact cache, not synced sidecar storage, unless a
  user explicitly opts in.
- Zotero fulltext is a cheap fallback for plain text retrieval only; it is not a
  structural parser.
- Zotron does not provide `ask-pdf`. Agents do reasoning and synthesis.
- PDF export with embedded annotations is not a core milestone while Zotero UI
  already supports it.
- AI highlighting writes Zotero-native annotations. These annotations sync as
  Zotero data and remain visible in Zotero; embedding/chunk artifacts remain
  hidden machine data.

## Milestone 0: Contract Freeze

Goal: lock the product boundary before implementation.

Deliverables:

- PRD section for the five capabilities:
  `parse-pdf`, `index-blocks`, `retrieve-blocks`,
  `locate-highlight-target`, `apply-annotations`.
- Roadmap updated to remove built-in `ask-pdf` from the core path.
- Roadmap records the attachment sidecar storage policy and the no-search-
  pollution requirement.
- Key-first schema contract for parsed blocks, chunks, retrieval hits, locate
  targets, and annotation results.

Acceptance:

- Docs state that Zotron returns evidence, not final answers.
- Docs state that Zotero fulltext is fallback, not structure extraction.
- Docs state that synced evidence sidecars do not create Zotero child
  attachments.
- `rg` over RAG/OCR docs and planned schemas shows no new public `*_id` fields.

## Milestone 1: `parse-pdf`

Goal: turn a PDF attachment into parser-backed structured evidence blocks.

Deliverables:

- CLI/RPC contract for parsing a Zotero attachment by `attachment_key`.
- MinerU adapter path that imports structured JSON/Markdown output.
- Normalized block JSONL artifact:
  `storage/<attachment-key>/.zotron/zotron-blocks.jsonl`.
- Provider raw artifact:
  `storage/<attachment-key>/.zotron/zotron-ocr.raw.zip`.

Block contract:

```json
{
  "block_key": "ATTACHKEY:p16:b004",
  "item_key": "ITEMKEY",
  "attachment_key": "ATTACHKEY",
  "page_idx": 15,
  "type": "text",
  "bbox": [120, 300, 860, 420],
  "section_path": ["PART2", "构建基于宏观因子的风险平价配置框架"],
  "text": "Blyth(2016)提出了..."
}
```

Acceptance:

- One sample PDF produces blocks with stable `block_key`, `page_idx`, text, and
  bbox when the parser provides bbox.
- Raw parser output is preserved for audit/re-normalization.
- Markdown is generated only as a convenience artifact, not source of truth.
- Zotero UI still shows only the original PDF attachment, not a new Zotron
  artifact child item.

## Milestone 2: `index-blocks`

Goal: index structured blocks without losing provenance.

Deliverables:

- Structure-first chunk builder from blocks.
- Optional embedding generation over chunks/blocks.
- Chunk artifact:
  `storage/<attachment-key>/.zotron/zotron-chunks.jsonl`.
- Optional embedding artifact:
  cross-platform machine cache path, e.g.
  `<zotron-artifact-cache>/items/<item-key>/attachments/<attachment-key>/zotron-embed.npz`.

Acceptance:

- Chunks do not cross section boundaries unless explicitly marked.
- Every chunk preserves `item_key`, `attachment_key`, `block_keys`,
  page range, section path, and text.
- Retrieval can run with lexical search only; embeddings are not mandatory.
- Removing `zotron-embed.npz` forces re-embedding but does not destroy
  provenance because chunks and blocks remain readable.

## Milestone 3: `retrieve-blocks`

Goal: provide agent-facing evidence retrieval.

Deliverables:

- CLI/RPC method returning ranked blocks or chunks for a query.
- Hybrid retrieval hooks: exact/lexical first, embedding optional.
- JSON/JSONL output suitable for Codex / Claude Code context ingestion.

Acceptance:

- Returns original text spans, not summaries.
- Each result includes enough provenance for manual verification:
  `block_key`, `item_key`, `attachment_key`, `page_idx`, optional `bbox`,
  `section_path`, and score/source metadata.
- No LLM call is made inside Zotron.

## Milestone 4: `locate-highlight-target`

Goal: convert evidence into annotation targets.

Deliverables:

- Input modes:
  quote-based, `block_key`-based, and page/bbox-based.
- Target modes:
  text highlight target and area-box target.
- Confidence and fallback reporting.

Target contract:

```json
{
  "target_key": "ATTACHKEY:p16:b004:t1",
  "attachment_key": "ATTACHKEY",
  "block_key": "ATTACHKEY:p16:b004",
  "mode": "area",
  "page_idx": 15,
  "bbox": [120, 300, 860, 420],
  "quote": "Blyth(2016)提出了...",
  "confidence": 0.91
}
```

Acceptance:

- For MinerU blocks with bbox, produces an area target without reading a PDF
  text layer.
- For text-layer PDFs, quote matching may attempt a text target.
- If text target fails, locator returns a clear fallback target or a structured
  failure; it does not silently invent a position.

## Milestone 5: `apply-annotations`

Goal: write located targets as Zotero-native annotations.

Deliverables:

- Batch application of located targets.
- Support for color, comment, type, provenance, and dry-run.
- Annotation result contract using `annotation_key`.

Acceptance:

- Area target creates a visible Zotero annotation on the expected page region.
- Comment includes AI/agent rationale when provided.
- Output reports created and failed targets separately.
- A test fixture proves no public result fields use `*_id`.
- A smoke test proves annotation creation does not require embedding artifacts.

## Deferred

- Built-in `ask-pdf`.
- CLI wrapper for exporting annotated PDFs.
- Graph RAG / citation graph.
- Full text-layer quote-to-Zotero-position perfection. Text highlights can be
  improved incrementally after area-box annotation works reliably.

## Artifact Vocabulary

These files are related but not interchangeable:

- `zotron-ocr.raw.zip`: provider output. It preserves what MinerU/OCR/parser
  returned so normalization bugs can be fixed without re-running OCR.
- `zotron-blocks.jsonl`: normalized document structure. One row is usually a
  paragraph, heading, table, figure caption, or other layout block with page and
  bbox provenance.
- `zotron-chunks.jsonl`: retrieval text units built from blocks. One chunk may
  contain several blocks, and every chunk keeps `block_keys` so it can be traced
  back to PDF locations.
- `zotron-embed.npz`: numeric vectors for chunks. It accelerates semantic
  search but is rebuildable from chunks and the selected embedding model.
- `manifest.json`: version and checksum map tying raw, blocks, chunks, embedding
  model, parser, and source PDF together.
