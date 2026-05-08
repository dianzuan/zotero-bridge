# Provider API Notes

Date: 2026-05-08

These notes capture provider-specific API shapes collected while preparing live
OCR and embedding integration. They are reference material for the Rust
migration; do not put credentials in this file.

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
OpenAI/VLM-style message payload for `paddleocr-vl`; it must be changed before
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
- HTTP transport must support both JSON and multipart/form-data.
- Async providers need a job poller abstraction and result downloader.
- The provider response parser must understand Paddle's
  `result.layoutParsingResults` and JSONL async result shape, not just the
  current `{pages:[{blocks:[...]}]}` scaffold.
- Keep provider raw output as `zotron-ocr.raw.zip` or equivalent audit artifact
  before normalization.
