use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::sync::{mpsc, Mutex};
use std::thread;

use serde_json::{json, Value};
use zotron_cli::{run_ocr_with_client, run_rag_with_client, run_with_client, RpcCaller};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[derive(Default)]
struct FakeClient {
    calls: Vec<(String, Option<Value>)>,
    responses: VecDeque<Value>,
}

impl FakeClient {
    fn with_response(response: Value) -> Self {
        Self {
            calls: Vec::new(),
            responses: VecDeque::from([response]),
        }
    }

    fn with_responses(responses: Vec<Value>) -> Self {
        Self {
            calls: Vec::new(),
            responses: VecDeque::from(responses),
        }
    }
}

impl RpcCaller for FakeClient {
    fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, String> {
        self.calls.push((method.to_string(), params));
        self.responses
            .pop_front()
            .ok_or_else(|| format!("no fake response queued for {method}"))
    }
}

#[test]
fn ping_calls_system_ping_and_prints_python_compact_json() {
    let mut client = FakeClient::with_response(json!({
        "status": "ok",
        "timestamp": "2026-04-22T12:00:00Z",
    }));
    let out = run_with_client(["zotron", "ping"], &mut client).expect("ping succeeds");

    assert_eq!(client.calls, vec![("system.ping".to_string(), None)]);
    assert_eq!(
        out,
        "{\"status\": \"ok\", \"timestamp\": \"2026-04-22T12:00:00Z\"}\n"
    );
}

#[test]
fn rpc_command_forwards_method_and_params_json() {
    let mut client = FakeClient::with_response(json!({"key":"YR5BUGHG"}));

    run_with_client(
        ["zotron", "rpc", "items.get", "{\"key\":\"YR5BUGHG\"}"],
        &mut client,
    )
    .expect("rpc succeeds");

    assert_eq!(
        client.calls,
        vec![("items.get".to_string(), Some(json!({"key":"YR5BUGHG"})))]
    );
}

#[test]
fn attachments_add_by_url_accepts_normalized_source_url() {
    let mut client = FakeClient::with_response(json!({
        "key": "ATT4",
        "title": "Remote PDF",
        "parentKey": "ITEM1",
    }));

    run_with_client(
        [
            "zotron",
            "attachments",
            "add-by-url",
            "--parent",
            "ITEM1",
            "--source-url",
            "https://example.com/paper.pdf",
        ],
        &mut client,
    )
    .expect("normalized --source-url succeeds");

    assert_eq!(
        client.calls,
        vec![(
            "attachments.addByURL".to_string(),
            Some(json!({
                "parentKey": "ITEM1",
                "url": "https://example.com/paper.pdf",
            })),
        )]
    );
}

#[test]
fn attachments_add_by_url_accepts_legacy_url_alias() {
    let mut client = FakeClient::with_response(json!({
        "key": "ATT3",
        "title": "Remote PDF",
        "parentKey": "ITEM1",
    }));

    let out = run_with_client(
        [
            "zotron",
            "attachments",
            "add-by-url",
            "--parent",
            "ITEM1",
            "--url",
            "https://example.com/paper.pdf",
            "--title",
            "Remote PDF",
        ],
        &mut client,
    )
    .expect("legacy --url alias succeeds");

    assert_eq!(
        client.calls,
        vec![(
            "attachments.addByURL".to_string(),
            Some(json!({
                "parentKey": "ITEM1",
                "url": "https://example.com/paper.pdf",
                "title": "Remote PDF",
            })),
        )]
    );
    assert_eq!(
        out,
        "{\"key\": \"ATT3\", \"title\": \"Remote PDF\", \"parentKey\": \"ITEM1\"}\n"
    );
}

#[test]
fn attachments_add_dry_run_translates_local_path_for_zotero() {
    let path =
        std::env::temp_dir().join(format!("zotron-cli-path-smoke-{}.pdf", std::process::id()));
    fs::write(&path, b"%PDF- test").expect("write temp pdf");
    let expected_path = expected_zotero_path(&path);
    let mut client = FakeClient::default();

    let out = run_with_client(
        [
            "zotron",
            "attachments",
            "add",
            "--parent",
            "ITEM1",
            "--path",
            path.to_str().expect("temp path is utf8"),
            "--dry-run",
        ],
        &mut client,
    )
    .expect("attachment dry-run succeeds");

    let payload: Value = serde_json::from_str(&out).expect("dry-run output is JSON");
    assert_eq!(payload["wouldCall"], "attachments.add");
    assert_eq!(payload["wouldCallParams"]["path"], expected_path);
    assert!(client.calls.is_empty(), "dry-run should not call RPC");
    let _ = fs::remove_file(path);
}

