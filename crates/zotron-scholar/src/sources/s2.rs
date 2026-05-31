use serde_json::Value;

use crate::sources::Source;
use crate::types::{Author, Paper};

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

    fn search(&self, query: &str, limit: usize) -> Result<Vec<Paper>, String> {
        let capped = limit.min(100);
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search\
             ?query={}&limit={}\
             &fields=title,authors,year,abstract,externalIds,openAccessPdf,journal,url,citationCount",
            urlenc(query),
            capped,
        );

        let resp: Value = ureq::get(&url)
            .call()
            .map_err(|e| format!("Semantic Scholar request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("Semantic Scholar JSON parse failed: {e}"))?;

        let data = resp["data"]
            .as_array()
            .ok_or("Semantic Scholar response missing 'data' array")?;

        let papers: Vec<Paper> = data.iter().map(parse_paper).collect();
        Ok(papers)
    }
}

fn parse_paper(p: &Value) -> Paper {
    let title = p["title"]
        .as_str()
        .unwrap_or("(untitled)")
        .to_string();

    let authors: Vec<Author> = p["authors"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    a["name"]
                        .as_str()
                        .map(|name| Author { name: name.to_string() })
                })
                .collect()
        })
        .unwrap_or_default();

    let doi = p["externalIds"]["DOI"]
        .as_str()
        .map(|s| s.to_string());

    let arxiv_id = p["externalIds"]["ArXiv"]
        .as_str()
        .map(|s| s.to_string());

    let abstract_text = p["abstract"]
        .as_str()
        .map(|s| s.to_string());

    let date = p["year"]
        .as_u64()
        .map(|y| y.to_string())
        .or_else(|| p["year"].as_str().map(|s| s.to_string()));

    let publication = p["journal"]["name"]
        .as_str()
        .map(|s| s.to_string());

    let volume = p["journal"]["volume"]
        .as_str()
        .map(|s| s.to_string());

    let url = p["url"]
        .as_str()
        .map(|s| s.to_string());

    let pdf_url = p["openAccessPdf"]["url"]
        .as_str()
        .map(|s| s.to_string());

    Paper {
        title,
        authors,
        doi,
        abstract_text,
        date,
        publication,
        volume,
        pages: None,
        url,
        pdf_url,
        arxiv_id,
        source: Some("s2".to_string()),
        cited_by_count: p["citationCount"].as_u64(),
    }
}

/// Percent-encode a string for use in URL query parameters.
fn urlenc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urlenc() {
        assert_eq!(urlenc("hello world"), "hello%20world");
        assert_eq!(urlenc("attention is all you need"), "attention%20is%20all%20you%20need");
    }

    #[test]
    fn test_parse_paper_full() {
        let p = serde_json::json!({
            "title": "Attention Is All You Need",
            "authors": [
                {"name": "Ashish Vaswani"},
                {"name": "Noam Shazeer"}
            ],
            "year": 2017,
            "abstract": "The dominant sequence transduction models...",
            "externalIds": {
                "DOI": "10.48550/arXiv.1706.03762",
                "ArXiv": "1706.03762"
            },
            "openAccessPdf": {
                "url": "https://arxiv.org/pdf/1706.03762"
            },
            "journal": {
                "name": "Advances in Neural Information Processing Systems",
                "volume": "30"
            },
            "url": "https://www.semanticscholar.org/paper/abc123"
        });
        let paper = parse_paper(&p);
        assert_eq!(paper.title, "Attention Is All You Need");
        assert_eq!(paper.authors.len(), 2);
        assert_eq!(paper.authors[0].name, "Ashish Vaswani");
        assert_eq!(paper.authors[1].name, "Noam Shazeer");
        assert_eq!(paper.doi.as_deref(), Some("10.48550/arXiv.1706.03762"));
        assert_eq!(paper.arxiv_id.as_deref(), Some("1706.03762"));
        assert_eq!(
            paper.abstract_text.as_deref(),
            Some("The dominant sequence transduction models...")
        );
        assert_eq!(paper.date.as_deref(), Some("2017"));
        assert_eq!(
            paper.publication.as_deref(),
            Some("Advances in Neural Information Processing Systems")
        );
        assert_eq!(paper.volume.as_deref(), Some("30"));
        assert_eq!(
            paper.pdf_url.as_deref(),
            Some("https://arxiv.org/pdf/1706.03762")
        );
        assert_eq!(
            paper.url.as_deref(),
            Some("https://www.semanticscholar.org/paper/abc123")
        );
        assert_eq!(paper.source.as_deref(), Some("s2"));
    }

    #[test]
    fn test_parse_paper_minimal() {
        let p = serde_json::json!({
            "title": "Minimal Paper",
        });
        let paper = parse_paper(&p);
        assert_eq!(paper.title, "Minimal Paper");
        assert!(paper.authors.is_empty());
        assert!(paper.doi.is_none());
        assert!(paper.arxiv_id.is_none());
        assert!(paper.abstract_text.is_none());
        assert!(paper.date.is_none());
        assert!(paper.publication.is_none());
        assert!(paper.pdf_url.is_none());
        assert_eq!(paper.source.as_deref(), Some("s2"));
    }

    #[test]
    fn test_parse_paper_null_fields() {
        let p = serde_json::json!({
            "title": null,
            "authors": null,
            "year": null,
            "abstract": null,
            "externalIds": null,
            "openAccessPdf": null,
            "journal": null,
            "url": null,
        });
        let paper = parse_paper(&p);
        assert_eq!(paper.title, "(untitled)");
        assert!(paper.authors.is_empty());
        assert!(paper.doi.is_none());
        assert!(paper.date.is_none());
    }

    #[test]
    fn test_parse_paper_year_as_integer() {
        let p = serde_json::json!({
            "title": "Test",
            "year": 2023,
        });
        let paper = parse_paper(&p);
        assert_eq!(paper.date.as_deref(), Some("2023"));
    }
}
