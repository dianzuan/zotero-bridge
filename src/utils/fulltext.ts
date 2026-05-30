// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/utils/fulltext.ts

export type FullTextResult = {
  key: string;
  content: string;
  indexedChars: number;
  totalChars: number;
};

/**
 * Extract full text for a single attachment.
 *
 * Reads Zotero's full-text cache file first; if it's empty and the attachment
 * is a PDF, falls back to live extraction via PDFWorker. Returns the extracted
 * text along with the indexed/total char counts (sourced from the fulltextItems
 * table, or the live char count when the fallback path produced the text).
 */
export async function extractAttachmentFullText(att: Zotero.Item): Promise<FullTextResult> {
  const cacheFile = Zotero.Fulltext.getItemCacheFile(att);
  let content = "";
  let fromFallback = false;
  try {
    content = (await Zotero.File.getContentsAsync(cacheFile.path) as string) ?? "";
  } catch {
    content = "";
  }

  // Fallback: cache empty → live extraction via PDFWorker
  if (!content && (att as any).attachmentContentType === "application/pdf") {
    try {
      const result = await (Zotero as any).PDFWorker.getFullText(att.id);
      content = (result?.text ?? "").replace(/\f/g, "\n");
      if (content) fromFallback = true;
    } catch { /* PDFWorker unavailable */ }
  }

  const rows = (
    (await Zotero.DB.queryAsync(
      "SELECT indexedChars, totalChars FROM fulltextItems WHERE itemID=?",
      [att.id],
    )) as Array<{ indexedChars: number; totalChars: number }>
  ) ?? [];
  const meta = rows[0] ?? { indexedChars: 0, totalChars: 0 };

  return {
    key: att.key,
    content: content ?? "",
    indexedChars: fromFallback ? content.length : (meta.indexedChars ?? 0),
    totalChars: fromFallback ? content.length : (meta.totalChars ?? 0),
  };
}