#[test]
fn collections_get_accepts_collection_key_reference() {
    let mut client = FakeClient::with_responses(vec![
        json!([
            {"key": "COL1", "name": "Research", "parentKey": null},
            {"key": "COL2", "name": "Other", "parentKey": null},
        ]),
        json!({"key": "COL1", "name": "Research", "parentKey": null}),
    ]);

    let out = run_with_client(["zotron", "collections", "get", "COL1"], &mut client)
        .expect("collection key reference resolves");

    assert_eq!(
        client.calls,
        vec![
            ("collections.list".to_string(), None),
            ("collections.get".to_string(), Some(json!({"key": "COL1"})),),
        ]
    );
    let payload: Value = serde_json::from_str(&out).expect("collection output is JSON");
    assert_eq!(payload["key"], "COL1");
    assert_eq!(payload["name"], "Research");
}

#[test]
fn top_level_help_returns_text_without_rpc_calls() {
    let mut client = FakeClient::default();
    let out = run_with_client(["zotron", "--help"], &mut client).expect("help succeeds");

    assert!(out.contains("Rust client + CLI for the Zotron XPI"));
    assert!(out.contains("Usage: zotron <COMMAND>"));
    assert!(client.calls.is_empty(), "help should not call RPC");
}

#[test]
fn zotron_binary_help_exits_successfully() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_zotron"))
        .arg("--help")
        .output()
        .expect("run zotron --help");

    assert!(output.status.success(), "status: {:?}", output.status);
    assert!(
        output.stderr.is_empty(),
        "stderr should be empty for help: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Usage: zotron <COMMAND>"));
}

#[test]
fn zotron_binary_runtime_errors_are_structured_json_on_stderr() {
    let output = ProcessCommand::new(env!("CARGO_BIN_EXE_zotron"))
        .args(["rpc", "items.get", "not-json"])
        .output()
        .expect("run zotron invalid json");

    assert!(!output.status.success(), "status: {:?}", output.status);
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty for errors: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr is utf8");
    let payload: Value = serde_json::from_str(stderr.trim()).expect("stderr is JSON");
    assert_eq!(payload["error"]["code"], "INVALID_JSON");
    assert!(payload["error"]["message"]
        .as_str()
        .expect("error message is a string")
        .contains("params must be a JSON object"));
}

#[test]
fn fixture_covered_commands_match_python_cli_parity_contracts() {
    for name in fixture_names() {
        let fixture = load_fixture(&name);
        let mut args = vec!["zotron".to_string()];
        args.extend(
            fixture["command"]
                .as_array()
                .expect("fixture command is an array")
                .iter()
                .map(|arg| arg.as_str().expect("command args are strings").to_string()),
        );

        let mut client = FakeClient::with_responses(expected_results(&fixture));
        let out = run_with_client(args.iter().map(String::as_str), &mut client)
            .unwrap_or_else(|err| panic!("{name} should succeed: {err}"));

        assert_eq!(
            out, fixture["expect"]["stdout"],
            "stdout mismatch for {name}"
        );
        assert_eq!(
            client.calls,
            expected_calls(&fixture),
            "RPC call mismatch for {name}"
        );
    }
}

#[test]
fn annotations_create_rejects_invalid_position_shape_without_rpc() {
    let mut client = FakeClient::default();
    let err = run_with_client(
        [
            "zotron",
            "annotations",
            "create",
            "--parent",
            "ATTACH1",
            "--type",
            "highlight",
            "--position",
            "{\"foo\":1}",
            "--dry-run",
        ],
        &mut client,
    )
    .expect_err("invalid position is rejected");

    assert!(err.contains("INVALID_ARGS"), "{err}");
    assert!(err.contains("pageIndex") || err.contains("rects"), "{err}");
    assert!(client.calls.is_empty(), "validation should fail before RPC");
}

#[test]
fn annotations_create_rejects_json_boolean_sort_index_without_rpc() {
    let mut client = FakeClient::default();
    let err = run_with_client(
        [
            "zotron",
            "annotations",
            "create",
            "--parent",
            "ATTACH1",
            "--type",
            "highlight",
            "--position",
            "{\"pageIndex\":0,\"rects\":[[1,2,3,4]]}",
            "--sort-index",
            "true",
            "--dry-run",
        ],
        &mut client,
    )
    .expect_err("boolean sort index is rejected");

    assert!(err.contains("INVALID_ARGS"), "{err}");
    assert!(err.contains("sort-index"), "{err}");
    assert!(client.calls.is_empty(), "validation should fail before RPC");
}

