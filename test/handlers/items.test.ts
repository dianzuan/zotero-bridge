import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero, fakeItem } from "../fixtures/zotero-mock";

describe("items handler", () => {
  beforeEach(() => {
    // Clear the require cache before each test to force re-evaluation
    // of the handler module with fresh Zotero stubs
    delete require.cache[require.resolve("../../src/handlers/items")];
  });

  afterEach(() => resetZotero());

  describe("addByURL processDocuments (fix #4)", () => {
    it("destructures the first document from the returned array — NOT passes the array", async () => {
      const fakeDoc = { url: "https://example.com/paper" };
      const processDocsStub = sinon.stub().resolves([fakeDoc]); // returns array per Zotero docs
      const setDocStub = sinon.stub();
      const translateStub = sinon.stub().resolves();
      const getTranslatorsStub = sinon.stub().resolves([]);
      const setHandlerStub = sinon.stub();

      class FakeWebTranslate {
        setDocument = setDocStub;
        setHandler = setHandlerStub;
        getTranslators = getTranslatorsStub;
        translate = translateStub;
      }

      installZotero({
        HTTP: { processDocuments: processDocsStub },
        Translate: { Web: FakeWebTranslate },
        Libraries: { userLibraryID: 1 },
        Items: { getAsync: sinon.stub().resolves([]) },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");

      // The handler likely throws or returns empty when no translators match;
      // we don't care — we only care that setDocument received `fakeDoc` not `[fakeDoc]`.
      try {
        await itemsHandlers.addByURL({ url: "https://example.com/paper" });
      } catch {
        /* ignore */
      }

      expect(setDocStub.called).to.equal(true);
      expect(setDocStub.firstCall.args[0]).to.equal(fakeDoc); // NOT [fakeDoc]
    });
  });

  describe("findDuplicates (fix #5)", () => {
    it("returns proper groups via per-itemID enumeration — NOT empty arrays", async () => {
      // Simulate a duplicate set: items 1, 2 are dupes; 3 alone; 4, 5, 6 are dupes.
      const sets: Record<number, number[]> = {
        1: [1, 2], 2: [1, 2],
        3: [3],
        4: [4, 5, 6], 5: [4, 5, 6], 6: [4, 5, 6],
      };

      const searchStub = { search: sinon.stub().resolves([1, 2, 3, 4, 5, 6]) };
      const getSearchObjectStub = sinon.stub().resolves(searchStub);
      const getSetItemsByItemIDStub = sinon.stub().callsFake((id: number) => sets[id]);

      installZotero({
        Libraries: { userLibraryID: 1 },
        Duplicates: function (libraryID: number) {
          return {
            getSearchObject: getSearchObjectStub,
            getSetItemsByItemID: getSetItemsByItemIDStub,
          };
        } as any,
      });

      delete require.cache[require.resolve("../../src/handlers/items")];
      const { itemsHandlers } = await import("../../src/handlers/items");

      const result = await itemsHandlers.findDuplicates();

      // Expected: 2 groups (the [1,2] pair and the [4,5,6] triple); item 3 is solo.
      // The PRD §3.1 keeps findDuplicates' shape as {groups, totalGroups}.
      expect(result.totalGroups).to.equal(2);
      expect(result.groups).to.have.lengthOf(2);
      const sorted = result.groups.map((g: number[]) => [...g].sort((a, b) => a - b));
      expect(sorted).to.deep.include([1, 2]);
      expect(sorted).to.deep.include([4, 5, 6]);
    });
  });

  describe("batchTrash (fix #11)", () => {
    it("returns {ok, keys, count} so callers keep the key trail", async () => {
      const items = [1, 2, 3].map((id) => ({
        id, key: `K${id}`, deleted: false, save: sinon.stub().resolves(),
      }));
      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(1).resolves(items[0]);
      getAsyncStub.withArgs(2).resolves(items[1]);
      getAsyncStub.withArgs(3).resolves(items[2]);
      installZotero({
        Items: { getAsync: getAsyncStub },
        DB: { executeTransaction: sinon.stub().callsFake((cb: () => Promise<void>) => cb()) },
      });

      delete require.cache[require.resolve("../../src/handlers/items")];
      delete require.cache[require.resolve("../../src/utils/guards")];
      const { itemsHandlers } = await import("../../src/handlers/items");

      const result = await itemsHandlers.batchTrash({ ids: [1, 2, 3] });
      expect(result.ok).to.equal(true);
      expect(result.count).to.equal(3);
      expect(result.keys).to.have.lengthOf(3);
      // Sanity: every save was called
      items.forEach((it) => expect(it.save.calledOnce).to.equal(true));
    });
  });

  describe("getRecent paginated envelope (fix #22 + #59)", () => {
    it("returns {items, total, limit} when limit provided", async () => {
      const itemIDs = [1, 2];
      const items = [
        { id: 1, key: "A", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
          getField: () => "T1", isNote: () => false, isAttachment: () => false,
          getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}) },
        { id: 2, key: "B", itemType: "journalArticle", itemTypeID: 1, dateAdded: "", dateModified: "", deleted: false,
          getField: () => "T2", isNote: () => false, isAttachment: () => false,
          getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}) },
      ];

      installZotero({
        Libraries: { userLibraryID: 1 },
        DB: { columnQueryAsync: sinon.stub().resolves(itemIDs) },
        Items: { getAsync: sinon.stub().withArgs(itemIDs).resolves(items) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.getRecent({ limit: 2 });

      expect(result).to.have.keys("items", "total", "limit");
      expect(result.limit).to.equal(2);
      expect(result.total).to.equal(2);
      expect(result.items).to.have.lengthOf(2);
      expect(result.items[0].key).to.equal("A");
    });
  });

  describe("addRelated key echo (fix #23)", () => {
    it("returns {ok: true, key} echoing the item key", async () => {
      const sourceItem: any = {
        id: 10, key: "S", addRelatedItem: sinon.stub().returns(true), saveTx: sinon.stub().resolves(),
      };
      const targetItem: any = { id: 20, key: "T", addRelatedItem: sinon.stub().returns(true), saveTx: sinon.stub().resolves() };
      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(10).resolves(sourceItem);
      getAsyncStub.withArgs(20).resolves(targetItem);
      installZotero({ Items: { getAsync: getAsyncStub } });

      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.addRelated({ id: 10, relatedId: 20 });
      expect(result).to.have.property("key", "S");
      expect(result.ok).to.equal(true);
    });
  });

  describe("removeRelated key echo (fix #24)", () => {
    it("returns {ok: true, key} echoing the item key", async () => {
      const sourceItem: any = {
        id: 10, key: "S", removeRelatedItem: sinon.stub().returns(true), saveTx: sinon.stub().resolves(),
      };
      const targetItem: any = { id: 20, key: "T", removeRelatedItem: sinon.stub().returns(true), saveTx: sinon.stub().resolves() };
      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(10).resolves(sourceItem);
      getAsyncStub.withArgs(20).resolves(targetItem);
      installZotero({ Items: { getAsync: getAsyncStub } });

      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.removeRelated({ id: 10, relatedId: 20 });
      expect(result).to.have.property("key", "S");
      expect(result.ok).to.equal(true);
    });
  });

  describe("addBy* translate `collections` option (fixes #55, #56, #57)", () => {
    it("addByURL passes collections option when params.collection provided", async () => {
      const fakeDoc = { url: "https://example.com" };
      const processDocsStub = sinon.stub().resolves([fakeDoc]);
      const fakeItem = {
        id: 99, key: "K99", itemType: "journalArticle", itemTypeID: 1,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "", isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        addToCollection: sinon.stub(), saveTx: sinon.stub().resolves(),
      };
      const translateStub = sinon.stub().resolves();
      const setHandlerStub = sinon.stub();

      class FakeWebTranslate {
        setDocument = sinon.stub();
        setHandler = (eventName: string, cb: (obj: any, items: any[]) => void) => {
          setHandlerStub(eventName, cb);
          if (eventName === "itemDone") {
            // Stash callback for later invocation
            (this as any)._itemDoneCb = cb;
          }
        };
        getTranslators = sinon.stub().resolves([{ label: "Fake" }]);
        setTranslator = sinon.stub();
        translate = (opts: any) => {
          translateStub(opts);
          // Fire itemDone for each (none here — empty translate)
          return Promise.resolve();
        };
      }

      installZotero({
        HTTP: { processDocuments: processDocsStub },
        Translate: { Web: FakeWebTranslate },
        Libraries: { userLibraryID: 1 },
        Items: { getAsync: sinon.stub().resolves([fakeItem]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");
      try {
        await itemsHandlers.addByURL({ url: "https://example.com", collection: 42 });
      } catch { /* fine — no items returned, but we only care about translate args */ }

      // The fix: translate is called with {libraryID, collections: [42]} — NOT just {libraryID}
      expect(translateStub.calledOnce).to.equal(true);
      const translateOpts = translateStub.firstCall.args[0];
      expect(translateOpts).to.have.property("collections");
      expect(translateOpts.collections).to.deep.equal([42]);
    });

    it("addByURL omits collections option when params.collection NOT provided", async () => {
      const fakeDoc = { url: "https://example.com" };
      const processDocsStub = sinon.stub().resolves([fakeDoc]);
      const translateStub = sinon.stub().resolves();
      class FakeWebTranslate {
        setDocument = sinon.stub();
        setHandler = sinon.stub();
        getTranslators = sinon.stub().resolves([{ label: "Fake" }]);
        setTranslator = sinon.stub();
        translate = (opts: any) => { translateStub(opts); return Promise.resolve(); };
      }
      installZotero({
        HTTP: { processDocuments: processDocsStub },
        Translate: { Web: FakeWebTranslate },
        Libraries: { userLibraryID: 1 },
        Items: { getAsync: sinon.stub().resolves([]) },
      });
      const { itemsHandlers } = await import("../../src/handlers/items");
      try { await itemsHandlers.addByURL({ url: "https://example.com" }); } catch {}
      const translateOpts = translateStub.firstCall.args[0];
      expect(translateOpts).to.not.have.property("collections");
    });
  });

  describe("getTrash uses getDeleted (fix #58)", () => {
    it("calls Zotero.Items.getDeleted instead of getAll+filter", async () => {
      const trashedIDs = [101, 102, 103];
      const trashedItems = trashedIDs.map((id) => ({
        id, key: `K${id}`, version: 1, itemType: "journalArticle", itemTypeID: 1,
        dateAdded: "", dateModified: "", deleted: true,
        getField: () => "", isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      }));
      const getDeletedStub = sinon.stub().resolves(trashedIDs);
      const getAsyncStub = sinon.stub().callsFake((ids: any) => Promise.resolve(trashedItems));
      installZotero({
        Libraries: { userLibraryID: 1 },
        Items: {
          getDeleted: getDeletedStub,
          getAsync: getAsyncStub,
          // Intentionally omit getAll — if the handler still calls it, the test fails on undefined
        },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });
      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.getTrash({ limit: 10 });
      expect(getDeletedStub.calledOnce).to.equal(true);
      // First arg = libraryID, second arg = asIDs (true)
      const [libArg, asIDsArg] = getDeletedStub.firstCall.args;
      expect(libArg).to.equal(1);
      expect(asIDsArg).to.equal(true);
      expect(result.items).to.have.lengthOf(3);
      expect(result.total).to.equal(3);
    });
  });

  describe("getRecent uses DB.columnQueryAsync (fix #59)", () => {
    it("queries DB directly for top-N IDs sorted by dateAdded — NOT full library scan", async () => {
      const sortedIDs = [203, 202, 201]; // most recent first (DB returned them sorted)
      const items = sortedIDs.map((id) => ({
        id, key: `K${id}`, itemType: "journalArticle", itemTypeID: 1,
        dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
        getField: () => "", isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      }));
      const queryStub = sinon.stub().resolves(sortedIDs);

      installZotero({
        Libraries: { userLibraryID: 1 },
        DB: { columnQueryAsync: queryStub },
        Items: {
          getAll: sinon.stub().rejects(new Error("getAll should NOT be called")),
          getAsync: sinon.stub().withArgs(sortedIDs).resolves(items),
        },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.getRecent({ limit: 3, type: "added" });

      expect(queryStub.calledOnce).to.equal(true);
      const [sql, sqlParams] = queryStub.firstCall.args;
      expect(sql).to.match(/ORDER BY dateAdded DESC/i);
      expect(sql).to.match(/LIMIT/i);
      expect(sqlParams).to.deep.equal([1, 3, 0]);  // [libraryID, limit, offset]
      expect(result).to.have.keys("items", "total", "limit");
      expect(result.items).to.have.lengthOf(3);
      expect(result.items[0].key).to.equal("K203"); // most recent
    });

    it("passes offset to SQL via OFFSET clause and echoes it in result", async () => {
      const sortedIDs = [1, 2, 3];
      const items = sortedIDs.map((id) => ({
        id, key: `K${id}`, itemType: "journalArticle", itemTypeID: 1,
        dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
        getField: () => "", isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      }));
      const queryStub = sinon.stub().resolves(sortedIDs);
      installZotero({
        Libraries: { userLibraryID: 1 },
        DB: { columnQueryAsync: queryStub },
        Items: { getAsync: sinon.stub().withArgs(sortedIDs).resolves(items) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.getRecent({ limit: 5, offset: 100 });

      const [sql, sqlParams] = queryStub.firstCall.args;
      expect(sql).to.match(/OFFSET/i);
      expect(sqlParams).to.deep.equal([1, 5, 100]); // [libraryID, limit, offset]
      expect(result).to.have.property("offset", 100);
    });

    it("uses dateModified sort when type is 'modified'", async () => {
      const queryStub = sinon.stub().resolves([301]);
      installZotero({
        Libraries: { userLibraryID: 1 },
        DB: { columnQueryAsync: queryStub },
        Items: {
          getAsync: sinon.stub().resolves([{
            id: 301, key: "K301", itemType: "journalArticle", itemTypeID: 1,
            dateAdded: "2026-01-01", dateModified: "2026-01-02", deleted: false,
            getField: () => "", isNote: () => false, isAttachment: () => false,
            getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
          }]),
        },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");
      await itemsHandlers.getRecent({ limit: 1, type: "modified" });

      const [sql] = queryStub.firstCall.args;
      expect(sql).to.match(/ORDER BY dateModified DESC/i);
    });
  });

  describe("batchTrash accepts string keys (Task 5)", () => {
    it("resolves string keys via getByLibraryAndKeyAsync and trashes the items", async () => {
      const item1 = {
        id: 1, key: "KEY00001", deleted: false, save: sinon.stub().resolves(),
      };
      const item2 = {
        id: 2, key: "KEY00002", deleted: false, save: sinon.stub().resolves(),
      };
      const getByKeyStub = sinon.stub();
      getByKeyStub.withArgs(1, "KEY00001").resolves(item1);
      getByKeyStub.withArgs(1, "KEY00002").resolves(item2);
      installZotero({
        Items: {
          getAsync: sinon.stub().resolves(null),
          getByLibraryAndKeyAsync: getByKeyStub,
        },
        Libraries: { userLibraryID: 1 },
        DB: { executeTransaction: sinon.stub().callsFake(async (fn: any) => fn()) },
        Collections: { get: () => null },
      });
      delete require.cache[require.resolve("../../src/handlers/items")];
      delete require.cache[require.resolve("../../src/utils/guards")];
      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.batchTrash({ ids: ["KEY00001", "KEY00002"] });
      expect(result.ok).to.equal(true);
      expect(result.count).to.equal(2);
      expect(result.keys).to.deep.equal(["KEY00001", "KEY00002"]);
      expect(item1.deleted).to.equal(true);
      expect(item2.deleted).to.equal(true);
    });
  });

  describe("citationKey on items namespace (fix #52 relocation)", () => {
    it("returns {citationKey, id} when called as items.citationKey", async () => {
      const item: any = {
        id: 5, key: "K5",
        getField: (n: string) => {
          if (n === "title") return "An Article";
          if (n === "date") return "2023-05-12";
          if (n === "extra") return "";
          return "";
        },
        getCreators: () => [{ lastName: "Smith", firstName: "J", creatorTypeID: 1 }],
        isNote: () => false, isAttachment: () => false,
      };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(item) },
        Date: { strToDate: sinon.stub().returns({ year: 2023 }) },
        CreatorTypes: { getName: () => "author" },
      });

      const { itemsHandlers } = await import("../../src/handlers/items");
      const result = await itemsHandlers.citationKey({ id: 5 });

      expect(result).to.have.property("key", "K5");
      expect(result).to.have.property("citationKey");
      expect(result.citationKey).to.be.a("string");
      // The citation key should include something derived from the item — be lenient on exact format
      expect(result.citationKey.length).to.be.greaterThan(0);
    });
  });
});
