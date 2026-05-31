use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::sources::arxiv::ArXiv;
use crate::sources::core::Core;
use crate::sources::crossref::CrossRef;
use crate::sources::doaj::Doaj;
use crate::sources::fatcat::Fatcat;
use crate::sources::unpaywall::Unpaywall;
use crate::types::Paper;

pub fn fetch_doi(doi: &str) -> Result<(Paper, Option<PathBuf>), String> {
    let crossref = CrossRef::new();

    let paper = crossref.fetch_doi(doi)?;

    // Open-access PDF cascade. Each resolver is best-effort: a lookup failure
    // (network, 5xx, bad key, etc.) must NOT block importing the CrossRef
    // metadata we already have, so every source is run through `try_source`,
    // which downgrades errors to warnings and yields `None`.
    //
    // Order (first downloadable PDF wins):
    //   Unpaywall -> DOAJ -> CORE -> fatcat -> OpenAlex/S2 (paper.pdf_url)
    //
    // We don't just take the first URL — we take the first URL that actually
    // DOWNLOADS, so a 404/dead link in one source falls through to the next.
    let unpaywall = Unpaywall::new();
    let doaj = Doaj::new();
    let core = Core::new();
    let fatcat = Fatcat::new();

    let candidates: [(&str, Option<String>); 5] = [
        ("Unpaywall", try_source("Unpaywall", || unpaywall.find_pdf(doi))),
        ("DOAJ", try_source("DOAJ", || doaj.find_pdf(doi))),
        ("CORE", try_source("CORE", || core.find_pdf(doi))),
        ("fatcat", try_source("fatcat", || fatcat.find_pdf(doi))),
        ("OpenAlex/S2", paper.pdf_url.clone()),
    ];

    let mut resolved_url: Option<String> = None;
    let mut pdf_path: Option<PathBuf> = None;
    for (source, url) in candidates.into_iter() {
        let url = match url {
            Some(u) => u,
            None => continue,
        };
        match download_pdf(&url, doi) {
            Ok(path) => {
                eprintln!("downloaded PDF via {source}: {url}");
                resolved_url = Some(url);
                pdf_path = Some(path);
                break;
            }
            Err(e) => {
                eprintln!("warning: {source} PDF download failed ({url}): {e}");
            }
        }
    }

    let mut paper = paper;
    if let Some(url) = resolved_url {
        paper.pdf_url = Some(url);
    }

    Ok((paper, pdf_path))
}

/// Run one OA-PDF resolver, downgrading any error to a warning and `None`.
/// This keeps a single source's failure (network, bad key, 5xx) from
/// aborting the whole fetch.
fn try_source<F>(name: &str, f: F) -> Option<String>
where
    F: FnOnce() -> Result<Option<String>, String>,
{
    match f() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("warning: {name} PDF lookup failed: {e}");
            None
        }
    }
}

pub fn fetch_arxiv(arxiv_id: &str) -> Result<(Paper, Option<PathBuf>), String> {
    let arxiv = ArXiv::new();
    let paper = arxiv.fetch_id(arxiv_id)?;

    let pdf_path = paper
        .pdf_url
        .as_ref()
        .and_then(|url| download_pdf(url, arxiv_id).ok());

    Ok((paper, pdf_path))
}

fn download_pdf(url: &str, id: &str) -> Result<PathBuf, String> {
    let dir = std::env::temp_dir().join("zotron-scholar");
    fs::create_dir_all(&dir).map_err(|e| format!("create temp dir: {e}"))?;

    let safe_name: String = id
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '.' { c } else { '_' })
        .collect();
    let path = dir.join(format!("{safe_name}.pdf"));

    // Many publishers (PeerJ, Wiley, etc.) 403 a request with no User-Agent.
    // Send a browser-like UA so direct-PDF links actually download.
    let resp = ureq::get(url)
        .set(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 \
             (KHTML, like Gecko) Chrome/124.0 Safari/537.36 zotron-scholar",
        )
        .set("Accept", "application/pdf,*/*")
        .call()
        .map_err(|e| format!("pdf download failed: {e}"))?;

    // Some "fulltext" links (notably DOAJ HTML links and publisher landing
    // pages) return an HTML document, not a PDF. If we saved that as a .pdf
    // the import would attach garbage. Read the first bytes and require the
    // PDF magic (`%PDF`) before committing — otherwise error so the cascade
    // moves on to the next source. A Content-Type of text/html is an early-
    // out for the same reason.
    if let Some(ct) = resp.header("Content-Type") {
        let ct = ct.to_ascii_lowercase();
        if ct.contains("text/html") || ct.contains("application/xhtml") {
            return Err(format!("not a PDF (Content-Type: {ct})"));
        }
    }

    let mut reader = resp.into_reader();

    // Peek the first bytes to confirm the PDF signature.
    let mut head = [0u8; 8];
    let mut head_len = 0usize;
    while head_len < head.len() {
        let n = std::io::Read::read(&mut reader, &mut head[head_len..])
            .map_err(|e| format!("read pdf: {e}"))?;
        if n == 0 {
            break;
        }
        head_len += n;
    }
    if !head[..head_len].starts_with(b"%PDF") {
        return Err("not a PDF (missing %PDF signature)".to_string());
    }

    let mut file = fs::File::create(&path)
        .map_err(|e| format!("create pdf file: {e}"))?;
    file.write_all(&head[..head_len])
        .map_err(|e| format!("write pdf: {e}"))?;

    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut reader, &mut buf)
            .map_err(|e| format!("read pdf: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| format!("write pdf: {e}"))?;
    }

    Ok(path)
}
