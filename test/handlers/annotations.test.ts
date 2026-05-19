import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

function mockPdfExtract(overrides: {
  extractAllPagesChars?: (...args: any[]) => Promise<any[][]>;
  extractPageChars?: (...args: any[]) => Promise<any[]>;
  getPageCount?: (...args: any[]) => Promise<number>;
}) {
  const allPages = overrides.extractAllPagesChars ?? (async () => []);
  const extractPage = overrides.extractPageChars ?? (async () => []);
  const pageCount = overrides.getPageCount ?? (async () => 0);
  const resolvedPath = require.resolve("../../src/utils/pdf-extract");
  require.cache[resolvedPath] = {
    id: resolvedPath,
    filename: resolvedPath,
    loaded: true,
    exports: {
      extractPageChars: extractPage,
      extractAllPagesChars: allPages,
      getPageCount: pageCount,
      shutdown: () => {},
      __test__: { setExtractImpl() {}, setAllCharsImpl() {}, setCountImpl() {}, reset() {} },
    },
  } as any;
}

function clearPdfExtractMock() {
  delete require.cache[require.resolve("../../src/utils/pdf-extract")];
}

describe("annotations handler", () => {
  beforeEach(() => {
    delete require.cache[require.resolve("../../src/handlers/annotations")];
  });

  afterEach(() => resetZotero());

  describe("list", () => {
    it("returns serialized annotations with annotation-specific fields", async () => {
      const parent: any = {
        id: 1,
        key: "ATT01",
        isAttachment: () => true,
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

      expect(result.items).to.have.lengthOf(2);
      expect(result.total).to.equal(2);
      expect(result.attachmentKey).to.equal("ATT01");
      expect(result.items[0].annotationType).to.equal("highlight");
      expect(result.items[0].annotationText).to.equal("important text");
      expect(result.items[0].annotationComment).to.equal("my comment");
      expect(result.items[0].annotationColor).to.equal("#ffd400");
      expect(result.items[0].annotationPosition).to.deep.equal({ pageIndex: 0, rects: [[0, 0, 100, 100]] });
      expect(result.items[1].annotationType).to.equal("note");
      expect(result.items[1].annotationPosition).to.be.null;
    });

    it("returns empty array when item has no annotations", async () => {
      const parent: any = { id: 2, key: "ATT02", isAttachment: () => true, getAnnotations: () => [] };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(2).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.list({ parentKey: 2 });

      expect(result).to.deep.equal({ items: [], total: 0, attachmentKey: "ATT02" });
    });

    it("accepts Zotero 9 annotation item refs from getAnnotations", async () => {
      const ann: any = {
        id: 12, key: "ANN12", itemType: "annotation", itemTypeID: 99,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "",
        isNote: () => false, isAttachment: () => false,
        isAnnotation: () => true,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        annotationType: "underline",
        annotationText: "underlined text",
        annotationComment: "comment",
        annotationColor: "#ff6666",
        annotationPosition: JSON.stringify({ pageIndex: 0, rects: [[10, 20, 30, 40]] }),
      };
      const parent: any = {
        id: 2,
        key: "ATT02",
        libraryID: 1,
        isAttachment: () => true,
        getAnnotations: () => [ann],
      };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(2).resolves(parent) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.list({ parentKey: 2 });

      expect(result.items).to.have.lengthOf(1);
      expect(result.total).to.equal(1);
      expect(result.attachmentKey).to.equal("ATT02");
      expect(result.items[0].key).to.equal("ANN12");
      expect(result.items[0].annotationType).to.equal("underline");
    });

    it("auto-resolves item key to first PDF attachment", async () => {
      const pdfAttachment: any = {
        id: 20,
        key: "PDFATT01",
        libraryID: 1,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
        getAnnotations: () => [30],
      };
      const ann: any = {
        id: 30, key: "ANN30", itemType: "annotation", itemTypeID: 99,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "",
        isNote: () => false, isAttachment: () => false,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        annotationType: "highlight",
        annotationText: "auto-resolved",
        annotationComment: "",
        annotationColor: "#ffd400",
        annotationPosition: JSON.stringify({ pageIndex: 0, rects: [[0, 0, 50, 50]] }),
      };
      const regularItem: any = {
        id: 10,
        key: "ITEM01",
        isAttachment: () => false,
        getAttachments: () => [20],
      };

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(10).resolves(regularItem);
      getAsyncStub.withArgs(20).resolves(pdfAttachment);
      getAsyncStub.withArgs([30]).resolves([ann]);

      installZotero({
        Items: { getAsync: getAsyncStub },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        Collections: { get: () => null },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.list({ parentKey: 10 });

      expect(result.items).to.have.lengthOf(1);
      expect(result.total).to.equal(1);
      expect(result.attachmentKey).to.equal("PDFATT01");
      expect(result.items[0].key).to.equal("ANN30");
      expect(result.items[0].annotationText).to.equal("auto-resolved");
    });

    it("throws -32602 when item has no PDF attachments", async () => {
      const regularItem: any = {
        id: 10,
        key: "ITEM01",
        isAttachment: () => false,
        getAttachments: () => [],
      };

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(10).resolves(regularItem) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.list({ parentKey: 10 });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/No PDF attachment found/);
      }
    });

    it("skips non-PDF attachments when auto-resolving", async () => {
      const htmlAttachment: any = {
        id: 21,
        isAttachment: () => true,
        attachmentContentType: "text/html",
      };
      const pdfAttachment: any = {
        id: 22,
        key: "PDFATT02",
        libraryID: 1,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
        getAnnotations: () => [],
      };
      const regularItem: any = {
        id: 10,
        key: "ITEM01",
        isAttachment: () => false,
        getAttachments: () => [21, 22],
      };

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(10).resolves(regularItem);
      getAsyncStub.withArgs(21).resolves(htmlAttachment);
      getAsyncStub.withArgs(22).resolves(pdfAttachment);

      installZotero({
        Items: { getAsync: getAsyncStub },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.list({ parentKey: 10 });

      expect(result.attachmentKey).to.equal("PDFATT02");
      expect(result.items).to.have.lengthOf(0);
    });
  });

  describe("create", () => {
    it("creates an annotation and returns {ok, key, attachmentKey}", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
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
      expect(result.attachmentKey).to.equal("ATT05");
      expect(saveTxStub.calledOnce).to.equal(true);
      expect(createdItem.libraryID).to.equal(1);
      expect(createdItem.parentID).to.equal(5);
      expect(createdItem.annotationType).to.equal("highlight");
      expect(createdItem.annotationText).to.equal("selected text");
      expect(createdItem.annotationComment).to.equal("my note");
      expect(createdItem.annotationColor).to.equal("#ffd400");
      expect(createdItem.annotationSortIndex).to.equal("00001|000000|00020");
      expect(JSON.parse(createdItem.annotationPosition)).to.deep.equal({
        pageIndex: 1, rects: [[10, 20, 30, 40]],
      });
    });

    it("auto-resolves item key to PDF attachment for create", async () => {
      const pdfAttachment: any = {
        id: 50,
        key: "PDFATT50",
        libraryID: 1,
        isAttachment: () => true,
        attachmentContentType: "application/pdf",
      };
      const regularItem: any = {
        id: 40,
        key: "ITEM40",
        isAttachment: () => false,
        getAttachments: () => [50],
      };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(40).resolves(regularItem);
      getAsyncStub.withArgs(50).resolves(pdfAttachment);

      installZotero({
        Items: { getAsync: getAsyncStub },
        Item: function (itemType: string) {
          createdItem = {
            itemType,
            libraryID: 0,
            parentID: 0,
            key: "NEWANN03",
            saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      (globalThis as any).Zotero.Item = (globalThis as any).Zotero.Item;

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 40,
        type: "highlight",
        text: "resolved text",
        position: { pageIndex: 0, rects: [[5, 10, 15, 20]] },
      });

      expect(result.ok).to.equal(true);
      expect(result.key).to.equal("NEWANN03");
      expect(result.attachmentKey).to.equal("PDFATT50");
      expect(createdItem.parentID).to.equal(50);
      expect(createdItem.libraryID).to.equal(1);
    });

    it("throws -32602 when item has no PDF for create", async () => {
      const regularItem: any = {
        id: 40,
        key: "ITEM40",
        isAttachment: () => false,
        getAttachments: () => [],
      };

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(40).resolves(regularItem) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 40,
          type: "highlight",
          text: "will fail",
          position: { pageIndex: 0, rects: [[0, 0, 10, 10]] },
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/No PDF attachment found/);
      }
    });

    it("preserves explicit Zotero PDF sortIndex strings", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        Item: function (itemType: string) {
          createdItem = {
            itemType,
            libraryID: 0,
            parentID: 0,
            key: "NEWANN02",
            saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      await annotationsHandlers.create({
        parentKey: 5,
        type: "underline",
        text: "selected text",
        color: "#ff6666",
        position: { pageIndex: 0, rects: [[10, 20, 30, 40]] },
        sortIndex: "00000|000000|00165",
      });

      expect(createdItem.annotationSortIndex).to.equal("00000|000000|00165");
    });

    it("rejects invalid annotation type with -32602", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
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

    it("rejects missing annotation position with -32602", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "highlight",
          text: "selected text",
          position: undefined as any,
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/position/i);
      }
    });

    it("rejects malformed annotation position with -32602", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "highlight",
          text: "selected text",
          position: { foo: 1 },
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/pageIndex|rects|position/i);
      }
    });

    it("rejects invalid sortIndex with -32602", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "highlight",
          text: "selected text",
          position: { pageIndex: 1, rects: [[10, 20, 30, 40]] },
          sortIndex: "bad",
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/sortIndex/i);
      }
    });

    it("rejects boolean sortIndex with -32602", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "highlight",
          text: "selected text",
          position: { pageIndex: 1, rects: [[10, 20, 30, 40]] },
          sortIndex: true,
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/sortIndex/i);
      }
    });
  });

  describe("create with quote (via pdf-extract)", () => {
    afterEach(() => clearPdfExtractMock());

    function buildCharsFromText(text: string, yTop: number): any[] {
      const chars: any[] = [];
      let x = 10;
      for (let i = 0; i < text.length; i++) {
        const c = text[i];
        const isSpace = c === " ";
        const isLast = i === text.length - 1;
        chars.push({
          c, rect: [x, yTop, x + 6, yTop + 12],
          inlineRect: [10, yTop - 2, 260, yTop + 14],
          spaceAfter: isSpace,
          lineBreakAfter: isLast,
        });
        x += 6;
      }
      return chars;
    }

    it("rejects quote for non-highlight/underline types with -32602", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      mockPdfExtract({});
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "note",
          quote: "some text to find",
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/quote.*highlight.*underline/i);
      }
    });

    it("throws -32602 when quote is not found on any page", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const page0Chars = buildCharsFromText("Hello world", 90);

      mockPdfExtract({
        extractAllPagesChars: async () => [page0Chars],
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5,
          type: "highlight",
          quote: "this text does not exist in the PDF",
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/Quote not found/);
      }
    });

    it("resolves quote on page 0 and creates annotation", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const page0Chars = [
        ...buildCharsFromText("This is an important sentence in the PDF.", 90),
        ...buildCharsFromText("And another line follows.", 70),
      ];

      mockPdfExtract({
        extractAllPagesChars: async () => [page0Chars, [], []],
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
        Item: function (itemType: string) {
          createdItem = {
            itemType, libraryID: 0, parentID: 0,
            key: "NEWANN04", saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5,
        type: "highlight",
        quote: "important sentence",
      });

      expect(result.ok).to.equal(true);
      expect(result.key).to.equal("NEWANN04");
      expect(result.attachmentKey).to.equal("ATT05");
      expect(saveTxStub.calledOnce).to.equal(true);
      expect(createdItem.annotationText).to.equal("important sentence");
      const pos = JSON.parse(createdItem.annotationPosition);
      expect(pos.pageIndex).to.equal(0);
      expect(pos.rects).to.have.lengthOf(1);
      expect(pos.rects[0]).to.have.lengthOf(4);
    });

    it("finds quote on page 2 after skipping pages 0 and 1", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const page2Chars = buildCharsFromText("The target quote lives here.", 500);

      mockPdfExtract({
        extractAllPagesChars: async () => [[], [], page2Chars, [], []],
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
        Item: function (itemType: string) {
          createdItem = {
            itemType, libraryID: 0, parentID: 0,
            key: "NEWANN05", saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5,
        type: "highlight",
        quote: "target quote",
      });

      expect(result.ok).to.equal(true);
      const pos = JSON.parse(createdItem.annotationPosition);
      expect(pos.pageIndex).to.equal(2);
      expect(pos.rects).to.have.lengthOf(1);
    });

    it("searches only the specified pageIndex when provided", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const page7Chars = buildCharsFromText("Specific page content here.", 400);
      const emptyPages = Array.from({ length: 10 }, () => [] as any[]);
      emptyPages[7] = page7Chars;

      mockPdfExtract({
        extractAllPagesChars: async () => emptyPages,
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
        Item: function (itemType: string) {
          createdItem = {
            itemType, libraryID: 0, parentID: 0,
            key: "NEWANN06", saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5,
        type: "highlight",
        quote: "page content",
        pageIndex: 7,
      });

      expect(result.ok).to.equal(true);
      const pos = JSON.parse(createdItem.annotationPosition);
      expect(pos.pageIndex).to.equal(7);
    });

    it("uses open reader chars before falling back to pdf-extract", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const readerChars = buildCharsFromText("Reader has this text already loaded.", 90);
      const mockReader = {
        itemID: 5,
        _internalReader: {
          _primaryView: {
            _pdfPages: [{ chars: readerChars }],
          },
        },
      };

      const allPagesStub = sinon.stub().resolves([[]]);
      mockPdfExtract({
        extractAllPagesChars: allPagesStub,
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [{
          Zotero_Tabs: {
            _tabs: [{ id: "tab1" }],
          },
        }],
        Reader: {
          getByTabID: sinon.stub().returns(mockReader),
        },
        Item: function (itemType: string) {
          createdItem = {
            itemType, libraryID: 0, parentID: 0,
            key: "NEWANN07", saveTx: saveTxStub,
          };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5,
        type: "highlight",
        quote: "text already loaded",
      });

      expect(result.ok).to.equal(true);
      const pos = JSON.parse(createdItem.annotationPosition);
      expect(pos.pageIndex).to.equal(0);
      // pdf-extract should NOT have been called — reader had the data
      expect(allPagesStub.called).to.be.false;
    });

    it("throws -32602 for 0-page PDF (no chars to search)", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      mockPdfExtract({
        extractAllPagesChars: async () => [],
      });
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({ parentKey: 5, type: "highlight", quote: "anything" });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/Quote not found/);
      }
    });

    it("finds quote on the last page of a multi-page PDF", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;
      const lastPageChars = buildCharsFromText("Final page treasure here.", 200);

      mockPdfExtract({
        extractAllPagesChars: async () => [[], [], [], lastPageChars],
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
        Item: function (itemType: string) {
          createdItem = { itemType, libraryID: 0, parentID: 0, key: "NEWLAST", saveTx: saveTxStub };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5, type: "highlight", quote: "Final page treasure",
      });

      expect(result.ok).to.equal(true);
      const pos = JSON.parse(createdItem.annotationPosition);
      expect(pos.pageIndex).to.equal(3);
    });

    it("throws -32602 when pageIndex is out of range", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      mockPdfExtract({
        extractAllPagesChars: async () => [buildCharsFromText("Only page zero.", 90)],
      });
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      try {
        await annotationsHandlers.create({
          parentKey: 5, type: "highlight", quote: "Only page zero", pageIndex: 999,
        });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/Quote not found on page 999/);
      }
    });

    it("falls back to pdf-extract when reader has no chars for the page", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const pdfChars = buildCharsFromText("Text from pdf-extract fallback.", 300);
      const mockReader = {
        itemID: 5,
        _internalReader: {
          _primaryView: {
            _pdfPages: [{ chars: [] }],
          },
        },
      };

      mockPdfExtract({
        extractAllPagesChars: async () => [pdfChars],
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [{
          Zotero_Tabs: { _tabs: [{ id: "tab1" }] },
        }],
        Reader: { getByTabID: sinon.stub().returns(mockReader) },
        Item: function (itemType: string) {
          createdItem = { itemType, libraryID: 0, parentID: 0, key: "NEWFALL", saveTx: saveTxStub };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      const result = await annotationsHandlers.create({
        parentKey: 5, type: "highlight", quote: "pdf-extract fallback",
      });

      expect(result.ok).to.equal(true);
      const pos = JSON.parse(createdItem.annotationPosition);
      expect(pos.pageIndex).to.equal(0);
    });

    it("populates charOffset in sortIndex middle segment for quote-based creation", async () => {
      const parent: any = { id: 5, key: "ATT05", libraryID: 1, isAttachment: () => true };
      const saveTxStub = sinon.stub().resolves();
      let createdItem: any = null;

      const pageChars = buildCharsFromText("prefix target suffix", 90);
      mockPdfExtract({
        extractAllPagesChars: async () => [pageChars],
      });

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(5).resolves(parent) },
        getMainWindows: () => [],
        Item: function (itemType: string) {
          createdItem = { itemType, libraryID: 0, parentID: 0, key: "NEWSORT", saveTx: saveTxStub };
          return createdItem;
        },
      });

      const { annotationsHandlers } = await import("../../src/handlers/annotations");
      await annotationsHandlers.create({
        parentKey: 5, type: "highlight", quote: "target",
      });

      const sortIndex = createdItem.annotationSortIndex;
      const [, charOff] = sortIndex.split("|");
      expect(parseInt(charOff, 10)).to.be.greaterThan(0);
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
