<div align="center">

<img src="assets/logo.png" alt="Zotron logo" width="160" />

# Zotron

**Zotero 8 的强类型 JSON-RPC 2.0 桥**

*把 Zotero 内部 77 个 API 方法通过 HTTP 暴露给 AI 智能体、命令行和外部工具。*

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
│  curl /zotron/rpc │ ──HTTP─▶│  77 个强类型 RPC 方法        │
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

在 Zotero 8.0.4 上、5000+ 条目 / 70+ 集合的真实库验证过。Zotero 7 暂未验证。

## 为啥要做

Zotero 自带的 `localhost:23119` HTTP 接口是为浏览器扩展硬编码的几个端点（如 `/connector/getSelectedCollection`），不是通用 API。绕过去的几条路——直接读 SQLite（脆弱、跟版本走、写锁问题）、debug-server 后门 eval JS（不安全、不支持）、每个项目自己写 bootstrap 插件（重复造轮子）——全是坏路径。Zotron 就是来填这个空白：一个稳定、强类型、统一的 API 表面，任何工具都能对接。

## 快速上手

### 路径 A —— Claude Code（推荐）

**前置：** [Claude Code](https://docs.claude.com/en/docs/claude-code/)、[`uv`](https://docs.astral.sh/uv/getting-started/installation/)、Zotero 8 桌面版。

```
/plugin marketplace add dianzuan/zotron
/plugin install zotron@zotron
/zotron:setup
```

`/zotron:setup` 会先 ping 一下 bridge；连不上就把插件自带的 `claude-plugin/xpi/zotron.xpi` 直接喂给 Zotero（不用下载，XPI 跟插件一起发），Zotero 弹原生安装对话框。点一次"安装"，重启 Zotero，搞定。Linux / macOS / Windows / WSL 四种环境都自动适配。装完直接对 Claude 说人话——*"找一下我库里关于注意力机制的论文"*、*"把 DOI 10.1038/nature12373 加到 ML 集合里"*、*"把 10、13、16 号条目按 GB/T 7714 格式导出"*。Claude 自动路由到对应子工作流（search / manage / export / OCR / RAG），子工作流调 RPC。

### 路径 B —— Python CLI / SDK

```bash
# 1) 手动装 XPI：https://github.com/dianzuan/zotron/releases/latest
# 2) 从 git 装 CLI（暂未发布到 PyPI）：
uv tool install "git+https://github.com/dianzuan/zotron.git#subdirectory=claude-plugin/python"

zotron ping
zotron search quick "数字经济" --limit 10
zotron rpc items.get '{"id":12345}'    # escape hatch —— 覆盖全部 77 个方法
```

`--jq` 过滤输出（仿 `gh api --jq`）；`--install-completion {bash|zsh|fish|powershell}` 装 shell 补全。SDK 稳定契约见 [`docs/api-stability.md`](docs/api-stability.md)。

### 路径 C —— 裸 HTTP

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

**约定：** 返回形状遵循 PRD §2——条目类返回走 `serializeItem(item)`、分页用 `{items, total, offset?, limit?}` 信封、wire 上统一小写 `libraryId`。错误是 JSON-RPC 2.0 结构化的 `{code, message}`（`-32602` 调用方错误，`-32603` 服务端错误）。`items.create` 自动拆中文姓名——`欧阳修` → `{lastName: "欧阳", firstName: "修"}`——覆盖 70+ 复姓。

## 带引用的 RAG

RAG 层（`claude-plugin/python/zotron/rag/`）把每个检索到的片段封装成 `Citation`，带 Zotero item key、附件 ID、章节、chunk 索引、相似度分数、原文，以及一个 `zotero://` URI 用于一键回 Zotero 核对。

```bash
zotron-rag index --collection 数字经济
zotron-rag cite "数字经济对就业的影响" --collection 数字经济 --output json
```

`--output json` 是面向 AI 的稳定契约：

```json
{ "itemKey": "ABC123", "attachmentId": 42, "title": "...", "authors": "...",
  "section": "第三章 实证分析", "chunkIndex": 7, "text": "...",
  "score": 0.87, "zoteroUri": "zotero://select/library/items/ABC123" }
```

## 开发

Node 18+，本地装了 Zotero 8。（Windows 用户推荐 WSL。）

```bash
npm install
npm test           # 99 个 mocha 单元测试
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
- `rag.*` —— 给未来的 `search.semantic` 方法

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
