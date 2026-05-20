# Search

Search and browse the user's Zotero library — find papers by keywords, read PDF fulltext, get annotations, browse collections.

## Choosing the right search

| User wants | Command | When to use |
|-----------|---------|-------------|
| Find by title/author/year | `zotron search "query"` | Most common, start here |
| Find by title/author/year inside a collection | `zotron search "query" --collection X` | User gives both keyword and collection |
| Search inside PDF text | `zotron search "query" --fulltext` | User asks "which paper mentions X" |
| Multiple filters | `zotron search --author X --after Y` | Author + date range + journal |
| Papers with a tag | `zotron search --tag "标签"` | User mentions a specific tag |
| Browse a collection | `zotron collections get-items` | User asks "what's in my X collection" |

## Quick search (default)

```bash
zotron search "数字经济 就业" --limit 10
zotron search "数字经济 就业" --collection "宏观因子" --limit 10
```

Returns an envelope: `{"items":[...],"total":N}`. Use item keys for follow-up operations.

`--collection` limits search to items in that collection. This is for metadata/title/author/year-style filtering, not full PDF text search.

## Fulltext PDF search

When the user asks "which of my papers talks about X" — this searches inside PDF content, not just metadata.

```bash
zotron search "regression discontinuity" --fulltext --limit 10
```

## Advanced multi-field search

Combine flags on `zotron search` to filter by multiple criteria:

```bash
zotron search --author "张三" --after 2020
zotron search "经济" --author "张三" --journal "经济研究" --after 2020
zotron search --tag "核心期刊" --after 2020 --limit 20
```

Available filter flags: `--author`, `--after`, `--before`, `--journal`, `--tag`.
These are combined with AND logic internally.

## Search by tag

```bash
zotron search --tag "核心期刊" --limit 20
```

## Search by identifier (DOI / ISBN / ISSN)

```bash
zotron search --doi 10.1038/nature12373
zotron search --isbn 9780262035613
zotron search --issn 0028-0836
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

After finding a paper, use its 8-char key directly:

```bash
# Full metadata
zotron items get YR5BUGHG

# Get fulltext from an item (auto-finds the PDF attachment)
zotron items fulltext YR5BUGHG

# Get the local file path of an item's PDF attachment
zotron items path ATT_KEY

# List attachments belonging to an item
zotron items attachments YR5BUGHG

# Notes (includes OCR markdown when OCR'd — filter for "ocr" tag)
zotron notes list --parent YR5BUGHG

# Read a specific note
zotron notes get <note-key>

# PDF annotations/highlights
zotron annotations list YR5BUGHG
```

Zotero automatically indexes PDFs. `items fulltext` finds the first PDF attachment and returns its cached text — no OCR needed for most papers. Use `zotron ocr ...` only for scanned PDFs or when fulltext is empty/garbled.

For searching relevant passages across a collection (not full text), see [rag.md](rag.md).

## Annotations

```bash
# List annotations on a PDF attachment
zotron annotations list ATT_KEY

# Create a highlight by quoting text (auto-locates in the PDF, works without opening it)
zotron annotations create ATT_KEY --quote "要高亮的文字"

# With color for a specific dimension
zotron annotations create ATT_KEY --quote "研究基于..." --color "#56B4E9" --comment "背景"

# Dry-run to verify the quote is found before creating the annotation
zotron annotations create ATT_KEY --quote "要高亮的文字" --dry-run

# With explicit type/color/position (for non-text annotations)
zotron annotations create ATT_KEY --type image --position '{"pageIndex":0,"rects":[[10,20,30,40]]}'
```

`--quote` handles Unicode punctuation variants automatically (curly quotes ↔ straight quotes, fullwidth ↔ halfwidth CJK punctuation). Quotes that span a page boundary are detected and create one annotation per page segment.

**Annotation color convention** (Okabe-Ito colorblind-safe palette):

| Dimension | Color | Hex | Use for |
|-----------|-------|-----|---------|
| Background | Sky blue | `#56B4E9` | Research context, motivation, literature review |
| Method | Teal | `#009E73` | Algorithm, experimental design, theoretical framework |
| Result | Orange | `#E69F00` | Key findings, data, empirical evidence |
| Conclusion | Yellow | `#F0E442` | Final contributions, implications |
| Question | Vermillion | `#D55E00` | Points to challenge, limitations, gaps |
| My idea | Pink | `#CC79A7` | Reader's own thoughts, connections |

When annotating a paper, use `--color` with the appropriate hex code. Default (no `--color`) is yellow `#ffd400`.

## Browse collections

```bash
# See all collections as tree
zotron collections tree

# Flat list of collections
zotron collections list

# Get a single collection's metadata
zotron collections get "Collection Name"

# List items in a collection. Alias: `zotron collections items ...`
zotron collections get-items "Collection Name" --limit 20
zotron collections get-items "Collection Name" | jq '.items[].title'

# Collection stats (item/attachment/note/subcollection counts)
zotron collections stats "Collection Name"
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
