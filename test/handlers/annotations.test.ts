import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("annotations handler", () => {
  beforeEach(() => {
    delete require.cache[require.resolve("../../src/handlers/annotations")];
  });

  afterEach(() => resetZotero());

  describe("list", () => {
    it("returns serialized annotations with annotation-specific fields", async () => {
      const parent: any = {
        id: 1,
        getAnnotations: () => [10, 11],
      };
      const ann1: any = {
        id: 10, key: "ANN10", itemType: "annotation", itemTypeID: 99,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "",
        isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        annotationType: "highlight",
        annotationText: "important text",
        annotationComment: "my comment",
        annotationColor: "#ffd400",
        annotationPosition: JSON.stringify({ pageIndex: 0, rects: [[0, 0, 100, 100]] }),
      };
      const ann2: any = {
        id: 11, key: "ANN11", itemType: "annotation", itemTypeID: 99,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "",
        isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        annotationType: "note",
        annotationText: "",
        annotationComment: "standalone note",
        annotationColor: "#ff0000",
        annotationPosition: null,
      };

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(1).resolves(parent);
      getAsyncStub.withArgs([10, 11]).resolves([ann1, ann2]);

      installZotero({
        Items: { getAsync: getAsyncStub },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.list({ parentKey: 1 });

      expect(result).to.have.lengthOf(2);
      expect(result[0].annotationType).to.equal("highlight");
      expect(result[0].annotationText).to.equal("important text");
      expect(result[0].annotationComment).to.equal("my comment");
      expect(result[0].annotationColor).to.equal("#ffd400");
      expect(result[0].annotationPosition).to.deep.equal({ pageIndex: 0, rects: [[0, 0, 100, 100]] });
      expect(result[1].annotationType).to.equal("note");
      expect(result[1].annotationPosition).to.be.null;
    });

    it("returns empty array when item has no annotations", async () => {
      const parent: any = { id: 2, getAnnotations: () => [] };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(2).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.list({ parentKey: 2 });

      expect(result).to.deep.equal([]);
    });
  });

  describe("create", () => {
    it("creates an annotation and returns {ok, key}", async () => {
      const parent: any = { id: 5, libraryID: 1 };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        Item: function (itemType: string) {
          createdItem = {
            itemType,
            libraryID: 0,
            parentID: 0,
            key: "NEWANN01",
            saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      // Zotero.Item constructor is used via `new Zotero.Item("annotation")`
      (globalThis as any).Zotero.Item = (globalThis as any).Zotero.Item;

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5,
        type: "highlight",
        text: "selected text",
        comment: "my note",
        color: "#ffd400",
        position: { pageIndex: 1, rects: [[10, 20, 30, 40]] },
      });

      expect(result.ok).to.equal(true);
      expect(result.key).to.equal("NEWANN01");
      expect(saveTxStub.calledOnce).to.equal(true);
      expect(createdItem.libraryID).to.equal(1);
      expect(createdItem.parentID).to.equal(5);
      expect(createdItem.annotationType).to.equal("highlight");
      expect(createdItem.annotationText).to.equal("selected text");
      expect(createdItem.annotationComment).to.equal("my note");
      expect(createdItem.annotationColor).to.equal("#ffd400");
      expect(createdItem.annotationSortIndex).to.equal(0);
      expect(JSON.parse(createdItem.annotationPosition)).to.deep.equal({
        pageIndex: 1, rects: [[10, 20, 30, 40]],
      });
    });

    it("rejects invalid annotation type with -32602", async () => {
      const parent: any = { id: 5, libraryID: 1 };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "invalid_type",
          position: {},
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/Invalid annotation type/);
      }
    });
  });

  describe("delete", () => {
    it("erases the annotation item and returns {ok, key}", async () => {
      const eraseTxStub = sinon.stub().resolves();
      const ann: any = { id: 77, key: "ANN77", eraseTx: eraseTxStub };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(77).resolves(ann) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.delete({ key: 77 });

      expect(result).to.deep.equal({ ok: true, key: "ANN77" });
      expect(eraseTxStub.calledOnce).to.equal(true);
    });
  });
});
