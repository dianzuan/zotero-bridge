import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("serializeItem", () => {
  beforeEach(() => {
    delete require.cache[require.resolve("../../src/utils/serialize")];
  });
  afterEach(() => resetZotero());

  it("includes note body via item.getNote() for note items", async () => {
    installZotero({
      ItemFields: {
        getItemTypeFields: () => [],
        getName: () => "",
      },
      CreatorTypes: { getName: () => "author" },
      Collections: { get: () => null },
    });
    const noteItem: any = {
      id: 100, key: "NOTE100", version: 1, itemType: "note", itemTypeID: 1,
      dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
      getField: () => "",
      isNote: () => true,
      isAttachment: () => false,
      getNote: () => "<p>Hello note body</p>",
      getCreators: () => [],
      getTags: () => [],
      getCollections: () => [],
      getRelations: () => ({}),
    };
    const { serializeItem } = await import("../../src/utils/serialize");
    const out = serializeItem(noteItem);
    expect(out.note).to.equal("<p>Hello note body</p>");
    expect(out.key).to.equal("NOTE100");
    expect(out.itemType).to.equal("note");
  });

  it("key-first shape: key, version present; no id, deleted, fieldMode", async () => {
    installZotero({
      ItemFields: { getItemTypeFields: () => [], getName: () => "" },
      CreatorTypes: { getName: () => "author" },
    });
    const item: any = {
      id: 42, key: "YR5BUGHG", version: 3, itemType: "journalArticle", itemTypeID: 2,
      dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
      getField: (n: string) => n === "title" ? "Test" : "",
      isNote: () => false, isAttachment: () => false,
      getCreators: () => [{ firstName: "三", lastName: "张", creatorTypeID: 1, fieldMode: 0 }],
      getTags: () => [{ tag: "AI", type: 0 }],
      getCollections: () => [10],
      getRelations: () => ({}),
    };

    // Mock Zotero.Collections.get to return a collection with a key
    (globalThis as any).Zotero.Collections = {
      get: (id: number) => id === 10 ? { key: "COL10KEY" } : null,
    };

    const { serializeItem } = await import("../../src/utils/serialize");
    const out = serializeItem(item);

    // key-first
    const keys = Object.keys(out);
    expect(keys[0]).to.equal("key");
    expect(keys[1]).to.equal("version");
    expect(out.key).to.equal("YR5BUGHG");
    expect(out.version).to.equal(3);

    // no numeric id
    expect(out).to.not.have.property("id");

    // no deleted
    expect(out).to.not.have.property("deleted");

    // creators: no fieldMode
    expect(out.creators[0]).to.not.have.property("fieldMode");
    expect(out.creators[0]).to.have.property("creatorType");

    // collections: keys not numeric IDs
    expect(out.collections).to.deep.equal(["COL10KEY"]);
  });

  it("does NOT include `note` field for regular items", async () => {
    installZotero({
      ItemFields: {
        getItemTypeFields: () => [],
        getName: () => "",
      },
      CreatorTypes: { getName: () => "author" },
      Collections: { get: () => null },
    });
    const articleItem: any = {
      id: 200, key: "ART200", version: 1, itemType: "journalArticle", itemTypeID: 2,
      dateAdded: "2026-01-01", dateModified: "2026-01-01", deleted: false,
      getField: (n: string) => n === "title" ? "An Article" : "",
      isNote: () => false,
      isAttachment: () => false,
      getNote: () => { throw new Error("should not be called for non-note"); },
      getCreators: () => [],
      getTags: () => [],
      getCollections: () => [],
      getRelations: () => ({}),
    };
    const { serializeItem } = await import("../../src/utils/serialize");
    const out = serializeItem(articleItem);
    expect(out).to.not.have.property("note");
    expect(out.title).to.equal("An Article");
  });
});
