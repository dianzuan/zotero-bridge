# Claude Handoff: Rust Migration, PDF Evidence, and Command Surface

Date: 2026-05-11

This is the handoff note for continuing Zotron on the `rust-migration` branch.

## Current Product Direction

Zotron is now a Rust CLI + Zotero-side JS/XPI project.

The target stack is:

```text
Zotero JS API
  -> Zotron XPI JSON-RPC server
  -> Rust `zotron` CLI
  -> Claude Code / Codex skills
```

Python remains legacy reference material only. New behavior should land in
Rust and/or the JS XPI.

The public command is one binary:

```text
zotron
```

Do not introduce public standalone binaries such as `zotron-ocr`,
`zotron-rag`, `zotero-ocr`, or `zotero-rag`. OCR and RAG are subcommands:

```text
zotron ocr ...
zotron rag ...
```

## What Has Been Done

Rust CLI parity and cleanup:

- Main Rust CLI command groups exist: `system`, `search`, `items`,
  `collections`, `notes`, `attachments`, `settings`, `tags`, `export`,
  `annotations`, `ocr`, `rag`, and `find-pdfs`.
- `zotron search quick --collection NAME QUERY` supports collection-scoped
  quick search.
- `zotron collections get-items NAME` is the canonical collection-member
  command. `collections items` is only an alias/compatibility convenience.
- Rust output is JSON-first. The intended filtering path is external `jq`.
  Do not reintroduce embedded Python-style `--jq`.
- Public product language is key-first. Avoid new public `*_id`,
  `collectionId`, `attachmentId`, or `itemId` fields.

OCR/RAG evidence work:

- `zotron ocr parse-pdf --provider mineru --parent ITEMKEY --attachment ATTACHKEY`
  works as the first end-to-end parser path.
- MinerU live smoke test produced hidden sidecars for item `2TGDLKDZ`,
  attachment `QI2YI74W`: 187 normalized blocks and 43 chunks.
- OCR/RAG artifacts are hidden under each PDF attachment storage directory:

```text
storage/<attachment-key>/.zotron/
  ocr/latest.raw.json
  ocr/latest.blocks.jsonl
  ocr/latest.native.md
  ocr/latest.assets.json
  chunks/chunks.v1.jsonl
```

