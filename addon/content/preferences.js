// Zotron Preference Pane
// Uses Zotero.HTTP.request() (not fetch!) for test buttons.

var PREF = "extensions.zotron.";

var DEFAULT_OCR_PROVIDER = "glm";
var DEFAULT_EMB_PROVIDER = "doubao";

var OCR_CONFIGS = {
  glm:    { label: "GLM-OCR",      url: "https://open.bigmodel.cn/api/paas/v4/layout_parsing",                                 model: "glm-ocr" },
  qwen:   { label: "Qwen-VL-OCR",  url: "https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation", model: "qwen-vl-ocr" },
  custom: { label: "Custom OCR",   url: "",                                                                                        model: "custom-ocr-model" },
};

var EMB_CONFIGS = {
  doubao:      { label: "Doubao",       url: "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal", model: "doubao-embedding-vision-251215" },
  ollama:      { label: "Ollama",       url: "http://localhost:11434",                                           model: "qwen3-embedding:4b" },
  zhipu:       { label: "Zhipu",        url: "https://open.bigmodel.cn/api/paas/v4/embeddings",                 model: "embedding-3" },
  dashscope:   { label: "DashScope",    url: "https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings",    model: "text-embedding-v4" },
  siliconflow: { label: "SiliconFlow",  url: "https://api.siliconflow.cn/v1/embeddings",                        model: "BAAI/bge-m3" },
  jina:        { label: "Jina",         url: "https://api.jina.ai/v1/embeddings",                               model: "jina-embeddings-v3" },
  voyage:      { label: "Voyage AI",    url: "https://api.voyageai.com/v1/embeddings",                          model: "voyage-4" },
  cohere:      { label: "Cohere",       url: "https://api.cohere.com/v2/embed",                                 model: "embed-v4.0" },
  gemini:      { label: "Google Gemini", url: "https://generativelanguage.googleapis.com/v1beta/models/gemini-embedding-001:embedContent", model: "gemini-embedding-001" },
  openai:      { label: "OpenAI",       url: "https://api.openai.com/v1/embeddings",                            model: "text-embedding-3-small" },
};

var I18N = {
  "en-US": {
    language: "Language:",
    english: "English",
    chinese: "Chinese",
    ocrTitle: "OCR Settings",
    ocrProvider: "OCR Provider:",
    embTitle: "Embedding Settings",
    embProvider: "Provider:",
    apiKey: "API Key:",
    apiKeyPlaceholder: "Leave empty until you configure a provider token",
    model: "Model:",
    apiUrl: "API URL:",
    test: "Test",
    ready: "Ready",
    missingKey: "Enter an API Key first",
    testing: "Testing...",
    invalidKey: "Invalid API Key",
    okHttp: "Connection OK",
    okDim: "Connection OK - vector dimension",
    failedHttp: "Request failed",
    failed: "Connection failed",
  },
  "zh-CN": {
    language: "语言:",
    english: "英文",
    chinese: "中文",
    ocrTitle: "OCR 设置",
    ocrProvider: "OCR Provider:",
    embTitle: "Embedding 设置",
    embProvider: "Provider:",
    apiKey: "API Key:",
    apiKeyPlaceholder: "Token 默认留空，配置 provider 后再填写",
    model: "Model:",
    apiUrl: "API URL:",
    test: "测试",
    ready: "就绪",
    missingKey: "请先填写 API Key",
    testing: "测试中...",
    invalidKey: "API Key 无效",
    okHttp: "连接成功",
    okDim: "连接成功 - 向量维度",
    failedHttp: "请求失败",
    failed: "连接失败",
  },
};

function sp(key, val) {
  try { Zotero.Prefs.set(PREF + key, val, true); } catch(e) {}
}

