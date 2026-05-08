# Zotron RAG / OCR Roadmap

> 2026-04-27 起草。本 roadmap 把 OCR、RAG、embedding storage、academic-zh 输出契约和 Codex 安装路径放到同一张路线图里。核心目标不是让人读 OCR 结果，而是让 Zotero 成为可检索、可定位、可复用的文献证据库。

## 0. 总判断

Zotron 应该作为 **Zotero/RAG producer**，输出带 provenance 的 retrieval hits。人仍然读 PDF；OCR 和 embedding 是机器层，用来节省 token、定位原文、支撑 academic-zh 后续生成 paper cards 和 citation map。

### 0.1 2026-05-07 修订：从 RAG 问答转向 PDF 证据与标注

当前路线收敛为 **PDF evidence and annotation pipeline**。Zotron 不内置
LLM 问答能力；Codex / Claude Code 是推理层，Zotron 只负责把 PDF 变成
可检索、可定位、可标注的证据。

修订后的核心能力是五个：

1. `parse-pdf`：把 PDF attachment 解析成结构化 blocks。主路径是 MinerU
   等 document parser；Zotero fulltext 只做便宜文本 fallback，不承担结构化解析。
2. `index-blocks`：按结构块建索引。embedding 可选，不能替代
   `block_key + page_idx + bbox + text` 这些 provenance。
3. `retrieve-blocks`：给 agent 返回证据块，不做问答、不调用 LLM。
4. `locate-highlight-target`：把 quote / block / bbox 定位成可写入 Zotero
   annotation 的目标。优先 text highlight，失败则 area-box。
5. `apply-annotations`：把定位结果写成 Zotero 原生 annotation。

由此废弃或降级的想法：

- `ask-pdf` 不进入 Zotron 核心能力；问答由 agent 完成。
- `export-annotated-pdf` 暂不列入核心 milestone；Zotero UI 已能导出嵌入标注
  的 PDF，CLI 以后可作为 convenience wrapper。
- 不把 PDF 按页/按 token 直接切 embedding 作为主路径。
- 不用 Zotero fulltext + 大量正则/LLM 反推结构文档。

第一阶段不再只做“保守修补”。既然要做，就把长期架构规划完整，并把 storage 一起纳入设计：

- OCR 结果不能只存 markdown。
- Provider 原始返回必须可审计、可重跑解析。
- RAG 不能直接消费各家 OCR 的私有格式，必须有统一中间层。
- academic-zh 不接收最终 paper card 作为主产物，优先接收一行一个 span 的 JSONL hit。
- Zotron artifact store 分成两类：可同步证据 sidecar 和本机机器缓存。
  Zotero 库里只保留真实文献、PDF attachment、人工/AI 可见 annotation。
  OCR、blocks、chunks、embedding 这类机器派生产物不能作为普通 Zotero
  note/attachment 写入，避免污染人类搜索和文献列表。

### 0.2 2026-05-07 修订：PDF attachment sidecar 存储

Zotron 可以把需要跨设备复核的证据 artifact 放进既有 PDF attachment 的
storage 目录，但不创建新的 Zotero child attachment。目标结构：

```text
Zotero Data Directory/
└── storage/
    └── <attachment-key>/
        ├── paper.pdf
        └── .zotron/
            ├── manifest.json
            ├── zotron-ocr.raw.zip
            ├── zotron-blocks.jsonl
            └── zotron-chunks.jsonl
```

这个方案的动机：

- Zotero UI 仍然只显示原 PDF，不显示 `zotron-chunks.jsonl` 等机器文件。
- Zotron 搜索不再需要过滤一堆伪附件；Zotero 人类搜索也不会被 artifact
  title 污染。
- Zotero 文件同步会压缩 attachment storage 目录。源码显示普通 dotfile 会被
  跳过，但 dot 目录下的普通文件有机会随 attachment zip 同步；落地前必须用
  真实 WebDAV/Zotero Storage 做往返验证。

约束：

- Sidecar 只放可审计、可复建检索上下文的证据文件：raw OCR/parser output、
  normalized blocks、chunks、manifest。
- `zotron-embed.npz` 默认仍放本机 artifact cache，不默认同步。embedding
  体积大、可从 chunks 重建，且更容易产生跨机器冲突。
- 插件写入 sidecar 后必须显式标记 attachment 文件同步状态；不能只依赖外部
  CLI 写文件后等待 Zotero 自动发现。
- 禁止用 ordinary Zotero note/child attachment 存机器 artifact，除非用户显式
  请求导出或调试。

