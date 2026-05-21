// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/annotations.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem, requirePdfAttachment } from "../utils/guards";
import { validateAnnotationParams } from "../utils/annotation";
import { findQuoteInChars, getRangeRects, getTextFromChars } from "../utils/pdf-locate";

interface QuotePosition {
  pageIndex: number;
  rects: number[][];
  charOffset: number;
}

function searchReaderChars(
  reader: any,
  quote: string,
  pageIndex?: number,
): QuotePosition | null {
  const primaryView = reader?._internalReader?._primaryView
    ?? reader?._internalReader?._state
    ?? null;
  const pdfPages = primaryView?._pdfPages;
  if (!pdfPages) return null;

  const totalPages = Array.isArray(pdfPages)
    ? pdfPages.length
    : Object.keys(pdfPages).length;
  const pagesToSearch = pageIndex !== undefined
    ? [pageIndex]
    : Array.from({ length: totalPages }, (_, i) => i);

  for (const pi of pagesToSearch) {
    const chars = pdfPages[pi]?.chars;
    if (!chars || chars.length === 0) continue;
    const match = findQuoteInChars(chars, quote);
    if (match) {
      return {
        pageIndex: pi,
        rects: getRangeRects(chars, match.offsetStart, match.offsetEnd),
        charOffset: match.offsetStart,
      };
    }
  }
  return null;
}

