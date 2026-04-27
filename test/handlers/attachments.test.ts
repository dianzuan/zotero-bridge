import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero, fakeItem } from "../fixtures/zotero-mock";

describe("attachments handler", () => {
  afterEach(() => resetZotero());

  describe("getFulltext (fix #2)", () => {
    it("reads cache file via getItemCacheFile + queries fulltextItems SQL — NOT non-existent getItemContent", async () => {
      const item = fakeItem({ id: 42, isAttachment: true });
      const cacheFile = { path: "/tmp/zotero/cache/42/.zotero-ft-cache" };

      const getCacheFileStub = sinon.stub().returns(cacheFile);
      const getContentsAsyncStub = sinon.stub().resolves("Hello world fulltext");
      const queryAsyncStub = sinon.stub().resolves([
        { indexedChars: 5000, totalChars: 5000 },
      ]);
      const getAsyncStub = sinon.stub().resolves(item);

      installZotero({
        Items: { getAsync: getAsyncStub },
        Fulltext: { getItemCacheFile: getCacheFileStub },
        File: { getContentsAsync: getContentsAsyncStub },
        DB: { queryAsync: queryAsyncStub },
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      const result = await attachmentsHandlers.getFulltext({ id: 42 });

      expect(getCacheFileStub.calledWith(item)).to.equal(true);
      expect(getContentsAsyncStub.calledWith(cacheFile.path)).to.equal(true);
      expect(queryAsyncStub.calledOnce).to.equal(true);
      // The SQL must select from fulltextItems by itemID
      const [sql, params] = queryAsyncStub.firstCall.args;
      expect(sql).to.match(/fulltextItems/i);
      expect(params).to.deep.equal([42]);

      expect(result).to.deep.equal({
        id: 42,
        content: "Hello world fulltext",
        indexedChars: 5000,
        totalChars: 5000,
      });
    });

    it("returns zeros when no fulltextItems row exists (un-indexed PDF)", async () => {
      const item = fakeItem({ id: 99, isAttachment: true });
      installZotero({
        Items: { getAsync: sinon.stub().resolves(item) },
        Fulltext: { getItemCacheFile: sinon.stub().returns({ path: "/tmp/none" }) },
        File: { getContentsAsync: sinon.stub().resolves("") },
        DB: { queryAsync: sinon.stub().resolves([]) }, // no row
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      const result = await attachmentsHandlers.getFulltext({ id: 99 });
      expect(result).to.deep.equal({
        id: 99,
        content: "",
        indexedChars: 0,
        totalChars: 0,
      });
    });

    it("rejects non-attachment items with -32602", async () => {
      const item = fakeItem({ id: 5, isAttachment: false });
      installZotero({
        Items: { getAsync: sinon.stub().resolves(item) },
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      try {
        await attachmentsHandlers.getFulltext({ id: 5 });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/attachment/i);
      }
    });
  });

  describe("list returns serializeItem array (fix #26)", () => {
    it("returns [(serializeItem)] not custom shape", async () => {
      const parent: any = {
        id: 1, getAttachments: () => [10, 11],
      };
      const att1: any = {
        id: 10, key: "A1", itemType: "attachment", itemTypeID: 3,
        dateAdded: "", dateModified: "", deleted: false,
        getField: (n: string) => n === "title" ? "PDF1" : "",
        isNote: () => false, isAttachment: () => true,
        attachmentContentType: "application/pdf", attachmentLinkMode: 1,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      };
      const att2: any = { ...att1, id: 11, key: "A2",
        getField: (n: string) => n === "title" ? "PDF2" : "" };

      const getAsyncStub = sinon.stub();
      getAsyncStub.withArgs(1).resolves(parent);
      getAsyncStub.withArgs([10, 11]).resolves([att1, att2]);
      installZotero({
        Items: { getAsync: getAsyncStub },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");
      const result = await attachmentsHandlers.list({ parentId: 1 });

      expect(result).to.have.lengthOf(2);
      // serializeItem-shape — has the standard envelope keys
      expect(result[0]).to.include.keys("id", "key", "itemType", "title", "dateAdded", "dateModified", "creators", "tags", "collections", "relations");
      // NOT the old custom shape — `path` / `linkMode` / `contentType` keys are gone (they're not on serializeItem)
      expect(result[0]).to.not.have.property("path");
      expect(result[0]).to.not.have.property("linkMode");
      expect(result[0]).to.not.have.property("contentType");
    });
  });

  describe("findPDF null-policy (fix #27)", () => {
    it("returns {attachment: (serializeItem)} when addAvailableFile resolves a PDF", async () => {
      const parent: any = { id: 1 };
      const pdf: any = {
        id: 10, key: "P", itemType: "attachment", itemTypeID: 3,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "PDF",
        isNote: () => false, isAttachment: () => true,
        attachmentContentType: "application/pdf",
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
      };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(1).resolves(parent) },
        Attachments: { addAvailableFile: sinon.stub().withArgs(parent).resolves(pdf) },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
      });

      const { attachmentsHandlers } = await import("../../src/handlers/attachments");
      const result = await attachmentsHandlers.findPDF({ parentId: 1 });

      expect(result).to.have.property("attachment");
      expect(result.attachment).to.not.be.null;
      expect(result.attachment.id).to.equal(10);
      // No `found` field — that was the discriminator we're dropping
      expect(result).to.not.have.property("found");
    });

    it("returns {attachment: null} when addAvailableFile resolves falsy", async () => {
      const parent: any = { id: 1 };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(1).resolves(parent) },
        Attachments: { addAvailableFile: sinon.stub().withArgs(parent).resolves(null) },
      });

      const { attachmentsHandlers } = await import("../../src/handlers/attachments");
      const result = await attachmentsHandlers.findPDF({ parentId: 1 });

      expect(result).to.deep.equal({ attachment: null });
    });
  });

  describe("delete", () => {
    it("erases attachment items", async () => {
      const eraseTx = sinon.stub().resolves();
      const attachment = fakeItem({ id: 44, isAttachment: true, eraseTx });
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(44).resolves(attachment) },
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");
      const result = await attachmentsHandlers.delete({ id: 44 });

      expect(result).to.deep.equal({ ok: true, id: 44 });
      expect(eraseTx.calledOnce).to.equal(true);
    });

    it("rejects non-attachment items", async () => {
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(45).resolves(fakeItem({ id: 45 })) },
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      try {
        await attachmentsHandlers.delete({ id: 45 });
        expect.fail("should have thrown");
      } catch (err: any) {
        expect(err.code).to.equal(-32602);
      }
    });
  });
});
