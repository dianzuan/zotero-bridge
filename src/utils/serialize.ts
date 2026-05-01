// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
export function serializeItem(item: Zotero.Item): Record<string, any> {
  const data: Record<string, any> = {
    key: item.key,
    version: (item as any).version ?? 0,
    itemType: item.itemType,
    title: item.getField("title") as string,
    dateAdded: item.dateAdded,
    dateModified: item.dateModified,
  };
  const fields = Zotero.ItemFields.getItemTypeFields(item.itemTypeID);
  for (const fieldID of fields) {
    const fieldName = Zotero.ItemFields.getName(fieldID);
    if (fieldName && fieldName !== "title") {
      const val = item.getField(fieldName);
      if (val) data[fieldName] = val;
    }
  }
  if (item.isNote()) {
    data.note = item.getNote();
  }
  data.creators = item.getCreators().map((c: any) => ({
    firstName: c.firstName || "",
    lastName: c.lastName || "",
    creatorType: Zotero.CreatorTypes.getName(c.creatorTypeID),
  }));
  data.tags = item.getTags().map((t: any) => ({ tag: t.tag, type: t.type }));
  data.collections = item.getCollections()
    .map((colId: number) => {
      const col = Zotero.Collections.get(colId);
      return col ? col.key : null;
    })
    .filter(Boolean);
  data.relations = item.getRelations();
  return data;
}

export function serializeCollection(col: Zotero.Collection): Record<string, any> {
  return {
    id: col.id, key: col.key, name: col.name, parentID: col.parentID || null,
    childCollections: col.getChildCollections(false).map((c: any) => c.id),
    itemCount: col.getChildItems(false).length,
  };
}
