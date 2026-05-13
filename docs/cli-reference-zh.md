# Zotron CLI 命令参考

## 顶层命令

| 命令 | 说明 |
|------|------|
| `zotron ping` | 检查 Zotero 是否运行且 XPI 已加载 |
| `zotron rpc <方法> [JSON参数]` | 通用 RPC 逃生舱，可调用任意 XPI 方法 |
| `zotron push <JSON文件>` | 推送准备好的 JSON 数据到 Zotero |
| `zotron find-pdfs --collection <名称>` | 批量为集合内缺失 PDF 的论文查找并下载 PDF |

---

## search — 搜索

| 用法 | 说明 |
|------|------|
| `search <关键词>` | 快速搜索（标题、作者、年份、标签） |
| `search <关键词> --fulltext` | 全文 PDF 内容搜索 |
| `search --author <名字>` | 按作者搜索（模糊匹配） |
| `search --tag <标签>` | 按标签搜索（精确匹配） |
| `search --after 2020 --before 2025` | 按日期范围搜索 |
| `search --journal <刊名>` | 按期刊搜索（模糊匹配） |
| `search --doi <DOI>` | 按 DOI 精确查找 |
| `search --isbn <ISBN>` | 按 ISBN 精确查找 |
| `search saved-searches` | 列出所有保存的搜索 |
| `search create-saved <名称> --condition "字段 操作符 值"` | 创建保存的搜索 |
| `search delete-saved <key>` | 删除保存的搜索 |

**通用参数：** `--collection`（限定集合范围）、`--limit`（默认50）、`--offset`

**组合示例：**
```bash
zotron search "就业" --collection "数字经济" --author "张三" --after 2020 --tag "核心期刊"
```

---

## items — 条目

### 查询

| 命令 | 参数 | 说明 |
|------|------|------|
| `items get <key>` | — | 获取单个条目的完整元数据 |
| `items list` | `--limit` `--offset` `--sort` `--direction` `--trash` | 列出库中所有条目。`--trash` 列出回收站条目 |
| `items recent` | `--limit` `--offset` `--type (added/modified)` | 最近添加/修改的条目 |
| `items fulltext <key>` | — | 获取条目 PDF 附件的全文（自动查找附件） |
| `items related <key>` | — | 列出相关条目 |
| `items citation-key <key>` | — | 获取引用 key |
| `items find-duplicates` | — | 查找重复条目 |

### 添加

| 命令 | 参数 | 说明 |
|------|------|------|
| `items add --doi <DOI>` | `--collection` `--dry-run` | 通过 DOI 添加论文 |
| `items add --isbn <ISBN>` | `--collection` `--dry-run` | 通过 ISBN 添加书籍 |
| `items add --from-url <URL>` | `--collection` `--dry-run` | 通过网页 URL 添加 |
| `items add --file <路径>` | `--collection` `--dry-run` | 从本地文件添加 |
| `items create` | `--type` `--field "key=value"` `--dry-run` | 手动创建条目 |

### 修改

| 命令 | 参数 | 说明 |
|------|------|------|
| `items update <key>` | `--field "key=value"` `--dry-run` | 更新条目字段 |
| `items delete <key>` | `--dry-run` | 永久删除 |
| `items trash <key1> [key2] ...` | `--dry-run` | 移入回收站（支持多个 key） |
| `items restore <key>` | `--dry-run` | 从回收站恢复 |
| `items merge-duplicates <key1> <key2> ...` | `--dry-run` | 合并重复条目 |
| `items add-related <key> --target <key>` | `--dry-run` | 添加关联关系 |
| `items remove-related <key> --target <key>` | `--dry-run` | 移除关联关系 |

---

## collections — 集合

| 命令 | 参数 | 说明 |
|------|------|------|
| `collections list` | — | 列出所有集合（平铺） |
| `collections tree` | — | 树形显示集合层级 |
| `collections get <名称/key>` | — | 获取单个集合的元数据 |
| `collections get-items <名称/key>` | `--limit` `--offset` | 列出集合内的条目。别名：`collections items` |
| `collections stats <名称/key>` | — | 集合统计（条目/附件/笔记/子集合数量） |
| `collections create <名称>` | `--parent` `--dry-run` | 创建集合 |
| `collections rename <旧名> <新名>` | `--dry-run` | 重命名集合 |
| `collections delete <名称/key>` | `--dry-run` | 删除集合 |
| `collections add-items <集合> <key1> <key2> ...` | `--dry-run` | 添加条目到集合 |
| `collections remove-items <集合> <key1> <key2> ...` | `--dry-run` | 从集合移除条目 |

---

## notes — 笔记

| 命令 | 参数 | 说明 |
|------|------|------|
| `notes list --parent <item key>` | `--limit` `--offset` | 列出条目的笔记 |
| `notes get <note key>` | — | 获取单个笔记 |
| `notes create --parent <key> --content <HTML>` | `--tag` `--dry-run` | 创建笔记 |
| `notes update <note key> --content <HTML>` | `--dry-run` | 更新笔记内容 |
| `notes delete <note key>` | `--dry-run` | 删除笔记 |
| `notes search <关键词>` | `--limit` | 搜索笔记内容 |

---

## attachments — 附件

