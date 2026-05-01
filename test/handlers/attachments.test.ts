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

  describe("list returns attachment metadata", () => {
    it("returns serialized attachments with contentType, linkMode, and path for duplicate-PDF checks", async () => {
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
        getFilePathAsync: sinon.stub().resolves("/storage/paper.pdf"),
      };
      const att2: any = { ...att1, id: 11, key: "A2",
        attachmentContentType: "application/octet-stream",
        getField: (n: string) => n === "title" ? "PDF2" : "",
        getFilePathAsync: sinon.stub().resolves("/storage/paper2.PDF") };

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
      expect(result[0]).to.include({
        contentType: "application/pdf",
        linkMode: 1,
        path: "/storage/paper.pdf",
      });
      expect(result[1]).to.include({
        contentType: "application/octet-stream",
        path: "/storage/paper2.PDF",
      });
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

  describe("add renames from parent metadata (Zotero rename template)", () => {
    function setup({
      renameBase,
      renameResult = true,
      sourceLeafName = "cnki-export-bnhuaoo2.pdf",
    }: {
      renameBase: string;
      renameResult?: boolean | -1 | -2;
      sourceLeafName?: string;
    }) {
      const parent: any = { id: 1, key: "PARENT" };
      const renameAttachmentFile = sinon.stub().resolves(renameResult);
      const importedAttachment: any = {
        id: 99, key: "A99", itemType: "attachment", itemTypeID: 3,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "Full Text PDF",
        isNote: () => false, isAttachment: () => true,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        renameAttachmentFile,
      };
      const fakeFile = { exists: () => true, leafName: sourceLeafName, path: "/tmp/" + sourceLeafName };
      const importFromFile = sinon.stub().resolves(importedAttachment);
      const getFileBaseNameFromItem = sinon.stub().returns(renameBase);

      installZotero({
        Items: { getAsync: sinon.stub().withArgs(1).resolves(parent) },
        File: { pathToFile: sinon.stub().withArgs(fakeFile.path).returns(fakeFile) },
        Attachments: { importFromFile, getFileBaseNameFromItem },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        debug: sinon.stub(),
      });

      return { parent, fakeFile, importFromFile, getFileBaseNameFromItem, renameAttachmentFile };
    }

    it("default: renames attachment file using getFileBaseNameFromItem + extension from source", async () => {
      const { parent, fakeFile, getFileBaseNameFromItem, renameAttachmentFile } = setup({
        renameBase: "史丹 - 2020 - 数字经济条件下产业发展趋势的演变",
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      await attachmentsHandlers.add({ parentId: 1, path: fakeFile.path, title: "Full Text PDF" });

      expect(getFileBaseNameFromItem.calledOnceWith(parent)).to.equal(true);
      expect(renameAttachmentFile.calledOnce).to.equal(true);
      const [newName, overwrite, unique] = renameAttachmentFile.firstCall.args;
      expect(newName).to.equal("史丹 - 2020 - 数字经济条件下产业发展趋势的演变.pdf");
      expect(overwrite).to.equal(false);
      expect(unique).to.equal(true);
    });

    it("renameFromParent=false: skips rename entirely", async () => {
      const { fakeFile, getFileBaseNameFromItem, renameAttachmentFile } = setup({
        renameBase: "should-not-be-used",
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      await attachmentsHandlers.add({
        parentId: 1, path: fakeFile.path, title: "Full Text PDF", renameFromParent: false,
      });

      expect(getFileBaseNameFromItem.called).to.equal(false);
      expect(renameAttachmentFile.called).to.equal(false);
    });

    it("empty getFileBaseNameFromItem result: keeps original name (no rename call)", async () => {
      const { fakeFile, getFileBaseNameFromItem, renameAttachmentFile } = setup({
        renameBase: "",
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      await attachmentsHandlers.add({ parentId: 1, path: fakeFile.path });

      expect(getFileBaseNameFromItem.calledOnce).to.equal(true);
      expect(renameAttachmentFile.called).to.equal(false);
    });

    it("rename throwing is non-fatal: attachment is still returned", async () => {
      const parent: any = { id: 1 };
      const importedAttachment: any = {
        id: 7, key: "A7", itemType: "attachment", itemTypeID: 3,
        dateAdded: "", dateModified: "", deleted: false,
        getField: () => "Full Text PDF",
        isNote: () => false, isAttachment: () => true,
        getCreators: () => [], getTags: () => [], getCollections: () => [], getRelations: () => ({}),
        renameAttachmentFile: sinon.stub().rejects(new Error("disk full")),
      };
      const fakeFile = { exists: () => true, leafName: "x.pdf", path: "/tmp/x.pdf" };
      installZotero({
        Items: { getAsync: sinon.stub().withArgs(1).resolves(parent) },
        File: { pathToFile: sinon.stub().withArgs(fakeFile.path).returns(fakeFile) },
        Attachments: {
          importFromFile: sinon.stub().resolves(importedAttachment),
          getFileBaseNameFromItem: sinon.stub().returns("foo"),
        },
        ItemFields: { getItemTypeFields: () => [], getName: () => "" },
        CreatorTypes: { getName: () => "author" },
        debug: sinon.stub(),
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      const result = await attachmentsHandlers.add({ parentId: 1, path: fakeFile.path });
      expect(result.id).to.equal(7);
    });

    it("file with no extension: passes baseName through unchanged", async () => {
      const { fakeFile, renameAttachmentFile } = setup({
        renameBase: "Some Title",
        sourceLeafName: "no_extension_file",
      });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");

      await attachmentsHandlers.add({ parentId: 1, path: fakeFile.path });

      expect(renameAttachmentFile.firstCall.args[0]).to.equal("Some Title");
    });
  });

  describe("delete artifact attachments", () => {
    it("erases attachment items and rejects non-attachments", async () => {
      const eraseTx = sinon.stub().resolves();
      const attachment = fakeItem({ id: 44, isAttachment: true, eraseTx });
      const getAsync = sinon.stub().withArgs(44).resolves(attachment);
      installZotero({ Items: { getAsync } });

      delete require.cache[require.resolve("../../src/handlers/attachments")];
      const { attachmentsHandlers } = await import("../../src/handlers/attachments");
      const result = await attachmentsHandlers.delete({ id: 44 });

      expect(result).to.deep.equal({ ok: true, id: 44 });
      expect(eraseTx.calledOnce).to.equal(true);
    });
  });

});
