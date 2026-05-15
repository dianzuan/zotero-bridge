# Export

Generate formatted citations from papers in the user's Zotero library. For Chinese academic writing default to GB/T 7714.

## Workflow

Usually you need to search first, then export:

```bash
# 1. Find the papers
zotron search "数字经济" --limit 10

# 2. Note the keys from results, then export
zotron export --format bibliography YR5BUGHG BF4I9QX4 X6LYTXEJ
```

## Formats

| Format | When to use | Command |
|--------|------------|---------|
| **GB/T 7714** (default) | Chinese academic papers, 中文参考文献 | `zotron export --format bibliography` |
| BibTeX (default format) | LaTeX users, .bib file | `zotron export` |
| RIS | EndNote/other reference managers | `zotron export --format ris` |
| CSL-JSON | Programmatic use | `zotron export --format csl-json` |

## GB/T 7714 (中文学术默认)

```bash
zotron export --format bibliography YR5BUGHG BF4I9QX4 X6LYTXEJ
```

Returns both `html` and `text` versions. Use `text` for plain output.

For the author-date variant:
```bash
zotron export --format bibliography YR5BUGHG --style apa
```

## BibTeX

```bash
zotron export YR5BUGHG BF4I9QX4 X6LYTXEJ
```

## Citation key

Look up a paper's Better-BibTeX citation key for LaTeX `\cite{}`:

```bash
zotron items citation-key YR5BUGHG
```

## Collection-scoped export

```bash
zotron export --collection "数字经济"
zotron export --format bibliography --collection "数字经济"
```

## Present to user

Output references as a numbered list:
```
[1] 张三, 李四. 数字经济对就业的影响[J]. 经济研究, 2024, 59(3): 15-30.
[2] ...
```
