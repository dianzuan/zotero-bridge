// Zotron Preference Pane
// Uses Zotero.HTTP.request() (not fetch!) for test buttons.

var PREF = "zotron.";

var DEFAULT_OCR_PROVIDER = "glm";
var DEFAULT_EMB_PROVIDER = "ollama";
var DEFAULT_RERANK_PROVIDER = "jina";

var OCR_CONFIGS = {
  glm:    { label: "GLM-OCR",      url: "https://open.bigmodel.cn/api/paas/v4/layout_parsing", model: "glm-ocr" },
  paddle: { label: "PaddleOCR-VL", url: "", model: "PaddleOCR-VL-1.5" },
  mineru: { label: "MinerU",       url: "https://mineru.net/api/v4/extract/task", model: "vlm" },
  custom: { label: "Custom OCR",   url: "", model: "custom-ocr-model" },
};

var EMB_CONFIGS = {
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
};

var RERANK_CONFIGS = {
  jina:       { label: "Jina",       url: "https://api.jina.ai/v1/rerank",       model: "jina-reranker-v2-base-multilingual" },
  cohere:     { label: "Cohere",     url: "https://api.cohere.com/v2/rerank",     model: "rerank-v3.5" },
  voyage:     { label: "Voyage",     url: "https://api.voyageai.com/v1/rerank",   model: "rerank-2" },
  dashscope:  { label: "DashScope",  url: "https://dashscope.aliyuncs.com/compatible-api/v1/reranks", model: "qwen3-rerank" },
  siliconflow:{ label: "SiliconFlow",url: "https://api.siliconflow.cn/v1/rerank", model: "BAAI/bge-reranker-v2-m3" },
  "openai-compatible": { label: "Custom", url: "", model: "" },
};

var I18N = {
  "en-US": {
    generalTitle: "General",
    language: "Language",
    english: "English",
    chinese: "Chinese",
    ocrTitle: "OCR",
    ocrProvider: "Provider",
    embTitle: "Embedding",
    embProvider: "Provider",
    apiKey: "API Key",
    apiKeyPlaceholder: "Leave empty until you configure a provider token",
    model: "Model",
    apiUrl: "API URL",
    test: "Test",
    missingKey: "Enter an API Key first",
    missingUrl: "No API URL configured",
    testing: "Testing…",
    okHttp: "Connection OK",
    okDim: "vector dimension",
    rerankTitle: "Reranker",
    rerankProvider: "Provider",
    rerankKeyPlaceholder: "Leave empty to disable reranking",
    ragTitle: "Advanced RAG",
    ragChunkSize: "Chunk Size",
    ragOverlap: "Chunk Overlap",
    ragTopK: "Top-K",
    ragMode: "Retrieval Mode",
    ragUnitChars: "chars",
    sourcesTitle: "Data Sources",
    coreKey: "CORE Key",
    coreKeyPlaceholder: "Optional — OA PDF fallback source",
    politeEmail: "Contact email",
    politeEmailHint: "For CrossRef / OpenAlex polite pool — optional, not a credential",
    noKeyNeeded: "no key needed",
    consoleHint: "Get a key:",
  },
  "zh-CN": {
    generalTitle: "常规",
    language: "界面语言",
    english: "英文",
    chinese: "中文",
    ocrTitle: "OCR",
    ocrProvider: "Provider",
    embTitle: "Embedding",
    embProvider: "Provider",
    apiKey: "API Key",
    apiKeyPlaceholder: "Token 默认留空，配置 provider 后再填写",
    model: "Model",
    apiUrl: "API URL",
    test: "测试",
    missingKey: "请先填写 API Key",
    missingUrl: "未配置 API URL",
    testing: "测试中…",
    okHttp: "连接正常",
    okDim: "向量维度",
    rerankTitle: "Reranker",
    rerankProvider: "Provider",
    rerankKeyPlaceholder: "留空则不启用 Reranking",
    ragTitle: "高级 RAG 参数",
    ragChunkSize: "Chunk 大小",
    ragOverlap: "Chunk 重叠",
    ragTopK: "检索数量 (Top-K)",
    ragMode: "检索模式",
    ragUnitChars: "字符",
    sourcesTitle: "数据源",
    coreKey: "CORE Key",
    coreKeyPlaceholder: "选填，OA PDF 兜底源",
    politeEmail: "联系邮箱",
    politeEmailHint: "用于 CrossRef / OpenAlex 礼貌池，选填，非登录凭证",
    noKeyNeeded: "无需配置",
    consoleHint: "申请 key：",
  },
};