## 1. 非目标

- 不把 OCR 结果设计成人类阅读界面。人类阅读源是 PDF。
- 不把 raw markdown 当唯一 truth。markdown 可以派生，但不能丢掉 page、bbox、table、figure、provider 原始字段。
- 不在 zotron 里直接生成最终 paper cards，除非同时保留 span provenance。paper card 聚合交给 academic-zh。
- 不默认复制 PDF 里的所有图片。默认保留 image reference / bbox / caption；需要视觉复核时再裁剪。
- 不做通用看图式 OCR fallback；不要把 PDF page 喂给视觉模型后让它按 prompt
  生成 OCR-like Markdown/JSON。主路径必须保留 parser/provider 的原始结构输出。
- 不做 graph RAG / citation graph。这是另一个阶段。

## 2. 三层数据模型

这里的“三层”不是为了制造冗余，而是为了隔离不同职责。

### 2.1 Provider Raw：原始证据层

Provider 返回什么，zotron 就尽量原样保存什么。

示例：

```text
GLM-OCR      -> glm-response.json
Mistral OCR  -> mistral-response.json
MinerU       -> mineru-output.zip
PaddleOCR-VL -> paddle-result.json + markdown/images/html/xlsx
olmOCR       -> dolma.jsonl + optional markdown
```

用途：

- debug：为什么某段没检索到。
- 重新 normalize：规则升级后不用重新 OCR。
- 保留 provider 专有信息：bbox、layout category、table、image crop、confidence、reading order。

建议 sidecar / artifact store 文件：

```text
storage/<attachment-key>/.zotron/zotron-ocr.raw.zip
```

如果 provider 只返回一个 JSON，可以直接放进 zip；如果 provider 返回目录，zip 保留目录结构。

### 2.2 Zotron Blocks：统一检索中间层

Zotron blocks 是统一后的 OCR / parser block JSONL。一行一个 block。它不是 embedding chunk，而是文档结构单位。

最小字段：

```json
{
  "block_id": "attABC:p12:b08",
  "type": "paragraph",
  "page": 12,
  "bbox": [72, 210, 510, 286],
  "section_heading": "三、研究设计",
  "text": "本文利用世界投入产出表和金融风险指标...",
  "source_provider": "mineru",
  "source_ref": "content_list_v2.json:42"
}
```

推荐字段：

```json
{
  "block_id": "attABC:p12:b08",
  "attachment_key": "attABC",
  "item_key": "Wang_2022_trade_risk",
  "type": "paragraph",
  "page": 12,
  "bbox": [72, 210, 510, 286],
  "reading_order": 8,
  "section_heading": "三、研究设计",
  "text": "本文利用世界投入产出表和金融风险指标...",
  "caption": "",
  "image_ref": "",
  "source_provider": "mineru",
  "source_ref": "content_list_v2.json:42",
  "confidence": 0.94
}
```

`type` 的第一版枚举：

```text
heading | paragraph | table | figure | equation | caption | footnote | header | footer | reference | unknown
```

建议 sidecar / artifact store 文件：

```text
storage/<attachment-key>/.zotron/zotron-blocks.jsonl
```

### 2.3 RAG Chunks：embedding / search 单位

Chunk 是从 blocks 组合出来的检索单位。一个 chunk 可以包含多个 block；一个过长 block 也可以拆成多个 chunk。

示例：

```json
{
  "chunk_key": "attABC:c42",
  "item_key": "Wang_2022_trade_risk",
  "attachment_key": "attABC",
  "block_keys": ["attABC:p12:b08", "attABC:p12:b09"],
  "section_heading": "三、研究设计",
  "page_start": 12,
  "page_end": 12,
  "text": "本文利用世界投入产出表...\n\n变量定义如下...",
  "char_start": 0,
  "char_end": 184,
  "level": "chunk"
}
```

建议 artifact 文件：

```text
storage/<attachment-key>/.zotron/zotron-chunks.jsonl
<zotron-artifact-cache>/items/<item-key>/attachments/<attachment-key>/zotron-embed.npz
```

`zotron-chunks.jsonl` 可以进入 PDF attachment sidecar；`zotron-embed.npz`
默认进入本机 cache。`zotron-embed.npz` 只存 vectors 和索引元数据，不替代
`zotron-chunks.jsonl`。这样调试时不用解 npz 才能看文本，换 embedding
provider 时也能从 chunks 重建。

