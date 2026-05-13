<div align="center">

<img src="assets/logo.png" alt="Zotron" width="120" />

# Zotron

在终端里读写你的 Zotero 文献库。搜索、管理、导出、OCR、RAG，一个命令搞定。

[![crates.io](https://img.shields.io/crates/v/zotron)](https://crates.io/crates/zotron)
[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Zotero 8+](https://img.shields.io/badge/Zotero-8.0+-orange)](https://www.zotero.org/)

[安装](#安装) · [用法](#用法) · [Agent 集成](#agent-集成) · [开发](#开发)

</div>

## 安装

```bash
cargo install zotron
```

然后在 Zotero 里安装 [XPI 插件](https://github.com/dianzuan/zotron/releases/latest)（工具 → 插件 → 从文件安装附加组件），重启 Zotero。

```bash
zotron ping   # 应返回 {"status": "ok", ...}
```

## 用法

```bash
# 搜索——默认搜标题/作者/年份，--fulltext 搜 PDF 正文
zotron search "数字经济" --author "张三" --after 2020
zotron search "回归不连续" --fulltext --collection "宏观因子"

# 条目管理
zotron items add --doi 10.1038/nature12373 --collection "ML Papers"
zotron items fulltext YR5BUGHG
zotron collections tree

# 导出——默认 BibTeX
zotron export --collection "宏观因子"
zotron export --format bibliography YR5BUGHG BF4I9QX4

# OCR + 语义检索
zotron ocr process --parent YR5BUGHG --provider mineru
zotron rag search --collection "宏观因子" "就业弹性"
```

输出是 JSON，用 `jq` 过滤：

```bash
zotron search "就业" | jq '.items[] | {key, title, year}'
```

`zotron --help` 看完整命令列表，`zotron <命令> --help` 看参数。

## Agent 集成

Zotron 可以作为 [Claude Code](https://docs.claude.com/en/docs/claude-code/) 和 [Codex](https://github.com/openai/codex) 的插件使用。Agent 直接调 `zotron` 子命令——不走 MCP，没有 tool schema 开销。

```bash
# Claude Code
/plugin marketplace add dianzuan/zotron && /zotron:setup

# Codex
codex plugin marketplace add dianzuan/zotron && $zotron-setup
```

装完直接说人话："搜一下我库里关于注意力机制的论文"、"把这个集合导出成 BibTeX"、"OCR 一下 ML 文件夹里的 PDF"。

## 工作原理

Zotron 分两部分：

1. **XPI 插件**——跑在 Zotero 里，通过 `localhost:23119` 暴露 86 个 JSON-RPC 方法
2. **Rust CLI**——强类型子命令，调用这些方法，为 shell 管道和 agent 设计

CLI 对接的是 Zotero 的内部 JS API——插件自己用的那套。覆盖了官方 [Local API](https://www.zotero.org/support/dev/web_api/v3/start) 做不到的事：按 DOI/URL/ISBN 添加、全文缓存、CiteProc 参考文献、重复合并、批量操作。

## 开发

```bash
npm install && npm test     # 127 个 XPI 单元测试
npm run build               # → .scaffold/build/zotron.xpi
cargo test                  # 44 个 CLI 契约测试
```

## 许可证

[AGPL-3.0-or-later](LICENSE)
