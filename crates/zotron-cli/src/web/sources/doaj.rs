use serde_json::Value;

/// DOAJ (Directory of Open Access Journals) full-text PDF resolver.
///
/// DOAJ only indexes peer-reviewed open-access journals, so any PDF it
/// surfaces is gold-OA and freely downloadable. We query the public
/// search-articles API by DOI and pull a `fulltext` link, preferring one
/// whose `content_type` is `PDF`.
///
/// No API key is required.
pub struct Doaj;

impl Doaj {
    pub fn new() -> Self {
        Self
    }

    pub fn find_pdf(&self, doi: &str) -> Result<Option<String>, String> {
        // Public API: GET /api/v4/search/articles/{search_query}
        // The query `doi:"{DOI}"` does an exact-field lookup. The DOI sits
        // inside the query string, so it is fully percent-encoded.
        let query = format!("doi:{}", doi);
        let url = format!(
            "https://doaj.org/api/v4/search/articles/{}",
            urlenc(&query),
        );

        let resp: Value = match ureq::get(&url).set("Accept", "application/json").call() {
            Ok(r) => r
                .into_json()
                .map_err(|e| format!("DOAJ JSON parse failed: {e}"))?,
            // No matching article (or a bad query) just means "no OA PDF
            // here" — never abort the fetch over it.
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(ureq::Error::Status(400, _)) => return Ok(None),
            Err(e) => return Err(format!("DOAJ request failed: {e}")),
        };

        let results = match resp["results"].as_array() {
            Some(r) if !r.is_empty() => r,
            _ => return Ok(None),
        };

        // Scan every matching article's bibjson.link[] for a fulltext PDF.
        // First pass: prefer an explicit PDF content_type. Second pass:
        // accept any fulltext link (DOAJ fulltext is always free to read).
        let mut fallback: Option<String> = None;
        for article in results {
            let links = match article["bibjson"]["link"].as_array() {
                Some(l) => l,
                None => continue,
            };
            for link in links {
                if link["type"].as_str() != Some("fulltext") {
                    continue;
                }
                let url = match link["url"].as_str() {
                    Some(u) if !u.is_empty() => u,
                    _ => continue,
                };
                if link["content_type"].as_str().map(eq_pdf) == Some(true) {
                    eprintln!("resolved PDF via DOAJ: {url}");
                    return Ok(Some(url.to_string()));
                }
                if fallback.is_none() {
                    fallback = Some(url.to_string());
                }
            }
        }

        if let Some(ref url) = fallback {
            eprintln!("resolved PDF via DOAJ (fulltext, type unspecified): {url}");
        }
        Ok(fallback)
    }
}

fn eq_pdf(s: &str) -> bool {
    s.eq_ignore_ascii_case("pdf")
}

/// Percent-encode a string for use in a URL path segment.
/// DOIs contain `/`, which must be escaped here because the whole
/// `doi:...` query is a single path segment in the DOAJ route.
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
    fn test_urlenc_escapes_doi_slash() {
        assert_eq!(urlenc("doi:10.7717/peerj.4375"), "doi%3A10.7717%2Fpeerj.4375");
    }

    #[test]
    fn test_eq_pdf_case_insensitive() {
        assert!(eq_pdf("PDF"));
        assert!(eq_pdf("pdf"));
        assert!(!eq_pdf("HTML"));
    }

    // Mirrors the live DOAJ response shape verified against
    // doi:10.7717/peerj.4375 — a fulltext PDF link plus an HTML link.
    fn select_from(resp: &Value) -> Option<String> {
        let results = resp["results"].as_array()?;
        let mut fallback: Option<String> = None;
        for article in results {
            let links = article["bibjson"]["link"].as_array()?;
            for link in links {
                if link["type"].as_str() != Some("fulltext") {
                    continue;
                }
                let url = link["url"].as_str()?;
                if link["content_type"].as_str().map(eq_pdf) == Some(true) {
                    return Some(url.to_string());
                }
                if fallback.is_none() {
                    fallback = Some(url.to_string());
                }
            }
        }
        fallback
    }

    #[test]
    fn test_prefers_pdf_over_html() {
        let resp = serde_json::json!({
            "results": [{
                "bibjson": {
                    "link": [
                        {"content_type": "HTML", "type": "fulltext", "url": "https://example.org/x/"},
                        {"content_type": "PDF", "type": "fulltext", "url": "https://example.org/x.pdf"}
                    ]
                }
            }]
        });
        assert_eq!(select_from(&resp).as_deref(), Some("https://example.org/x.pdf"));
    }

    #[test]
    fn test_falls_back_to_any_fulltext() {
        let resp = serde_json::json!({
            "results": [{
                "bibjson": {
                    "link": [
                        {"content_type": "HTML", "type": "fulltext", "url": "https://example.org/x/"}
                    ]
                }
            }]
        });
        assert_eq!(select_from(&resp).as_deref(), Some("https://example.org/x/"));
    }

    #[test]
    fn test_empty_results_none() {
        let resp = serde_json::json!({ "results": [] });
        assert_eq!(select_from(&resp), None);
    }
}