#[test]
fn ocr_provider_contracts_are_key_first_for_glm_paddle_and_mineru() {
    let specs = zotron_cli::ocr_provider_specs();
    let ids: Vec<_> = specs.iter().map(|spec| spec.id).collect();
    assert!(ids.contains(&"glm"));
    assert!(ids.contains(&"paddleocr-vl"));
    assert!(ids.contains(&"mineru"));

    let glm = zotron_cli::ocr_provider_spec("glm").expect("glm spec");
    assert_eq!(glm.request_style, "glm-layout-parsing");
    assert_eq!(glm.auth, "bearer");
    assert!(glm.supports_pdf_direct);

    let paddle = zotron_cli::ocr_provider_spec("paddleocr-vl").expect("paddle spec");
    assert_eq!(paddle.request_style, "paddleocr-vl");
    assert_eq!(paddle.auth, "token");
    assert_eq!(paddle.auth_header, "Authorization");

    let mineru = zotron_cli::ocr_provider_spec("mineru").expect("mineru spec");
    assert_eq!(mineru.request_style, "mineru-cli");
    assert_eq!(mineru.auth, "none");

    let serialized = serde_json::to_value(glm).expect("spec serializes");
    assert!(
        serialized.get("provider_id").is_none(),
        "provider specs must not expose *_id"
    );
    assert!(
        serialized.get("item_id").is_none(),
        "provider specs must not expose item_id"
    );
    assert_eq!(serialized["key_field"], "attachment_key");
}

#[test]
fn zotron_ocr_providers_prints_provider_matrix_without_rpc() {
    let mut client = FakeClient::default();
    let out =
        run_ocr_with_client(["zotron-ocr", "providers"], &mut client).expect("providers succeeds");
    let payload: Value = serde_json::from_str(&out).expect("providers output is JSON");

    assert!(client.calls.is_empty());
    assert_eq!(payload["providers"][0]["key_field"], "attachment_key");
    assert!(payload["providers"]
        .as_array()
        .expect("providers array")
        .iter()
        .any(|provider| provider["id"] == "paddleocr-vl"));
}

#[test]
fn embedding_provider_contracts_cover_volcengine_alibaba_and_custom_without_ids() {
    let volcengine = zotron_cli::embedding_provider_spec("volcengine").expect("volcengine spec");
    assert_eq!(volcengine.request_style, "openai-compatible");
    assert!(volcengine.default_url.contains("volces.com"));
    assert_eq!(volcengine.key_field, "item_key");

    let alibaba = zotron_cli::embedding_provider_spec("alibaba").expect("alibaba spec");
    assert_eq!(alibaba.provider, "alibaba");
    assert!(alibaba.default_url.contains("dashscope.aliyuncs.com"));

    let custom = zotron_cli::embedding_provider_spec("custom").expect("custom spec");
    assert_eq!(custom.default_url, "");
    assert_eq!(custom.auth, "bearer");

    let serialized =
        serde_json::to_string(&[volcengine, alibaba, custom]).expect("specs serialize");
    assert!(!serialized.contains("provider_id"));
    assert!(!serialized.contains("item_id"));
    assert!(!serialized.contains("attachment_id"));
}

#[test]
fn zotron_rag_embedding_providers_prints_provider_matrix_without_rpc() {
    let mut client = FakeClient::default();
    let out = run_rag_with_client(["zotron-rag", "embedding-providers"], &mut client)
        .expect("embedding providers succeeds");
    let payload: Value = serde_json::from_str(&out).expect("embedding providers output is JSON");

    assert!(client.calls.is_empty());
    assert!(payload["providers"]
        .as_array()
        .expect("providers array")
        .iter()
        .any(|provider| provider["provider"] == "volcengine"));
    assert!(payload["providers"]
        .as_array()
        .expect("providers array")
        .iter()
        .any(|provider| provider["provider"] == "alibaba"));
}

#[test]
fn zotron_ocr_provider_json_executes_local_http_with_endpoint_and_env_credential() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local provider server");
    let url = format!("http://{}", listener.local_addr().expect("local addr"));
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept OCR provider request");
        let mut reader = BufReader::new(stream);
        let (headers, body) = read_http_request(&mut reader);
        tx.send((headers, body)).expect("send captured OCR request");

        let response = r#"{"choices":[{"message":{"content":"{\"pages\":[{\"page\":3,\"blocks\":[{\"type\":\"text\",\"text\":\"GLM CLI 正文\"}]}]}"}}]}"#
            .as_bytes();
        let mut stream = reader.into_inner();
        write_json_response(&mut stream, response);
    });

    let input_path = temp_json_file(
        "zotron-ocr-provider-json",
        &json!({
            "item_key": "ITEMKEY",
            "attachment_key": "ATTACHKEY",
            "file_name": "paper.pdf",
            "mime_type": "application/pdf",
            "content_base64": "JVBERi0x"
        }),
    );
    std::env::set_var("ZOTRON_TEST_OCR_KEY", "ocr-env-key");

    let out = zotron_cli::run_ocr([
        "zotron-ocr",
        "provider-json",
        "--provider",
        "glm",
        "--input",
        input_path.to_str().expect("temp input path is utf8"),
        "--endpoint",
        &url,
        "--api-key-env",
        "ZOTRON_TEST_OCR_KEY",
    ])
    .expect("OCR provider-json succeeds");

    let payload: Value = serde_json::from_str(&out).expect("OCR provider output is JSON");
    assert_eq!(payload["provider"], "glm");
    assert_eq!(payload["blocks"][0]["block_key"], "ATTACHKEY:p3:b0");
    assert_eq!(payload["blocks"][0]["text"], "GLM CLI 正文");

    let (headers, body) = rx.recv().expect("captured OCR request");
    assert!(headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("Authorization: Bearer ocr-env-key")));
    let request_body: Value = serde_json::from_str(&body).expect("request body is JSON");
    assert_eq!(request_body["model"], "glm-ocr");
    assert_eq!(request_body["file"], "data:application/pdf;base64,JVBERi0x");
    assert!(
        request_body.get("item_key").is_none(),
        "provider body should not leak local item key"
    );
    assert!(
        request_body.get("attachment_key").is_none(),
        "provider body should not leak local attachment key"
    );

    std::env::remove_var("ZOTRON_TEST_OCR_KEY");
    let _ = fs::remove_file(input_path);
    handle.join().expect("server thread joins");
}

