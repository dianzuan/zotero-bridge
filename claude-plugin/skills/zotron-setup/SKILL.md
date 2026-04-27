---
name: zotron-setup
description: "Set up Zotron for Codex: verify the Zotero bridge, expose the bundled zotron CLI shims, download the release XPI to Downloads when needed, and guide the user through Zotero's native add-on install/update flow. Use when the user asks to install, configure, bootstrap, or verify Zotron."
---

# Zotron Setup

Run this when the user has just installed the Zotron Codex plugin or when `zotron ping` cannot reach Zotero.

## Goal

End state: `zotron ping` succeeds and Codex can call `zotron`, `zotron-rag`, and `zotron-ocr`.

## Procedure

1. Resolve the plugin root.

Prefer `CODEX_PLUGIN_ROOT`. If it is not set, use `CLAUDE_PLUGIN_ROOT`. If neither exists, locate the installed plugin root that contains `bin/zotron`, `python/`, and `scripts/setup-zotron.sh`.

2. Run the bundled setup script:

```bash
bash "$CODEX_PLUGIN_ROOT/scripts/setup-zotron.sh"
```

If `CODEX_PLUGIN_ROOT` is not set but the current working directory is this repository, run:

```bash
bash claude-plugin/scripts/setup-zotron.sh
```

3. If the script reports that the bridge is already live, stop after showing the successful `zotron ping` result.

4. If the script stages `zotron.xpi`, tell the user to install it in Zotero:

```text
Tools -> Plugins -> gear icon -> Install Add-on From File -> choose zotron.xpi -> restart Zotero
```

5. After restart, verify:

```bash
zotron ping
zotron system libraries
```

## Notes

Zotero does not provide a reliable command-line XPI install path. The script can expose the CLI shims and copy the XPI to Downloads, but the final add-on confirmation still happens inside Zotero.

Generated XPI files are not tracked in the repository or bundled in the plugin. For first install, setup downloads the release XPI. If GitHub is unavailable, set `ZOTRON_XPI_URLS` to mirror URLs. If Zotron is already installed but old, use Zotero's add-on update flow instead of reinstalling from setup.