| 命令 | 参数 | 说明 |
|------|------|------|
| `attachments list --parent <item key>` | `--limit` `--offset` | 列出条目的附件 |
| `attachments get <attachment key>` | — | 获取附件元数据 |
| `attachments fulltext <attachment key>` | — | 获取附件全文 |
| `attachments path <attachment key>` | — | 获取附件本地文件路径 |
| `attachments add --parent <key> --path <文件>` | `--title` `--dry-run` | 附加本地文件 |
| `attachments add --parent <key> --from-url <URL>` | `--title` `--dry-run` | 附加远程文件 |
| `attachments delete <key>` | `--dry-run` | 删除附件 |
| `attachments find-pdf --parent <key>` | — | 触发 Zotero 查找可用 PDF |

---

## tags — 标签

| 命令 | 参数 | 说明 |
|------|------|------|
| `tags list` | `--limit` | 列出所有标签 |
| `tags add <key1> [key2] ... --tag <标签>` | `--dry-run` | 给条目加标签（支持多个 key） |
| `tags remove <key1> [key2] ... --tag <标签>` | `--dry-run` | 移除条目标签（支持多个 key） |
| `tags rename <旧名> <新名>` | `--dry-run` | 全局重命名标签 |
| `tags delete <标签>` | `--dry-run` | 全局删除标签 |

---

## annotations — 批注

| 命令 | 参数 | 说明 |
|------|------|------|
| `annotations list --parent <item/attachment key>` | — | 列出 PDF 上的批注（自动解析 item key） |
| `annotations create --parent <item/attachment key> --type <类型>` | `--position` `--sort-index` `--text` `--comment` `--color` `--dry-run` | 创建批注。类型：highlight/note/underline/image/ink |
| `annotations delete <annotation key>` | `--dry-run` | 删除批注 |

---

## export — 导出

| 命令 | 参数 | 说明 |
|------|------|------|
| `export <key1> <key2> ... --format bibtex` | — | 导出 BibTeX（默认格式） |
| `export <key1> <key2> ... --format ris` | — | 导出 RIS |
| `export <key1> <key2> ... --format csl-json` | — | 导出 CSL-JSON |
| `export <key1> <key2> ... --format bibliography` | `--style` `--html` | 导出格式化参考文献。默认 GB/T 7714 |
| `export --collection <名称> --format bibtex` | — | 导出整个集合 |

---

## ocr — OCR/文档解析

| 命令 | 参数 | 说明 |
|------|------|------|
| `ocr providers` | — | 列出支持的 OCR 提供商 |
| `ocr run --provider <名称>` | `--input` `--file` `--item-key` `--attachment-key` `--mime-type` `--endpoint` `--api-key-env` | 执行 OCR 请求并输出标准化 blocks |
| `ocr status --collection <名称>` | — | 查看集合的 OCR 状态 |
| `ocr process --parent <item key>` | `--attachment` `--provider` `--source-url` `--result-dir` `--result-zip` `--provider-endpoint` `--api-key-env` `--poll-interval-seconds` `--timeout-seconds` `--chunk-chars` | 解析 PDF 并写入隐藏 sidecar。`--attachment` 可选，自动从 parent 查找 |

---

## rag — 检索增强

默认使用混合检索（BM25 + 向量 + RRF 融合），无需额外标志。当不存在向量索引（sidecar 文件）时自动回退到关键词匹配。Embedding 提供商在 Zotero → 设置 → Zotron 面板中配置（默认 Ollama nomic-embed-text，本地免费）。支持 10 种 embedding 提供商：Ollama、OpenAI、火山引擎、DashScope、智谱、Jina、SiliconFlow、Voyage、Cohere、自定义。

| 命令 | 参数 | 说明 |
|------|------|------|
| `rag providers` | — | 列出支持的 embedding 提供商 |
| `rag embed --provider <名称> --input <文件>` | `--endpoint` `--model` `--input-type` `--api-key-env` | 执行 embedding 请求 |
| `rag status --collection <名称>` | — | 查看集合的 RAG 索引状态 |
| `rag search <查询词>` | `--collection` `--key` `--top-spans-per-item` `--include-fulltext-spans` `--limit` `--output (json/jsonl)` | 混合检索（BM25 + 向量 + RRF），返回相关段落 |

---

## settings — Zotero 设置

| 命令 | 参数 | 说明 |
|------|------|------|
| `settings list` | — | 列出所有设置 |
| `settings get <key>` | — | 获取单个设置值 |
| `settings set <key> <value> [key value ...]` | `--dry-run` | 设置一个或多个值 |
| `settings set --file <JSON文件>` | `--dry-run` | 从 JSON 文件批量设置 |

---

## system — 系统

| 命令 | 参数 | 说明 |
|------|------|------|
| `system version` | — | XPI 版本和方法数 |
| `system libraries` | — | 列出所有库 |
| `system library-stats` | `--library` | 库统计 |
| `system schema` | — | 列出所有条目类型 |
| `system schema --type <类型>` | — | 列出某类型的字段和创建者类型 |
| `system current-collection` | — | 获取当前选中的集合 |
| `system list-methods` | — | 列出所有 RPC 方法 |
| `system describe [方法名]` | — | 描述某个/所有 RPC 方法 |

---

## 通用参数

所有命令都有 `--url`（默认 `http://127.0.0.1:23119/zotron/rpc`），写操作都有 `--dry-run`。
