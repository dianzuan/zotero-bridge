import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

describe("settings handler", () => {
  afterEach(() => resetZotero());

  describe("setAll (fix #19)", () => {
    it("throws -32602 on unknown key", async () => {
      installZotero({
        Prefs: { set: sinon.stub() },
      });

      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");

      try {
        await settingsHandlers.setAll({ bogusKey: 1 });
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/bogusKey/);
      }
    });
  });

  describe("get requires key (fix #38)", () => {
    it("throws -32602 when key is missing", async () => {
      installZotero({
        Prefs: { get: sinon.stub() },
      });
      const { settingsHandlers } = await import("../../src/handlers/settings");
      try {
        await settingsHandlers.get({} as any);
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.code).to.equal(-32602);
        expect(e.message).to.match(/key/i);
      }
    });

    it("returns single-pair {[key]: val} when key provided", async () => {
      installZotero({
        Prefs: { get: sinon.stub().withArgs("zotron.ocr.provider").returns("openai") },
      });
      const { settingsHandlers } = await import("../../src/handlers/settings");
      const result = await settingsHandlers.get({ key: "ocr.provider" });
      expect(result).to.deep.equal({ "ocr.provider": "openai" });
    });

    it("returns provider defaults when prefs are unset", async () => {
      installZotero({
        Prefs: { get: sinon.stub().returns(undefined) },
      });
      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");
      expect(await settingsHandlers.get({ key: "ocr.provider" })).to.deep.equal({ "ocr.provider": "" });
      expect(await settingsHandlers.get({ key: "embedding.provider" })).to.deep.equal({ "embedding.provider": "" });
      expect(await settingsHandlers.get({ key: "embedding.model" })).to.deep.equal({
        "embedding.model": "",
      });
      expect(await settingsHandlers.get({ key: "embedding.apiKey" })).to.deep.equal({ "embedding.apiKey": "" });
    });
  });

  describe("getAll defaults", () => {
    it("returns non-empty default providers and blank tokens when prefs are unset", async () => {
      installZotero({
        Prefs: { get: sinon.stub().returns(undefined) },
      });
      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");
      const result = await settingsHandlers.getAll();
      expect(result["ocr.provider"]).to.equal("");
      expect(result["ocr.model"]).to.equal("");
      expect(result["ocr.apiKey"]).to.equal("");
      expect(result["embedding.provider"]).to.equal("");
      expect(result["embedding.model"]).to.equal("");
      expect(result["embedding.apiKey"]).to.equal("");
      expect(result["ui.language"]).to.equal("en-US");
    });
  });


  describe("API key propagation", () => {
    it("redacts configured OCR and embedding API keys on read without Zotero logging", async () => {
      const store = new Map<string, any>();
      const zoteroLog = sinon.stub();
      installZotero({
        log: zoteroLog,
        Prefs: {
          get: sinon.stub().callsFake((key: string) => store.get(key)),
          set: sinon.stub().callsFake((key: string, value: any) => { store.set(key, value); }),
        },
      });
      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");

      await settingsHandlers.setAll({
        "ocr.apiKey": "test-ocr-secret",
        "embedding.apiKey": "test-embedding-secret",
      });

      expect(await settingsHandlers.get({ key: "ocr.apiKey" })).to.deep.equal({ "ocr.apiKey": "REDACTED" });
      expect(await settingsHandlers.get({ key: "embedding.apiKey" })).to.deep.equal({
        "embedding.apiKey": "REDACTED",
      });

      const all = await settingsHandlers.getAll();
      expect(all["ocr.apiKey"]).to.equal("REDACTED");
      expect(all["embedding.apiKey"]).to.equal("REDACTED");
      expect(zoteroLog.called).to.equal(false);
    });

    it("redacts secret values in write echoes", async () => {
      const set = sinon.stub();
      installZotero({
        Prefs: { set },
      });
      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");

      expect(await settingsHandlers.set({ key: "ocr.apiKey", value: "test-ocr-secret" })).to.deep.equal({
        key: "ocr.apiKey",
        value: "REDACTED",
      });
      expect(set.firstCall.args).to.deep.equal(["zotron.ocr.apiKey", "test-ocr-secret"]);
    });
  });

  describe("setAll Record echo (fix #39)", () => {
    it("returns {updated: Record<key,val>} echoing the applied pairs", async () => {
      installZotero({
        Prefs: { set: sinon.stub() },
      });
      const { settingsHandlers } = await import("../../src/handlers/settings");
      const result = await settingsHandlers.setAll({ "ocr.provider": "openai", "rag.topK": 5 });
      expect(result).to.have.property("updated");
      expect(result.updated).to.deep.equal({ "ocr.provider": "openai", "rag.topK": 5 });
    });

    it("also accepts the legacy CLI {settings: Record} wrapper", async () => {
      const set = sinon.stub();
      installZotero({
        Prefs: { set },
      });
      const { settingsHandlers } = await import("../../src/handlers/settings");
      const result = await settingsHandlers.setAll({ settings: { "ocr.provider": "glm", "rag.topK": 7 } });
      expect(result.updated).to.deep.equal({ "ocr.provider": "glm", "rag.topK": 7 });
      expect(set.firstCall.args).to.deep.equal(["zotron.ocr.provider", "glm"]);
      expect(set.secondCall.args).to.deep.equal(["zotron.rag.topK", 7]);
    });
  });

  describe("source keys (SP1)", () => {
    it("round-trips non-secret source keys and redacts secret ones", async () => {
      const store = new Map<string, any>();
      installZotero({
        Prefs: {
          get: sinon.stub().callsFake((k: string) => store.get(k)),
          set: sinon.stub().callsFake((k: string, v: any) => { store.set(k, v); }),
        },
      });
      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");

      await settingsHandlers.setAll({
        "sources.enabled": "openalex, crossref, core",
        "source.mailto": "me@example.org",
        "source.core.apiKey": "core-secret",
        "source.ads.token": "ads-secret",
        "source.aminer.apiKey": "aminer-secret",
      });

      expect(await settingsHandlers.getRaw({ key: "sources.enabled" }))
        .to.deep.equal({ "sources.enabled": "openalex, crossref, core" });
      expect(await settingsHandlers.get({ key: "source.mailto" }))
        .to.deep.equal({ "source.mailto": "me@example.org" });

      expect(await settingsHandlers.get({ key: "source.core.apiKey" }))
        .to.deep.equal({ "source.core.apiKey": "REDACTED" });
      expect(await settingsHandlers.getRaw({ key: "source.core.apiKey" }))
        .to.deep.equal({ "source.core.apiKey": "core-secret" });
      expect(await settingsHandlers.get({ key: "source.ads.token" }))
        .to.deep.equal({ "source.ads.token": "REDACTED" });
      expect(await settingsHandlers.get({ key: "source.aminer.apiKey" }))
        .to.deep.equal({ "source.aminer.apiKey": "REDACTED" });
    });

    it("exposes source defaults in getAll when prefs are unset", async () => {
      installZotero({ Prefs: { get: sinon.stub().returns(undefined) } });
      delete require.cache[require.resolve("../../src/handlers/settings")];
      const { settingsHandlers } = await import("../../src/handlers/settings");
      const all = await settingsHandlers.getAll();
      expect(all["sources.enabled"]).to.equal("openalex, crossref");
      expect(all["source.mailto"]).to.equal("");
      expect(all["source.core.apiKey"]).to.equal("");
    });
  });
});
