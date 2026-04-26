# Export

Generate formatted citations from papers in the user's Zotero library. For Chinese academic writing default to GB/T 7714.

## Workflow

Usually you need to search first, then export:

```bash
# 1. Find the papers
zotero-bridge rpc search.quick '{"query":"数字经济","limit":10}'

# 2. Note the IDs from results, then export
zotero-bridge export bibliography 10 13 16
```

## Formats

| Format | When to use | Command |
|--------|------------|---------|
| **GB/T 7714** (default) | Chinese academic papers, 中文参考文献 | `export.bibliography` |
| BibTeX | LaTeX users, .bib file | `export.bibtex` |
| RIS | EndNote/other reference managers | `export.ris` |
| CSL-JSON | Programmatic use | `export.cslJson` |

## GB/T 7714 (中文学术默认)

```bash
zotero-bridge export bibliography 10 13 16
```

Returns both `html` and `text` versions. Use `text` for plain output.

For a specific style variant:
```bash
zotero-bridge rpc export.bibliography '{"ids":[10],"style":"http://www.zotero.org/styles/gb-t-7714-2015-author-date"}'
```

## BibTeX

```bash
zotero-bridge export bibtex 10 13 16
```

## Citation key

Get a paper's citation key (for LaTeX `\cite{}`):

```bash
zotero-bridge rpc export.citationKey '{"id":10}'
```

## Present to user

Output references as a numbered list:
```
[1] 张三, 李四. 数字经济对就业的影响[J]. 经济研究, 2024, 59(3): 15-30.
[2] ...
```
