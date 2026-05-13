use crate::sources::Source;
use crate::types::Paper;

pub struct OpenAlex {
    pub email: Option<String>,
}

impl OpenAlex {
    pub fn new() -> Self {
        Self {
            email: std::env::var("ZOTRON_SCHOLAR_EMAIL").ok(),
        }
    }
}

impl Source for OpenAlex {
    fn name(&self) -> &str {
        "openalex"
    }

    fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Paper>, String> {
        Err("openalex search not yet implemented".into())
    }
}
