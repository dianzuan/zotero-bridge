// Zotron per-slot management sub-window. Reads window.arguments[0].slotId,
// renders a form from ZOTRON_CAPABILITIES, saves to the SAME pref keys the old
// pane used. Test buttons reuse Zotero.HTTP.request (not fetch).

function mEl(id) { return document.getElementById(id); }

var currentSave = null;

function langKey() {
  var l = zGp("ui.language");
  return (l === "zh-CN") ? "zh-CN" : "en-US";
}

function renderSingle(slot) {
  var provider = zGp(slot.prefix + ".provider") || slot.fallback;
  var body = mEl("manage-body");
  var providerOptions = Object.keys(slot.providers).map(function (id) {
    var sel = id === provider ? " selected=\"selected\"" : "";
    return "<option value=\"" + id + "\"" + sel + ">" + esc(slot.providers[id].label) + "</option>";
  }).join("");
  body.innerHTML =
    "<div class=\"row\"><label>Provider</label>" +
    "<select id=\"f-provider\">" + providerOptions + "</select></div>" +
    "<div class=\"row\"><label>API Key</label>" +
    "<input id=\"f-apiKey\" type=\"password\" value=\"" + esc(zGp(slot.prefix + ".apiKey")) + "\" /></div>" +
    "<div class=\"status\" id=\"f-status\"></div>" +
    "<div class=\"row\"><label>Model</label>" +
    "<input id=\"f-model\" type=\"text\" value=\"" + esc(zGp(slot.prefix + ".model")) + "\" /></div>" +
    "<div class=\"row\"><label>API URL</label>" +
    "<input id=\"f-apiUrl\" type=\"text\" value=\"" + esc(zGp(slot.prefix + ".apiUrl")) + "\" /></div>" +
    "<div class=\"row\"><label></label><button id=\"f-test\">Test</button></div>";

  mEl("f-provider").addEventListener("change", function () {
    var cfg = slot.providers[mEl("f-provider").value];
    if (cfg) { mEl("f-model").value = cfg.model; mEl("f-apiUrl").value = cfg.url; }
  });
  mEl("f-test").addEventListener("click", function () { testSlot(slot); });

  currentSave = function () {
    zSp(slot.prefix + ".provider", mEl("f-provider").value);
    zSp(slot.prefix + ".apiKey", mEl("f-apiKey").value);
    zSp(slot.prefix + ".model", mEl("f-model").value);
    zSp(slot.prefix + ".apiUrl", mEl("f-apiUrl").value);
    window.close();
  };
}

function esc(s) {
  return String(s == null ? "" : s)
    .replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// Generic HTTP probe — preserves the behavior of the old testOCR/testEmb/testRerank.
function testSlot(slot) {
  var status = mEl("f-status");
  function setS(msg, color) { status.textContent = msg; status.style.color = color; }
  var url = mEl("f-apiUrl").value;
  var key = mEl("f-apiKey").value;
  var model = mEl("f-model").value || slot.providers[slot.fallback].model;
  var keyless = slot.keylessProvider && mEl("f-provider").value === slot.keylessProvider;
  if (!keyless && !key) { setS("Enter an API Key first", "#e74c3c"); return; }
  if (!url) { setS("No API URL configured", "#e74c3c"); return; }
  setS("Testing...", "#f39c12");

  var reqUrl, body, headers;
  // NOTE: carry-over from original testEmb — for keyless Ollama we append
  // "/api/embeddings" to the stored URL; if that URL already ends in
  // /v1/embeddings this double-paths. Pre-existing; fix when Ollama default URL is revisited.
  if (slot.id === "embedding" && keyless) {
    reqUrl = url + "/api/embeddings";
    body = JSON.stringify({ model: model, prompt: "test" });
    headers = { "Content-Type": "application/json" };
  } else if (slot.id === "embedding") {
    reqUrl = url; body = JSON.stringify({ model: model, input: "test" });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  } else if (slot.id === "rerank") {
    reqUrl = url;
    body = JSON.stringify({ model: model, query: "test", documents: ["hello world"], top_n: 1 });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  } else { // ocr
    reqUrl = url; body = JSON.stringify({ model: model });
    headers = { "Content-Type": "application/json", "Authorization": "Bearer " + key };
  }

  Zotero.HTTP.request("POST", reqUrl, { headers: headers, body: body, timeout: 10000, successCodes: false })
    .then(function (xhr) {
      if (xhr.status === 401 || xhr.status === 403) setS("Invalid API Key (" + xhr.status + ")", "#e74c3c");
      else if (xhr.status >= 200 && xhr.status < 300) setS("Connection OK (HTTP " + xhr.status + ")", "#27ae60");
      else setS("Request failed (HTTP " + xhr.status + ")", "#e74c3c");
    })
    .catch(function (e) { setS("Connection failed: " + (e.message || e), "#e74c3c"); });
}

function renderMulti(slot) {
  var enabled = parseEnabled(zGp(slot.enabledPref) || slot.defaultEnabled);
  var isOn = {};
  enabled.forEach(function (id) { isOn[id] = true; });
  var body = mEl("manage-body");

  var rows = slot.catalog.map(function (src) {
    var checked = isOn[src.id] ? " checked=\"checked\"" : "";
    var keyInput = "";
    if (src.auth !== "none") {
      keyInput = "<input id=\"src-key-" + src.id + "\" type=\"password\" value=\"" +
        esc(zGp(src.pref)) + "\" placeholder=\"" + esc(src.auth) + "\" />";
    } else {
      keyInput = "<span style=\"flex:1;color:#888\">no key needed</span>";
    }
    return "<div class=\"src-row\">" +
      "<input type=\"checkbox\" id=\"src-on-" + src.id + "\"" + checked + " />" +
      "<span class=\"src-name\">" + esc(src.label) + "</span>" + keyInput + "</div>";
  }).join("");

  body.innerHTML =
    rows +
    "<div class=\"row\" style=\"margin-top:10px\"><label>Polite email</label>" +
    "<input id=\"src-mailto\" type=\"text\" value=\"" + esc(zGp(slot.mailtoPref)) + "\" /></div>";

  // Set the window-level save handler (manage-save listener is wired once in startManage).
  currentSave = function () {
    var on = [];
    slot.catalog.forEach(function (src) {
      if (mEl("src-on-" + src.id).checked) on.push(src.id);
      if (src.auth !== "none") zSp(src.pref, mEl("src-key-" + src.id).value);
    });
    zSp(slot.enabledPref, serializeEnabled(on));
    zSp(slot.mailtoPref, mEl("src-mailto").value);
    window.close();
  };
}

function startManage() {
  var slotId = (window.arguments && window.arguments[0] && window.arguments[0].slotId) || "ocr";
  var slot = capabilityById(slotId);
  if (!slot) { window.close(); return; }
  mEl("manage-title").textContent = slot.label[langKey()] || slot.id;
  mEl("manage-cancel").addEventListener("click", function () { window.close(); });
  mEl("manage-save").addEventListener("click", function () { if (currentSave) currentSave(); });
  if (slot.kind === "single") renderSingle(slot);
  else renderMulti(slot); // defined in Task 4
}

if (typeof capabilityById === "function") { startManage(); }
else { window.addEventListener("load", startManage); }
