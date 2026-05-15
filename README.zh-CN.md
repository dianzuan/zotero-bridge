<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

让 AI agent 读取、搜索、标注你的 Zotero 文献库。

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![CI](https://github.com/dianzuan/zotron/actions/workflows/ci.yml/badge.svg)](https://github.com/dianzuan/zotron/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[为什么选 Zotron？](#为什么选-zotron) · [功能](#功能) · [安装](#安装) · [Agent 集成](#agent-集成) · [CLI 参考](#cli-参考) · [常见问题](#常见问题) · [English](README.md)

</div>

<!-- TODO: 加 demo GIF —— 录一个终端 session 展示 search → annotate → export -->

## 为什么选 Zotron？

Zotron 是一个本地 Rust CLI，直接对接 Zotero 内部 JS API——完整读写权限，结构化 JSON 输出，管道给 `jq` 即可。Zotero 官方 API 是只读的 HTTP 接口；MCP server 每次调用都有延迟和 token 开销。

## 功能

安装后，你的 agent 可以：

- **搜索**论文——按标题、作者、年份、标签、DOI 或 PDF 全文。多个条件可以组合：`--author "李" --after 2020 --tag "核心" --collection "宏观"`。全文搜索（`--fulltext`）查的是 PDF 内容，不只是元数据。

- **阅读**论文内容、元数据和笔记。`items fulltext` 返回 PDF 附件的缓存文本。`items get` 返回结构化元数据（标题、作者、日期、期刊、DOI、标签、集合）。`notes list` 在有 OCR 结果时包含 OCR markdown。

- **标注** PDF——引用一段文字，Zotron 自动在 PDF 中定位并创建高亮，不需要打开 PDF 阅读器。支持高亮和下划线两种类型，支持 Zotero 内置的 8 种颜色。

- **导出**引用——BibTeX、APA、Chicago 或任何 CSL 样式。可以导出单个条目、多个条目或整个集合。输出到 stdout，按需重定向到文件。

- **OCR** 扫描件——支持多种 provider（MinerU、GLM、PaddleOCR）。OCR 结果以 sidecar 文件存储在附件旁。OCR 之后，`rag search` 运行混合检索：BM25 词法匹配 + 余弦向量相似度 + RRF 融合排序，全部本地执行。

- **管理**集合、标签和附件。创建集合、在集合间移动条目、批量增删标签、通过 URL 或本地路径添加附件、查找缺失的 PDF。

## 安装

### 从 crates.io 安装（需要 Rust）

```bash
cargo install zotron
```

### 从 GitHub Releases 下载（不需要 Rust）

从[最新 release](https://github.com/dianzuan/zotron/releases/latest) 下载你平台的预编译二进制，放到 `PATH` 上即可。

### Zotero 插件

从同一个 release 页面下载 [zotron.xpi](https://github.com/dianzuan/zotron/releases/latest)。在 Zotero 里：工具 → 插件 → 从文件安装附加组件。重启 Zotero。

### 验证

```bash
zotron ping   # 应返回 {"status": "ok", ...}
```

如果 `ping` 失败，确认 Zotero 正在运行且 Zotron 插件已启用（工具 → 插件）。

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

典型的 agent 工作流：搜论文 → 读全文 → 用 `--quote` 标注关键段落 → 导出引用。每一步是一次 CLI 调用，agent 根据上一步的 JSON 输出决定下一步操作。

### 数据源插件

外部数据源通过插件接入——`PATH` 上以 `zotron-*` 命名的独立二进制：

- **[zotron-scholar](https://github.com/dianzuan/zotron-scholar)** — OpenAlex、CrossRef、Semantic Scholar、Unpaywall、arXiv

插件输出 JSON 到 stdout，通过管道传给 `zotron push` 写入 Zotero。

## CLI 参考

输出是 JSON，用 `jq` 过滤。

### 搜索

```bash
# 按标题/作者/年份快速搜索
zotron search "数字经济"

# 组合条件
zotron search "数字经济" --author "张三" --after 2020 --collection "宏观因子"

# 搜索 PDF 正文内容
zotron search "回归不连续" --fulltext

# 按 DOI 搜索
zotron search --doi 10.1038/nature12373
```

### 阅读

```bash
# 完整元数据
zotron items get YR5BUGHG

# PDF 全文（Zotero 缓存）
zotron items fulltext YR5BUGHG

# 列出附件
zotron attachments list --parent YR5BUGHG

# 集合树
zotron collections tree
```

### 标注

```bash
# 引用文字自动高亮（无需打开 PDF）
zotron annotations create YR5BUGHG --quote "重要发现" --color "#2ea8e5"

# 列出已有标注
zotron annotations list YR5BUGHG
```

### 导出

```bash
# BibTeX（默认）
zotron export --collection "宏观因子"

# APA 参考文献
zotron export --format bibliography YR5BUGHG BF4I9QX4

# 重定向到文件
zotron export --collection "宏观因子" > refs.bib
```

### OCR + RAG

```bash
# OCR 扫描件
zotron ocr process --parent YR5BUGHG --provider mineru

# 混合语义检索（BM25 + 向量 + RRF）
zotron rag search --collection "宏观因子" "就业弹性"
```

### 管道给 jq

```bash
# 提取关键字段
zotron search "就业" | jq '.items[] | {key, title, year}'

# 计数
zotron search "气候" | jq '.total'
```

`zotron --help` 看完整命令列表，`zotron <命令> --help` 看参数。

## 常见问题

**Q: Zotero 必须运行吗？**
是的。Zotron 通过 XPI 插件在 `localhost:23119` 和 Zotero 通信。用 `zotron ping` 检查连接。

**Q: 支持 Zotero 6 吗？**
不支持。Zotron 需要 Zotero 7+（已在 Zotero 8 上测试）。

**Q: 不用 Claude Code 或 Codex 也行吗？**
可以。CLI 独立运行。任何能调 shell 的 agent、脚本或人都可以直接使用。

**Q: 支持哪些平台？**
Windows、macOS、Linux。CLI 是单个 Rust 二进制。XPI 插件在 Zotero 支持的所有平台上运行。

**Q: 能用多个 Zotero 库吗？**
可以。`zotron system libraries` 列出可用库，`zotron system switchLibrary --id 2` 切换。

**Q: `zotron ping` 失败怎么办？**
1. Zotero 是否在运行？
2. Zotron 插件是否已启用？（工具 → 插件）
3. 23119 端口是否被其他程序占用？
4. Windows 上检查防火墙是否允许 localhost 连接。

**Q: `--quote` 高亮不打开 PDF 怎么做到的？**
Zotron 在后台打开一个不可见的 reader 标签页，提取逐字符位置数据，定位引用文本，创建标注，然后关闭后台标签页。

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
