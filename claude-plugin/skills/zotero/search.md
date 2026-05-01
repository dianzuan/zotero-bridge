# Search

Search and browse the user's Zotero library — find papers by keywords, read PDF fulltext, get annotations, browse collections.

## Choosing the right search

| User wants | Method | When to use |
|-----------|--------|-------------|
| Find by title/author/year | `search.quick` | Most common, start here |
| Search inside PDF text | `search.fulltext` | User asks "which paper mentions X" |
| Multiple filters | `search.advanced` | Author + date range + journal |
| Papers with a tag | `search.byTag` | User mentions a specific tag |
| Browse by folder | `collections.tree` → `collections.getItems` | User says "what's in my X collection" |

## Quick search (default)

```bash
zotron rpc search.quick '{"query":"数字经济 就业","limit":10}'
```

Returns: item ID, title, authors, date, journal, tags. Use the ID for follow-up operations.

## Fulltext PDF search

When the user asks "which of my papers talks about X" — this searches inside PDF content, not just metadata.

```bash
zotron rpc search.fulltext '{"query":"regression discontinuity","limit":10}'
```

## Advanced multi-field search

Combine conditions with field/operator/value:

```bash
zotron rpc search.advanced '{"conditions":[{"field":"creator","op":"contains","value":"张三"},{"field":"date","op":"isAfter","value":"2020-01-01"}]}'
```

Common fields: `title`, `creator`, `date`, `publicationTitle` (journal), `DOI`, `tag`.
Operators: `is`, `isNot`, `contains`, `doesNotContain`, `isAfter`, `isBefore`.

## Read paper content

After finding a paper, get its full text or details. All methods accept either a numeric `id` or an 8-char item `key` string (e.g. `"YR5BUGHG"` from RAG hits):

```bash
# Full metadata (by numeric ID or key)
zotron rpc items.get '{"id":12345}'
zotron rpc items.get '{"id":"YR5BUGHG"}'

# PDF full text (for AI reading)
zotron rpc attachments.getFulltext '{"id":12345}'

# PDF annotations/highlights
zotron rpc notes.getAnnotations '{"parentId":"YR5BUGHG"}'

# Notes attached to paper (includes OCR preview notes)
zotron rpc notes.get '{"parentId":"YR5BUGHG"}'

# RAG chunks for specific items (returns OCR spans with provenance)
zotron rpc rag.searchHits '{"query":"关键词","itemKeys":["YR5BUGHG","BF4I9QX4"]}'
```

## Browse collections

```bash
# See all collections as tree
zotron rpc collections.tree

# List items in a collection
zotron rpc collections.getItems '{"id":COLLECTION_ID,"limit":20}'
```

## Present results to user

After searching, summarize results as a numbered list:
1. **标题** — 作者 (年份) 期刊名
2. ...

Then ask: "要看哪篇的详细内容？" or proceed based on context.
