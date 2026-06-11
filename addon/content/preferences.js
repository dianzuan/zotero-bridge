// Zotron Preference Pane — registry-driven summary list.
// Per-slot detail editing happens in manage.xhtml (opened via openManage).

var SLOT_TITLE = { "en-US": "Capabilities", "zh-CN": "能力" };
var LANG_LABEL = { "en-US": "Language:", "zh-CN": "语言:" };

var manageWin = null;

function pEl(id) { return document.getElementById(id); }
function paneLang() { var l = zGp("ui.language"); return (l === "zh-CN") ? "zh-CN" : "en-US"; }

function openManage(slotId) {
  try { if (manageWin && !manageWin.closed) manageWin.close(); } catch (e) {}
  manageWin = Zotero.getMainWindow().openDialog(
    "chrome://zotron/content/manage.xhtml", "_blank",
    "chrome,centerscreen,resizable,dialog=no", { slotId: slotId });
}

function renderSlots() {
  var lang = paneLang();
  var container = pEl("zotron-slots");
  if (!container) return;
  container.textContent = "";
  ZOTRON_CAPABILITIES.forEach(function (slot) {
    var sum = slotSummary(slot);
    var row = document.createElementNS("http://www.mozilla.org/keymaster/gatekeeper/there.is.only.xul", "hbox");
    row.setAttribute("align", "center");
    row.style.margin = "3px 0";

    var name = document.createElementNS(row.namespaceURI, "label");
    name.setAttribute("value", slot.label[lang] || slot.id);
    name.style.width = "120px";

    var value = document.createElementNS(row.namespaceURI, "label");
    value.setAttribute("value", sum.value);
    value.style.width = "160px";
    value.style.color = "#bbb";

    var dot = document.createElementNS(row.namespaceURI, "label");
    dot.setAttribute("value", sum.ok ? "●" : "○");
    dot.style.color = sum.ok ? "#27ae60" : "#888";
    dot.style.width = "24px";

    var btn = document.createElementNS(row.namespaceURI, "button");
    btn.setAttribute("label", lang === "zh-CN" ? "管理" : "Manage");
    btn.addEventListener("command", function () { openManage(slot.id); });

    row.appendChild(name); row.appendChild(value); row.appendChild(dot); row.appendChild(btn);
    container.appendChild(row);
  });
}

function initPane() {
  var langLabel = pEl("zotron-lang-label");
  if (langLabel) langLabel.setAttribute("value", LANG_LABEL[paneLang()]);
  var slotsTitle = pEl("zotron-slots-title");
  if (slotsTitle) slotsTitle.textContent = SLOT_TITLE[paneLang()];

  var langSel = pEl("zotron-language");
  if (langSel) {
    langSel.value = paneLang();
    langSel.addEventListener("command", function () {
      zSp("ui.language", langSel.value);
      pEl("zotron-lang-label").setAttribute("value", LANG_LABEL[paneLang()]);
      pEl("zotron-slots-title").textContent = SLOT_TITLE[paneLang()];
      renderSlots();
    });
  }
  renderSlots();

  // Refresh summary rows when returning from a sub-window.
  window.addEventListener("focus", renderSlots);
}

if (typeof ZOTRON_CAPABILITIES !== "undefined" && pEl("zotron-slots")) { initPane(); }
else { setTimeout(initPane, 300); }