#[test]
fn zotron_rag_embedding_json_executes_custom_provider_against_local_http() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local embedding server");
    let url = format!("http://{}", listener.local_addr().expect("local addr"));
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (stream, _) = listener
            .accept()
            .expect("accept embedding provider request");
        let mut reader = BufReader::new(stream);
        let (headers, body) = read_http_request(&mut reader);
        tx.send((headers, body))
            .expect("send captured embedding request");

        let response = br#"{"model":"local-bge","data":[{"index":0,"embedding":[0.125,0.875]}]}"#;
        let mut stream = reader.into_inner();
        write_json_response(&mut stream, response);
    });

    let input_path = temp_json_file(
        "zotron-rag-embedding-json",
        &json!({
            "item_key": "ITEMKEY",
            "chunks": [{"chunk_key": "ATTACHKEY:c0", "text": "结构化证据块。"}]
        }),
    );
    std::env::set_var("ZOTRON_TEST_EMBED_KEY", "embed-env-key");

    let out = zotron_cli::run_rag([
        "zotron-rag",
        "embedding-json",
        "--provider",
        "custom",
        "--input",
        input_path.to_str().expect("temp input path is utf8"),
        "--endpoint",
        &url,
        "--model",
        "local-bge",
        "--api-key-env",
        "ZOTRON_TEST_EMBED_KEY",
    ])
    .expect("embedding-json succeeds");

    let payload: Value = serde_json::from_str(&out).expect("embedding output is JSON");
    assert_eq!(payload["provider"], "custom");
    assert_eq!(payload["vectors"][0]["chunk_key"], "ATTACHKEY:c0");
    assert_eq!(payload["vectors"][0]["vector"], json!([0.125, 0.875]));

    let (headers, body) = rx.recv().expect("captured embedding request");
    assert!(headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("Authorization: Bearer embed-env-key")));
    let request_body: Value = serde_json::from_str(&body).expect("request body is JSON");
    assert_eq!(request_body["model"], "local-bge");
    assert_eq!(request_body["input"], json!(["结构化证据块。"]));
    assert_eq!(request_body["chunk_keys"], json!(["ATTACHKEY:c0"]));

    std::env::remove_var("ZOTRON_TEST_EMBED_KEY");
    let _ = fs::remove_file(input_path);
    handle.join().expect("server thread joins");
}

#[test]
fn chunks_from_blocks_preserve_structure_and_do_not_cross_sections() {
    let blocks = vec![
        json!({"block_key":"ATT1:p1:b0","item_key":"ITEM1","attachment_key":"ATT1","type":"heading","page_idx":1,"text":"引言","section_path":["引言"]}),
        json!({"block_key":"ATT1:p1:b1","item_key":"ITEM1","attachment_key":"ATT1","type":"paragraph","page_idx":1,"text":"Alpha risk evidence","section_path":["引言"]}),
        json!({"block_key":"ATT1:p1:b2","item_key":"ITEM1","attachment_key":"ATT1","type":"table","page_idx":1,"bbox":[1,2,3,4],"text":"Beta table evidence","section_path":["引言"]}),
        json!({"block_key":"ATT1:p2:b0","item_key":"ITEM1","attachment_key":"ATT1","type":"heading","page_idx":2,"text":"方法","section_path":["方法"]}),
        json!({"block_key":"ATT1:p2:b1","item_key":"ITEM1","attachment_key":"ATT1","type":"paragraph","page_idx":2,"text":"Gamma model evidence","section_path":["方法"]}),
    ];

    let chunks = zotron_cli::chunks_from_blocks(&blocks, 1000).expect("chunk blocks");
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0]["chunk_key"], "ATT1:c0");
    assert_eq!(chunks[0]["item_key"], "ITEM1");
    assert_eq!(chunks[0]["attachment_key"], "ATT1");
    assert_eq!(chunks[0]["block_keys"], json!(["ATT1:p1:b1", "ATT1:p1:b2"]));
    assert_eq!(chunks[0]["section_path"], json!(["引言"]));
    assert_eq!(chunks[0]["page_start"], 1);
    assert_eq!(chunks[0]["page_end"], 1);
    assert_eq!(chunks[0]["evidence_refs"][1]["bbox"], json!([1, 2, 3, 4]));
    assert_eq!(chunks[1]["section_path"], json!(["方法"]));

    let serialized = serde_json::to_string(&chunks).expect("chunks serialize");
    assert!(!serialized.contains("block_id"));
    assert!(!serialized.contains("chunk_id"));
    assert!(!serialized.contains("item_id"));
}

