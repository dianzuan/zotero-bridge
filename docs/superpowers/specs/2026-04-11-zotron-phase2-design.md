# Zotron Phase 2: OCR + RAG 设计文档

> **Historical note (2026-05-09):** This design predates the Rust migration.
> The current branch does not expose `zotron-ocr` / `zotron-rag` as product
> commands. Use `zotron ocr ...` and `zotron rag ...`; Python remains reference
> material only.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 为 Zotron 添加课题级 PDF OCR 和段落级 RAG 语义搜索，全部通过 Python CLI 工具实现，XPI 不改动。服务于中文学术论文写作流程：CNKI 收集文献 → OCR 提取全文 → RAG 语义检索 → 文献综述写作。

**Architecture:** Python 脚本通过现有 JSON-RPC 接口与 Zotero 交互。OCR 调云端 API 将 PDF 直接转为 Markdown 并写回 Zotero Note。RAG 按 Zotero 集合（collection）为单位建索引，提供段落级语义检索。配置通过环境变量或 config 文件管理。

**Tech Stack:** Python 3.11+, httpx, numpy, Ollama (本地 embedding), 云端 OCR/Embedding API

---

## 1. 用户工作流

```
1. CNKI skill 搜索下载 → 存入 Zotero 集合（如"数字经济"）
2. 积累 30-100 篇相关文献
3. zotron-ocr --collection "数字经济"    ← 批量 OCR 该集合的 PDF
4. zotron-rag index --collection "数字经济"  ← 建该集合的语义索引
5. zotron-rag search --collection "数字经济" "就业效应的异质性分析方法"
   → 返回最相关的段落 + 出处
6. researcher agent 辅助文献综述写作，引用填入论文
```

**核心概念：课题级工作集。** 不是全库索引，而是针对当前研究课题的一个 Zotero 集合（几十到一百篇）进行 OCR 和 RAG。

## 2. 整体架构

```
Claude Code / researcher agent
    │
    ├── zotero-cli (Phase 1, 不改)  ←── JSON-RPC ──→  XPI (不改)
    │
    ├── zotron-ocr (新增 Python CLI)
    │       │
    │       ├── 获取集合内论文的 PDF 路径 ← zotero-cli
    │       ├── PDF 直接发送到云端 OCR API → Markdown
    │       └── 写回 Zotero Note ← zotero-cli notes.create
    │
    └── zotron-rag (新增 Python CLI)
            │
            ├── 拉取集合内文献文本 ← zotero-cli
            ├── 分块 + embedding → JSON 向量文件
            └── 语义检索 → 返回段落 + item ID + 论文元数据
```

**XPI 零改动。** Phase 1 的 JSON-RPC method 已提供全部数据接口。

## 3. 配置管理

### 3.1 配置文件

路径：`~/.config/zotron/config.json`

```json
{
  "ocr": {
    "provider": "glm",
    "api_key": "xxx",
    "api_url": null
  },
  "embedding": {
    "provider": "ollama",
    "model": "qwen3-embedding:4b",
    "api_key": null,
    "api_url": "http://localhost:11434"
  },
  "rag": {
    "chunk_size": 512,
    "chunk_overlap": 50,
    "top_k": 10
  },
  "zotero": {
    "rpc_url": "http://localhost:23119/zotron/rpc"
  }
}
```

### 3.2 环境变量覆盖

环境变量优先级高于 config 文件：

| 环境变量 | 说明 |
|---------|------|
| `ZOTRON_RPC_URL` | RPC 端点（已有） |
| `OCR_PROVIDER` | glm / paddleocr / custom |
| `OCR_API_KEY` | OCR 服务 API Key |
| `EMBEDDING_PROVIDER` | ollama / zhipu / doubao / openai / deepseek |
| `EMBEDDING_MODEL` | 模型名称 |
| `EMBEDDING_API_KEY` | 云端 embedding API Key |

### 3.3 首次运行

首次运行时，如果没有 config 文件也没有环境变量，交互式提示用户选择 provider 并输入 API key，然后写入 config 文件。

