# zotron API Stability

This document is the contract for external consumers of `zotron` on `main`.

## Product Surface

The supported product stack is:

- The `zotron` CLI/crates for agent-facing commands and provider integration.
- Zotero-side XPI JSON-RPC bridge for Zotero operations.
- Claude/Codex skill wrappers that call the `zotron` CLI.

## CLI

The user-facing command is a single binary:

```text
zotron
```

OCR and RAG are command groups under that binary:

```text
zotron ocr ...
zotron rag ...
```

OCR and RAG are subcommand groups of `zotron`, not standalone binaries.

### Stable Command Groups

The product currently exposes these command groups:

- `zotron ping`
- `zotron rpc`
- `zotron push`
- `zotron system ...`
- `zotron search ...`
- `zotron items ...`
- `zotron collections ...`
- `zotron notes ...`
- `zotron settings ...`
- `zotron tags ...`
- `zotron export ...`
- `zotron annotations ...`
- `zotron ocr ...`
- `zotron rag ...`
- `zotron sources ...`

Attachment operations (`fulltext`, `path`, `attachments`, `find-pdfs`) are
subcommands of `zotron items`.

New flags and subcommands may be added. Existing command names and flag
semantics should not be removed or changed without a major-version decision.

### Filtering

Rust `zotron` prints structured JSON by default. Filtering is done with an
external pipe:

```text
zotron collections get-items "test" | jq '.items[].title'
```

An embedded `--jq` convenience is not part of the contract.

### Collection-Scoped Search

`zotron search QUERY --collection NAME` is supported as a convenience path for
collection-limited metadata search. `zotron collections get-items NAME` remains
the direct command for listing collection members.

## JSON Contract

Success output is JSON. Shape depends on the command.

Errors use this envelope:

```json
{
  "error": {
    "code": "UPPERCASE_TOKEN",
    "message": "human-readable string"
  }
}
```

Known stable error-code families include:

- `INVALID_JSON`
- `INVALID_ARGS`
- `INVALID_REQUEST`
- `ZOTERO_UNAVAILABLE`
- `RPC_ERROR`
- `COLLECTION_NOT_FOUND`
- `COLLECTION_AMBIGUOUS`
- `INVALID_PDF`
- `INVALID_PROVIDER`
- `PROVIDER_ERROR`
- `ZOTERO_ERROR`

Dry-run write commands return JSON describing the skipped operation. The exact
payload is command-specific, but it must clearly identify the RPC method or
local action that would have run.

## Identifier Policy

Public schemas are key-first:

- `item_key`
- `attachment_key`
- `collection_key`
- `annotation_key`
- `block_key`
- `chunk_key`
- `block_keys`
- `keys`

New public OCR/RAG/PDF-evidence fields must not introduce `item_id`,
`attachment_id`, `collection_id`, `collectionId`, `itemId`, or similar Zotero
numeric-id fields.

## OCR and RAG Evidence

The canonical evidence path is hidden per-PDF sidecar storage under the Zotero
attachment storage directory:

```text
storage/<attachment-key>/.zotron/
  ocr/latest.raw.json
  ocr/latest.blocks.jsonl
  chunks/chunks.v1.jsonl
  embeddings/vectors.jsonl
```

`zotron ocr process` writes provider raw output, normalized blocks,
structure-first chunks, Markdown preview, and assets when the provider returns
them. Markdown is a convenience artifact; blocks/chunks are the evidence source
of truth.

`zotron rag search` returns evidence spans for agents. Zotron does not provide
an `ask-pdf` LLM service; Codex/Claude perform reasoning from returned evidence.

## Rust Crates

The repository may use multiple internal crates for maintainability, but the
public user-facing package and command name remain `zotron`.

Current intended crate boundaries:

- `zotron-cli`: command parsing and CLI orchestration; binary name `zotron`.
- `zotron-rpc`: JSON-RPC client and transport helpers.
- `zotron-types`: shared request/response/provider types.

Internal crates do not need to be published to crates.io until there is a
clear external reuse need. Publishing a single `zotron` crate that installs the
`zotron` binary remains compatible with internal crate splitting.

## Versioning

Zotron follows semver for the Rust/XPI product surface:

- Major: breaking command, flag, JSON schema, or sidecar layout changes.
- Minor: new commands, flags, providers, or fields.
- Patch: bug fixes and documentation updates that preserve behavior.

The XPI bridge and `zotron` CLI should be tested together.
