// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/tags.ts
import { registerHandlers } from "../server";
import { requireItem } from "../utils/guards";

export const tagsHandlers = {
  async list(params: { limit?: number; offset?: number; libraryId?: number }) {
    const limit = params.limit || 200;
    const offset = params.offset ?? 0;
    const libraryID = params.libraryId ?? Zotero.Libraries.userLibraryID;
    const tags = await Zotero.Tags.getAll(libraryID);
    return tags.slice(offset, offset + limit).map((t: any) => ({
      tag: t.tag,
      type: t.type ?? 0,
    }));
  },

  async add(params: { itemId: number; tags: string[] }) {
    const item = await requireItem(params.itemId);
    for (const tag of params.tags) {
      item.addTag(tag);
    }
    await item.saveTx();
    return { ok: true, key: item.key };
  },

  async remove(params: { itemId: number; tags: string[] }) {
    const item = await requireItem(params.itemId);
    for (const tag of params.tags) {
      item.removeTag(tag);
    }
    await item.saveTx();
    return { ok: true, key: item.key };
  },

  async rename(params: { oldName: string; newName: string; libraryId?: number }) {
    const libraryID = params.libraryId ?? Zotero.Libraries.userLibraryID;
    await Zotero.Tags.rename(libraryID, params.oldName, params.newName);
    return { ok: true, tag: params.oldName, newName: params.newName };
  },

  async delete(params: { tag: string; libraryId?: number }) {
    const libraryID = params.libraryId ?? Zotero.Libraries.userLibraryID;
    const tagID = Zotero.Tags.getID(params.tag);
    if (!tagID) throw { code: -32602, message: `Tag not found: ${params.tag}` };
    await (Zotero.Tags as any).removeFromLibrary(libraryID, tagID);
    return { ok: true, tag: params.tag };
  },

  async batchUpdate(params: {
    operations: Array<{ itemId: number | string; add?: string[]; remove?: string[] }>;
  }) {
    let totalAdded = 0;
    let totalRemoved = 0;
    await Zotero.DB.executeTransaction(async () => {
      for (const op of params.operations) {
        const item = await requireItem(op.itemId);
        if (op.add) {
          for (const tag of op.add) { item.addTag(tag); totalAdded++; }
        }
        if (op.remove) {
          for (const tag of op.remove) { item.removeTag(tag); totalRemoved++; }
        }
        await item.save();
      }
    });
    return { ok: true, added: totalAdded, removed: totalRemoved };
  },
};

registerHandlers("tags", tagsHandlers);