## 4. OCR 模块 (zotron-ocr)

### 4.1 功能

将 Zotero 集合中的 PDF 通过云端 OCR API 直接转为 Markdown，结果作为 Note 写回 Zotero。云端 OCR API 直接接收 PDF 文件，不需要先转图片。

### 4.2 支持的 OCR 引擎

| Provider | 服务 | 中文学术质量 | 备注 |
|----------|------|------------|------|
| `glm` | 智谱 GLM-4V OCR | 最佳 (94.62) | 默认推荐，直接吃 PDF |
| `paddleocr` | PaddleOCR-VL API | 优秀 (94.50) | 开源模型的云端版 |
| `custom` | 自定义 URL | 取决于模型 | 兼容 OpenAI vision API 格式 |

### 4.3 CLI 接口

```bash
# 按集合批量 OCR（核心用法）
zotron-ocr --collection "数字经济"

# 单篇 OCR
zotron-ocr --item 12345

# 强制重新 OCR（已有 OCR Note 也重做）
zotron-ocr --collection "数字经济" --force

# 查看集合 OCR 状态（已 OCR / 未 OCR / 总数）
zotron-ocr status --collection "数字经济"
```

### 4.4 处理流程

```
zotron-ocr --collection "数字经济"
  1. zotero-cli collections.tree
     → 找到"数字经济"集合 ID
  2. zotero-cli collections.getItems '{"id": <colId>}'
     → 获取集合内所有条目
  3. 对每个条目：
     a. 检查是否已有 "ocr" 标签的 Note → 有则跳过（除非 --force）
     b. zotero-cli attachments.list '{"parentId": <itemId>}'
        → 找到 PDF 附件
     c. zotero-cli attachments.getPath '{"id": <attachmentId>}'
        → 获取 PDF 文件路径
     d. 读取 PDF 文件 → 发送到 OCR API → 收到 Markdown
     e. Markdown → HTML 转换
     f. zotero-cli notes.create '{"parentId": <itemId>, "content": "<html>", "tags": ["ocr"]}'
        → 写回 Zotero Note，打上 "ocr" 标签
  4. 输出统计：成功 N 篇，跳过 M 篇，失败 K 篇
```

### 4.5 OCR Note 格式

```html
<h1>OCR: {论文标题}</h1>
<p><em>OCR by {provider} | {date} | {page_count} pages</em></p>
<hr/>
{markdown_converted_to_html}
```

### 4.6 为什么所有论文都需要 OCR

Zotero 内置的 `getFulltext` 提取质量一般（尤其中文 PDF），表格、公式、多栏排版经常乱码。云端 OCR（GLM-4V 等视觉模型）能产出结构化 Markdown，保留标题层级、表格、公式，质量远超纯文本提取。因此 OCR 不仅是"扫描件补救"，而是所有论文获取高质量文本的首选方式。

## 5. RAG 模块 (zotron-rag)

### 5.1 功能与动机

按 Zotero 集合为单位，将文献文本分块 embedding，提供段落级语义检索。典型工作集：30-100 篇论文。

**为什么需要 RAG（token 节省）：** 50-100 篇规模下，关键词搜索 + Claude 读全文也能完成文献综述，检索质量差别不大。但 token 成本差 10 倍：

| 方式 | 流程 | 每次查询 token |
|------|------|---------------|
| 不用 RAG | search.quick → Claude 挑 5 篇 → 读 5 篇全文 | ~50K |
| 用 RAG | embedding 搜索 → 直接返回 10 个相关段落 | ~5K |

写一篇论文需要查询几十次，累计节省巨大。RAG 的价值不是"搜得更准"，而是**让 Claude 跳过读全文，直接拿到需要的段落**。

### 5.2 支持的 Embedding 引擎

