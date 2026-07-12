use serde_json::Value;

/// fatcat / Internet Archive Scholar full-text PDF resolver.
///
/// fatcat is the open bibliographic catalog behind scholar.archive.org. Its
/// killer feature for us is *dead-link recovery*: when a publisher or
/// repository copy has rotted, fatcat often still points at a preserved
/// snapshot in the Wayback Machine (`web.archive.org`). That makes it the
/// last-resort green-OA source in the cascade.
///
/// We do a release lookup by DOI with `expand=files`, then pick a PDF URL
/// from `files[].urls[]`, preferring `application/pdf` mimetype and
/// preferring a live/web URL over a Wayback snapshot (but a Wayback URL is
/// perfectly acceptable — that is the whole point of this source).
///
/// No API key is required.
pub struct Fatcat;

impl Fatcat {
    pub fn new() -> Self {
        Self
    }

    pub fn find_pdf(&self, doi: &str) -> Result<Option<String>, String> {
        // fatcat lowercases DOIs in its index; the lookup is case-insensitive
        // but we normalize to be safe.
        let url = format!(
            "https://api.fatcat.wiki/v0/release/lookup?doi={}&expand=files",
            urlenc(&doi.to_lowercase()),
        );

        let resp: Value = match ureq::get(&url).set("Accept", "application/json").call() {
            Ok(r) => r
                .into_json()
                .map_err(|e| format!("fatcat JSON parse failed: {e}"))?,
            // 404 = no release with that DOI; 400 = malformed DOI. Both mean
            // "nothing here" and must not abort the fetch.
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(ureq::Error::Status(400, _)) => return Ok(None),
            Err(e) => return Err(format!("fatcat request failed: {e}")),
        };

        Ok(select_pdf(&resp))
    }
}

/// Choose the best PDF URL from a fatcat release with expanded files.
///
/// A fatcat `file` entity looks like:
/// ```json
/// {
///   "mimetype": "application/pdf",
///   "sha1": "...",
///   "urls": [
///     { "url": "https://repo.example.org/x.pdf", "rel": "repository" },
///     { "url": "https://web.archive.org/web/2020.../...pdf", "rel": "webarchive" }
///   ]
/// }
/// ```
/// We rank PDF-mimetyped files first, then within URLs prefer a live web/
/// repository/publisher URL, falling back to a Wayback snapshot.
fn select_pdf(release: &Value) -> Option<String> {
    let files = release["files"].as_array()?;

    let mut pdf_live: Option<String> = None;
    let mut pdf_archive: Option<String> = None;
    let mut any_live: Option<String> = None;
    let mut any_archive: Option<String> = None;

    for file in files {
        let is_pdf = file["mimetype"].as_str() == Some("application/pdf");
        let urls = match file["urls"].as_array() {
            Some(u) => u,
            None => continue,
        };
        for entry in urls {
            let url = match entry["url"].as_str() {
                Some(u) if !u.is_empty() => u,
                _ => continue,
            };
            let archived = is_wayback(url);
            if is_pdf {
                if archived {
                    pdf_archive.get_or_insert_with(|| url.to_string());
                } else {
                    pdf_live.get_or_insert_with(|| url.to_string());
                }
            } else if looks_like_pdf_url(url) {
                if archived {
                    any_archive.get_or_insert_with(|| url.to_string());
                } else {
                    any_live.get_or_insert_with(|| url.to_string());
                }
            }
        }
    }

    let pick = pdf_live.or(pdf_archive).or(any_live).or(any_archive);
    if let Some(ref url) = pick {
        if is_wayback(url) {
            eprintln!("resolved PDF via fatcat (Wayback snapshot): {url}");
        } else {
            eprintln!("resolved PDF via fatcat: {url}");
        }
    }
    pick
}

fn is_wayback(url: &str) -> bool {
    url.contains("web.archive.org/") || url.contains("://archive.org/")
}

/// Heuristic for a non-mimetyped URL that still looks like a direct PDF.
fn looks_like_pdf_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    lower.ends_with(".pdf") || lower.contains(".pdf?") || lower.contains("/pdf")
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
    fn test_is_wayback() {
        assert!(is_wayback("https://web.archive.org/web/2020/http://x.org/a.pdf"));
        assert!(!is_wayback("https://repo.example.org/a.pdf"));
    }

    #[test]
    fn test_prefers_pdf_mimetype_live_over_archive() {
        let release = serde_json::json!({
            "files": [{
                "mimetype": "application/pdf",
                "urls": [
                    {"url": "https://web.archive.org/web/2020/http://x.org/a.pdf", "rel": "webarchive"},
                    {"url": "https://repo.example.org/a.pdf", "rel": "repository"}
                ]
            }]
        });
        assert_eq!(select_pdf(&release).as_deref(), Some("https://repo.example.org/a.pdf"));
    }

    #[test]
    fn test_accepts_wayback_only() {
        let release = serde_json::json!({
            "files": [{
                "mimetype": "application/pdf",
                "urls": [
                    {"url": "https://web.archive.org/web/2020/http://x.org/a.pdf", "rel": "webarchive"}
                ]
            }]
        });
        assert_eq!(
            select_pdf(&release).as_deref(),
            Some("https://web.archive.org/web/2020/http://x.org/a.pdf")
        );
    }

    #[test]
    fn test_pdf_mimetype_beats_pdf_looking_url() {
        let release = serde_json::json!({
            "files": [
                {
                    "mimetype": "text/html",
                    "urls": [{"url": "https://x.org/view/pdf", "rel": "web"}]
                },
                {
                    "mimetype": "application/pdf",
                    "urls": [{"url": "https://web.archive.org/web/2020/x.pdf", "rel": "webarchive"}]
                }
            ]
        });
        // PDF-mimetyped (even archived) outranks a guessed pdf-looking URL.
        assert_eq!(
            select_pdf(&release).as_deref(),
            Some("https://web.archive.org/web/2020/x.pdf")
        );
    }

    #[test]
    fn test_no_files_none() {
        let release = serde_json::json!({ "files": [] });
        assert_eq!(select_pdf(&release), None);
        let release2 = serde_json::json!({ "ident": "abc" });
        assert_eq!(select_pdf(&release2), None);
    }

    #[test]
    fn test_urlenc() {
        assert_eq!(urlenc("10.1371/journal.pone.0173664"), "10.1371%2Fjournal.pone.0173664");
    }
}
