# Rust PDF Evidence Verification

Date: 2026-05-07
Team: `tdd-continue-rust-pdf-evidence`

> Superseded status note, 2026-05-09: this was a worker verification snapshot.
> The current Rust branch now has the end-user MinerU path
> `zotron ocr parse-pdf`, which writes hidden per-PDF sidecars and was live
> tested against Zotero item `2TGDLKDZ` / attachment `QI2YI74W` with 187 blocks
> and 43 chunks.

## Objective coverage audited

- Artifact persistence defaults to hidden per-PDF sidecars, not Zotero-visible attachments.
- Zotero attachment persistence remains an explicit opt-in contract.
- `zotron ocr status` checks hidden per-PDF sidecar chunk artifacts before legacy Zotero attachment/note probes.
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
- GLM/PaddleOCR-VL still expose provider helpers; MinerU now has the full
  `zotron ocr parse-pdf` pipeline.
- Legacy Python OCR/RAG commands still exist as reference code. New user-facing
  behavior should land in Rust+JS.