### 2.4 Retrieval Hits：对 academic-zh 的输出层

Hit 是检索结果，不是内部存储格式。一行一个 span，用 JSONL 输出。

最小字段：

```json
{
  "item_key": "Wang_2022_trade_risk",
  "title": "产业贸易中心性、贸易外向度与金融风险",
  "text": "本文利用世界投入产出表和金融风险指标..."
}
```

推荐字段：

```json
{
  "item_key": "Wang_2022_trade_risk",
  "title": "产业贸易中心性、贸易外向度与金融风险",
  "authors": ["王姝黛", "杨子荣"],
  "year": 2022,
  "venue": "中国工业经济",
  "doi": "",
  "zotero_uri": "zotero://select/items/...",
  "section_heading": "三、研究设计",
  "chunk_key": "attABC:c42",
  "block_keys": ["attABC:p12:b08", "attABC:p12:b09"],
  "query": "贸易中心性 金融风险 识别策略",
  "score": 0.82,
  "text": "本文利用世界投入产出表和金融风险指标..."
}
```

academic-zh 后续消费 hits，自己生成：

```text
paper_cards.jsonl
citation_map.json
```

## 3. Chunking 规则

最佳实践采用结构优先、长度兜底：

```text
provider/parser blocks -> normalized blocks -> section-aware chunks -> retrieval hits
```

MVP 规则：

1. 优先使用 provider 给出的 layout / element / block。
2. 如果 provider 只给 page markdown，则按 heading / paragraph / table / caption 解析 block。
3. 不跨 section 合并 chunk。
4. 同一 section 下连续短 paragraph 可以合并到 600-1000 tokens。
5. table / figure / equation 默认单独成 block；embedding 时优先使用 caption、标题、附近正文和线性化 table text。
6. block 太长才在 block 内按句子或 token 二次拆分。
7. overlap 不用粗暴字符 overlap；优先保留 `block_keys`，必要时在 chunk text 中附带上一句/下一句。
8. 每个 hit 必须能追溯到 `chunk_key` 和 `block_keys`。

粗粒度检索需要额外 `level`：

```text
doc     = title + abstract + keywords + metadata
section = section heading + section summary/first paragraphs
chunk   = section 内的具体 span
```

查询时可以三层融合：

- 短 query / “哪篇论文讲 X”：提高 doc / section 权重。
- 长 query / “识别策略、变量定义、公式”：提高 chunk 权重。
- 精确词、人名、年份、模型名：走 grep / lexical path。

## 4. OCR Provider 路线

Provider 分三类，不要混成一种。

当前代码状态：

| 层级 | Provider | 状态 |
|---|---|---|
| 默认 live | GLM-OCR | 默认 OCR provider；走智谱 layout parsing endpoint |
| parser scaffold | MinerU | 已有 raw parser scaffold；待接本地 CLI transport |
| parser scaffold | Mistral OCR | 已有 raw parser scaffold；待接 `/v1/ocr` transport |
| parser scaffold | PaddleOCR-VL | 已有 raw parser scaffold；待接本地/服务端 transport |
| spec only | Mathpix | 公式/表格/STEM 专项候选 |
| spec only | olmOCR | 自托管英文 PDF linearization 候选 |

### 4.1 结构化 document parser 优先

第一优先级是能给结构和 provenance 的 provider：

- MinerU
- PaddleOCR-VL
- Mistral OCR
- GLM-OCR layout parsing

这些 provider 更适合作为 blocks 来源。

调研补充：

- MinerU 官方输出不止 markdown，还包括 `content_list.json`、`content_list_v2.json`、`middle.json`、layout/span debug PDFs 和图片等辅助文件；`content_list_v2.json` 按 page 分组，block 有 `type/content/bbox/anchor` 等字段，最适合直接 normalize 成 zotron blocks。
- Mistral OCR 官方 `mistral-ocr-latest` / `/v1/ocr` 返回 markdown、图片 bbox 和文档结构 metadata；新版还支持 table_format、header/footer、confidence scores 等参数，适合云端结构化 OCR。
- PaddleOCR/PaddleOCR-VL 是本地/服务化优先的开源路线；PaddleOCR-VL 1.5 面向 document parsing，适合做自托管 provider。

### 4.2 图表处理边界

Zotron 不把图像内容改写成合成文本作为检索证据。图像、插图、截图类 block
默认只保留 provider 返回的引用、caption、bbox、page 和原始 metadata；需要视觉
复核时再裁剪，不默认复制所有图片。

MVP 策略：

