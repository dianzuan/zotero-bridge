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
const SECRET_KEYS: ReadonlySet<string> = new Set([
  "ocr.apiKey",
  "embedding.apiKey",
]);

function getSetting(key: string): any {
  return getPref(key);
}

function redactSetting(key: string, value: any): any {
  if (!SECRET_KEYS.has(key)) return value;
  if (value === undefined || value === null || value === "") return "";
  return "REDACTED";
}

export const settingsHandlers = {
  async get(params: { key: string }) {
    if (!params.key) throw { code: -32602, message: "key is required" };
    if (!KNOWN_KEYS.has(params.key)) {
      throw { code: -32602, message: `Unknown setting key: ${params.key}` };
    }
    return { [params.key]: redactSetting(params.key, getSetting(params.key)) };
  },

  async set(params: { key: string; value: any }) {
    if (!params.key) throw { code: -32602, message: "key is required" };
    if (!KNOWN_KEYS.has(params.key)) {
      throw { code: -32602, message: `Unknown setting: ${params.key}. Valid: ${SETTINGS_KEYS.join(", ")}` };
    }
    setPref(params.key, params.value);
    return { key: params.key, value: redactSetting(params.key, params.value) };
  },

  async getAll() {
    const result: Record<string, any> = {};
    for (const key of SETTINGS_KEYS) {
      result[key] = redactSetting(key, getSetting(key));
    }
    return result;
  },

  async setAll(params: Record<string, any>) {
    const updates = normalizeSetAllPayload(params);
    const unknown = findUnknownKey(updates, KNOWN_KEYS);
    if (unknown) throw { code: -32602, message: `Unknown setting key: ${unknown}` };

    const updated: Record<string, any> = {};
    for (const [key, value] of Object.entries(updates)) {
      setPref(key, value);
      updated[key] = redactSetting(key, value);
    }
    return { updated };
  },
};

function normalizeSetAllPayload(params: Record<string, any>): Record<string, any> {
  if (
    params
    && typeof params === "object"
    && !Array.isArray(params)
    && Object.keys(params).length === 1
    && params.settings
    && typeof params.settings === "object"
    && !Array.isArray(params.settings)
  ) {
    return params.settings;
  }
  return params;
}

registerHandlers("settings", settingsHandlers);
