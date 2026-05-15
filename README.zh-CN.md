<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

让 AI agent 读取、搜索、标注你的 Zotero 文献库。

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[功能](#功能) · [安装](#安装) · [Agent 集成](#agent-集成) · [CLI 参考](#cli-参考) · [开发](#开发)

</div>

## 功能

安装 Zotron 后，你的 AI agent 可以：

- **搜索**论文——按标题、作者、年份、标签、DOI 或 PDF 全文
- **阅读**论文内容、元数据和笔记
- **标注** PDF——高亮、下划线，引用文字自动定位（无需打开 PDF）
- **导出**引用——BibTeX、APA 或任何 CSL 样式
- **OCR** 扫描件，运行混合语义检索（BM25 + 向量 + RRF）
- **管理**集合、标签和附件

你用自然语言跟 agent 说话，agent 在后台调用 Zotron。

## 安装

```bash
cargo install zotron
```

然后在 Zotero 里安装 [XPI 插件](https://github.com/dianzuan/zotron/releases/latest)（工具 → 插件 → 从文件安装附加组件），重启 Zotero。

```bash
zotron ping   # 应返回 {"status": "ok", ...}
```

## Agent 集成

Zotron 可以作为 [Claude Code](https://docs.claude.com/en/docs/claude-code/) 和 [Codex](https://github.com/openai/codex) 的插件使用。安装一次，然后直接跟 agent 说话：

```bash
# Claude Code
/plugin marketplace add dianzuan/zotron && /zotron:setup

# Codex
codex plugin marketplace add dianzuan/zotron && $zotron-setup
```

装完直接说：

> "搜一下我库里关于注意力机制的论文"
>
> "读一下这篇论文，把关键发现用蓝色高亮"
>
> "把 ML 集合导出成 BibTeX"
>
> "我哪篇论文讨论了回归不连续？"

Agent 内部调用 `zotron` CLI 命令——不走 MCP，没有 tool schema 开销。

### 数据源插件

Zotron 支持通过数据源插件扩展——`PATH` 上以 `zotron-*` 命名的独立二进制：

- **[zotron-scholar](https://github.com/dianzuan/zotron-scholar)** — OpenAlex、CrossRef、Semantic Scholar、Unpaywall、arXiv

插件输出 JSON 到 stdout，通过管道传给 `zotron push` 写入 Zotero。

## 工作原理

三层架构：

1. **XPI 插件**（TypeScript）——运行在 Zotero 8 内，通过 `localhost:23119/zotron/rpc` 暴露 86 个 JSON-RPC 2.0 方法，涵盖 11 个命名空间
2. **Rust CLI**——名词-动词子命令（`zotron items get`、`zotron search "query"`），发布在 [crates.io](https://crates.io/crates/zotron)
3. **Agent 插件**——Claude Code 和 Codex 的 skills，让 AI agent 通过 CLI 驱动 Zotero

CLI 对接的是 Zotero 的内部 JS API——插件自己用的那套。覆盖了官方 [Local API](https://www.zotero.org/support/dev/web_api/v3/start) 做不到的事：按 DOI/URL/ISBN 添加、全文缓存、CiteProc 参考文献、重复合并、批量操作。

## CLI 参考

输出是 JSON，用 `jq` 过滤：

```bash
zotron search "数字经济" --author "张三" --after 2020
zotron search "回归不连续" --fulltext --collection "宏观因子"
zotron items fulltext YR5BUGHG
zotron annotations create ITEM_KEY --quote "重要发现" --color "#2ea8e5"
zotron export --collection "宏观因子"
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "宏观因子" "就业弹性"
```

`zotron --help` 看完整命令列表，`zotron <命令> --help` 看参数。详见：[CLI 参考 (中文)](docs/cli-reference-zh.md) · [CLI reference (en)](docs/cli-reference.md)

## 开发

```bash
npm install && npm test     # XPI 单元测试
npm run build               # → .scaffold/build/zotron.xpi
cargo test                  # CLI + types 测试
```

## 发布

推送 `v*` tag 会自动触发 [release workflow](.github/workflows/release.yml)：构建 XPI → 创建 GitHub Release → 发布到 crates.io。

查看[最新版本](https://github.com/dianzuan/zotron/releases/latest)。

## 许可证

[AGPL-3.0-or-later](LICENSE)
