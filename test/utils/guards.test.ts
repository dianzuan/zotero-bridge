import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("guards", () => {
  beforeEach(() => {
    delete require.cache[require.resolve("../../src/utils/guards")];
  });
  afterEach(() => resetZotero());

  describe("requireItem", () => {
    it("returns the item when it exists", async () => {
      const item = { id: 42, key: "K42" };
      installZotero({ Items: { getAsync: sinon.stub().withArgs(42).resolves(item) } });
      const { requireItem } = await import("../../src/utils/guards");
      const result = await requireItem(42);
      expect(result).to.equal(item);
    });

    it("throws -32602 with `Item ${id} not found` when missing", async () => {
      installZotero({ Items: { getAsync: sinon.stub().resolves(null) } });
      const { requireItem } = await import("../../src/utils/guards");
      try {
        await requireItem(999);
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.equal("Item 999 not found");
      }
    });
  });

  describe("requireCollection", () => {
    it("returns the collection when it exists", async () => {
      const col = { id: 7, key: "C7" };
      installZotero({ Collections: { getAsync: sinon.stub().withArgs(7).resolves(col) } });
      const { requireCollection } = await import("../../src/utils/guards");
      const result = await requireCollection(7);
      expect(result).to.equal(col);
    });

    it("throws -32602 with `Collection ${id} not found` when missing", async () => {
      installZotero({ Collections: { getAsync: sinon.stub().resolves(null) } });
      const { requireCollection } = await import("../../src/utils/guards");
      try {
        await requireCollection(99);
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.equal("Collection 99 not found");
      }
    });

    it("requireCollection resolves 8-char key string", async () => {
      const col: any = { id: 5, key: "COL12345", name: "Test" };
      installZotero({
        Collections: {
          getAsync: sinon.stub().resolves(null),
          getByLibraryAndKeyAsync: sinon.stub().withArgs(1, "COL12345").resolves(col),
        },
        Libraries: { userLibraryID: 1 },
      });
      delete require.cache[require.resolve("../../src/utils/guards")];
      const { requireCollection } = await import("../../src/utils/guards");
      const result = await requireCollection("COL12345");
      expect(result.key).to.equal("COL12345");
    });

    it("requireCollection resolves numeric string as number", async () => {
      const col: any = { id: 7, key: "C7" };
      installZotero({
        Collections: {
          getAsync: sinon.stub().withArgs(7).resolves(col),
        },
      });
      delete require.cache[require.resolve("../../src/utils/guards")];
      const { requireCollection } = await import("../../src/utils/guards");
      const result = await requireCollection("7");
      expect(result).to.equal(col);
    });
  });
});
