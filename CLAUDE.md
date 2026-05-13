# Zotron — Project Rules

## Architecture

Three layers: XPI plugin (TypeScript, `src/`) → Rust CLI/SDK (`crates/`) → Claude Code/Codex plugin (`claude-plugin/`).

- XPI: JSON-RPC 2.0 server inside Zotero, 86 methods across 11 namespaces
- Rust CLI: noun-verb subcommands (`zotron items get`, `zotron search "query"`); published on crates.io as `zotron`
- Plugin: skills for Claude Code and Codex (setup + zotero, no agents dir)

## Branching Strategy

**One feature per branch.** Don't pile unrelated changes onto a long-lived branch.

- `main` — stable, releasable
- Feature branches: `feature/<name>` (e.g. `feature/hybrid-rag`, `feature/cli-cleanup`)
- Each feature branch PRs into `main` when complete
- Never merge locally — always go through a GitHub PR
- Keep branches short-lived: finish, PR, merge, delete

Bad: a single `rust-migration` branch with 50 commits spanning CLI cleanup + RAG + docs + publishing. Good: separate `feature/cli-unification`, `feature/hybrid-rag`, `feature/crates-publish` branches.

## Design Note: CLI vs MCP

Zotron's primary interface is the Rust CLI, not MCP. The project goal is to give shell-capable agents such as Codex and Claude Code a stable, low-token, composable command surface over Zotero RPC.

Do not solve CLI roughness by duplicating the surface in MCP. Prefer improving CLI help, stable output envelopes, compact JSONL/file outputs, and a small number of task-level aggregate commands.

## Test Commands

```bash
# XPI tests (127 mocha)
npx tsx node_modules/.bin/mocha 'test/**/*.test.ts' --timeout 30000

# CLI tests (47 rust contract tests)
cargo test -p zotron

# Types tests (50 tests including BM25/cosine/RRF)
cargo test -p zotron-types

# Type check
npx tsc --noEmit

# Build XPI
npm run build  # → .scaffold/build/zotron.xpi
```

## Release Workflow

1. Work on feature branch
2. `git push origin feature/<name>`
3. `gh pr create --base main --head feature/<name>`
4. Merge PR on GitHub (never merge locally)
5. `git checkout main && git pull`
6. Build + release from main

**Never** do `git checkout main && git merge <branch> && git push` — always go through a PR.

## Version Bumps

All files must be updated together:

1. `package.json`
2. `addon/manifest.json`
3. `src/handlers/system.ts` (plugin version string)
4. `update.json`
5. `update-beta.json`
6. `claude-plugin/.claude-plugin/plugin.json`
7. `claude-plugin/.codex-plugin/plugin.json`
8. `crates/zotron-cli/Cargo.toml` (+ zotron-rpc, zotron-types if publishing)

Use patch bumps (0.1.x) unless explicitly told otherwise.

## Release Channels

| Channel | Command |
|---------|---------|
| GitHub Release (XPI) | `gh release create v0.1.x .scaffold/build/zotron.xpi --title "v0.1.x" --notes ""` |
| crates.io | `cargo publish -p zotron-types && cargo publish -p zotron-rpc && cargo publish -p zotron` |
| Claude Code Plugin | Auto-pulled from GitHub main |

## Plugin Structure (Claude Code + Codex)

```
claude-plugin/
├── .claude-plugin/plugin.json    # Claude Code manifest
├── .codex-plugin/plugin.json     # Codex manifest
├── skills/                       # Shared — both platforms read this
│   ├── setup/SKILL.md            # /zotron:setup
│   └── zotero/SKILL.md           # /zotron:zotero
├── bin/
└── scripts/
```

No `commands/` directory, no `agents/` directory — use `skills/` only.

## What NOT to Commit

- Conversation/discussion docs (design exploration artifacts)
- Subagent execution plans (`docs/superpowers/plans/`)
- `.claude/worktrees/` leftovers
- `.superpowers/` brainstorming artifacts
- Python legacy code (deleted)

Only commit normative docs: PRDs, API specs, READMEs.

## RPC API Conventions

- **Key-first**: items and collections use `key` (8-char alphanumeric), no numeric `id` in responses
- **Version field**: all items include `version` for sync
- **Mutation returns**: `{ok: true, key: "..."}` consistently
- **Batch params**: all accept `(number | string)[]` — both numeric IDs and key strings
- **Fuzzy suggestions**: unknown methods get "Did you mean?" via Levenshtein matching
- **Errors**: JSON-RPC 2.0 `{code, message}` — `-32602` caller error, `-32603` server error
- **Settings secrets**: `settings.getRaw` returns unredacted values (local CLI use); `settings.get`/`getAll` redact API keys

## Naming

- Plugin name: `zotron`
- Skills user-facing name: `zotero` (users say "我的 Zotero 文献库" not "我的 zotron")
- Slash commands: always `/zotron:xxx` in docs, never bare `/xxx`
- CLI subcommands: unified style (`search "query"` not `search quick "query"`, `export --format bibtex` not `export bibtex`)

## RAG Architecture

Retrieval runs in Rust CLI, not XPI JS. XPI provides settings storage and metadata resolution only.

- **OCR**: `ocr process --provider glm/mineru/paddle` → blocks → chunks → auto-embed → sidecar files
- **Retrieval**: `rag search` reads sidecar chunks + vectors locally, does BM25 + cosine + RRF hybrid
- **Settings**: stored in Zotero preference pane, read via `settings.getRaw` RPC
- **Vectors**: stored per provider+model as `embeddings/{provider}--{model}.jsonl` — switching providers doesn't destroy old vectors
- **Fallback**: no sidecar files → falls back to XPI lexical search
