use crate::sources::Source;
use crate::types::Paper;

pub struct CrossRef;

impl CrossRef {
    pub fn new() -> Self {
        Self
    }

    pub fn fetch_doi(&self, _doi: &str) -> Result<Paper, String> {
        Err("crossref fetch_doi not yet implemented".into())
    }
}

impl Source for CrossRef {
    fn name(&self) -> &str {
        "crossref"
    }

    fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Paper>, String> {
        Err("crossref search not yet implemented".into())
    }
}
