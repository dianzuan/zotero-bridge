use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

use serde_json::{json, Value};
use zotron_rpc::{StdProviderCommandRunner, UreqProviderHttpTransport};
use zotron_types::{
    execute_embedding_provider_request, EmbeddingChunkInput, EmbeddingRequestInput,
    ProviderCommandRunner, ProviderHttpInvocation, ProviderHttpTransport,
};

#[test]
fn provider_http_transport_posts_json_with_injected_api_key() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let url = format!("http://{}", listener.local_addr().expect("local addr"));
    let (tx, rx) = mpsc::channel();

    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept provider request");
        let mut reader = BufReader::new(stream);
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
        tx.send((headers, String::from_utf8(body).expect("utf8 request body")))
            .expect("send captured request");

        let response = br#"{"data":[{"index":0,"embedding":[1.0,2.0]}]}"#;
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        )
        .expect("write response headers");
        stream.write_all(response).expect("write response body");
    });

    let mut transport = UreqProviderHttpTransport::with_api_key("Bearer test-key");
    let payload = transport
        .post_json(&ProviderHttpInvocation {
            provider: "custom".to_string(),
            style: "openai-compatible".to_string(),
            method: "POST".to_string(),
            url: Some(url),
            auth_header_name: Some("Authorization".to_string()),
            auth_header_value: None,
            body: json!({"input":["hello"],"model":"local-bge"}),
        })
        .expect("provider request succeeds");

    assert_eq!(payload, json!({"data":[{"index":0,"embedding":[1.0,2.0]}]}));
    let (headers, body) = rx.recv().expect("captured provider request");
    assert!(headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("Authorization: Bearer test-key")));
    let request_body: Value = serde_json::from_str(&body).expect("request body is JSON");
    assert_eq!(request_body["model"], "local-bge");

    handle.join().expect("server thread joins");
}

#[test]
fn embedding_executor_can_use_live_http_adapter_against_local_endpoint() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let url = format!("http://{}", listener.local_addr().expect("local addr"));

    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept provider request");
        let mut reader = BufReader::new(stream);
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("read header line");
            if line.trim_end().is_empty() {
                break;
            }
        }

        let response = br#"{"model":"local-bge","data":[{"index":0,"embedding":[0.25,0.75]}]}"#;
        let mut stream = reader.into_inner();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response.len()
        )
        .expect("write response headers");
        stream.write_all(response).expect("write response body");
    });

    let input = EmbeddingRequestInput {
        item_key: "ITEMKEY".to_string(),
        chunks: vec![EmbeddingChunkInput {
            chunk_key: "ATTACHKEY:c0".to_string(),
            text: "结构化证据块。".to_string(),
        }],
        model: Some("local-bge".to_string()),
        url: Some(url),
        input_type: None,
    };
    let mut transport = UreqProviderHttpTransport::with_bearer_token("test-key");

    let vectors = execute_embedding_provider_request("custom", &input, &mut transport)
        .expect("executor works with concrete HTTP adapter");

    assert_eq!(vectors[0].item_key, "ITEMKEY");
    assert_eq!(vectors[0].chunk_key, "ATTACHKEY:c0");
    assert_eq!(vectors[0].vector, vec![0.25, 0.75]);

    handle.join().expect("server thread joins");
}

#[test]
fn provider_http_transport_requires_credential_when_auth_header_is_declared() {
    let mut transport = UreqProviderHttpTransport::new();

    let err = transport
        .post_json(&ProviderHttpInvocation {
            provider: "volcengine".to_string(),
            style: "openai-compatible".to_string(),
            method: "POST".to_string(),
            url: Some("http://127.0.0.1:1/embeddings".to_string()),
            auth_header_name: Some("Authorization".to_string()),
            auth_header_value: None,
            body: json!({"input":["hello"]}),
        })
        .expect_err("missing credential is rejected before network IO");

    assert!(err.contains("requires Authorization"));
}

#[test]
fn std_provider_command_runner_parses_stdout_json() {
    let mut runner = StdProviderCommandRunner;
    let command = vec![
        "sh".to_string(),
        "-c".to_string(),
        "printf '%s' '{\"pages\":[{\"page\":1,\"blocks\":[{\"text\":\"ok\"}]}]}'".to_string(),
    ];

    let payload = runner.run_json(&command).expect("command JSON parses");

    assert_eq!(payload["pages"][0]["blocks"][0]["text"], "ok");
}

#[test]
fn std_provider_command_runner_reports_nonzero_stderr() {
    let mut runner = StdProviderCommandRunner;
    let command = vec![
        "sh".to_string(),
        "-c".to_string(),
        "printf '%s' failure >&2; exit 7".to_string(),
    ];

    let err = runner
        .run_json(&command)
        .expect_err("nonzero commands fail");

    assert!(err.contains("failure"));
}
