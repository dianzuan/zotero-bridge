// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/items.ts
import { registerHandlers } from "../server";
import { splitChineseName, CJK_REGEX } from "../utils/chinese-name";
import { serializeItem } from "../utils/serialize";
import { extractYear } from "../utils/citation-key";
import { requireItem, requireCollection, resolveItems } from "../utils/guards";
import { sanitizePath } from "../utils/safe-path";

/** Auto-split a creator's CJK name when firstName is empty and lastName
 *  contains 2+ CJK characters. Callers that pre-split (firstName set) pass
 *  through unchanged. Non-CJK names also pass through. */
function autoSplitCreator(c: { firstName: string; lastName: string; creatorType: string }) {
  // Only auto-split when caller clearly left firstName empty AND lastName
  // looks like a full Chinese name (≥ 2 CJK chars).
  if (c.firstName && c.firstName.length > 0) return c;
  if (!c.lastName || !CJK_REGEX.test(c.lastName)) return c;
  // Count CJK chars; single-character surnames like 李 need no splitting.
  const cjkChars = Array.from(c.lastName).filter(ch => CJK_REGEX.test(ch));
  if (cjkChars.length < 2) return c;
  const split = splitChineseName(c.lastName);
  return { ...c, firstName: split.firstName, lastName: split.lastName };
}

/**
 * Build a `Zotero.Search` scoped to the user library.
 */
function makeScopedSearch(): Zotero.Search {
  const s = new Zotero.Search();
  (s as any).libraryID = Zotero.Libraries.userLibraryID;
  return s;
}

async function findDupByDOIOrTitle(fields?: Record<string, string>): Promise<string | null> {
  if (!fields) return null;
  if (fields.DOI) {
    const s = makeScopedSearch();
    s.addCondition("DOI", "is", fields.DOI);
    const ids = await s.search();
    if (ids.length > 0) {
      const found = await Zotero.Items.getAsync(ids[0]);
      if (found) return (found as any).key;
    }
  }
  const title = fields.title;
  if (title && title.length >= 10) {
    const s = makeScopedSearch();
    s.addCondition("quicksearch-titleCreatorYear", "contains", title);
    const ids = await s.search();
    for (const id of ids.slice(0, 20)) {
      const candidate = await Zotero.Items.getAsync(id);
      if (candidate && (candidate.getField("title") as string) === title) {
        return (candidate as any).key;
      }
    }
  }
  return null;
}

function applyFields(
  zotItem: Zotero.Item,
  input: { fields?: Record<string, string>; creators?: Array<{ firstName: string; lastName: string; creatorType: string }>; tags?: string[] },
) {
  if (input.fields) {
    for (const [field, value] of Object.entries(input.fields)) {
      zotItem.setField(field, value);
    }
  }
  if (input.creators) {
    zotItem.setCreators(input.creators.map(c => autoSplitCreator(c)) as any);
  }
  if (input.tags && input.tags.length > 0) {
    for (const existing of zotItem.getTags()) zotItem.removeTag(existing.tag);
    for (const tag of input.tags) zotItem.addTag(tag);
  }
}


async function tryAttachPdf(itemKey: string, pdfPath: string): Promise<boolean> {
  const item = await requireItem(itemKey);
  const attIDs = item.getAttachments ? item.getAttachments() : [];
  for (const attID of attIDs) {
    const att = await Zotero.Items.getAsync(attID);
    if (att && (att as any).attachmentContentType === "application/pdf") return false;
  }
  const cleanPath = sanitizePath(pdfPath);
  const file = Zotero.File.pathToFile(cleanPath);
  if (!file.exists()) return false;
  await Zotero.Attachments.importFromFile({ file: cleanPath, parentItemID: item.id });
  return true;
}

