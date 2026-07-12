use serde_json::Value;

use crate::web::sources::Source;
use crate::web::types::{Author, Paper};

pub struct OpenAlex {
    pub email: Option<String>,
}

impl OpenAlex {
    pub fn new(email: Option<String>) -> Self {
        Self { email }
    }
}

impl Source for OpenAlex {
    fn name(&self) -> &str {
        "openalex"
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<Paper>, String> {
        let capped = limit.min(50);
        let mut url = format!(
            "https://api.openalex.org/works?search={}&per_page={}",
            urlenc(query),
            capped,
        );
        if let Some(ref email) = self.email {
            url.push_str(&format!("&mailto={}", urlenc(email)));
        }

        let resp: Value = ureq::get(&url)
            .call()
            .map_err(|e| format!("OpenAlex request failed: {e}"))?
            .into_json()
            .map_err(|e| format!("OpenAlex JSON parse failed: {e}"))?;

        let results = resp["results"]
            .as_array()
            .ok_or("OpenAlex response missing 'results' array")?;

        let papers: Vec<Paper> = results.iter().map(parse_work).collect();
        Ok(papers)
    }
}

fn parse_work(work: &Value) -> Paper {
    let title = work["title"]
        .as_str()
        .unwrap_or("(untitled)")
        .to_string();

    let authors: Vec<Author> = work["authorships"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| {
                    a["author"]["display_name"]
                        .as_str()
                        .map(|name| Author { name: name.to_string() })
                })
                .collect()
        })
        .unwrap_or_default();

    let doi = work["doi"]
        .as_str()
        .map(|d| d.strip_prefix("https://doi.org/").unwrap_or(d).to_string());

    let abstract_text = work["abstract_inverted_index"]
        .as_object()
        .map(reconstruct_abstract);

    let date = work["publication_date"].as_str().map(|s| s.to_string());

    let publication = work["primary_location"]["source"]["display_name"]
        .as_str()
        .map(|s| s.to_string());

    let volume = work["biblio"]["volume"].as_str().map(|s| s.to_string());

    let pages = work["biblio"]["first_page"].as_str().map(|s| s.to_string());

    let url = work["primary_location"]["landing_page_url"]
        .as_str()
        .map(|s| s.to_string());

    let pdf_url = work["open_access"]["oa_url"]
        .as_str()
        .or_else(|| work["best_oa_location"]["pdf_url"].as_str())
        .map(|s| s.to_string());

    Paper {
        title,
        authors,
        doi,
        abstract_text,
        date,
        publication,
        volume,
        pages,
        url,
        pdf_url,
        arxiv_id: None,
        source: Some("openalex".to_string()),
        cited_by_count: work["cited_by_count"].as_u64(),
    }
}

/// Reconstruct abstract from OpenAlex inverted index format.
///
/// The inverted index maps each word to an array of positions where it appears.
/// We invert this back to a flat word list sorted by position, then join with spaces.
fn reconstruct_abstract(index: &serde_json::Map<String, Value>) -> String {
    let mut words: Vec<(u64, &str)> = Vec::new();
    for (word, positions) in index {
        if let Some(arr) = positions.as_array() {
            for pos in arr {
                if let Some(p) = pos.as_u64() {
                    words.push((p, word.as_str()));
                }
            }
        }
    }
    words.sort_by_key(|(pos, _)| *pos);
    words.iter().map(|(_, w)| *w).collect::<Vec<_>>().join(" ")
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
        assert_eq!(urlenc("machine+learning"), "machine%2Blearning");
        assert_eq!(urlenc("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn test_reconstruct_abstract() {
        let json: Value = serde_json::json!({
            "We": [0],
            "study": [1],
            "deep": [2],
            "learning": [3],
            "methods.": [4]
        });
        let index = json.as_object().unwrap();
        let result = reconstruct_abstract(index);
        assert_eq!(result, "We study deep learning methods.");
    }

    #[test]
    fn test_reconstruct_abstract_repeated_words() {
        let json: Value = serde_json::json!({
            "the": [0, 3],
            "cat": [1],
            "and": [2],
            "dog": [4]
        });
        let index = json.as_object().unwrap();
        let result = reconstruct_abstract(index);
        assert_eq!(result, "the cat and the dog");
    }

    #[test]
    fn test_parse_work_minimal() {
        let work = serde_json::json!({
            "title": "Test Paper",
            "authorships": [],
            "doi": null,
        });
        let paper = parse_work(&work);
        assert_eq!(paper.title, "Test Paper");
        assert!(paper.authors.is_empty());
        assert!(paper.doi.is_none());
        assert_eq!(paper.source.as_deref(), Some("openalex"));
    }

    #[test]
    fn test_parse_work_strips_doi_prefix() {
        let work = serde_json::json!({
            "title": "Test",
            "doi": "https://doi.org/10.1234/test",
            "authorships": [{
                "author": { "display_name": "Jane Doe" }
            }],
        });
        let paper = parse_work(&work);
        assert_eq!(paper.doi.as_deref(), Some("10.1234/test"));
        assert_eq!(paper.authors.len(), 1);
        assert_eq!(paper.authors[0].name, "Jane Doe");
    }
}
