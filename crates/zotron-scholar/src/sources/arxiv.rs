use crate::sources::Source;
use crate::types::Paper;

pub struct ArXiv;

impl ArXiv {
    pub fn new() -> Self {
        Self
    }

    pub fn fetch_id(&self, _id: &str) -> Result<Paper, String> {
        Err("arxiv fetch_id not yet implemented".into())
    }
}

impl Source for ArXiv {
    fn name(&self) -> &str {
        "arxiv"
    }

    fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Paper>, String> {
        Err("arxiv search not yet implemented".into())
    }
}
