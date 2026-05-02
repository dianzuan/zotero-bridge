<div align="center">

<img src="assets/logo.png" alt="Zotron logo" width="160" />

# Zotron

**Zotero 8 的强类型 JSON-RPC 2.0 桥**

*把 Zotero 内部 81 个 API 方法通过 HTTP 暴露给 AI 智能体、命令行和外部工具。*

[![License: AGPL-3.0-or-later](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![Zotero](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)
[![GitHub release](https://img.shields.io/github/v/release/dianzuan/zotron?color=brightgreen)](https://github.com/dianzuan/zotron/releases/latest)

[**English**](README.md) · [**简体中文**](README.zh-CN.md)

</div>

---

## 这是什么

Zotron 是一个 [bootstrap-extension](https://www.zotero.org/support/dev/zotero_7_for_developers) 插件——把跑着的 Zotero 变成 JSON-RPC 2.0 服务器。外部工具（研究 agent、引文流水线、爬虫、MCP server、自定义 CLI）通过 HTTP 读写文献库，不用直接动 SQLite。

```
┌──────────────────────────┐         ┌─────────────────────────────┐
│  你的工具/智能体          │         │  Zotero（装了本插件）        │
│                          │         │                             │
│  curl /zotron/rpc │ ──HTTP─▶│  81 个强类型 RPC 方法        │
│  cnki-plugin 推送         │         │  • items.* (19)             │
│  研究 agent              │         │  • collections.* (12)       │
│  Better-BibTeX 消费者    │         │  • attachments.* (8)        │
│  …                       │         │  • notes.* (5)              │
│                          │         │  • search.* (8)             │
│                          │         │  • tags.* (6)               │
│                          │         │  • export.* (5)             │
│                          │         │  • settings.* (4)           │
│                          │         │  • system.* (13)            │
└──────────────────────────┘         └─────────────────────────────┘
```

在 Zotero 8.0.4 上、5000+ 条目 / 70+ 集合的真实库验证过。Zotero 7 暂未验证。

## 为什么不用 Zotero 官方 Local API？

Zotero 7 上线了官方 [Local API](https://www.zotero.org/support/dev/web_api/v3/start)，把云端 Web API 端口到 `localhost:23119/api/`。如果你的客户端本来就是冲着 `api.zotero.org` 写的（`pyzotero`、Web-API 兼容的各种插件），改一下 base URL 就能跑——这是它的最佳场景，Zotron 不抢这块。

但 Local API **以读为主**，schema 锁死在 `api.zotero.org` 公开的那套结构上。一旦你做 agent / 工具链，能力差距立刻显现：

| | Zotero Local API (`/api/`) | Zotron (`/zotron/rpc`) |
|---|---|---|
| 读条目、集合、标签、注释 | ✅ | ✅ |
| **按 DOI / URL / ISBN / 本地文件添加（走 translator）** | ❌ | ✅ |
| **去重、集合层级操作、批量改标签** | 部分 | ✅ |
| **全文缓存（`getCachedFile`）、内嵌关联** | ❌ | ✅ |
| **当前选中条目、切换文献库、触发同步、热重载插件** | ❌ | ✅ |
| **任意 CSL 样式的参考文献导出（完整 CiteProc）** | 部分 | ✅ |
| 跟 `pyzotero` / Web-API 客户端开箱即用 | ✅ | ❌（自定义 RPC） |
| 需要勾选"允许其他应用通讯" | 是 | **否**（plugin 端点绕过这个 gate） |

Zotron 是 Zotero **内部 JS API** 的强类型 JSON-RPC 桥——插件自己用的那套接口，你和数据之间没有 Web-API schema 翻译层。10 个命名空间共 81 个方法，覆盖 CRUD + 搜索 + 导出 + 标签 + 同步 + RAG + system。

Zotero 7 之前的几条绕路——直接读 SQLite（脆弱、跟版本走、写锁）、debug-server 后门 eval JS（不安全、不支持）、每个项目自己写一次性 bootstrap 插件（重复造轮子）——全是坏路径。Zotron 用一套稳定 typed surface 把它们替掉。

## 快速上手

### 路径 A —— Claude Code（推荐）

**前置：** [Claude Code](https://docs.claude.com/en/docs/claude-code/)、[`uv`](https://docs.astral.sh/uv/getting-started/installation/)、Zotero 8 桌面版。

```
/plugin marketplace add dianzuan/zotron
/plugin install zotron@zotron
/zotron:setup
```

`/zotron:setup` 会先 ping 一下 bridge；如果 XPI 没装，就把 release 里的 `zotron.xpi` 下载到你的真实 Downloads 文件夹（自动识别 `E:\Downloads` 之类盘符重定向、OneDrive 转移和 POSIX 默认路径），默认先试 GitHub，再试配置的镜像 URL。如果 XPI 已安装但版本旧，它不会重新下载覆盖，而是提示你在 Zotero 插件管理里走内置更新。装完直接对 Claude 说人话——*"找一下我库里关于注意力机制的论文"*、*"把 DOI 10.1038/nature12373 加到 ML 集合里"*、*"把 10、13、16 号条目按 GB/T 7714 格式导出"*。Claude 自动路由到对应子工作流（search / manage / export / OCR / RAG），子工作流调 RPC。

### 路径 B —— OpenAI Codex CLI / code-cli

如果你在 Codex 里工作而不是用 Claude Code，走这条路径。同一个 `claude-plugin/` 包现在也带原生 Codex plugin manifest，所以 Codex 和 Claude Code 复用同一套 bridge、Python CLI、XPI 和 skills。

**前置：** OpenAI Codex CLI（`codex`；有些环境会称为 `code-cli`）、[`uv`](https://docs.astral.sh/uv/getting-started/installation/)、Zotero 8 桌面版。

```bash
# 1) 如果当前环境还没有 Codex CLI，先安装。
npm install -g @openai/codex

# 2) 添加 Zotron 插件 marketplace。
codex plugin marketplace add dianzuan/zotron

# 本地 checkout 可用：
# codex plugin marketplace add .
```

然后在 Codex 的插件界面安装 **Zotron**，并调用 setup skill：

```text
$zotron-setup
```

setup skill 会暴露插件自带的 `zotron`、`zotron-rag`、`zotron-ocr` shims；需要时把 release 里的 `zotron.xpi` 下载到 Downloads，并带你走 Zotero 原生的 **工具 → 插件 → ⚙ → 从文件安装附加组件 → 重启**。仓库不再跟踪生成的 XPI；安装来源是 release。GitHub 访问不通时，可以用 `ZOTRON_XPI_URLS` 配置空格/逗号/分号分隔的镜像 URL 列表。

Zotero 重启后：

```bash
zotron ping
zotron search quick "数字经济" --limit 10
```

`zotron ping` 成功后，Codex 可以通过已安装的 plugin skill 直接调用 `zotron`、`zotron-rag`、`zotron-ocr` 或裸 HTTP。

### 路径 C —— Python CLI / SDK

```bash
# 1) 手动装 XPI：https://github.com/dianzuan/zotron/releases/latest
# 2) 从 git 装 CLI（暂未发布到 PyPI）：
uv tool install "git+https://github.com/dianzuan/zotron.git#subdirectory=claude-plugin/python"

zotron ping
zotron search quick "数字经济" --limit 10
zotron rpc items.get '{"id":"YR5BUGHG"}'  # escape hatch —— 覆盖全部 81 个方法
```

`--jq` 过滤输出（仿 `gh api --jq`）；`--install-completion {bash|zsh|fish|powershell}` 装 shell 补全。SDK 稳定契约见 [`docs/api-stability.md`](docs/api-stability.md)。

### 路径 D —— 裸 HTTP

```bash
curl -s -X POST http://localhost:23119/zotron/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"system.ping","id":1}'
```

### 故障排查

| 现象 | 原因 | 修法 |
|---|---|---|
| `/zotron:setup` 输出 `MISSING_UV` | 没装 `uv` | `curl -LsSf https://astral.sh/uv/install.sh \| sh` |
| 启动横幅提示 *"Zotron not detected"* | Zotero 没开，或 XPI 没装 | 启动 Zotero，再跑 `/zotron:setup` |
| 23119 端口 `connection refused` | Zotero 自带 HTTP 服务被关 | 编辑 → 设置 → 高级 → 配置编辑器 → `extensions.zotero.httpServer.enabled` 设为 `true` |
| 装完 skill 不自动触发 | 插件没在当前会话加载 | `/reload-plugins`，或重启 Claude Code |
| Bash 里 `zotron: command not found` | 插件的 `bin/` 没进 PATH | 插件必须 enabled —— 在 `/plugin` 的 **Installed** 标签确认 |

## API 一览

10 个命名空间共 81 个方法。完整规范见 [docs/superpowers/specs/2026-04-23-xpi-api-prd.md](docs/superpowers/specs/2026-04-23-xpi-api-prd.md)。

| 命名空间 | 方法数 | 干啥的 |
|---|---|---|
| `items.*` | 19 | 条目 CRUD、按 DOI/URL/ISBN/文件添加、最近、回收站、查重、关联 |
| `collections.*` | 12 | 集合列表、创建、改名、移动、树形、集合内条目 |
| `attachments.*` | 8 | 列表、单条获取、全文（走 cache 文件）、添加、URL 添加、路径、删除、找 PDF |
| `notes.*` | 5 | 按父条目列表、单条获取、创建、更新、搜索 |
| `search.*` | 8 | 快速 / 全文 / 按标签 / 按标识符 / 高级搜索；保存的搜索 |
| `tags.*` | 6 | 标签列表、添加、移除、改名、删除（支持跨库） |
| `export.*` | 5 | BibTeX / CSL-JSON / RIS / CSV / 参考文献（CiteProc） |
| `settings.*` | 4 | 插件侧偏好（如 OCR 提供商、embedding 模型） |
| `system.*` | 13 | ping、version、libraries、switchLibrary、sync、currentCollection、listMethods、describe、**`system.reload`** |

**约定：** 响应遵循 **key-first** 原则——条目和集合对象以 `key`（8 位字母数字，对齐 Zotero Web API v3）为主标识符，不暴露数字 `id`。条目带 `version` 字段供同步用。变更类返回统一为 `{ok: true, key}`。分页用 `{items, total, offset?, limit?}` 信封。wire 上统一小写 `libraryId`。所有接受标识符的参数同时支持数字 ID 和 key 字符串。调用不存在的方法会得到模糊匹配建议（"Did you mean?"）。错误是 JSON-RPC 2.0 结构化的 `{code, message}`（`-32602` 调用方错误，`-32603` 服务端错误）。`items.create` 自动拆中文姓名——`欧阳修` → `{lastName: "欧阳", firstName: "修"}`——覆盖 70+ 复姓。

## 带引用的 RAG

RAG 层（`claude-plugin/python/zotron/rag/`）把每个检索到的片段封装成 `Citation`，带 Zotero item key、附件 ID、章节、chunk 索引、相似度分数、原文，以及一个 `zotero://` URI 用于一键回 Zotero 核对。

```bash
zotron-rag index --collection 数字经济
zotron-rag cite "数字经济对就业的影响" --collection 数字经济 --output json
```

`--output json` 是面向 AI 的稳定契约：

```json
{ "itemKey": "ABC123", "attachmentKey": "ATT42XY", "title": "...", "authors": "...",
  "section": "第三章 实证分析", "chunkIndex": 7, "text": "...",
  "score": 0.87, "zoteroUri": "zotero://select/library/items/ABC123" }
```

2026 RAG/OCR roadmap 会把这一层扩展成 Zotero-native artifacts 和适合 academic-zh 消费的 JSONL hit stream。稳定目标是：

- provider raw evidence 存在 `<item-key>.zotron-ocr.raw.zip`；
- 统一后的 OCR/parser blocks 存在 `<item-key>.zotron-blocks.jsonl`；
- retrieval chunks 存在 `<item-key>.zotron-chunks.jsonl`；
- vectors 和索引元数据存在 `<item-key>.zotron-embed.npz`；
- retrieval hits 一行一个 JSON 对象，必须包含 `item_key`、`title`、`text`，并建议带上 `zotero_uri`、`chunk_id`、`block_ids`、`section_heading`、`query`、`score` 等 provenance 字段。XPI 已暴露 `rag.searchHits` / `rag.searchCards` 做 Zotero-native chunk artifact 检索；`zotron-rag hits --zotero` 会调用这个 JSON-RPC backend。

Markdown 可以作为派生的便利输出，但不能作为 OCR/RAG 的唯一 truth，因为它会丢 page、bbox、table、figure、provider 和 reading-order provenance。

## 开发

Node 18+，本地装了 Zotero 8。（Windows 用户推荐 WSL。）

```bash
npm install
npm test           # 127 个 mocha 单元测试
npm run build      # 类型检查 + 打包 + XPI 输出到 .scaffold/build/
```

热重载：`ZOTERO_PLUGIN_ZOTERO_BIN_PATH=/path/to/zotero npm start`。WSL 跨系统场景下 scaffold 自带的 RDP 重载死透（profile 路径问题）—— `rsync` 把 build 输出推到 dev profile 后，用插件自带的 `system.reload` RPC 旁路：

```bash
npm run build && \
  rsync -a --delete .scaffold/build/addon/ "$DEV_ADDON_DIR" && \
  curl -s -X POST http://localhost:23119/zotron/rpc \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"system.reload","id":1}'
```

## Roadmap

`SETTINGS_KEYS` 里已预留偏好键（可以 `settings.set` 写入），消费方法尚未实现：

- `ocr.*` —— 给未来的 `attachments.ocr` 方法
- `embedding.*` —— 给未来的语义搜索 / 分块
- `rag.searchHits` / `rag.searchCards` —— 基于 Zotero 附件 chunk artifacts 的 retrieval hits

当前 storage 和 retrieval contract 见 [`docs/2026-04-27-rag-ocr-roadmap.md`](docs/2026-04-27-rag-ocr-roadmap.md)。后续一等 RAG/OCR 实现应保留 provider raw 输出，normalize 成 blocks/chunks，并暴露 academic-zh 兼容的 retrieval hits，不能把 markdown 当作唯一 truth。

欢迎 PR。新 RPC 方法需要带一个 mocha 测试，用 `test/fixtures/zotero-mock.ts`。

## 许可证

[AGPL-3.0-or-later](LICENSE)。要在闭源产品里用，开 issue 聊商业授权。

## 鸣谢

- [Zotero](https://www.zotero.org/) by Corporation for Digital Scholarship（AGPL-3.0）
- [`zotero-plugin-toolkit`](https://github.com/windingwind/zotero-plugin-toolkit) by windingwind（MIT）
- [`zotero-plugin-scaffold`](https://github.com/zotero-plugin-dev/zotero-plugin-scaffold)（AGPL-3.0）
- [`zotero-types`](https://github.com/windingwind/zotero-types)（MIT）
- 受 [`Jasminum`](https://github.com/l0o0/jasminum)（AGPL-3.0）启发——Zotero 的中文学术元数据插件
- Zotero 插件社区（Knowledge4Zotero、zotero-pdf-translate、zotero-actions-tags、zotero-style，全部 AGPL-3.0）
