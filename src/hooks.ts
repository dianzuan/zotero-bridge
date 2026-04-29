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
import { getRawPref, PREF_DEFAULTS, setPref } from "./utils/prefs";

function readLegacyPref(candidates: string[]): any {
  for (const key of candidates) {
    for (const global of [true, false]) {
      const value = Zotero.Prefs.get(key, global);
      if (value !== undefined && value !== null && value !== "") {
        return value;
      }
    }
  }
  return undefined;
}

function legacyCandidates(key: string): string[] {
  return [
    `extensions.zotron.${key}`,
    key,
    `extensions.zoteroBridge.${key}`,
    `extensions.zotero-bridge.${key}`,
    `extensions.zotero_bridge.${key}`,
  ];
}

function migrateLegacyPrefs() {
  for (const [target, defaultValue] of Object.entries(PREF_DEFAULTS)) {
    const currentValue = getRawPref(target);
    const legacyValue = readLegacyPref(legacyCandidates(target));
    if (legacyValue === undefined) continue;
    if (
      currentValue === undefined
      || currentValue === null
      || currentValue === ""
      || currentValue === defaultValue
    ) {
      setPref(target, legacyValue);
    }
  }
}

function setPreferenceDefaults() {
  for (const [key, val] of Object.entries(PREF_DEFAULTS)) {
    if (getRawPref(key) === undefined) {
      setPref(key, val);
    }
  }

  migrateLegacyPrefs();

  const hasUntouchedOldOllamaDefault =
    getRawPref("embedding.provider") === "ollama"
    && getRawPref("embedding.model") === "qwen3-embedding:4b"
    && getRawPref("embedding.apiUrl") === "http://localhost:11434"
    && getRawPref("embedding.apiKey") === "";

  if (hasUntouchedOldOllamaDefault) {
    setPref("embedding.provider", PREF_DEFAULTS["embedding.provider"]);
    setPref("embedding.model", PREF_DEFAULTS["embedding.model"]);
    setPref("embedding.apiUrl", PREF_DEFAULTS["embedding.apiUrl"]);
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
