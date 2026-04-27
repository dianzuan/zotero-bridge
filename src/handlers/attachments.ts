// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/attachments.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem } from "../utils/guards";

export const attachmentsHandlers = {
  async list(params: { parentId: number }) {
    const parent = await requireItem(params.parentId);
    const attIDs = parent.getAttachments();
    if (attIDs.length === 0) return [];
    const atts = await Zotero.Items.getAsync(attIDs);
    return atts.map(serializeItem);
  },

  async getFulltext(params: { id: number }) {
    const item = await requireItem(params.id);
    if (!item.isAttachment()) throw { code: -32602, message: `Not an attachment: ${params.id}` };

    // Zotero 8 has no Zotero.Fulltext.getItemContent — read the cache file and
    // pull chars from the fulltextItems SQL row (anchored at fulltext.js#L672+L692).
    const cacheFile = Zotero.Fulltext.getItemCacheFile(item);
    let content = "";
    try {
      content = (await Zotero.File.getContentsAsync(cacheFile.path) as string) ?? "";
    } catch {
      content = "";  // un-indexed → no cache file
    }

    const rows = (
      (await Zotero.DB.queryAsync(
        "SELECT indexedChars, totalChars FROM fulltextItems WHERE itemID=?",
        [item.id],
      )) as Array<{ indexedChars: number; totalChars: number }>
    ) ?? [];
    const meta = rows[0] ?? { indexedChars: 0, totalChars: 0 };

    return {
      id: item.id,
      content: content ?? "",
      indexedChars: meta.indexedChars ?? 0,
      totalChars: meta.totalChars ?? 0,
    };
  },

  async add(params: { parentId: number; path: string; title?: string }) {
    const parent = await requireItem(params.parentId);
    const file = Zotero.File.pathToFile(params.path);
    if (!file.exists()) throw { code: -32602, message: `File not found: ${params.path}` };
    const attachment = await Zotero.Attachments.importFromFile({
      file,
      parentItemID: params.parentId,
      title: params.title,
    });
    return serializeItem(attachment);
  },

  async addByURL(params: { parentId: number; url: string; title?: string }) {
    const parent = await requireItem(params.parentId);
    const attachment = await Zotero.Attachments.importFromURL({
      url: params.url,
      parentItemID: params.parentId,
      title: params.title,
    });
    return serializeItem(attachment);
  },

  async getPath(params: { id: number }) {
    const item = await Zotero.Items.getAsync(params.id);
    if (!item || !item.isAttachment()) {
      throw { code: -32602, message: `Attachment ${params.id} not found` };
    }
    const path = await item.getFilePathAsync();
    return { id: params.id, path: path || null };
  },

  async delete(params: { id: number }) {
    const item = await Zotero.Items.getAsync(params.id);
    if (!item || !item.isAttachment()) {
      throw { code: -32602, message: `Attachment ${params.id} not found` };
    }
    await item.eraseTx();
    return { ok: true, id: params.id };
  },

  async findPDF(params: { parentId: number }) {
    const parent = await requireItem(params.parentId);
    const attachment = await Zotero.Attachments.addAvailableFile(parent);
    return { attachment: attachment ? serializeItem(attachment) : null };
  },
};

registerHandlers("attachments", attachmentsHandlers);
