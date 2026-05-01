# Export

Generate formatted citations from papers in the user's Zotero library. For Chinese academic writing default to GB/T 7714.

## Workflow

Usually you need to search first, then export:

```bash
# 1. Find the papers
zotron search quick "数字经济" --limit 10

# 2. Note the IDs from results, then export
zotron export bibliography 10 13 16
```

## Formats

| Format | When to use | Command |
|--------|------------|---------|
| **GB/T 7714** (default) | Chinese academic papers, 中文参考文献 | `zotron export bibliography` |
| BibTeX | LaTeX users, .bib file | `zotron export bibtex` |
| RIS | EndNote/other reference managers | `zotron export ris` |
| CSL-JSON | Programmatic use | `zotron export csl-json` |

## GB/T 7714 (中文学术默认)

```bash
zotron export bibliography 10 13 16
```

Returns both `html` and `text` versions. Use `text` for plain output.

For the author-date variant:
```bash
zotron export bibliography 10 --style apa
```

## BibTeX

```bash
zotron export bibtex 10 13 16
```

## Citation key

Look up a paper's Better-BibTeX citation key for LaTeX `\cite{}`:

```bash
zotron items citation-key 10
```

## Present to user

Output references as a numbered list:
```
[1] 张三, 李四. 数字经济对就业的影响[J]. 经济研究, 2024, 59(3): 15-30.
[2] ...
```
