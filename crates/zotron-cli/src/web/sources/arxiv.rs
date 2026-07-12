use crate::web::sources::Source;
use crate::web::types::{Author, Paper};

pub struct ArXiv;

impl ArXiv {
    pub fn new() -> Self {
        Self
    }

    pub fn fetch_id(&self, id: &str) -> Result<Paper, String> {
        let id = id
            .strip_prefix("arxiv:")
            .or_else(|| id.strip_prefix("arXiv:"))
            .unwrap_or(id);

        let url = format!(
            "https://export.arxiv.org/api/query?id_list={}&max_results=1",
            urlenc(id),
        );

        let body = ureq::get(&url)
            .call()
            .map_err(|e| format!("arXiv request failed: {e}"))?
            .into_string()
            .map_err(|e| format!("arXiv response read failed: {e}"))?;

        let entries = split_entries(&body);
        if entries.is_empty() {
            return Err(format!("arXiv: no entry found for id '{id}'"));
        }

        parse_single_entry(&entries[0])
    }
}

impl Source for ArXiv {
    fn name(&self) -> &str {
        "arxiv"
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<Paper>, String> {
        let capped = limit.min(50);
        let url = format!(
            "https://export.arxiv.org/api/query?search_query=all:{}&max_results={}",
            urlenc(query),
            capped,
        );

        let body = ureq::get(&url)
            .call()
            .map_err(|e| format!("arXiv request failed: {e}"))?
            .into_string()
            .map_err(|e| format!("arXiv response read failed: {e}"))?;

        let entries = split_entries(&body);
        let mut papers = Vec::new();
        for entry in &entries {
            match parse_single_entry(entry) {
                Ok(p) => papers.push(p),
                Err(_) => continue, // skip malformed entries
            }
        }
        Ok(papers)
    }
}

/// Split the Atom XML body into individual `<entry>...</entry>` blocks.
fn split_entries(xml: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("<entry>") {
        if let Some(end) = rest[start..].find("</entry>") {
            let block = &rest[start..start + end + "</entry>".len()];
            entries.push(block.to_string());
            rest = &rest[start + end + "</entry>".len()..];
        } else {
            break;
        }
    }
    entries
}

/// Extract text content of a simple XML tag. Handles tags with attributes like `<link href="..."/>`.
fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);

    let start_pos = xml.find(&open)?;
    let after_open = &xml[start_pos + open.len()..];

    // Find the closing `>` of the opening tag
    let gt = after_open.find('>')?;
    let content_start = start_pos + open.len() + gt + 1;

    let close_pos = xml[content_start..].find(&close)?;
    let content = &xml[content_start..content_start + close_pos];

    let trimmed = content.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

/// Extract all author names from an entry. Each author is in `<author><name>...</name></author>`.
fn extract_authors(entry: &str) -> Vec<Author> {
    let mut authors = Vec::new();
    let mut rest = entry;
    while let Some(start) = rest.find("<author>") {
        if let Some(end) = rest[start..].find("</author>") {
            let block = &rest[start..start + end + "</author>".len()];
            if let Some(name) = extract_tag(block, "name") {
                authors.push(Author { name });
            }
            rest = &rest[start + end + "</author>".len()..];
        } else {
            break;
        }
    }
    authors
}

/// Extract the arXiv ID from the `<id>` tag. The id is a URL like
/// `http://arxiv.org/abs/2301.12345v1` — we extract `2301.12345v1`.
fn extract_arxiv_id(entry: &str) -> Option<String> {
    let id_url = extract_tag(entry, "id")?;
    // Strip version suffix for cleaner ID, but keep it if present
    if let Some(pos) = id_url.rfind("/abs/") {
        Some(id_url[pos + 5..].to_string())
    } else if let Some(pos) = id_url.rfind('/') {
        Some(id_url[pos + 1..].to_string())
    } else {
        Some(id_url)
    }
}

