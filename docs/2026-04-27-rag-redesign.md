# RAG 与 Embedding 重设计

> Historical Python RAG redesign note. Current `rust-migration` branch treats
> Python as reference material only. The active product path is Rust+JS:
> `zotron ocr parse-pdf` produces hidden sidecar blocks/chunks and
> `zotron rag hits --zotero` reads provenance-rich hits through the XPI.

> 2026-04-27 起草。诊断当前 RAG 流水线的两个症结——provider 接入面太窄、embedding 策略颗粒度单一——并给出按 ROI 排序的修法。

## 范围

只覆盖 `claude-plugin/python/zotron/rag/` 下的 chunker / embedder / search / cli 四个文件，外加 cli 入口对 OCR 文本的取数路径。不涉及 XPI 侧的 OCR 写入（那是 `ocr/` 的职责）。

对应 charter 的位置：RAG 是 bridge "读" 那一半的核心——AI 通过 RAG 拿到带 provenance 的引用，然后在 AI 客户端里写综述。所以这次重设计的判据始终是"让读出来的东西更准、更可溯源"，不是把 RAG 做成检索产品。

## 一、Provider 现状与缺口

### 现状

`embedder.py` 提供三个类：

| 类 | 覆盖 |
|---|---|
| `OllamaEmbedder` | 本地 Ollama，唯一本地通路 |
| `CloudEmbedder` | OpenAI 兼容协议：openai / zhipu / dashscope / doubao（文本） |
| `DoubaoMultimodalEmbedder` | 历史 Python 多模态版，带 query/corpus instructions + ThreadPool 并发 |

### 缺口

按"中文学术 + AI agent"场景影响排序：

| 缺的 provider | 价值 |
|---|---|
| **Voyage** (`voyage-3`, `voyage-3-large`) | Anthropic 官方推荐，英文学术 SOTA；`input_type` 字段区分 query/document，与 doubao 那套 instruction 同源 |
| **SiliconFlow**（硅基流动）| 国内聚合器，免费额度大，单 key 多模型；`bge-m3` / `bge-large-zh-v1.5` 在中文学术几乎是事实标准 |
| **Jina v3** (`jina-embeddings-v3`) | 多语言 + 任务 prefix（`retrieval.query` / `retrieval.passage`）；可商用，定价低 |
| **本地 OpenAI 兼容 endpoint** | vLLM / TEI / lmdeploy 用户无法走 ollama，但 `CloudEmbedder` 实际已经能干，只差一个 preset 和文档 |

### 真正的问题不是数量，是抽象

`_QUERY_INSTRUCTION` / `_CORPUS_INSTRUCTION` 写死在历史 `DoubaoMultimodalEmbedder` 里。Voyage、Jina、BGE 都有 query/document 区分，但实现路径各不相同：

- Voyage：API 字段 `input_type: "query" | "document"`
- Jina：API 字段 `task: "retrieval.query" | "retrieval.passage"`
- BGE：客户端拼前缀字符串 `"为这个句子生成表示用于检索相关文章："`
- Volcengine/Doubao text embedding：客户端拼 `Instruction:...\nQuery:` 字符串

每加一个 provider 就 fork 一个 `Embedder` 子类是错的。应抽出一个 provider spec：

```python
@dataclass
class ProviderSpec:
    base_url: str
    payload_style: Literal["openai", "ollama"]   # 决定 input 字段名 / data 字段位置
    query_strategy: Literal["prefix", "input_type", "task", "none"]
    query_value: str | None                       # prefix 字符串 / "query" / "retrieval.query"
    passage_value: str | None
    concurrency: int                               # 是否需要客户端并发（多模态那种）
```

然后 `create_embedder` 按 provider 名查表组装一个统一 `Embedder` 实例。

## 二、Embedding 策略问题（按伤害排序）

### A. 单一颗粒度（最大问题）

`chunk_text` 默认 512 字符 + 50/64 overlap，所有 query 共用一种 chunk。但实际查询天然分两类：

- **粗粒度查询**："哪篇论文讲了 X" / "X 这个概念是谁提的" → 应 hit **title + abstract**
- **细粒度查询**："X 的具体公式 / 实验数据" → 应 hit **段落 / 句子**

现在 title 和 abstract 跟正文一起切碎，粗粒度 query 召回被稀释。

**修法**：分层索引。三套 vector 共存：

1. **doc-level**：title + abstract + 关键词（一个 item 一条）
2. **section-level**：每个识别出的章节合并成一条
3. **chunk-level**：现有 512 字符切片

查询时三层都查，按 query 长度 / 类型加权融合（短 query 偏 doc-level，长 query 偏 chunk-level）。存储上每条 chunk 加 `level: "doc" | "section" | "chunk"` 字段。

