---
name: setup
description: Set up Zotron — verify the XPI plugin is reachable on localhost:23119; if missing, download the release XPI to Downloads and guide Zotero's native install dialog. Use when the user asks to install, configure, bootstrap, or verify Zotron.
---

# Zotron Setup

Run this when the user has just installed the Zotron plugin or when `zotron ping` cannot reach Zotero.

## Goal

End state: `zotron ping` succeeds and the agent can call `zotron`, `zotron-rag`, and `zotron-ocr`.

## Distribution model

The repository does not track `zotron.xpi`. New installs download the release XPI. If GitHub is unavailable, set `ZOTRON_XPI_URLS` to one or more mirror URLs. If a local file is already available, set `ZOTRON_XPI_PATH=/path/to/zotron.xpi`.

If Zotron is already installed but the setup target version is newer, do not reinstall from setup. Tell the user to use Zotero's native update flow:

```text
Tools -> Plugins -> Zotron -> gear icon -> Check for Updates -> restart Zotero
```

## Procedure

1. Check `uv` is available.

```bash
command -v uv >/dev/null || echo "MISSING_UV"
```

2. Resolve the plugin root and run the setup script.

```bash
PLUGIN_ROOT="${CODEX_PLUGIN_ROOT:-${CLAUDE_PLUGIN_ROOT:-}}"
bash "$PLUGIN_ROOT/scripts/setup-zotron.sh"
```

3. If the bridge is live at the expected version, stop.

4. If the bridge is live but the plugin version is older, tell the user to update inside Zotero using the update flow above.

5. If the bridge is down, the script downloads or stages `zotron.xpi` into the user's real Downloads folder and prints the path as Zotero will see it.

6. Tell the user:

```text
In Zotero:
1. Tools -> Plugins
2. Gear icon -> Install Add-on From File
3. Choose zotron.xpi from the path printed by setup
4. Install, then restart Zotero
```

7. After restart, verify:

```bash
zotron ping
zotron system version
```

## Mirror Controls

```bash
ZOTRON_XPI_URLS='https://mirror.example/zotron.xpi https://github.com/dianzuan/zotron/releases/download/v0.1.5/zotron.xpi' \
  bash "$PLUGIN_ROOT/scripts/setup-zotron.sh"
```

Use `ZOTRON_XPI_PATH=/path/to/zotron.xpi` only when the file has already been downloaded through another channel.