Testing/fixes already performed in this handoff window:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --workspace --all-targets -- -D warnings` passed after fixing
  Rust warnings.
- `cargo test --workspace --all-targets` passed.
- `npm test` passed.
- `npm run build` passed and built XPI version `0.1.5`.
- Live Zotero smoke tests were run against the `test` collection for search,
  collections, items, attachments, notes, tags, export, annotations, OCR, and
  RAG paths.
- A Windows/WSL attachment path bug was fixed so `zotron attachments path`
  localizes Zotero-side Windows paths for the local CLI environment.

## What Was Learned

The important design lesson is that Zotron does not need to become MCP-first.

Zotron already has a command surface: the Rust CLI is a facade over XPI RPC.
The issue is that some commands are still too resource-shaped and not enough
task-shaped for agents.

Documented decision:

- Primary interface: Rust CLI.
- MCP: optional future compatibility layer only, not a mirror of all RPC or CLI
  commands.
- Keep skill docs short and use `zotron ... --help` for detailed discovery.
- Return compact JSON/JSONL and file paths for large artifacts.
- Improve CLI consistency rather than duplicating the surface through MCP.

The Zotero MCP Plugin review found useful capabilities but also exposed the
risk of MCP bloat:

- Around twenty tools are exposed at once.
- Several tools overlap semantically.
- Tool schemas are large.
- Large JSON results are still returned directly into model context.
- Mutation/write tools require careful safety defaults.

Use that project as a benchmark for capability grouping, not as a template for
Zotron's primary interface.

Detailed notes:

- `docs/2026-05-11-command-surface-lessons.md`
- `CLAUDE.md`, section `Design Note: CLI vs MCP`

## Current PRD / Roadmap / Milestone Files

Primary PRD:

- `docs/2026-05-01-api-v2-prd.md`
  - CLI-first API strategy.
  - Key-first public identifiers.
  - Standard list envelopes.
  - PDF evidence amendment: `parse-pdf`, `index-blocks`,
    `retrieve-blocks`, `locate-highlight-target`, `apply-annotations`.

OCR/RAG roadmap:

- `docs/2026-04-27-rag-ocr-roadmap.md`
  - Evidence producer strategy.
  - Sidecar storage policy.
  - PDF remains the human reading truth.
  - OCR/parser output is machine evidence.
  - Tables enter retrieval as structured textual evidence; figures/images keep
    references/captions/assets and use VLM only on demand.

PDF evidence milestones:

- `docs/2026-05-07-pdf-evidence-annotation-milestones.md`
  - Milestone 0: contract freeze.
  - Milestone 1: `parse-pdf`; MinerU path is working.
  - Milestone 2: `index-blocks`; chunk sidecars are partially implemented.
  - Milestone 3: `retrieve-blocks`; still product work.
  - Milestone 4: `locate-highlight-target`; still product work.
  - Milestone 5: `apply-annotations`; still product work.

Rust migration surface:

- `docs/rust-migration/cli-surface.md`
  - Current Rust command surface.
  - OCR/RAG subcommands.
  - Remaining Rust migration work.

Provider API notes:

- `docs/2026-05-08-provider-api-notes.md`
  - MinerU, GLM-OCR, PaddleOCR-VL, and embedding-provider API notes.
  - Do not put credentials in docs.
  - Generic VLM OCR-like output is out of scope for the current mainline.

Command surface lessons:

- `docs/2026-05-11-command-surface-lessons.md`
  - CLI vs MCP rationale.
  - Review of `cookjohn/zotero-mcp`.
  - Candidate aggregate CLI commands.

## What To Do Next

Priority 1: stabilize the Rust command surface.

- Reconcile list outputs so list/search commands consistently use:

```json
{
  "items": [],
  "total": 0,
  "limit": 50,
  "offset": 0,
  "hasMore": false
}
```

- Keep external `jq` as the filtering strategy.
- Keep command help concise and accurate.
- Avoid hidden Python behavior creeping back into Rust docs or skills.

Priority 2: add task-shaped facade commands without duplicating logic.

Candidate commands:

```text
zotron content get
zotron annotations search
zotron evidence find
zotron evidence annotate
zotron index status
```

These should reuse existing Rust/XPI RPC logic. They are aggregate/facade
commands for common agent workflows, not a second stack.

Priority 3: continue the PDF evidence pipeline.

- Finish sidecar-backed retrieval over `chunks.v1.jsonl`.
- Add production embedding write/read over `vectors.jsonl`.
- Implement `locate-highlight-target`:
  - quote-based input,
  - `block_key` input,
  - page/bbox input,
  - text highlight target when reliable,
  - area-box fallback when text position is not reliable.
- Implement `apply-annotations`:
  - batch apply,
  - color/comment/type/provenance,
  - dry-run,
  - result with `annotation_key`,
  - no public `*_id` fields.

Priority 4: provider normalization.

- MinerU is the first full parse/download/sidecar pipeline.
- GLM and Paddle adapters exist as provider contracts, but provider-native
  sidecar ingestion/asset preservation still needs product work.
- Tables should become independent retrievable chunks.
- Figure/chart/image pixels should not enter text embedding by default.

Priority 5: live verification.

Use Zotero collection `test` for manual/live smoke tests. A full feature test
should include:

- XPI loaded at expected version.
- Basic ping/system/version/list-methods.
- Collection search and get-items.
- Item/attachment fulltext/path.
- OCR parse-pdf into hidden sidecars.
- RAG hits from sidecar chunks.
- Annotation create/list/delete.
- At least one visible PDF annotation check in Zotero.

Do not call a narrow command smoke test "full testing" unless OCR, retrieval,
and annotation paths were exercised.

## Worktree Warning

At the time of this handoff, the worktree may contain both Rust changes and
Claude plugin packaging changes. Do not revert unrelated files.

Known modified areas from the prior session included:

```text
CLAUDE.md
crates/zotron-cli/src/lib.rs
crates/zotron-cli/tests/cli_contract.rs
claude-plugin/.codex-plugin/plugin.json
claude-plugin/bin/zotron
claude-plugin/scripts/setup-zotron.sh
claude-plugin/commands/
```

Before editing, run:

```bash
git status --short
git diff --stat
```

Then separate Rust product changes from plugin packaging changes when committing.
