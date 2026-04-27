---
description: Set up Zotron — verify the XPI plugin is reachable on localhost:23119, semi-automatically install the bundled XPI into Zotero if missing.
---

# /zotron:setup — Zotron bootstrap

Run this when the user has just installed the `zotron` Claude Code plugin and needs to get the Zotero side wired up.

## Goal

End state: `system.ping` over `localhost:23119/zotron/rpc` returns `{"pong": true, ...}` and the user can ask Claude to do real Zotero work.

## What "semi-automatic" means

The XPI ships bundled with the plugin at `${CLAUDE_PLUGIN_ROOT}/xpi/zotron.xpi`. We don't download anything from GitHub — we just hand that local file to Zotero, which pops its native install dialog. The user clicks **Install** once and restarts Zotero. No Tools → Plugins → ⚙ → Install From File hunt.

## Procedure

### 1. Check `uv` is available

The bundled `zotron` / `zotron-rag` / `zotron-ocr` shims invoke `uv run`. If `uv` is missing, nothing else matters.

```bash
command -v uv >/dev/null || echo "MISSING_UV"
```

If missing, point the user at https://docs.astral.sh/uv/getting-started/installation/ and stop — single-command install: `curl -LsSf https://astral.sh/uv/install.sh | sh`. Do not proceed until `uv` is on PATH.

### 2. Ping the bridge

```bash
curl -sf -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","params":{},"id":1}' \
  && echo OK || echo DOWN
```

- **OK** → bridge is live. Run `zotron ping` to print version, confirm to the user, stop.
- **DOWN** → continue.

### 3. Confirm Zotero is running

Ask the user: *"Is Zotero currently open?"*

- **No** → tell them to start Zotero, wait ~5s, re-ping (step 2). If still DOWN → step 4.
- **Yes** → assume the XPI is missing → step 4.

### 4. Verify the bundled XPI exists

```bash
test -f "${CLAUDE_PLUGIN_ROOT}/xpi/zotron.xpi" && echo OK || echo BUNDLED_XPI_MISSING
```

If `BUNDLED_XPI_MISSING`, the plugin install is broken. Tell the user:
> Reinstall the plugin: `/plugin uninstall zotron@zotron` then `/plugin install zotron@zotron`.

### 5. Hand the XPI to Zotero (semi-auto install)

Detect the platform and launch Zotero with the bundled XPI. Zotero will open a native install dialog.

```bash
XPI="${CLAUDE_PLUGIN_ROOT}/xpi/zotron.xpi"

case "$(uname -s)" in
  Darwin*)
    # macOS — use `open -a` to launch Zotero with the file
    open -a Zotero "$XPI"
    echo LAUNCHED_MAC
    ;;

  Linux*)
    if grep -qi microsoft /proc/version 2>/dev/null && ! command -v zotero >/dev/null 2>&1; then
      # WSL with Zotero on the Windows host — copy XPI to Windows %TEMP% (always
      # under the user's profile, no hardcoded drive letter), then launch via cmd.exe.
      WIN_TEMP=$(cmd.exe /c "echo %TEMP%" 2>/dev/null | tr -d '\r\n')
      if [ -z "$WIN_TEMP" ]; then
        echo NO_WIN_TEMP
      else
        WSL_TEMP=$(wslpath -u "$WIN_TEMP")
        cp "$XPI" "$WSL_TEMP/zotron.xpi"
        # cmd.exe expects backslash; the empty "" is the start-command title slot.
        cmd.exe /c start "" "$WIN_TEMP\\zotron.xpi" 2>/dev/null
        echo LAUNCHED_WSL
      fi
    elif command -v zotero >/dev/null 2>&1; then
      # Native Linux (or WSLg with Linux Zotero) — Zotero handles its own .xpi files.
      zotero "$XPI" >/dev/null 2>&1 &
      echo LAUNCHED_LINUX
    else
      echo NO_ZOTERO_LAUNCHER
    fi
    ;;

  MINGW*|MSYS*|CYGWIN*)
    # Native Windows shell (Git Bash / Cygwin / MSYS2) — convert path and use file association.
    cmd.exe /c start "" "$(cygpath -w "$XPI")" 2>/dev/null
    echo LAUNCHED_WIN
    ;;

  *)
    echo UNKNOWN_PLATFORM
    ;;
esac
```

**Branches:**

- `LAUNCHED_*` → Zotero should now show an "Install add-on?" dialog. Continue to step 6.
- `NO_ZOTERO_LAUNCHER` (Linux but no `zotero` binary, not WSL) → Zotero isn't installed or isn't on PATH. Manual fallback (below).
- `NO_WIN_TEMP` (WSL but `cmd.exe` failed) → WSL interop is broken. Manual fallback.
- `UNKNOWN_PLATFORM` → odd `uname`. Manual fallback.

**Manual fallback** — tell the user verbatim:

> Open Zotero → **Tools → Plugins** → click the gear icon (⚙) → **Install Add-on From File…** → pick this file: `${CLAUDE_PLUGIN_ROOT}/xpi/zotron.xpi` → click **Install** → **Restart Zotero**.
>
> (On WSL with Windows Zotero: I copied it to `$WIN_TEMP\zotron.xpi` — drag that file onto Zotero's plugin window.)

### 6. Walk the user through the dialog

When step 5 launches Zotero, tell the user **verbatim**:

> Zotero should now show an **"Install add-on?"** dialog. Click **Install**, then click **Restart** when prompted.
>
> Tell me when Zotero has restarted and I'll verify.

Then wait.

### 7. Verify

After the user confirms restart, ping again (step 2). Expected: `OK`.

If still `DOWN`:
- `Tools → Plugins` should list `Zotron` as enabled. If it's there but greyed out, click **Enable**.
- Port 23119 connection refused? Zotero's HTTP server is off: `Edit → Settings → Advanced → Config Editor` → set `extensions.zotero.httpServer.enabled = true`, restart.
- Mismatch warning ("not compatible")? Only Zotero 8.0+ is verified. Warn the user; offer to proceed anyway.

Once `system.ping` returns `OK`, run `zotron system.libraries` to print the user's library names. Hand off to the `zotero` skill for actual work.

## Skip when

- The user says they already installed the XPI manually → just run step 2 (ping) and confirm.
- The user is on Zotero 7 → warn them only Zotero 8.0+ is verified. Offer to proceed anyway.