/// Parse a single `<entry>` XML block into a Paper.
fn parse_single_entry(entry: &str) -> Result<Paper, String> {
    let title = extract_tag(entry, "title")
        .ok_or("arXiv entry missing <title>")?
        // arXiv titles can have internal newlines; collapse them
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let authors = extract_authors(entry);

    let abstract_text = extract_tag(entry, "summary").map(|s| {
        s.replace('\n', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    });

    let date = extract_tag(entry, "published").map(|d| {
        // "2023-01-15T12:00:00Z" → "2023-01-15"
        d.split('T').next().unwrap_or(&d).to_string()
    });

    let arxiv_id = extract_arxiv_id(entry);

    let pdf_url = arxiv_id
        .as_ref()
        .map(|id| format!("https://arxiv.org/pdf/{}", id));

    let url = arxiv_id
        .as_ref()
        .map(|id| format!("https://arxiv.org/abs/{}", id));

    Ok(Paper {
        title,
        authors,
        doi: None,
        abstract_text,
        date,
        publication: None,
        volume: None,
        pages: None,
        url,
        pdf_url,
        arxiv_id,
        source: Some("arxiv".to_string()),
        cited_by_count: None,
    })
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
    }

    #[test]
    fn test_extract_tag_simple() {
        let xml = r#"<entry><title>Test Title</title><summary>Abstract here</summary></entry>"#;
        assert_eq!(extract_tag(xml, "title"), Some("Test Title".to_string()));
        assert_eq!(
            extract_tag(xml, "summary"),
            Some("Abstract here".to_string())
        );
    }

    #[test]
    fn test_extract_tag_with_attributes() {
        let xml = r#"<title type="html">Some Title</title>"#;
        assert_eq!(extract_tag(xml, "title"), Some("Some Title".to_string()));
    }

    #[test]
    fn test_extract_tag_missing() {
        let xml = "<entry><title>Foo</title></entry>";
        assert_eq!(extract_tag(xml, "summary"), None);
    }

    #[test]
    fn test_extract_authors() {
        let entry = r#"
            <entry>
                <author><name>Alice Smith</name></author>
                <author><name>Bob Jones</name></author>
            </entry>
        "#;
        let authors = extract_authors(entry);
        assert_eq!(authors.len(), 2);
        assert_eq!(authors[0].name, "Alice Smith");
        assert_eq!(authors[1].name, "Bob Jones");
    }

    #[test]
    fn test_extract_arxiv_id() {
        let entry = "<entry><id>http://arxiv.org/abs/2301.12345v1</id></entry>";
        assert_eq!(
            extract_arxiv_id(entry),
            Some("2301.12345v1".to_string())
        );
    }

    #[test]
    fn test_split_entries() {
        let xml = r#"
            <feed>
                <entry><title>A</title></entry>
                <entry><title>B</title></entry>
            </feed>
        "#;
        let entries = split_entries(xml);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].contains("A"));
        assert!(entries[1].contains("B"));
    }

    #[test]
    fn test_parse_single_entry() {
        let entry = r#"
            <entry>
                <id>http://arxiv.org/abs/2301.12345v1</id>
                <title>Attention Is All You Need</title>
                <summary>We propose a new architecture.</summary>
                <published>2023-01-15T12:00:00Z</published>
                <author><name>Ashish Vaswani</name></author>
                <author><name>Noam Shazeer</name></author>
            </entry>
        "#;
        let paper = parse_single_entry(entry).unwrap();
        assert_eq!(paper.title, "Attention Is All You Need");
        assert_eq!(paper.authors.len(), 2);
        assert_eq!(paper.authors[0].name, "Ashish Vaswani");
        assert_eq!(
            paper.abstract_text.as_deref(),
            Some("We propose a new architecture.")
        );
        assert_eq!(paper.date.as_deref(), Some("2023-01-15"));
        assert_eq!(paper.arxiv_id.as_deref(), Some("2301.12345v1"));
        assert_eq!(
            paper.pdf_url.as_deref(),
            Some("https://arxiv.org/pdf/2301.12345v1")
        );
        assert_eq!(
            paper.url.as_deref(),
            Some("https://arxiv.org/abs/2301.12345v1")
        );
        assert_eq!(paper.source.as_deref(), Some("arxiv"));
    }

    #[test]
    fn test_parse_single_entry_missing_title() {
        let entry = "<entry><summary>No title</summary></entry>";
        let result = parse_single_entry(entry);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_single_entry_multiline_title() {
        let entry = r#"
            <entry>
                <id>http://arxiv.org/abs/0000.00000v1</id>
                <title>
                    Multi
                    Line Title
                </title>
                <summary>Abstract</summary>
                <published>2024-06-01T00:00:00Z</published>
            </entry>
        "#;
        let paper = parse_single_entry(entry).unwrap();
        assert_eq!(paper.title, "Multi Line Title");
    }
}
