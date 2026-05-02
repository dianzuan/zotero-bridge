# Zotron — Project Rules

## Architecture

Three layers: XPI plugin (TypeScript, `src/`) → Python CLI/SDK (`claude-plugin/python/`) → Claude Code plugin (`claude-plugin/skills/`).

- XPI: JSON-RPC 2.0 server inside Zotero, 86 methods across 11 namespaces
- Python CLI: typer-based, noun-verb subcommands (`zotron items get`, `zotron search quick`)
- Plugin: skills + agents for Claude Code and Codex

## Test Commands

```bash
# XPI tests (127 mocha)
npx tsx node_modules/.bin/mocha 'test/**/*.test.ts' --timeout 30000

# CLI tests (344 pytest)
cd claude-plugin/python && uv run pytest tests/ -q

# Type check
npx tsc --noEmit

# Lint
cd claude-plugin/python && uv run ruff check zotron/

# Build XPI
npm run build  # → .scaffold/build/zotron.xpi
```

## Release Workflow

1. Work on `dev` branch
2. `git push origin dev`
3. `gh pr create --base main --head dev`
4. Merge PR on GitHub (never merge locally)
5. `git checkout main && git pull`
6. Build + release from main

**Never** do `git checkout main && git merge dev && git push` — always go through a PR.

## Version Bumps

All 7 files must be updated together:

1. `package.json`
2. `addon/manifest.json`
3. `src/handlers/system.ts` (plugin version string)
4. `update.json`
5. `update-beta.json`
6. `claude-plugin/.claude-plugin/plugin.json`
7. `claude-plugin/.codex-plugin/plugin.json`
8. `claude-plugin/python/pyproject.toml`

Use patch bumps (0.1.x) unless explicitly told otherwise. Version tags are clean: `v0.1.5`, no suffix.

## Release Channels

| Channel | Command |
|---------|---------|
| GitHub Release (XPI) | `gh release create v0.1.x .scaffold/build/zotron.xpi --title "v0.1.x" --notes ""` |
| PyPI | `cd claude-plugin/python && uv build && uv publish` |
| Claude Code Plugin | Auto-pulled from GitHub main |

## Plugin Structure (Claude Code + Codex)

```
claude-plugin/
├── .claude-plugin/plugin.json    # Claude Code manifest
├── .codex-plugin/plugin.json     # Codex manifest
├── skills/                       # Shared — both platforms read this
│   ├── setup/SKILL.md            # /zotron:setup
│   └── zotero/SKILL.md           # /zotron:zotero
├── agents/zotero-researcher.md
├── bin/
├── scripts/
└── python/
```

No `commands/` directory — use `skills/` only. Both platforms share the same `skills/` dir.

## What NOT to Commit

- Conversation/discussion docs (design exploration artifacts)
- Subagent execution plans (`docs/superpowers/plans/`)
- `.claude/worktrees/` leftovers

Only commit normative docs: PRDs, API specs, READMEs.

## RPC API Conventions

- **Key-first**: items and collections use `key` (8-char alphanumeric), no numeric `id` in responses
- **Version field**: all items include `version` for sync
- **Mutation returns**: `{ok: true, key: "..."}` consistently
- **Batch params**: all accept `(number | string)[]` — both numeric IDs and key strings
- **Fuzzy suggestions**: unknown methods get "Did you mean?" via Levenshtein matching
- **Errors**: JSON-RPC 2.0 `{code, message}` — `-32602` caller error, `-32603` server error

## Naming

- Plugin name: `zotron`
- Skills/agents user-facing name: `zotero` (users say "我的 Zotero 文献库" not "我的 zotron")
- Slash commands: always `/zotron:xxx` in docs, never bare `/xxx`
