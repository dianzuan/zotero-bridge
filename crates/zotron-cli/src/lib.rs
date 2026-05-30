//! Minimal typed CLI surface for the Rust migration scaffold.

use clap::{error::ErrorKind, Parser, Subcommand};
use serde_json::Value;
use zotron_rpc::ZoteroRpc;
use zotron_types::{builtin_ocr_provider_specs, DEFAULT_RPC_URL};

mod commands;
mod ocr;
mod output;
mod rag;
mod rpc;

use crate::commands::*;
use crate::ocr::*;
use crate::output::*;
pub use crate::output::{classify_error, format_error_json};
use crate::rag::*;
pub use crate::rag::{fetch_rerank_settings, RerankSettings};
use crate::rpc::*;
pub use crate::rpc::RpcCaller;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CliOcrProviderSpec {
    pub id: &'static str,
    pub provider: &'static str,
    pub request_style: &'static str,
    pub auth: &'static str,
    pub auth_header: &'static str,
    pub supports_pdf_direct: bool,
    pub key_field: &'static str,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct CliEmbeddingProviderSpec {
    pub id: &'static str,
    pub provider: &'static str,
    pub request_style: &'static str,
    pub default_url: String,
    pub default_model: &'static str,
    pub auth: &'static str,
    pub key_field: &'static str,
}

pub fn ocr_provider_specs() -> Vec<CliOcrProviderSpec> {
    builtin_ocr_provider_specs()
        .into_iter()
        .map(cli_ocr_provider_spec)
        .collect()
}

pub fn ocr_provider_spec(provider: &str) -> Result<CliOcrProviderSpec, String> {
    zotron_types::ocr_provider_spec(provider).map(cli_ocr_provider_spec)
}

pub fn embedding_provider_spec(provider: &str) -> Result<CliEmbeddingProviderSpec, String> {
    let spec = zotron_types::embedding_provider_spec(provider)?;
    Ok(CliEmbeddingProviderSpec {
        id: spec.id,
        provider: spec.provider_key,
        request_style: if spec.provider_key == "alibaba" {
            "dashscope"
        } else {
            spec.request_style.as_str()
        },
        default_url: spec.default_url.unwrap_or("").to_string(),
        default_model: spec.default_model,
        auth: spec.auth,
        key_field: spec.key_field,
    })
}

pub fn chunks_from_blocks(blocks: &[Value], max_chars: usize) -> Result<Vec<Value>, String> {
    let typed = blocks
        .iter()
        .map(json_block_to_pdf_block)
        .collect::<Result<Vec<_>, _>>()?;
    let chunks = zotron_types::chunks_from_blocks(&typed, max_chars);
    chunks
        .into_iter()
        .map(|chunk| chunk_to_cli_value(&chunk, &typed))
        .collect()
}

fn cli_ocr_provider_spec(spec: zotron_types::OcrProviderSpec) -> CliOcrProviderSpec {
    CliOcrProviderSpec {
        id: spec.provider_key,
        provider: spec.provider_key,
        request_style: spec.request_style.as_str(),
        auth: spec.auth,
        auth_header: spec.auth_header,
        supports_pdf_direct: spec.supports_pdf_direct,
        key_field: spec.key_field,
    }
}

fn json_block_to_pdf_block(value: &Value) -> Result<zotron_types::PdfEvidenceBlock, String> {
    let block_key = value
        .get("block_key")
        .and_then(Value::as_str)
        .ok_or_else(|| "block missing block_key".to_string())?
        .to_string();
    let item_key = value
        .get("item_key")
        .and_then(Value::as_str)
        .ok_or_else(|| "block missing item_key".to_string())?
        .to_string();
    let attachment_key = value
        .get("attachment_key")
        .and_then(Value::as_str)
        .ok_or_else(|| "block missing attachment_key".to_string())?
        .to_string();
    let page_idx = value
        .get("page_idx")
        .or_else(|| value.get("page"))
        .and_then(Value::as_u64)
        .unwrap_or(1);
    let block_type = value
        .get("type")
        .or_else(|| value.get("block_type"))
        .and_then(Value::as_str)
        .unwrap_or("paragraph")
        .to_string();
    let section_path = value
        .get("section_path")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let bbox = value.get("bbox").and_then(value_bbox4);

    Ok(zotron_types::PdfEvidenceBlock {
        block_key,
        item_key,
        attachment_key,
        page_idx,
        block_type,
        bbox,
        section_path,
        text,
    })
}

