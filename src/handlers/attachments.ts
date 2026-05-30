// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/attachments.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem, requirePdfAttachment } from "../utils/guards";
import { sanitizePath } from "../utils/safe-path";
import { extractAttachmentFullText } from "../utils/fulltext";

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
    return extractAttachmentFullText(item);
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

  async rename(params: { key: number | string; name?: string; fromParent?: boolean }) {
    const item = await requireItem(params.key);
    if (!item.isAttachment()) {
      throw { code: -32602, message: `Item ${params.key} is not an attachment` };
    }

    const currentPath = await (item as any).getFilePathAsync();
    if (!currentPath) throw { code: -32603, message: `No file path for ${params.key}` };
    const leaf: string = currentPath.replace(/^.*[/\\]/, "");
    const dot = leaf.lastIndexOf(".");
    const ext = dot > 0 ? leaf.slice(dot + 1) : "";

    let baseName: string;
    if (params.fromParent) {
      const parentID = (item as any).parentItemID;
      if (!parentID) throw { code: -32602, message: `Attachment ${params.key} has no parent` };
      const parent = await Zotero.Items.getAsync(parentID);
      baseName = (Zotero.Attachments as any).getFileBaseNameFromItem(parent);
    } else if (params.name) {
      baseName = params.name;
    } else {
      baseName = (item as any).getField("title") || leaf;
    }

    const newName = ext ? `${baseName}.${ext}` : baseName;
    const renamed = await (item as any).renameAttachmentFile(newName, false, true);
    if (!renamed) throw { code: -32603, message: `Rename failed for ${params.key}` };

    const newPath = await (item as any).getFilePathAsync();
    return { ok: true, key: item.key, filename: newName, path: newPath };
  },

  async exportPDF(params: {
    key: number | string;
    path: string;
    attachmentKey?: string;
    transfer?: boolean;
    password?: string;
  }) {
    const { attachment } = await requirePdfAttachment(params.key, params.attachmentKey);
    const outputPath = sanitizePath(params.path);

    try {
      const count: number = await (Zotero as any).PDFWorker.export(
        attachment.id,
        outputPath,
        true,
        params.password ?? undefined,
        params.transfer ?? false,
      );
      return {
        ok: true,
        key: attachment.key,
        path: outputPath,
        annotationsWritten: count,
      };
    } catch (e: any) {
      if (e?.name === "PasswordException") {
        throw { code: -32602, message: "PDF is password-protected. Provide the 'password' parameter." };
      }
      throw { code: -32603, message: `PDF export failed: ${e?.message ?? e}` };
    }
  },

  async findPDF(params: { parentKey: number | string }) {
    const parent = await requireItem(params.parentKey);
    const attachment = await Zotero.Attachments.addAvailableFile(parent);
    return { attachment: attachment ? serializeItem(attachment) : null };
  },
};

registerHandlers("attachments", attachmentsHandlers);
