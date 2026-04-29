// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/notes.ts
import { registerHandlers } from "../server";
import { validateAnnotationParams } from "../utils/annotation";
import { serializeItem } from "../utils/serialize";
import { requireItem } from "../utils/guards";

/**
 * If `item` is already a PDF attachment, return it. Otherwise look at
 * its child attachments and return the first PDF, or throw -32602 if
 * none exist. Used by both `getAnnotations` and `createAnnotation`.
 */
async function resolvePDFAttachment(item: Zotero.Item): Promise<Zotero.Item> {
  if (item.isAttachment()) return item;
  const attIDs = item.getAttachments();
  const atts = await Zotero.Items.getAsync(attIDs);
  const pdf = atts.find((a: any) => a.attachmentContentType === "application/pdf");
  if (!pdf) throw { code: -32602, message: "No PDF attachment found" };
  return pdf;
}

function defaultAnnotationSortIndex(position: any): string {
  const pageIndex = Number.isFinite(position?.pageIndex) ? Number(position.pageIndex) : 0;
  const firstRect = Array.isArray(position?.rects) ? position.rects[0] : null;
  const y = Array.isArray(firstRect) && Number.isFinite(firstRect[1]) ? Number(firstRect[1]) : 0;
  return [
    Math.max(0, Math.round(pageIndex)).toString().padStart(5, "0"),
    "000000",
    Math.max(0, Math.round(y)).toString().padStart(5, "0"),
  ].join("|");
}

function serializeAnnotation(a: any): Record<string, any> {
  return {
    id: a.id,
    type: a.annotationType,
    text: a.annotationText,
    comment: a.annotationComment,
    color: a.annotationColor,
    pageLabel: a.annotationPageLabel,
    position: a.annotationPosition,
    dateAdded: a.dateAdded,
    tags: a.getTags().map((t: any) => t.tag),
  };
}

export const notesHandlers = {
  async get(params: { parentId: number }) {
    const parent = await requireItem(params.parentId);
    const noteIDs = parent.getNotes();
    if (noteIDs.length === 0) return [];
    const notes = await Zotero.Items.getAsync(noteIDs);
    return notes.map(serializeItem);
  },

  async create(params: { parentId: number; content: string; tags?: string[] }) {
    const parent = await requireItem(params.parentId);
    const note = new Zotero.Item("note");
    note.libraryID = parent.libraryID;
    note.parentID = params.parentId;
    note.setNote(params.content);
    if (params.tags) {
      for (const tag of params.tags) note.addTag(tag);
    }
    await note.saveTx();
    return { id: note.id, key: note.key };
  },

  async update(params: { id: number; content: string }) {
    const note = await Zotero.Items.getAsync(params.id);
    if (!note || !note.isNote()) throw { code: -32602, message: `Note ${params.id} not found` };
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

  async getAnnotations(params: { parentId: number }) {
    const parent = await requireItem(params.parentId);
    const pdfItem = await resolvePDFAttachment(parent);
    const annotationsOrIDs: any[] = (pdfItem as any).getAnnotations(false, true) || [];
    const annotations = annotationsOrIDs.every((value) => typeof value === "number")
      ? await Zotero.Items.getAsync(annotationsOrIDs)
      : annotationsOrIDs;
    return annotations.map(serializeAnnotation);
  },

  async createAnnotation(params: {
    parentId: number;
    type: string;
    text?: string;
    comment?: string;
    color?: string;
    position: any;
    sortIndex?: string;
  }) {
    const v = validateAnnotationParams(params as any);
    if (!v.ok) throw { code: -32602, message: v.message };
    const parent = await requireItem(params.parentId);
    const pdfItem = await resolvePDFAttachment(parent);
    const annotation = new Zotero.Item("annotation");
    annotation.libraryID = pdfItem.libraryID;
    annotation.parentID = pdfItem.id;
    (annotation as any).annotationType = params.type;
    if (params.text) annotation.annotationText = params.text;
    if (params.comment) annotation.annotationComment = params.comment;
    annotation.annotationColor = params.color || "#ffd400";
    (annotation as any).annotationSortIndex = params.sortIndex || defaultAnnotationSortIndex(params.position);
    annotation.annotationPosition = JSON.stringify(params.position);
    await annotation.saveTx();
    return { id: annotation.id, key: annotation.key };
  },
};

registerHandlers("notes", notesHandlers);
