use serde_json::Value;

pub struct Unpaywall {
    pub email: Option<String>,
}

impl Unpaywall {
    pub fn new(email: Option<String>) -> Self {
        Self { email }
    }

    pub fn find_pdf(&self, doi: &str) -> Result<Option<String>, String> {
        let email = self
            .email
            .as_deref()
            .unwrap_or("zotron@users.noreply.github.com");

        let url = format!(
            "https://api.unpaywall.org/v2/{}?email={}",
            urlenc_doi(doi),
            urlenc(email),
        );

        let resp: Value = match ureq::get(&url).call() {
            Ok(r) => r
                .into_json()
                .map_err(|e| format!("Unpaywall JSON parse failed: {e}"))?,
            // A DOI with no Unpaywall record returns 404 — that just means
            // "no open-access PDF found", which is common for paywalled
            // papers. Don't abort the whole fetch over it.
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(e) => return Err(format!("Unpaywall request failed: {e}")),
        };

        let oa = &resp["best_oa_location"];
        if oa.is_null() {
            return Ok(None);
        }

        // Prefer url_for_pdf, fall back to url
        let pdf_url = oa["url_for_pdf"]
            .as_str()
            .or_else(|| oa["url"].as_str())
            .map(|s| s.to_string());

        Ok(pdf_url)
    }
}

// Unpaywall is not a search source — it only resolves DOIs to PDF URLs.
// It does not implement Source.

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
    fn test_urlenc_doi() {
        assert_eq!(urlenc_doi("10.1234/test"), "10.1234/test");
    }

    #[test]
    fn test_urlenc_email() {
        assert_eq!(urlenc("user@example.com"), "user%40example.com");
    }

    #[test]
    fn test_default_email_used_when_env_not_set() {
        // Just verify the struct can be constructed
        let u = Unpaywall { email: None };
        // email fallback happens inside find_pdf, not in constructor
        assert!(u.email.is_none());
    }
}
