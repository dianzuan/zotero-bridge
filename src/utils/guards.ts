// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { rpcError, INVALID_PARAMS } from "./errors";

/**
 * Fetch an Item by key. Numeric IDs are accepted only for internal Zotero calls.
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
 * Fetch a Collection by key. Numeric IDs are accepted only for internal Zotero calls.
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

/**
 * Resolve an item key to its first PDF attachment.
 * If the item itself is an attachment, returns it directly.
 * Otherwise iterates child attachments looking for application/pdf.
 */
export async function requirePdfAttachment(idOrKey: number | string): Promise<Zotero.Item> {
  const item = await requireItem(idOrKey);
  if (item.isAttachment()) return item;
  const attIDs = item.getAttachments ? item.getAttachments() : [];
  for (const attID of attIDs) {
    const att = await Zotero.Items.getAsync(attID);
    if (att && att.isAttachment() && (att as any).attachmentContentType === "application/pdf") {
      return att;
    }
  }
  throw rpcError(INVALID_PARAMS, `No PDF attachment found for item ${item.key ?? idOrKey}`);
}

/**
 * Resolve item keys to Items. Numeric IDs are accepted only for internal Zotero calls.
 * Delegates to `requireItem` for each entry, so callers can freely pass
 * `[42, "YR5BUGHG", 7]` and get back `Zotero.Item[]`.
 */
export async function resolveItems(idsOrKeys: (number | string)[]): Promise<Zotero.Item[]> {
  const items: Zotero.Item[] = [];
  for (const idOrKey of idsOrKeys) {
    items.push(await requireItem(idOrKey));
  }
  return items;
}
