# Zotron XPI 插件设计文档

> Historical design note. Current `rust-migration` status: Zotero-side XPI is
> still the JS JSON-RPC bridge, but the user-facing CLI is the Rust `zotron`
> binary. OCR/RAG are `zotron ocr ...` and `zotron rag ...`; legacy
> Python/Bash-only names are reference material, not the target product surface.

> Phase 1：基础能力，追平 cli-anything-zotero (~55 命令级别)
> Phase 2（后续）：OCR 集成、外部数据源对接、语义搜索、引用图谱

## 目标

自建 Zotero 7 xpi 插件 `zotron`，通过 HTTP JSON-RPC 暴露 Zotero 内部 JS API，配合 CLI wrapper 供 Claude Code 调用。

**定位**：不是 Zotero 的遥控器，是中文学术研究的智能管家。Phase 1 先打好基础盘。

## 架构

```
Zotero 7 进程内
┌──────────────────────────────────┐
│  zotron.xpi (TypeScript)  │
│  ├── bootstrap.js 生命周期       │
│  ├── JSON-RPC Router             │
│  │   └── Zotero.Server.Endpoints │
│  │       /zotron/rpc      │
│  ├── handlers/                   │
│  │   ├── items.ts    (条目 CRUD) │
│  │   ├── search.ts   (搜索)     │
│  │   ├── attachments.ts (附件)  │
│  │   ├── notes.ts    (笔记标注) │
│  │   ├── collections.ts (集合)  │
│  │   ├── tags.ts     (标签)     │
│  │   ├── export.ts   (引用导出) │
│  │   └── system.ts   (库/状态)  │
│  └── utils/                      │
│      └── chinese-name.ts (姓名)  │
└──────────────────────────────────┘
          ↑ HTTP (localhost:23119)
          │
Claude Code 侧
┌──────────────────────────────────┐
│  .claude-plugin (zotero-plugin)  │
│  ├── bin/zotero-cli   (Shell)    │
│  ├── skills/          (SKILL.md) │
│  └── agents/          (Agent)    │
└──────────────────────────────────┘
```

## 技术选型

| 组件 | 选择 | 理由 |
|------|------|------|
| xpi 脚手架 | windingwind/zotero-plugin-template | 事实标准，TS+esbuild，hot reload |
| 类型定义 | zotero-types | 官方 TS 类型 |
| UI 工具 | zotero-plugin-toolkit | 菜单/preference/快捷键封装 |
| HTTP 协议 | JSON-RPC 2.0 over `Zotero.Server.Endpoints` | BBT 验证过的模式，单端点 |
| 端口 | 23119（Zotero 内置，共用） | 不额外占端口 |
| 路径 | `/zotron/rpc` | 避免和其他插件冲突 |
| CLI | Rust `zotron` binary | JSON-first typed CLI for agents; Bash wrapper only locates/execs it |
| 构建 | esbuild + npm scripts | 模板自带 |

## JSON-RPC 协议

单端点 `POST http://localhost:23119/zotron/rpc`

请求：
```json
{
  "jsonrpc": "2.0",
  "method": "items.search",
  "params": {"query": "数字经济", "limit": 20},
  "id": 1
}
```

响应：
```json
{
  "jsonrpc": "2.0",
  "result": {"items": [...], "total": 156},
  "id": 1
}
```

错误：
```json
{
  "jsonrpc": "2.0",
  "error": {"code": -32602, "message": "Item not found"},
  "id": 1
}
```

## Phase 1 API 列表（60 methods）

### 条目管理 (items.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `items.get` | `{id}` | 获取条目完整元数据 |
| `items.create` | `{itemType, fields, creators, tags, collections}` | 手动创建条目 |
| `items.update` | `{id, fields}` | 修改元数据 |
| `items.delete` | `{id}` | 永久删除 |
| `items.trash` | `{id}` | 移至回收站 |
| `items.restore` | `{id}` | 从回收站恢复 |
| `items.getTrash` | `{limit, offset}` | 查看回收站 |
| `items.batchTrash` | `{ids}` | 批量回收 |
| `items.getRecent` | `{limit, type: "added"|"modified"}` | 最近条目 |
| `items.addByDOI` | `{doi}` | 按 DOI 添加（调 translator） |
| `items.addByURL` | `{url}` | 按 URL 添加（调 translator） |
| `items.addByISBN` | `{isbn}` | 按 ISBN 添加 |
| `items.addFromFile` | `{path, collection?}` | 从文件导入 |
| `items.findDuplicates` | `{}` | 查找重复 |
| `items.mergeDuplicates` | `{ids}` | 合并重复 |
| `items.getRelated` | `{id}` | 获取关联条目 |
| `items.addRelated` | `{id, relatedId}` | 添加关联 |
| `items.removeRelated` | `{id, relatedId}` | 移除关联 |

