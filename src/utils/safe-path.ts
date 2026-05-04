// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill

const ILLEGAL_WIN_CHARS = /[""«»]/g;

/**
 * Sanitize and translate a file path for Zotero's file APIs.
 *
 * Handles two problems:
 * 1. WSL: POSIX paths (/tmp/..., /mnt/c/...) need translation to Windows
 *    paths because Zotero runs on the Windows side.
 * 2. CNKI filenames may contain smart quotes or typographic characters
 *    that are illegal on NTFS.
 */
export function sanitizePath(rawPath: string): string {
  let p = rawPath;

  // On Windows Zotero, translate POSIX paths from WSL
  if (typeof Zotero !== "undefined" && (Zotero as any).isWin) {
    // Already a Windows path — leave it
    if (/^[A-Z]:\\/i.test(p) || p.startsWith("\\\\")) {
      // just strip illegal chars below
    } else if (p.startsWith("/mnt/") && p.length > 6) {
      // /mnt/c/Users/... → C:\Users\...
      const drive = p.charAt(5).toUpperCase();
      p = `${drive}:${p.slice(6).replace(/\//g, "\\")}`;
    } else if (p.startsWith("/")) {
      // /tmp/... or /home/... → \\wsl$\<distro>\...
      // Zotero on Windows can access WSL filesystems via UNC path
      try {
        const distro = (Zotero as any).wslDistroName;
        if (distro) {
          p = `\\\\wsl$\\${distro}${p.replace(/\//g, "\\")}`;
        }
      } catch { /* fall through with original path */ }
    }
  }

  // Strip NTFS-illegal characters that CNKI filenames sometimes contain
  p = p.replace(ILLEGAL_WIN_CHARS, "");
  return p;
}