#[test]
fn search_quick_filters_zotron_ocr_and_embedding_artifacts_from_cli_output() {
    let mut client = FakeClient::with_response(json!({
        "items": [
            {"key": "ITEM1", "title": "Real Literature", "version": 1},
            {"key": "ATT1", "title": "ITEM1.zotron-chunks.jsonl", "version": 1},
            {"key": "ATT2", "title": "ITEM1.zotron-embed.npz", "version": 1},
            {"key": "ATT3", "title": "ITEM1.zotron-ocr.raw.zip", "version": 1}
        ],
        "total": 4
    }));

    let out = run_with_client(["zotron", "search", "quick", "risk"], &mut client)
        .expect("search succeeds");
    let payload: Value = serde_json::from_str(&out).expect("search output is JSON");
    assert_eq!(payload["total"], 1);
    assert_eq!(payload["items"].as_array().expect("items array").len(), 1);
    assert_eq!(payload["items"][0]["key"], "ITEM1");
}

#[test]
fn search_quick_collection_filters_collection_items_locally() {
    let mut client = FakeClient::with_responses(vec![
        json!([{"key": "COL1", "name": "习中心", "parentKey": null}]),
        json!({
            "items": [
                {"key": "ITEM1", "title": "宏观因子与资产配置"},
                {"key": "ITEM2", "title": "产业政策评估"},
                {"key": "ITEM3", "title": "宏观因子定价", "creators": [{"lastName": "王"}]},
                {"key": "ATT1", "title": "ITEM1.zotron-chunks.jsonl"}
            ],
            "total": 4
        }),
    ]);

    let out = run_with_client(
        [
            "zotron",
            "search",
            "quick",
            "宏观 因子",
            "--collection",
            "习中心",
            "--limit",
            "1",
        ],
        &mut client,
    )
    .expect("collection quick search succeeds");
    let payload: Value = serde_json::from_str(&out).expect("search output is JSON");

    assert_eq!(payload["total"], 2);
    assert_eq!(payload["items"].as_array().expect("items").len(), 1);
    assert_eq!(payload["items"][0]["key"], "ITEM1");
    assert_eq!(
        client.calls,
        vec![
            ("collections.list".to_string(), None),
            (
                "collections.getItems".to_string(),
                Some(json!({"key": "COL1"})),
            ),
        ]
    );
}

#[test]
fn collections_items_alias_calls_get_items_rpc() {
    let mut client = FakeClient::with_responses(vec![
        json!([{"key": "COL1", "name": "习中心", "parentKey": null}]),
        json!({"items": [{"key": "ITEM1", "title": "A"}], "total": 1}),
    ]);

    let out = run_with_client(["zotron", "collections", "items", "习中心"], &mut client)
        .expect("collections items alias succeeds");
    let payload: Value = serde_json::from_str(&out).expect("items output is JSON");

    assert_eq!(payload["total"], 1);
    assert_eq!(
        client.calls,
        vec![
            ("collections.list".to_string(), None),
            (
                "collections.getItems".to_string(),
                Some(json!({"key": "COL1"})),
            ),
        ]
    );
}

#[test]
fn ocr_status_fixture_matches_python_cli_behavior() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let original_store = std::env::var_os("ZOTRON_ARTIFACT_STORE");
    std::env::remove_var("ZOTRON_ARTIFACT_STORE");
    let fixture = load_fixture_from("ocr-cli-parity", "status");
    let mut args = vec!["zotron-ocr".to_string()];
    args.extend(
        fixture["command"]
            .as_array()
            .expect("fixture command is an array")
            .iter()
            .map(|arg| arg.as_str().expect("command args are strings").to_string()),
    );

    let mut client = FakeClient::with_responses(expected_results(&fixture));
    let out = run_ocr_with_client(args.iter().map(String::as_str), &mut client)
        .unwrap_or_else(|err| panic!("zotron-ocr status should succeed: {err}"));

    assert_eq!(out, fixture["expect"]["stdout"]);
    assert_eq!(client.calls, expected_calls(&fixture));
    match original_store {
        Some(value) => std::env::set_var("ZOTRON_ARTIFACT_STORE", value),
        None => std::env::remove_var("ZOTRON_ARTIFACT_STORE"),
    }
}

