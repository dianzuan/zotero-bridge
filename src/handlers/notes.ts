// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/notes.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem } from "../utils/guards";

export const notesHandlers = {
  async get(params: { id: number | string }) {
    const note = await requireItem(params.id);
    if (!note.isNote()) throw { code: -32602, message: `Item ${params.id} is not a note` };
    return serializeItem(note);
  },

  async list(params: { parentId: number | string }) {
    const parent = await requireItem(params.parentId);
    const noteIDs = parent.getNotes();
    if (noteIDs.length === 0) return [];
    const notes = await Zotero.Items.getAsync(noteIDs);
    return notes.map(serializeItem);
  },

  async create(params: { parentId: number | string; content: string; tags?: string[] }) {
    const parent = await requireItem(params.parentId);
    const note = new Zotero.Item("note");
    note.libraryID = parent.libraryID;
    note.parentID = parent.id;
    note.setNote(params.content);
    if (params.tags) {
      for (const tag of params.tags) note.addTag(tag);
    }
    await note.saveTx();
    return { key: note.key };
  },

  async update(params: { id: number | string; content: string }) {
    const note = await requireItem(params.id);
    if (!note.isNote()) throw { code: -32602, message: `Note ${params.id} not found` };
    note.setNote(params.content);
    await note.saveTx();
    return serializeItem(note);
  },

  async search(params: { query: string; limit?: number }) {
    const limit = params.limit ?? 25;
    const libraryID = Zotero.Libraries.userLibraryID;
    const s = new Zotero.Search();
    (s as any).libraryID = libraryID;
    s.addCondition("itemType", "is", "note");
    s.addCondition("note", "contains", params.query);
    const ids = await s.search();
    const sliced = ids.slice(0, limit);
    const notes = await Zotero.Items.getAsync(sliced);
    const result: Record<string, any> = {
      items: notes.map(serializeItem),
      total: ids.length,
    };
    if (params.limit !== undefined) result.limit = params.limit;
    return result;
  },

};

registerHandlers("notes", notesHandlers);
