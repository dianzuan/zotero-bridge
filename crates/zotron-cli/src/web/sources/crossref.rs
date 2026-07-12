use serde_json::Value;

use crate::web::sources::Source;
use crate::web::types::{Author, Paper};

pub struct CrossRef {
    pub email: Option<String>,
}

impl CrossRef {
    pub fn new() -> Self {
        Self {
            email: std::env::var("ZOTRON_SCHOLAR_EMAIL").ok(),
        }
    }

    pub fn fetch_doi(&self, doi: &str) -> Result<Paper, String> {
        let mut url = format!(
            "https://api.crossref.org/works/{}",
            urlenc_doi(doi),
        );
        if let Some(ref email) = self.email {
            url.push_str(&format!("?mailto={}", urlenc(email)));
        }

        let resp: Value = crate::web::http::get_with_retry(&url)
            .map_err(|e| format!("CrossRef request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("CrossRef JSON parse failed: {e}"))?;

        let msg = &resp["message"];
        if msg.is_null() {
            return Err("CrossRef response missing 'message'".into());
        }

        Ok(parse_message(msg, doi))
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

fn parse_message(msg: &Value, doi: &str) -> Paper {
    let title = msg["title"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("(untitled)")
        .to_string();

    let authors: Vec<Author> = msg["author"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    let given = a["given"].as_str().unwrap_or("");
                    let family = a["family"].as_str().unwrap_or("");
                    let name = match (given.is_empty(), family.is_empty()) {
                        (true, true) => return None,
                        (true, false) => family.to_string(),
                        (false, true) => given.to_string(),
                        (false, false) => format!("{} {}", given, family),
                    };
                    Some(Author { name })
                })
                .collect()
        })
        .unwrap_or_default();

    let abstract_text = msg["abstract"].as_str().map(|s| s.to_string());

    let date = extract_date(msg, "published-print")
        .or_else(|| extract_date(msg, "published-online"));

    let publication = msg["container-title"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let volume = msg["volume"].as_str().map(|s| s.to_string());
    let pages = msg["page"].as_str().map(|s| s.to_string());
    let url = msg["URL"].as_str().map(|s| s.to_string());

    Paper {
        title,
        authors,
        doi: Some(doi.to_string()),
        abstract_text,
        date,
        publication,
        volume,
        pages,
        url,
        pdf_url: None,
        arxiv_id: None,
        source: Some("crossref".to_string()),
        cited_by_count: msg["is-referenced-by-count"].as_u64(),
    }
}

/// Extract a date string from a CrossRef date field like "published-print" or "published-online".
/// Date parts are an array like [[2023, 4, 15]] → "2023-4-15".
fn extract_date(msg: &Value, field: &str) -> Option<String> {
    let parts = msg[field]["date-parts"]
        .as_array()?
        .first()?
        .as_array()?;

    let strs: Vec<String> = parts
        .iter()
        .filter_map(|v| v.as_u64().map(|n| n.to_string()))
        .collect();

    if strs.is_empty() {
        None
    } else {
        Some(strs.join("-"))
    }
}

/// Percent-encode a DOI for use in a URL path segment.
/// Preserves `/` since DOIs contain slashes that are part of the path structure.
fn urlenc_doi(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
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
    fn test_urlenc_doi_preserves_slash() {
        assert_eq!(urlenc_doi("10.1234/test.foo"), "10.1234/test.foo");
        assert_eq!(urlenc_doi("10.1000/s12345-678"), "10.1000/s12345-678");
    }

    #[test]
    fn test_urlenc_doi_encodes_special_chars() {
        // Parentheses in DOIs should be encoded
        assert_eq!(urlenc_doi("10.1000/foo(bar)"), "10.1000/foo%28bar%29");
    }

    #[test]
    fn test_urlenc_query() {
        assert_eq!(urlenc("hello world"), "hello%20world");
        assert_eq!(urlenc("foo@bar.com"), "foo%40bar.com");
    }

    #[test]
    fn test_parse_message_full() {
        let msg = serde_json::json!({
            "title": ["Deep Learning for NLP"],
            "author": [
                {"given": "Jane", "family": "Doe"},
                {"given": "John", "family": "Smith"}
            ],
            "abstract": "We study methods.",
            "published-print": {"date-parts": [[2023, 4, 15]]},
            "container-title": ["Nature"],
            "volume": "42",
            "page": "100-110",
            "URL": "https://doi.org/10.1234/test"
        });
        let paper = parse_message(&msg, "10.1234/test");
        assert_eq!(paper.title, "Deep Learning for NLP");
        assert_eq!(paper.authors.len(), 2);
        assert_eq!(paper.authors[0].name, "Jane Doe");
        assert_eq!(paper.authors[1].name, "John Smith");
        assert_eq!(paper.doi.as_deref(), Some("10.1234/test"));
        assert_eq!(paper.abstract_text.as_deref(), Some("We study methods."));
        assert_eq!(paper.date.as_deref(), Some("2023-4-15"));
        assert_eq!(paper.publication.as_deref(), Some("Nature"));
        assert_eq!(paper.volume.as_deref(), Some("42"));
        assert_eq!(paper.pages.as_deref(), Some("100-110"));
        assert_eq!(paper.url.as_deref(), Some("https://doi.org/10.1234/test"));
        assert_eq!(paper.source.as_deref(), Some("crossref"));
    }

    #[test]
    fn test_parse_message_minimal() {
        let msg = serde_json::json!({});
        let paper = parse_message(&msg, "10.0000/minimal");
        assert_eq!(paper.title, "(untitled)");
        assert!(paper.authors.is_empty());
        assert_eq!(paper.doi.as_deref(), Some("10.0000/minimal"));
        assert!(paper.date.is_none());
    }

    #[test]
    fn test_parse_message_online_date_fallback() {
        let msg = serde_json::json!({
            "title": ["Online Only"],
            "published-online": {"date-parts": [[2024, 1]]}
        });
        let paper = parse_message(&msg, "10.0000/online");
        assert_eq!(paper.date.as_deref(), Some("2024-1"));
    }

    #[test]
    fn test_extract_date_year_only() {
        let msg = serde_json::json!({
            "published-print": {"date-parts": [[2020]]}
        });
        assert_eq!(extract_date(&msg, "published-print").as_deref(), Some("2020"));
    }
}
