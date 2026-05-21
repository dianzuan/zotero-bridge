// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/attachments.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem, requirePdfAttachment } from "../utils/guards";
import { sanitizePath } from "../utils/safe-path";

async function serializeAttachment(item: Zotero.Item): Promise<Record<string, any>> {
  const data = serializeItem(item);
  const attachment = item as any;
  data.contentType = attachment.attachmentContentType || null;
  data.linkMode = attachment.attachmentLinkMode ?? null;
  if (typeof attachment.getFilePathAsync === "function") {
    try {
      data.path = await attachment.getFilePathAsync();
    } catch {
      data.path = null;
    }
  } else {
    data.path = null;
  }
  return data;
}

export const attachmentsHandlers = {
  async get(params: { key: number | string }) {
    const item = await requireItem(params.key);
    if (!item.isAttachment()) throw { code: -32602, message: `Item ${params.key} is not an attachment` };
    return serializeAttachment(item);
  },

  async list(params: { parentKey: number | string }) {
    const parent = await requireItem(params.parentKey);
    const attIDs = parent.getAttachments();
    if (attIDs.length === 0) return { items: [], total: 0 };
    const atts = await Zotero.Items.getAsync(attIDs);
    const items = await Promise.all(atts.map(serializeAttachment));
    return { items, total: items.length };
  },

  async getFulltext(params: { key: number | string }) {
    const item = await requireItem(params.key);
    if (!item.isAttachment()) throw { code: -32602, message: `Not an attachment: ${params.key}` };

    const cacheFile = Zotero.Fulltext.getItemCacheFile(item);
    let content = "";
    try {
      content = (await Zotero.File.getContentsAsync(cacheFile.path) as string) ?? "";
    } catch {
      content = "";
    }

    // Fallback: cache empty → live extraction via PDFWorker
    if (!content && item.attachmentContentType === "application/pdf") {
      try {
        const result = await (Zotero as any).PDFWorker.getFullText(item.id);
        content = (result?.text ?? "").replace(/\f/g, "\n");
      } catch { /* PDFWorker unavailable */ }
    }

    const rows = (
      (await Zotero.DB.queryAsync(
        "SELECT indexedChars, totalChars FROM fulltextItems WHERE itemID=?",
        [item.id],
      )) as Array<{ indexedChars: number; totalChars: number }>
    ) ?? [];
    const meta = rows[0] ?? { indexedChars: 0, totalChars: 0 };

    return {
      key: item.key,
      content: content ?? "",
      indexedChars: content ? content.length : (meta.indexedChars ?? 0),
      totalChars: content ? content.length : (meta.totalChars ?? 0),
    };
  },

  async getFulltextByPage(params: { key: number | string }) {
    const { attachment } = await requirePdfAttachment(params.key);

    const result = await (Zotero as any).PDFWorker.getFullText(attachment.id);
    const fullText: string = result?.text ?? "";
    const pages = fullText.split("\f");
    return {
      key: attachment.key,
      totalPages: pages.length,
      pages: pages.map((text: string, i: number) => ({
        page: i,
        text: text.trim(),
      })),
    };
  },

  async add(params: {
    parentKey: number | string;
    path: string;
    title?: string;
    renameFromParent?: boolean;
  }) {
    const parent = await requireItem(params.parentKey);
    const cleanPath = sanitizePath(params.path);
    const file = Zotero.File.pathToFile(cleanPath);
    if (!file.exists()) throw { code: -32602, message: `File not found: ${params.path}` };
    const attachment = await Zotero.Attachments.importFromFile({
      file,
      parentItemID: parent.id,
      title: params.title,
    });

    // Honor Zotero's "Rename Files from Parent Metadata" template
    // (attachmentRenameFormatString pref) so attachments don't end up with
    // upstream-source names like export-bnhuaoo2.pdf in the user's
    // storage/. Keep params.title untouched — that's the UI label, separate
    // from the on-disk filename.
    if (params.renameFromParent !== false) {
      try {
        const baseName: string = (Zotero.Attachments as any).getFileBaseNameFromItem(parent);
        if (baseName) {
          const leaf = file.leafName ?? "";
          const dot = leaf.lastIndexOf(".");
          const ext = dot > 0 ? leaf.slice(dot + 1) : "";
          const newName = ext ? `${baseName}.${ext}` : baseName;
          await (attachment as any).renameAttachmentFile(newName, false, true);
        }
      } catch (e) {
        Zotero.debug(`zotron attachments.add: rename-from-parent failed: ${e}`);
      }
    }

    return serializeItem(attachment);
  },

  async addByURL(params: { parentKey: number | string; url: string; title?: string }) {
    const parent = await requireItem(params.parentKey);
    const attachment = await Zotero.Attachments.importFromURL({
      url: params.url,
      parentItemID: parent.id,
      title: params.title,
    });
    return serializeItem(attachment);
  },

  async getPath(params: { key: number | string }) {
    const item = await requireItem(params.key);
    if (!item.isAttachment()) {
      throw { code: -32602, message: `Attachment ${params.key} not found` };
    }
    const path = await item.getFilePathAsync();
    return { key: item.key, path: path || null };
  },

  async delete(params: { key: number | string }) {
    const item = await requireItem(params.key);
    if (!item.isAttachment()) {
      throw { code: -32602, message: `Attachment ${params.key} not found` };
    }
    await item.eraseTx();
    return { ok: true, key: item.key };
  },

  async findPDF(params: { parentKey: number | string }) {
    const parent = await requireItem(params.parentKey);
    const attachment = await Zotero.Attachments.addAvailableFile(parent);
    return { attachment: attachment ? serializeItem(attachment) : null };
  },
};

registerHandlers("attachments", attachmentsHandlers);