function sp(key, val) {
  try { Zotero.Prefs.set(PREF + key, val); } catch(e) {}
}

function gp(key) {
  try {
    var v = Zotero.Prefs.get(PREF + key);
    return (v === undefined || v === null || v === "undefined") ? "" : v;
  } catch(e) {
    return "";
  }
}

function el(id) { return document.getElementById(id); }
function se(id, val) { var node = el(id); if (node) node.value = val || ""; }
function setAttr(id, attr, val) { var node = el(id); if (node) node.setAttribute(attr, val); }
function setText(id, val) { var node = el(id); if (node) node.textContent = val; }
function setStatus(id, msg, color) {
  var node = el(id);
  if (node) {
    node.textContent = msg;
    node.style.color = color;
  }
}

function currentLanguage() {
  var saved = gp("ui.language");
  if (I18N[saved]) return saved;
  sp("ui.language", "en-US");
  return "en-US";
}

function t(key) {
  var lang = currentLanguage();
  return (I18N[lang] && I18N[lang][key]) || I18N["en-US"][key] || key;
}

function ensureProvider(prefKey, configs, fallback) {
  var provider = gp(prefKey);
  if (!provider || !configs[provider]) {
    provider = fallback;
    sp(prefKey, provider);
  }
  return provider;
}

function setReadonly(id, readonly) {
  var node = el(id);
  if (!node) return;
  if (readonly) node.setAttribute("readonly", "readonly");
  else node.removeAttribute("readonly");
}

function applyProvider(kind, forceDefaults) {
  var isOCR = kind === "ocr";
  var configs = isOCR ? OCR_CONFIGS : EMB_CONFIGS;
  var fallback = isOCR ? DEFAULT_OCR_PROVIDER : DEFAULT_EMB_PROVIDER;
  var prefix = isOCR ? "ocr" : "embedding";
  var providerId = isOCR ? "zotron-ocr-provider" : "zotron-emb-provider";
  var urlId = isOCR ? "zotron-ocr-apiurl" : "zotron-emb-apiurl";
  var modelId = isOCR ? "zotron-ocr-model" : "zotron-emb-model";
  var provider = ensureProvider(prefix + ".provider", configs, fallback);
  var config = configs[provider];
  var selector = el(providerId);
  if (selector) selector.value = provider;

  if (forceDefaults || !gp(prefix + ".apiUrl")) sp(prefix + ".apiUrl", config.url);
  if (forceDefaults || !gp(prefix + ".model")) sp(prefix + ".model", config.model);
  se(urlId, gp(prefix + ".apiUrl"));
  se(modelId, gp(prefix + ".model"));
  setReadonly(urlId, false);
  setReadonly(modelId, false);

  if (!isOCR) {
    var keyless = provider === "ollama";
    ["zotron-emb-key-label", "zotron-emb-apikey", "zotron-emb-test", "zotron-emb-hintrow"]
      .forEach(function(id) {
        var node = el(id);
        if (!node) return;
        if (keyless) node.setAttribute("hidden", "hidden");
        else node.removeAttribute("hidden");
      });
  }
}

function applyOCR(forceDefaults) { applyProvider("ocr", !!forceDefaults); }
function applyEmb(forceDefaults) { applyProvider("embedding", !!forceDefaults); }

function applyRerank(forceDefaults) {
  var provider = ensureProvider("rerank.provider", RERANK_CONFIGS, DEFAULT_RERANK_PROVIDER);
  var config = RERANK_CONFIGS[provider];
  var editable = provider === "openai-compatible";
  var selector = el("zotron-rerank-provider");
  if (selector) selector.value = provider;
  if (forceDefaults || !editable || !gp("rerank.apiUrl")) sp("rerank.apiUrl", config.url);
  if (forceDefaults || !editable || !gp("rerank.model")) sp("rerank.model", config.model);
  se("zotron-rerank-url", gp("rerank.apiUrl"));
  se("zotron-rerank-model", gp("rerank.model"));
  setReadonly("zotron-rerank-url", false);
  setReadonly("zotron-rerank-model", false);
}

function updateRagPreview() {
  var node = el("zotron-rag-preview");
  if (!node) return;
  var chunk = gp("rag.chunkSize") || 512;
  var overlap = gp("rag.chunkOverlap") || 64;
  var topk = gp("rag.topK") || 5;
  var mode = gp("rag.retrievalMode") || "hybrid";
  node.textContent = "chunk " + chunk + " · overlap " + overlap
    + " · top-k " + topk + " · " + mode;
}

