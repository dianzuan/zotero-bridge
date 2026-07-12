# Zotron CLI 命令参考

## 顶层命令

| 命令 | 说明 |
|------|------|
| `zotron ping` | 检查 Zotero 是否运行且 XPI 已加载 |
| `zotron rpc <方法> [JSON参数]` | 通用 RPC 逃生舱，可调用任意 XPI 方法 |
| `zotron push <JSON文件>` | 推送准备好的 JSON 数据到 Zotero |
| `zotron web` | 从公开学术 API 搜索与抓取论文 |

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
| `items fulltext <key>` | `--ocr` | 获取条目 PDF 附件的全文。优先返回干净的 OCR sidecar 文本，无 OCR 时回退 Zotero 内置抽取。`--ocr` 强制只用 OCR（无则报错） |
| `items related <key>` | — | 列出相关条目 |
| `items citation-key <key>` | — | 获取引用 key |
| `items path <key>` | — | 获取条目 PDF 附件的本地文件路径 |
| `items attachments <key>` | `--limit` `--offset` | 列出条目的附件 |
| `items find-pdfs --collection <名称>` | `--limit` | 批量为集合内缺失 PDF 的条目查找 PDF |
| `items find-duplicates` | — | 查找重复条目 |

### 添加

| 命令 | 参数 | 说明 |
|------|------|------|
| `items add --doi <DOI>` | `--collection` `--dry-run` | 通过 DOI 添加论文 |
| `items add --isbn <ISBN>` | `--collection` `--dry-run` | 通过 ISBN 添加书籍 |
| `items add --from-url <URL>` | `--collection` `--dry-run` | 通过网页 URL 添加 |
| `items add --file <路径>` | `--collection` `--dry-run` | 从本地文件添加 |
| `items add --type <类型> --field "key=value"` | `--collection` `--dry-run` | 手动创建条目 |

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

## web — 学术搜索

| 命令 | 说明 |
|------|------|
| `web search <关键词>` | 搜索学术论文（`-s openalex|crossref|s2|arxiv`，默认 openalex） |
| `web fetch --doi <DOI>` | 按 DOI 抓取元数据 + 开放获取 PDF |
| `web fetch --arxiv <ID>` | 按 arXiv ID 抓取 |

`web fetch` 输出 Zotero JSON（含 `_pdf` 路径），管道给 `zotron push` 即可入库。凭据（联系邮箱、CORE key）读取自 Zotero 设置；Zotero 未运行时无 key 照常可用。

> 附件操作是 `items` 的子命令：`items attachments <key>`、`items path <key>`、`items fulltext <key>`、`items find-pdfs`。

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
| `annotations list <item/attachment key>` | `--context` | 列出 PDF 上的批注（自动解析 item key）。`--context N` 附带每条批注前后 N 字符的上下文 |
| `annotations create <item/attachment key> --quote "文字"` | `--page` `--comment` `--color` | 按引用文字自动定位并高亮，无需打开 PDF（默认 type=highlight） |
| `annotations create <item/attachment key> --type <类型> --position <JSON>` | `--sort-index` `--text` `--comment` `--color` `--dry-run` | 手动指定位置创建批注 |
| `annotations create-batch <item/attachment key>` | `--file` | 从 stdin 或 `--file` 的 JSON 数组批量创建批注。每条：`{"quote","color","comment","type"}` |
| `annotations locate <item/attachment key> --quote "文字"` | — | 在 PDF 中定位引用文字但不创建批注，返回页码和矩形框 |
| `annotations delete <annotation key>` | `--dry-run` | 删除批注 |

---

## export — 导出

| 命令 | 参数 | 说明 |
|------|------|------|
| `export <key1> <key2> ... --format bibtex` | — | 导出 BibTeX（默认格式） |
| `export <key1> <key2> ... --format ris` | — | 导出 RIS |
| `export <key1> <key2> ... --format csl-json` | — | 导出 CSL-JSON |
| `export <key1> <key2> ... --format bibliography` | `--style` `--html` | 导出格式化参考文献。`--style` 默认 APA（`http://www.zotero.org/styles/apa`），可换 GB/T 7714 等样式 URL |
| `export --collection <名称> --format bibtex` | — | 导出整个集合 |

---

## ocr — OCR/文档解析

