import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("search handler", () => {
  beforeEach(() => {
    // Clear the require cache before each test to force re-evaluation
    // of the handler module with fresh Zotero stubs
    delete require.cache[require.resolve("../../src/handlers/search")];
  });

  afterEach(() => resetZotero());

  describe("savedSearches (fix #10)", () => {
    it("calls Zotero.Searches.getAll(libraryID) — NOT getByLibrary which is cold-cache buggy", async () => {
      const search = {
        id: 1,
        key: "S1",
        name: "My Search",
        getConditions: () => [{ condition: "title", operator: "contains", value: "x" }],
      };
      const getAllStub = sinon.stub().resolves([search]);

      installZotero({
        Libraries: { userLibraryID: 1 },
        Searches: { getAll: getAllStub },
      });

      const { searchHandlers } = await import("../../src/handlers/search");

      const result = await searchHandlers.savedSearches();

      expect(getAllStub.calledOnceWith(1)).to.equal(true);
      expect(result).to.have.lengthOf(1);
      expect(result[0]).to.not.have.property("id");
      expect(result[0].key).to.equal("S1");
      expect(result[0].name).to.equal("My Search");
    });
  });

  describe("quick drops query echo (fix #32)", () => {
    it("returns {items, total, limit?} without query field", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([1, 2]);
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().resolves([
          { id: 1, key: "K1", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
            getField: () => "", isNote: () => false, isAttachment: () => false,
            getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}) },
          { id: 2, key: "K2", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
            getField: () => "", isNote: () => false, isAttachment: () => false,
            getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}) },
        ]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.quick({ query: "echo this", limit: 10 });
      expect(result).to.not.have.property("query");
      expect(result.total).to.equal(2);
      expect(result.items).to.have.lengthOf(2);
    });
  });

  describe("fulltext drops query echo (fix #33)", () => {
    it("returns {items, total, limit?} without query field", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([5]);
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().resolves([
          { id: 5, key: "K5", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
            getField: () => "", isNote: () => false, isAttachment: () => false,
            getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}) },
        ]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.fulltext({ query: "fulltext q" });
      expect(result).to.not.have.property("query");
      expect(result.items).to.have.lengthOf(1);
    });
  });

  describe("byTag drops tag echo (fix #34)", () => {
    it("returns {items, total, limit?} without tag field", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([7]);
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().resolves([
          { id: 7, key: "K7", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
            getField: () => "", isNote: () => false, isAttachment: () => false,
            getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}) },
        ]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.byTag({ tag: "bookmark" });
      expect(result).to.not.have.property("tag");
      expect(result.items).to.have.lengthOf(1);
    });
  });

  describe("advanced accepts limit (fix #35)", () => {
    it("slices results by limit", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([1, 2, 3, 4, 5]);
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().callsFake((ids: number[]) => Promise.resolve(ids.map(id => ({
          id, key: `K${id}`, itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
          getField: () => "", isNote: () => false, isAttachment: () => false,
          getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        })))) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.advanced({
        conditions: [{ field: "title", op: "contains", value: "x" }],
        limit: 2,
      });
      expect(result.items).to.have.lengthOf(2);
      expect(result.total).to.equal(5);
      expect(result.limit).to.equal(2);
    });

    it("accepts operator alias from Rust CLI conditions", async () => {
      const addCondition = sinon.stub();
      class FakeSearch {
        libraryID: number = 1;
        addCondition = addCondition;
        search = sinon.stub().resolves([]);
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().resolves([]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      await searchHandlers.advanced({
        conditions: [{ field: "title", operator: "contains", value: "数字经济" }],
        limit: 2,
      });
      expect(addCondition.calledWith("title", "contains", "数字经济")).to.equal(true);
    });
  });

  describe("byIdentifier accepts limit (fix #36)", () => {
    it("slices results by limit", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([1, 2, 3]);
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().callsFake((ids: number[]) => Promise.resolve(ids.map(id => ({
          id, key: `K${id}`, itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
          getField: () => "", isNote: () => false, isAttachment: () => false,
          getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        })))) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.byIdentifier({ doi: "10.1/x", limit: 1 });
      expect(result.items).to.have.lengthOf(1);
      expect(result.limit).to.equal(1);
    });
  });

  describe("createSavedSearch returns key (fix #37)", () => {
    it("returns {ok, key, name} without id", async () => {
      const fakeSavedSearch: any = {
        id: 42, key: "SS42",
        addCondition: sinon.stub(),
        saveTx: sinon.stub().resolves(),
        libraryID: 1,
      };
      class FakeSearch {
        constructor() { Object.assign(this, fakeSavedSearch); return fakeSavedSearch; }
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.createSavedSearch({
        name: "My Saved", conditions: [{ field: "title", op: "contains", value: "x" }],
      });
      expect(fakeSavedSearch.addCondition.calledWith("title", "contains", "x")).to.equal(true);
      expect(result).to.not.have.property("id");
      expect(result).to.have.property("ok", true);
      expect(result).to.have.property("key", "SS42");
      expect(result).to.have.property("name", "My Saved");
    });

    it("accepts operator alias from Rust CLI conditions", async () => {
      const fakeSavedSearch: any = {
        id: 43, key: "SS43",
        addCondition: sinon.stub(),
        saveTx: sinon.stub().resolves(),
        libraryID: 1,
      };
      class FakeSearch {
        constructor() { Object.assign(this, fakeSavedSearch); return fakeSavedSearch; }
      }
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      await searchHandlers.createSavedSearch({
        name: "My Saved",
        conditions: [{ field: "title", operator: "contains", value: "数字经济" }],
      });
      expect(fakeSavedSearch.addCondition.calledWith("title", "contains", "数字经济")).to.equal(true);
    });
  });
});
