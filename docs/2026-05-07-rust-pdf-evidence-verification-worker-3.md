# Rust PDF Evidence Verification

Date: 2026-05-07
Team: `tdd-continue-rust-pdf-evidence`

## Objective coverage audited

- Artifact persistence defaults to the external Zotron artifact store, not Zotero-visible attachments.
- Zotero attachment persistence remains an explicit opt-in contract.
- `zotron-ocr status` checks external chunk artifacts before legacy Zotero attachment/note probes.
- Provider execution is locked behind injected Rust transports with no live credentials in tests.
- `zotron-rpc` includes concrete runtime adapters for provider HTTP calls and local JSON-emitting commands.
- OCR provider coverage includes GLM, PaddleOCR-VL, and MinerU.
- Embedding provider coverage includes Volcengine, Alibaba/DashScope, and custom OpenAI-compatible endpoints.
- Structure-aware chunks remain key-first and preserve block provenance.

## Final verification evidence

| Check | Result | Evidence |
| --- | --- | --- |
| Rust formatting | PASS | `cargo fmt --all -- --check` |
| Rust provider runtime adapter tests | PASS | `cargo test -p zotron-rpc --test provider_runtime_adapters` -> 5 passing |
| Rust workspace tests | PASS | `cargo test --workspace` |
| Rust lint/static analysis | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| TypeScript/plugin build | PASS | `npm run build` |
| Node tests | PASS | `npm test` -> 139 passing |
| Python compatibility tests | PASS | `cd claude-plugin/python && python -m pytest` -> 440 passed |

## Remaining gaps

- Credentialed cloud-provider smoke tests are still not executed in automated tests; provider execution is covered through mocked/injected transports and a local HTTP endpoint so real credentials are not required.
- GLM/PaddleOCR-VL/MinerU are wired at the Rust contract/executor layer, but the default CLI still exposes them as provider contracts rather than a full end-user OCR pipeline.
- Legacy Python OCR/RAG commands still exist for compatibility. The Rust contracts and CLI status path now default to external artifacts, but a later migration should retire the Python artifact-attachment path instead of expanding it.