function applyI18n() {
  var lang = currentLanguage();
  var selector = el("zotron-language");
  if (selector) selector.value = lang;

  setText("zotron-general-title", t("generalTitle"));
  setAttr("zotron-lang-label", "value", t("language"));
  setAttr("zotron-lang-en", "label", t("english"));
  setAttr("zotron-lang-zh", "label", t("chinese"));

  setText("zotron-ocr-title", t("ocrTitle"));
  setAttr("zotron-ocr-provider-label", "value", t("ocrProvider"));
  setText("zotron-emb-title", t("embTitle"));
  setAttr("zotron-emb-provider-label", "value", t("embProvider"));
  setAttr("zotron-ocr-key-label", "value", t("apiKey"));
  setAttr("zotron-emb-key-label", "value", t("apiKey"));
  setAttr("zotron-ocr-model-label", "value", t("model"));
  setAttr("zotron-emb-model-label", "value", t("model"));
  setAttr("zotron-ocr-url-label", "value", t("apiUrl"));
  setAttr("zotron-emb-url-label", "value", t("apiUrl"));
  setAttr("zotron-ocr-test", "label", t("test"));
  setAttr("zotron-emb-test", "label", t("test"));
  setAttr("zotron-ocr-apikey", "placeholder", t("apiKeyPlaceholder"));
  setAttr("zotron-emb-apikey", "placeholder", t("apiKeyPlaceholder"));

  setText("zotron-rerank-title", t("rerankTitle"));
  setAttr("zotron-rerank-provider-label", "value", t("rerankProvider"));
  setAttr("zotron-rerank-key-label", "value", t("apiKey"));
  setAttr("zotron-rerank-model-label", "value", t("model"));
  setAttr("zotron-rerank-url-label", "value", t("apiUrl"));
  setAttr("zotron-rerank-test", "label", t("test"));
  setAttr("zotron-rerank-apikey", "placeholder", t("rerankKeyPlaceholder"));

  setText("zotron-rag-title", t("ragTitle"));
  setAttr("zotron-rag-chunk-label", "value", t("ragChunkSize"));
  setAttr("zotron-rag-overlap-label", "value", t("ragOverlap"));
  setAttr("zotron-rag-topk-label", "value", t("ragTopK"));
  setAttr("zotron-rag-mode-label", "value", t("ragMode"));
  setText("zotron-rag-chunk-unit", t("ragUnitChars"));
  setText("zotron-rag-overlap-unit", t("ragUnitChars"));

  setText("zotron-src-title", t("sourcesTitle"));
  setAttr("zotron-src-core-label", "value", t("coreKey"));
  setAttr("zotron-src-core-key", "placeholder", t("coreKeyPlaceholder"));
  setAttr("zotron-src-mailto-label", "value", t("politeEmail"));
  setText("zotron-src-mailto-hint", t("politeEmailHint"));
  setAttr("zotron-src-openalex-note", "value", "✓ " + t("noKeyNeeded"));
  setAttr("zotron-src-crossref-note", "value", "✓ " + t("noKeyNeeded"));

  updateRagPreview();
}

function testOCR() {
  var url = gp("ocr.apiUrl");
  var keyNode = el("zotron-ocr-apikey");
  var key = keyNode ? keyNode.value : gp("ocr.apiKey");
  if (keyNode) sp("ocr.apiKey", key);
  if (!key) { setStatus("zotron-ocr-status", t("missingKey"), "#e74c3c"); return; }
  setStatus("zotron-ocr-status", t("testing"), "#f39c12");
  var model = gp("ocr.model") || OCR_CONFIGS[DEFAULT_OCR_PROVIDER].model;
  Zotero.HTTP.request("POST", url, {
    headers: { "Content-Type": "application/json", "Authorization": "Bearer " + key },
    body: JSON.stringify({ model: model }),
    timeout: 10000,
    successCodes: false,
  }).then(function(xhr) {
    if (xhr.status === 401 || xhr.status === 403) {
      setStatus("zotron-ocr-status", t("invalidKey") + " (" + xhr.status + ")", "#e74c3c");
    } else {
      setStatus("zotron-ocr-status", t("okHttp") + " (HTTP " + xhr.status + ")", "#27ae60");
    }
  }).catch(function(e) {
    setStatus("zotron-ocr-status", t("failed") + ": " + (e.message || e), "#e74c3c");
  });
}