fn chunk_to_cli_value(
    chunk: &zotron_types::StructureChunk,
    blocks: &[zotron_types::PdfEvidenceBlock],
) -> Result<Value, String> {
    let refs = chunk
        .block_keys
        .iter()
        .filter_map(|key| blocks.iter().find(|block| &block.block_key == key))
        .map(|block| {
            serde_json::json!({
                "block_key": block.block_key,
                "page_idx": block.page_idx,
                "bbox": block.bbox.map(|bbox| bbox.iter().map(|n| {
                    if n.fract() == 0.0 {
                        Value::from(*n as i64)
                    } else {
                        Value::from(*n)
                    }
                }).collect::<Vec<_>>()),
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "chunk_key": chunk.chunk_key,
        "item_key": chunk.item_key,
        "attachment_key": chunk.attachment_key,
        "block_keys": chunk.block_keys,
        "section_path": chunk.section_path,
        "text": chunk.text,
        "page_start": chunk.page_start,
        "page_end": chunk.page_end,
        "evidence_refs": refs,
    }))
}

fn value_bbox4(value: &Value) -> Option<[f64; 4]> {
    let arr = value.as_array()?;
    if arr.len() != 4 {
        return None;
    }
    Some([
        arr[0].as_f64()?,
        arr[1].as_f64()?,
        arr[2].as_f64()?,
        arr[3].as_f64()?,
    ])
}

#[derive(Debug, Parser)]
#[command(name = "zotron", about = "Rust client + CLI for the Zotron XPI")]
struct Cli {
    /// Zotero JSON-RPC endpoint. Applies to every subcommand.
    #[arg(long, default_value = DEFAULT_RPC_URL, global = true)]
    url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum OcrCommand {
    /// Print supported OCR provider contracts.
    Providers,
    /// Execute an OCR provider request from JSON and emit normalized blocks.
    #[command(name = "run")]
    Run {
        #[arg(long)]
        provider: String,
        /// Path to an OcrRequestInput JSON file, or "-" to read stdin.
        #[arg(long)]
        input: Option<String>,
        /// Local PDF/image file to encode into an OcrRequestInput.
        #[arg(long)]
        file: Option<String>,
        /// Zotero item key used when --file builds the OCR request.
        #[arg(long = "item-key")]
        item_key: Option<String>,
        /// Zotero attachment key used when --file builds the OCR request.
        #[arg(long = "attachment-key")]
        attachment_key: Option<String>,
        /// MIME type used when --file builds the OCR request.
        #[arg(long = "mime-type")]
        mime_type: Option<String>,
        /// Override the provider endpoint, required for service-hosted PaddleOCR-VL.
        #[arg(long)]
        endpoint: Option<String>,
        /// Environment variable containing the provider bearer token.
        #[arg(long = "api-key-env")]
        api_key_env: Option<String>,
    },
    /// Show OCR statistics for a collection.
    Status {
        #[arg(long)]
        collection: String,
    },
    /// Re-chunk and re-embed existing OCR results without re-running OCR.
    Reindex {
        #[arg(long)]
        collection: Option<String>,
        #[arg(long)]
        key: Option<String>,
        #[arg(long, help = "Only reindex items with stale schema version")]
        stale_only: bool,
        #[arg(long = "chunk-chars", default_value_t = 1200)]
        chunk_chars: usize,
    },
    /// Parse a Zotero PDF and write hidden sidecar OCR/RAG artifacts. Provider read from Zotero settings unless --provider is given.
    #[command(name = "process")]
    Process {
        /// Override OCR provider (default: read from Zotero settings ocr.provider).
        #[arg(long)]
        provider: Option<String>,
        /// Parent Zotero item key. Required unless --collection is given.
        #[arg(long, conflicts_with = "collection")]
        parent: Option<String>,
        /// Collection name (fuzzy) or key: OCR every item in the collection.
        #[arg(long)]
        collection: Option<String>,
        /// Zotero PDF attachment key (auto-resolved from --parent when omitted; ignored with --collection).
        #[arg(long, conflicts_with = "collection")]
        attachment: Option<String>,
        /// Public URL for MinerU cloud parsing. Use --result-dir/--result-zip for offline ingestion.
        #[arg(long = "source-url")]
        source_url: Option<String>,
        /// Already-extracted MinerU result directory, used by tests/offline replay.
        #[arg(long = "result-dir")]
        result_dir: Option<String>,
        /// Already-downloaded MinerU result zip, used by tests/offline replay.
        #[arg(long = "result-zip")]
        result_zip: Option<String>,
        /// Override provider endpoint (default: read from Zotero settings ocr.apiUrl).
        #[arg(long = "provider-endpoint")]
        provider_endpoint: Option<String>,
        /// Environment variable containing the provider bearer token (fallback: Zotero settings ocr.apiKey).
        #[arg(long = "api-key-env")]
        api_key_env: Option<String>,
        #[arg(long = "poll-interval-seconds", default_value_t = 5)]
        poll_interval_seconds: u64,
        #[arg(long = "timeout-seconds", default_value_t = 900)]
        timeout_seconds: u64,
        #[arg(long = "chunk-chars", default_value_t = 1200)]
        chunk_chars: usize,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Check that Zotero is running with the Zotron XPI enabled.
    Ping,
    /// Generic RPC escape hatch.
    Rpc {
        method: String,
        #[arg(default_value = "{}")]
        params_json: String,
        #[arg(long)]
        paginate: bool,
        #[arg(long, default_value_t = 100)]
        page_size: usize,
    },
    /// Push prepared Zotero JSON (from file or stdin) to Zotero.
    Push {
        /// Path to a JSON file, or "-" to read from stdin.
        json_file: String,
        /// Optional PDF attachment path.
        #[arg(long)]
        pdf: Option<String>,
        /// Collection name (fuzzy) or key.
        #[arg(long)]
        collection: Option<String>,
        /// Duplicate handling: skip | update | create.
        #[arg(long = "on-duplicate", default_value = "skip")]
        on_duplicate: String,
        /// Parse input + resolve collection only; do not push to Zotero.
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
    /// System and plugin introspection commands.
    System {
        #[command(subcommand)]
        command: SystemCommand,
    },
    /// Search items by text, tag, identifier, or structured conditions.
    Search(SearchArgs),
    /// Inspect and manage Zotero items.
    Items {
        #[command(subcommand)]
        command: ItemsCommand,
    },
    /// Inspect Zotero collections.
    Collections {
        #[command(subcommand)]
        command: CollectionsCommand,
    },
    /// Inspect Zotero notes.
    Notes {
        #[command(subcommand)]
        command: NotesCommand,
    },
    /// Inspect Zotero preferences.
    Settings {
        #[command(subcommand)]
        command: SettingsCommand,
    },
    /// Inspect and manage Zotero tags.
    Tags {
        #[command(subcommand)]
        command: TagsCommand,
    },
    /// Export items as BibTeX, RIS, CSL-JSON, or formatted bibliography.
    Export(ExportArgs),
    /// List, create, and delete PDF annotations.
    Annotations {
        #[command(subcommand)]
        command: AnnotationsCommand,
    },
    /// OCR PDFs and manage raw/block/chunk evidence artifacts.
    Ocr {
        #[command(subcommand)]
        command: OcrCommand,
    },
    /// Build and search retrieval artifacts.
    Rag {
        #[command(subcommand)]
        command: RagCommand,
    },
}

pub(crate) struct RagSearchOptions {
    pub(crate) query: String,
    pub(crate) collection: Option<String>,
    pub(crate) keys: Vec<String>,
    pub(crate) zotero: bool,
    pub(crate) top_spans_per_item: u64,
    pub(crate) include_fulltext_spans: bool,
    pub(crate) top_k: u64,
    pub(crate) output: String,
}

#[derive(Debug, Subcommand)]
pub(crate) enum RagCommand {
    /// Print supported embedding provider contracts.
    #[command(name = "providers")]
    Providers,
    /// Execute an embedding provider request from JSON and emit vectors.
    #[command(name = "embed")]
    Embed {
        #[arg(long)]
        provider: String,
        /// Path to an EmbeddingRequestInput JSON file, or "-" to read stdin.
        #[arg(long)]
        input: String,
        /// Override the embedding endpoint.
        #[arg(long)]
        endpoint: Option<String>,
        /// Override the embedding model.
        #[arg(long)]
        model: Option<String>,
        /// Override provider input type, for example document or query.
        #[arg(long = "input-type")]
        input_type: Option<String>,
        /// Environment variable containing the provider bearer token.
        #[arg(long = "api-key-env")]
        api_key_env: Option<String>,
    },
    /// Show index status for a collection.
    Status {
        #[arg(long)]
        collection: String,
    },
    /// Emit academic-zh retrieval hits with item_key/title/text provenance.
    #[command(name = "search")]
    Search {
        query: String,
        #[arg(long)]
        collection: Option<String>,
        /// Limit retrieval to one or more Zotero item keys.
        #[arg(long = "key", alias = "keys")]
        keys: Vec<String>,
        #[arg(long)]
        zotero: bool,
        #[arg(long = "top-spans-per-item", default_value_t = 3)]
        top_spans_per_item: u64,
        #[arg(long = "include-fulltext-spans")]
        include_fulltext_spans: bool,
        #[arg(long = "limit", alias = "top-k", default_value_t = 50)]
        top_k: u64,
        #[arg(long, default_value = "json", value_parser = ["json", "jsonl"])]
        output: String,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SystemCommand {
    /// Show XPI version and exposed method metadata.
    Version,
    /// List all libraries (user + groups).
    Libraries,
    /// Get statistics for the current (or specified) library.
    #[command(name = "library-stats")]
    LibraryStats {
        #[arg(long)]
        library: Option<i64>,
    },
    /// Show item type schema. Without --type, lists all types. With --type, shows fields and creator types.
    Schema {
        #[arg(long = "type")]
        item_type: Option<String>,
    },
    /// Get the currently selected Zotero collection (or null).
    #[command(name = "current-collection")]
    CurrentCollection,
    /// List RPC methods, or describe a specific method.
    Methods {
        /// Method name to describe. Omit to list all methods.
        method: Option<String>,
    },
}

#[derive(Debug, clap::Args)]
pub(crate) struct SearchArgs {
    /// Search query (title/creator/year by default; PDF content with --fulltext).
    pub(crate) query: Option<String>,
    /// Search inside PDF full-text content instead of metadata.
    #[arg(long)]
    pub(crate) fulltext: bool,
    /// Filter by author/creator name (contains match).
    #[arg(long)]
    pub(crate) author: Option<String>,
    /// Filter by date after (YYYY or YYYY-MM-DD).
    #[arg(long)]
    pub(crate) after: Option<String>,
    /// Filter by date before (YYYY or YYYY-MM-DD).
    #[arg(long)]
    pub(crate) before: Option<String>,
    /// Filter by journal/publication title (contains match).
    #[arg(long)]
    pub(crate) journal: Option<String>,
    /// Filter by tag (exact match).
    #[arg(long)]
    pub(crate) tag: Option<String>,
    /// Find by DOI.
    #[arg(long)]
    pub(crate) doi: Option<String>,
    /// Find by ISBN.
    #[arg(long)]
    pub(crate) isbn: Option<String>,
    /// Find by ISSN.
    #[arg(long)]
    pub(crate) issn: Option<String>,
    /// Limit results to a collection name or key.
    #[arg(long)]
    pub(crate) collection: Option<String>,
    #[arg(long, default_value_t = 50)]
    pub(crate) limit: u64,
    #[arg(long, default_value_t = 0)]
    pub(crate) offset: u64,
    #[command(subcommand)]
    pub(crate) management: Option<SearchManagementCommand>,
}

#[derive(Debug, Subcommand)]
pub(crate) enum SearchManagementCommand {
    /// List all saved searches in the library.
    #[command(name = "saved-searches")]
    SavedSearches,
    /// Create a saved search with one or more conditions.
    #[command(name = "create-saved")]
    CreateSaved {
        name: String,
        #[arg(long = "condition", required = true)]
        condition: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a saved search by key.
    #[command(name = "delete-saved")]
    DeleteSaved {
        search_key: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum ItemsCommand {
    /// Add an item by DOI, ISBN, URL, local file, or manual entry (--type + --field).
    Add {
        #[arg(long)]
        doi: Option<String>,
        #[arg(long)]
        isbn: Option<String>,
        /// Web page URL to add from.
        #[arg(long = "from-url")]
        from_url: Option<String>,
        /// Local file path to add from.
        #[arg(long)]
        file: Option<String>,
        /// Item type for manual creation (e.g. journalArticle).
        #[arg(long = "type")]
        item_type: Option<String>,
        /// Field values for manual creation (e.g. title="My Paper").
        #[arg(long = "field")]
        fields: Vec<String>,
        #[arg(long)]
        collection: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Update fields on an existing item.
    Update {
        key: String,
        #[arg(long = "field")]
        fields: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Permanently delete an item.
    Delete {
        key: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Move one or more items to trash.
    Trash {
        items: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Restore a trashed item.
    Restore {
        item: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Merge a group of duplicate items.
    #[command(name = "merge-duplicates")]
    MergeDuplicates {
        keys: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a related-item link between two items.
    #[command(name = "add-related")]
    AddRelated {
        key: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove a related-item link between two items.
    #[command(name = "remove-related")]
    RemoveRelated {
        key: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Print the full serialization of an item by key.
    Get {
        item: String,
    },
    /// List items in the library with optional sorting and pagination.
    List {
        #[arg(long, default_value_t = 50)]
        limit: u64,
        #[arg(long, default_value_t = 0)]
        offset: u64,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long, default_value = "asc")]
        direction: String,
        /// List trashed items instead of regular items.
        #[arg(long)]
        trash: bool,
    },
    /// Run Zotero's duplicate scan and print groups.
    #[command(name = "find-duplicates")]
    FindDuplicates,
    /// List recently added or modified items.
    Recent {
        #[arg(long, default_value_t = 20)]
        limit: u64,
        #[arg(long, default_value_t = 0)]
        offset: u64,
        #[arg(long = "type", default_value = "added")]
        recent_type: String,
    },
    /// Retrieve the full-text content of an item's attachment.
    Fulltext {
        key: String,
    },
    /// List items related to the given item.
    Related {
        key: String,
    },
    /// Get the citation key for an item.
    #[command(name = "citation-key")]
    CitationKey {
        key: String,
    },
    /// Get the local filesystem path of an item's PDF attachment.
    Path {
        key: String,
    },
    /// List attachments belonging to an item.
    Attachments {
        key: String,
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
    /// Batch find missing PDFs in a collection via Zotero's resolver chain.
    #[command(name = "find-pdfs")]
    FindPdfs {
        #[arg(long)]
        collection: String,
        #[arg(long, default_value_t = 0)]
        limit: usize,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum SettingsCommand {
    /// Get a single Zotero preference value.
    Get {
        key: String,
    },
    /// List all Zotero preferences as a key->value dict.
    #[command(visible_alias = "get-all")]
    List,
    /// Set one or more Zotero preferences (key value pairs), or bulk-set from a JSON file.
    Set {
        /// key value key value ... (pairs of positional args)
        pairs: Vec<String>,
        /// Bulk-set from a JSON file.
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum TagsCommand {
    /// List all tags in the library (flat).
    List {
        #[arg(long, default_value_t = 200)]
        limit: u64,
    },
    /// Rename a tag across all items.
    Rename {
        old: String,
        new: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a tag library-wide.
    Delete {
        tag: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Add tags to one or more items.
    Add {
        keys: Vec<String>,
        #[arg(long = "tag", required = true)]
        tags: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove tags from one or more items.
    Remove {
        keys: Vec<String>,
        #[arg(long = "tag", required = true)]
        tags: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, clap::Args)]
pub(crate) struct ExportArgs {
    /// Item keys to export.
    pub(crate) keys: Vec<String>,
    /// Output format: bibtex, ris, csl-json, bibliography.
    #[arg(long, default_value = "bibtex")]
    pub(crate) format: String,
    /// Export all items from this collection (name or key).
    #[arg(long)]
    pub(crate) collection: Option<String>,
    /// Citation style URL (only for bibliography format).
    #[arg(long, default_value = "http://www.zotero.org/styles/apa")]
    pub(crate) style: String,
    /// Output HTML instead of plain text (only for bibliography format).
    #[arg(long)]
    pub(crate) html: bool,
}

#[derive(Debug, Subcommand)]
pub(crate) enum AnnotationsCommand {
    /// List annotations on a PDF. Accepts an item key (auto-resolves to PDF) or attachment key.
    List {
        /// Item key or attachment key
        parent: String,
        /// Use a specific attachment when the item has multiple PDFs
        #[arg(long)]
        attachment: Option<String>,
        /// Include N characters of surrounding text for each annotation
        #[arg(long)]
        context: Option<u32>,
    },
    /// Create a new annotation on a PDF. Accepts an item key (auto-resolves to PDF) or attachment key.
    Create {
        /// Item key or attachment key
        parent: String,
        /// Use a specific attachment when the item has multiple PDFs
        #[arg(long)]
        attachment: Option<String>,
        #[arg(long = "type")]
        annotation_type: Option<String>,
        /// JSON annotation position, for example '{"pageIndex":0,"rects":[[10,20,30,40]]}'.
        /// Not required when --quote is given.
        #[arg(long)]
        position: Option<String>,
        /// Text to locate in the PDF and highlight. Resolves to rects automatically.
        /// Locates text headlessly (no PDF viewer required).
        #[arg(long)]
        quote: Option<String>,
        /// Restrict quote search to a specific page (0-indexed).
        #[arg(long)]
        page: Option<u32>,
        /// Zotero annotation sort index.
        #[arg(long = "sort-index")]
        sort_index: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        comment: Option<String>,
        #[arg(long, default_value = "#ffd400")]
        color: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Batch-create annotations from a JSON array on stdin or --file.
    /// Each entry: {"quote": "...", "color": "#hex", "comment": "...", "type": "highlight"}
    CreateBatch {
        /// Item key or attachment key
        parent: String,
        /// Use a specific attachment when the item has multiple PDFs
        #[arg(long)]
        attachment: Option<String>,
        /// Read annotations from a JSON file instead of stdin
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Locate a text quote in a PDF without creating an annotation.
    /// Returns page index and rects if found.
    Locate {
        /// Item key or attachment key
        parent: String,
        /// Use a specific attachment when the item has multiple PDFs
        #[arg(long)]
        attachment: Option<String>,
        /// Text to locate in the PDF
        #[arg(long)]
        quote: String,
        /// Restrict search to a specific page (0-indexed)
        #[arg(long)]
        page: Option<u32>,
    },
    /// Delete an annotation by key.
    Delete {
        annotation_key: String,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum NotesCommand {
    /// List notes attached to a parent item.
    List {
        #[arg(long)]
        parent: String,
        #[arg(long, default_value_t = 50)]
        limit: u64,
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
    /// Get a single note by key.
    Get {
        note_key: String,
    },
    /// Create a note attached to a parent item.
    Create {
        #[arg(long)]
        parent: String,
        #[arg(long)]
        content: String,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Update the content of an existing note.
    Update {
        note_key: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a note by key.
    Delete {
        note_key: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Search notes by text content.
    Search {
        query: String,
        #[arg(long, default_value_t = 50)]
        limit: u64,
    },
}

#[derive(Debug, Subcommand)]
pub(crate) enum CollectionsCommand {
    /// List all collections in the user library (flat).
    List,
    /// Print the collection hierarchy as a tree.
    Tree,
    /// Get a single collection's metadata.
    Get {
        name_or_id: String,
    },
    /// List all items in a collection.
    #[command(name = "get-items", visible_alias = "items")]
    GetItems {
        name_or_id: String,
        #[arg(long)]
        limit: Option<u64>,
        #[arg(long, default_value_t = 0)]
        offset: u64,
    },
    /// Show item/attachment/note/subcollection counts for a collection.
    Stats {
        name_or_id: String,
    },
    /// Rename a collection.
    Rename {
        old_name: String,
        new_name: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a collection, optionally nested under a parent.
    Create {
        name: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a collection.
    Delete {
        name_or_id: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Add existing items to a collection.
    #[command(name = "add-items")]
    AddItems {
        collection: String,
        item_keys: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove items from a collection.
    #[command(name = "remove-items")]
    RemoveItems {
        collection: String,
        item_keys: Vec<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

enum ParseOutcome<T> {
    Command(T),
    Display(String),
}

fn parse_cli<T>(
    args: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
) -> Result<ParseOutcome<T>, String>
where
    T: Parser,
{
    match T::try_parse_from(args) {
        Ok(cli) => Ok(ParseOutcome::Command(cli)),
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) => {
            Ok(ParseOutcome::Display(err.to_string()))
        }
        Err(err) => Err(err.to_string()),
    }
}

pub fn run(
    args: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
) -> Result<String, String> {
    // Emit plugin hint for Claude Code discovery
    if std::env::var("CLAUDECODE").as_deref() == Ok("1") {
        eprintln!(r#"<claude-code-hint v="1" type="plugin" value="zotron@dianzuan/zotron" />"#);
    }

    let cli = match parse_cli::<Cli>(args)? {
        ParseOutcome::Command(cli) => cli,
        ParseOutcome::Display(output) => return Ok(output),
    };
    let mut client = ZoteroRpc::new(cli.url);
    run_command(cli.command, &mut client)
}

pub fn run_with_client(
    args: impl IntoIterator<Item = impl Into<std::ffi::OsString> + Clone>,
    client: &mut impl RpcCaller,
) -> Result<String, String> {
    let cli = match parse_cli::<Cli>(args)? {
        ParseOutcome::Command(cli) => cli,
        ParseOutcome::Display(output) => return Ok(output),
    };
    run_command(cli.command, client)
}

fn run_command(command: Command, client: &mut impl RpcCaller) -> Result<String, String> {
    if let Command::Export(args) = command {
        return run_export(args, client);
    }

    let value = match command {
        Command::Ping => call_json(client, "system.ping", None)?,
        Command::Rpc {
            method,
            params_json,
            paginate,
            page_size,
            ..
        } => {
            let params = serde_json::from_str::<Value>(&params_json)
                .map_err(|err| format!("INVALID_JSON: params must be a JSON object: {err}"))?;
            if !params.is_object() {
                return Err("INVALID_JSON: params must be a JSON object".to_string());
            }
            if paginate {
                paginate_rpc(client, &method, params, page_size)?
            } else {
                call_json(client, &method, Some(params))?
            }
        }
        Command::Push {
            json_file,
            pdf,
            collection,
            on_duplicate,
            dry_run,
            ..
        } => return run_push_command(json_file, pdf, collection, on_duplicate, dry_run, client),
        Command::System { command } => run_system_command(command, client)?,
        Command::Search(args) => {
            if let Some(mgmt) = args.management {
                run_search_management_command(mgmt, client)?
            } else {
                run_search(args, client)?
            }
        }
        Command::Items { command } => run_items_command(command, client)?,
        Command::Collections { command } => run_collections_command(command, client)?,
        Command::Notes { command } => run_notes_command(command, client)?,
        Command::Settings { command } => run_settings_command(command, client)?,
        Command::Tags { command } => run_tags_command(command, client)?,
        Command::Annotations { command } => run_annotations_command(command, client)?,
        Command::Ocr { command } => {
            return run_ocr_command(command, client);
        }
        Command::Rag { command } => {
            return run_rag_command(command, client);
        }
        Command::Export(_) => unreachable!("export commands return raw output above"),
    };

    format_json(&value)
}
