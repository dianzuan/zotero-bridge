use serde_json::Value;

/// CORE (core.ac.uk) full-text PDF resolver — KEY-GATED.
///
/// CORE aggregates open-access papers from thousands of repositories and
/// hosts its own copy of each PDF, exposed as a `downloadUrl`. This is an
/// excellent green-OA source (author/institutional repository deposits),
/// complementing Unpaywall.
///
/// The CORE API v3 requires a free API key, read from Zotero settings
/// (`source.core.apiKey`) and sent as `Authorization: Bearer {key}`. When
/// the key is absent the resolver returns `Ok(None)` immediately — it is
/// silently skipped, never an error.
///
/// API shape (CORE API v3, https://api.core.ac.uk/v3/):
///   GET /v3/search/works?q=doi:"{DOI}"&limit=5   (Authorization: Bearer KEY)
///   -> { "totalHits": N, "results": [ { "downloadUrl": "https://core.ac.uk/download/....pdf", ... } ] }
/// `downloadUrl` is a direct, CORE-hosted PDF link.
pub struct Core {
    pub api_key: Option<String>,
}

impl Core {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key: api_key.filter(|k| !k.trim().is_empty()),
        }
    }

    pub fn find_pdf(&self, doi: &str) -> Result<Option<String>, String> {
        // No key -> skip gracefully. This is the common case for users who
        // have not signed up for a CORE key, and must never surface as an
        // error in the cascade.
        let key = match self.api_key.as_deref() {
            Some(k) => k,
            None => return Ok(None),
        };

        // Exact DOI field query. The `q` value is fully percent-encoded.
        let query = format!("doi:\"{}\"", doi);
        let url = format!(
            "https://api.core.ac.uk/v3/search/works?q={}&limit=5",
            urlenc(&query),
        );

        let resp: Value = match ureq::get(&url)
            .set("Authorization", &format!("Bearer {key}"))
            .set("Accept", "application/json")
            .call()
        {
            Ok(r) => r
                .into_json()
                .map_err(|e| format!("CORE JSON parse failed: {e}"))?,
            // 404 = nothing found. 401/403 = bad/expired key — treat as a
            // soft skip with a warning rather than aborting the whole fetch.
            // 429 = rate limited; also non-fatal.
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => {
                eprintln!("warning: CORE rejected the API key (401/403); skipping");
                return Ok(None);
            }
            Err(ureq::Error::Status(429, _)) => {
                eprintln!("warning: CORE rate-limited (429); skipping");
                return Ok(None);
            }
            Err(e) => return Err(format!("CORE request failed: {e}")),
        };

        Ok(select_pdf(&resp))
    }
}

/// Extract a direct PDF `downloadUrl` from a CORE search response.
///
/// Walks `results[]` and returns the first non-empty `downloadUrl`.
fn select_pdf(resp: &Value) -> Option<String> {
    let results = resp["results"].as_array()?;
    for work in results {
        if let Some(url) = work["downloadUrl"].as_str() {
            if !url.is_empty() {
                eprintln!("resolved PDF via CORE: {url}");
                return Some(url.to_string());
            }
        }
    }
    None
}

/// Percent-encode a string for use in a URL query parameter value.
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
    fn test_no_key_skips_without_error() {
        // Construct explicitly with no key — find_pdf must return Ok(None)
        // before doing any network work.
        let core = Core { api_key: None };
        assert_eq!(core.find_pdf("10.1234/test"), Ok(None));
    }

    #[test]
    fn test_blank_key_treated_as_absent() {
        // new() filters whitespace-only keys.
        let core = Core::new(Some("   ".to_string()));
        assert!(core.api_key.is_none());
    }

    #[test]
    fn test_empty_key_treated_as_absent() {
        let core = Core::new(Some(String::new()));
        assert!(core.api_key.is_none());
    }

    #[test]
    fn test_present_key_kept() {
        let core = Core::new(Some("core-secret".to_string()));
        assert_eq!(core.api_key.as_deref(), Some("core-secret"));
    }

    #[test]
    fn test_select_pdf_picks_download_url() {
        let resp = serde_json::json!({
            "totalHits": 1,
            "results": [
                { "doi": "10.1/x", "downloadUrl": "https://core.ac.uk/download/123.pdf" }
            ]
        });
        assert_eq!(
            select_pdf(&resp).as_deref(),
            Some("https://core.ac.uk/download/123.pdf")
        );
    }

    #[test]
    fn test_select_pdf_skips_empty_download_url() {
        let resp = serde_json::json!({
            "results": [
                { "downloadUrl": "" },
                { "downloadUrl": "https://core.ac.uk/download/9.pdf" }
            ]
        });
        assert_eq!(
            select_pdf(&resp).as_deref(),
            Some("https://core.ac.uk/download/9.pdf")
        );
    }

    #[test]
    fn test_select_pdf_no_results_none() {
        let resp = serde_json::json!({ "totalHits": 0, "results": [] });
        assert_eq!(select_pdf(&resp), None);
    }

    #[test]
    fn test_urlenc_quotes() {
        assert_eq!(urlenc("doi:\"10.1/x\""), "doi%3A%2210.1%2Fx%22");
    }
}
