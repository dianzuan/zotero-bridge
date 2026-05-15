<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

让 AI agent 读取、搜索、标注你的 Zotero 文献库。

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[功能](#功能) · [安装](#安装) · [Agent 集成](#agent-集成) · [CLI 参考](#cli-参考) · [English](README.md)

</div>

<!-- TODO: 加 demo GIF —— 录一个终端 session 展示 search → annotate → export -->

## 为什么选 Zotron？

Zotron 是一个本地 Rust CLI，直接对接 Zotero 内部 JS API——完整读写权限，结构化 JSON 输出，管道给 `jq` 即可。Zotero 官方 API 是只读的 HTTP 接口；MCP server 每次调用都有延迟和 token 开销。

## 功能

安装后，你的 agent 可以：

- **搜索**论文——按标题、作者、年份、标签、DOI 或 PDF 全文
- **阅读**论文内容、元数据和笔记
- **标注** PDF——高亮、下划线，引用文字自动定位（无需打开 PDF）
- **导出**引用——BibTeX、APA 或任何 CSL 样式
- **OCR** 扫描件，运行混合语义检索（BM25 + 向量 + RRF）
- **管理**集合、标签和附件

## 安装

### 1. Rust CLI

```bash
cargo install zotron
```

### 2. Zotero 插件

下载最新的 [zotron.xpi](https://github.com/dianzuan/zotron/releases/latest)，在 Zotero 里：工具 → 插件 → 从文件安装附加组件。重启 Zotero。

### 3. 验证

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

Agent 直接调用 `zotron` CLI 命令。

### 数据源插件

外部数据源通过插件接入——`PATH` 上以 `zotron-*` 命名的独立二进制：

- **[zotron-scholar](https://github.com/dianzuan/zotron-scholar)** — OpenAlex、CrossRef、Semantic Scholar、Unpaywall、arXiv

插件输出 JSON 到 stdout，通过管道传给 `zotron push` 写入 Zotero。

## CLI 参考

输出是 JSON，用 `jq` 过滤：

```bash
# 搜索
zotron search "数字经济" --author "张三" --after 2020
zotron search "回归不连续" --fulltext --collection "宏观因子"

# 阅读
zotron items get YR5BUGHG
zotron items fulltext YR5BUGHG
zotron collections tree

# 标注
zotron annotations create YR5BUGHG --quote "重要发现" --color "#2ea8e5"
zotron annotations list YR5BUGHG

# 导出
zotron export --collection "宏观因子"
zotron export --format bibliography YR5BUGHG BF4I9QX4

# OCR + RAG
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "宏观因子" "就业弹性"

# 管道给 jq
zotron search "就业" | jq '.items[] | {key, title, year}'
```

`zotron --help` 看完整命令列表，`zotron <命令> --help` 看参数。

## 常见问题

**Q: Zotero 必须运行吗？**
是的。Zotron 通过 XPI 插件与运行中的 Zotero 通信。用 `zotron ping` 检查连接。

**Q: 支持 Zotero 6 吗？**
不支持。Zotron 需要 Zotero 7+（已在 Zotero 8 上测试）。

**Q: 不用 Claude Code 或 Codex 也行吗？**
可以。CLI 独立运行——任何能调 shell 的 agent 或脚本都可以直接使用 `zotron` 命令。

## 贡献

欢迎 PR。Fork、建分支、提 pull request——CI 通过后即可合并。

## Star History

<a href="https://star-history.com/#dianzuan/zotron&Date">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date&theme=dark" />
    <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date" />
    <img alt="Star History" src="https://api.star-history.com/svg?repos=dianzuan/zotron&type=Date" width="500" />
  </picture>
</a>

## 许可证

[AGPL-3.0-or-later](LICENSE)
