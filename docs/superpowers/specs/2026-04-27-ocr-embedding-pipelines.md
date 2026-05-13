# OCR + Embedding 双 Pipeline 重整

> **Historical note (2026-05-09):** This spec predates the Rust migration.
> Current product work uses the single Rust `zotron` CLI, hidden per-PDF
> `.zotron` sidecars, and structure-first blocks/chunks. Do not implement the
> visible Zotero note / child attachment storage design from this file as the
> default path.

> 2026-04-27 起草。本 spec 只覆盖 OCR pipeline 和 embedding pipeline 两件事，不动 chunk 策略、retrieval API、MCP 形态、检索原语等其他议题（这些后续单独 spec）。

## 一句话决策

1. **保留 RAG 路线**——zotron 一次算好 embedding，结果存进 Zotero 库（child attachment），跟着 Zotero 自身的 sync 走；AI agent 直接用 retrieval 结果，不重做 embedding。
2. **OCR pipeline 扩 provider + 双写存储**——HTML note 给 Zotero UI 看，原始 markdown 单独存便于 embedding/grep 消费。
3. **Embedding pipeline 做 ProviderSpec 抽象**——出厂 10 个 preset，YAML 可扩，optional `litellm` extra 兜底 long-tail。
4. **外置 `~/.local/share/zotron/rag/*.json` 被 Zotero-native storage 覆盖（替换）**——不再用本地磁盘累积。

## 〇、之前 spec 的方向修正

`bridge-no-rag-design.md`（已删）那份的"删除 RAG，让 agent + 1M context 自己解决"是误读用户意图。真实方向是 **embedding 该做，但 storage 要在 Zotero 库内**——跟 OCR markdown 已经存进 Zotero note 是同一个思路（library 数据资产，跨设备 / 跨 AI 客户端共用）。

## 一、范围

**IN（本 spec 处理）：**

- OCR pipeline 扩 provider，支持 olmOCR / Mistral OCR / MinerU / Doubao OCR / Mathpix
- OCR 输出双写：Zotero note (HTML, user-visible) + 原始 markdown 单独存
- Embedding pipeline 抽象层（ProviderSpec + ModelSpec + 5 个 request_style adapter）
- 出厂 10 个 embedding provider preset
- Embedding storage 迁到 Zotero child attachment，删除外置 json
- 旧 `OllamaEmbedder/CloudEmbedder/DoubaoMultimodalEmbedder` 三个 class 全删

**OUT（本 spec 不处理，后续单独 spec）：**

- Chunk 策略（chunk size / overlap / late chunking / IMRaD 切分等）—— "再商量"
- Retrieval API 设计（向量 search 的具体接口、top-K 默认值、rerank、hybrid）
- 新增检索原语（grep / outline / section / download）
- MCP server 形态决策
- DOI citation graph
- 新 handler 缺口补足（attachments.download / items.template / 等）
- Web API 移植

## 二、OCR Pipeline 扩展

### 2.1 现状（保留）

`claude-plugin/python/zotron/ocr/`：
- `engine.py` 抽象 + 3 provider（GLMEngine / QwenOCREngine / CustomEngine）
- `processor.py`：collection 遍历、PDF 路径解析（含 WSL 转换）、Note HTML 生成、skip-if-exists
- 输出落 Zotero note，tag=`ocr`，body=HTML（markdown→html via Python `markdown` lib）

不动：collection 遍历、WSL 路径处理、skip 逻辑、Note HTML 渲染。

### 2.2 多 provider — `OCREngineSpec` 抽象

新增 `claude-plugin/python/zotron/ocr/spec.py`：

```python
@dataclass(frozen=True)
class OCREngineSpec:
    id: str                          # "glm" | "qwen-vl" | "olmocr" | "mistral-ocr" | "mineru" | "mathpix" | "doubao-ocr" | "openai-vision-compat"
    request_style: Literal[
        "glm-layout-parsing",        # GLM /v4/layout_parsing 专用
        "dashscope-multimodal",      # Qwen-VL DashScope 原生
        "openai-vision",             # OpenAI vision 兼容（GPT-4V / Custom / Doubao 走这里）
        "olmocr-vllm",               # vLLM 跑 olmOCR endpoint，response 字段不同
        "mistral-ocr",               # Mistral 专用 /v1/ocr
        "mineru-cli",                # subprocess CLI，本地
        "mathpix",                   # Mathpix 公式专用 API
    ]
    base_url: str | None             # None for subprocess-based
    auth: Literal["bearer", "header-key", "none"]
    auth_header: str = "Authorization"
    auth_prefix: str = "Bearer "
    supports_pdf_direct: bool = True # 否则需先 pdf→images
    max_pages_per_request: int | None = None  # 触发分页拼接
    cost_per_page_usd: float | None = None
    notes: str = ""
```

