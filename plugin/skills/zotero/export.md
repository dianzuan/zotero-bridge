# Export

Generate formatted citations from papers in the user's Zotero library. CLI defaults to APA style; for Chinese academic writing, specify GB/T 7714 via `--style`.

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
| BibTeX (default) | LaTeX users, .bib file | `zotron export` |
| **GB/T 7714** | Chinese academic papers, 中文参考文献 | `zotron export --format bibliography --style http://www.zotero.org/styles/china-national-standard-gb-t-7714-2015-numeric` |
| APA | Default bibliography style | `zotron export --format bibliography` |
| RIS | EndNote/other reference managers | `zotron export --format ris` |
| CSL-JSON | Programmatic use | `zotron export --format csl-json` |

## GB/T 7714 (中文学术)

```bash
zotron export --format bibliography --style http://www.zotero.org/styles/china-national-standard-gb-t-7714-2015-numeric YR5BUGHG BF4I9QX4 X6LYTXEJ
```

Returns both `html` and `text` versions. Use `text` for plain output.

For the APA style (default when no `--style` is specified):
```bash
zotron export --format bibliography YR5BUGHG
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
