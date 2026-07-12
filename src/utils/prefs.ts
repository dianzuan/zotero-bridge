const ADDON_PREF_PREFIX = "zotron.";

export const PREF_DEFAULTS: Record<string, string | number> = {
  "ui.language": "en-US",
  "ocr.provider": "",
  "ocr.apiKey": "",
  "ocr.apiUrl": "",
  "ocr.model": "",
  "embedding.provider": "",
  "embedding.model": "",
  "embedding.apiKey": "",
  "embedding.apiUrl": "",
  "rag.chunkSize": 512,
  "rag.chunkOverlap": 64,
  "rag.topK": 5,
  "rag.retrievalMode": "hybrid",
  "source.mailto": "",
  "source.core.apiKey": "",
};

function prefKey(key: string): string {
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