- 表格正常进入主线：如果 provider 返回结构化 table 或 faithful Markdown table，
  就 normalize 成 table block/chunk，并保留原始 provider 字段。
- Figure/image block 不默认进入全文语义检索文本；除非有 caption 或文档 parser
  明确给出可引用文本。
- 不引入 Qwen/Doubao/OpenAI-compatible vision 这类 prompt-only OCR fallback。

### 4.3 公式/表格专项

Mathpix 等适合公式和表格专项增强，不作为默认全文 OCR 主路径。

专项/候选池：

- Mathpix：PDF/image/document OCR，主输出 Mathpix Markdown，强项是 STEM、公式、表格、化学图。
- olmOCR：AllenAI 自托管 PDF linearization 工具，适合大批量英文 PDF / markdown 或 JSONL 输出。
- Marker：PDF -> markdown/json/html/chunks 的本地 document parser 候选，可作为非 OCR 或 forced OCR 路径。
- Azure Document Intelligence / Google Document AI / AWS Textract：企业云 OCR 候选，适合后续按用户需求加，不作为 MVP 默认依赖。

## 5. Embedding Provider 路线

Embedding provider 要抽成 registry/spec，而不是每个 provider 写一个大 class。

第一版 provider 组合：

- OpenAI compatible：OpenAI、Zhipu、DashScope compatible、SiliconFlow、TEI/vLLM。
- Voyage：支持 query/document input type。
- Jina：支持 retrieval.query / retrieval.passage。
- Cohere：embed-v4。
- Google Gemini embedding。
- Ollama：本地 fallback。

当前实现状态：

- Ollama：本地默认 fallback。
- OpenAI / Zhipu / DashScope / SiliconFlow：OpenAI-compatible payload。
- Jina：`task=retrieval.query` / `retrieval.passage`。
- Voyage：`input_type=query` / `document`。
- Cohere：`input_type=search_query` / `search_document`，解析 `embeddings.float`。
- Gemini：`taskType=RETRIEVAL_QUERY` / `RETRIEVAL_DOCUMENT`，使用 `:embedContent` endpoint。
- Doubao：保留现有 multimodal embedding adapter 和 query/corpus instructions。

关键要求：

- 建索引用 document/passage role。
- 查询用 query role。
- provider 支持 input_type/task/prefix 时必须正确区分。
- 模型维度、max tokens、modalities 写进 ModelSpec。

## 6. Zotron Artifact Store

长期主路径分为 attachment sidecar 和本机 artifact cache。Zotero 只保留真实
文献、PDF attachment 和可见 annotation。每篇 item/attachment 的证据产物路径：

```text
storage/<attachment-key>/.zotron/manifest.json
storage/<attachment-key>/.zotron/zotron-ocr.raw.zip
storage/<attachment-key>/.zotron/zotron-blocks.jsonl
storage/<attachment-key>/.zotron/zotron-chunks.jsonl
<zotron-artifact-cache>/items/<item-key>/attachments/<attachment-key>/zotron-embed.npz
```

Sidecar 负责可同步、可复核的证据；本机 cache 负责可重建的大型机器索引。

可选的人类预览必须显式 opt-in，且不能作为默认流程：

```text
OCR Preview note, tag=ocr
```

但 HTML note 不是 RAG source of truth。它只是兼容现有用户习惯或调试预览。

失效检测：

```text
PDF attachment hash 变了 -> OCR stale
provider/model/config 变了 -> OCR stale
blocks schema version 变了 -> blocks stale
chunking config 变了 -> chunks stale
embedding provider/model/dim 变了 -> embed stale
```

`zotron-embed.npz` 建议字段：

```text
schema_version
embedder_id
embedder_dim
source_chunks_sha256
created_at
chunk_keys
vectors
```

## 7. RPC / CLI Contract

### 7.1 OCR

```text
zotron-ocr run --collection "中国工业经济"
zotron-ocr status --collection "中国工业经济"
zotron-ocr rebuild --item <item-id>
```

内部写入 attachment sidecar 或本机 artifact cache，不写入普通 Zotero
note/child attachment。

### 7.2 RAG Index

```text
zotron-rag index --collection "中国工业经济"
zotron-rag status --collection "中国工业经济"
zotron-rag migrate-to-zotero
```

旧 `~/.local/share/zotron/rag/*.json`：

- 不自动删除。
- 提供迁移命令。
- README 标注 deprecated。

### 7.3 Retrieval Hits

