# Zotron Source Plugin System Design

Date: 2026-05-13

## Problem

Zotron's core handles Zotero read/write but has no academic paper discovery
capability. The existing CNKI plugin is a separate Claude Code plugin with
its own CLI, skills, and RPC integration — requiring independent installation
and registration. Adding English sources (OpenAlex, CrossRef, arXiv, etc.)
under the same ad-hoc pattern would fragment the user experience further.

## Design Principles

1. **Core independence** — Zotron core knows nothing about academic data
   sources. All source knowledge lives in plugins.
2. **Transparent proxy** — `zotron cnki search` and `zotron scholar search`
   feel identical to the user. No visible distinction between plugin types.
3. **Unix composability** — Plugins produce data (stdout JSON), `zotron push`
   writes to Zotero. Pipes connect them.
4. **Progressive skill disclosure** — Each plugin installs its own SKILL.md
   under zotron's skill namespace. Agent discovers sources on demand.

## Architecture

```text
Claude Code / Codex Agent
        │
  zotron plugin (single Claude Code plugin)
  skills: /zotron:zotero, /zotron:setup, /zotron:scholar, /zotron:cnki
        │
  zotron CLI (Rust binary)
  ├── built-in: items, collections, search, push, ocr, rag, ...
  ├── sources: discovers zotron-* on PATH
  └── sources sync: symlinks plugin skills into claude-plugin/skills/
        │
  ┌─────┴──────┐
  │            │
zotron-scholar    zotron-cnki
(Rust binary)     (Python binary)
  │                  │
REST APIs         Playwright + Camoufox
OpenAlex          kns.cnki.net
CrossRef
Semantic Scholar
Unpaywall
arXiv
```

Data flow for import:

```text
zotron-scholar fetch --doi 10.1038/xxx   (stdout: Paper JSON)
  | pipe
zotron push --collection "My Papers"     (writes to Zotero via RPC)
```

## Plugin Protocol

### Naming Convention

- Binary: `zotron-<name>` (on PATH)
- CLI usage: `zotron <name> <subcommand> [args]`
- Skill: `/zotron:<name>`
- Package: same as binary (`cargo install zotron-scholar`,
  `uv tool install zotron-cnki`)

### Required: `manifest` Subcommand

Every plugin must implement `zotron-<name> manifest`:

```json
{
  "name": "scholar",
  "version": "0.1.0",
  "description": "English academic sources: OpenAlex, CrossRef, Semantic Scholar, Unpaywall, arXiv",
  "capabilities": ["search", "fetch", "pdf"],
  "skill_dir": "/home/user/.cargo/share/zotron-scholar/skills"
}
```

Fields:
- `name` — short identifier, matches the binary suffix
- `version` — semver
- `description` — human-readable, shown in `zotron sources`
- `capabilities` — suggested values: `search`, `fetch`, `pdf`, `export`
- `skill_dir` — absolute path to the directory containing this plugin's
  SKILL.md for AI agents

### I/O Convention

- All output to stdout, must be valid JSON
- All errors to stderr
- Exit code 0 = success, non-zero = failure
- No mandatory JSON schema for command output — each plugin defines its own
  commands and response fields
- Plugins should follow zotron style where possible: key-first, compact JSON

### Writing to Zotero

Plugins do **not** call Zotero RPC directly. The standard path is:

```bash
zotron-<name> fetch <identifier> | zotron push [--collection NAME] [--on-duplicate skip|update|create]
```

`zotron push` already exists and accepts Zotero-format JSON from stdin.
Plugins output Zotero-compatible item JSON with an optional `_pdf` field
containing the local path to a downloaded PDF:

```json
{
  "itemType": "journalArticle",
  "title": "...",
  "creators": [{"firstName": "...", "lastName": "...", "creatorType": "author"}],
  "DOI": "10.1038/xxx",
  "abstractNote": "...",
  "date": "2024",
  "publicationTitle": "Nature",
  "volume": "625",
  "pages": "1-10",
  "url": "https://...",
  "_pdf": "/tmp/zotron-scholar/xxx.pdf"
}
```

`zotron push` reads `_pdf` and attaches the file automatically. This field
is stripped before sending to Zotero RPC.

## Plugin Discovery

### `zotron sources`

Scans `$PATH` for all `zotron-*` executables, calls each one's `manifest`
subcommand, aggregates results:

```json
{
  "sources": [
    {"name": "scholar", "version": "0.1.0", "description": "English academic sources", "capabilities": ["search", "fetch", "pdf"], "binary": "/home/user/.cargo/bin/zotron-scholar"},
    {"name": "cnki", "version": "0.2.0", "description": "中国知网", "capabilities": ["search", "fetch", "pdf"], "binary": "/home/user/.local/bin/zotron-cnki"}
  ]
}
```

### `zotron sources sync`

Links plugin skills into zotron's Claude Code plugin:

1. Calls `zotron sources` to discover all plugins
2. Reads each plugin's `manifest.skill_dir`
3. Creates symlinks in `claude-plugin/skills/`:
   ```
   claude-plugin/skills/scholar -> /path/to/zotron-scholar/skills/scholar
   claude-plugin/skills/cnki -> /path/to/zotron-cnki/skills/cnki
   ```
4. Removes symlinks for plugins no longer on PATH

### Transparent Proxy in `zotron` CLI

When `zotron <name> [args]` is invoked and `<name>` is not a built-in
subcommand:

1. Search PATH for `zotron-<name>`
2. If not found: error "unknown command '<name>'. No plugin 'zotron-<name>'
   found on PATH."
3. If found: exec `zotron-<name> [args]`, transparent passthrough of
   stdin/stdout/stderr and exit code

## Planned Plugins

### zotron-scholar (new, Rust)

English academic sources. Single binary, 5 internal sources selected via
`-s` flag:

| Source | Short name | Default | Purpose |
|--------|-----------|---------|---------|
| OpenAlex | `openalex` | yes | Primary search, 250M records, OA PDF URLs |
| Semantic Scholar | `s2` | no | Citation graphs, openAccessPdf |
| CrossRef | `crossref` | no | DOI → metadata authority |
| Unpaywall | `unpaywall` | no | DOI → OA PDF fallback |
| arXiv | `arxiv` | no | Preprints, 100% PDF coverage |

Commands:

```bash
zotron scholar search "deep learning" [--limit N] [-s openalex|s2]
zotron scholar fetch --doi 10.1038/xxx     # CrossRef metadata + Unpaywall PDF
zotron scholar fetch --arxiv 2301.00001    # arXiv metadata + PDF
zotron scholar manifest
```

PDF resolution chain in `fetch`:
1. arXiv ID present? → `arxiv.org/pdf/{id}` (deterministic)
2. DOI present? → CrossRef for metadata → Unpaywall for OA PDF
3. Search result? → use `pdf_url` from OpenAlex/S2 response

Dependencies: `clap`, `ureq`, `serde`, `serde_json`. No async runtime.

All 5 APIs are zero-config (no API keys required). Optional `mailto` param
via `ZOTRON_SCHOLAR_EMAIL` env var for polite-pool rate limits on
OpenAlex/CrossRef/Unpaywall.

### zotron-cnki (migrated from cnki-plugin, Python)

Migration checklist:
1. `pyproject.toml`: rename script entry from `cnki` to `zotron-cnki`
2. Add `manifest` subcommand returning standard JSON
3. Add `emit-skill` subcommand (or bundle skills with package data)
4. Change `export` output to Zotero-compatible JSON on stdout instead of
   calling RPC directly
5. Remove `.claude-plugin/` directory (skills aggregated by zotron)
6. Remove dependency on `zotron` Python package (no more direct ZoteroRPC)

## Skill Structure (after sync)

```
claude-plugin/skills/
├── setup/SKILL.md            ← core: installation guide
├── zotero/SKILL.md           ← core: Zotero management commands
│   ├── search.md
│   ├── manage.md
│   ├── export.md
│   ├── ocr.md
│   └── rag.md
├── scholar/SKILL.md          ← symlink from zotron-scholar
└── cnki/SKILL.md             ← symlink from zotron-cnki
```

The core `/zotron:zotero` skill mentions: "Use `zotron sources` to list
available academic sources. Each source has its own skill for detailed
usage."

## End-to-End User Flow

```bash
# Install
cargo install zotron                # core
cargo install zotron-scholar        # English sources
uv tool install zotron-cnki         # Chinese sources (CNKI)
zotron sources sync                 # aggregate skills

# Discover
zotron sources                      # list installed sources

# Search
zotron scholar search "transformer architecture" --limit 10
zotron cnki search "乡村振兴" --limit 10

# Import to Zotero
zotron scholar fetch --doi 10.1038/xxx | zotron push --collection "My Papers"
zotron cnki fetch URL | zotron push --collection "课题文献"

# Claude Code skills (user invokes these slash commands):
#   /zotron:scholar  → AI loads English source usage guide
#   /zotron:cnki     → AI loads CNKI usage guide
#   /zotron:zotero   → AI loads Zotero management guide
```

## Changes to Zotron Core

New code in `crates/zotron-cli/src/lib.rs`:

1. **`sources` subcommand** — scan PATH, call manifests, aggregate JSON
2. **`sources sync` subcommand** — read manifests, create/clean symlinks
3. **Unknown command fallback** — when clap doesn't match a subcommand,
   search PATH for `zotron-<name>` and exec it with remaining args
4. **`push` enhancement** — read `_pdf` field from input JSON and auto-attach

Estimated: ~150-250 lines of new Rust code in the CLI crate.

## Out of Scope

- Plugin version management / auto-update (use cargo/pip for now)
- Plugin registry / marketplace
- Cross-source deduplication or unified search
- MCP server for plugins
- GUI for source management
