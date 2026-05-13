# Rust CLI Surface

Updated: 2026-05-09

This file is the current Rust CLI surface summary for the `rust-migration`
branch. The old Python Typer CLI is reference material only.

## Public Binary

```text
zotron
```

Do not introduce separate public binaries such as `zotron-ocr` or
`zotron-rag`. OCR and RAG remain subcommands of the single binary.

## Command Groups

```text
zotron ping
zotron rpc
zotron push
zotron system ...
zotron search ...
zotron items ...
zotron collections ...
zotron notes ...
zotron attachments ...
zotron settings ...
zotron tags ...
zotron export ...
zotron annotations ...
zotron ocr ...
zotron rag ...
zotron find-pdfs
```

## Agent-Facing Conventions

- Prefer keys over numeric IDs in public JSON and CLI examples.
- Use `zotron collections get-items NAME` to list collection members.
- Use `zotron search quick --collection NAME QUERY` for collection-scoped
  quick search.
- Pipe to external `jq` for filtering instead of relying on embedded `--jq`.
- Keep OCR and RAG machine artifacts hidden under the PDF attachment sidecar
  directory by default.

## OCR/RAG Surface

```text
zotron ocr providers
zotron ocr provider-json --provider mineru --input request.json
zotron ocr status --parent ITEMKEY --attachment ATTACHKEY
zotron ocr parse-pdf --provider mineru --parent ITEMKEY --attachment ATTACHKEY
zotron rag embedding-providers
zotron rag embedding-json --provider alibaba --input request.json
zotron rag status --collection test
zotron rag hits --zotero --collection test "query"
```

`parse-pdf` is the current structure-first evidence path. It writes provider
raw output, blocks, chunks, Markdown preview, and assets into the hidden
`.zotron` sidecar below the Zotero PDF attachment storage directory.

## Remaining Work

- Production embedding index write/read over `chunks.v1.jsonl`.
- Higher-level evidence locator and annotation application commands.
- GLM/Paddle parser normalization into the same sidecar schema.
- Cross-platform sidecar sync/path validation.
- Final crate publishing decision.
