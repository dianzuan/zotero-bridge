// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/annotations.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem } from "../utils/guards";
import { validateAnnotationParams } from "../utils/annotation";
import { findQuoteInChars, getRangeRects } from "../utils/pdf-locate";

/**
 * Resolve a parentKey to the annotation-bearing attachment item.
 *
 * If the item is already an attachment, return it directly.
 * If the item is a regular item, find its first PDF attachment and return that.
 * Throws -32602 if the item has no PDF attachments.
 */
async function resolveAnnotationParent(parentKey: number | string): Promise<Zotero.Item> {
  const item = await requireItem(parentKey);
  if (item.isAttachment()) return item;

  const attIDs = item.getAttachments ? item.getAttachments() : [];
  for (const attID of attIDs) {
    const att = await Zotero.Items.getAsync(attID);
    if (att && att.isAttachment() && (att as any).attachmentContentType === "application/pdf") {
      return att;
    }
  }
  throw { code: -32602, message: `No PDF attachment found for item ${item.key ?? parentKey}` };
}

async function searchReaderForQuote(
  reader: any,
  quote: string,
  pageIndex?: number,
): Promise<{ pageIndex: number; rects: number[][] } | null> {
  const internalReader = reader?._internalReader;
  const primaryView = internalReader?._primaryView ?? internalReader?._state;
  if (!internalReader || !primaryView) return null;

  const pdfPages = primaryView._pdfPages ?? {};
  const pdfDocument = primaryView._pdfDocument;
  const totalPages: number = pdfDocument?.pagesCount
    ?? pdfDocument?.numPages
    ?? (Array.isArray(pdfPages) ? pdfPages.length : Object.keys(pdfPages).length);

  const pagesToSearch = pageIndex !== undefined
    ? [pageIndex]
    : Array.from({ length: totalPages }, (_, i) => i);

  for (const pi of pagesToSearch) {
    let chars = pdfPages[pi]?.chars;
    if ((!chars || chars.length === 0) && pdfDocument?.getPageData) {
      try {
        const loaded = await pdfDocument.getPageData({ pageIndex: pi });
        chars = loaded?.chars;
      } catch { /* skip */ }
    }
    if (!chars || chars.length === 0) continue;

    const match = findQuoteInChars(chars, quote);
    if (match) {
      return { pageIndex: pi, rects: getRangeRects(chars, match.offsetStart, match.offsetEnd) };
    }
  }
  return null;
}

async function waitForReaderReady(reader: any, timeoutMs = 15000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const ir = reader._internalReader;
    const pv = ir?._primaryView ?? ir?._state;
    if (pv?._pdfDocument) return;
    const pages = pv?._pdfPages;
    if (pages && (Array.isArray(pages) ? pages.some((p: any) => p?.chars?.length) : Object.values(pages).some((p: any) => (p as any)?.chars?.length))) return;
    await new Promise(r => setTimeout(r, 250));
  }
}

function closeBackgroundReader(reader: any): void {
  try {
    const tabID = reader._tabID ?? reader.tabID ?? (reader as any)._options?.tabID;
    if (!tabID) return;
    const windows = (Zotero as any).getMainWindows?.() ?? [];
    for (const win of windows) {
      if (win?.Zotero_Tabs?.close) {
        win.Zotero_Tabs.close(tabID);
        return;
      }
    }
  } catch { /* ignore */ }
}

async function resolveQuotePosition(
  attachment: Zotero.Item,
  quote: string,
  pageIndex?: number,
): Promise<{ pageIndex: number; rects: number[][] }> {
  // Path 1: use already-open reader (no side effects)
  const existingReader = findOpenReaderForAttachment(attachment);
  if (existingReader) {
    try {
      const result = await searchReaderForQuote(existingReader, quote, pageIndex);
      if (result) return result;
    } catch { /* fall through */ }
  }

  // Path 2: open reader in background (user won't see it), search, then close
  if (!existingReader) {
    let bgReader: any = null;
    try {
      bgReader = await (Zotero as any).Reader.open(
        attachment.id, null, { openInBackground: true },
      );
      if (bgReader) {
        await waitForReaderReady(bgReader);
        const result = await searchReaderForQuote(bgReader, quote, pageIndex);
        if (result) return result;
      }
    } catch { /* background open failed */ }
    finally {
      if (bgReader) closeBackgroundReader(bgReader);
    }
  }

  const scope = pageIndex !== undefined ? `page ${pageIndex}` : "any page";
  throw {
    code: -32602,
    message: `Quote not found on ${scope}: ${quote.slice(0, 80)}${quote.length > 80 ? "..." : ""}`,
  };
}


function findOpenReaderForAttachment(attachment: Zotero.Item): any | null {
  for (const reader of getAllReaders()) {
    if (reader.itemID === attachment.id) return reader;
  }
  return null;
}

