# Command Surface Lessons: CLI, MCP, and Agent Use

Date: 2026-05-11

This note records the command-surface decision after reviewing MCP guidance and
nearby Zotero AI/MCP plugins. It is a product/API design note, not a promise to
implement MCP.

## Current Decision

Zotron's primary agent interface is the Rust `zotron` CLI.

The CLI is already a command surface over Zotero-side RPC. The problem is not
that agents are using raw RPC directly. The problem is that parts of the CLI
still feel like an engineering resource API instead of a stable task API for
agents.

MCP may be added later only as an optional compatibility layer for clients that
cannot run shell commands. It should not mirror every RPC method or every CLI
subcommand.

## Why CLI Remains Primary

For shell-capable agents such as Codex and Claude Code, CLI has several
practical advantages:

- `--help` is discovered on demand instead of loading every command definition
  into the model context.
- Large OCR/RAG/fulltext results can be written to files and inspected with
  `jq`, `rg`, `head`, or focused follow-up commands.
- JSONL output lets agents stream and filter evidence without loading a whole
  corpus result into context.
- Shell composition can collapse multi-step local workflows into one command or
  script and return a compact summary.
- Existing agent training strongly favors shell patterns over arbitrary custom
  tool schemas.

These advantages disappear if the skill or README dumps every command,
parameter, example, and response schema into context. CLI is not magically
token-cheap; it is easier to keep token cost under the agent's control.

## Why MCP Can Become Expensive

The token cost is not only "all tools are exposed at the beginning." Important
cost sources are:

- Tool schemas are verbose: each tool has descriptions, JSON schema,
  properties, enums, required fields, and examples.
- Large tool lists increase tool-selection ambiguity.
- Multi-step workflows often push each intermediate tool result back into model
  context.
- Many MCP servers return large JSON/text payloads directly as tool content.
- Unless the server supports deferred loading, search/fetch separation, result
  limits, field projection, or file references, large local data gets copied
  into the model context.

Official MCP/Claude guidance has moved toward progressive discovery:
deferred-loading tools, tool search, and keeping only a few high-frequency
tools eagerly available.

Useful references:

- MCP architecture: https://modelcontextprotocol.io/docs/learn/architecture
- MCP client best practices: https://modelcontextprotocol.io/docs/develop/clients/client-best-practices
- Claude tool search/deferred loading: https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-search-tool
- Claude tool definition guidance: https://platform.claude.com/docs/en/agents-and-tools/tool-use/define-tools
- Claude programmatic tool calling: https://platform.claude.com/docs/en/agents-and-tools/tool-use/programmatic-tool-calling

## Review: `cookjohn/zotero-mcp`

The Zotero MCP Plugin is useful as a benchmark, but it also demonstrates the
risks Zotron wants to avoid.

Good ideas to learn from:

- A unified `get_content` concept that hides whether content comes from
  metadata, notes, PDF fulltext, or attachments.
- Content modes such as minimal, preview, standard, and complete.
- Annotation search and filtering by query, type, color, and tags.
- Semantic status/index UI concepts inside Zotero.
- A fulltext cache abstraction for faster repeated lookup.

Problems for Zotron's target use:

- It exposes around twenty tools at once through `tools/list`.
- Several tools overlap semantically: library search, fulltext search,
  semantic search, fulltext database search, and content retrieval all compete
  for similar intents.
- Some schemas are large and knob-heavy, especially content retrieval.
- Results are commonly returned as JSON string content, so large results still
  enter model context.
- `complete` content modes can be unlimited unless the caller is careful.
- Write tools and collection mutations are part of the exposed surface; this
  requires strong confirmation and safety defaults.
- Local HTTP/MCP surfaces need careful auth/rate-limit assumptions if mutations
  are enabled.

Conclusion: copy the capability grouping and some UX ideas, not the "mirror a
large tool catalog into MCP" surface.

## Zotron Command Surface Lessons

The right improvement path is to make the CLI more predictable and task-shaped,
not to duplicate it in MCP.

Rules:

- Keep one public binary: `zotron`.
- Keep OCR/RAG under subcommands, not separate binaries such as `zotron-ocr`.
- Prefer key-based public parameters and JSON fields.
- Keep skill docs short. Let `zotron ... --help` carry detailed discovery.
- Use external `jq` for filtering; do not reintroduce embedded `--jq`.
- Use consistent list envelopes: `items`, `total`, `limit`, `offset`,
  `hasMore`.
- Use JSON for structured results and JSONL for large retrieval streams.
- For large artifacts, write files and return paths plus compact summaries.
- If a user intent is one conceptual step, consider one aggregate CLI command.
- Mutations should be explicit and automation-safe: clear names, dry-run where
  useful, and compact verification output.

Candidate aggregate commands to consider:

- `zotron content get`: unified item/attachment content retrieval with mode,
  include flags, and file-output support.
- `zotron annotations search`: query/filter annotations by text, color, type,
  tag, item, or attachment.
- `zotron evidence find`: retrieve evidence spans with stable provenance.
- `zotron evidence annotate`: find or consume evidence targets and write
  Zotero-native annotations.
- `zotron index status`: summarize OCR/chunk/vector readiness across a
  collection or item.

These should reuse existing Rust/XPI RPC implementations. They are facade
commands, not a second product stack.

## Optional MCP Shape If Added Later

If Zotron adds MCP, it should be a thin compatibility surface over the CLI/RPC
stack. Do not expose every noun/verb command.

Possible MCP shape:

```text
zotron_search
zotron_fetch
zotron_annotate
zotron_status
zotron_execute
```

Even this should support strict limits, field projection, file references for
large results, and conservative write/mutation defaults.
