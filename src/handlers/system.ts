// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { registerHandlers, getRegisteredMethods } from "../server";
import { setPref } from "../utils/prefs";
import { requirePdfAttachment } from "../utils/guards";

export const systemHandlers = {
  async ping() { return { status: "ok", timestamp: new Date().toISOString() }; },
  async version() { return { zotero: Zotero.version, plugin: "0.2.2", methods: getRegisteredMethods().length }; },
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
    return types.map((t: any) => ({ itemType: t.name, localized: Zotero.ItemTypes.getLocalizedString(t.id) }));
  },
  async itemFields(params: { itemType: string }) {
    const typeID = Zotero.ItemTypes.getID(params.itemType);
    if (!typeID) throw { code: -32602, message: `Unknown item type: ${params.itemType}` };
    const fields = Zotero.ItemFields.getItemTypeFields(typeID);
    return fields.map((fid: number) => ({ field: Zotero.ItemFields.getName(fid), localized: Zotero.ItemFields.getLocalizedString(fid) }));
  },
  async creatorTypes(params: { itemType: string }) {
    const typeID = Zotero.ItemTypes.getID(params.itemType);
    if (!typeID) throw { code: -32602, message: `Unknown item type: ${params.itemType}` };
    const types = Zotero.CreatorTypes.getTypesForItemType(typeID);
    return types.map((t: any) => ({ creatorType: t.name, localized: Zotero.CreatorTypes.getLocalizedString(t.id) }));
  },
  async sync() { await Zotero.Sync.Runner.sync(); return { status: "ok" }; },
  async currentCollection() {
    // @ts-ignore — Zotero global
    const pane = Zotero.getActiveZoteroPane();
    if (!pane) return null;
    const col = pane.getSelectedCollection();
    if (!col) return null;
    return {
      key: col.key,
      name: col.name,
      libraryId: col.libraryID,
    };
  },
  async listMethods() {
    return getRegisteredMethods();
  },

  async pdfRecognizerData(params: { key: number | string; maxChars?: number }) {
    const { attachment } = await requirePdfAttachment(params.key);
    const data = await (Zotero as any).PDFWorker.getRecognizerData(attachment.id);
    // Return summary + first page sample to understand the format
    const pages = data?.pages ?? [];
    const pageSummaries = pages.map((page: any, i: number) => {
      const [pw, ph, lineGroups] = [page[0], page[1], page[2] ?? []];
      let charCount = 0;
      let text = "";
      for (const lg of lineGroups) {
        for (const line of lg) {
          for (const wg of line) {
            for (const char of wg) {
              if (Array.isArray(char) && char.length >= 14) {
                charCount++;
                text += String(char[13]);
                if (char[5]) text += " "; // spaceAfter
              }
            }
          }
        }
      }
      return { page: i, width: pw, height: ph, chars: charCount, text: text.slice(0, 200) };
    });
    return {
      totalPages: data?.totalPages ?? pages.length,
      metadata: data?.metadata,
      pages: pageSummaries,
      charFieldOrder: "[xMin,yMin,xMax,yMax,fontSize,spaceAfter,baseline,rotation,underlined,bold,italic,colorIndex,fontType,text]",
    };
  },

  async describe(params: { method?: string }) {
    const methods = getRegisteredMethods();
    if (params.method) {
      if (!methods.includes(params.method)) {
        throw { code: -32601, message: `Method not found: ${params.method}` };
      }
      return { name: params.method, description: `RPC method ${params.method}` };
    }
    return methods.map(name => ({ name, description: `RPC method ${name}` }));
  },

  async openPDF(params: { key: number | string }) {
    const { attachment } = await requirePdfAttachment(params.key);
    const win = (Zotero as any).getMainWindow?.();
    if (!win) throw { code: -32603, message: "No main window" };
    const zp = win.ZoteroPane;
    if (!zp) throw { code: -32603, message: "No ZoteroPane" };
    await zp.viewAttachment(attachment.id);
    return { ok: true, attachmentKey: attachment.key, attachmentID: attachment.id };
  },

  async closePDF(params: { key: number | string }) {
    const { attachment } = await requirePdfAttachment(params.key);
    const win = (Zotero as any).getMainWindow?.();
    if (!win) throw { code: -32603, message: "No main window" };
    const tabs = win.Zotero_Tabs?._tabs ?? [];
    for (const tab of tabs) {
      if (tab.id) {
        const reader = (Zotero as any).Reader.getByTabID(tab.id);
        if (reader && reader.itemID === attachment.id) {
          win.Zotero_Tabs.close(tab.id);
          return { ok: true, closed: true, attachmentKey: attachment.key };
        }
      }
    }
    return { ok: true, closed: false, message: "No open reader tab found" };
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
