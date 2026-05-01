import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero, fakeItem } from "../fixtures/zotero-mock";

describe("notes handler", () => {
  afterEach(() => resetZotero());

  describe("get returns serializeItem array (fix #28)", () => {
    it("returns [(serializeItem)] with note body in `note` field", async () => {
      const parent: any = { id: 1, getNotes: () => [100] };
      const noteItem: any = {
        id: 100, key: "N1", itemType: "note", itemTypeID: 1,
        dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
        getField: () => "",
        isNote: () => true, isAttachment: () => false,
        getNote: () => "<p>Hello note</p>",
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      };
      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(1).resolves(parent);
      getAsyncStub.withArgs([100]).resolves([noteItem]);
      installZotero({
        Items: { getAsync: getAsyncStub },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      const { notesHandlers } = await import("../../src/handlers/notes");
      const result = await notesHandlers.get({ parentId: 1 });

      expect(result).to.have.lengthOf(1);
      expect(result[0].id).to.equal(100);
      expect(result[0].note).to.equal("<p>Hello note</p>");  // from T0 upgrade
      expect(result[0]).to.have.keys("id", "key", "itemType", "title", "dateAdded", "dateModified", "deleted", "note", "creators", "tags", "collections", "relations");
      // Old custom-shape keys gone:
      expect(result[0]).to.not.have.property("content");  // renamed to `note`
    });
  });

  describe("search paginated envelope (fix #29)", () => {
    it("returns {items: (serializeItem)[], total, limit}", async () => {
      const noteItem: any = {
        id: 100, key: "N1", itemType: "note", itemTypeID: 1,
        dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
        getField: () => "",
        isNote: () => true, isAttachment: () => false,
        getNote: () => "<p>The query word matches here</p>",
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      };

      class FakeSearch {
        libraryID: number = 1;
        addCondition = sinon.stub();
        search = sinon.stub().resolves([100]);
      }

      installZotero({
        Libraries: { userLibraryID: 1 },
        Search: FakeSearch,
        Items: { getAsync: sinon.stub().withArgs([100]).resolves([noteItem]) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      const { notesHandlers } = await import("../../src/handlers/notes");
      const result = await notesHandlers.search({ query: "query word", limit: 10 });

      expect(result).to.have.keys("items", "total", "limit");
      expect(result.limit).to.equal(10);
      expect(result.total).to.equal(1);
      expect(result.items).to.have.lengthOf(1);
      expect(result.items[0].note).to.contain("query word");
    });
  });

  describe("update returns serializeItem (fix #30)", () => {
    it("returns the updated note as (serializeItem)", async () => {
      const noteItem: any = {
        id: 100, key: "N1", itemType: "note", itemTypeID: 1,
        dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
        getField: () => "",
        isNote: () => true, isAttachment: () => false,
        getNote: () => "<p>Updated body</p>",
        setNote: sinon.stub(),
        saveTx: sinon.stub().resolves(),
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(100).resolves(noteItem) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      const { notesHandlers } = await import("../../src/handlers/notes");
      const result = await notesHandlers.update({ id: 100, content: "<p>Updated body</p>" });

      expect(result.id).to.equal(100);
      expect(result.note).to.equal("<p>Updated body</p>");
      expect(result).to.not.have.property("updated");  // old shape's flag is gone
    });
  });

});
