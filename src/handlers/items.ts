// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/items.ts
import { registerHandlers } from "../server";
import { CJK_REGEX } from "../utils/chinese-name";
import { serializeItem } from "../utils/serialize";
import { creatorKeyName, extractYear } from "../utils/citation-key";
import { requireItem, requireCollection, resolveItems } from "../utils/guards";
import { sanitizePath } from "../utils/safe-path";

/** Normalize a creator entry. CJK full names with empty firstName are stored
 *  in single-field mode (fieldMode=1) instead of being split — matches
 *  common CJK translator behavior. */
function normalizeCreator(c: { firstName: string; lastName: string; creatorType: string; fieldMode?: number }) {
  if (c.firstName && c.firstName.length > 0) return c;
  if (!c.lastName || !CJK_REGEX.test(c.lastName)) return c;
  const cjkChars = Array.from(c.lastName).filter(ch => CJK_REGEX.test(ch));
  if (cjkChars.length < 2) return c;
  return { ...c, fieldMode: 1 };
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
  input: { fields?: Record<string, string>; creators?: Array<{ firstName: string; lastName: string; creatorType: string; fieldMode?: number }>; tags?: string[] },
) {
  if (input.fields) {
    for (const [field, value] of Object.entries(input.fields)) {
      zotItem.setField(field, value);
    }
  }
  if (input.creators) {
    zotItem.setCreators(input.creators.map(c => normalizeCreator(c)) as any);
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
    creators?: Array<{ firstName: string; lastName: string; creatorType: string; fieldMode?: number }>;
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
        params.creators.map((c) => normalizeCreator(c)) as any
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
    creators?: Array<{ firstName: string; lastName: string; creatorType: string; fieldMode?: number }>;
    tags?: string[];
  }) {
    const item = await requireItem(params.key);

    if (params.fields) {
      for (const [field, value] of Object.entries(params.fields)) {
        item.setField(field, value);
      }
    }

    if (params.creators) {
      item.setCreators(
        params.creators.map((c) => normalizeCreator(c)) as any
      );
    }

    if (params.tags && params.tags.length > 0) {
      // Full replace semantics — drop existing tags, apply new set. Matches
      // user intent for `on_duplicate=update` (refresh metadata to latest).
      // Empty array is treated as "don't touch tags" to avoid clobbering
      // user-added tags when an external parse returns no keywords.
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
    const sortColumn = params.type === "modified" ? "dateModified" : "dateAdded";
    const whereClause = `WHERE libraryID=?
                   AND itemTypeID NOT IN (
                     SELECT itemTypeID FROM itemTypes WHERE typeName IN ('note', 'attachment'))
                   AND itemID NOT IN (SELECT itemID FROM deletedItems)`;
    const total = Number(await Zotero.DB.valueQueryAsync(
      `SELECT COUNT(*) FROM items ${whereClause}`, [libraryID]));
    const sql = `SELECT itemID FROM items ${whereClause}
                 ORDER BY ${sortColumn} DESC LIMIT ? OFFSET ?`;
    const ids: number[] = await Zotero.DB.columnQueryAsync(sql, [libraryID, limit, offset]);
    const items = ids.length > 0 ? await Zotero.Items.getAsync(ids) : [];
    return {
      items: items.map(serializeItem),
      total,
      limit,
      offset,
    };
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
      throw { code: -32602, message: "Need at least 2 item keys to merge" };
    }
    const items = await resolveItems(params.keys);
    const master = items[0];

    const mergeKeySet = new Set(items.map((i: any) => i.key));
    const externalRelatedKeys = new Set<string>();
    for (const item of items) {
      for (const rk of (item.relatedItems || [])) {
        if (!mergeKeySet.has(rk)) externalRelatedKeys.add(rk);
      }
    }

    for (let i = 1; i < items.length; i++) {
      await Zotero.Items.merge(master, [items[i]]);
    }

    for (const rk of externalRelatedKeys) {
      const related = await Zotero.Items.getByLibraryAndKeyAsync(master.libraryID, rk);
      if (related) {
        master.addRelatedItem(related);
        related.addRelatedItem(master);
        await related.saveTx();
      }
    }
    if (externalRelatedKeys.size > 0) await master.saveTx();

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
    const whereClause = `WHERE libraryID=?
                   AND itemTypeID NOT IN (
                     SELECT itemTypeID FROM itemTypes WHERE typeName IN ('note', 'attachment'))
                   AND itemID NOT IN (SELECT itemID FROM deletedItems)`;
    const total = Number(await Zotero.DB.valueQueryAsync(
      `SELECT COUNT(*) FROM items ${whereClause}`, [libraryID]));
    const sql = `SELECT itemID FROM items ${whereClause}
                 ORDER BY ${sortColumn} ${sortDir} LIMIT ? OFFSET ?`;
    const ids: number[] = await Zotero.DB.columnQueryAsync(sql, [libraryID, limit, offset]);
    const items = ids.length > 0 ? await Zotero.Items.getAsync(ids) : [];
    return {
      items: items.map(serializeItem),
      total,
      limit,
      offset,
    };
  },

  async getFullText(params: { key: number | string }) {
    const item = await requireItem(params.key);
    await item.loadAllData();
    const attIDs = item.getAttachments ? item.getAttachments() : [];
    for (const attID of attIDs) {
      const att = await Zotero.Items.getAsync(attID);
      if (att && att.isAttachment() && (att as any).attachmentContentType === "application/pdf") {
        const cacheFile = Zotero.Fulltext.getItemCacheFile(att);
        let content = "";
        let fromFallback = false;
        try {
          content = (await Zotero.File.getContentsAsync(cacheFile.path) as string) ?? "";
        } catch { content = ""; }

        // Fallback: cache empty → live extraction via PDFWorker
        if (!content) {
          try {
            const result = await (Zotero as any).PDFWorker.getFullText(att.id);
            content = (result?.text ?? "").replace(/\f/g, "\n");
            if (content) fromFallback = true;
          } catch { /* PDFWorker unavailable */ }
        }

        const rows = ((await Zotero.DB.queryAsync(
          "SELECT indexedChars, totalChars FROM fulltextItems WHERE itemID=?",
          [att.id],
        )) as Array<{ indexedChars: number; totalChars: number }>) ?? [];
        const meta = rows[0] ?? { indexedChars: 0, totalChars: 0 };
        return {
          key: att.key,
          content: content ?? "",
          indexedChars: fromFallback ? content.length : (meta.indexedChars ?? 0),
          totalChars: fromFallback ? content.length : (meta.totalChars ?? 0),
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
    const key = match ? match[1] : `${creatorKeyName(item.getCreators()[0])}${extractYear(item)}`;
    return { key: item.key, citationKey: key };
  },

  async push(params: {
    item: {
      itemType?: string;
      fields?: Record<string, string>;
      creators?: Array<{ firstName: string; lastName: string; creatorType: string; fieldMode?: number }>;
      tags?: string[];
    };
    pdf?: string;
    collection?: string | number;
    onDuplicate?: "skip" | "update" | "create";
  }) {
    const onDuplicate = params.onDuplicate ?? "skip";
    const item = params.item;

    let collectionKey: number | null = null;
    if (params.collection !== undefined && params.collection !== null && params.collection !== 0) {
      collectionKey = (await requireCollection(params.collection)).id;
    }

    const dupKey = await findDupByDOIOrTitle(item.fields);

    let itemKey: string;
    let status: "created" | "updated" | "skipped_duplicate";

    if (dupKey && onDuplicate === "skip") {
      itemKey = dupKey;
      status = "skipped_duplicate";
      if (collectionKey) {
        const dupItem = await requireItem(dupKey);
        dupItem.addToCollection(collectionKey);
        await dupItem.saveTx();
      }
    } else if (dupKey && onDuplicate === "update") {
      const dupItem = await requireItem(dupKey);
      applyFields(dupItem, item);
      if (collectionKey) dupItem.addToCollection(collectionKey);
      await dupItem.saveTx();
      itemKey = dupKey;
      status = "updated";
    } else {
      const newItem = new Zotero.Item((item.itemType || "journalArticle") as any);
      newItem.libraryID = Zotero.Libraries.userLibraryID;
      applyFields(newItem, item);
      if (collectionKey) newItem.addToCollection(collectionKey);
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