function getAllReaders(): any[] {
  try {
    const windows = (Zotero as any).getMainWindows?.() ?? [];
    const readers: any[] = [];
    for (const win of windows) {
      const tabs = win?.Zotero_Tabs?._tabs ?? [];
      for (const tab of tabs) {
        if (tab.id) {
          const reader = (Zotero as any).Reader.getByTabID(tab.id);
          if (reader) readers.push(reader);
        }
      }
    }
    return readers;
  } catch {
    return [];
  }
}

export const annotationsHandlers = {
  async list(params: { parentKey: number | string }) {
    const attachment = await resolveAnnotationParent(params.parentKey);
    const annotationRefs: any[] = (attachment as any).getAnnotations?.() ?? [];
    if (annotationRefs.length === 0) return { items: [], total: 0, attachmentKey: attachment.key };
    const anns = await resolveAnnotationRefs(attachment.libraryID, annotationRefs);
    const items = anns.map((a: any) => {
      const data = serializeItem(a);
      data.annotationType = a.annotationType;
      data.annotationText = a.annotationText || "";
      data.annotationComment = a.annotationComment || "";
      data.annotationColor = a.annotationColor || "";
      data.annotationPosition = a.annotationPosition ? JSON.parse(a.annotationPosition) : null;
      return data;
    });
    return { items, total: items.length, attachmentKey: attachment.key };
  },

  async create(params: {
    parentKey: number | string;
    type: string;
    text?: string;
    comment?: string;
    color?: string;
    position?: any;
    quote?: string;
    pageIndex?: number;
    sortIndex?: unknown;
  }) {
    const parent = await resolveAnnotationParent(params.parentKey);
    const validation = validateAnnotationParams({
      type: params.type as any,
      text: params.text,
      color: params.color,
      comment: params.comment,
      position: params.position,
      quote: params.quote,
      sortIndex: params.sortIndex,
    });
    if (!validation.ok) throw { code: -32602, message: validation.message };

    let position = params.position;
    if (params.quote && !position) {
      position = await resolveQuotePosition(parent, params.quote, params.pageIndex);
    }

    const ann = new Zotero.Item("annotation");
    ann.libraryID = parent.libraryID;
    ann.parentID = parent.id;
    (ann as any).annotationType = params.type;
    const annotationText = params.text || params.quote;
    if (annotationText) (ann as any).annotationText = annotationText;
    if (params.comment) (ann as any).annotationComment = params.comment;
    if (params.color) (ann as any).annotationColor = params.color;
    (ann as any).annotationPosition = JSON.stringify(position);
    (ann as any).annotationSortIndex = normalizeAnnotationSortIndex(
      position,
      params.sortIndex,
    );
    await ann.saveTx();
    return { ok: true, key: ann.key, attachmentKey: parent.key };
  },

  async delete(params: { key: number | string }) {
    const item = await requireItem(params.key);
    await item.eraseTx();
    return { ok: true, key: item.key };
  },
};

async function resolveAnnotationRefs(libraryID: number, refs: any[]): Promise<any[]> {
  if (refs.every((ref) => typeof ref === "number" && Number.isFinite(ref))) {
    return (await Zotero.Items.getAsync(refs)) as any[];
  }

  const anns: any[] = [];
  for (const ref of refs) {
    if (ref && typeof ref === "object" && typeof ref.isAnnotation === "function") {
      anns.push(ref);
    } else if (ref && typeof ref === "object" && Number.isFinite(ref.id)) {
      const ann = await Zotero.Items.getAsync(ref.id);
      if (ann) anns.push(ann);
    } else if (ref && typeof ref === "object" && typeof ref.key === "string") {
      const ann = await Zotero.Items.getByLibraryAndKeyAsync(libraryID, ref.key);
      if (ann) anns.push(ann);
    } else if (typeof ref === "string") {
      const ann = await Zotero.Items.getByLibraryAndKeyAsync(libraryID, ref);
      if (ann) anns.push(ann);
    }
  }
  return anns;
}

function normalizeAnnotationSortIndex(position: any, sortIndex: unknown): string {
  if (typeof sortIndex === "string" && /^\d{5}\|\d{6}\|\d{5}$/.test(sortIndex.trim())) {
    return sortIndex.trim();
  }

  const page = padInt(position.pageIndex, 5);
  const yOffset = sortIndex === undefined
    ? firstRectY(position)
    : Number(String(sortIndex).trim());
  return `${page}|000000|${padInt(yOffset, 5)}`;
}

function firstRectY(position: any): number {
  const firstRect = Array.isArray(position?.rects) ? position.rects[0] : undefined;
  const y = Array.isArray(firstRect) ? firstRect[1] : 0;
  return typeof y === "number" && Number.isFinite(y) ? y : 0;
}

function padInt(value: unknown, width: number): string {
  const normalized = Math.max(0, Math.floor(Number(value) || 0));
  return String(normalized).padStart(width, "0").slice(-width);
}

registerHandlers("annotations", annotationsHandlers);