建议不要叫 `cards`，避免和 academic-zh paper cards 混淆。更清楚的名字：

```text
rag.searchHits
zotron-rag hits
```

请求：

```json
{
  "query": "贸易中心性 金融风险 识别策略",
  "collection": "中国工业经济",
  "limit": 50,
  "top_spans_per_item": 3,
  "include_fulltext_spans": true
}
```

返回：

```json
{
  "hits": [
    {
      "item_key": "Wang_2022_trade_risk",
      "title": "产业贸易中心性、贸易外向度与金融风险",
      "authors": ["王姝黛", "杨子荣"],
      "year": 2022,
      "venue": "中国工业经济",
      "doi": "",
      "zotero_uri": "zotero://select/items/...",
      "section_heading": "三、研究设计",
      "chunk_key": "attABC:c42",
      "block_keys": ["attABC:p12:b08"],
      "query": "贸易中心性 金融风险 识别策略",
      "score": 0.82,
      "text": "本文利用世界投入产出表和金融风险指标..."
    }
  ],
  "total": 50
}
```

JSONL 输出：

```text
zotron-rag hits "贸易中心性 金融风险 识别策略" --collection "中国工业经济" --output jsonl
```

## 8. Codex / Code CLI 安装路径

现有 README 是 Claude Code-first。不要破坏原格式，只增加并列路径。

建议 README 安装结构：

```text
Path A -- Claude Code
Path B -- Codex / Code CLI
Path C -- Python CLI only
Path D -- Manual XPI + CLI
```

Codex 路径第一阶段只做文档和可复制目录，不强行发布 marketplace：

```text
codex-plugin/
  .codex-plugin/plugin.json
  skills/zotero/SKILL.md
  skills/zotero/*.md
  agents/zotero-researcher.md
  bin/zotron
  bin/zotron-ocr
  bin/zotron-rag
```

如果后续确认 Codex 插件规范与 Claude plugin 可以共用大部分文件，再减少重复：

```text
agent-plugin/
  claude/
  codex/
  shared/
```

第一阶段验收：

- README.md / README.zh-CN.md 有 Codex install path。
- Codex 用户知道如何安装 Python CLI、XPI，并把 `zotron*` 命令放到 PATH。
- 不改变 Claude Code 原 setup flow。

## 9. Roadmap

本 roadmap 以 `docs/2026-05-07-pdf-evidence-annotation-milestones.md`
中的 milestone 为当前执行准线。下列早期 phase 仍保留历史上下文；如果与
2026-05-07 milestone 冲突，以 milestone 文档为准。

### Phase 0 -- Contract Freeze

产出：

- 本 roadmap。
- `docs/api-stability.md` 更新 retrieval hits JSONL contract。
- 明确 `rag.searchHits` / `zotron-rag hits` 命名。
- 明确 blocks/chunks/hits schema version。
- 明确 key-first contract：RAG/OCR/PDF evidence 新输出不使用 `*_id` 字段。
- 明确 `parse-pdf` / `index-blocks` / `retrieve-blocks` /
  `locate-highlight-target` / `apply-annotations` 是当前能力边界。

验收：

- academic-zh 可以按 JSONL contract 开始对接。
- README 不再暗示 RAG 只返回 paper-level result。
- PRD 中不再把 `ask-pdf` 作为 Zotron 内置服务。

### Phase 1 -- Storage + Schema Foundation

产出：

- Zotron artifact helper：add/list/delete/find by item/attachment key and
  artifact kind，支持 attachment sidecar 和本机 cache 两类 backend。
- `provider_raw` zip 写入。
- `zotron-blocks.jsonl` 写入。
- `zotron-chunks.jsonl` 写入。
- `zotron-embed.npz` 本机 cache 写入/读取。
- stale 检测字段。

验收：

- 单篇论文 OCR 后能看到 sidecar/cache artifacts，Zotero item 列表/search
  不新增机器派生条目。
- 不依赖 HTML note 做 RAG source。
- 本地临时文件不会残留。

### Phase 2 -- OCR Provider Adapters

产出：

- `OCREngineSpec` / registry。
- Adapter：GLM、Mistral、MinerU、PaddleOCR-VL。
- Provider raw -> zotron blocks normalizer。

验收：

- 每个 adapter 有 mock-based test。
- 同一篇 sample PDF 用不同 provider 后都能生成 `zotron-blocks.jsonl`。
- 图片 block 只保存 refs/caption/bbox，不默认复制全部图片。

### Phase 3 -- Structure-First Chunking