async function resolveQuotePosition(
  attachment: Zotero.Item,
  quote: string,
  pageIndex?: number,
): Promise<QuotePosition[]> {
  // Fast path: if Reader is open, use its cached chars
  const existingReader = findOpenReaderForAttachment(attachment);
  if (existingReader) {
    const cached = searchReaderChars(existingReader, quote, pageIndex);
    if (cached) return [cached];
  }

  // Primary path: extract all pages in parallel, cache, then search
  // @ts-expect-error — lazy require for test mockability; esbuild resolves at bundle time
  const pdfExtract: typeof import("../utils/pdf-extract") = require("../utils/pdf-extract");
  const allChars = await pdfExtract.extractAllPagesChars(attachment);

  const pages = pageIndex !== undefined ? [pageIndex] : allChars.map((_, i) => i);
  for (const pi of pages) {
    const chars = allChars[pi];
    if (!chars || chars.length === 0) continue;
    const match = findQuoteInChars(chars, quote);
    if (match) {
      return [{
        pageIndex: pi,
        rects: getRangeRects(chars, match.offsetStart, match.offsetEnd),
        charOffset: match.offsetStart,
      }];
    }
  }

  // Cross-page fallback: split the quote at every position and search
  // each half independently on adjacent pages. No concatenation needed,
  // so footnotes / page numbers / running headers don't interfere.
  if (pageIndex === undefined) {
    const MIN_HALF = 2;
    for (let pi = 0; pi < allChars.length - 1; pi++) {
      const charsA = allChars[pi];
      const charsB = allChars[pi + 1];
      if (!charsA?.length || !charsB?.length) continue;

      for (let k = MIN_HALF; k <= quote.length - MIN_HALF; k++) {
        const left = quote.slice(0, k);
        const right = quote.slice(k);
        const matchA = findQuoteInChars(charsA, left);
        const matchB = findQuoteInChars(charsB, right);
        if (matchA && matchB) {
          return [
            { pageIndex: pi, rects: getRangeRects(charsA, matchA.offsetStart, matchA.offsetEnd), charOffset: matchA.offsetStart },
            { pageIndex: pi + 1, rects: getRangeRects(charsB, matchB.offsetStart, matchB.offsetEnd), charOffset: matchB.offsetStart },
          ];
        }
      }
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

interface AnnotationInput {
  type?: string;
  text?: string;
  comment?: string;
  color?: string;
  position?: any;
  quote?: string;
  pageIndex?: number;
  sortIndex?: unknown;
}

async function saveOneAnnotation(
  parent: Zotero.Item,
  ann: AnnotationInput,
): Promise<string[]> {
  const type = ann.type ?? "highlight";
  let position = ann.position;
  let charOffset: number | undefined;
  let crossPagePositions: QuotePosition[] | undefined;

  if (ann.quote && !position) {
    const resolved = await resolveQuotePosition(parent, ann.quote, ann.pageIndex);
    if (resolved.length === 1) {
      charOffset = resolved[0].charOffset;
      position = { pageIndex: resolved[0].pageIndex, rects: resolved[0].rects };
    } else {
      crossPagePositions = resolved;
    }
  }

  if (crossPagePositions) {
    const keys: string[] = [];
    for (const pos of crossPagePositions) {
      const item = new Zotero.Item("annotation");
      item.libraryID = parent.libraryID;
      item.parentID = parent.id;
      (item as any).annotationType = type;
      if (ann.quote) (item as any).annotationText = ann.quote;
      if (ann.comment) (item as any).annotationComment = ann.comment;
      if (ann.color) (item as any).annotationColor = ann.color;
      const annPos = { pageIndex: pos.pageIndex, rects: pos.rects };
      (item as any).annotationPosition = JSON.stringify(annPos);
      (item as any).annotationSortIndex = normalizeAnnotationSortIndex(annPos, ann.sortIndex, pos.charOffset);
      await item.saveTx();
      keys.push(item.key);
    }
    return keys;
  }

  const item = new Zotero.Item("annotation");
  item.libraryID = parent.libraryID;
  item.parentID = parent.id;
  (item as any).annotationType = type;
  const annotationText = ann.text || ann.quote;
  if (annotationText) (item as any).annotationText = annotationText;
  if (ann.comment) (item as any).annotationComment = ann.comment;
  if (ann.color) (item as any).annotationColor = ann.color;
  (item as any).annotationPosition = JSON.stringify(position);
  (item as any).annotationSortIndex = normalizeAnnotationSortIndex(position, ann.sortIndex, charOffset);
  await item.saveTx();
  return [item.key];
}

export const annotationsHandlers = {
  async list(params: { parentKey: number | string; attachmentKey?: string; context?: number }) {
    const { attachment, otherAttachments } = await requirePdfAttachment(params.parentKey, params.attachmentKey);
    const annotationRefs: any[] = (attachment as any).getAnnotations?.() ?? [];
    if (annotationRefs.length === 0) {
      const result: Record<string, any> = { items: [], total: 0, attachmentKey: attachment.key };
      if (otherAttachments.length > 0) result.otherAttachments = otherAttachments;
      return result;
    }
    const anns = await resolveAnnotationRefs(attachment.libraryID, annotationRefs);

    let allChars: import("../utils/pdf-locate").CharData[][] | null = null;
    const contextChars = typeof params.context === "number" && params.context > 0 ? params.context : 0;
    if (contextChars > 0) {
      // @ts-expect-error — lazy require for test mockability
      const pdfExtract: typeof import("../utils/pdf-extract") = require("../utils/pdf-extract");
      allChars = await pdfExtract.extractAllPagesChars(attachment);
    }

    const items = anns.map((a: any) => {
      const data = serializeItem(a);
      data.annotationType = a.annotationType;
      data.annotationText = a.annotationText || "";
      data.annotationComment = a.annotationComment || "";
      data.annotationColor = a.annotationColor || "";
      const pos = a.annotationPosition ? JSON.parse(a.annotationPosition) : null;
      data.annotationPosition = pos;

      if (allChars && pos && typeof pos.pageIndex === "number") {
        const pageChars = allChars[pos.pageIndex];
        if (pageChars && pageChars.length > 0) {
          data.contextText = extractContextText(pageChars, a.annotationText || "", contextChars);
        }
      }
      return data;
    });
    const result: Record<string, any> = { items, total: items.length, attachmentKey: attachment.key };
    if (otherAttachments.length > 0) result.otherAttachments = otherAttachments;
    return result;
  },

  async create(params: {
    parentKey: number | string;
    attachmentKey?: string;
    type: string;
    text?: string;
    comment?: string;
    color?: string;
    position?: any;
    quote?: string;
    pageIndex?: number;
    sortIndex?: unknown;
  }) {
    const { attachment: parent, otherAttachments } = await requirePdfAttachment(params.parentKey, params.attachmentKey);
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

    const keys = await saveOneAnnotation(parent, params);
    const result: Record<string, any> = { ok: true, key: keys[0], attachmentKey: parent.key };
    if (keys.length > 1) result.keys = keys;
    if (otherAttachments.length > 0) result.otherAttachments = otherAttachments;
    return result;
  },

  async locate(params: {
    parentKey: number | string;
    attachmentKey?: string;
    quote: string;
    pageIndex?: number;
  }) {
    const { attachment } = await requirePdfAttachment(params.parentKey, params.attachmentKey);
    const positions = await resolveQuotePosition(attachment, params.quote, params.pageIndex);
    return {
      found: true,
      attachmentKey: attachment.key,
      positions: positions.map(p => ({
        pageIndex: p.pageIndex,
        rects: p.rects,
        charOffset: p.charOffset,
      })),
    };
  },

  async createBatch(params: {
    parentKey: number | string;
    attachmentKey?: string;
    annotations: Array<{
      type?: string;
      text?: string;
      comment?: string;
      color?: string;
      position?: any;
      quote?: string;
      pageIndex?: number;
      sortIndex?: unknown;
    }>;
  }) {
    if (!Array.isArray(params.annotations) || params.annotations.length === 0) {
      throw { code: -32602, message: "annotations array is required and must not be empty" };
    }
    const { attachment: parent, otherAttachments } = await requirePdfAttachment(params.parentKey, params.attachmentKey);

    // Pre-warm the char cache by extracting all pages once
    // @ts-expect-error — lazy require for test mockability
    const pdfExtract: typeof import("../utils/pdf-extract") = require("../utils/pdf-extract");
    await pdfExtract.extractAllPagesChars(parent);

    const results: Array<{ ok: true; key: string; index: number } | { ok: false; index: number; error: string }> = [];

    for (let i = 0; i < params.annotations.length; i++) {
      const ann = params.annotations[i];
      try {
        const validation = validateAnnotationParams({
          type: (ann.type ?? "highlight") as any,
          text: ann.text,
          color: ann.color,
          comment: ann.comment,
          position: ann.position,
          quote: ann.quote,
          sortIndex: ann.sortIndex,
        });
        if (!validation.ok) {
          results.push({ ok: false, index: i, error: validation.message });
          continue;
        }
        const keys = await saveOneAnnotation(parent, ann);
        results.push({ ok: true, key: keys[0], index: i });
      } catch (e: any) {
        const msg = e?.message ?? String(e);
        results.push({ ok: false, index: i, error: msg });
      }
    }

    const succeeded = results.filter(r => r.ok).length;
    const failed = results.filter(r => !r.ok).length;
    const result: Record<string, any> = { results, succeeded, failed, total: params.annotations.length, attachmentKey: parent.key };
    if (otherAttachments.length > 0) result.otherAttachments = otherAttachments;
    return result;
  },

  async delete(params: { key: number | string }) {
    const item = await requireItem(params.key);
    await item.eraseTx();
    return { ok: true, key: item.key };
  },
};

function extractContextText(
  chars: import("../utils/pdf-locate").CharData[],
  annotationText: string,
  contextChars: number,
): { before: string; highlight: string; after: string } | null {
  if (!annotationText) return null;
  const match = findQuoteInChars(chars, annotationText);
  if (!match) return null;

  const fullText = getTextFromChars(chars);

  // Build char-index → text-position map
  const charToText: number[] = [];
  let textPos = 0;
  for (let i = 0; i < chars.length; i++) {
    if (chars[i].ignorable) {
      charToText.push(-1);
      continue;
    }
    charToText.push(textPos);
    textPos += 1;
    if (chars[i].spaceAfter || chars[i].lineBreakAfter) textPos += 1;
  }

  const textStart = charToText[match.offsetStart] ?? 0;
  const textEnd = (charToText[match.offsetEnd] ?? textStart) + 1;
  const beforeStart = Math.max(0, textStart - contextChars);
  const afterEnd = Math.min(fullText.length, textEnd + contextChars);

  return {
    before: fullText.slice(beforeStart, textStart),
    highlight: fullText.slice(textStart, textEnd),
    after: fullText.slice(textEnd, afterEnd),
  };
}

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

function normalizeAnnotationSortIndex(position: any, sortIndex: unknown, charOffset?: number): string {
  if (typeof sortIndex === "string" && /^\d{5}\|\d{6}\|\d{5}$/.test(sortIndex.trim())) {
    return sortIndex.trim();
  }

  const page = padInt(position.pageIndex, 5);
  const charOff = padInt(charOffset ?? 0, 6);
  const yOffset = sortIndex === undefined
    ? firstRectY(position)
    : Number(String(sortIndex).trim());
  return `${page}|${charOff}|${padInt(yOffset, 5)}`;
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
