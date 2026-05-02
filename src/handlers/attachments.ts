// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/attachments.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem } from "../utils/guards";

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
  async get(params: { id: number | string }) {
    const item = await requireItem(params.id);
    if (!item.isAttachment()) throw { code: -32602, message: `Item ${params.id} is not an attachment` };
    return serializeAttachment(item);
  },

  async list(params: { parentId: number | string }) {
    const parent = await requireItem(params.parentId);
    const attIDs = parent.getAttachments();
    if (attIDs.length === 0) return [];
    const atts = await Zotero.Items.getAsync(attIDs);
    return Promise.all(atts.map(serializeAttachment));
  },

  async getFulltext(params: { id: number | string }) {
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
      key: item.key,
      content: content ?? "",
      indexedChars: meta.indexedChars ?? 0,
      totalChars: meta.totalChars ?? 0,
    };
  },

  async add(params: {
    parentId: number | string;
    path: string;
    title?: string;
    renameFromParent?: boolean;
  }) {
    const parent = await requireItem(params.parentId);
    const file = Zotero.File.pathToFile(params.path);
    if (!file.exists()) throw { code: -32602, message: `File not found: ${params.path}` };
    const attachment = await Zotero.Attachments.importFromFile({
      file,
      parentItemID: parent.id,
      title: params.title,
    });

    // Honor Zotero's "Rename Files from Parent Metadata" template
    // (attachmentRenameFormatString pref) so attachments don't end up with
    // upstream-source names like cnki-export-bnhuaoo2.pdf in the user's
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

  async addByURL(params: { parentId: number | string; url: string; title?: string }) {
    const parent = await requireItem(params.parentId);
    const attachment = await Zotero.Attachments.importFromURL({
      url: params.url,
      parentItemID: parent.id,
      title: params.title,
    });
    return serializeItem(attachment);
  },

  async getPath(params: { id: number | string }) {
    const item = await requireItem(params.id);
    if (!item.isAttachment()) {
      throw { code: -32602, message: `Attachment ${params.id} not found` };
    }
    const path = await item.getFilePathAsync();
    return { key: item.key, path: path || null };
  },

  async delete(params: { id: number | string }) {
    const item = await requireItem(params.id);
    if (!item.isAttachment()) {
      throw { code: -32602, message: `Attachment ${params.id} not found` };
    }
    await item.eraseTx();
    return { ok: true, key: item.key };
  },

  async findPDF(params: { parentId: number | string }) {
    const parent = await requireItem(params.parentId);
    const attachment = await Zotero.Attachments.addAvailableFile(parent);
    return { attachment: attachment ? serializeItem(attachment) : null };
  },
};

registerHandlers("attachments", attachmentsHandlers);
