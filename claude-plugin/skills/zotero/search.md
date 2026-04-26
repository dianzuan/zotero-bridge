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
zotero-bridge rpc search.quick '{"query":"数字经济 就业","limit":10}'
```

Returns: item ID, title, authors, date, journal, tags. Use the ID for follow-up operations.

## Fulltext PDF search

When the user asks "which of my papers talks about X" — this searches inside PDF content, not just metadata.

```bash
zotero-bridge rpc search.fulltext '{"query":"regression discontinuity","limit":10}'
```

## Advanced multi-field search

Combine conditions with field/operator/value:

```bash
zotero-bridge rpc search.advanced '{"conditions":[{"field":"creator","op":"contains","value":"张三"},{"field":"date","op":"isAfter","value":"2020-01-01"}]}'
```

Common fields: `title`, `creator`, `date`, `publicationTitle` (journal), `DOI`, `tag`.
Operators: `is`, `isNot`, `contains`, `doesNotContain`, `isAfter`, `isBefore`.

## Read paper content

After finding a paper, get its full text or details:

```bash
# Full metadata
zotero-bridge rpc items.get '{"id":ITEM_ID}'

# PDF full text (for AI reading)
zotero-bridge rpc attachments.getFulltext '{"id":ITEM_ID}'

# PDF annotations/highlights
zotero-bridge rpc notes.getAnnotations '{"parentId":ITEM_ID}'

# Notes attached to paper
zotero-bridge rpc notes.get '{"parentId":ITEM_ID}'
```

## Browse collections

```bash
# See all collections as tree
zotero-bridge rpc collections.tree

# List items in a collection
zotero-bridge rpc collections.getItems '{"id":COLLECTION_ID,"limit":20}'
```

## Present results to user

After searching, summarize results as a numbered list:
1. **标题** — 作者 (年份) 期刊名
2. ...

Then ask: "要看哪篇的详细内容？" or proceed based on context.