#[test]
fn ocr_status_prefers_external_artifact_store_without_attachment_rpc() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let original_store = std::env::var_os("ZOTRON_ARTIFACT_STORE");
    let store =
        std::env::temp_dir().join(format!("zotron-cli-artifact-store-{}", std::process::id()));
    let _ = fs::remove_dir_all(&store);
    std::env::set_var("ZOTRON_ARTIFACT_STORE", &store);
    zotron_types::write_machine_artifact(
        &store,
        "ITEM11",
        "ATT11",
        zotron_types::MachineArtifactKind::Chunks,
        br#"{"chunk_key":"ATT11:c0","text":"evidence"}\n"#,
    )
    .expect("write external chunks artifact");

    let mut client = FakeClient::with_responses(vec![
        json!([{"key": "COL1", "name": "Research", "children": []}]),
        json!({"items": [{"key": "ITEM11", "title": "Paper"}]}),
    ]);

    let out = run_ocr_with_client(
        ["zotron-ocr", "status", "--collection", "Research"],
        &mut client,
    )
    .expect("ocr status succeeds from external artifact store");
    let payload: Value = serde_json::from_str(&out).expect("status output is JSON");

    assert_eq!(payload["has_ocr"], 1);
    assert_eq!(payload["missing_ocr"], 0);
    assert_eq!(
        client.calls,
        vec![
            ("collections.tree".to_string(), None),
            (
                "collections.getItems".to_string(),
                Some(json!({"key": "COL1", "limit": 500, "offset": 0}))
            ),
        ],
        "external artifacts should not require attachments.list, notes.get, or attachments.add"
    );

    match original_store {
        Some(value) => std::env::set_var("ZOTRON_ARTIFACT_STORE", value),
        None => std::env::remove_var("ZOTRON_ARTIFACT_STORE"),
    }
    let _ = fs::remove_dir_all(&store);
}

#[test]
fn ocr_status_missing_collection_returns_coded_error() {
    let mut client = FakeClient::with_response(json!([
        {"key": "OTHER", "name": "Other", "children": []}
    ]));

    let err = run_ocr_with_client(
        ["zotron-ocr", "status", "--collection", "Missing"],
        &mut client,
    )
    .expect_err("missing collection should fail");

    assert!(err.contains("COLLECTION_NOT_FOUND"), "{err}");
    assert!(err.contains("Missing"), "{err}");
    assert_eq!(client.calls, vec![("collections.tree".to_string(), None)]);
}

