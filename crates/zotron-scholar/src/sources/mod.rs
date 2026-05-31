pub mod arxiv;
pub mod crossref;
pub mod openalex;
pub mod s2;
pub mod unpaywall;

pub use arxiv::ArXiv;
pub use crossref::CrossRef;
pub use openalex::OpenAlex;
pub use s2::SemanticScholar;
use crate::types::Paper;

pub trait Source {
    #[allow(dead_code)]
    fn name(&self) -> &str;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<Paper>, String>;
}
