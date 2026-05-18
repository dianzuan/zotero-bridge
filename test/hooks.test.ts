import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "./fixtures/zotero-mock";

describe("startup preference defaults", () => {
  afterEach(() => resetZotero());

  function installPrefs(initial: Record<string, any>) {
    const store = new Map(Object.entries(initial).map(([key, value]) => [`zotron.${key}`, value]));
    const prefs = {
      get: sinon.stub().callsFake((key: string) => store.get(key)),
      set: sinon.stub().callsFake((key: string, value: any) => { store.set(key, value); }),
      store,
    };
    installZotero({ Prefs: prefs });
    return prefs;
  }

  async function loadHooks() {
    delete require.cache[require.resolve("../src/hooks")];
    return import("../src/hooks");
  }

  it("writes empty provider defaults for fresh installs", async () => {
    const prefs = installPrefs({});
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.ocr.provider")).to.equal("");
    expect(prefs.store.get("zotron.embedding.provider")).to.equal("");
    expect(prefs.store.get("zotron.embedding.model")).to.equal("");
    expect(prefs.store.get("zotron.embedding.apiKey")).to.equal("");
    expect(prefs.store.get("zotron.ui.language")).to.equal("en-US");
  });

  it("clears the untouched old Ollama default", async () => {
    const prefs = installPrefs({
      "embedding.provider": "ollama",
      "embedding.model": "qwen3-embedding:4b",
      "embedding.apiUrl": "http://localhost:11434",
      "embedding.apiKey": "",
    });
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.embedding.provider")).to.equal("");
    expect(prefs.store.get("zotron.embedding.model")).to.equal("");
    expect(prefs.store.get("zotron.embedding.apiUrl")).to.equal("");
    expect(prefs.store.get("zotron.embedding.apiKey")).to.equal("");
  });

  it("clears the old Doubao multimodal default", async () => {
    const prefs = installPrefs({
      "embedding.provider": "doubao",
      "embedding.model": "doubao-embedding-vision-251215",
      "embedding.apiUrl": "https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal",
      "embedding.apiKey": "existing-key",
    });
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.embedding.provider")).to.equal("");
    expect(prefs.store.get("zotron.embedding.model")).to.equal("");
    expect(prefs.store.get("zotron.embedding.apiUrl")).to.equal("");
    expect(prefs.store.get("zotron.embedding.apiKey")).to.equal("existing-key");
  });

  it("does not migrate customized Ollama settings", async () => {
    const prefs = installPrefs({
      "embedding.provider": "ollama",
      "embedding.model": "custom-ollama-model",
      "embedding.apiUrl": "http://localhost:11434",
      "embedding.apiKey": "",
    });
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.embedding.provider")).to.equal("ollama");
    expect(prefs.store.get("zotron.embedding.model")).to.equal("custom-ollama-model");
    expect(prefs.store.get("zotron.embedding.apiUrl")).to.equal("http://localhost:11434");
  });

  it("migrates legacy unprefixed API keys when current keys are empty", async () => {
    const prefs = installPrefs({});
    prefs.store.set("ocr.apiKey", "legacy-ocr-key");
    prefs.store.set("embedding.apiKey", "legacy-embedding-key");
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.ocr.apiKey")).to.equal("legacy-ocr-key");
    expect(prefs.store.get("zotron.embedding.apiKey")).to.equal("legacy-embedding-key");
  });

  it("migrates legacy extensions.zotron settings into the Zotero add-on branch", async () => {
    const prefs = installPrefs({});
    prefs.store.set("extensions.zotron.ui.language", "zh-CN");
    prefs.store.set("extensions.zotron.ocr.provider", "paddle");
    prefs.store.set("extensions.zotron.ocr.apiKey", "old-ocr-key");
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.ui.language")).to.equal("zh-CN");
    expect(prefs.store.get("zotron.ocr.provider")).to.equal("paddle");
    expect(prefs.store.get("zotron.ocr.apiKey")).to.equal("old-ocr-key");
  });

  it("does not overwrite current API keys with legacy values", async () => {
    const prefs = installPrefs({
      "ocr.apiKey": "current-ocr-key",
      "embedding.apiKey": "current-embedding-key",
    });
    prefs.store.set("ocr.apiKey", "legacy-ocr-key");
    prefs.store.set("embedding.apiKey", "legacy-embedding-key");
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("zotron.ocr.apiKey")).to.equal("current-ocr-key");
    expect(prefs.store.get("zotron.embedding.apiKey")).to.equal("current-embedding-key");
  });
});
