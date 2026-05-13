import { expect } from "chai";
import sinon from "sinon";
import { installZotero, resetZotero } from "../fixtures/zotero-mock";

function makeItem(attachmentIds = [4201]) {
  return {
    id: 42,
    key: "ITEM42",
    isNote: () => false,
    isAttachment: () => false,
    getAttachments: () => attachmentIds,
    getField: sinon.stub().callsFake((name: string) => ({
      title: "产业贸易中心性、贸易外向度与金融风险",
      date: "2022",
      publicationTitle: "中国工业经济",
      DOI: "10.test/example",
    }[name] || "")),
    getCreators: () => [
      { firstName: "姝黛", lastName: "王" },
      { firstName: "子荣", lastName: "杨" },
    ],
  };
}

function makeChunkAttachment(id = 4201) {
  return {
    id,
    key: "ATT42",
    isAttachment: () => true,
    getField: sinon.stub().callsFake((name: string) => (
      name === "title" ? "ITEM42.zotron-chunks.jsonl" : ""
    )),
    getFilePathAsync: sinon.stub().resolves("/tmp/ITEM42.zotron-chunks.jsonl"),
  };
}

function makeEmbedAttachment() {
  return {
    id: 4202,
    key: "EMB42",
    isAttachment: () => true,
    getField: sinon.stub().callsFake((name: string) => (
      name === "title" ? "ITEM42.zotron-embed.npz" : ""
    )),
    getFilePathAsync: sinon.stub().resolves("/tmp/ITEM42.zotron-embed.npz"),
  };
}

