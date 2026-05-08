import { expect } from "chai";
import sinon from "sinon";
import { readFileSync } from "fs";
import vm from "vm";

function loadBootstrap(overrides: Record<string, any> = {}) {
  const code = readFileSync("addon/bootstrap.js", "utf8");
  const context = vm.createContext({
    console,
    APP_SHUTDOWN: "APP_SHUTDOWN",
    Components: {
      classes: {
        "@mozilla.org/addons/addon-manager-startup;1": {
          getService: sinon.stub().returns({ registerChrome: sinon.stub().returns({ destruct: sinon.stub() }) }),
        },
      },
      interfaces: { amIAddonManagerStartup: "amIAddonManagerStartup" },
    },
    Services: {
      io: { newURI: sinon.stub().callsFake((uri: string) => uri) },
      prefs: {
        getDefaultBranch: sinon.stub().returns({
          setBoolPref: sinon.stub(),
          setStringPref: sinon.stub(),
          setIntPref: sinon.stub(),
        }),
      },
      scriptloader: { loadSubScript: sinon.stub() },
    },
    Zotero: {
      Zotron: { data: {}, hooks: { onStartup: sinon.stub().resolves(), onShutdown: sinon.stub().resolves() } },
      logError: sinon.stub(),
    },
    ...overrides,
  });
  vm.runInContext(code, context, { filename: "addon/bootstrap.js" });
  return context as any;
}

describe("addon bootstrap", () => {
  it("loads the bundled script with a single slash after rootURI", async () => {
    const ctx = loadBootstrap();
    const rootURI = "jar:file:///tmp/zotron.xpi!/";

    await ctx.startup({ id: "zotron@diamondrill", version: "0.1.5", rootURI }, "ADDON_ENABLE");

    const loadedScripts = ctx.Services.scriptloader.loadSubScript.getCalls().map((call: sinon.SinonSpyCall) => call.args[0]);
    expect(loadedScripts).to.include("jar:file:///tmp/zotron.xpi!/prefs.js");
    expect(loadedScripts).to.include("jar:file:///tmp/zotron.xpi!/content/scripts/zotron.js");
    expect(loadedScripts.some((uri: string) => uri.includes("!//content"))).to.equal(false);
  });

  it("publishes rootURI before invoking plugin startup hooks", async () => {
    const onStartup = sinon.stub().callsFake(function (this: any) {
      expect(ctx.Zotero.Zotron.data.rootURI).to.equal("resource://zotron/");
      return Promise.resolve();
    });
    const ctx = loadBootstrap({
      Zotero: {
        Zotron: { data: {}, hooks: { onStartup, onShutdown: sinon.stub().resolves() } },
        logError: sinon.stub(),
      },
    });

    await ctx.startup({ id: "zotron@diamondrill", version: "0.1.5", rootURI: "resource://zotron/" }, "ADDON_ENABLE");

    expect(onStartup.calledOnce).to.equal(true);
  });

  it("runs addon shutdown cleanup except during application shutdown", async () => {
    const ctx = loadBootstrap();

    await ctx.startup({ id: "zotron@diamondrill", version: "0.1.5", rootURI: "resource://zotron/" }, "ADDON_ENABLE");
    await ctx.shutdown({ id: "zotron@diamondrill", version: "0.1.5", rootURI: "resource://zotron/" }, "ADDON_DISABLE");

    expect(ctx.Zotero.Zotron.hooks.onShutdown.calledOnce).to.equal(true);
    const chromeHandle = ctx.Components.classes["@mozilla.org/addons/addon-manager-startup;1"].getService.firstCall.returnValue.registerChrome.firstCall.returnValue;
    expect(chromeHandle.destruct.calledOnce).to.equal(true);

    await ctx.startup({ id: "zotron@diamondrill", version: "0.1.5", rootURI: "resource://zotron/" }, "ADDON_ENABLE");
    await ctx.shutdown({ id: "zotron@diamondrill", version: "0.1.5", rootURI: "resource://zotron/" }, "APP_SHUTDOWN");

    expect(ctx.Zotero.Zotron.hooks.onShutdown.calledOnce).to.equal(true);
  });
});
