//! Built-in academic web search and fetch (formerly the zotron-scholar
//! plugin). Sources are official public REST APIs only: OpenAlex, CrossRef,
//! Semantic Scholar, arXiv; the fetch path cascades OA PDF resolvers
//! (Unpaywall, DOAJ, CORE, fatcat, publisher-direct).

pub(crate) mod fetch;
pub(crate) mod http;
pub(crate) mod sources;
pub(crate) mod types;

use crate::RpcCaller;
use crate::WebCommand;
use serde_json::Value;
use sources::Source;

pub(crate) struct WebConfig {
    pub mailto: Option<String>,
    pub core_api_key: Option<String>,
}

/// Read web-source credentials from Zotero settings. Every failure —
/// Zotero not running, RPC error, blank value — degrades to `None`:
/// the public APIs work keyless, so config must never block a command.
pub(crate) fn load_web_config(client: &mut impl RpcCaller) -> WebConfig {
    WebConfig {
        mailto: fetch_setting(client, "source.mailto"),
        core_api_key: fetch_setting(client, "source.core.apiKey"),
    }
}

fn fetch_setting(client: &mut impl RpcCaller, key: &str) -> Option<String> {
    client
        .call("settings.getRaw", Some(serde_json::json!({ "key": key })))
        .ok()
        .and_then(|raw| raw.get(key).and_then(Value::as_str).map(str::to_string))
        .filter(|v| !v.trim().is_empty())
}

pub(crate) fn run_web_command(
    command: WebCommand,
    client: &mut impl RpcCaller,
) -> Result<String, String> {
    let config = load_web_config(client);
    match command {
        WebCommand::Search { query, limit, source } => {
            let src: Box<dyn Source> = match source.as_str() {
                "openalex" => Box::new(sources::OpenAlex::new(config.mailto.clone())),
                "crossref" => Box::new(sources::CrossRef::new(config.mailto.clone())),
                "s2" => Box::new(sources::SemanticScholar::new()),
                "arxiv" => Box::new(sources::ArXiv::new()),
                other => return Err(format!("unknown source: {other}")),
            };
            let papers = src.search(&query, limit)?;
            serde_json::to_string(&papers).map_err(|e| e.to_string())
        }
        WebCommand::Fetch { doi, arxiv } => {
            let (paper, pdf_path) = if let Some(arxiv_id) = arxiv {
                fetch::fetch_arxiv(&arxiv_id)?
            } else if let Some(doi) = doi {
                fetch::fetch_doi(&doi, &config)?
            } else {
                return Err("provide --doi or --arxiv".to_string());
            };
            let mut json = paper.to_zotero_json();
            if let Some(path) = pdf_path {
                json.as_object_mut().unwrap().insert(
                    "_pdf".to_string(),
                    serde_json::Value::String(path.to_string_lossy().to_string()),
                );
            }
            serde_json::to_string(&json).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RpcCaller;
    use serde_json::{json, Value};

    struct FakeRpc {
        responses: Vec<Result<Value, String>>,
        calls: Vec<(String, Option<Value>)>,
    }

    impl RpcCaller for FakeRpc {
        fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value, String> {
            self.calls.push((method.to_string(), params));
            if self.responses.is_empty() {
                Err("rpc unreachable".to_string())
            } else {
                self.responses.remove(0)
            }
        }
    }

    #[test]
    fn load_web_config_reads_settings() {
        let mut rpc = FakeRpc {
            responses: vec![
                Ok(json!({"source.mailto": "me@example.org"})),
                Ok(json!({"source.core.apiKey": "core-secret"})),
            ],
            calls: Vec::new(),
        };
        let config = load_web_config(&mut rpc);
        assert_eq!(config.mailto.as_deref(), Some("me@example.org"));
        assert_eq!(config.core_api_key.as_deref(), Some("core-secret"));
        assert_eq!(rpc.calls.len(), 2);
        assert_eq!(rpc.calls[0].0, "settings.getRaw");
        assert_eq!(rpc.calls[0].1, Some(json!({"key": "source.mailto"})));
        assert_eq!(rpc.calls[1].1, Some(json!({"key": "source.core.apiKey"})));
    }

    #[test]
    fn load_web_config_degrades_when_zotero_is_down() {
        let mut rpc = FakeRpc { responses: Vec::new(), calls: Vec::new() };
        let config = load_web_config(&mut rpc);
        assert!(config.mailto.is_none());
        assert!(config.core_api_key.is_none());
    }

    #[test]
    fn load_web_config_treats_blank_values_as_unset() {
        let mut rpc = FakeRpc {
            responses: vec![
                Ok(json!({"source.mailto": "  "})),
                Ok(json!({"source.core.apiKey": ""})),
            ],
            calls: Vec::new(),
        };
        let config = load_web_config(&mut rpc);
        assert!(config.mailto.is_none());
        assert!(config.core_api_key.is_none());
    }
}
