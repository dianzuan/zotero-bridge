// Zotron capability registry — shared by preferences.xhtml (main pane) and
// manage.xhtml (per-slot sub-window). Plain chrome script: defines globals on
// the loading window. No module system, no zotero-plugin-toolkit.

var ZOTRON_PREF = "zotron.";

function zGp(key) {
  try {
    var v = Zotero.Prefs.get(ZOTRON_PREF + key);
    return (v === undefined || v === null || v === "undefined") ? "" : v;
  } catch (e) { return ""; }
}
function zSp(key, val) { try { Zotero.Prefs.set(ZOTRON_PREF + key, val); } catch (e) {} }

function parseEnabled(csv) {
  return String(csv || "").split(",").map(function (s) { return s.trim(); }).filter(Boolean);
}
function serializeEnabled(arr) { return arr.join(", "); }

// Provider maps copied verbatim from the pre-refactor preferences.js
// (OCR_CONFIGS / EMB_CONFIGS / RERANK_CONFIGS). Keep entries byte-identical so
// existing saved prefs keep resolving.
var ZOTRON_CAPABILITIES = [
  {
    id: "ocr",
    label: { "en-US": "OCR", "zh-CN": "OCR" },
    kind: "single",
    prefix: "ocr",
    fallback: "glm",
    keylessProvider: null,
    providers: {
      glm:    { label: "GLM-OCR",      url: "https://open.bigmodel.cn/api/paas/v4/layout_parsing", model: "glm-ocr" },
      paddle: { label: "PaddleOCR-VL", url: "", model: "PaddleOCR-VL-1.5" },
      mineru: { label: "MinerU",       url: "https://mineru.net/api/v4/extract/task", model: "vlm" },
      custom: { label: "Custom OCR",   url: "", model: "custom-ocr-model" },
    },
  },
  {
    id: "embedding",
    label: { "en-US": "Embedding", "zh-CN": "向量" },
    kind: "single",
    prefix: "embedding",
    fallback: "ollama",
    keylessProvider: "ollama",
    providers: {
      ollama:     { label: "Ollama (本地)", url: "http://localhost:11434/v1/embeddings", model: "nomic-embed-text" },
      openai:     { label: "OpenAI",       url: "https://api.openai.com/v1/embeddings", model: "text-embedding-3-small" },
      volcengine: { label: "Volcengine",   url: "https://ark.cn-beijing.volces.com/api/v3/embeddings", model: "doubao-embedding-text-240715" },
      dashscope:  { label: "DashScope",    url: "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings", model: "text-embedding-v4" },
      zhipu:      { label: "Zhipu",        url: "https://open.bigmodel.cn/api/paas/v4/embeddings", model: "embedding-3" },
      jina:       { label: "Jina",         url: "https://api.jina.ai/v1/embeddings", model: "jina-embeddings-v3" },
      siliconflow:{ label: "SiliconFlow",  url: "https://api.siliconflow.cn/v1/embeddings", model: "BAAI/bge-m3" },
      voyage:     { label: "Voyage",       url: "https://api.voyageai.com/v1/embeddings", model: "voyage-4" },
      cohere:     { label: "Cohere",       url: "https://api.cohere.com/v2/embed", model: "embed-multilingual-v3.0" },
      custom:     { label: "Custom",       url: "", model: "" },
    },
  },
  {
    id: "rerank",
    label: { "en-US": "Rerank", "zh-CN": "重排" },
    kind: "single",
    prefix: "rerank",
    fallback: "jina",
    keylessProvider: null,
    providers: {
      jina:       { label: "Jina",       url: "https://api.jina.ai/v1/rerank",       model: "jina-reranker-v2-base-multilingual" },
      cohere:     { label: "Cohere",     url: "https://api.cohere.com/v2/rerank",     model: "rerank-v3.5" },
      voyage:     { label: "Voyage",     url: "https://api.voyageai.com/v1/rerank",   model: "rerank-2" },
      dashscope:  { label: "DashScope",  url: "https://dashscope.aliyuncs.com/compatible-api/v1/reranks", model: "qwen3-rerank" },
      siliconflow:{ label: "SiliconFlow",url: "https://api.siliconflow.cn/v1/rerank", model: "BAAI/bge-reranker-v2-m3" },
      "openai-compatible": { label: "Custom", url: "", model: "" },
    },
  },
  {
    id: "sources",
    label: { "en-US": "Data Sources", "zh-CN": "数据源" },
    kind: "multi",
    enabledPref: "sources.enabled",
    mailtoPref: "source.mailto",
    defaultEnabled: "openalex, crossref",
    catalog: [
      { id: "openalex", label: "OpenAlex",  auth: "none" },
      { id: "crossref", label: "CrossRef",  auth: "none", needsMailto: true },
      { id: "core",     label: "CORE",      auth: "apiKey", pref: "source.core.apiKey",   secret: true },
      { id: "ads",      label: "NASA ADS",  auth: "token",  pref: "source.ads.token",     secret: true },
      { id: "aminer",   label: "AMiner",    auth: "apiKey", pref: "source.aminer.apiKey", secret: true },
    ],
  },
];

function capabilityById(id) {
  for (var i = 0; i < ZOTRON_CAPABILITIES.length; i++) {
    if (ZOTRON_CAPABILITIES[i].id === id) return ZOTRON_CAPABILITIES[i];
  }
  return null;
}

// Returns { value: string, ok: boolean } for the main-pane summary row.
function slotSummary(slot) {
  if (slot.kind === "single") {
    var p = zGp(slot.prefix + ".provider") || slot.fallback;
    var cfg = slot.providers[p];
    var keyless = slot.keylessProvider && p === slot.keylessProvider;
    var configured = !!keyless || !!zGp(slot.prefix + ".apiKey");
    return { value: cfg ? cfg.label : p, ok: configured };
  }
  var enabled = parseEnabled(zGp(slot.enabledPref) || slot.defaultEnabled);
  return { value: enabled.length + " enabled", ok: enabled.length > 0 };
}
