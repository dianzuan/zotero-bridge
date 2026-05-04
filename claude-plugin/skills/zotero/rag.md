# RAG / Retrieval Hits

Find relevant spans across papers in a Zotero collection and return provenance-rich hits for literature review or `academic-zh`. Requires the target papers to have OCR chunk artifacts. Semantic embedding artifacts can be generated and attached to Zotero, but the current Zotero XPI `rag.searchHits` path uses lexical fallback plus artifact-aware provenance.

## Workflow

```bash
# 1. OCR the collection or a few target items if not done.
zotron-ocr run --collection "财务报表造假识别"
zotron-ocr run --item 5843

# 2. Attach embedding artifacts for Zotero-native chunks.
zotron-rag index-artifacts --zotero --collection "财务报表造假识别"
zotron-rag index-artifacts --zotero --item 5843

# 3. Emit academic-zh retrieval hits via Zotero XPI JSON-RPC.
zotron-rag hits --zotero \
  --collection "财务报表造假识别" \
  --limit 50 \
  --top-spans-per-item 3 \
  --output jsonl \
  "财务报表 舞弊 识别 风险"
```

## Search

```bash
# Search across a collection
zotron-rag hits --zotero --collection "数字经济" --output jsonl "数字经济对劳动力市场的影响机制"

# Search specific items by key (from RAG hits or search results)
zotron rpc rag.searchHits '{"query":"关键词","keys":["YR5BUGHG","BF4I9QX4"],"top_spans_per_item":10}'
```

Returns one JSON hit per line with score, paper title, authors, section heading, chunk id, block ids, and Zotero URI.

## Retrieval hits

```bash
zotron-rag hits --zotero \
  --collection "中国工业经济" \
  --limit 50 \
  --top-spans-per-item 3 \
  --output jsonl \
  "贸易中心性 金融风险 识别策略"
```

`--zotero` calls the XPI `rag.searchHits` method, so callers do not need to know where item-attached `.zotron-chunks.jsonl` artifacts live. The output is one `academic-zh` retrieval hit per line with span provenance:

```json
{
  "item_key": "X6LYTXEJ",
  "title": "上市公司财务报表舞弊识别的实证研究——基于Logistic回归模型",
  "authors": ["濮双羽", "赵洪进"],
  "year": 2021,
  "venue": "农场经济管理",
  "zotero_uri": "zotero://select/library/items/X6LYTXEJ",
  "section_heading": "一、引言",
  "chunk_id": "NBUVZGWJ:c2",
  "block_ids": ["NBUVZGWJ:p0:b8"],
  "query": "财务报表 舞弊 识别 风险",
  "score": 4,
  "text": "可引用的原文 span"
}
```

Do not collapse these hits into final paper cards unless the caller explicitly asks. `academic-zh` consumes hits JSONL and builds `paper_cards.jsonl` plus `citation_map.json` itself.

For a real fixture matching this contract, see:

```bash
fixtures/academic_zh_hits.jsonl
```

## Cite — retrieve with full citation provenance

```bash
zotron-rag cite "how do transformers attend to long-range context?" --collection "ML Papers" --output json
```

Returns JSON array of `Citation` objects with item key, attachment key, section heading, chunk text, similarity score, and `zotero://` URI.

## Index management

```bash
zotron-rag status --collection "数字经济"
zotron-rag index-artifacts --zotero --collection "数字经济"
```

## Why RAG saves tokens

Without RAG: read 5 full papers → ~50K tokens per query
With RAG: get 10 relevant paragraphs → ~5K tokens per query

## Configuration

Default provider setup is managed from Zotron settings. Current recommended defaults are GLM for OCR and Doubao for embeddings; API tokens are user-provided and should not be hardcoded in commands or skill docs.

```bash
zotron-ocr run --item 5843
zotron-rag index-artifacts --zotero --item 5843
```
