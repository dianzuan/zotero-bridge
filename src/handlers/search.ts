// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/search.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireCollection } from "../utils/guards";

/**
 * Build a `Zotero.Search` scoped to the user library. Buries the single
 * `(s as any).libraryID` cast (zotero-types declares libraryID only on
 * the constructor params, not as an assignable property).
 */
function makeScopedSearch(): Zotero.Search {
  const s = new Zotero.Search();
  (s as any).libraryID = Zotero.Libraries.userLibraryID;
  return s;
}

type SearchCondition = {
  field: string;
  op?: string;
  operator?: string;
  value: string;
};

function conditionOperator(cond: SearchCondition): string {
  return cond.op ?? cond.operator ?? "";
}

function searchTerms(query: string): string[] {
  const terms = query
    .split(/[\s,;，；、]+/)
    .map((term) => term.trim())
    .filter(Boolean);
  return terms.length ? Array.from(new Set(terms)) : [query];
}

export const searchHandlers = {
  async quick(params: { query: string; limit?: number }) {
    const limit = params.limit ?? 25;
    const s = makeScopedSearch();
    s.addCondition("quicksearch-titleCreatorYear", "contains", params.query);
    s.addCondition("noChildren", "true");
    const ids = await s.search();
    const items = await Zotero.Items.getAsync(ids.slice(0, limit));
    const result: Record<string, any> = { items: items.map(serializeItem), total: ids.length };
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

  async advanced(params: { conditions: SearchCondition[]; limit?: number }) {
    const s = makeScopedSearch();
    for (const cond of params.conditions) {
      s.addCondition(cond.field as any, conditionOperator(cond) as any, cond.value);
    }
    s.addCondition("noChildren", "true");
    const ids = await s.search();
    const sliced = params.limit !== undefined ? ids.slice(0, params.limit) : ids;
    const items = await Zotero.Items.getAsync(sliced);
    const result: Record<string, any> = { items: items.map(serializeItem), total: ids.length };
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

  async fulltext(params: { query: string; limit?: number; collection?: string | number; collectionKey?: string | number }) {
    const limit = params.limit ?? 25;
    const s = makeScopedSearch();
    for (const term of searchTerms(params.query)) {
      s.addCondition("fulltextContent", "contains", term);
    }
    s.addCondition("noChildren", "true");
    let ids = await s.search();
    const collectionRef = params.collectionKey ?? params.collection;
    let collectionItems: any[] | null = null;
    if (collectionRef !== undefined && collectionRef !== null && collectionRef !== "") {
      const collection = await requireCollection(collectionRef);
      collectionItems = collection.getChildItems(false) || [];
      const allowed = new Set(collectionItems.map((item: any) => item.id));
      ids = ids.filter((id: number) => allowed.has(id));
      if (ids.length === 0) {
        ids = await fallbackFulltextIdsFromCollectionItems(collectionItems, searchTerms(params.query));
      }
    }
    const items = await Zotero.Items.getAsync(ids.slice(0, limit));
    const result: Record<string, any> = { items: items.map(serializeItem), total: ids.length };
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

  async byTag(params: { tag: string; limit?: number }) {
    const limit = params.limit ?? 50;
    const s = makeScopedSearch();
    s.addCondition("tag", "is", params.tag);
    s.addCondition("noChildren", "true");
    const ids = await s.search();
    const items = await Zotero.Items.getAsync(ids.slice(0, limit));
    const result: Record<string, any> = { items: items.map(serializeItem), total: ids.length };
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

  async byIdentifier(params: { doi?: string; isbn?: string; issn?: string; pmid?: string; limit?: number }) {
    const s = makeScopedSearch();
    if (params.doi) s.addCondition("DOI", "is", params.doi);
    if (params.isbn) s.addCondition("ISBN", "is", params.isbn);
    if (params.issn) s.addCondition("ISSN", "is", params.issn);
    if (params.pmid) s.addCondition("extra", "contains", `PMID: ${params.pmid}`);
    s.addCondition("noChildren", "true");
    const ids = await s.search();
    const sliced = params.limit !== undefined ? ids.slice(0, params.limit) : ids;
    const items = await Zotero.Items.getAsync(sliced);
    const result: Record<string, any> = { items: items.map(serializeItem), total: ids.length };
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

  async savedSearches() {
    const libraryID = Zotero.Libraries.userLibraryID;
    const searches = await (Zotero.Searches as any).getAll(libraryID);
    const items = searches.map((s: any) => ({
      key: s.key,
      name: s.name,
      conditions: Object.values(s.getConditions()).map((c: any) => {
        const { conditionID, ...rest } = c;
        return rest;
      }),
    }));
    return { items, total: items.length };
  },

  async createSavedSearch(params: {
    name: string;
    conditions: SearchCondition[];
  }) {
    const s = makeScopedSearch();
    s.name = params.name;
    for (const cond of params.conditions) {
      s.addCondition(cond.field as any, conditionOperator(cond) as any, cond.value);
    }
    await s.saveTx();
    return { ok: true, key: s.key, name: s.name };
  },

  async deleteSavedSearch(params: { key: number }) {
    const s = await Zotero.Searches.getAsync(params.key);
    if (!s) throw { code: -32602, message: `Saved search ${params.key} not found` };
    const key = (s as any).key;
    await s.eraseTx();
    return { ok: true, key };
  },
};

async function fallbackFulltextIdsFromCollectionItems(items: any[], terms: string[]): Promise<number[]> {
  const matches: number[] = [];
  for (const item of items) {
    if (item.isNote?.() || item.isAttachment?.()) continue;
    const content = await firstPdfAttachmentText(item);
    const haystack = content.toLowerCase();
    if (terms.every((term) => haystack.includes(term.toLowerCase()))) {
      matches.push(item.id);
    }
  }
  return matches;
}

async function firstPdfAttachmentText(item: any): Promise<string> {
  const chunks: string[] = [];
  for (const attID of item.getAttachments?.() || []) {
    const att = await Zotero.Items.getAsync(attID);
    if (!att?.isAttachment?.()) continue;
    if (String(att.attachmentContentType || "").toLowerCase() !== "application/pdf") continue;
    try {
      const cacheFile = Zotero.Fulltext.getItemCacheFile(att);
      const content = (await Zotero.File.getContentsAsync(cacheFile.path) as string) ?? "";
      if (content.trim()) chunks.push(content);
    } catch {
      continue;
    }
  }
  return chunks.join("\n");
}

registerHandlers("search", searchHandlers);
