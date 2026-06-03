# RAG / Retrieval Hits

Find relevant spans across papers in a Zotero collection and return provenance-rich hits for literature review or `academic-zh`. Use `zotron rag ...`; do not call standalone `zotron-rag`.

Uses hybrid BM25 + vector + RRF retrieval by default. Falls back to keyword matching when no vector index exists.

Embedding provider is configured in Zotero → Settings → Zotron. Default: Ollama (nomic-embed-text, local, free). 10 providers supported: Ollama, OpenAI, Volcengine, DashScope, Zhipu, Jina, SiliconFlow, Voyage, Cohere, Custom.

## Workflow

```bash
# 1. Check whether OCR/chunk sidecars exist.
zotron ocr status --collection "财务报表造假识别"

# 2. Check RAG artifact state.
zotron rag status --collection "财务报表造假识别"

# 3. Emit academic-zh retrieval hits (hybrid search is the default).
zotron rag search \
  --collection "财务报表造假识别" \
  --limit 50 \
  --top-spans-per-item 3 \
  --output jsonl \
  "财务报表 舞弊 识别 风险"
```

## Search

```bash
# Search across a collection
zotron rag search --collection "数字经济" --output jsonl "数字经济对劳动力市场的影响机制"

# Search specific items by key (from RAG hits or search results)
zotron rag search --key YR5BUGHG --key BF4I9QX4 --top-spans-per-item 10 --output jsonl "关键词"
```

Returns one JSON hit per line with score, paper title, authors, section heading, `chunkKey`, `blockKeys`, page/bbox provenance, and Zotero URI.

## Retrieval hits

```bash
zotron rag search \
  --collection "中国工业经济" \
  --limit 50 \
  --top-spans-per-item 3 \
  --output jsonl \
  "贸易中心性 金融风险 识别策略"
```

Hybrid search runs BM25 + vector + RRF fusion locally, then calls the XPI for metadata enrichment. Callers do not need to know where hidden per-PDF `.zotron/chunks/chunks.v1.jsonl` sidecars live. The output is one `academic-zh` retrieval hit per line with span provenance:

```json
{
  "itemKey": "X6LYTXEJ",
  "attachmentKey": "NBUVZGWJ",
  "title": "上市公司财务报表舞弊识别的实证研究——基于Logistic回归模型",
  "authors": ["濮双羽", "赵洪进"],
  "year": 2021,
  "venue": "农场经济管理",
  "zotero_uri": "zotero://select/library/items/X6LYTXEJ",
  "section_heading": "一、引言",
  "sectionPath": ["一、引言"],
  "chunkKey": "NBUVZGWJ:c2",
  "blockKeys": ["NBUVZGWJ:p0:b8"],
  "pageIdx": 0,
  "bbox": [72.0, 180.0, 510.0, 220.0],
  "evidenceRefs": [{"blockKey": "NBUVZGWJ:p0:b8", "pageIdx": 0, "bbox": [72.0, 180.0, 510.0, 220.0]}],
  "query": "财务报表 舞弊 识别 风险",
  "score": 4,
  "text": "可引用的原文 span"
}
```

Do not collapse these hits into final paper cards unless the caller explicitly asks. `academic-zh` consumes hits JSONL and builds `paper_cards.jsonl` plus `citation_map.json` itself.

## Retrieval pipeline: rerank + dynamic cutoff + diversity

After the BM25 + vector + RRF fusion, `rag search` runs a quality pipeline before returning hits:

1. **Rerank** (optional) — when a reranker is configured in Zotero → Settings → Zotron, the top candidates are re-scored by a cross-encoder. Reranker scores land in a clean 0..1 relevance range. Configured via `rerank.provider`, `rerank.apiKey`, `rerank.candidateCount`.
2. **Dynamic cutoff** — when a reranker is active, a score floor (`rerank.scoreFloor`) drops weak hits and a gap cutoff (`rerank.gapThreshold`) trims the long tail at the largest score drop, so you get only as many hits as are actually relevant instead of a fixed `k`.
3. **Diversity (MMR)** — near-duplicate spans are removed so the same passage repeated across papers does not crowd out distinct evidence. Tuned with `rag.mmrLambda` (higher = favor relevance, lower = favor diversity). Relevance scores are normalized to a 0..1 scale before this step so diversity works in every mode, including when no reranker is configured.
4. **Token budget + bounds** — the result set is capped by `rag.tokenBudget` and bounded by `rag.minK` / `rag.maxK`.

### New output fields

Every `rag search` result now reports how it was actually produced:

- **`mode`** (top-level) — the retrieval path actually used: `hybrid`, `dense`, or `lexical`. This can differ from the configured mode: if embedding vectors or the query embedding are unavailable, the search transparently falls back to lexical (BM25) and reports `lexical` here instead of silently returning nothing.
- **`scoreKind`** (per hit) — the origin/scale of that hit's `score`: `rerank` (0..1 reranker score), `rrf` (fused rank score), `cosine` (vector similarity), or `bm25` (keyword score). Use this to interpret the `score` value, since the scale differs by path.

Check `zotron rag status` before searching: `embeddingsAvailable` and `totalVectors` tell you whether semantic (dense) retrieval is actually possible for the collection, or whether the search will run lexical-only.

> **After upgrading:** old v1 chunk sidecars are not schema-versioned and will be treated as stale. Run `zotron ocr reindex --stale-only` (optionally scoped with `--collection` or `--key`) once after upgrading so old sidecars are rebuilt to the current schema. Skipping this mixes stale chunks into retrieval. `--stale-only` only rebuilds out-of-date sidecars, so it is safe and cheap to run.

For a real fixture matching this contract, see:

```bash
fixtures/academic_zh_hits.jsonl
```

## Index management

```bash
zotron rag status --collection "数字经济"
zotron rag providers
zotron rag embed --provider custom --input /tmp/request.json --endpoint "$EMBEDDING_ENDPOINT" --model "$EMBEDDING_MODEL"
```

## Why RAG saves tokens

Without RAG: read 5 full papers → ~50K tokens per query
With RAG: get 10 relevant paragraphs → ~5K tokens per query

## Configuration

Embedding provider and retrieval mode (hybrid/dense/lexical) are configured in Zotero → Settings → Zotron panel. API tokens are user-provided and should not be hardcoded in commands or skill docs.

```bash
zotron ocr process --parent ITEMKEY
zotron rag search --key ITEMKEY --output jsonl "研究问题"
```