function testEmb() {
  var provider = ensureProvider("embedding.provider", EMB_CONFIGS, DEFAULT_EMB_PROVIDER);
  var url = gp("embedding.apiUrl");
  var keyNode = el("zotron-emb-apikey");
  var key = keyNode ? keyNode.value : gp("embedding.apiKey");
  if (keyNode) sp("embedding.apiKey", key);
  var model = gp("embedding.model") || EMB_CONFIGS[provider].model;
  if (provider !== "ollama" && !key) { setStatus("zotron-emb-status", t("missingKey"), "#e74c3c"); return; }
  setStatus("zotron-emb-status", t("testing"), "#f39c12");

  var reqUrl, body, headers;
  if (provider === "ollama") {
    reqUrl = url + "/api/embeddings";
    body = JSON.stringify({ model: model, prompt: "test" });
    headers = { "Content-Type": "application/json" };
  } else {
    reqUrl = url;
    body = JSON.stringify({ model: model, input: "test" });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  }

  Zotero.HTTP.request("POST", reqUrl, {
    headers: headers,
    body: body,
    timeout: 10000,
    successCodes: false,
  }).then(function(xhr) {
    if (xhr.status === 401 || xhr.status === 403) {
      setStatus("zotron-emb-status", t("invalidKey") + " (" + xhr.status + ")", "#e74c3c");
    } else if (xhr.status >= 200 && xhr.status < 300) {
      try {
        var data = JSON.parse(xhr.responseText);
        var dim = provider === "ollama"
          ? (data.embedding ? data.embedding.length : "?")
          : (data.data && data.data[0] ? data.data[0].embedding.length : "?");
        setStatus("zotron-emb-status", t("okDim") + ": " + dim, "#27ae60");
      } catch(e) {
        setStatus("zotron-emb-status", t("okHttp") + " (HTTP " + xhr.status + ")", "#27ae60");
      }
    } else {
      setStatus("zotron-emb-status", t("failedHttp") + " (HTTP " + xhr.status + ")", "#e74c3c");
    }
  }).catch(function(e) {
    setStatus("zotron-emb-status", t("failed") + ": " + (e.message || e), "#e74c3c");
  });
}

function testRerank() {
  var url = gp("rerank.apiUrl");
  var keyNode = el("zotron-rerank-apikey");
  var key = keyNode ? keyNode.value : gp("rerank.apiKey");
  if (keyNode) sp("rerank.apiKey", key);
  if (!key) { setStatus("zotron-rerank-status", t("missingKey"), "#e74c3c"); return; }
  if (!url) { setStatus("zotron-rerank-status", "No API URL configured", "#e74c3c"); return; }
  setStatus("zotron-rerank-status", t("testing"), "#f39c12");
  var model = gp("rerank.model") || RERANK_CONFIGS[DEFAULT_RERANK_PROVIDER].model;
  Zotero.HTTP.request("POST", url, {
    headers: { "Content-Type": "application/json", "Authorization": "Bearer " + key },
    body: JSON.stringify({ model: model, query: "test", documents: ["hello world"], top_n: 1 }),
    timeout: 10000,
    successCodes: false,
  }).then(function(xhr) {
    if (xhr.status === 401 || xhr.status === 403) {
      setStatus("zotron-rerank-status", t("invalidKey") + " (" + xhr.status + ")", "#e74c3c");
    } else if (xhr.status >= 200 && xhr.status < 300) {
      setStatus("zotron-rerank-status", t("okHttp") + " (HTTP " + xhr.status + ")", "#27ae60");
    } else {
      setStatus("zotron-rerank-status", t("failedHttp") + " (HTTP " + xhr.status + ")", "#e74c3c");
    }
  }).catch(function(e) {
    setStatus("zotron-rerank-status", t("failed") + ": " + (e.message || e), "#e74c3c");
  });
}