#[test]
fn ocr_status_paginates_collection_items_before_counting_ocr() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let original_store = std::env::var_os("ZOTRON_ARTIFACT_STORE");
    let store = std::env::temp_dir().join(format!(
        "zotron-cli-ocr-status-paginate-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&store);
    std::env::set_var("ZOTRON_ARTIFACT_STORE", &store);

    let first_page = (0..500)
        .map(|idx| {
            let item_key = format!("ITEM{idx:03}");
            zotron_types::write_machine_artifact(
                &store,
                &item_key,
                &format!("ATT{idx:03}"),
                zotron_types::MachineArtifactKind::Chunks,
                br#"{"chunk_key":"c0","text":"evidence"}\n"#,
            )
            .unwrap_or_else(|err| panic!("write artifact for {item_key}: {err}"));
            json!({"key": item_key, "title": format!("Paper {idx}")})
        })
        .collect::<Vec<_>>();
    zotron_types::write_machine_artifact(
        &store,
        "ITEM500",
        "ATT500",
        zotron_types::MachineArtifactKind::Chunks,
        br#"{"chunk_key":"c0","text":"evidence"}\n"#,
    )
    .expect("write last page artifact");

    let mut client = FakeClient::with_responses(vec![
        json!([{"key": "COL1", "name": "Research", "children": []}]),
        json!({"items": first_page}),
        json!({"items": [{"key": "ITEM500", "title": "Paper 500"}]}),
    ]);

    let out = run_ocr_with_client(
        ["zotron-ocr", "status", "--collection", "Research"],
        &mut client,
    )
    .expect("ocr status succeeds with pagination");
    let payload: Value = serde_json::from_str(&out).expect("status output is JSON");

    assert_eq!(payload["total"], 501);
    assert_eq!(payload["has_ocr"], 501);
    assert_eq!(payload["missing_ocr"], 0);
    assert_eq!(
        client.calls,
        vec![
            ("collections.tree".to_string(), None),
            (
                "collections.getItems".to_string(),
                Some(json!({"key": "COL1", "limit": 500, "offset": 0}))
            ),
            (
                "collections.getItems".to_string(),
                Some(json!({"key": "COL1", "limit": 500, "offset": 500}))
            ),
        ]
    );

    match original_store {
        Some(value) => std::env::set_var("ZOTRON_ARTIFACT_STORE", value),
        None => std::env::remove_var("ZOTRON_ARTIFACT_STORE"),
    }
    let _ = fs::remove_dir_all(&store);
}

#[test]
fn rag_status_prefers_xdg_data_home_over_home_default_path() {
    let _guard = ENV_LOCK.lock().expect("env lock");
    let original_home = std::env::var_os("HOME");
    let original_xdg = std::env::var_os("XDG_DATA_HOME");
    let test_home = std::env::temp_dir().join(format!("zotron-rag-home-{}", std::process::id()));
    let test_xdg = std::env::temp_dir().join(format!("zotron-rag-xdg-{}", std::process::id()));
    let _ = fs::remove_dir_all(&test_home);
    let _ = fs::remove_dir_all(&test_xdg);
    fs::create_dir_all(test_xdg.join("zotron").join("rag")).expect("create xdg rag dir");
    std::env::set_var("HOME", &test_home);
    std::env::set_var("XDG_DATA_HOME", &test_xdg);

    let store_path = test_xdg.join("zotron").join("rag").join("Research.json");
    fs::write(
        &store_path,
        serde_json::to_vec(&json!({
            "collection": "Research",
            "collection_key": "COL1",
            "model": "embed-v1",
            "chunks": [
                {"item_key": "ITEM1"},
                {"item_key": "ITEM1"},
                {"item_key": "ITEM2"}
            ]
        }))
        .expect("serialize rag store"),
    )
    .expect("write rag store");

    let mut client = FakeClient::default();
    let out = run_rag_with_client(
        ["zotron-rag", "status", "--collection", "Research"],
        &mut client,
    )
    .expect("rag status succeeds from xdg path");
    let payload: Value = serde_json::from_str(&out).expect("status output is JSON");

    assert_eq!(payload["status"], "indexed");
    assert_eq!(payload["total_chunks"], 3);
    assert_eq!(payload["total_items"], 2);
    assert_eq!(payload["store_path"], store_path.to_string_lossy().as_ref());
    assert!(client.calls.is_empty(), "rag status should not call RPC");

    match original_home {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    match original_xdg {
        Some(value) => std::env::set_var("XDG_DATA_HOME", value),
        None => std::env::remove_var("XDG_DATA_HOME"),
    }
    let _ = fs::remove_dir_all(&test_home);
    let _ = fs::remove_dir_all(&test_xdg);
}

#[test]
fn rag_hits_missing_collection_returns_coded_error_instead_of_raw_json() {
    let mut client = FakeClient::default();
    let err = run_rag_with_client(["zotron-rag", "hits", "query", "--zotero"], &mut client)
        .expect_err("missing collection should fail");

    assert_eq!(
        err,
        "INVALID_ARGS: --collection is required when --zotero is used"
    );
    assert!(
        !err.trim_start().starts_with('{'),
        "error should remain a plain coded message so binaries can format it exactly once"
    );
}

#[test]
fn rag_fixture_covered_commands_match_python_cli_parity_contracts() {
    let original_home = std::env::var_os("HOME");
    let test_home = std::env::temp_dir().join(format!("zotron-rag-parity-{}", std::process::id()));
    let _ = fs::remove_dir_all(&test_home);
    fs::create_dir_all(&test_home).expect("create isolated HOME");
    std::env::set_var("HOME", &test_home);

    for name in rag_fixture_names() {
        let fixture = load_rag_fixture(&name);
        let mut args = vec!["zotron-rag".to_string()];
        args.extend(
            fixture["command"]
                .as_array()
                .expect("fixture command is an array")
                .iter()
                .map(|arg| arg.as_str().expect("command args are strings").to_string()),
        );

        let mut client = FakeClient::with_responses(expected_results(&fixture));
        let out = run_rag_with_client(args.iter().map(String::as_str), &mut client)
            .unwrap_or_else(|err| panic!("{name} should succeed: {err}"));

        assert_eq!(
            out, fixture["expect"]["stdout"],
            "stdout mismatch for {name}"
        );
        assert_eq!(
            client.calls,
            expected_calls(&fixture),
            "RPC call mismatch for {name}"
        );
    }

    match original_home {
        Some(home) => std::env::set_var("HOME", home),
        None => std::env::remove_var("HOME"),
    }
    let _ = fs::remove_dir_all(&test_home);
}

#[test]
fn default_package_exposes_only_stable_zotron_binary() {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = fs::read_to_string(&manifest_path)
        .unwrap_or_else(|err| panic!("read {manifest_path:?}: {err}"));

    assert!(
        manifest.contains("default = []"),
        "zotron-cli should keep default features empty so the basic package exposes only stable bins"
    );
    assert!(
        manifest.contains("unstable-rust-ocr-rag-bins = []"),
        "unfinished Rust OCR/RAG binaries should require an explicit opt-in feature"
    );

    let zotron_bin = manifest_section(&manifest, "name = \"zotron\"");
    assert!(
        !zotron_bin.contains("required-features"),
        "the stable zotron binary must remain available by default"
    );

    for bin_name in ["zotron-ocr", "zotron-rag"] {
        let section = manifest_section(&manifest, &format!("name = \"{bin_name}\""));
        assert!(
            section.contains("required-features = [\"unstable-rust-ocr-rag-bins\"]"),
            "{bin_name} must stay out of the default basic package surface"
        );
    }
}

fn temp_json_file(prefix: &str, value: &Value) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "{prefix}-{}-{}.json",
        std::process::id(),
        thread_id_suffix()
    ));
    fs::write(
        &path,
        serde_json::to_vec(value).expect("serialize temp JSON"),
    )
    .expect("write temp JSON");
    path
}

