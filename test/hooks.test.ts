import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "./fixtures/zotero-mock";

describe("startup preference defaults", () => {
  afterEach(() => resetZotero());

  function installPrefs(initial: Record<string, any>) {
    const store = new Map(Object.entries(initial).map(([key, value]) => [`extensions.zotron.${key}`, value]));
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

  it("writes GLM, Doubao, and English defaults for fresh installs", async () => {
    const prefs = installPrefs({});
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("extensions.zotron.ocr.provider")).to.equal("glm");
    expect(prefs.store.get("extensions.zotron.embedding.provider")).to.equal("doubao");
    expect(prefs.store.get("extensions.zotron.embedding.model")).to.equal("doubao-embedding-vision-251215");
    expect(prefs.store.get("extensions.zotron.embedding.apiKey")).to.equal("");
    expect(prefs.store.get("extensions.zotron.ui.language")).to.equal("en-US");
  });

  it("migrates only the untouched old Ollama default to Doubao", async () => {
    const prefs = installPrefs({
      "embedding.provider": "ollama",
      "embedding.model": "qwen3-embedding:4b",
      "embedding.apiUrl": "http://localhost:11434",
      "embedding.apiKey": "",
    });
    const { __test__ } = await loadHooks();

    __test__.setPreferenceDefaults();

    expect(prefs.store.get("extensions.zotron.embedding.provider")).to.equal("doubao");
    expect(prefs.store.get("extensions.zotron.embedding.model")).to.equal("doubao-embedding-vision-251215");
    expect(prefs.store.get("extensions.zotron.embedding.apiUrl")).to.equal("https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal");
    expect(prefs.store.get("extensions.zotron.embedding.apiKey")).to.equal("");
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

    expect(prefs.store.get("extensions.zotron.embedding.provider")).to.equal("ollama");
    expect(prefs.store.get("extensions.zotron.embedding.model")).to.equal("custom-ollama-model");
    expect(prefs.store.get("extensions.zotron.embedding.apiUrl")).to.equal("http://localhost:11434");
  });
});