// Data Sources — credentials consumed by `zotron web` (settings-only).
// openalex/crossref need no configuration; only CORE key + polite-pool
// mailto are wired.
function initSources() {
  var coreKey = el("zotron-src-core-key");
  if (coreKey) {
    var savedK = gp("source.core.apiKey");
    if (savedK) coreKey.value = savedK;
    var saveK = function() { sp("source.core.apiKey", coreKey.value); };
    coreKey.addEventListener("input", saveK);
    coreKey.addEventListener("change", saveK);
    coreKey.addEventListener("blur", saveK);
  }

  var mailto = el("zotron-src-mailto");
  if (mailto) {
    var savedM = gp("source.mailto");
    if (savedM) mailto.value = savedM;
    var saveM = function() { sp("source.mailto", mailto.value); };
    mailto.addEventListener("input", saveM);
    mailto.addEventListener("change", saveM);
    mailto.addEventListener("blur", saveM);
  }
}

function init() {
  var keys = ["ocr.apiKey", "embedding.apiKey", "rerank.apiKey"];
  for (var i = 0; i < keys.length; i++) {
    if (!gp(keys[i])) sp(keys[i], "");
  }

  applyI18n();
  applyOCR();
  applyEmb();
  applyRerank();

  var langSel = el("zotron-language");
  var ocrSel = el("zotron-ocr-provider");
  var embSel = el("zotron-emb-provider");
  if (langSel) langSel.addEventListener("command", function() { sp("ui.language", langSel.value); applyI18n(); });
  if (ocrSel) ocrSel.addEventListener("command", function() { sp("ocr.provider", ocrSel.value); applyOCR(true); });
  if (embSel) embSel.addEventListener("command", function() { sp("embedding.provider", embSel.value); applyEmb(true); });
  var rerankSel = el("zotron-rerank-provider");
  if (rerankSel) rerankSel.addEventListener("command", function() { sp("rerank.provider", rerankSel.value); applyRerank(true); });

  var bindings = [
    ["zotron-ocr-apikey", "ocr.apiKey"],
    ["zotron-ocr-apiurl", "ocr.apiUrl"],
    ["zotron-ocr-model", "ocr.model"],
    ["zotron-emb-apikey", "embedding.apiKey"],
    ["zotron-emb-apiurl", "embedding.apiUrl"],
    ["zotron-emb-model", "embedding.model"],
  ];
  for (var b of bindings) {
    (function(elId, prefKey) {
      var node = el(elId);
      if (node) {
        var saved = gp(prefKey);
        if (saved) node.value = saved;
        var save = function() { sp(prefKey, node.value); };
        node.addEventListener("input", save);
        node.addEventListener("change", save);
        node.addEventListener("blur", save);
      }
    })(b[0], b[1]);
  }

  var rerankBindings = [
    ["zotron-rerank-apikey", "rerank.apiKey"],
    ["zotron-rerank-url", "rerank.apiUrl"],
    ["zotron-rerank-model", "rerank.model"],
  ];
  for (var rrb of rerankBindings) {
    (function(elId, prefKey) {
      var node = el(elId);
      if (node) {
        var saved = gp(prefKey);
        if (saved) node.value = saved;
        var save = function() { sp(prefKey, node.value); };
        node.addEventListener("input", save);
        node.addEventListener("change", save);
        node.addEventListener("blur", save);
      }
    })(rrb[0], rrb[1]);
  }

  var ocrBtn = el("zotron-ocr-test");
  var embBtn = el("zotron-emb-test");
  var rerankBtn = el("zotron-rerank-test");
  if (ocrBtn) ocrBtn.addEventListener("click", testOCR);
  if (embBtn) embBtn.addEventListener("click", testEmb);
  if (rerankBtn) rerankBtn.addEventListener("click", testRerank);

  // RAG bindings
  var ragBindings = [
    ["zotron-rag-chunksize", "rag.chunkSize"],
    ["zotron-rag-overlap", "rag.chunkOverlap"],
    ["zotron-rag-topk", "rag.topK"],
  ];
  for (var rb of ragBindings) {
    (function(elId, prefKey) {
      var node = el(elId);
      if (node) {
        var saved = gp(prefKey);
        if (saved !== "" && saved !== undefined) node.value = saved;
        var save = function() { sp(prefKey, Number(node.value) || 0); updateRagPreview(); };
        node.addEventListener("input", save);
        node.addEventListener("change", save);
      }
    })(rb[0], rb[1]);
  }

  var ragMode = el("zotron-rag-mode");
  if (ragMode) {
    ragMode.value = gp("rag.retrievalMode") || "hybrid";
    ragMode.addEventListener("command", function() { sp("rag.retrievalMode", ragMode.value); updateRagPreview(); });
  }

  initSources();
}

if (document.getElementById("zotron-ocr-provider")) { init(); }
else { setTimeout(init, 300); }
