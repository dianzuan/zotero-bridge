// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/collections.ts
import { registerHandlers } from "../server";
import { serializeCollection, serializeItem } from "../utils/serialize";
import { requireCollection, resolveItems } from "../utils/guards";

function buildTree(collections: Zotero.Collection[]): any[] {
  const map = new Map<number, any>();
  for (const col of collections) {
    map.set(col.id, { ...serializeCollection(col), children: [] });
  }
  const roots: any[] = [];
  for (const col of collections) {
    const node = map.get(col.id)!;
    if (col.parentID && map.has(col.parentID)) {
      map.get(col.parentID)!.children.push(node);
    } else {
      roots.push(node);
    }
  }
  return roots;
}

export const collectionsHandlers = {
  async list() {
    const libraryID = Zotero.Libraries.userLibraryID;
    const cols = Zotero.Collections.getByLibrary(libraryID, true);
    const items = cols.map(serializeCollection);
    return { items, total: items.length };
  },

  async get(params: { key: number | string }) {
    const col = await requireCollection(params.key);
    return serializeCollection(col);
  },

  async getItems(params: { key: number | string; limit?: number; offset?: number }) {
    const col = await requireCollection(params.key);
    const allItems = col.getChildItems(false) || [];
    const offset = params.offset ?? 0;
    const sliced = params.limit !== undefined
      ? allItems.slice(offset, offset + params.limit)
      : allItems.slice(offset);
    const result: Record<string, any> = {
      items: sliced.map(serializeItem),
      total: allItems.length,
    };
    if (params.offset !== undefined) result.offset = params.offset;
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

  async getSubcollections(params: { key: number | string }) {
    const col = await requireCollection(params.key);
    const children = col.getChildCollections(false);
    return children.map(serializeCollection);
  },

  async tree() {
    const libraryID = Zotero.Libraries.userLibraryID;
    const cols = Zotero.Collections.getByLibrary(libraryID, true);
    return buildTree(cols);
  },

  async create(params: { name: string; parentKey?: number | string }) {
    const col = new Zotero.Collection();
    (col as any).libraryID = Zotero.Libraries.userLibraryID;
    col.name = params.name;
    if (params.parentKey) {
      const parent = await requireCollection(params.parentKey);
      col.parentID = parent.id;
    }
    await col.saveTx();
    return serializeCollection(col);
  },

  async rename(params: { key: number | string; name: string }) {
    const col = await requireCollection(params.key);
    col.name = params.name;
    await col.saveTx();
    return serializeCollection(col);
  },

  async delete(params: { key: number | string }) {
    const col = await requireCollection(params.key);
    await col.eraseTx();
    return { ok: true, key: col.key };
  },

  async move(params: { key: number | string; newParentKey: number | string | null }) {
    const col = await requireCollection(params.key);
    if (params.newParentKey) {
      const parent = await requireCollection(params.newParentKey);
      (col as any).parentID = parent.id;
    } else {
      (col as any).parentID = false;
    }
    await col.saveTx();
    return serializeCollection(col);
  },

  async addItems(params: { key: number | string; keys: (number | string)[] }) {
    const col = await requireCollection(params.key);
    const items = await resolveItems(params.keys);
    const numericIds = items.map(i => i.id);
    await Zotero.DB.executeTransaction(async () => {
      await col.addItems(numericIds);
    });
    return { ok: true, key: col.key, count: numericIds.length };
  },

  async removeItems(params: { key: number | string; keys: (number | string)[] }) {
    const col = await requireCollection(params.key);
    const items = await resolveItems(params.keys);
    const numericIds = items.map(i => i.id);
    await Zotero.DB.executeTransaction(async () => {
      await col.removeItems(numericIds);
    });
    return { ok: true, key: col.key, count: numericIds.length };
  },

  async stats(params: { key: number | string }) {
    const col = await requireCollection(params.key);
    const items = col.getChildItems(false);
    const subcols = col.getChildCollections(false);
    let childAttachmentCount = 0;
    for (const item of items) {
      if (item.isNote() || item.isAttachment()) continue;
      for (const attachmentID of item.getAttachments?.() || []) {
        const attachment = await Zotero.Items.getAsync(attachmentID);
        if (attachment?.isAttachment?.()) childAttachmentCount += 1;
      }
    }
    return {
      key: col.key,
      name: col.name,
      items: items.filter((i: any) => !i.isNote() && !i.isAttachment()).length,
      attachments: items.filter((i: any) => i.isAttachment()).length + childAttachmentCount,
      notes: items.filter((i: any) => i.isNote()).length,
      subcollections: subcols.length,
    };
  },
};

registerHandlers("collections", collectionsHandlers);
