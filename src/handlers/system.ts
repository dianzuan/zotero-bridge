// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { registerHandlers, getRegisteredMethods } from "../server";
import { setPref } from "../utils/prefs";

export const systemHandlers = {
  async ping() { return { status: "ok", timestamp: new Date().toISOString() }; },
  async version() { return { zotero: Zotero.version, plugin: "0.1.4", methods: getRegisteredMethods().length }; },
  async libraries() {
    const libs = Zotero.Libraries.getAll();
    return libs.map((lib: any) => ({ id: lib.id, type: lib.libraryType, name: lib.name, editable: lib.editable }));
  },
  async switchLibrary(params: { id: number }) {
    const lib = Zotero.Libraries.get(params.id);
    if (!lib) throw { code: -32602, message: `Library ${params.id} not found` };
    setPref("lastLibraryID", params.id);
    return { id: lib.id, name: lib.name };
  },
  async libraryStats(params: { id?: number }) {
    const libraryID = params.id || Zotero.Libraries.userLibraryID;
    const items = await Zotero.Items.getAll(libraryID, false, false, true);
    const collections = Zotero.Collections.getByLibrary(libraryID, true);
    return { libraryId: libraryID, items: items.length, collections: collections.length };
  },
  async itemTypes() {
    const types = Zotero.ItemTypes.getAll();
    return types.map((t: any) => ({ itemType: t.name, itemTypeID: t.id, localized: Zotero.ItemTypes.getLocalizedString(t.id) }));
  },
  async itemFields(params: { itemType: string }) {
    const typeID = Zotero.ItemTypes.getID(params.itemType);
    if (!typeID) throw { code: -32602, message: `Unknown item type: ${params.itemType}` };
    const fields = Zotero.ItemFields.getItemTypeFields(typeID);
    return fields.map((fid: number) => ({ field: Zotero.ItemFields.getName(fid), fieldID: fid, localized: Zotero.ItemFields.getLocalizedString(fid) }));
  },
  async creatorTypes(params: { itemType: string }) {
    const typeID = Zotero.ItemTypes.getID(params.itemType);
    if (!typeID) throw { code: -32602, message: `Unknown item type: ${params.itemType}` };
    const types = Zotero.CreatorTypes.getTypesForItemType(typeID);
    return types.map((t: any) => ({ creatorType: t.name, creatorTypeID: t.id, localized: Zotero.CreatorTypes.getLocalizedString(t.id) }));
  },
  async sync() { await Zotero.Sync.Runner.sync(); return { status: "ok" }; },
  async currentCollection() {
    // @ts-ignore — Zotero global
    const pane = Zotero.getActiveZoteroPane();
    if (!pane) return null;
    const col = pane.getSelectedCollection();
    if (!col) return null;
    return {
      id: col.id,
      key: col.key,
      name: col.name,
      libraryId: col.libraryID,
    };
  },
  async reload() {
    // Self-reload bypasses scaffold's broken RDP path on WSL→Windows.
    // Delay so the HTTP response flushes before shutdown() tears down the endpoint.
    setTimeout(async () => {
      // Invalidate Gecko's startupcache so loadSubScript re-reads from disk —
      // without this, addon.reload() does disable→enable but the bundled
      // content/scripts/*.js stays cached in memory and the new build's code
      // is not picked up.
      (Services.obs as any).notifyObservers(null, "startupcache-invalidate", null);
      const { AddonManager } = ChromeUtils.importESModule(
        "resource://gre/modules/AddonManager.sys.mjs",
      );
      const addon = await AddonManager.getAddonByID("zotron@diamondrill");
      if (addon) await addon.reload();
    }, 100);
    return { status: "reloading" };
  },
};

registerHandlers("system", systemHandlers);