### 搜索 (search.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `search.quick` | `{query, limit}` | 关键词快速搜索 |
| `search.advanced` | `{conditions: [{field, op, value}]}` | 高级多条件 |
| `search.fulltext` | `{query, limit}` | 全文 PDF 内容搜索 |
| `search.byTag` | `{tag}` | 按标签 |
| `search.byIdentifier` | `{doi?, isbn?, pmid?}` | 按标识符 |
| `search.savedSearches` | `{}` | 列出保存的搜索 |
| `search.createSavedSearch` | `{name, conditions}` | 创建保存的搜索 |
| `search.deleteSavedSearch` | `{id}` | 删除保存的搜索 |

### 附件 (attachments.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `attachments.list` | `{parentId}` | 子条目/附件列表 |
| `attachments.getFulltext` | `{id}` | 提取 PDF 文本 |
| `attachments.getPDFOutline` | `{id}` | PDF 大纲/目录 |
| `attachments.add` | `{parentId, path, title?}` | 附加本地文件 |
| `attachments.addByURL` | `{parentId, url, title?}` | 从 URL 下载附件 |
| `attachments.getPath` | `{id}` | 获取附件文件路径 |
| `attachments.findPDF` | `{parentId}` | 自动查找 PDF |

### 笔记与标注 (notes.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `notes.get` | `{parentId}` | 获取笔记 |
| `notes.create` | `{parentId, content, tags?}` | 创建笔记（HTML） |
| `notes.update` | `{id, content}` | 更新笔记 |
| `notes.search` | `{query}` | 搜索笔记内容 |
| `notes.getAnnotations` | `{parentId}` | 获取 PDF 标注 |
| `notes.createAnnotation` | `{parentId, type, text, position}` | 创建标注 |

### 集合 (collections.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `collections.list` | `{}` | 所有集合 |
| `collections.get` | `{id}` | 集合详情 |
| `collections.getItems` | `{id, limit, offset}` | 集合中的条目 |
| `collections.getSubcollections` | `{id}` | 子集合 |
| `collections.tree` | `{}` | 完整层级树 |
| `collections.create` | `{name, parentId?}` | 创建 |
| `collections.rename` | `{id, name}` | 重命名 |
| `collections.delete` | `{id}` | 删除 |
| `collections.move` | `{id, newParentId}` | 移动 |
| `collections.addItems` | `{id, itemIds}` | 添加条目 |
| `collections.removeItems` | `{id, itemIds}` | 移除条目 |
| `collections.stats` | `{id}` | 统计 |

### 标签 (tags.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `tags.list` | `{limit?}` | 所有标签 |
| `tags.add` | `{itemId, tags}` | 添加标签 |
| `tags.remove` | `{itemId, tags}` | 移除标签 |
| `tags.rename` | `{oldName, newName}` | 重命名 |
| `tags.delete` | `{tag}` | 全库删除 |
| `tags.batchUpdate` | `{operations: [{itemId, add?, remove?}]}` | 批量更新 |

### 引用导出 (export.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `export.bibtex` | `{ids}` | BibTeX |
| `export.cslJson` | `{ids}` | CSL-JSON |
| `export.ris` | `{ids}` | RIS |
| `export.csv` | `{ids, fields?}` | CSV |
| `export.bibliography` | `{ids, style?}` | 格式化引用（含 GB/T 7714） |
| `export.citationKey` | `{id}` | Citation Key |

### 系统 (system.*)

| Method | 参数 | 说明 |
|--------|------|------|
| `system.ping` | `{}` | 健康检查 |
| `system.version` | `{}` | Zotero + 插件版本 |
| `system.libraries` | `{}` | 列出所有库 |
| `system.switchLibrary` | `{id}` | 切换当前库 |
| `system.libraryStats` | `{id?}` | 库统计 |
| `system.itemTypes` | `{}` | 条目类型列表 |
| `system.itemFields` | `{itemType}` | 字段定义 |
| `system.creatorTypes` | `{itemType}` | 创建者类型 |
| `system.sync` | `{}` | 触发同步 |

