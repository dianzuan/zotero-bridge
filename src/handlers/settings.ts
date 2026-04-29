// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { registerHandlers } from "../server";
import { findUnknownKey } from "../utils/settings-validate";

const PREF_PREFIX = "extensions.zotron.";

const SETTINGS_KEYS = [
  "ui.language",
  "ocr.provider",      // default: glm
  "ocr.apiKey",
  "ocr.apiUrl",
  "ocr.model",
  "embedding.provider", // doubao | ollama | openai | zhipu | dashscope | siliconflow | jina | voyage | cohere | gemini
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

const SETTINGS_DEFAULTS: Record<string, string | number> = {
  "ui.language": "",
  "ocr.provider": "glm",
  "ocr.apiKey": "",
  "ocr.apiUrl": "https://open.bigmodel.cn/api/paas/v4/layout_parsing",
  "ocr.model": "glm-ocr",
  "embedding.provider": "doubao",
  "embedding.model": "doubao-embedding-vision-251215",
  "embedding.apiKey": "",
  "embedding.apiUrl": "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal",
  "rag.chunkSize": 512,
  "rag.chunkOverlap": 64,
  "rag.topK": 5,
};

function getSetting(key: string): any {
  const val = Zotero.Prefs.get(PREF_PREFIX + key, true);
  return val === undefined || val === null ? SETTINGS_DEFAULTS[key] ?? null : val;
}

export const settingsHandlers = {
  async get(params: { key: string }) {
    if (!params.key) throw { code: -32602, message: "key is required" };
    if (!KNOWN_KEYS.has(params.key)) {
      throw { code: -32602, message: `Unknown setting key: ${params.key}` };
    }
    return { [params.key]: getSetting(params.key) };
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
      result[key] = getSetting(key);
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
