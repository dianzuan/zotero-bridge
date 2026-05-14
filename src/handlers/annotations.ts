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

/**
 * Resolve a text quote to PDF coordinates using Zotero reader's per-char data.
 *
 * Access path: reader._internalReader._primaryView._pdfPages[pageIndex].chars
 * Each char has { c, rect, spaceAfter, lineBreakAfter, ignorable, inlineRect, rotation }
 * with precise glyph bounding boxes from font metrics.
 */
async function resolveQuotePosition(
  attachment: Zotero.Item,
  quote: string,
  pageIndex?: number,
): Promise<{ pageIndex: number; rects: number[][] }> {
  const reader = await getReaderForAttachment(attachment);

  // Navigate to Zotero reader's internal per-char page data
  const internalReader = (reader as any)._internalReader;
  if (!internalReader) {
    throw {
      code: -32603,
      message: "Reader missing _internalReader — Zotero reader structure may have changed",
    };
  }

  const primaryView = internalReader._primaryView ?? internalReader._state;
  if (!primaryView) {
    throw {
      code: -32603,
      message: "Reader missing _primaryView and _state — cannot access per-char page data",
    };
  }

  const pdfPages = primaryView._pdfPages;
  if (!pdfPages) {
    throw {
      code: -32603,
      message: "Reader missing _pdfPages — cannot access per-char page data",
    };
  }

  const pageCount = Array.isArray(pdfPages) ? pdfPages.length : Object.keys(pdfPages).length;
  const pagesToSearch = pageIndex !== undefined
    ? [pageIndex]
    : Array.from({ length: pageCount }, (_, i) => i);

  for (const pi of pagesToSearch) {
    const pageData = pdfPages[pi];
    if (!pageData) continue;

    const chars = pageData.chars;
    if (!chars || chars.length === 0) continue;

    const match = findQuoteInChars(chars, quote);
    if (match) {
      const rects = getRangeRects(chars, match.offsetStart, match.offsetEnd);
      return { pageIndex: pi, rects };
    }
  }

  const scope = pageIndex !== undefined ? `page ${pageIndex}` : "any page";
  throw {
    code: -32602,
    message: `Quote not found on ${scope}: ${quote.slice(0, 80)}${quote.length > 80 ? "..." : ""}`,
  };
}

async function getReaderForAttachment(attachment: Zotero.Item): Promise<any> {
  // Try to find an already-open reader tab for this attachment
  const readers: any[] = (Zotero as any).Reader?.getByTabID
    ? getAllReaders()
    : [];
  for (const reader of readers) {
    if (reader.itemID === attachment.id) return reader;
  }
  // Open the attachment in a new reader tab
  const reader = await (Zotero as any).Reader.open(attachment.id);
  if (!reader) {
    throw { code: -32602, message: `Could not open PDF reader for attachment ${attachment.key}` };
  }
  return reader;
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