新增 `claude-plugin/python/zotron/ocr/registry.py`，列出 8 个出厂 spec。adapter 由 `request_style` 决定，每个 style 一个 class（共 7 个 adapter，覆盖目前所有商业 + 本地 OCR 选型）。

### 2.3 OCR 输出存储 — 双写

**问题**：现状只写 Zotero note (HTML)。embedding pipeline 和 grep 都要消费 markdown，每次从 HTML 反解 markdown 不可靠（公式、表格、代码块都会失真）。

**方案 A（推荐）**：Zotero **child attachment** 存 raw markdown
- 文件名：`<item-key>.zotron-ocr.md`
- mime：`text/markdown`
- 跟 Zotero storage sync 一起走
- HTML note 同时保留（user 在 UI 里能看）
- 消费方（embedding pipeline / 未来的 grep 原语）从 attachment 读

**方案 B**：第二个 Zotero note，tag=`ocr-raw`，body 是 `<pre>` 包住的 raw markdown（避免 Zotero 自己渲染破坏内容）
- 优点：不占 Zotero storage 配额（note 走数据库不走 storage 文件）
- 缺点：note size 通常有限制（Zotero 内部 sqlite 字段大）；正文带 `<pre>` 在 UI 里看着乱

**默认走方案 A**——markdown 是 file，本来就该是 attachment；Zotero 免费层 300MB 配额对个人库够用（1000 篇论文 OCR markdown 平均 50KB，总计 ~50MB）。

**实施：**

`processor.py` 的 `process_item` 写完 HTML note 之后，再调 `attachments.add` 把 `.zotron-ocr.md` 临时文件附到 item 下。已存在的同名附件先 delete 后重写（覆盖）。

### 2.4 长 PDF 分页拼接（增量）

GLM / Qwen DashScope 对 PDF 大小有限制。新增 `processor._maybe_split_pages`：用 `pymupdf` 切页 → 每 N 页一组 OCR → markdown 拼接（页与页之间插 `<!-- page N -->` marker，便于后续 page-aware grep）。

`max_pages_per_request` 在 spec 里声明，processor 按声明决定切不切。

### 2.5 retry + rate limit（增量）

httpx Transport 加 3 次指数退避；engine 层 per-spec 配置 QPS 上限（每个 spec 一个 `qps_limit` 字段）。

## 三、Embedding Pipeline 重整

### 3.1 抽象层 — `ProviderSpec` + `ModelSpec`

新模块 `claude-plugin/python/zotron/embedding/`，结构：

```
embedding/
  __init__.py
  spec.py            # ProviderSpec / ModelSpec dataclass
  registry.py        # 10 出厂 preset
  adapters/
    __init__.py
    openai.py        # OpenAI-compatible（覆盖 ~80% 厂商）
    cohere.py        # Cohere v2 /embed
    google.py        # Gemini embedding
    dashscope.py     # 阿里 DashScope 原生协议
    bedrock.py       # AWS SigV4
  embedder.py        # Embedder 统一入口，embed(texts, role)
  cli.py             # zotron embed --model X "text..."
```

```python
@dataclass(frozen=True)
class ProviderSpec:
    id: str                          # "voyage" | "dashscope" | "ollama" | ...
    base_url: str
    auth: Literal["bearer", "header-key", "sigv4", "none"]
    auth_header: str = "Authorization"
    auth_prefix: str = "Bearer "
    request_style: Literal["openai", "cohere", "google", "dashscope", "bedrock"]
    env_key_var: str
    models: list[ModelSpec]

@dataclass(frozen=True)
class ModelSpec:
    name: str
    dimensions: int | list[int]      # 单值或 Matryoshka 候选
    max_input_tokens: int
    modalities: frozenset[Literal["text", "image"]]
    # query/document 区分（每家 quirk 不同）
    input_type_field: str | None     # "input_type" | "task_type" | "task" | None
    input_type_query: str | None
    input_type_document: str | None
    # instruction prefix 风格（INSTRUCTOR / E5）
    query_instruction: str | None
    passage_instruction: str | None
    pricing_per_mtok: float | None
    license: str = "proprietary"
```

