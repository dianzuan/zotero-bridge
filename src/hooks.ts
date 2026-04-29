// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { registerEndpoint, unregisterEndpoint } from "./server";
import "./handlers/system";
import "./handlers/items";
import "./handlers/search";
import "./handlers/collections";
import "./handlers/tags";
import "./handlers/attachments";
import "./handlers/notes";
import "./handlers/export";
import "./handlers/settings";
import "./handlers/rag";
export async function onStartup() {
  registerEndpoint();

  // Set preference defaults
  const PREF = "extensions.zotron.";
  const defaults: Record<string, string> = {
    "ocr.provider": "glm",
    "ocr.apiKey": "",
    "ocr.apiUrl": "https://open.bigmodel.cn/api/paas/v4/layout_parsing",
    "ocr.model": "glm-ocr",
    "embedding.provider": "doubao",
    "embedding.model": "doubao-embedding-vision-251215",
    "embedding.apiKey": "",
    "embedding.apiUrl": "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal",
    "ui.language": "",
  };
  for (const [key, val] of Object.entries(defaults)) {
    if (Zotero.Prefs.get(PREF + key, true) === undefined) {
      Zotero.Prefs.set(PREF + key, val, true);
    }
  }

  // Register preference pane
  const rootURI = (Zotero as any).Zotron?.data?.rootURI;
  if (rootURI) {
    Zotero.PreferencePanes.register({
      pluginID: "zotron@diamondrill",
      src: rootURI + "content/preferences.xhtml",
      scripts: [rootURI + "content/preferences.js"],
      label: "Zotron",
      image: rootURI + "content/icons/icon.png",
    });
  }
}
export function onShutdown() { unregisterEndpoint(); }
export function onMainWindowLoad(_win: Window) {}
export function onMainWindowUnload(_win: Window) {}