function gp(key) {
  try {
    var v = Zotero.Prefs.get(PREF + key, true);
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
  var editable = provider === "custom";

  var selector = el(providerId);
  if (selector) selector.value = provider;

  if (forceDefaults || !editable || !gp(prefix + ".apiUrl")) sp(prefix + ".apiUrl", config.url);
  if (forceDefaults || !editable || !gp(prefix + ".model")) sp(prefix + ".model", config.model);
  se(urlId, gp(prefix + ".apiUrl"));
  se(modelId, gp(prefix + ".model"));
  setReadonly(urlId, !editable);
  setReadonly(modelId, !editable);
}

function applyOCR(forceDefaults) { applyProvider("ocr", !!forceDefaults); }
function applyEmb(forceDefaults) { applyProvider("embedding", !!forceDefaults); }

function applyI18n() {
  var lang = currentLanguage();
  var selector = el("zotron-language");
  if (selector) selector.value = lang;

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
  setStatus("zotron-ocr-status", t("ready"), "#888");
  setStatus("zotron-emb-status", t("ready"), "#888");
}

function testOCR() {
  var url = gp("ocr.apiUrl");
  var keyNode = el("zotron-ocr-apikey");
  var key = keyNode ? keyNode.value : gp("ocr.apiKey");
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
  var model = gp("embedding.model") || EMB_CONFIGS[provider].model;
  if (provider !== "ollama" && !key) { setStatus("zotron-emb-status", t("missingKey"), "#e74c3c"); return; }
  setStatus("zotron-emb-status", t("testing"), "#f39c12");

  var reqUrl, body, headers;
  if (provider === "ollama") {
    reqUrl = url + "/api/embeddings";
    body = JSON.stringify({ model: model, prompt: "test" });
    headers = { "Content-Type": "application/json" };
  } else if (provider === "doubao") {
    reqUrl = url;
    body = JSON.stringify({ model: model, input: [{ type: "text", text: "test" }] });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  } else if (provider === "cohere") {
    reqUrl = url;
    body = JSON.stringify({ model: model, texts: ["test"], input_type: "search_query", embedding_types: ["float"] });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  } else if (provider === "gemini") {
    reqUrl = url;
    body = JSON.stringify({ taskType: "RETRIEVAL_QUERY", content: { parts: [{ text: "test" }] } });
    headers = { "Content-Type": "application/json", "x-goog-api-key": key };
  } else if (provider === "voyage") {
    reqUrl = url;
    body = JSON.stringify({ model: model, input: "test", input_type: "query" });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  } else if (provider === "jina") {
    reqUrl = url;
    body = JSON.stringify({ model: model, input: "test", task: "retrieval.query" });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
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
          : provider === "cohere"
            ? (data.embeddings && data.embeddings.float && data.embeddings.float[0] ? data.embeddings.float[0].length : "?")
            : provider === "gemini"
              ? (data.embedding && data.embedding.values ? data.embedding.values.length : "?")
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

function init() {
  var keys = ["ocr.apiKey", "embedding.apiKey"];
  for (var i = 0; i < keys.length; i++) {
    if (!gp(keys[i])) sp(keys[i], "");
  }

  applyI18n();
  applyOCR();
  applyEmb();

  var langSel = el("zotron-language");
  var ocrSel = el("zotron-ocr-provider");
  var embSel = el("zotron-emb-provider");
  if (langSel) langSel.addEventListener("command", function() { sp("ui.language", langSel.value); applyI18n(); });
  if (ocrSel) ocrSel.addEventListener("command", function() { sp("ocr.provider", ocrSel.value); applyOCR(true); });
  if (embSel) embSel.addEventListener("command", function() { sp("embedding.provider", embSel.value); applyEmb(true); });

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
        node.addEventListener("change", function() { sp(prefKey, node.value); });
      }
    })(b[0], b[1]);
  }

  var ocrBtn = el("zotron-ocr-test");
  var embBtn = el("zotron-emb-test");
  if (ocrBtn) ocrBtn.addEventListener("click", testOCR);
  if (embBtn) embBtn.addEventListener("click", testEmb);
}

if (document.getElementById("zotron-ocr-provider")) { init(); }
else { setTimeout(init, 300); }
