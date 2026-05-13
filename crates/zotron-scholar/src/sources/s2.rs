use crate::sources::Source;
use crate::types::Paper;

pub struct SemanticScholar;

impl SemanticScholar {
    pub fn new() -> Self {
        Self
    }
}

impl Source for SemanticScholar {
    fn name(&self) -> &str {
        "s2"
    }

    fn search(&self, _query: &str, _limit: usize) -> Result<Vec<Paper>, String> {
        Err("semantic scholar search not yet implemented".into())
    }
}
