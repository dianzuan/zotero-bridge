// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
import { registerHandlers } from "../server";
import { findUnknownKey } from "../utils/settings-validate";
import { getPref, setPref } from "../utils/prefs";

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

function getSetting(key: string): any {
  return getPref(key);
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
    setPref(params.key, params.value);
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
      setPref(key, value);
      updated[key] = value;
    }
    return { updated };
  },
};

registerHandlers("settings", settingsHandlers);