## CLI 设计

`bin/zotero-cli` — Bash 脚本，自动加入 PATH：

```bash
#!/usr/bin/env bash
# Usage: zotero-cli <namespace.method> [json_params]
# Examples:
#   zotero-cli search.quick '{"query":"数字经济"}'
#   zotero-cli items.get '{"id":12345}'
#   zotero-cli system.ping
set -euo pipefail
METHOD="${1:?Usage: zotero-cli <method> [params]}"
PARAMS="${2:-{}}"
curl -s "http://localhost:23119/zotron/rpc" \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"$METHOD\",\"params\":$PARAMS,\"id\":1}" \
  | python3 -c "
import sys,json
r=json.load(sys.stdin)
if 'error' in r: print(f'Error: {r[\"error\"][\"message\"]}',file=sys.stderr); sys.exit(1)
print(json.dumps(r.get('result'),ensure_ascii=False,indent=2))
"
```

便捷别名（可选，在 skill 里用）：
```bash
zotero-cli search.quick '{"query":"数字经济","limit":10}'
zotero-cli items.addByDOI '{"doi":"10.1016/j.jfineco.2024.01.001"}'
zotero-cli export.bibliography '{"ids":[12345],"style":"gb-t-7714-2015"}'
```

## xpi 项目结构

```
zotron/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts              # bootstrap 入口
│   ├── server.ts             # JSON-RPC router + Zotero.Server.Endpoints 注册
│   ├── handlers/
│   │   ├── items.ts          # items.* methods
│   │   ├── search.ts         # search.* methods
│   │   ├── attachments.ts    # attachments.* methods
│   │   ├── notes.ts          # notes.* methods
│   │   ├── collections.ts    # collections.* methods
│   │   ├── tags.ts           # tags.* methods
│   │   ├── export.ts         # export.* methods
│   │   └── system.ts         # system.* methods
│   └── utils/
│       ├── chinese-name.ts   # 中文姓名拆分（移植茉莉花复姓表）
│       └── response.ts       # JSON-RPC 响应构建
├── addon/
│   ├── manifest.json
│   ├── content/
│   └── locale/
│       ├── en-US/
│       └── zh-CN/
└── scripts/
    └── build.mjs
```

## 中文姓名处理

移植茉莉花的逻辑：
- 160+ 复姓表（欧阳、司马、上官...）
- 少数民族名按 `·` 分割
- 默认首字为姓
- `fieldMode: 1` 时 `lastName` 存全名
- 输入 `"张三"` → `{firstName: "三", lastName: "张"}`
- 输入 `"欧阳修"` → `{firstName: "修", lastName: "欧阳"}`

## Claude Code Plugin 结构

```
.claude/skills/zotero-plugin/
├── .claude-plugin/
│   └── plugin.json
├── bin/
│   └── zotero-cli
├── settings.json              # 环境变量
├── hooks/
│   └── hooks.json             # SessionStart: 检查 Zotero 是否运行
├── skills/
│   ├── zotero-search/SKILL.md
│   ├── zotero-add/SKILL.md
│   ├── zotero-export/SKILL.md
│   ├── zotero-manage/SKILL.md
│   └── zotero/ocr.md          # current Rust CLI skill doc
└── agents/
    └── zotero-researcher.md
```

## Phase 1 范围

- 60 个 JSON-RPC methods（上述全部）
- CLI wrapper
- 3-4 个核心 SKILL.md
- 中文姓名拆分
- 安装即用（拖 .xpi + 配 Claude Code plugin）

## Phase 2 范围（后续）

- OCR 云端 API → Zotero Note
- 外部学术数据源对接（source plugin 体系）
- 语义搜索（SPECTRE2 学术嵌入）
- RAG 段落级检索
- 引用关系图谱
- PDF 书签生成
- 海外知网支持

## 分发

- **xpi**：GitHub Releases 下载 .xpi，拖入 Zotero 安装
- **Claude Code plugin**：git clone 到 `.claude/skills/` 或通过 plugin registry
- **依赖**：Zotero 7+，无 Python/Node 额外依赖
