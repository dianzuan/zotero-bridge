use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::sources::crossref::CrossRef;
use crate::sources::unpaywall::Unpaywall;
use crate::sources::arxiv::ArXiv;
use crate::types::Paper;

pub fn fetch_doi(doi: &str) -> Result<(Paper, Option<PathBuf>), String> {
    let crossref = CrossRef::new();
    let unpaywall = Unpaywall::new();

    let paper = crossref.fetch_doi(doi)?;

    let pdf_url = unpaywall.find_pdf(doi)?
        .or_else(|| paper.pdf_url.clone());

    let pdf_path = pdf_url
        .as_ref()
        .and_then(|url| download_pdf(url, doi).ok());

    let mut paper = paper;
    if pdf_url.is_some() {
        paper.pdf_url = pdf_url;
    }

    Ok((paper, pdf_path))
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

    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("pdf download failed: {e}"))?;

    let mut file = fs::File::create(&path)
        .map_err(|e| format!("create pdf file: {e}"))?;

    let mut reader = resp.into_reader();
    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut reader, &mut buf)
            .map_err(|e| format!("read pdf: {e}"))?;
        if n == 0 { break; }
        file.write_all(&buf[..n]).map_err(|e| format!("write pdf: {e}"))?;
    }

    Ok(path)
}
