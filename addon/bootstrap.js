var chromeHandle;

function install(data, reason) {}

async function startup({ id, version, resourceURI, rootURI }, reason) {
  var aomStartup = Components.classes[
    "@mozilla.org/addons/addon-manager-startup;1"
  ].getService(Components.interfaces.amIAddonManagerStartup);
  var manifestURI = Services.io.newURI(rootURI + "manifest.json");
  chromeHandle = aomStartup.registerChrome(manifestURI, [
    ["content", "zotron", rootURI + "content/"],
  ]);
  setDefaultPrefs(rootURI);

  const ctx = { rootURI };
  ctx._globalThis = ctx;

  Services.scriptloader.loadSubScript(
    `${rootURI}/content/scripts/zotron.js`,
    ctx,
  );
  Zotero.Zotron.data.rootURI = rootURI;
  await Zotero.Zotron.hooks.onStartup();
}

async function onMainWindowLoad({ window }, reason) {
  await Zotero.Zotron?.hooks.onMainWindowLoad(window);
}

async function onMainWindowUnload({ window }, reason) {
  await Zotero.Zotron?.hooks.onMainWindowUnload(window);
}

async function shutdown({ id, version, resourceURI, rootURI }, reason) {
  if (reason === APP_SHUTDOWN) {
    return;
  }

  await Zotero.Zotron?.hooks.onShutdown();

  if (chromeHandle) {
    chromeHandle.destruct();
    chromeHandle = null;
  }
}

async function uninstall(data, reason) {}

function setDefaultPrefs(rootURI) {
  var branch = Services.prefs.getDefaultBranch("");
  var obj = {
    pref(pref, value) {
      switch (typeof value) {
        case "boolean":
          branch.setBoolPref(pref, value);
          break;
        case "string":
          branch.setStringPref(pref, value);
          break;
        case "number":
          branch.setIntPref(pref, value);
          break;
        default:
          Zotero.logError(`Invalid type '${typeof value}' for pref '${pref}'`);
      }
    },
  };
  Services.scriptloader.loadSubScript(rootURI + "prefs.js", obj);
}