### 3.2 出厂 10 个 preset

按"装机即可用 / 学术中文 / 本地兜底"三条线选：

| # | provider | 默认模型 | 用例 |
|---|---|---|---|
| 1 | OpenAI | text-embedding-3-large | 行业基准 |
| 2 | Voyage | voyage-3-large | 学术英文 SOTA，input_type=query/document |
| 3 | Jina | jina-embeddings-v4 | 多模态 + 中英都强，task=retrieval.query/passage |
| 4 | Cohere | embed-v4 | 128K context，多语种 |
| 5 | Google | gemini-embedding-001 | task_type=RETRIEVAL_QUERY/DOCUMENT |
| 6 | DashScope | text-embedding-v4 | 国内首选，OAI 兼容，text_type 区分 |
| 7 | Zhipu | embedding-3 | 国内备选 |
| 8 | SiliconFlow | BAAI/bge-m3 | 国内便宜兜底，单 key 多模型 |
| 9 | Ollama | bge-m3 / nomic-embed-text | 本地默认 |
| 10 | TEI / vLLM 通用 | 用户自配 | 自托管学术模型（SPECTER2 / Qwen3-Embedding）通用承载 |

**注**：DeepSeek 和 Moonshot 截至 2026-04-27 仍无 embedding endpoint，不进 preset；用户需要时通过 `litellm` extra 兜底（如果他们后续上线）。

`registry.py` 把 10 条 ProviderSpec 写死；user 可在配置里覆盖 `models` / `base_url` 字段，或自定义新 ProviderSpec 追加。

### 3.3 五个 request_style adapter

每个 adapter 实现 `Adapter` Protocol：

```python
class Adapter(Protocol):
    def build_request(
        self,
        model: ModelSpec,
        texts: list[str],
        role: Literal["query", "document"],
    ) -> tuple[str, dict, dict]:   # (url, json_body, headers)
        ...
    def parse_response(self, raw: dict) -> list[list[float]]:
        ...
```

5 个 adapter 覆盖：
1. **openai** — 覆盖 OpenAI / Voyage / Jina / SiliconFlow / Ollama / TEI / vLLM / DashScope-兼容模式 / Zhipu
2. **cohere** — Cohere v2 `/embed`（`input_type` 字段、`embeddings.float` 嵌套）
3. **google** — Gemini `:embedContent`（`task_type` 字段、单 text 一个请求需要 batch loop）
4. **dashscope** — DashScope 原生 `/services/embeddings/text-embedding/text-embedding`（`text_type` 字段、`output.embeddings[].embedding`）
5. **bedrock** — AWS SigV4 签名 + 模型 ID 路由（amazon.titan / cohere.embed）

### 3.4 `python/zotron/rag/` 目录变更

| 文件 | 处置 | 原因 |
|---|---|---|
| `embedder.py` | **删** | `OllamaEmbedder` / `CloudEmbedder` / `DoubaoMultimodalEmbedder` 全部被新 `embedding/` 模块替代 |
| `search.py` | **删** | `VectorStore` 走外置 json 存储，被 Zotero-native 取代 |
| `citation.py` | **删** | `Citation` / `retrieve_with_citations` 是旧 retrieval 路径的 helper，retrieval API 本 spec 不重做（OUT），先删；后续 retrieval spec 重新设计 |
| `chunker.py` | **保留**（暂） | OCR→embed 流程仍需切片；chunk 策略本 spec 是 OUT，下一个 spec 再决定要不要重写。新 embedding pipeline 在写入前调用现有 `chunk_text`。 |
| `cli.py` | **改写为 stub** | 旧 `zotron-rag index/search/cite` 全停；改写成只剩 `zotron-rag migrate-to-zotero`（迁移命令）+ `zotron-rag --help` 打印迁移说明。原 `zotron-rag index` / `search` / `cite` 调用打印 deprecation 信息并 exit 1。 |
| `__init__.py` | 清理导出 | 移除 `Citation` / `retrieve_with_citations` / `format_citation_markdown` / `format_citation_json` |

配置层面：`[embedding]` section 含义变化——`provider` 字段从 `"ollama" \| "doubao" \| "openai" \| "zhipu" \| "dashscope"` 扩到 10 个 preset id；新增 `[embedding.providers.<id>]` 子表格供用户覆盖默认 model 列表 / base_url。`[rag]` section 本 spec 不动（chunk 策略 / top_k 等仍由 chunker 读，待后续 spec 决定去留）。