产出：

- blocks -> chunks builder。
- doc / section / chunk 三层 level。
- table/figure/equation chunk policy。
- chunk provenance：`block_keys`、page range、section heading。
- `retrieve-blocks` CLI/RPC：返回原文证据块，不做 LLM synthesis。

验收：

- chunk 不跨 section。
- hit 能回查 block 和 PDF page。
- 中文论文标题、摘要、章节、表格 caption 不被粗暴切断。

### Phase 4 -- Embedding Provider Registry

产出：

- `ProviderSpec` / `ModelSpec`。
- OpenAI-compatible adapter。
- Voyage / Jina / Cohere / Google / DashScope / Ollama support。
- query/document role 区分。
- embeddings 写入本机 artifact cache 的 `zotron-embed.npz`。

验收：

- 同一 chunk 用不同 provider 可 embed。
- 查询和建索引用不同 role。
- 旧 `zotron-rag index/search/cite` 有兼容或清晰 deprecation。

### Phase 5 -- Retrieval Hits for academic-zh

产出：

- `rag.searchHits` JSON-RPC method。
- `zotron-rag hits --output jsonl`。
- `top_spans_per_item`。
- `include_fulltext_spans`。
- optional grep/hybrid path。
- `locate-highlight-target` 第一版：quote/block/bbox -> area annotation target。
- `apply-annotations` 第一版：area-box targets -> Zotero native annotations。

验收：

- 输出一行一个 hit。
- 每个 hit 至少有 `item_key/title/text`。
- 推荐字段齐全时 academic-zh 能生成 `paper_cards.jsonl` 和 `citation_map.json`。
- `text` 是可引用原文 span，不是泛泛摘要。
- 对至少一个 MinerU block bbox 能创建可见 Zotero annotation。
- 对 text-layer PDF 的 quote 定位失败时，能自动降级为 bbox area annotation。

### Phase 6 -- Codex Install Surface

产出：

- README Codex path。
- README.zh-CN 同步。
- 可选 `codex-plugin/` scaffold。
- 安装/验证命令复用现有 XPI + Python CLI。

验收：

- Claude Code 用户路径不退化。
- Codex/code-cli 用户能安装 XPI、安装 CLI、调用 zotron。
- README 格式保持 Path A/B/C/D 风格。

### Phase 7 -- Migration + Compatibility

产出：

- `zotron-rag migrate-to-zotero`。
- 旧 JSON index 检测和 warning。
- 迁移文档。

验收：

- 旧索引不自动删除。
- 用户可以手动迁移。
- 迁移失败不破坏旧数据。

## 10. 测试策略

单元测试：

- OCR adapter response parsing。
- Provider raw zip round trip。
- Blocks schema validation。
- Blocks -> chunks。
- Embedding npz round trip。
- Retrieval hits formatting。

集成测试：

- Mock Zotero RPC：附件写入/读取/覆盖。
- Sample Chinese academic markdown/blocks：验证 section-aware chunking。
- academic-zh fixture：hits JSONL 可被下游读取。

回归测试：

- 现有 `zotron-rag cite/search` 的兼容行为。
- 现有 `zotron-ocr status/run` 的 CLI 参数。
- README 命令仍可复制执行。

## 11. 关键风险

- 如果机器派生产物继续写成 Zotero child attachment，Zotero UI 和搜索会被污染；
  默认必须写入 attachment sidecar 或本机 artifact cache。
- `.npz` 不是人读格式，所以必须保留 `.zotron-chunks.jsonl`。
- Provider bbox 坐标系可能不同，需要记录 coordinate system。
- 看图式 OCR-like provider 不进入主线，避免 prompt 生成文本污染原始证据。
- Full Zotero storage sync 会占配额；图片 crop 默认不复制是必要约束。
- 一次改动较大，应按 phase 合并，避免一个 PR 同时改 OCR、embedding、README、RPC。

## 12. 推荐执行顺序

先做 schema/storage，再做 provider，再做 chunking，再做 retrieval API：

```text
Contract Freeze
  -> Zotron sidecar/cache artifact store
  -> OCR raw + blocks
  -> blocks -> chunks
  -> embedding provider registry + npz
  -> rag.searchHits / JSONL
  -> Codex README/install
  -> migration
```

这个顺序的原因：如果先做 provider 或 chunking，没有稳定 artifact schema，后面会反复改数据格式；如果先做 retrieval API，没有 blocks/chunks provenance，academic-zh 拿到的 hit 不够稳。