export const itemsHandlers = {
  async get(params: { key: number | string }) {
    const item = await requireItem(params.key);
    return serializeItem(item);
  },

  async create(params: {
    itemType: string;
    fields?: Record<string, string>;
    creators?: Array<{ firstName: string; lastName: string; creatorType: string }>;
    tags?: string[];
    collections?: number[];
  }) {
    const item = new Zotero.Item(params.itemType as any);
    item.libraryID = Zotero.Libraries.userLibraryID;

    if (params.fields) {
      for (const [field, value] of Object.entries(params.fields)) {
        item.setField(field, value);
      }
    }

    if (params.creators) {
      item.setCreators(
        params.creators.map((c) => autoSplitCreator({
          firstName: c.firstName,
          lastName: c.lastName,
          creatorType: c.creatorType,
        })) as any
      );
    }

    if (params.tags) {
      for (const tag of params.tags) {
        item.addTag(tag);
      }
    }

    if (params.collections) {
      for (const colID of params.collections) {
        item.addToCollection(colID);
      }
    }

    await item.saveTx();
    return serializeItem(item);
  },

  async update(params: {
    key: number | string;
    fields?: Record<string, string>;
    creators?: Array<{ firstName: string; lastName: string; creatorType: string }>;
    tags?: string[];
  }) {
    const item = await requireItem(params.key);

    if (params.fields) {
      for (const [field, value] of Object.entries(params.fields)) {
        item.setField(field, value);
      }
    }

    if (params.creators) {
      // Same auto-split as items.create so re-push metadata updates keep
      // compound-surname behavior (欧阳 → 欧阳/修).
      item.setCreators(
        params.creators.map((c) => autoSplitCreator({
          firstName: c.firstName,
          lastName: c.lastName,
          creatorType: c.creatorType,
        })) as any
      );
    }

    if (params.tags && params.tags.length > 0) {
      // Full replace semantics — drop existing tags, apply new set. Matches
      // user intent for `on_duplicate=update` (refresh metadata to latest).
      // Empty array is treated as "don't touch tags" to avoid clobbering
      // user-added tags when a CNKI parse returns no keywords.
      for (const existing of item.getTags()) {
        item.removeTag(existing.tag);
      }
      for (const tag of params.tags) {
        item.addTag(tag);
      }
    }

    await item.saveTx();
    return serializeItem(item);
  },

  async delete(params: { key: number | string }) {
    const item = await requireItem(params.key);
    await item.eraseTx();
    return { ok: true, key: item.key };
  },

  async trash(params: { key: number | string }) {
    const item = await requireItem(params.key);
    item.deleted = true;
    await item.saveTx();
    return { ok: true, key: item.key };
  },

  async restore(params: { key: number | string }) {
    const item = await requireItem(params.key);
    item.deleted = false;
    await item.saveTx();
    return { ok: true, key: item.key };
  },

  async getTrash(params: { limit?: number; offset?: number }) {
    const libraryID = Zotero.Libraries.userLibraryID;
    const trashedIDs = (await Zotero.Items.getDeleted(libraryID, true)) as number[];
    const offset = params.offset ?? 0;
    const limit = params.limit ?? 50;
    const sliceIDs = trashedIDs.slice(offset, offset + limit);
    const items = (sliceIDs.length > 0 ? await Zotero.Items.getAsync(sliceIDs) : []) as any[];
    return { items: items.map(serializeItem), total: trashedIDs.length, limit, offset };
  },

  async batchTrash(params: { keys: (number | string)[] }) {
    const items = await resolveItems(params.keys);
    const keys: string[] = [];
    for (const item of items) {
      item.deleted = true;
    }
    await Zotero.DB.executeTransaction(async () => {
      for (const item of items) {
        await item.save();
        keys.push(item.key);
      }
    });
    return { ok: true, keys, count: keys.length };
  },

  async getRecent(params: { limit?: number; offset?: number; type?: "added" | "modified" }) {
    const libraryID = Zotero.Libraries.userLibraryID;
    const limit = params.limit ?? 20;
    const offset = params.offset ?? 0;
    // Zotero.Search has no orderByField/sortDirection conditions — query DB
    // directly for top-N item IDs sorted by dateAdded or dateModified.
    // Excludes notes / attachments (top-level only) and deleted items.
    const sortColumn = params.type === "modified" ? "dateModified" : "dateAdded";
    const sql = `SELECT itemID FROM items
                 WHERE libraryID=?
                   AND itemTypeID NOT IN (
                     SELECT itemTypeID FROM itemTypes WHERE typeName IN ('note', 'attachment'))
                   AND itemID NOT IN (SELECT itemID FROM deletedItems)
                 ORDER BY ${sortColumn} DESC LIMIT ? OFFSET ?`;
    const ids: number[] = await Zotero.DB.columnQueryAsync(sql, [libraryID, limit, offset]);
    const items = ids.length > 0 ? await Zotero.Items.getAsync(ids) : [];
    const result: Record<string, any> = {
      items: items.map(serializeItem),
      total: items.length,
    };
    if (params.limit !== undefined) result.limit = params.limit;
    if (params.offset !== undefined) result.offset = params.offset;
    return result;
  },

  async addByDOI(params: { doi: string; collection?: number }) {
    const translate = new Zotero.Translate.Search();
    translate.setIdentifier({ DOI: params.doi });
    const translators = await translate.getTranslators();
    translate.setTranslator(translators);
    const translateOpts: any = { libraryID: Zotero.Libraries.userLibraryID };
    if (params.collection !== undefined) translateOpts.collections = [params.collection];
    const items = await translate.translate(translateOpts);
    if (!items || items.length === 0) {
      throw { code: -32602, message: `No results for DOI: ${params.doi}` };
    }
    return items.map(serializeItem);
  },

  async addByURL(params: { url: string; collection?: number }) {
    const translate = new Zotero.Translate.Web();
    // Create a hidden browser for URL translation
    const [doc] = await Zotero.HTTP.processDocuments(params.url, (doc: any) => doc);
    translate.setDocument(doc);
    const translators = await translate.getTranslators();
    if (!translators || translators.length === 0) {
      throw { code: -32602, message: `No translator for URL: ${params.url}` };
    }
    translate.setTranslator(translators[0]);
    const translateOpts: any = { libraryID: Zotero.Libraries.userLibraryID };
    if (params.collection !== undefined) translateOpts.collections = [params.collection];
    const items = await translate.translate(translateOpts);
    return (items || []).map(serializeItem);
  },

  async addByISBN(params: { isbn: string; collection?: number }) {
    const translate = new Zotero.Translate.Search();
    translate.setIdentifier({ ISBN: params.isbn });
    const translators = await translate.getTranslators();
    translate.setTranslator(translators);
    const translateOpts: any = { libraryID: Zotero.Libraries.userLibraryID };
    if (params.collection !== undefined) translateOpts.collections = [params.collection];
    const items = await translate.translate(translateOpts);
    if (!items || items.length === 0) {
      throw { code: -32602, message: `No results for ISBN: ${params.isbn}` };
    }
    return items.map(serializeItem);
  },

  async addFromFile(params: { path: string; collection?: number }) {
    const cleanPath = sanitizePath(params.path);
    const file = Zotero.File.pathToFile(cleanPath);
    if (!file.exists()) {
      throw { code: -32602, message: `File not found: ${params.path}` };
    }
    const libraryID = Zotero.Libraries.userLibraryID;
    const collections = params.collection ? [params.collection] : [];
    const importedItem = await Zotero.Attachments.importFromFile({
      file,
      libraryID,
      collections,
    });
    return [serializeItem(importedItem)];
  },

  async findDuplicates() {
    const libraryID = Zotero.Libraries.userLibraryID;
    const duplicates = new (Zotero as any).Duplicates(libraryID);
    const search = await duplicates.getSearchObject();
    const memberIDs: number[] = await search.search();
    const seen = new Set<number>();
    const groups: number[][] = [];
    for (const id of memberIDs) {
      if (seen.has(id)) continue;
      const set: number[] = duplicates.getSetItemsByItemID(id);
      if (!set || set.length === 0) {
        seen.add(id);
        continue;
      }
      for (const sid of set) seen.add(sid);
      if (set.length > 1) groups.push(set);  // groups of 1 are not duplicates
    }
    const keyGroups: string[][] = [];
    for (const group of groups) {
      const items = await Zotero.Items.getAsync(group);
      keyGroups.push(items.map((i: any) => i.key));
    }
    return { groups: keyGroups, totalGroups: keyGroups.length };
  },

  async mergeDuplicates(params: { keys: (number | string)[] }) {
    if (params.keys.length < 2) {
      throw { code: -32602, message: "Need at least 2 item IDs to merge" };
    }
    const items = await resolveItems(params.keys);
    const master = items[0];
    for (let i = 1; i < items.length; i++) {
      await Zotero.Items.merge(master, [items[i]]);
    }
    return serializeItem(master);
  },

  async getRelated(params: { key: number | string }) {
    const item = await requireItem(params.key);
    const relatedKeys = item.relatedItems;
    const related = [];
    for (const key of relatedKeys) {
      const relItem = await Zotero.Items.getByLibraryAndKeyAsync(item.libraryID, key);
      if (relItem) related.push(serializeItem(relItem));
    }
    return related;
  },

  async addRelated(params: { key: number | string; relatedKey: number | string }) {
    const item = await requireItem(params.key);
    const related = await requireItem(params.relatedKey);
    item.addRelatedItem(related);
    await item.saveTx();
    related.addRelatedItem(item);
    await related.saveTx();
    return { ok: true, key: item.key };
  },

  async removeRelated(params: { key: number | string; relatedKey: number | string }) {
    const item = await requireItem(params.key);
    const related = await requireItem(params.relatedKey);
    item.removeRelatedItem(related);
    await item.saveTx();
    related.removeRelatedItem(item);
    await related.saveTx();
    return { ok: true, key: item.key };
  },

  async list(params: { limit?: number; offset?: number; sort?: string; direction?: string }) {
    const libraryID = Zotero.Libraries.userLibraryID;
    const limit = params.limit ?? 50;
    const offset = params.offset ?? 0;
    const sortColumn = params.sort === "dateModified" ? "dateModified" : "dateAdded";
    const sortDir = params.direction === "asc" ? "ASC" : "DESC";
    const sql = `SELECT itemID FROM items
                 WHERE libraryID=?
                   AND itemTypeID NOT IN (
                     SELECT itemTypeID FROM itemTypes WHERE typeName IN ('note', 'attachment'))
                   AND itemID NOT IN (SELECT itemID FROM deletedItems)
                 ORDER BY ${sortColumn} ${sortDir} LIMIT ? OFFSET ?`;
    const ids: number[] = await Zotero.DB.columnQueryAsync(sql, [libraryID, limit, offset]);
    const items = ids.length > 0 ? await Zotero.Items.getAsync(ids) : [];
    return {
      items: items.map(serializeItem),
      total: items.length,
      limit,
      offset,
    };
  },

  async getFullText(params: { key: number | string }) {
    const item = await requireItem(params.key);
    const attIDs = item.getAttachments ? item.getAttachments() : [];
    for (const attID of attIDs) {
      const att = await Zotero.Items.getAsync(attID);
      if (att && att.isAttachment() && (att as any).attachmentContentType === "application/pdf") {
        const cacheFile = Zotero.Fulltext.getItemCacheFile(att);
        let content = "";
        try {
          content = (await Zotero.File.getContentsAsync(cacheFile.path) as string) ?? "";
        } catch { content = ""; }
        const rows = ((await Zotero.DB.queryAsync(
          "SELECT indexedChars, totalChars FROM fulltextItems WHERE itemID=?",
          [att.id],
        )) as Array<{ indexedChars: number; totalChars: number }>) ?? [];
        const meta = rows[0] ?? { indexedChars: 0, totalChars: 0 };
        return {
          key: att.key,
          content: content ?? "",
          indexedChars: meta.indexedChars ?? 0,
          totalChars: meta.totalChars ?? 0,
        };
      }
    }
    return { key: item.key, content: "", indexedChars: 0, totalChars: 0 };
  },

  async citationKey(params: { key: number | string }) {
    const item = await requireItem(params.key);
    // Try Better BibTeX citation key if available
    const extra = item.getField("extra") as string;
    const match = extra?.match(/Citation Key:\s*(\S+)/i);
    const key = match ? match[1] : `${item.getCreators()[0]?.lastName || "Unknown"}${extractYear(item)}`;
    return { key: item.key, citationKey: key };
  },

  async push(params: {
    item: {
      itemType?: string;
      fields?: Record<string, string>;
      creators?: Array<{ firstName: string; lastName: string; creatorType: string }>;
      tags?: string[];
    };
    pdf?: string;
    collection?: string | number;
    onDuplicate?: "skip" | "update" | "create";
  }) {
    const onDuplicate = params.onDuplicate ?? "skip";
    const item = params.item;

    let collectionId: number | null = null;
    if (params.collection !== undefined && params.collection !== null && params.collection !== 0) {
      collectionId = (await requireCollection(params.collection)).id;
    }

    const dupKey = await findDupByDOIOrTitle(item.fields);

    let itemKey: string;
    let status: "created" | "updated" | "skipped_duplicate";

    if (dupKey && onDuplicate === "skip") {
      itemKey = dupKey;
      status = "skipped_duplicate";
      if (collectionId) {
        const dupItem = await requireItem(dupKey);
        dupItem.addToCollection(collectionId);
        await dupItem.saveTx();
      }
    } else if (dupKey && onDuplicate === "update") {
      const dupItem = await requireItem(dupKey);
      applyFields(dupItem, item);
      if (collectionId) dupItem.addToCollection(collectionId);
      await dupItem.saveTx();
      itemKey = dupKey;
      status = "updated";
    } else {
      const newItem = new Zotero.Item((item.itemType || "journalArticle") as any);
      newItem.libraryID = Zotero.Libraries.userLibraryID;
      applyFields(newItem, item);
      if (collectionId) newItem.addToCollection(collectionId);
      await newItem.saveTx();
      itemKey = newItem.key;
      status = "created";
    }

    const pdfAttached = params.pdf
      ? await tryAttachPdf(itemKey, sanitizePath(params.pdf))
      : false;

    return { status, key: itemKey, pdfAttached };
  },
};

registerHandlers("items", itemsHandlers);
