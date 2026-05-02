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
    const cols = Zotero.Collections.getByLibrary(libraryID, false);
    return cols.map(serializeCollection);
  },

  async get(params: { id: number }) {
    const col = await requireCollection(params.id);
    return serializeCollection(col);
  },

  async getItems(params: { id: number; limit?: number; offset?: number }) {
    const col = await requireCollection(params.id);
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

  async getSubcollections(params: { id: number }) {
    const col = await requireCollection(params.id);
    const children = col.getChildCollections(false);
    return children.map(serializeCollection);
  },

  async tree() {
    const libraryID = Zotero.Libraries.userLibraryID;
    const cols = Zotero.Collections.getByLibrary(libraryID, true);
    return buildTree(cols);
  },

  async create(params: { name: string; parentId?: number }) {
    const col = new Zotero.Collection();
    (col as any).libraryID = Zotero.Libraries.userLibraryID;
    col.name = params.name;
    if (params.parentId) col.parentID = params.parentId;
    await col.saveTx();
    return serializeCollection(col);
  },

  async rename(params: { id: number; name: string }) {
    const col = await requireCollection(params.id);
    col.name = params.name;
    await col.saveTx();
    return serializeCollection(col);
  },

  async delete(params: { id: number }) {
    const col = await requireCollection(params.id);
    await col.eraseTx();
    return { ok: true, key: col.key };
  },

  async move(params: { id: number; newParentId: number | null }) {
    const col = await requireCollection(params.id);
    (col as any).parentID = params.newParentId || false;
    await col.saveTx();
    return serializeCollection(col);
  },

  async addItems(params: { id: number | string; itemIds: (number | string)[] }) {
    const col = await requireCollection(params.id);
    const items = await resolveItems(params.itemIds);
    const numericIds = items.map(i => i.id);
    await Zotero.DB.executeTransaction(async () => {
      await col.addItems(numericIds);
    });
    return { ok: true, key: col.key, count: numericIds.length };
  },

  async removeItems(params: { id: number | string; itemIds: (number | string)[] }) {
    const col = await requireCollection(params.id);
    const items = await resolveItems(params.itemIds);
    const numericIds = items.map(i => i.id);
    await Zotero.DB.executeTransaction(async () => {
      await col.removeItems(numericIds);
    });
    return { ok: true, key: col.key, count: numericIds.length };
  },

  async stats(params: { id: number }) {
    const col = await requireCollection(params.id);
    const items = col.getChildItems(false);
    const subcols = col.getChildCollections(false);
    return {
      key: col.key,
      name: col.name,
      items: items.filter((i: any) => !i.isNote() && !i.isAttachment()).length,
      attachments: items.filter((i: any) => i.isAttachment()).length,
      notes: items.filter((i: any) => i.isNote()).length,
      subcollections: subcols.length,
    };
  },
};

registerHandlers("collections", collectionsHandlers);
