// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/annotations.ts
import { registerHandlers } from "../server";
import { serializeItem } from "../utils/serialize";
import { requireItem } from "../utils/guards";
import { validateAnnotationParams } from "../utils/annotation";

export const annotationsHandlers = {
  async list(params: { parentKey: number | string }) {
    const item = await requireItem(params.parentKey);
    const annIDs: number[] = (item as any).getAnnotations?.() ?? [];
    if (annIDs.length === 0) return [];
    const anns = (await Zotero.Items.getAsync(annIDs)) as any[];
    return anns.map((a: any) => {
      const data = serializeItem(a);
      data.annotationType = a.annotationType;
      data.annotationText = a.annotationText || "";
      data.annotationComment = a.annotationComment || "";
      data.annotationColor = a.annotationColor || "";
      data.annotationPosition = a.annotationPosition ? JSON.parse(a.annotationPosition) : null;
      return data;
    });
  },

  async create(params: {
    parentKey: number | string;
    type: string;
    text?: string;
    comment?: string;
    color?: string;
    position: any;
    sortIndex?: unknown;
  }) {
    const parent = await requireItem(params.parentKey);
    const validation = validateAnnotationParams({
      type: params.type as any,
      text: params.text,
      color: params.color,
      comment: params.comment,
      position: params.position,
      sortIndex: params.sortIndex,
    });
    if (!validation.ok) throw { code: -32602, message: validation.message };

    const ann = new Zotero.Item("annotation");
    ann.libraryID = parent.libraryID;
    ann.parentID = parent.id;
    (ann as any).annotationType = params.type;
    if (params.text) (ann as any).annotationText = params.text;
    if (params.comment) (ann as any).annotationComment = params.comment;
    if (params.color) (ann as any).annotationColor = params.color;
    (ann as any).annotationPosition = JSON.stringify(params.position);
    (ann as any).annotationSortIndex = params.sortIndex === undefined
      ? 0
      : typeof params.sortIndex === "number"
        ? params.sortIndex
        : Number(String(params.sortIndex).trim());
    await ann.saveTx();
    return { ok: true, key: ann.key };
  },

  async delete(params: { key: number | string }) {
    const item = await requireItem(params.key);
    await item.eraseTx();
    return { ok: true, key: item.key };
  },
};

registerHandlers("annotations", annotationsHandlers);