### 3.5 LiteLLM 作为 optional extra

`pyproject.toml`：

```toml
[project.optional-dependencies]
litellm = ["litellm>=1.50"]
```

`adapters/litellm.py`（仅当 `litellm` 已安装时注册），任何用户自定义 ProviderSpec 设 `request_style="litellm"` 即走 LiteLLM 兜底。

## 四、Embedding Storage — Zotero-Native（覆盖外置 json）

### 4.1 问题

现状：`~/.local/share/zotron/rag/<collection>.json` 单文件存整 collection 所有 chunks + vectors。问题：
- 文件无上限累积，磁盘越占越多
- 跨设备不同步
- 跨 AI 客户端不共用
- 用户换电脑全部失效

### 4.2 新方案：每个 item 一个 child attachment

**文件名**：`<item-key>.zotron-embed.npz`（`item-key` 是 Zotero 8 字符 key）

**mime**：`application/octet-stream`

**内容**（npz，保 float16 节省一半空间）：

```python
np.savez_compressed(
    path,
    schema_version=np.array([1]),
    embedder_id=np.array(["voyage-3-large@1024"]),  # provider/model@dim
    embedder_dim=np.array([1024]),
    source_md_sha256=np.array(["<hex>"]),           # OCR markdown 的 hash，用来失效判定
    created_at=np.array(["2026-04-27T15:30:00Z"]),
    vectors=vectors_f16,        # shape (n_chunks, dim)
    texts=np.array(chunk_texts),
    sections=np.array(chunk_sections),               # 保留切片元数据
    chunk_ranges=np.array(chunk_char_ranges),        # 每个 chunk 在 markdown 里的 (start, end)
)
```

**写入路径**：

`Embedder.persist_to_zotero(rpc, item_id, ...)`：
1. 算好 vectors
2. 写本地临时 npz 文件（`tempfile.NamedTemporaryFile`）
3. 调 `attachments.list(parentId=item_id)`，找已有 `.zotron-embed.npz`
4. 有则 `items.delete`（先删旧）
5. 调 `attachments.add(parentId=..., path=tmp_path, title="<key>.zotron-embed.npz")`
6. 删本地临时文件

### 4.3 失效检测

下次 embed 前检查：

| 字段 | 触发重 embed 条件 |
|---|---|
| `source_md_sha256` | OCR markdown hash 变了（OCR 重做了） |
| `embedder_id` | 用户改了 default model 或 dim |
| `schema_version` | spec 升级，旧文件不能用 |

### 4.4 检索时

（**虽然本 spec 不锁 retrieval API，但写入路径必须考虑读取可行性**）

读取流程：list collection items → 对每个 item 调 `attachments.list` 找 `.zotron-embed.npz` → `attachments.getPath` 拿 file 路径 → 本地 `np.load` 解码 → 余弦计算。

**注意**：每个 item 一个文件意味着检索时要 N 次磁盘读。对个人库（~1000 items 量级）可接受；上万级需要 lazy load + LRU cache，那是性能 spec 的事，不在本 spec。

### 4.5 Zotero 配额估计

每个 item embedding 文件大小：
- 30 chunks × 1024 dim × 2 bytes (float16) ≈ 60 KB
- 加 texts 和 metadata，≈ 100–150 KB
- 1000 items ≈ 100–150 MB

Zotero 免费层 300 MB，足够 1500 篇规模个人库。**默认 dim 选 1024 或更小**（voyage-3-large Matryoshka 可降到 256/512），不上 3072。

如果用户库规模超 3000 items，建议 WebDAV / 自托管 storage backend（Zotero 原生支持），无配额限制。

### 4.6 迁移路径

旧 `~/.local/share/zotron/rag/*.json` 启动时检测：
1. 如果 zotron 启动时发现存在，打 deprecation warning
2. 提供 `zotron rag migrate-to-zotero` 一次性命令：把 json 里的 vectors 拆分成 per-item npz 写回 Zotero
3. 迁移完成后用户可手动删 `~/.local/share/zotron/rag/`
4. **不**自动删（保守）

## 五、配置文件改动

`~/.config/zotron/config.toml`（举例）：