| 命令 | 参数 | 说明 |
|------|------|------|
| `ocr providers` | — | 列出支持的 OCR 提供商 |
| `ocr run --provider <名称>` | `--input` `--file` `--item-key` `--attachment-key` `--mime-type` `--endpoint` `--api-key-env` | 执行 OCR 请求并输出标准化 blocks |
| `ocr status --collection <名称>` | — | 查看集合的 OCR 状态 |
| `ocr process --parent <item key>` | `--attachment` `--provider` `--source-url` `--result-dir` `--result-zip` `--provider-endpoint` `--api-key-env` `--poll-interval-seconds` `--timeout-seconds` `--chunk-chars` | 解析单篇 PDF 并写入隐藏 sidecar。`--attachment` 可选，自动从 parent 查找 |
| `ocr process --collection <名称>` | `--provider` `--source-url` `--provider-endpoint` `--api-key-env` `--poll-interval-seconds` `--timeout-seconds` `--chunk-chars` | 批量 OCR 集合内每个条目，自动逐条解析其 PDF 附件；无 PDF 的条目跳过（不报错），输出含 `processed` / `skipped` / `failed` 计数。不能与 `--result-dir` / `--result-zip` 同用 |
| `ocr reindex` | `--collection` `--key` `--stale-only` `--chunk-chars` `--reparse` | 在不重新 OCR 的前提下，从已抽取的 blocks 重新切块并重新生成向量（免费）。`--reparse` 更进一步：从保存的原始返回 `latest.raw.json` 重新解析 blocks，回灌 block 级解析改进（如 GLM 标题恢复），同样不调 OCR API |

chunk sidecar 带有 `schema_version` 头行。`--stale-only` 读取该头行并跳过已是当前 schema 的 sidecar，因此只重建过期的部分。**升级后请运行一次 `zotron ocr reindex --stale-only`**，把升级前未带版本头的旧 v1 sidecar 重建到当前 schema（否则陈旧 chunk 会混入检索）。reindex 还会重新生成向量，使此前只切块未生成向量的文档恢复语义检索能力。

---

## rag — 检索增强

默认使用混合检索（BM25 + 向量 + RRF 融合），无需额外标志。当不存在向量索引（sidecar 文件）时自动回退到关键词匹配。Embedding 提供商在 Zotero → 设置 → Zotron 面板中配置（默认 Ollama nomic-embed-text，本地免费）。支持 10 种 embedding 提供商：Ollama、OpenAI、火山引擎、DashScope、智谱、Jina、SiliconFlow、Voyage、Cohere、自定义。

| 命令 | 参数 | 说明 |
|------|------|------|
| `rag providers` | — | 列出支持的 embedding 提供商 |
| `rag embed --provider <名称> --input <文件>` | `--endpoint` `--model` `--input-type` `--api-key-env` | 执行 embedding 请求 |
| `rag status --collection <名称>` | — | 查看集合的 RAG 索引状态。输出含 `embeddingsAvailable` / `totalVectors`，可在检索前判断是否能进行语义（向量）检索 |
| `rag search <查询词>` | `--collection` `--key` `--top-spans-per-item` `--include-fulltext-spans` `--limit` `--output (json/jsonl)` | 混合检索（BM25 + 向量 + RRF），返回相关段落 |

融合之后，结果会经过一条质量流水线：可选的 cross-encoder **重排（rerank）**；**动态截断**（分数下限 + 最大间隙裁剪，仅在配置了重排器时生效），只返回真正相关的命中而非固定数量；**MMR 多样性**去除近重复段落（先把相关性分数 min-max 归一化到 0..1，使多样性在所有模式下都生效）；以及由 min/max K 约束的 **token 预算**。

输出字段：
- `mode`（顶层）——实际使用的检索路径：`hybrid`、`dense` 或 `lexical`。当向量或查询 embedding 不可用时，检索会回退到 `lexical`（BM25）并在此如实标注，而不是静默返回空结果。
- `scoreKind`（每条命中）——该命中 `score` 的来源/量纲：`rerank`（0..1 重排分）、`rrf`（融合排名分）、`cosine`（向量相似度）或 `bm25`（关键词分）。

检索流水线设置（Zotero → 设置 → Zotron 面板）：
- `rag.retrievalMode`——`hybrid`（默认）| `dense` | `lexical`
- `rag.minK`（默认 3）/ `rag.maxK`（默认 20）——结果数量上下界
- `rag.tokenBudget`（默认 6000）——返回段落的 token 总上限
- `rag.mmrLambda`（默认 0.7）——多样性权衡（越高越偏相关性）
- `rerank.provider` / `rerank.apiKey` / `rerank.model` / `rerank.apiUrl`——重排器配置
- `rerank.candidateCount`（默认 30）——送入重排的融合候选数量
- `rerank.scoreFloor`（默认 0.1）——丢弃低于该分数的重排命中
- `rerank.gapThreshold`（默认 0.15）——在最大分数间隙处裁剪长尾

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
| `system methods [方法名]` | — | 列出所有 RPC 方法；带方法名则描述该方法 |

---

## 通用参数

所有命令都有 `--url`（默认 `http://127.0.0.1:23119/zotron/rpc`），写操作都有 `--dry-run`。