### B. 没有 grep（grep vs RAG）

引用编号（`[12]`）、人名、数据集名、模型名（`DeepSeek-V3`）、年份这类 query，embedding 召回经常不如 `ripgrep`。学术场景里这类 query 占比不低，且 grep 零成本零延迟。

**修法**：在 Rust `zotron rag ...` 下增加 lexical/hybrid 检索路径，直接对
sidecar chunks 做精确匹配，结果 schema 与 `hits` 对齐（`item_key` /
`attachment_key` / `chunk_key` / `title` / `text` / `score`）。grep 不替代
semantic retrieval，是另一条调用路径——agent 在 prompt 里会被告知"精确匹配用
lexical，语义模糊用 semantic/hybrid"。

更上一层是 hybrid（BM25 + dense + rerank），但 grep 是最低成本的第一步，且与 charter 一致（不当产品，给 AI 提供互补的检索原语）。

### C. 没有 reranker

top-10 dense 召回的 precision 在中文学术里偏低。Voyage rerank-2 / Jina reranker / BGE reranker-v2-m3 接一个，top-K 质量能跳一档。

**修法**：retrieval 后处理层。未来在 `zotron rag hits` 或后续
sidecar-backed search 中增加 `--rerank` 开关，召回 top-50 → rerank →
top-10。不动 blocks/chunks sidecar。reranker provider 与 embedder 的 spec
可以复用同一套抽象（只是 API endpoint / payload 不同）。

### D. 存储退化（性能问题，非正确性）

现状：`VectorStore` 整个 collection 存为 JSON，每次 search load + parse + 重新 norm。库到 1 万 chunk 就明显卡。

**修法路线**：

1. 短期：sqlite（chunks 表）+ numpy memmap（vectors 文件），search 时仅加载 vectors 块
2. 长期：lancedb / chroma / qdrant（嵌入式模式，单文件，不要求用户起 server）

不紧急，但分层索引上线后 chunk 数会翻 1.5–2 倍，会更早触发瓶颈。

### E. metadata 太薄（影响 citation precision）

chunk 现在只记 `section + chunk_index`。但 charter 强调"可追溯"——应该记到：

- `page_number`（OCR 时 page 信息要传下来）
- `bbox`（如果 OCR 提供，便于 PDF reader 跳转高亮）
- `figure_ref` / `equation_ref`（章节内编号）

OCR 那层是源头，需要把 page 写进 chunk metadata；chunker 透传；citation 输出时带上。这一项必须配合 `ocr/` 改动一起做。

### F. provider-aware prefix 缺位（A 节抽象的副产物）

修完 A 节的 provider spec 后，`embed_batch`（建索引）走 passage prefix / `input_type=document`，`embed`（查询）走 query prefix / `input_type=query`。当前只有 doubao 这么做，其他 provider 都把 query 和 corpus 当同一种东西编码——浪费了模型自身的不对称设计。

## 三、ROI 排序的执行顺序

| 顺序 | 项目 | 工作量 | 收益 |
|---|---|---|---|
| 1 | grep 子命令 | 半天 | 立刻可用，charter 完美契合，零依赖 |
| 2 | provider spec 抽象 + 加 voyage / siliconflow / jina | 一天 | 解决"接得少"+ 副产物修复 F |
| 3 | 分层 chunking（doc summary / section / chunk） | 两到三天 | 召回质量跳变最大 |
| 4 | reranker 后处理层 | 一天 | precision 再跳一档 |
| 5 | metadata 传递（page / figure / equation） | 配合 OCR 改一起做 | citation 真·可溯源 |
| 6 | 存储替换（sqlite memmap → lancedb） | 一两天 | 1 万+ chunk 才明显，可拖 |

每一步都可以独立合并，不需要打包成一个大 PR。

## 四、不在本次范围

- 不做 query rewriting / multi-query expansion——那是 AI 客户端侧 agent 自己的事（charter：写作工作流不在 bridge 内）
- 不做 graph RAG / 引用图谱——单独议题，与本次的颗粒度问题正交
- 不做检索结果摘要——RAG 返回的就是原文 chunk + provenance，摘要由消费方做

## 五、待确认

- 分层索引第 1 层（doc-level）的输入：是 title + abstract 拼起来直接 embed，还是先 LLM 生成一条"摘要 + 关键词"再 embed？后者更好但引入额外依赖，建议 v1 用前者。
- grep 子命令的输入源：应该 grep 原始 OCR 文本（含格式 noise），还是 chunked 文本（已清洗）？倾向 grep 原始文本——召回率优先，正文里的精确字符串不该被 chunk 边界切断。
