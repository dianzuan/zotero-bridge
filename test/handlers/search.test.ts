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

    it("splits whitespace terms into separate fulltext conditions", async () => {
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
      await searchHandlers.fulltext({ query: "数字经济 体育产业" });

      expect(addCondition.calledWith("fulltextContent", "contains", "数字经济")).to.equal(true);
      expect(addCondition.calledWith("fulltextContent", "contains", "体育产业")).to.equal(true);
      expect(addCondition.calledWith("fulltextContent", "contains", "数字经济 体育产业")).to.equal(false);
    });

    it("can limit fulltext results to a collection key", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([5, 6]);
      }
      const collection = {
        id: 2,
        key: "COLLKEY1",
        getChildItems: () => [{ id: 6 }],
      };
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Collections: {
          getByLibraryAndKeyAsync: sinon.stub().withArgs(1, "COLLKEY1").resolves(collection),
        },
        Items: { getAsync: sinon.stub().callsFake((ids: number[]) => Promise.resolve(ids.map(id => ({
          id, key: `K${id}`, itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
          getField: () => "", isNote: () => false, isAttachment: () => false,
          getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        })))) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.fulltext({ query: "数字经济", collection: "COLLKEY1" } as any);

      expect(result.total).to.equal(1);
      expect(result.items.map((item: any) => item.key)).to.deep.equal(["K6"]);
    });

    it("falls back to attachment cache text for collection fulltext misses", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([]);
      }
      const article: any = {
        id: 6, key: "K6", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
        getField: () => "数字经济赋能体育产业",
        isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        getAttachments: () => [60],
      };
      const attachment: any = {
        id: 60,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
      };
      const collection = {
        id: 2,
        key: "COLLKEY1",
        getChildItems: () => [article],
      };
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Collections: {
          getByLibraryAndKeyAsync: sinon.stub().withArgs(1, "COLLKEY1").resolves(collection),
        },
        Items: {
          getAsync: sinon.stub().callsFake((ids: number[] | number) => {
            if (ids === 60) return Promise.resolve(attachment);
            if (Array.isArray(ids)) return Promise.resolve(ids.map(id => article));
            return Promise.resolve(article);
          }),
        },
        Fulltext: { getItemCacheFile: sinon.stub().returns({ path: "/tmp/cache.txt" }) },
        File: { getContentsAsync: sinon.stub().withArgs("/tmp/cache.txt").resolves("数字经济和体育产业都在正文里") },
        DB: { queryAsync: sinon.stub().resolves([{ indexedChars: 10, totalChars: 10 }]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.fulltext({ query: "数字经济 体育产业", collection: "COLLKEY1" } as any);

      expect(result.total).to.equal(1);
      expect(result.items[0].key).to.equal("K6");
    });

    it("continues through empty PDF cache text when falling back", async () => {
      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([]);
      }
      const article: any = {
        id: 6, key: "K6", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
        getField: () => "数字经济赋能体育产业",
        isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        getAttachments: () => [60, 61],
      };
      const attachment = (id: number, path: string) => ({
        id,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
        path,
      });
      const collection = {
        id: 2,
        key: "COLLKEY1",
        getChildItems: () => [article],
      };
      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Collections: {
          getByLibraryAndKeyAsync: sinon.stub().withArgs(1, "COLLKEY1").resolves(collection),
        },
        Items: {
          getAsync: sinon.stub().callsFake((ids: number[] | number) => {
            if (ids === 60) return Promise.resolve(attachment(60, "/tmp/empty-cache.txt"));
            if (ids === 61) return Promise.resolve(attachment(61, "/tmp/match-cache.txt"));
            if (Array.isArray(ids)) return Promise.resolve(ids.map(id => article));
            return Promise.resolve(article);
          }),
        },
        Fulltext: { getItemCacheFile: sinon.stub().callsFake((att: any) => ({ path: att.path })) },
        File: {
          getContentsAsync: sinon.stub().callsFake((path: string) => {
            if (path === "/tmp/empty-cache.txt") return Promise.resolve("");
            if (path === "/tmp/match-cache.txt") return Promise.resolve("数字经济和体育产业都在第二个 PDF 正文里");
            return Promise.resolve("");
          }),
        },
        DB: { queryAsync: sinon.stub().resolves([{ indexedChars: 10, totalChars: 10 }]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { searchHandlers } = await import("../../src/handlers/search");
      const result = await searchHandlers.fulltext({ query: "数字经济 体育产业", collection: "COLLKEY1" } as any);

      expect(result.total).to.equal(1);
      expect(result.items[0].key).to.equal("K6");
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
