import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero, fakeItem } from "../fixtures/zotero-mock";

describe("notes handler", () => {
  afterEach(() => resetZotero());

  describe("getAnnotations (fixes #6 + #7)", () => {
    it("calls pdfItem.getAnnotations(false, true) for IDs — NOT no-args (which returns Items and breaks)", async () => {
      const annotationIDs = [101, 102, 103];
      const annotationItems = annotationIDs.map((id) => ({
        id,
        key: `ANN${id}`,
        annotationType: "highlight",
        annotationText: `text${id}`,
        annotationComment: "",
        annotationColor: "#ffd400",
        annotationPageLabel: "1",
        annotationPosition: JSON.stringify({}),
        getTags: () => [],
        dateAdded: "2026-01-01T00:00:00Z",
      }));

      const getAnnotationsStub = sinon.stub().returns(annotationIDs);
      const pdfChild = {
        id: 200,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
        getAnnotations: getAnnotationsStub,
      };
      const parent = fakeItem({ id: 1 });
      (parent as any).getAttachments = () => [200];

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(1).resolves(parent);
      getAsyncStub.withArgs([200]).resolves([pdfChild]);
      getAsyncStub.withArgs(annotationIDs).resolves(annotationItems);

      installZotero({
        Items: { getAsync: getAsyncStub },
      });

      delete require.cache[require.resolve("../../src/handlers/notes")];
      const { notesHandlers } = await import("../../src/handlers/notes");

      const result = await notesHandlers.getAnnotations({ parentId: 1 });

      // The fix: call getAnnotations(false, true) — second arg `asIDs=true`.
      expect(getAnnotationsStub.calledOnce).to.equal(true);
      const args = getAnnotationsStub.firstCall.args;
      expect(args[1]).to.equal(true); // asIDs
      // Result should have 3 annotations.
      expect(result).to.have.lengthOf(3);
      expect(result[0].id).to.equal(101);
    });

    it("accepts Zotero 8 object results from getAnnotations", async () => {
      const annotationItems = [101, 102].map((id) => ({
        id,
        key: `ANN${id}`,
        annotationType: "highlight",
        annotationText: `text${id}`,
        annotationComment: "",
        annotationColor: "#ffd400",
        annotationPageLabel: "1",
        annotationPosition: JSON.stringify({}),
        getTags: () => [],
        dateAdded: "2026-01-01T00:00:00Z",
      }));

      const getAnnotationsStub = sinon.stub().returns(annotationItems);
      const pdfChild = {
        id: 200,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
        getAnnotations: getAnnotationsStub,
      };
      const parent = fakeItem({ id: 1 });
      (parent as any).getAttachments = () => [200];

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(1).resolves(parent);
      getAsyncStub.withArgs([200]).resolves([pdfChild]);

      installZotero({
        Items: { getAsync: getAsyncStub },
      });

      const { notesHandlers } = await import("../../src/handlers/notes");
      const result = await notesHandlers.getAnnotations({ parentId: 1 });

      expect(result).to.have.lengthOf(2);
      expect(result[0].id).to.equal(101);
      expect(getAsyncStub.neverCalledWith(annotationItems)).to.equal(true);
    });
  });

  describe("createAnnotation validation (fix #13)", () => {
    it("throws -32602 when text is provided for image annotation", async () => {
      installZotero({
        Items: { getAsync: sinon.stub().resolves(fakeItem({ id: 1, isAttachment: true })) },
      });

      delete require.cache[require.resolve("../../src/handlers/notes")];
      const { notesHandlers } = await import("../../src/handlers/notes");

      try {
        await notesHandlers.createAnnotation({
          parentId: 1, type: "image", text: "nope", position: {},
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/text.*highlight.*underline/i);
      }
    });

    it("throws -32602 on bad hex color", async () => {
      installZotero({
        Items: { getAsync: sinon.stub().resolves(fakeItem({ id: 1, isAttachment: true })) },
      });

      delete require.cache[require.resolve("../../src/handlers/notes")];
      const { notesHandlers } = await import("../../src/handlers/notes");

      try {
        await notesHandlers.createAnnotation({
          parentId: 1, type: "highlight", color: "#fff", position: {},
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/color/i);
      }
    });
  });

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

  describe("createAnnotation returns key (fix #31)", () => {
    it("returns {id, key} symmetric with notes.create", async () => {
      const parent: any = {
        id: 1, libraryID: 1, isAttachment: () => true, attachmentContentType: "application/pdf",
      };
      const annotation: any = {
        id: 999, key: "ANN999",
        annotationType: undefined, annotationText: undefined, annotationComment: undefined,
        annotationColor: undefined, annotationPosition: undefined,
        libraryID: 1, parentID: 1,
        saveTx: sinon.stub().callsFake(function (this: any) {
          return Promise.resolve();
        }),
      };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(1).resolves(parent) },
        Item: function (type: string) { return annotation; } as any,
      });
      // The handler does `new Zotero.Item("annotation")` — emulate that constructor:
      (globalThis as any).Zotero.Item = function (type: string) {
        return annotation;
      };

      const { notesHandlers } = await import("../../src/handlers/notes");
      const result = await notesHandlers.createAnnotation({
        parentId: 1, type: "highlight", text: "selected", position: { rects: [[0, 0, 100, 100]] },
      });
      expect(result).to.have.property("id", 999);
      expect(result).to.have.property("key", "ANN999");
      expect(annotation.annotationSortIndex).to.equal("00000|000000|00000");
    });

    it("uses caller-provided annotation sortIndex when supplied", async () => {
      const parent: any = {
        id: 1, libraryID: 1, isAttachment: () => true, attachmentContentType: "application/pdf",
      };
      const annotation: any = {
        id: 999, key: "ANN999",
        saveTx: sinon.stub().resolves(),
      };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(1).resolves(parent) },
        Item: function () { return annotation; } as any,
      });
      (globalThis as any).Zotero.Item = function () {
        return annotation;
      };

      const { notesHandlers } = await import("../../src/handlers/notes");
      await notesHandlers.createAnnotation({
        parentId: 1,
        type: "highlight",
        text: "selected",
        position: { pageIndex: 3, rects: [[0, 42, 100, 50]] },
        sortIndex: "00003|000012|00042",
      });
      expect(annotation.annotationSortIndex).to.equal("00003|000012|00042");
    });
  });
});