| Provider | 模型 | 类型 | 中文质量 | 备注 |
|----------|------|------|---------|------|
| `ollama` | Qwen3-Embedding-4B | 本地 | 最强 | 默认推荐，需 Ollama |
| `ollama` | BGE-M3 | 本地 | 强 | 支持稠密+稀疏 |
| `zhipu` | embedding-3 | 云端 | 强 | 智谱，国内快 |
| `doubao` | doubao-embedding | 云端 | 强 | 字节豆包，便宜 |
| `openai` | text-embedding-3-small | 云端 | 一般中文 | 英文强 |
| `deepseek` | deepseek-embedding | 云端 | 强 | APAC 折扣 |

### 5.3 CLI 接口

```bash
# 为集合建索引（核心用法）
zotron-rag index --collection "数字经济"

# 语义搜索
zotron-rag search --collection "数字经济" "就业效应的异质性分析方法"

# 查看索引状态
zotron-rag status --collection "数字经济"

# 重建索引（集合内容有变动时）
zotron-rag index --collection "数字经济" --rebuild
```

### 5.4 索引流程

```
zotron-rag index --collection "数字经济"
  1. zotero-cli collections.tree → 找到集合 ID
  2. zotero-cli collections.getItems → 获取条目列表
  3. 对每个条目，获取最佳文本源（优先级）：
     a. OCR Note（"ocr" 标签）→ 质量最高
     b. getFulltext → 退而求其次
     c. 都没有 → 跳过，提示用户先 OCR
  4. 文本分块：
     - 按章节标题切分（# 标题 / "一、" / "1." 等模式）
     - 每个章节内递归切分，目标 512 token，50 token 重叠
     - 每个 chunk 携带元数据：item_id, title, authors, section, chunk_index
  5. 调用 embedding 模型获取向量
  6. 存入 JSON 向量文件
```

### 5.5 数据存储

30-100 篇论文的向量数据很小（几 MB），用 JSON 文件 + numpy 余弦相似度即可，不需要 LanceDB。

存储路径：`~/.local/share/zotron/rag/{collection_name}.json`

```json
{
  "collection": "数字经济",
  "collection_id": 42,
  "model": "qwen3-embedding:4b",
  "created_at": "2026-04-11T20:00:00",
  "chunks": [
    {
      "item_id": 12345,
      "title": "数字经济与就业结构转型",
      "authors": "张三, 李四",
      "section": "四、实证结果",
      "chunk_index": 12,
      "text": "回归结果显示，数字经济发展水平每提高1%...",
      "vector": [0.012, -0.034, ...]
    }
  ]
}
```

### 5.6 检索流程

```
zotron-rag search --collection "数字经济" "query"
  1. query → embedding 向量
  2. numpy 余弦相似度计算 → 排序
  3. 取 top_k 结果（默认 10）
  4. 输出 JSON：
     [
       {
         "item_id": 12345,
         "title": "数字经济与就业结构转型",
         "authors": "张三, 李四",
         "year": "2024",
         "journal": "经济研究",
         "section": "四、实证结果",
         "text": "回归结果显示，数字经济发展水平每提高1%...",
         "score": 0.87
       }
     ]
```

30-100 篇的暴力搜索在毫秒级完成，不需要近似最近邻索引。

## 6. CNKI 元数据对接

XPI 和 Python 工具均不涉及 CNKI 抓取逻辑。CNKI skill 体系（Playwright + Python 验证码）负责抓取，通过现有 CLI 接口写入 Zotero：

```bash
# CNKI skill 抓取完元数据后：
zotero-cli items.update '{"id": 12345, "fields": {
  "title": "数字经济对就业的影响研究",
  "abstractNote": "本文基于...",
  "DOI": "10.xxxx/yyyy",
  "publicationTitle": "经济研究"
}, "creators": [...]}'
```

Phase 1 的 `items.update` 已支持全部字段更新，无需额外开发。

## 7. Claude Code Skill 更新

### 7.1 新增 zotron-ocr skill

触发词："OCR"、"扫描件"、"识别PDF"、"PDF转文字"、"提取全文"
流程：调用 `zotron-ocr` CLI，按集合或单篇处理。

### 7.2 新增 zotron-rag skill

