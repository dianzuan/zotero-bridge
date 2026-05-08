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

The Rust scaffold currently does not match this contract. It still points GLM
OCR at the chat-completions endpoint with model `glm-4.5v`; live GLM-OCR support
must switch to `/layout_parsing`, `glm-ocr`, and this response shape.

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

The Rust scaffold currently does not match this contract. It still builds an
chat/message-style payload for `paddleocr-vl`; it must be changed before
live Paddle sync calls can work.

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

## Implementation Implications

- Provider adapters need per-provider auth schemes; one generic Bearer wrapper
  is not enough because Paddle sync uses `Authorization: token ...`.
- GLM-OCR is synchronous JSON with `Authorization: Bearer ...`, but it is not a
  chat-completions request.
- HTTP transport must support both JSON and multipart/form-data.
- Async providers need a job poller abstraction and result downloader.
- The provider response parser must understand GLM's `layout_details`,
  Paddle's `result.layoutParsingResults`, and Paddle JSONL async result shape,
  not just the current `{pages:[{blocks:[...]}]}` scaffold.
- Keep provider raw output as `zotron-ocr.raw.zip` or equivalent audit artifact
  before normalization.