```toml
[ocr]
default_engine = "glm"           # 切到 olmOCR / mistral 等需修改这里

[ocr.engines.glm]
api_key = "${ZHIPU_API_KEY}"

[ocr.engines.olmocr]
base_url = "http://localhost:8080/v1"   # 自托管 vLLM endpoint
api_key = "EMPTY"

[embedding]
default = "dashscope/text-embedding-v4"   # provider/model 形式
dim = 1024

[embedding.providers.voyage]
api_key = "${VOYAGE_API_KEY}"

[embedding.providers.dashscope]
api_key = "${DASHSCOPE_API_KEY}"

[embedding.providers.ollama]
base_url = "http://localhost:11434/v1"

# 用户自定义 provider（覆盖 long-tail）
[embedding.providers.my-tei]
request_style = "openai"
base_url = "http://gpu-box:8080"
[[embedding.providers.my-tei.models]]
name = "allenai/specter2_base"
dimensions = 768
max_input_tokens = 512
input_type_field = null
```

废弃字段：`[rag]` section 不再被 embedding pipeline 读（chunk 策略另设；retrieval 另设）。

## 六、SDK 公共面影响

`zotron/__init__.py` 的 `__all__`：

- 删除：`Citation`、`retrieve_with_citations`、`format_citation_markdown`、`format_citation_json`（这些都是旧 RAG 路径上的 helper，新路径 retrieval API 还没定，本 spec 不重新暴露）
- 新增：`Embedder`、`OCRPipeline`（如果有外部 SDK 消费需求）
- **breaking change**：cnki-plugin 等下游需要确认无引用旧 4 个符号

`zotron` Python SDK **重置到 0.1.0**——参考 memory `feedback_rebrand_style`：rebrand / 大改不留旧名，这次属于 RAG storage paradigm 改动，重置版本号干脆。

## 七、验收标准

- [ ] OCR `OCREngineSpec` 抽象落地，registry 出厂 ≥ 8 个 spec
- [ ] OCR adapter 5 个新 request_style 有实现：olmocr-vllm / mistral-ocr / mineru-cli / mathpix / openai-vision（已有的复用）
- [ ] OCR 输出双写：Zotero HTML note + child attachment `.zotron-ocr.md`
- [ ] 长 PDF 分页拼接 + page marker 写入 markdown
- [ ] retry/QPS 配置项落地
- [ ] Embedding `ProviderSpec` + `ModelSpec` 抽象 + 5 个 adapter
- [ ] 出厂 10 个 embedding preset
- [ ] LiteLLM 作为 optional extra
- [ ] Embedding storage 全迁到 Zotero per-item child attachment
- [ ] 失效检测（hash / embedder_id / schema_version）
- [ ] `zotron rag migrate-to-zotero` 一次性迁移命令
- [ ] 旧 `OllamaEmbedder/CloudEmbedder/DoubaoMultimodalEmbedder` 删除
- [ ] SDK 公共面 `__all__` 更新；版本号重置 0.1.0
- [ ] README zh-CN / EN 同步更新（OCR provider 列表 + embedding provider 列表）
- [ ] 单元测试：每个 adapter 一个 mock-based 测试；ZoteroNative storage 一个 round-trip 测试

## 八、待用户 ACK / 决策点

逐条请你 ACK / 改：

1. **OCR markdown 双写位置**：默认走方案 A（child attachment `.zotron-ocr.md`，mime=text/markdown），不走 B（hidden note）—— ACK？
2. **OCR 多 provider 出厂列表**：8 个 spec 是 GLM / Qwen-VL / olmOCR / Mistral OCR / MinerU / Mathpix / Doubao OCR / OpenAI vision 兼容（含原 Custom）—— 需要加 / 减谁？
3. **Embedding 默认 provider**：偏向哪个？建议 `dashscope/text-embedding-v4`（国内可用 + OAI 兼容 + 成熟 API），本地兜底 `ollama/bge-m3`。要不要切到别的？
4. **Embedding 默认 dim**：1024（兼顾质量和 Zotero 配额）—— ACK？还是 768？
5. **storage 二进制格式**：`.npz`（numpy 压缩格式，紧凑）还是 `.json.gz`（人读友好）？默认 npz，理由：节省空间一半 + numpy 加载快。
6. **迁移策略**：旧 `~/.local/share/zotron/rag/*.json` 不自动删，提供 `zotron rag migrate-to-zotero` 命令 —— ACK？
7. **LiteLLM optional extra**：作为 long-tail 兜底，不进核心依赖 —— ACK？
8. **SDK 版本重置 0.1.0** —— ACK？还是 bump 到 2.0.0？
