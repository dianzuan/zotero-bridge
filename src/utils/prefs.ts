export const ADDON_PREF_PREFIX = "zotron.";

export const PREF_DEFAULTS: Record<string, string | number> = {
  "ui.language": "en-US",
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

export function prefKey(key: string): string {
  return `${ADDON_PREF_PREFIX}${key}`;
}

export function getPref(key: string): any {
  const value = Zotero.Prefs.get(prefKey(key));
  return value === undefined || value === null ? PREF_DEFAULTS[key] ?? null : value;
}

export function getRawPref(key: string): any {
  return Zotero.Prefs.get(prefKey(key));
}

export function setPref(key: string, value: any): void {
  Zotero.Prefs.set(prefKey(key), value);
}

export function clearPref(key: string): void {
  Zotero.Prefs.clear(prefKey(key));
}
