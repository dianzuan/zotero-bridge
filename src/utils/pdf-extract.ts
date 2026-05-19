// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import type { CharData } from "./pdf-locate";

type ExtractFn = (attachment: Zotero.Item, pageIndex: number) => Promise<CharData[]>;
type CountFn = (attachment: Zotero.Item) => Promise<number>;

let pdfJs: any = null;
const docCache = new Map<number, any>();

async function ensurePdfJs(): Promise<any> {
  if (pdfJs) return pdfJs;
  pdfJs = ChromeUtils.importESModule(
    "resource://zotero/reader/pdf/build/pdf.mjs",
  );
  if (!pdfJs.GlobalWorkerOptions.workerSrc) {
    pdfJs.GlobalWorkerOptions.workerSrc =
      "resource://zotero/reader/pdf/build/pdf.worker.mjs";
  }
  return pdfJs;
}

async function loadDocument(attachment: Zotero.Item): Promise<any> {
  const cached = docCache.get(attachment.id);
  if (cached) return cached;

  const path = await attachment.getFilePathAsync();
  if (!path) throw { code: -32602, message: "Attachment has no file" };

  const buf = await (globalThis as any).IOUtils.read(path);
  const { getDocument } = await ensurePdfJs();
  const doc = await getDocument({
    data: new Uint8Array(buf),
    cMapUrl: "resource://zotero/reader/pdf/web/cmaps/",
    cMapPacked: true,
    standardFontDataUrl: "resource://zotero/reader/pdf/web/standard_fonts/",
  }).promise;

  docCache.set(attachment.id, doc);
  return doc;
}

async function defaultExtractPageChars(
  attachment: Zotero.Item,
  pageIndex: number,
): Promise<CharData[]> {
  const doc = await loadDocument(attachment);
  const pageData = await doc.getPageData({ pageIndex });
  return pageData?.chars ?? [];
}

async function defaultGetPageCount(attachment: Zotero.Item): Promise<number> {
  const doc = await loadDocument(attachment);
  return doc.numPages;
}

let extractImpl: ExtractFn = defaultExtractPageChars;
let countImpl: CountFn = defaultGetPageCount;

export async function extractPageChars(
  attachment: Zotero.Item,
  pageIndex: number,
): Promise<CharData[]> {
  return extractImpl(attachment, pageIndex);
}

export async function getPageCount(
  attachment: Zotero.Item,
): Promise<number> {
  return countImpl(attachment);
}

export function shutdown(): void {
  for (const doc of docCache.values()) {
    try { doc.destroy(); } catch { /* ignore */ }
  }
  docCache.clear();
  pdfJs = null;
}

export const __test__ = {
  setExtractImpl(fn: ExtractFn) { extractImpl = fn; },
  setCountImpl(fn: CountFn) { countImpl = fn; },
  reset() {
    extractImpl = defaultExtractPageChars;
    countImpl = defaultGetPageCount;
  },
};
