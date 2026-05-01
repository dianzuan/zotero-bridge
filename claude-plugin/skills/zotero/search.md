# Search

Search and browse the user's Zotero library — find papers by keywords, read PDF fulltext, get annotations, browse collections.

## Choosing the right search

| User wants | Command | When to use |
|-----------|---------|-------------|
| Find by title/author/year | `zotron search quick` | Most common, start here |
| Search inside PDF text | `zotron search fulltext` | User asks "which paper mentions X" |
| Multiple filters | `zotron search advanced` | Author + date range + journal |
| Papers with a tag | `zotron search by-tag` | User mentions a specific tag |
| Browse by folder | `zotron collections tree` | User says "what's in my X collection" |

## Quick search (default)

```bash
zotron search quick "数字经济 就业" --limit 10
```

Returns: item ID, title, authors, date, journal, tags. Use the ID for follow-up operations.

## Fulltext PDF search

When the user asks "which of my papers talks about X" — this searches inside PDF content, not just metadata.

```bash
zotron search fulltext "regression discontinuity" --limit 10
```

## Advanced multi-field search

Combine conditions with `--condition "field operator value"`:

```bash
zotron search advanced --condition "creator contains 张三" --condition "date isAfter 2020"
```

Common fields: `title`, `creator`, `date`, `publicationTitle` (journal), `DOI`, `tag`.
Operators: `is`, `isNot`, `contains`, `doesNotContain`, `isAfter`, `isBefore`.

Use `--operator or` to match any condition (default is `and`).

## Search by tag

```bash
zotron search by-tag "核心期刊" --limit 20
```

## Saved searches

```bash
# List saved searches
zotron search saved-searches

# Create a saved search
zotron search create-saved "张三近5年" --condition "creator contains 张三" --condition "date isAfter 2020"

# Delete
zotron search delete-saved <search-id>
```

## Read paper content

After finding a paper, use its ID or 8-char key directly:

```bash
# Full metadata
zotron items get 12345
zotron items get YR5BUGHG

# PDF full text — auto-finds the PDF attachment
zotron items fulltext 12345

# List attachments
zotron attachments list --parent 12345

# Get attachment file path
zotron attachments path <attachment-id>

# Notes (includes OCR markdown when OCR'd — filter for "ocr" tag)
zotron notes list --parent YR5BUGHG

# Read a specific note
zotron notes get <note-id>

# PDF annotations/highlights
zotron annotations list --parent YR5BUGHG
```

For searching relevant passages across a collection (not full text), see [rag.md](rag.md).

## Browse collections

```bash
# See all collections as tree
zotron collections tree

# List items in a collection
zotron rpc collections.getItems '{"id":COLLECTION_ID,"limit":20}'
```

## Browse recent items

```bash
# Recently added
zotron items recent --limit 10

# Recently modified
zotron items recent --limit 10 --type modified
```

## Present results to user

After searching, summarize results as a numbered list:
1. **标题** — 作者 (年份) 期刊名
2. ...

Then ask: "要看哪篇的详细内容？" or proceed based on context.