触发词："找相关段落"、"语义搜索"、"文献综述"、"前人研究"、"相关文献怎么说"
流程：调用 `zotron-rag search`，返回相关段落供写作引用。

### 7.3 更新 zotero-researcher agent

在 agent 工具表中添加 `zotron-ocr` 和 `zotron-rag`，更新工作流：

```
写论文文献综述时：
1. 确认工作集合 → zotron-rag status 检查索引
2. 没索引 → 先 zotron-ocr + zotron-rag index
3. 按研究问题语义搜索 → 找到相关段落
4. 综合多篇论文的相关段落 → 组织文献综述
5. 自动生成 GB/T 7714 引用格式
```

## 8. Python 包结构

```
zotron/
├── src/                    # TypeScript XPI (不改)
├── claude-plugin/          # Skills + agents
│   ├── bin/
│   │   ├── zotero-cli      # 现有，不改
│   │   ├── zotron-ocr      # 新增：bash 入口，uv run 调 Python
│   │   └── zotron-rag      # 新增：bash 入口，uv run 调 Python
│   ├── skills/
│   │   ├── zotero-search/   # 现有
│   │   ├── zotero-manage/   # 现有
│   │   ├── zotero-export/   # 现有
│   │   ├── zotron-ocr/      # 新增
│   │   └── zotron-rag/      # 新增
│   └── agents/
│       └── zotero-researcher.md  # 更新
└── python/                  # 新增：Python 包
    ├── pyproject.toml
    └── zotron/
        ├── __init__.py
        ├── config.py        # 配置加载（config file + env vars）
        ├── rpc.py           # httpx 直接调 JSON-RPC（不走 bash CLI）
        ├── ocr/
        │   ├── __init__.py
        │   ├── cli.py       # zotron-ocr CLI 入口 (argparse)
        │   ├── engine.py    # OCR 引擎抽象 + GLM/PaddleOCR/custom 实现
        │   └── processor.py # 集合遍历 → OCR → 写 Note 流程
        └── rag/
            ├── __init__.py
            ├── cli.py       # zotron-rag CLI 入口 (argparse)
            ├── chunker.py   # 章节感知 + 递归切分
            ├── embedder.py  # embedding 引擎抽象 + Ollama/云端实现
            └── search.py    # JSON 向量存储 + numpy 余弦检索
```

## 9. 依赖

```toml
[project]
name = "zotron-tools"
requires-python = ">=3.11"
dependencies = [
    "httpx>=0.27",           # HTTP 客户端（OCR API / embedding API / Zotero RPC）
    "numpy>=1.24",           # 向量计算（余弦相似度）
    "markdown>=3.7",         # Markdown → HTML（OCR 结果写 Note）
]

[project.optional-dependencies]
local = [
    "ollama>=0.4",           # 本地 embedding via Ollama
]

[project.scripts]
zotron-ocr = "zotron.ocr.cli:main"
zotron-rag = "zotron.rag.cli:main"
```

## 10. 实施顺序

1. **Python 包骨架 + 配置模块** — pyproject.toml, config.py, rpc.py
2. **OCR 模块** — engine.py, processor.py, cli.py
3. **RAG 分块** — chunker.py（纯文本处理，可独立测试）
4. **RAG embedding + 检索** — embedder.py, search.py, cli.py
5. **bin 入口脚本** — zotron-ocr, zotron-rag (bash wrapper, uv run)
6. **Claude Code skills** — zotron-ocr, zotron-rag SKILL.md
7. **更新 researcher agent** — 集成新工具

## 11. 不做

- Zotero 设置面板（配置走 env / config file）
- 引用图谱（保留 Phase 3）
- CNKI 抓取逻辑（在 cnki-plugin 体系中）
- 全库索引（按集合工作，几十篇规模）
- LanceDB / 复杂向量数据库（JSON + numpy 足够）
- GraphRAG / 知识图谱（工作集规模不需要）
- PDF 转图片步骤（云端 OCR API 直接接收 PDF）
