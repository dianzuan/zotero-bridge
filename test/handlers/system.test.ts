import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("system handler", () => {
  beforeEach(() => {
    // Clear the require cache before each test to force re-evaluation
    // of the handler module with fresh Zotero stubs
    delete require.cache[require.resolve("../../src/handlers/system")];
  });

  afterEach(() => resetZotero());

  describe("sync (fix #9)", () => {
    it("calls Zotero.Sync.Runner.sync with no arguments — NOT {libraries:'all'}", async () => {
      const syncStub = sinon.stub().resolves();
      installZotero({
        Sync: { Runner: { sync: syncStub } },
      });

      // Import the handler AFTER installing the stub so that top-level Zotero
      // references resolve to the mocked version
      const { systemHandlers } = await import("../../src/handlers/system");

      const result = await systemHandlers.sync();

      expect(syncStub.calledOnce).to.equal(true);
      // The call must omit the libraries key OR pass {} — but never the
      // string "all" which Zotero converts via Array.from("all") to
      // ["a","l","l"] — see audit external row system.ts.
      const args = syncStub.firstCall.args;
      if (args.length === 1) {
        expect(args[0]).to.not.have.property("libraries");
      }
      expect(result).to.deep.equal({ status: "ok" });
    });
  });

  describe("switchLibrary (fix #8)", () => {
    it("writes to the zotron add-on preference branch", async () => {
      const setStub = sinon.stub();
      const getLibStub = sinon.stub().returns({ id: 5, name: "My Library" });
      installZotero({
        Prefs: { set: setStub },
        Libraries: { get: getLibStub },
      });

      delete require.cache[require.resolve("../../src/handlers/system")];
      const { systemHandlers } = await import("../../src/handlers/system");

      await systemHandlers.switchLibrary({ id: 5 });

      expect(setStub.calledOnce).to.equal(true);
      const [key, value, global] = setStub.firstCall.args;
      expect(key).to.equal("zotron.lastLibraryID");
      expect(value).to.equal(5);
      expect(global).to.equal(undefined);
    });
  });

  describe("libraryStats (fix #20)", () => {
    it("calls Collections.getByLibrary with recursive=true to count library-wide", async () => {
      const collectionsStub = sinon.stub().returns([{ id: 1 }, { id: 2 }, { id: 3 }]);
      installZotero({
        Libraries: { userLibraryID: 1 },
        Items: { getAll: sinon.stub().resolves([{}, {}, {}, {}]) },
        Collections: { getByLibrary: collectionsStub },
      });

      delete require.cache[require.resolve("../../src/handlers/system")];
      const { systemHandlers } = await import("../../src/handlers/system");

      await systemHandlers.libraryStats({});

      const args = collectionsStub.firstCall.args;
      expect(args[1]).to.equal(true); // recursive — was false, must now be true
    });
  });

  describe("reload (T24 hot-reload bootstrap)", () => {
    it("returns {status:'reloading'} immediately without blocking on the actual reload", async () => {
      installZotero({});
      // ChromeUtils is a Zotero/Gecko-only global; stub it on globalThis
      // so the deferred reload call doesn't crash the test process.
      const reloadStub = sinon.stub().resolves();
      const getAddonStub = sinon.stub().resolves({ reload: reloadStub });
      (globalThis as any).ChromeUtils = {
        importESModule: () => ({ AddonManager: { getAddonByID: getAddonStub } }),
      };

      const { systemHandlers } = await import("../../src/handlers/system");

      const result = await systemHandlers.reload();
      expect(result).to.deep.equal({ status: "reloading" });

      delete (globalThis as any).ChromeUtils;
    });
  });

  describe("libraryStats libraryID → libraryId (fix #40)", () => {
    it("returns libraryId (camelCase) not libraryID", async () => {
      installZotero({
        Libraries: { userLibraryID: 1 },
        Items: { getAll: sinon.stub().resolves([{}, {}, {}]) },
        Collections: { getByLibrary: sinon.stub().returns([{ id: 1 }]) },
      });
      const { systemHandlers } = await import("../../src/handlers/system");
      const result = await systemHandlers.libraryStats({});
      expect(result).to.have.property("libraryId", 1);
      expect(result).to.not.have.property("libraryID");
    });
  });

  describe("currentCollection libraryID → libraryId (fix #41)", () => {
    it("returns key (not id) and libraryId (camelCase) when collection selected", async () => {
      const col = { id: 1, key: "K", name: "Test", libraryID: 1 };
      installZotero({
        getActiveZoteroPane: () => ({ getSelectedCollection: () => col }),
      });
      const { systemHandlers } = await import("../../src/handlers/system");
      const result = await systemHandlers.currentCollection();
      expect(result).to.have.property("key", "K");
      expect(result).to.have.property("libraryId", 1);
      expect(result).to.not.have.property("libraryID");
      expect(result).to.not.have.property("id");
    });
  });
});
