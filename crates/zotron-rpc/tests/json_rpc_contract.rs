use serde_json::{json, Value};
use zotron_rpc::{RpcError, Transport, ZoteroRpc};

#[derive(Default)]
struct MockTransport {
    seen: Vec<Value>,
    replies: Vec<Result<Value, RpcError>>,
}

impl Transport for MockTransport {
    fn post_json(&mut self, _url: &str, payload: &Value) -> Result<Value, RpcError> {
        self.seen.push(payload.clone());
        self.replies.remove(0)
    }
}

#[test]
fn call_posts_python_compatible_json_rpc_payload() {
    let transport = MockTransport {
        replies: vec![Ok(json!({"jsonrpc":"2.0","result":{"status":"ok"},"id":1}))],
        ..Default::default()
    };
    let mut rpc = ZoteroRpc::with_transport("http://127.0.0.1:23119/zotron/rpc", transport);

    let result = rpc.call("system.ping", None).expect("rpc succeeds");

    assert_eq!(result, json!({"status":"ok"}));
    assert_eq!(
        rpc.transport().seen[0],
        json!({
            "jsonrpc": "2.0",
            "method": "system.ping",
            "params": {},
            "id": 1
        })
    );
}

#[test]
fn json_rpc_error_matches_python_error_text() {
    let transport = MockTransport {
        replies: vec![Ok(json!({
            "jsonrpc":"2.0",
            "error":{"code":-32601,"message":"Method not found"},
            "id":1
        }))],
        ..Default::default()
    };
    let mut rpc = ZoteroRpc::with_transport("http://test/zotron/rpc", transport);

    let err = rpc.call("missing.method", None).unwrap_err();

    assert_eq!(err.to_string(), "[-32601] Method not found");
}
