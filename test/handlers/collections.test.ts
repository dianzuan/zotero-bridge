import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero, fakeCollection } from "../fixtures/zotero-mock";

describe("collections handler", () => {
  afterEach(() => resetZotero());

  describe("tree (fix #1)", () => {
    it("calls Zotero.Collections.getByLibrary with recursive=true — NOT false", async () => {
      const root = fakeCollection({ id: 1, name: "Root" });
      const child = fakeCollection({ id: 2, name: "Child", parentID: 1 });
      const getByLibraryStub = sinon.stub().returns([root, child]);

      installZotero({
        Libraries: { userLibraryID: 1 },
        Collections: {
          getByLibrary: getByLibraryStub,
          get: sinon.stub().callsFake((id: number) => {
            if (id === 1) return root;
            if (id === 2) return child;
            return null;
          }),
        },
      });

      delete require.cache[require.resolve("../../src/handlers/collections")];
      const { collectionsHandlers } = await import("../../src/handlers/collections");

      const result = await collectionsHandlers.tree();

      expect(getByLibraryStub.calledOnce).to.equal(true);
      const args = getByLibraryStub.firstCall.args;
      expect(args[0]).to.equal(1);
      expect(args[1]).to.equal(true); // recursive — was false, must now be true
      // Tree should have one root with one child nested
      expect(result).to.have.lengthOf(1);
      expect(result[0].key).to.equal("COL1");
      expect(result[0].children).to.have.lengthOf(1);
      expect(result[0].children[0].key).to.equal("COL2");
    });
  });

  describe("serializeCollection key-first (P2)", () => {
    it("serializeCollection returns key-first, no id, parentKey, children as keys", async () => {
      const parent = fakeCollection({ id: 1, key: "PARENT01", name: "Root" });
      const child = fakeCollection({ id: 2, key: "CHILD001", name: "Sub", parentID: 1 });
      parent.getChildCollections = () => [child];

      installZotero({
        Libraries: { userLibraryID: 1 },
        Collections: {
          getByLibrary: sinon.stub().returns([parent]),
          get: sinon.stub().callsFake((id: number) => {
            if (id === 1) return parent;
            return null;
          }),
        },
      });

      delete require.cache[require.resolve("../../src/handlers/collections")];
      const { collectionsHandlers } = await import("../../src/handlers/collections");
      const result = await collectionsHandlers.list();

      expect(result[0]).to.not.have.property("id");
      expect(result[0].key).to.equal("PARENT01");
      expect(result[0]).to.have.property("version");
      expect(result[0].parentKey).to.equal(null);
      expect(result[0]).to.not.have.property("parentID");
      expect(result[0].childCollections).to.deep.equal(["CHILD001"]);
    });
  });

  describe("removeItems (fix #12)", () => {
    it("throws -32602 when collection is missing — mirroring addItems", async () => {
      installZotero({
        Collections: { getAsync: sinon.stub().resolves(null) }, // missing
        Items: { getAsync: sinon.stub() },
      });

      delete require.cache[require.resolve("../../src/handlers/collections")];
      const { collectionsHandlers } = await import("../../src/handlers/collections");

      try {
        await collectionsHandlers.removeItems({ id: 999, itemIds: [1, 2] });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/collection/i);
        expect(e.message).to.include("999");
      }
    });
  });

  describe("getItems cursor echo (fix #25)", () => {
    it("returns {items, total, offset, limit} echoing the input cursor", async () => {
      const collection: any = {
        id: 1, key: "C1", name: "Test",
        getChildItems: () => Array.from({length: 50}, (_, i) => ({
          id: i, key: `K${i}`, itemType: "journalArticle", itemTypeID: 1,
          dateAdded: "", dateModified: "", deleted: false,
          getField: () => `T${i}`, isNote: () => false, isAttachment: () => false,
          getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        })),
      };
      installZotero({
        Collections: { getAsync: sinon.stub().resolves(collection) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      const { collectionsHandlers } = await import("../../src/handlers/collections");
      const result = await collectionsHandlers.getItems({ id: 1, limit: 10, offset: 5 });

      expect(result).to.have.property("limit", 10);
      expect(result).to.have.property("offset", 5);
      expect(result).to.have.property("total", 50);
      expect(result.items).to.have.lengthOf(10);
      expect(result.items[0].id).to.equal(5); // offset
    });

    it("omits offset/limit when not provided", async () => {
      const collection: any = {
        id: 2, key: "C2", name: "Empty",
        getChildItems: () => [],
      };
      installZotero({
        Collections: { getAsync: sinon.stub().resolves(collection) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });
      const { collectionsHandlers } = await import("../../src/handlers/collections");
      const result = await collectionsHandlers.getItems({ id: 2 });
      expect(result).to.have.property("total", 0);
      expect(result).to.have.property("items").that.deep.equals([]);
      expect(result).to.not.have.property("limit");
      expect(result).to.not.have.property("offset");
    });
  });

  describe("addItems batch helper (fix #53)", () => {
    it("calls col.addItems(itemIDs) inside a transaction instead of N+1 loop", async () => {
      const colAddItemsStub = sinon.stub().resolves();
      const executeTransactionStub = sinon.stub().callsFake(async (cb: () => Promise<void>) => cb());
      const collection: any = {
        id: 1, key: "K", name: "Test",
        addItems: colAddItemsStub,
      };
      installZotero({
        Collections: { getAsync: sinon.stub().resolves(collection) },
        DB: { executeTransaction: executeTransactionStub },
      });
      const { collectionsHandlers } = await import("../../src/handlers/collections");
      const result = await collectionsHandlers.addItems({ id: 1, itemIds: [10, 20, 30] });
      expect(executeTransactionStub.calledOnce).to.equal(true);
      expect(colAddItemsStub.calledOnceWith([10, 20, 30])).to.equal(true);
      expect(result).to.have.property("added");
      expect(result).to.have.property("collectionId", 1);
    });
  });

  describe("removeItems batch helper (fix #54)", () => {
    it("calls col.removeItems(itemIDs) inside a transaction instead of N+1 loop", async () => {
      const colRemoveItemsStub = sinon.stub().resolves();
      const executeTransactionStub = sinon.stub().callsFake(async (cb: () => Promise<void>) => cb());
      const collection: any = {
        id: 1, key: "K", name: "Test",
        removeItems: colRemoveItemsStub,
      };
      installZotero({
        Collections: { getAsync: sinon.stub().resolves(collection) },
        DB: { executeTransaction: executeTransactionStub },
      });
      const { collectionsHandlers } = await import("../../src/handlers/collections");
      const result = await collectionsHandlers.removeItems({ id: 1, itemIds: [10, 20, 30] });
      expect(executeTransactionStub.calledOnce).to.equal(true);
      expect(colRemoveItemsStub.calledOnceWith([10, 20, 30])).to.equal(true);
      expect(result).to.have.property("removed");
      expect(result).to.have.property("collectionId", 1);
    });
  });
});