describe("rag handler", () => {
  afterEach(() => resetZotero());

  it("searchHits returns academic-zh hits from Zotero-attached chunk artifacts", async () => {
    const item = makeItem();
    const attachment = makeChunkAttachment();
    const collection = {
      id: 7,
      name: "中国工业经济",
      getChildItems: () => [item],
    };
    const chunks = [
      {
        item_key: "ITEM42",
        title: "产业贸易中心性、贸易外向度与金融风险",
        text: "本文利用世界投入产出表和金融风险指标构造识别策略。",
        section_heading: "三、研究设计",
        section_path: ["三、研究设计"],
        attachment_key: "ATT42",
        chunk_key: "ATT42:c000001",
        block_keys: ["ATT42:p12:b08"],
        page_range: [12, 12],
        evidence_refs: [{ block_key: "ATT42:p12:b08", page_idx: 12, bbox: [10, 20, 300, 40] }],
      },
      {
        item_key: "ITEM42",
        title: "产业贸易中心性、贸易外向度与金融风险",
        text: "附录说明数据清洗过程。",
        section_heading: "附录",
        chunk_key: "ATT42:c000002",
      },
    ];
    installZotero({
      Libraries: { userLibraryID: 1 },
      Collections: { getByLibrary: sinon.stub().returns([collection]) },
      Items: {
        getAsync: sinon.stub().callsFake(async (id: number | number[]) => {
          if (Array.isArray(id)) return id.map((one) => one === 42 ? item : attachment);
          return id === 42 ? item : attachment;
        }),
      },
      File: {
        getContentsAsync: sinon.stub().resolves(chunks.map((row) => JSON.stringify(row)).join("\n")),
      },
    });

    delete require.cache[require.resolve("../../src/handlers/rag")];
    const { ragHandlers } = await import("../../src/handlers/rag");
    const result = await ragHandlers.searchHits({
      query: "贸易中心性 金融风险 识别策略",
      collection: "中国工业经济",
      limit: 10,
      top_spans_per_item: 1,
    });

    expect(result.total).to.equal(1);
    expect(result.hits[0]).to.include({
      item_key: "ITEM42",
      title: "产业贸易中心性、贸易外向度与金融风险",
      text: "本文利用世界投入产出表和金融风险指标构造识别策略。",
      zotero_uri: "zotero://select/library/items/ITEM42",
      section_heading: "三、研究设计",
      chunk_key: "ATT42:c000001",
      query: "贸易中心性 金融风险 识别策略",
    });
    expect(result.hits[0].authors).to.deep.equal(["王姝黛", "杨子荣"]);
    expect(result.hits[0].year).to.equal(2022);
    expect(result.hits[0].venue).to.equal("中国工业经济");
    expect(result.hits[0].doi).to.equal("10.test/example");
    expect(result.hits[0].block_keys).to.deep.equal(["ATT42:p12:b08"]);
    expect(result.hits[0].attachment_key).to.equal("ATT42");
    expect(result.hits[0].section_path).to.deep.equal(["三、研究设计"]);
    expect(result.hits[0].page_idx).to.equal(12);
    expect(result.hits[0].page_range).to.deep.equal([12, 12]);
    expect(result.hits[0].bbox).to.deep.equal([10, 20, 300, 40]);
    expect(result.hits[0].evidence_refs).to.deep.equal([
      { block_key: "ATT42:p12:b08", page_idx: 12, bbox: [10, 20, 300, 40] },
    ]);
    expect(result.hits[0].score).to.be.greaterThan(0);
  });

  it("searchHits reads hidden per-PDF .zotron sidecar chunks before visible artifact attachments", async () => {
    const item = makeItem();
    const pdfAttachment = {
      id: 4201,
      key: "ATT42",
      isAttachment: () => true,
      attachmentContentType: "application/pdf",
      getField: sinon.stub().callsFake((name: string) => (
        name === "title" ? "Full Text PDF" : ""
      )),
      getFilePathAsync: sinon.stub().resolves("C:\\Users\\testuser\\Zotero\\storage\\ATT42\\paper.pdf"),
    };
    installZotero({
      Items: {
        getAsync: sinon.stub().callsFake(async (id: number | number[]) => {
          if (Array.isArray(id)) return [item];
          if (id === 42) return item;
          return pdfAttachment;
        }),
      },
      File: {
        pathToFile: sinon.stub().returns({ exists: () => true }),
        getContentsAsync: sinon.stub().callsFake(async (path: string) => {
          if (path.endsWith("\\.zotron\\chunks\\chunks.v1.jsonl")) {
            return JSON.stringify({
              item_key: "ITEM42",
              attachment_key: "ATT42",
              title: "Title",
              text: "宏观因子风险配置",
              chunk_key: "ATT42:c0",
              block_keys: ["ATT42:p1:b0"],
              evidence_refs: [{ block_key: "ATT42:p1:b0", page_idx: 1, bbox: [1, 2, 3, 4] }],
            });
          }
          return "";
        }),
      },
    });

    delete require.cache[require.resolve("../../src/handlers/rag")];
    const { ragHandlers } = await import("../../src/handlers/rag");
    const result = await ragHandlers.searchHits({
      query: "宏观因子",
      keys: [42],
    });

    expect(result.total).to.equal(1);
    expect(result.hits[0]).to.deep.include({
      item_key: "ITEM42",
      attachment_key: "ATT42",
      chunk_key: "ATT42:c0",
      page_idx: 1,
    });
    expect(result.hits[0].bbox).to.deep.equal([1, 2, 3, 4]);
  });

  it("searchHits reports embedding artifact metadata while safely using lexical fallback", async () => {
    const item = makeItem([4201, 4202]);
    const chunkAttachment = makeChunkAttachment();
    const embedAttachment = makeEmbedAttachment();
    installZotero({
      Items: {
        getAsync: sinon.stub().callsFake(async (id: number | number[]) => {
          if (Array.isArray(id)) return [item];
          if (id === 42) return item;
          if (id === 4202) return embedAttachment;
          return chunkAttachment;
        }),
      },
      File: {
        getContentsAsync: sinon.stub().resolves(JSON.stringify({
          item_key: "ITEM42",
          title: "Title",
          text: "体育产业数字化机制检验",
          chunk_key: "ITEM42:c1",
        })),
      },
    });

    delete require.cache[require.resolve("../../src/handlers/rag")];
    const { ragHandlers } = await import("../../src/handlers/rag");
    const result = await ragHandlers.searchHits({
      query: "体育产业数字化",
      keys: [42],
    });

    expect(result.total).to.equal(1);
    expect(result.retrieval).to.deep.include({
      mode: "lexical_fallback",
      semantic_available: true,
      semantic_used: false,
      embedding_artifacts: 1,
    });
    expect(result.retrieval.reason).to.match(/NPZ parsing/);
    expect(result.hits[0]).to.deep.include({
      retrieval_mode: "lexical_fallback",
      embedding_artifact_title: "ITEM42.zotron-embed.npz",
    });
  });

  it("searchHits accepts legacy chunk_id and block_ids but emits key-first fields", async () => {
    const item = makeItem();
    const attachment = makeChunkAttachment();
    installZotero({
      Items: {
        getAsync: sinon.stub().callsFake(async (id: number | number[]) => {
          if (Array.isArray(id)) return [item];
          return id === 42 ? item : attachment;
        }),
      },
      File: {
        getContentsAsync: sinon.stub().resolves(JSON.stringify({
          item_key: "ITEM42",
          title: "Title",
          text: "宏观因子风险配置",
          chunk_id: "ITEM42:c7",
          block_ids: ["ITEM42:p1:b1"],
        })),
      },
    });

    delete require.cache[require.resolve("../../src/handlers/rag")];
    const { ragHandlers } = await import("../../src/handlers/rag");
    const result = await ragHandlers.searchHits({
      query: "宏观因子",
      keys: [42],
    });

    expect(result.total).to.equal(1);
    expect(result.hits[0].chunk_key).to.equal("ITEM42:c7");
    expect(result.hits[0].block_keys).to.deep.equal(["ITEM42:p1:b1"]);
    expect(result.hits[0]).not.to.have.property("chunk_id");
    expect(result.hits[0]).not.to.have.property("block_ids");
  });

});
