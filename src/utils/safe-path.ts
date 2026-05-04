// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill

const ILLEGAL_WIN_CHARS = /[""«»]/g;

/**
 * Sanitize a file path for Zotero.File.pathToFile / Zotero.Attachments.importFromFile.
 *
 * Zotero.File.pathToFile internally does `new FileUtils.File(path)` →
 * `nsIFile.initWithPath(path)`. initWithPath accepts full Unicode (AString),
 * so Chinese characters are fine. The real failure cases are:
 *   1. Smart quotes / typographic dashes in filenames (illegal on Windows NTFS)
 *   2. Path separators wrong for the OS
 *
 * All mainstream Zotero 8 plugins (jasminum, attanger) pass plain string paths
 * directly. We do the same — just clean up known problematic characters first.
 */
export function sanitizePath(rawPath: string): string {
  let p = rawPath;
  // Normalize path separators on Windows
  if (typeof Zotero !== "undefined" && (Zotero as any).isWin) {
    p = p.replace(/\//g, "\\");
  }
  // Strip characters illegal on Windows NTFS that CNKI filenames sometimes contain
  p = p.replace(ILLEGAL_WIN_CHARS, "");
  return p;
}
