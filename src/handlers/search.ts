// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/search.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";

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

  async fulltext(params: { query: string; limit?: number }) {
    const limit = params.limit ?? 25;
    const s = makeScopedSearch();
    s.addCondition("fulltextContent", "contains", params.query);
    s.addCondition("noChildren", "true");
    const ids = await s.search();
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
    return searches.map((s: any) => ({
      key: s.key,
      name: s.name,
      conditions: s.getConditions(),
    }));
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

registerHandlers("search", searchHandlers);
