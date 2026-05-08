# Provider API Notes

Date: 2026-05-08

These notes capture provider-specific API shapes collected while preparing live
OCR and embedding integration. They are reference material for the Rust
migration; do not put credentials in this file.

## Scope Decision

Zotron's OCR/parser pipeline should preserve source-document evidence rather
than ask a general vision model to rewrite page images. The mainline providers
must expose document-parser style structure such as text spans, tables,
reading order, page numbers, and bbox/layout metadata.

Out of scope for the current milestone:

- Generic image-to-text or OCR-like VLM fallback providers.
- Prompt-only conversion of PDF pages into invented Markdown/JSON.
- Treating figures as textual evidence. Figure/image blocks should keep
  references, captions, bbox, and raw provider metadata; they should not be
  summarized into synthetic text by default.

Tables are in scope when a document parser returns structured table content or
faithful Markdown. Provider raw output remains the audit source, and Markdown is
only a convenience projection.

Tables should be normalized into table blocks and table chunks for retrieval.
The embedding text for a table should be built from title/caption, headers,
row labels, units, and cell content, while preserving the raw table HTML/JSON or
Markdown as the audit source. Figures and charts keep captions, bbox, page, and
asset references by default; pixels are read by a VLM only when a query requires
visual interpretation.

## GLM-OCR

Source: https://docs.bigmodel.cn/api-reference/%E6%A8%A1%E5%9E%8B-api/%E6%96%87%E6%A1%A3%E8%A7%A3%E6%9E%90

Purpose: synchronous document layout parsing with Markdown, layout details, and
optional visualization output. The provider supports images, but Zotron's core
path is PDF/document parsing rather than generic image-to-text.

Environment:

```bash
ZOTRON_GLM_API_KEY=
ZOTRON_GLM_OCR_ENDPOINT=https://open.bigmodel.cn/api/paas/v4/layout_parsing
ZOTRON_GLM_OCR_MODEL=glm-ocr
```

HTTP contract:

```text
POST {ZOTRON_GLM_OCR_ENDPOINT}
Authorization: Bearer <token>
Content-Type: application/json
```

Request body:

```json
{
  "model": "glm-ocr",
  "file": "<file URL or base64 document>",
  "return_crop_images": false,
  "need_layout_visualization": false,
  "start_page_id": 1,
  "end_page_id": 1
}
```

Required fields:

- `model`: must be `glm-ocr`.
- `file`: PDF document URL/base64 for the Zotron mainline path. The upstream
  API also accepts images, but generic image-to-text is not a Zotron milestone.

Documented limits:

- Supports PDF, JPG, and PNG.
- Single image up to 10 MB.
- PDF up to 50 MB.
- PDF up to 100 pages.

Response shape:

```json
{
  "id": "task_123456789",
  "created": 1727156815,
  "model": "GLM-OCR",
  "md_results": "# 文档标题\n这是文档内容...",
  "layout_details": [[
    {
      "index": 1,
      "label": "text",
      "bbox_2d": [0.1, 0.1, 0.5, 0.3],
      "content": "这是文本内容",
      "height": 800,
      "width": 600
    }
  ]],
  "layout_visualization": ["<url or string>"],
  "data_info": {
    "num_pages": 5,
    "pages": [{"width": 600, "height": 800}]
  },
  "usage": {"total_tokens": 123},
  "request_id": "req_123456789"
}
```

Normalization notes:

- `layout_details` is an array per page; each entry should become a Zotron
  block.
- `label` maps to block `type`.
- `content` maps to block `text`.
- `bbox_2d` is normalized `[x1, y1, x2, y2]`; use `width` / `height` or
  `data_info.pages` to convert to page-space coordinates only when needed.
- `md_results` is a convenience artifact and must not replace structured
  blocks.

Current Rust migration note: the live adapter now uses `/layout_parsing`,
`glm-ocr`, and data-url encoding for local PDFs. Raw provider output and
downloaded image/layout assets still need to be persisted as first-class
artifacts.

## PaddleOCR-VL

### Sync layout parsing endpoint

Purpose: quick layout parsing for small PDFs or smoke tests.

Environment:

```bash
ZOTRON_PADDLEOCR_VL_API_KEY=
ZOTRON_PADDLEOCR_VL_SYNC_ENDPOINT=<your-aistudio-layout-parsing-endpoint>
```

HTTP contract:

```text
POST {ZOTRON_PADDLEOCR_VL_SYNC_ENDPOINT}
Authorization: token <access token>
Content-Type: application/json
```

Request body:

```json
{
  "file": "<base64 file bytes>",
  "fileType": 0,
  "useDocOrientationClassify": false,
  "useDocUnwarping": false,
  "useChartRecognition": false
}
```

Notes:

- `fileType = 0` for PDF documents.
- `fileType = 1` for images.
- Auth scheme is literal `token`, not `Bearer`.
- Response content is under `result.layoutParsingResults`.
- Each layout parsing result can include `markdown.text`,
  `markdown.images`, and `outputImages`.

Current Rust migration note: the sync adapter now uses this `file` / `fileType`
contract and `Authorization: token ...`. Raw `markdown.images`, `outputImages`,
and provider-native Markdown still need to be persisted as first-class artifacts.

### Async OCR jobs endpoint

Purpose: production path for larger PDFs, progress reporting, and timeout
resilience.

Environment:

```bash
ZOTRON_PADDLEOCR_VL_API_KEY=
ZOTRON_PADDLEOCR_VL_JOBS_ENDPOINT=https://paddleocr.aistudio-app.com/api/v2/ocr/jobs
ZOTRON_PADDLEOCR_VL_MODEL=PaddleOCR-VL-1.5
```

Submit local file:

```text
POST {ZOTRON_PADDLEOCR_VL_JOBS_ENDPOINT}
Authorization: bearer <access token>
Content-Type: multipart/form-data
```

Multipart fields:

```text
file=<PDF file>
model=PaddleOCR-VL-1.5
optionalPayload={"useDocOrientationClassify":false,"useDocUnwarping":false,"useChartRecognition":false}
```

Submit remote file URL:

```text
POST {ZOTRON_PADDLEOCR_VL_JOBS_ENDPOINT}
Authorization: bearer <access token>
Content-Type: application/json
```

JSON body:

```json
{
  "fileUrl": "<file URL>",
  "model": "PaddleOCR-VL-1.5",
  "optionalPayload": {
    "useDocOrientationClassify": false,
    "useDocUnwarping": false,
    "useChartRecognition": false
  }
}
```

Polling:

```text
GET {ZOTRON_PADDLEOCR_VL_JOBS_ENDPOINT}/{jobId}
Authorization: bearer <access token>
```

States:

- `pending`
- `running`
- `done`
- `failed`

When state is `done`, download `data.resultUrl.jsonUrl`. The result is JSONL:
each line contains a `result.layoutParsingResults` payload with the same
markdown/images style as the sync endpoint.

### Normalization target

Both sync and async Paddle outputs should normalize into Zotron blocks:

```json
{
  "block_key": "ATTACHKEY:p16:b004",
  "item_key": "ITEMKEY",
  "attachment_key": "ATTACHKEY",
  "page_idx": 15,
  "type": "text",
  "bbox": [120, 300, 860, 420],
  "section_path": ["..."],
  "text": "..."
}
```

Open question for implementation: Paddle's markdown output is immediately
available, but bbox/layout details may live in provider-specific fields inside
the full result. The live adapter should preserve raw Paddle output first, then
normalize conservatively rather than inventing bbox.

## MinerU Cloud

Source: https://mineru.net/apiManage/docs

Decision: Zotron should integrate MinerU Cloud's precise parsing API, not the
lightweight Agent API, for the main evidence pipeline.

Rationale:

- The precise API supports PDF/image/Office inputs up to 200 MB and 200 pages.
- It supports `pipeline`, `vlm`, and `MinerU-HTML` model versions.
- It returns a result zip URL containing Markdown and JSON artifacts; docs map
  `full.md` to the Markdown result, `*_content_list.json` to content list JSON,
  `*_model.json` to model inference output, and `layout.json` to middle JSON.
- The Agent lightweight API is useful for temporary AI-agent workflows, but it
  only returns a Markdown CDN URL and has tighter file/page limits; it is not
  enough for Zotron's auditable blocks/chunks/artifact pipeline.

Environment:

```bash
ZOTRON_MINERU_API_KEY=
ZOTRON_MINERU_ENDPOINT=https://mineru.net/api/v4
ZOTRON_MINERU_MODEL_VERSION=vlm
```

Remote file flow:

```text
POST {ZOTRON_MINERU_ENDPOINT}/extract/task
Authorization: Bearer <token>
Content-Type: application/json
```

Request body:

```json
{
  "url": "<public or uploaded file URL>",
  "model_version": "vlm",
  "is_ocr": false,
  "enable_formula": true,
  "enable_table": true,
  "language": "ch",
  "data_id": "<attachment_key>",
  "page_ranges": "1-200"
}
```

Polling:

```text
GET {ZOTRON_MINERU_ENDPOINT}/extract/task/{task_id}
Authorization: Bearer <token>
```

When `data.state` is `done`, download `data.full_zip_url` and preserve the zip
as raw artifact before normalization.

Local file flow:

- Use the signed upload API when parsing a local Zotero attachment that is not
  already reachable by URL.
- Request upload URL from `/api/v4/file-urls/batch`, PUT the PDF bytes to the
  returned URL, then poll the generated task/batch result.

Normalization target:

- `full.md`: provider-native preview artifact, not truth.
- `*_content_list.json` / `content_list_v2.json`: preferred source for Zotron
  blocks and table chunks.
- Images and layout/span PDFs: preserved as raw/debug assets and referenced by
  block metadata when available.
- Tables: normalize into independent table blocks/chunks and include them in
  embedding.
- Figures/charts/images: keep caption/title/page/bbox/asset_ref; run VLM only
  on demand.

## Implementation Implications

- Provider adapters need per-provider auth schemes; one generic Bearer wrapper
  is not enough because Paddle sync uses `Authorization: token ...`.
- GLM-OCR is synchronous JSON with `Authorization: Bearer ...`, but it is not a
  chat-completions request.
- HTTP transport must support both JSON and multipart/form-data.
- Async providers need a job poller abstraction and result downloader. MinerU
  precise parsing and Paddle async both require this path.
- The provider response parser must understand GLM's `layout_details`,
  Paddle's `result.layoutParsingResults`, Paddle JSONL async result shape, and
  MinerU content-list JSON, not just the current `{pages:[{blocks:[...]}]}`
  scaffold.
- Keep provider raw output as `.zotron/ocr/latest.raw.json` or equivalent audit artifact
  before normalization.
