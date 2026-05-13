// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill

const ILLEGAL_WIN_CHARS = /[""«»]/g;

/**
 * Sanitize a file path before passing to Zotero.File.pathToFile().
 *
 * Strips NTFS-illegal characters (smart quotes, etc.) that external filenames
 * sometimes contain. Does NOT do WSL path translation — that is the
 * client's responsibility (see zotron.paths.zotero_path in Python).
 */
export function sanitizePath(rawPath: string): string {
  return rawPath.replace(ILLEGAL_WIN_CHARS, "");
}
