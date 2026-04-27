// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { registerHandlers } from "../server";
import { findUnknownKey } from "../utils/settings-validate";

const PREF_PREFIX = "extensions.zotron.";

const SETTINGS_KEYS = [
  "ocr.provider",      // default: glm
  "ocr.apiKey",
  "ocr.apiUrl",
  "ocr.model",
  "embedding.provider", // ollama | openai | zhipu | dashscope | siliconflow | jina | voyage | cohere | gemini | doubao
  "embedding.model",
  "embedding.apiKey",
  "embedding.apiUrl",
  "rag.chunkSize",
  "rag.chunkOverlap",
  "rag.topK",
];

// ReadonlySet derived from SETTINGS_KEYS — shared by set (includes-check) and
// setAll (findUnknownKey). Extend here when new settings are introduced.
const KNOWN_KEYS: ReadonlySet<string> = new Set(SETTINGS_KEYS);

export const settingsHandlers = {
  async get(params: { key: string }) {
    if (!params.key) throw { code: -32602, message: "key is required" };
    if (!KNOWN_KEYS.has(params.key)) {
      throw { code: -32602, message: `Unknown setting key: ${params.key}` };
    }
    const val = Zotero.Prefs.get(PREF_PREFIX + params.key, true) ?? null;
    return { [params.key]: val };
  },

  async set(params: { key: string; value: any }) {
    if (!params.key) throw { code: -32602, message: "key is required" };
    if (!KNOWN_KEYS.has(params.key)) {
      throw { code: -32602, message: `Unknown setting: ${params.key}. Valid: ${SETTINGS_KEYS.join(", ")}` };
    }
    Zotero.Prefs.set(PREF_PREFIX + params.key, params.value, true);
    return { key: params.key, value: params.value };
  },

  async getAll() {
    const result: Record<string, any> = {};
    for (const key of SETTINGS_KEYS) {
      result[key] = Zotero.Prefs.get(PREF_PREFIX + key, true) ?? null;
    }
    return result;
  },

  async setAll(params: Record<string, any>) {
    const unknown = findUnknownKey(params, KNOWN_KEYS);
    if (unknown) throw { code: -32602, message: `Unknown setting key: ${unknown}` };

    const updated: Record<string, any> = {};
    for (const [key, value] of Object.entries(params)) {
      Zotero.Prefs.set(PREF_PREFIX + key, value, true);
      updated[key] = value;
    }
    return { updated };
  },
};

registerHandlers("settings", settingsHandlers);
