mod manifest;
mod sources;
mod types;

use clap::Parser;
use sources::Source;

#[derive(Parser)]
#[command(name = "zotron-scholar", version, about = env!("CARGO_PKG_DESCRIPTION"))]
enum Cli {
    /// Print JSON manifest (name, version, capabilities)
    Manifest,

    /// Search academic papers
    Search {
        /// Search query
        query: String,

        /// Maximum results to return
        #[arg(short, long, default_value_t = 10)]
        limit: usize,

        /// Source to search (openalex, s2, crossref, arxiv)
        #[arg(short, long, default_value = "openalex")]
        source: String,
    },

    /// Fetch paper by identifier
    Fetch {
        /// DOI to fetch
        #[arg(long)]
        doi: Option<String>,

        /// arXiv ID to fetch
        #[arg(long)]
        arxiv: Option<String>,
    },
}

fn run() -> Result<String, String> {
    let cli = Cli::parse();

    match cli {
        Cli::Manifest => {
            manifest::print_manifest();
            Ok(String::new())
        }

        Cli::Search {
            query,
            limit,
            source,
        } => {
            let src: Box<dyn Source> = match source.as_str() {
                "openalex" => Box::new(sources::OpenAlex::new()),
                "crossref" => Box::new(sources::CrossRef::new()),
                "s2" => Box::new(sources::SemanticScholar::new()),
                "arxiv" => Box::new(sources::ArXiv::new()),
                other => return Err(format!("unknown source: {other}")),
            };

            let papers = src.search(&query, limit)?;
            serde_json::to_string_pretty(&papers).map_err(|e| e.to_string())
        }

        Cli::Fetch { doi, arxiv } => {
            if doi.is_none() && arxiv.is_none() {
                return Err("provide --doi or --arxiv".into());
            }
            Err("fetch not yet implemented".into())
        }
    }
}

fn main() {
    match run() {
        Ok(output) => {
            if !output.is_empty() {
                print!("{output}");
            }
        }
        Err(message) => {
            eprintln!(
                "{}",
                serde_json::json!({"error": message})
            );
            std::process::exit(1);
        }
    }
}