fn thread_id_suffix() -> String {
    format!("{:?}", thread::current().id())
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn read_http_request(reader: &mut BufReader<std::net::TcpStream>) -> (Vec<String>, String) {
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("read header line");
        let trimmed = line.trim_end().to_string();
        if trimmed.is_empty() {
            break;
        }
        headers.push(trimmed);
    }
    let content_length = headers
        .iter()
        .find_map(|header| {
            let (name, value) = header.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .expect("content-length header");
    let mut body = vec![0; content_length];
    reader.read_exact(&mut body).expect("read request body");
    (headers, String::from_utf8(body).expect("utf8 request body"))
}

fn write_json_response(stream: &mut std::net::TcpStream, response: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.len()
    )
    .expect("write response headers");
    stream.write_all(response).expect("write response body");
}

fn fixture_names() -> Vec<String> {
    let dir = repo_root().join("fixtures").join("cli-parity");
    let mut names = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {dir:?}: {err}"))
        .filter_map(|entry| {
            let entry = entry.unwrap_or_else(|err| panic!("read fixture dir entry: {err}"));
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("json")).then_some(path)
        })
        .map(|entry| {
            entry
                .file_stem()
                .expect("fixture path has stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn load_fixture(name: &str) -> Value {
    load_fixture_from("cli-parity", name)
}

fn load_fixture_from(dir_name: &str, name: &str) -> Value {
    let path = repo_root()
        .join("fixtures")
        .join(dir_name)
        .join(format!("{name}.json"));
    let raw = fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {path:?}: {err}"));
    serde_json::from_str(&raw).unwrap_or_else(|err| panic!("parse {path:?}: {err}"))
}

fn rag_fixture_names() -> Vec<String> {
    let dir = repo_root().join("fixtures").join("rag-parity");
    let mut names = fs::read_dir(&dir)
        .unwrap_or_else(|err| panic!("read {dir:?}: {err}"))
        .filter_map(|entry| {
            let entry = entry.unwrap_or_else(|err| panic!("read fixture dir entry: {err}"));
            let path = entry.path();
            (path.extension().and_then(|ext| ext.to_str()) == Some("json")).then_some(path)
        })
        .map(|entry| {
            entry
                .file_stem()
                .expect("fixture path has stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn load_rag_fixture(name: &str) -> Value {
    load_fixture_from("rag-parity", name)
}

fn manifest_section<'a>(manifest: &'a str, needle: &str) -> &'a str {
    let start = manifest
        .find(needle)
        .unwrap_or_else(|| panic!("manifest section containing {needle:?} exists"));
    let tail = &manifest[start..];
    let end = tail.find("\n[[bin]]").unwrap_or(tail.len());
    &tail[..end]
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate has parent")
        .parent()
        .expect("workspace has parent")
        .to_path_buf()
}

fn expected_results(fixture: &Value) -> Vec<Value> {
    if let Some(calls) = fixture["rpc"].get("calls").and_then(Value::as_array) {
        calls.iter().map(|call| call["result"].clone()).collect()
    } else {
        vec![fixture["rpc"]["result"].clone()]
    }
}

fn expected_calls(fixture: &Value) -> Vec<(String, Option<Value>)> {
    if let Some(calls) = fixture["rpc"].get("calls").and_then(Value::as_array) {
        calls.iter().map(expected_call).collect()
    } else {
        vec![expected_call(&fixture["rpc"])]
    }
}

fn expected_call(call: &Value) -> (String, Option<Value>) {
    let method = call["method"]
        .as_str()
        .expect("fixture RPC method is a string")
        .to_string();
    let params = call.get("params").and_then(|params| {
        if params.is_null() {
            None
        } else {
            Some(params.clone())
        }
    });
    (method, params)
}

fn expected_zotero_path(path: &Path) -> String {
    let canonical = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();
    if test_is_wsl() {
        return ProcessCommand::new("wslpath")
            .arg("-w")
            .arg(&canonical)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|converted| converted.trim().to_string())
            .filter(|converted| !converted.is_empty())
            .unwrap_or(canonical);
    }
    canonical
}

fn test_is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|release| release.to_ascii_lowercase().contains("microsoft"))
            .unwrap_or(false)
}
