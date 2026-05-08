import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("export handler", () => {
  beforeEach(() => {
    delete require.cache[require.resolve("../../src/handlers/export")];
  });

  afterEach(() => resetZotero());

  describe("bibtex (fix #14)", () => {
    it("throws structured {code:-32603, message} when translator fails", async () => {
      // Zotero.Translate.Export constructor — the handler calls:
      //   const translate = new Zotero.Translate.Export();
      //   translate.setItems(items);
      //   translate.setTranslator(translatorID);
      //   translate.setHandler("done", cb);
      //   translate.translate();
      // We simulate a failure by firing the "done" handler with status=false.

      const handlers: Record<string, Function> = {};
      const FakeExport = function () {};
      FakeExport.prototype.setItems = sinon.stub();
      FakeExport.prototype.setTranslator = sinon.stub();
      FakeExport.prototype.setHandler = function (event: string, cb: Function) {
        handlers[event] = cb;
      };
      FakeExport.prototype.translate = function () {
        // Fire the done callback synchronously with failure status
        handlers["done"]?.(null, false);
      };

      installZotero({
        Items: { getAsync: sinon.stub().resolves([{ id: 1 }]) },
        Translate: { Export: FakeExport },
      });

      const { exportHandlers } = await import("../../src/handlers/export");

      try {
        await exportHandlers.bibtex({ keys: [1] });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32603);
        expect(e.message).to.include("bibtex");
        expect(e.message).to.include("translator returned failure status");
      }
    });
  });

  describe("bibliography key-first export", () => {
    it("resolves item keys before passing numeric IDs to citeproc", async () => {
      const updateItems = sinon.stub();
      const makeBibliography = sinon.stub().returns([{}, ["Rendered bibliography"]]);
      const engine = {
        setOutputFormat: sinon.stub(),
        updateItems,
        makeBibliography,
        free: sinon.stub(),
      };
      const item = { id: 123, key: "ITEMKEY" };

      installZotero({
        Libraries: { userLibraryID: 1 },
        Items: {
          getByLibraryAndKeyAsync: sinon.stub().withArgs(1, "ITEMKEY").resolves(item),
        },
        Styles: {
          get: sinon.stub().withArgs("http://www.zotero.org/styles/apa").returns({
            getCiteProc: () => engine,
          }),
        },
      });

      const { exportHandlers } = await import("../../src/handlers/export");
      const result = await exportHandlers.bibliography({
        keys: ["ITEMKEY"],
        style: "http://www.zotero.org/styles/apa",
      });

      expect(updateItems.calledTwice).to.equal(true);
      expect(updateItems.alwaysCalledWith([123])).to.equal(true);
      expect(result.text).to.equal("Rendered bibliography");
      expect(result.count).to.equal(1);
    });
  });

  describe("cslJson", () => {
    it("parses translator JSON into a JSON value instead of returning repr text", async () => {
      const handlers: Record<string, Function> = {};
      const FakeExport = function (this: any) {
        this.string = "";
      };
      FakeExport.prototype.setItems = sinon.stub();
      FakeExport.prototype.setTranslator = sinon.stub();
      FakeExport.prototype.setHandler = function (event: string, cb: Function) {
        handlers[event] = cb;
      };
      FakeExport.prototype.translate = function (this: any) {
        this.string = JSON.stringify([{ id: "ITEM1", title: "Paper" }]);
        handlers["done"]?.(null, true);
      };

      installZotero({
        Items: { getAsync: sinon.stub().resolves([{ id: 1 }]) },
        Translate: { Export: FakeExport },
      });

      const { exportHandlers } = await import("../../src/handlers/export");
      const result = await exportHandlers.cslJson({ keys: [1] });

      expect(result.content).to.deep.equal([{ id: "ITEM1", title: "Paper" }]);
    });
  });
});
