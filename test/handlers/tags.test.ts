import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("tags handler", () => {
  afterEach(() => resetZotero());

  describe("list (fix #18)", () => {
    it("emits type: 0 for manual tags where Zotero returns type: undefined", async () => {
      const tags = [
        { tag: "manual-tag", type: undefined },     // manual
        { tag: "auto-tag", type: 1 },               // automatic
      ];
      installZotero({
        Libraries: { userLibraryID: 1 },
        Tags: { getAll: sinon.stub().resolves(tags) },
      });

      delete require.cache[require.resolve("../../src/handlers/tags")];
      const { tagsHandlers } = await import("../../src/handlers/tags");

      const result = await tagsHandlers.list({});

      expect(result).to.deep.equal({
        items: [
          { tag: "manual-tag", type: 0 },
          { tag: "auto-tag", type: 1 },
        ],
        total: 2,
        limit: 200,
        offset: 0,
      });
    });
  });

  describe("tags family accepts libraryId? (fixes #42, #43, #44)", () => {
    it("list passes provided libraryId to Tags.getAll", async () => {
      const getAllStub = sinon.stub().resolves([]);
      installZotero({
        Libraries: { userLibraryID: 1 },
        Tags: { getAll: getAllStub },
      });
      delete require.cache[require.resolve("../../src/handlers/tags")];
      const { tagsHandlers } = await import("../../src/handlers/tags");
      await tagsHandlers.list({ libraryId: 5 });
      expect(getAllStub.firstCall.args[0]).to.equal(5);
    });

    it("list defaults to userLibraryID when libraryId omitted", async () => {
      const getAllStub = sinon.stub().resolves([]);
      installZotero({
        Libraries: { userLibraryID: 1 },
        Tags: { getAll: getAllStub },
      });
      delete require.cache[require.resolve("../../src/handlers/tags")];
      const { tagsHandlers } = await import("../../src/handlers/tags");
      await tagsHandlers.list({});
      expect(getAllStub.firstCall.args[0]).to.equal(1);
    });

    it("rename passes libraryId to Tags.rename", async () => {
      const renameStub = sinon.stub().resolves(true);
      installZotero({
        Libraries: { userLibraryID: 1 },
        Tags: { rename: renameStub },
      });
      delete require.cache[require.resolve("../../src/handlers/tags")];
      const { tagsHandlers } = await import("../../src/handlers/tags");
      await tagsHandlers.rename({ oldName: "old", newName: "new", libraryId: 5 });
      expect(renameStub.firstCall.args[0]).to.equal(5);
    });

    it("delete passes libraryId to Tags.removeFromLibrary", async () => {
      const removeStub = sinon.stub().resolves(true);
      installZotero({
        Libraries: { userLibraryID: 1 },
        Tags: { removeFromLibrary: removeStub, getID: sinon.stub().returns(99) },
      });
      delete require.cache[require.resolve("../../src/handlers/tags")];
      const { tagsHandlers } = await import("../../src/handlers/tags");
      await tagsHandlers.delete({ tag: "X", libraryId: 5 });
      expect(removeStub.firstCall.args[0]).to.equal(5);
    });
  });

  describe("list honors offset (pagination fix)", () => {
    it("skips first N tags when offset is provided", async () => {
      const allTags = [
        { tag: "a", type: 1 }, { tag: "b", type: 1 }, { tag: "c", type: 1 },
        { tag: "d", type: 1 }, { tag: "e", type: 1 },
      ];
      installZotero({
        Libraries: { userLibraryID: 1 },
        Tags: { getAll: sinon.stub().resolves(allTags) },
      });
      delete require.cache[require.resolve("../../src/handlers/tags")];
      const { tagsHandlers } = await import("../../src/handlers/tags");
      const result = await tagsHandlers.list({ offset: 2, limit: 2 });
      expect(result).to.deep.equal({
        items: [
          { tag: "c", type: 1 },
          { tag: "d", type: 1 },
        ],
        total: 5,
        limit: 2,
        offset: 2,
      });
    });
  });
});
