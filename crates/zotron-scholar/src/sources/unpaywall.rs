pub struct Unpaywall {
    pub email: Option<String>,
}

impl Unpaywall {
    pub fn new() -> Self {
        Self {
            email: std::env::var("ZOTRON_SCHOLAR_EMAIL").ok(),
        }
    }

    pub fn find_pdf(&self, _doi: &str) -> Result<Option<String>, String> {
        Err("unpaywall find_pdf not yet implemented".into())
    }
}

// Unpaywall is not a search source — it only resolves DOIs to PDF URLs.
// It does not implement Source.
