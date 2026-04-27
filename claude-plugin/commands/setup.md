---
description: Set up Zotron — verify the XPI plugin is installed and reachable, install it if missing, and confirm the bridge is live on localhost:23119.
---

# /zotron:setup — Zotron bootstrap

Run this when the user has just installed the `zotron` Claude Code plugin and needs to get the Zotero side wired up.

## Goal

End state: `system.ping` over `localhost:23119/zotron/rpc` returns `{"pong": true, ...}` and the user can ask Claude to do real Zotero work.

## Procedure

### 1. Check `uv` is available

The bundled `zotron` / `zotron-rag` / `zotron-ocr` shims invoke `uv run`. If `uv` is missing, nothing else matters.

```bash
command -v uv >/dev/null || echo "MISSING_UV"
```

If missing, point the user at https://docs.astral.sh/uv/getting-started/installation/ and stop — they need to install `uv` themselves (it's a single-command install: `curl -LsSf https://astral.sh/uv/install.sh | sh`). Do not proceed until `uv` is on PATH.

### 2. Ping the bridge

```bash
curl -sf -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","params":{},"id":1}' \
  && echo OK || echo DOWN
```

- **OK** → the bridge is live. Run `zotron ping` to print version info, confirm to the user, and stop.
- **DOWN** → continue to step 3.

### 3. Diagnose why it's down

Two reasons it could be down:

1. **Zotero is not running.** Tell the user to start Zotero, wait ~5 seconds, and re-ping. If still down → step 4 (XPI not installed).
2. **XPI is not installed.** Continue to step 4.

You can't reliably tell these apart from the shell. Ask the user: *"Is Zotero currently open?"* — if yes, assume the XPI is the issue.

### 4. Install the XPI

#### 4a. Get the download URL

Fetch the latest release's XPI asset from GitHub:

```bash
curl -sL https://api.github.com/repos/dianzuan/zotron/releases/latest \
  | grep -oE '"browser_download_url": "[^"]+\.xpi"' \
  | head -1 \
  | sed 's/.*: "//; s/"$//'
```

If the user is on a beta channel or wants a specific version, list releases instead:
```bash
curl -sL https://api.github.com/repos/dianzuan/zotron/releases | grep -E '"tag_name"|browser_download_url.*xpi' | head -20
```

#### 4b. Download the XPI

Download to a stable location the user can find:

```bash
mkdir -p "$HOME/Downloads"
curl -sL -o "$HOME/Downloads/zotron.xpi" "<URL_FROM_4A>"
ls -lh "$HOME/Downloads/zotron.xpi"
```

On WSL, also offer to drop it on the Windows side (`/mnt/c/Users/<USER>/Downloads/`) — Zotero on Windows can't read the WSL filesystem directly.

#### 4c. Walk the user through installing it

Tell the user **verbatim**:

> Now in Zotero:
> 1. **Tools → Plugins**
> 2. Click the gear icon (⚙) at the top right
> 3. **Install Add-on From File…**
> 4. Pick `zotron.xpi` from the path I just downloaded it to
> 5. **Restart Zotero**
>
> Tell me when you've restarted and I'll verify.

Then wait for the user.

### 5. Verify

After the user confirms restart, ping again (step 2). Expected: `OK`.

If still `DOWN`:
- Check Zotero's plugin pane: `Tools → Plugins` should show `Zotron` as enabled.
- Check the port isn't blocked: `curl -v http://localhost:23119/` should return Zotero's connector page even without the plugin (Zotero's own HTTP service).
- If port 23119 returns connection refused, Zotero's HTTP server is off — `Edit → Settings → Advanced → Config Editor` → `extensions.zotero.httpServer.enabled` must be `true`.

Once `system.ping` returns `OK`, run `zotron system.libraries` to print the user's library names and confirm a working bridge end-to-end. Hand off to the `zotero` skill for actual work.

## Skip when

- The user says "I already installed the XPI manually" → just run step 2 (ping) and confirm.
- The user is on Zotero 7 → warn them: only Zotero 8.0+ is verified. Offer to proceed anyway.
