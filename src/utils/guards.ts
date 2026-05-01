// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { rpcError, INVALID_PARAMS } from "./errors";

/**
 * Fetch an Item by numeric ID or 8-char alphanumeric key.
 * Accepts both `42` and `"YR5BUGHG"` so callers that have an item_key
 * from RAG hits or search results can pass it directly.
 */
export async function requireItem(idOrKey: number | string): Promise<Zotero.Item> {
  let item: Zotero.Item | null = null;

  if (typeof idOrKey === "number") {
    item = await Zotero.Items.getAsync(idOrKey);
  } else {
    const parsed = Number(idOrKey);
    if (Number.isFinite(parsed) && String(parsed) === idOrKey) {
      item = await Zotero.Items.getAsync(parsed);
    } else {
      const libraryID = Zotero.Libraries.userLibraryID;
      item = (await Zotero.Items.getByLibraryAndKeyAsync(libraryID, idOrKey)) as Zotero.Item | null;
    }
  }

  if (!item) throw rpcError(INVALID_PARAMS, `Item ${idOrKey} not found`);
  return item;
}

/**
 * Fetch a Collection by numeric ID or 8-char alphanumeric key.
 * Accepts both `42` and `"COL12345"` so callers that have a collection_key
 * from RAG hits or search results can pass it directly.
 */
export async function requireCollection(idOrKey: number | string): Promise<Zotero.Collection> {
  let col: Zotero.Collection | null = null;

  if (typeof idOrKey === "number") {
    col = await Zotero.Collections.getAsync(idOrKey);
  } else {
    const parsed = Number(idOrKey);
    if (Number.isFinite(parsed) && String(parsed) === idOrKey) {
      col = await Zotero.Collections.getAsync(parsed);
    } else {
      const libraryID = Zotero.Libraries.userLibraryID;
      col = (await Zotero.Collections.getByLibraryAndKeyAsync(libraryID, idOrKey)) as Zotero.Collection | null;
    }
  }

  if (!col) throw rpcError(INVALID_PARAMS, `Collection ${idOrKey} not found`);
  return col;
}
