---
name: scholar
description: Search and fetch English academic papers from OpenAlex, CrossRef, Semantic Scholar, Unpaywall, and arXiv. Use when user needs to find, download, or import English-language research papers into Zotero.
---

## Search papers

```bash
zotron-scholar search "query" [--limit N] [-s openalex|s2]
```

Default source is OpenAlex (250M records). Use `-s s2` for Semantic Scholar.

## Fetch by identifier

```bash
zotron-scholar fetch --doi 10.1038/xxx     # DOI -> metadata + PDF
zotron-scholar fetch --arxiv 2301.00001    # arXiv -> metadata + PDF
```

## Import to Zotero

```bash
zotron-scholar fetch --doi 10.1038/xxx | zotron push --collection "My Papers"
```
