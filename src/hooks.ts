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

const PREF = "extensions.zotron.";

const DEFAULT_PREFS: Record<string, string> = {
  "ocr.provider": "glm",
  "ocr.apiKey": "",
  "ocr.apiUrl": "https://open.bigmodel.cn/api/paas/v4/layout_parsing",
  "ocr.model": "glm-ocr",
  "embedding.provider": "doubao",
  "embedding.model": "doubao-embedding-vision-251215",
  "embedding.apiKey": "",
  "embedding.apiUrl": "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal",
  "ui.language": "en-US",
};

function prefGet(key: string): any {
  return Zotero.Prefs.get(PREF + key, true);
}

function prefSet(key: string, value: string): void {
  Zotero.Prefs.set(PREF + key, value, true);
}

function setPreferenceDefaults() {
  for (const [key, val] of Object.entries(DEFAULT_PREFS)) {
    if (prefGet(key) === undefined) {
      prefSet(key, val);
    }
  }

  const hasUntouchedOldOllamaDefault =
    prefGet("embedding.provider") === "ollama"
    && prefGet("embedding.model") === "qwen3-embedding:4b"
    && prefGet("embedding.apiUrl") === "http://localhost:11434"
    && prefGet("embedding.apiKey") === "";

  if (hasUntouchedOldOllamaDefault) {
    prefSet("embedding.provider", DEFAULT_PREFS["embedding.provider"]);
    prefSet("embedding.model", DEFAULT_PREFS["embedding.model"]);
    prefSet("embedding.apiUrl", DEFAULT_PREFS["embedding.apiUrl"]);
  }
}

export async function onStartup() {
  registerEndpoint();

  setPreferenceDefaults();

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

export const __test__ = { setPreferenceDefaults };
