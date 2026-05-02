/**
 * Zotero global stub for handler tests.
 *
 * Every handler test imports this BEFORE importing the handler module:
 *
 *   import { installZotero, resetZotero } from "../fixtures/zotero-mock";
 *   beforeEach(() => installZotero({ ... }));
 *   afterEach(() => resetZotero());
 *
 * Pass overrides per-test for the specific Zotero.* surface the handler
 * touches. Anything not overridden is `undefined` — accessing it from
 * the handler will throw, which is the desired loud failure.
 */
import sinon from "sinon";

type ZoteroStub = Record<string, any>;

let installed = false;

export function installZotero(stub: ZoteroStub): ZoteroStub {
  if (installed) {
    throw new Error("Zotero stub already installed — call resetZotero() first");
  }
  globalThis.Zotero = stub;
  installed = true;
  return stub;
}

export function resetZotero(): void {
  delete globalThis.Zotero;
  installed = false;
  sinon.restore();
}

/**
 * Convenience: a minimal `Zotero.Item`-like fake with the methods most
 * handlers call. Pass per-test field overrides via `data`.
 */
export function fakeItem(data: {
  id: number;
  key?: string;
  itemType?: string;
  fields?: Record<string, any>;
  isAttachment?: boolean;
  isNote?: boolean;
  deleted?: boolean;
  version?: number;
  parentItemID?: number | null;
  saveTx?: sinon.SinonStub;
  eraseTx?: sinon.SinonStub;
}): any {
  return {
    id: data.id,
    key: data.key ?? `KEY${data.id}`,
    itemType: data.itemType ?? "journalArticle",
    itemTypeID: 1,
    deleted: data.deleted ?? false,
    version: data.version ?? 1,
    parentItemID: data.parentItemID ?? null,
    dateAdded: "2026-01-01T00:00:00Z",
    dateModified: "2026-01-01T00:00:00Z",
    getField: sinon.stub().callsFake((name: string) => data.fields?.[name] ?? ""),
    isAttachment: () => data.isAttachment ?? false,
    isNote: () => data.isNote ?? false,
    isRegularItem: () => !(data.isAttachment || data.isNote),
    getCreators: () => [],
    getTags: () => [],
    getCollections: () => [],
    getRelations: () => ({}),
    getChildItems: () => [],
    saveTx: data.saveTx ?? sinon.stub().resolves(),
    eraseTx: data.eraseTx ?? sinon.stub().resolves(),
  };
}

/** Minimal Zotero.Collection-like fake. */
export function fakeCollection(data: {
  id: number;
  key?: string;
  name?: string;
  parentID?: number | null;
  version?: number;
  childCollections?: any[];
  childItems?: any[];
}): any {
  return {
    id: data.id,
    key: data.key ?? `COL${data.id}`,
    name: data.name ?? `Collection ${data.id}`,
    parentID: data.parentID ?? null,
    version: data.version ?? 1,
    libraryID: 1,
    getChildCollections: () => data.childCollections ?? [],
    getChildItems: () => data.childItems ?? [],
    saveTx: sinon.stub().resolves(),
    eraseTx: sinon.stub().resolves(),
    addItems: sinon.stub().resolves(),
    removeItems: sinon.stub().resolves(),
  };
}
