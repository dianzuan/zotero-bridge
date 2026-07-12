//! Built-in academic web search and fetch (formerly the zotron-scholar
//! plugin). Sources are official public REST APIs only: OpenAlex, CrossRef,
//! Semantic Scholar, arXiv; the fetch path cascades OA PDF resolvers
//! (Unpaywall, DOAJ, CORE, fatcat, publisher-direct).

pub(crate) mod fetch;
pub(crate) mod http;
pub(crate) mod sources;
pub(crate) mod types;

use crate::WebCommand;
use sources::Source;

pub(crate) fn run_web_command(command: WebCommand) -> Result<String, String> {
    match command {
        WebCommand::Search { query, limit, source } => {
            let src: Box<dyn Source> = match source.as_str() {
                "openalex" => Box::new(sources::OpenAlex::new()),
                "crossref" => Box::new(sources::CrossRef::new()),
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
                fetch::fetch_doi(&doi)?
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
