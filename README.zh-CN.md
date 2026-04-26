<div align="center">

<img src="assets/logo.png" alt="Zotero Bridge logo" width="160" />

# Zotero Bridge

**Zotero 8 的强类型 JSON-RPC 2.0 桥**

*把 Zotero 内部 77 个 API 方法通过 HTTP 暴露给 AI 智能体、命令行和外部工具。*

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![CI](https://github.com/dianzuan/zotero-bridge/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotero-bridge/actions/workflows/ci.yml)
[![Zotero](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)
[![GitHub release](https://img.shields.io/github/v/release/dianzuan/zotero-bridge?color=brightgreen)](https://github.com/dianzuan/zotero-bridge/releases/latest)

[**English**](README.md) · [**简体中文**](README.zh-CN.md)

</div>

---

## ✨ 亮点

- **1 个 umbrella Claude Code skill** —— 5 个工作流（search / manage / export / OCR / RAG），渐进披露，AI 替你读写文献库
- **77 个强类型 RPC 方法**底层撑场子，9 大命名空间 —— Python / curl / MCP server 等任意客户端都能对接
- **Python CLI + SDK** —— 基于 typer，`--jq` 过滤、`--paginate` 自动翻页、`--dry-run` 预演、shell 补全
- **带溯源的 RAG** —— 每段引用都带 `zotero://` URI，一键回到 Zotero 原文核对
- **在 Zotero 8.0.4 实测** —— Zotero 7 暂未验证
- **AGPL-3.0** —— 完全开源

## 📑 目录

- [这是什么](#这是什么)
- [为啥要做](#为啥要做)
- [快速上手](#快速上手)
- [API 一览](#api-一览)
- [开发](#开发)
- [带引用的 RAG](#带引用的-ragai-像人一样读-pdf接口)
- [API 稳定性](#api-稳定性)
- [Roadmap](#roadmap计划中尚未实现)
- [贡献](#贡献)
- [许可证](#许可证)

---

## 这是什么

Zotero Bridge 是一个 [bootstrap-extension](https://www.zotero.org/support/dev/zotero_7_for_developers) 插件——它把你正在跑的 Zotero 变成一个 JSON-RPC 2.0 服务器。任何外部工具——研究智能体、引文流水线、爬虫、MCP 服务器、自定义 CLI——都可以通过 HTTP 读写你的文献库，无需直接动 SQLite。

```
┌──────────────────────────┐         ┌─────────────────────────────┐
│  你的工具/智能体          │         │  Zotero（装了本插件）        │
│                          │         │                             │
│  curl /zotero-bridge/rpc │ ──HTTP─▶│  77 个强类型 RPC 方法        │
│  cnki-plugin 推送         │         │  • items.* (19)             │
│  研究 agent              │         │  • collections.* (12)       │
│  Better-BibTeX 消费者    │         │  • attachments.* (6)        │
│  …                       │         │  • notes.* (6)              │
│                          │         │  • search.* (8)             │
│                          │         │  • tags.* (6)               │
│                          │         │  • export.* (5)             │
│                          │         │  • settings.* (4)           │
│                          │         │  • system.* (11)            │
└──────────────────────────┘         └─────────────────────────────┘
```

## 为啥要做

Zotero 自带的 `localhost:23119` HTTP 接口是为浏览器扩展硬编码的几个端点（如 `/connector/getSelectedCollection`），不是通用 API。如果你想问"给我最近 5 篇带 X 标签的期刊文章"，你只能：

- 自己读 SQLite 文件（脆弱、随版本变、写锁问题）
- 通过 debug-server 后门 eval 任意 JS（不安全、官方不支持）
- 每个项目自己从零写 bootstrap 插件（重复造轮子，没共享规范）

Zotero Bridge 填补了这个空白：**一个稳定、强类型、统一的 API 表面**，任何工具都能对接。

## 快速上手

### 1. 装 Zotero 插件（所有人都得做这一步）

1. 从 [Releases](https://github.com/dianzuan/zotero-bridge/releases) 下载最新 `zotero-bridge.xpi`。
2. Zotero 里：**工具 → 插件 → ⚙ → 从文件安装插件…** → 选 `.xpi`。
3. 重启 Zotero。HTTP 服务跑在 `localhost:23119/zotero-bridge/rpc`。

### 2. 选一个客户端

#### A. Claude Code（推荐）

仓库自带 1 个 umbrella skill（5 个工作流，渐进披露），让 Claude 直接帮你搜库、入库、导出引用、OCR、做 RAG。

在 Claude Code 里：

```
/plugin marketplace add dianzuan/zotero-bridge
/plugin install zotero-bridge@zotero-bridge
```

然后直接对 Claude 说："找一下我库里关于注意力机制的论文" 或 "把 DOI 10.1038/nature12373 加到 ML 集合里" —— Claude 自动路由到对应工作流，工作流调 RPC。

（本地开发：clone 仓库后 `/plugin marketplace add ~/zotero-bridge`。）

#### B. Python CLI / SDK（写脚本或别的 AI agent）

```bash
uv tool install zotero-bridge        # 或：pip install zotero-bridge

zotero-bridge ping
zotero-bridge search quick "数字经济" --limit 10
zotero-bridge rpc items.get '{"id":12345}'    # escape hatch —— 覆盖全部 77 个方法
```

`rpc` 子命令是协议层 escape hatch：任何没有友好 typer 子命令的 RPC 方法都能直接调用。`--jq` 过滤输出（仿 `gh api --jq`），`--install-completion {bash|zsh|fish|powershell}` 装 shell 补全。SDK 稳定契约见 [`docs/api-stability.md`](docs/api-stability.md)。

#### C. 裸 HTTP（任何工具、任何语言）

```bash
curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":1}'
```

## API 一览

9 个命名空间共 77 个方法。完整规范见 [docs/superpowers/specs/2026-04-23-xpi-api-prd.md](docs/superpowers/specs/2026-04-23-xpi-api-prd.md)。

| 命名空间 | 方法数 | 干啥的 |
|---|---|---|
| `items.*` | 19 | 条目 CRUD、按 DOI/URL/ISBN/文件添加、最近、回收站、查重、关联 |
| `collections.*` | 12 | 集合列表、创建、改名、移动、树形、集合内条目 |
| `attachments.*` | 6 | 附件列表、获取全文（走 cache 文件）、获取路径、找 PDF |
| `notes.*` | 6 | 笔记 CRUD、注释、笔记内搜索 |
| `search.*` | 8 | 快速 / 全文 / 按标签 / 按标识符 / 高级搜索；保存的搜索 |
| `tags.*` | 6 | 标签列表、添加、移除、改名、删除（支持跨库） |
| `export.*` | 5 | BibTeX / CSL-JSON / RIS / CSV / 参考文献（CiteProc） |
| `settings.*` | 4 | 插件侧偏好（如 OCR 提供商、embedding 模型） |
| `system.*` | 11 | ping、version、libraries、switchLibrary、sync、currentCollection、**`system.reload`**（开发用自重载） |

### 设计约定

- **所有返回形状遵循 PRD §2**——条目类返回走 `serializeItem(item)`，分页用 `{items, total, offset?, limit?}` 信封，wire 上统一小写 `libraryId`
- **错误是 JSON-RPC 2.0 结构化的 `{code, message}`**——`-32602` 给调用方错误（字段缺失/错误），`-32603` 给服务端错误（Zotero 内部异常）
- **中文姓名处理**——`items.create` 自动把"欧阳修"拆成 `{lastName: "欧阳", firstName: "修"}` 进 Zotero creator 记录，覆盖 70+ 复姓

## 开发

### 前置

- Node.js 18+
- 本地装了 Zotero 8
- （可选但推荐）Windows 用户用 WSL 跑开发流程

### 构建 + 测试

```bash
npm install
npm test           # 99 个 mocha 单元测试
npm run build      # 类型检查 + 打包 + XPI 输出到 .scaffold/build/
```

### 热重载开发流程

设置 `ZOTERO_PLUGIN_ZOTERO_BIN_PATH` 指向你 Zotero 的二进制：

```bash
ZOTERO_PLUGIN_ZOTERO_BIN_PATH=/path/to/zotero npm start
```

这会用 proxy 模式启 Zotero 加载插件。改源码自动重建 + 重载。

**WSL → Windows 注意**：scaffold 自带的 RDP 重载在跨系统场景下死透（profile 路径问题）。改用插件自带的 `system.reload` RPC 旁路：

```bash
npm run build && \
  rsync -a --delete .scaffold/build/addon/ "$DEV_ADDON_DIR" && \
  curl -s -X POST http://localhost:23119/zotero-bridge/rpc \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"system.reload","id":1}'
```

这会清 Gecko 的 startup cache 并原地重载插件。

## 项目结构

```
src/
├── handlers/         # 9 个 handler 文件，每个命名空间一个
├── utils/            # 纯函数 helper：errors、guards、serialize 等
├── server.ts         # JSON-RPC 2.0 dispatcher + 端点注册
├── hooks.ts          # 启动时 setup（pref 默认值）
└── index.ts          # 插件入口
test/
├── handlers/         # 每个 handler 的测试（sinon + mock Zotero 全局）
├── utils/            # 纯 helper 单测
├── fixtures/         # Zotero mock harness（installZotero/resetZotero）
└── chinese-name.test.ts
addon/
└── manifest.json     # 插件元数据（名字、版本、目标 Zotero 版本）
```

## 状态

`v1.3.4` —— 生产可用。在 5000+ 条目 / 70+ 集合的真实库上验证过。99/99 mocha 测试通过。

## 带引用的 RAG（"让 AI 像人一样读 PDF"接口）

Zotero Bridge 的 RAG 层（`python/zotero_bridge/rag/`）让 AI 从用户的
Zotero 库里取文本时**带上结构化出处**。每个片段都封装成 `Citation`
对象，带 Zotero item key、附件 ID、章节、chunk 索引、相似度分数、
原文，以及一个 `zotero://` URI 用于一键回到 Zotero 验证。

### Python API

```python
from zotero_bridge import retrieve_with_citations
from zotero_bridge.rag.embedder import create_embedder
from pathlib import Path

embedder = create_embedder(
    provider="ollama", model="nomic-embed-text",
    api_url="http://localhost:11434",
)
citations = retrieve_with_citations(
    query="数字经济对就业的影响",
    store_path=Path("~/.local/share/zotero-bridge/rag/数字经济.json").expanduser(),
    embedder=embedder,
    top_k=10,
)
for c in citations:
    print(f"{c.title} [{c.zotero_uri()}] section={c.section} score={c.score:.2f}")
    print(c.text)
```

### CLI

```bash
# 1) 给一个 Zotero collection 建索引（已有命令）
zotero-rag index --collection 数字经济

# 2) 用查询取出引用
zotero-rag cite "数字经济对就业的影响" --collection 数字经济 --output markdown
zotero-rag cite "数字经济对就业的影响" --collection 数字经济 --output json --top-k 5
```

### 稳定的 JSON schema

`--output json` 输出一个对象数组，schema 稳定如下：

```json
[
  {
    "itemKey": "ABC123",
    "attachmentId": 42,
    "title": "数字经济与就业",
    "authors": "张三; 李四",
    "section": "第三章 实证分析",
    "chunkIndex": 7,
    "text": "...",
    "score": 0.87,
    "zoteroUri": "zotero://select/library/items/ABC123"
  }
]
```

这是面向 AI 的契约 —— 任何消费 zotero-bridge 引用的 agent 都可以依赖这些字段名。

## API 稳定性

SDK / CLI 消费者的稳定契约见 [docs/api-stability.md](docs/api-stability.md)。

## Roadmap（计划中尚未实现）

`SETTINGS_KEYS` 里已预留偏好键（可以 `settings.set` 写入），但消费方法还没写：

- `ocr.*` —— 给未来的 `attachments.ocr` 方法
- `embedding.*` —— 给未来的语义搜索 / 分块
- `rag.*` —— 给未来的 `search.semantic` 方法

欢迎 PR。

## 贡献

欢迎 PR。提交前跑 `npm test`；新方法至少带一个 mocha 测试，用 `test/fixtures/zotero-mock.ts`。

## 许可证

[AGPL-3.0-or-later](LICENSE)。要在闭源产品里用，开 issue 聊商业授权。

## 鸣谢

- [Zotero](https://www.zotero.org/) by Corporation for Digital Scholarship（AGPL-3.0）
- [`zotero-plugin-toolkit`](https://github.com/windingwind/zotero-plugin-toolkit) by windingwind（MIT）
- [`zotero-plugin-scaffold`](https://github.com/zotero-plugin-dev/zotero-plugin-scaffold)（AGPL-3.0）
- [`zotero-types`](https://github.com/windingwind/zotero-types)（MIT）
- 受 [`Jasminum`](https://github.com/l0o0/jasminum)（AGPL-3.0）启发——Zotero 的中文学术元数据插件
- Zotero 插件社区（Knowledge4Zotero、zotero-pdf-translate、zotero-actions-tags、zotero-style，全部 AGPL-3.0）
