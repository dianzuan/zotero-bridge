//! Minimal Zotron JSON-RPC client for the Rust CLI scaffold.

use std::fmt;
use std::process::Command;

use serde_json::Value;
use zotron_types::{
    JsonRpcRequest, JsonRpcResponse, ProviderCommandRunner, ProviderHttpInvocation,
    ProviderHttpTransport,
};

/// Transport boundary so tests can lock wire payloads without a live Zotero app.
pub trait Transport {
    fn post_json(&mut self, url: &str, payload: &Value) -> Result<Value, RpcError>;
}

/// Blocking HTTP transport for the local Zotron XPI endpoint.
#[derive(Debug, Default, Clone, Copy)]
pub struct UreqTransport;

impl Transport for UreqTransport {
    fn post_json(&mut self, url: &str, payload: &Value) -> Result<Value, RpcError> {
        let response = ureq::post(url)
            .send_json(payload.clone())
            .map_err(|err| RpcError::Connection(format!("Cannot connect to Zotero: {err}")))?;
        response
            .into_json::<Value>()
            .map_err(|err| RpcError::Protocol(format!("Invalid JSON-RPC response: {err}")))
    }
}

/// Blocking HTTP transport for live OCR and embedding provider calls.
///
/// The provider contract keeps credentials outside request builders. Callers
/// inject one here, or pass an explicit `auth_header_value` in the invocation.
#[derive(Debug, Default, Clone)]
pub struct UreqProviderHttpTransport {
    api_key: Option<String>,
}

impl UreqProviderHttpTransport {
    pub fn new() -> Self {
        Self { api_key: None }
    }

    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
        }
    }

    pub fn with_bearer_token(token: impl AsRef<str>) -> Self {
        Self::with_api_key(format!("Bearer {}", token.as_ref()))
    }

    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }
}

impl ProviderHttpTransport for UreqProviderHttpTransport {
    fn post_json(&mut self, invocation: &ProviderHttpInvocation) -> Result<Value, String> {
        let method = invocation.method.to_ascii_uppercase();
        if method != "POST" {
            return Err(format!(
                "provider {} uses unsupported HTTP method {}",
                invocation.provider, invocation.method
            ));
        }

        let url = invocation
            .url
            .as_deref()
            .ok_or_else(|| format!("provider {} missing request URL", invocation.provider))?;

        let mut request = ureq::post(url);
        if let Some(header_name) = invocation.auth_header_name.as_deref() {
            let header_value = invocation
                .auth_header_value
                .as_deref()
                .or(self.api_key.as_deref())
                .ok_or_else(|| {
                    format!(
                        "provider {} requires {} but no credential was supplied",
                        invocation.provider, header_name
                    )
                })?;
            request = request.set(header_name, header_value);
        }

        let response = request
            .send_json(invocation.body.clone())
            .map_err(|err| format!("provider {} request failed: {err}", invocation.provider))?;
        response.into_json::<Value>().map_err(|err| {
            format!(
                "provider {} returned invalid JSON: {err}",
                invocation.provider
            )
        })
    }
}

/// Command runner for local OCR providers that emit JSON on stdout.
#[derive(Debug, Default, Clone, Copy)]
pub struct StdProviderCommandRunner;

impl ProviderCommandRunner for StdProviderCommandRunner {
    fn run_json(&mut self, command: &[String]) -> Result<Value, String> {
        let program = command
            .first()
            .ok_or_else(|| "provider command is empty".to_string())?;
        let output = Command::new(program)
            .args(&command[1..])
            .output()
            .map_err(|err| format!("provider command {program} failed to start: {err}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("provider command {program} exited with {}", output.status)
            } else {
                format!(
                    "provider command {program} exited with {}: {stderr}",
                    output.status
                )
            });
        }

        serde_json::from_slice::<Value>(&output.stdout)
            .map_err(|err| format!("provider command {program} returned invalid JSON: {err}"))
    }
}

/// Errors mirror the Python client text where caller-visible behavior exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcError {
    Connection(String),
    Protocol(String),
    JsonRpc { code: i64, message: String },
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::Connection(message) | RpcError::Protocol(message) => f.write_str(message),
            RpcError::JsonRpc { code, message } => write!(f, "[{code}] {message}"),
        }
    }
}

impl std::error::Error for RpcError {}

/// Stateful JSON-RPC client that increments request ids like the Python client.
#[derive(Debug, Clone)]
pub struct ZoteroRpc<T = UreqTransport> {
    url: String,
    transport: T,
    next_id: u64,
}

impl ZoteroRpc<UreqTransport> {
    pub fn new(url: impl Into<String>) -> Self {
        Self::with_transport(url, UreqTransport)
    }
}

impl<T: Transport> ZoteroRpc<T> {
    pub fn with_transport(url: impl Into<String>, transport: T) -> Self {
        Self {
            url: url.into(),
            transport,
            next_id: 1,
        }
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, RpcError> {
        let request = JsonRpcRequest::new(self.next_id, method, params);
        self.next_id += 1;
        let payload = serde_json::to_value(request)
            .map_err(|err| RpcError::Protocol(format!("Invalid JSON-RPC request: {err}")))?;
        let response_value = self.transport.post_json(&self.url, &payload)?;
        let response: JsonRpcResponse = serde_json::from_value(response_value)
            .map_err(|err| RpcError::Protocol(format!("Invalid JSON-RPC response: {err}")))?;

        if let Some(error) = response.error {
            return Err(RpcError::JsonRpc {
                code: error.code,
                message: error.message,
            });
        }

        Ok(response
            .result
            .unwrap_or_else(|| Value::Object(Default::default())))
    }
}
