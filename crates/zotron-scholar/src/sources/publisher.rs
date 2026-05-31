use crate::http::get_with_retry;

/// Resolve a DOI to the publisher's full-text PDF via the `citation_pdf_url`
/// HTML meta tag on the article landing page.
///
/// This is publisher-agnostic — most scholarly publishers (Springer, Elsevier,
/// Wiley, Taylor & Francis, SAGE, OUP, …) emit
/// `<meta name="citation_pdf_url" content="…">` for Google Scholar indexing.
/// It rides the caller's OWN access: from a subscribed campus/VPN IP the linked
/// PDF downloads; otherwise the publisher returns a paywall page or 403 and the
/// cascade simply moves on. This is NOT a paywall bypass — it only ever follows
/// the publisher's own advertised PDF link, using whatever access the caller's
/// network already has.
pub struct PublisherDirect;

impl PublisherDirect {
    pub fn new() -> Self {
        Self
    }

    pub fn find_pdf(&self, doi: &str) -> Result<Option<String>, String> {
        let landing = format!("https://doi.org/{doi}");
        let html = get_with_retry(&landing)?
            .into_string()
            .map_err(|e| format!("landing page read failed: {e}"))?;
        Ok(extract_citation_pdf_url(&html))
    }
}

/// Return the `content` of the first `<meta name="citation_pdf_url" …>` tag.
fn extract_citation_pdf_url(html: &str) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    let mut from = 0usize;
    while let Some(rel) = lower[from..].find("<meta") {
        let start = from + rel;
        let end = lower[start..]
            .find('>')
            .map_or(html.len(), |e| start + e + 1);
        let tag_lower = &lower[start..end];
        if tag_lower.contains("citation_pdf_url") {
            if let Some(url) = extract_attr(&html[start..end], tag_lower, "content") {
                let url = url.trim();
                if !url.is_empty() {
                    return Some(url.to_string());
                }
            }
        }
        from = end;
    }
    None
}

/// Extract a quoted attribute value (`attr="…"` or `attr='…'`) from a tag.
fn extract_attr(tag: &str, tag_lower: &str, attr: &str) -> Option<String> {
    let key = format!("{attr}=");
    let pos = tag_lower.find(&key)? + key.len();
    let rest = tag.get(pos..)?;
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let inner = &rest[quote.len_utf8()..];
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_when_name_then_content_double_quotes() {
        let html = r#"<head><meta name="citation_pdf_url" content="https://x.com/a.pdf"></head>"#;
        assert_eq!(
            extract_citation_pdf_url(html).as_deref(),
            Some("https://x.com/a.pdf")
        );
    }

    #[test]
    fn extracts_when_content_before_name_single_quotes() {
        let html = r#"<meta content='https://y.org/b.pdf' name='citation_pdf_url'/>"#;
        assert_eq!(
            extract_citation_pdf_url(html).as_deref(),
            Some("https://y.org/b.pdf")
        );
    }

    #[test]
    fn none_when_no_citation_pdf_url() {
        let html = r#"<meta name="citation_title" content="A Paper"><meta name="dc.format" content="application/pdf">"#;
        assert!(extract_citation_pdf_url(html).is_none());
    }
}
