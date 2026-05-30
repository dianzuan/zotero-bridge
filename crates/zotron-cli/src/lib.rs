//! Minimal typed CLI surface for the Rust migration scaffold.

use std::{
    env, fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use clap::{error::ErrorKind, Parser, Subcommand};
use serde_json::Value;
use zotron_rpc::{StdProviderCommandRunner, UreqProviderHttpTransport, ZoteroRpc};
use zotron_types::{
    build_embedding_provider_request, build_ocr_provider_request, builtin_ocr_provider_specs,
    is_zotron_evidence_artifact, machine_artifact_exists_for_item,
    machine_artifact_exists_in_sidecar, machine_artifact_store_root,
    ocr_provider_spec as raw_ocr_provider_spec, parse_embedding_provider_response,
    parse_ocr_provider_response, write_machine_artifact_sidecar, EmbeddingChunkInput,
    EmbeddingRequestInput, EmbeddingVector, MachineArtifactKind, OcrRequestInput,
    ProviderCommandRunner, ProviderHttpInvocation, ProviderHttpTransport, DEFAULT_RPC_URL,
};

mod output;
mod rag;
mod rpc;

use crate::output::*;
pub use crate::output::format_error_json;
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
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum OcrCommand {
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Parse a Zotero PDF and write hidden sidecar OCR/RAG artifacts. Provider read from Zotero settings unless --provider is given.
    #[command(name = "process")]
    Process {
        /// Override OCR provider (default: read from Zotero settings ocr.provider).
        #[arg(long)]
        provider: Option<String>,
        /// Parent Zotero item key.
        #[arg(long)]
        parent: String,
        /// Zotero PDF attachment key (auto-resolved from --parent when omitted).
        #[arg(long)]
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check that Zotero is running with the Zotron XPI enabled.
    Ping {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Generic RPC escape hatch.
    Rpc {
        method: String,
        #[arg(default_value = "{}")]
        params_json: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum SystemCommand {
    /// Show XPI version and exposed method metadata.
    Version {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List all libraries (user + groups).
    Libraries {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Get statistics for the current (or specified) library.
    #[command(name = "library-stats")]
    LibraryStats {
        #[arg(long)]
        library: Option<i64>,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Show item type schema. Without --type, lists all types. With --type, shows fields and creator types.
    Schema {
        #[arg(long = "type")]
        item_type: Option<String>,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Get the currently selected Zotero collection (or null).
    #[command(name = "current-collection")]
    CurrentCollection {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List RPC methods, or describe a specific method.
    Methods {
        /// Method name to describe. Omit to list all methods.
        method: Option<String>,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, clap::Args)]
struct SearchArgs {
    /// Search query (title/creator/year by default; PDF content with --fulltext).
    query: Option<String>,
    /// Search inside PDF full-text content instead of metadata.
    #[arg(long)]
    fulltext: bool,
    /// Filter by author/creator name (contains match).
    #[arg(long)]
    author: Option<String>,
    /// Filter by date after (YYYY or YYYY-MM-DD).
    #[arg(long)]
    after: Option<String>,
    /// Filter by date before (YYYY or YYYY-MM-DD).
    #[arg(long)]
    before: Option<String>,
    /// Filter by journal/publication title (contains match).
    #[arg(long)]
    journal: Option<String>,
    /// Filter by tag (exact match).
    #[arg(long)]
    tag: Option<String>,
    /// Find by DOI.
    #[arg(long)]
    doi: Option<String>,
    /// Find by ISBN.
    #[arg(long)]
    isbn: Option<String>,
    /// Find by ISSN.
    #[arg(long)]
    issn: Option<String>,
    /// Limit results to a collection name or key.
    #[arg(long)]
    collection: Option<String>,
    #[arg(long, default_value_t = 50)]
    limit: u64,
    #[arg(long, default_value_t = 0)]
    offset: u64,
    #[arg(long, default_value = DEFAULT_RPC_URL)]
    url: String,
    #[command(subcommand)]
    management: Option<SearchManagementCommand>,
}

#[derive(Debug, Subcommand)]
enum SearchManagementCommand {
    /// List all saved searches in the library.
    #[command(name = "saved-searches")]
    SavedSearches {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Create a saved search with one or more conditions.
    #[command(name = "create-saved")]
    CreateSaved {
        name: String,
        #[arg(long = "condition", required = true)]
        condition: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Delete a saved search by key.
    #[command(name = "delete-saved")]
    DeleteSaved {
        search_key: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum ItemsCommand {
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Update fields on an existing item.
    Update {
        key: String,
        #[arg(long = "field")]
        fields: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Permanently delete an item.
    Delete {
        key: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Move one or more items to trash.
    Trash {
        items: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Restore a trashed item.
    Restore {
        item: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Merge a group of duplicate items.
    #[command(name = "merge-duplicates")]
    MergeDuplicates {
        keys: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Add a related-item link between two items.
    #[command(name = "add-related")]
    AddRelated {
        key: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Remove a related-item link between two items.
    #[command(name = "remove-related")]
    RemoveRelated {
        key: String,
        #[arg(long)]
        target: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Print the full serialization of an item by key.
    Get {
        item: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Run Zotero's duplicate scan and print groups.
    #[command(name = "find-duplicates")]
    FindDuplicates {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List recently added or modified items.
    Recent {
        #[arg(long, default_value_t = 20)]
        limit: u64,
        #[arg(long, default_value_t = 0)]
        offset: u64,
        #[arg(long = "type", default_value = "added")]
        recent_type: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Retrieve the full-text content of an item's attachment.
    Fulltext {
        key: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List items related to the given item.
    Related {
        key: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Get the citation key for an item.
    #[command(name = "citation-key")]
    CitationKey {
        key: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Get the local filesystem path of an item's PDF attachment.
    Path {
        key: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List attachments belonging to an item.
    Attachments {
        key: String,
        #[arg(long, default_value_t = 0)]
        offset: u64,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Batch find missing PDFs in a collection via Zotero's resolver chain.
    #[command(name = "find-pdfs")]
    FindPdfs {
        #[arg(long)]
        collection: String,
        #[arg(long, default_value_t = 0)]
        limit: usize,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum SettingsCommand {
    /// Get a single Zotero preference value.
    Get {
        key: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List all Zotero preferences as a key->value dict.
    #[command(visible_alias = "get-all")]
    List {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Set one or more Zotero preferences (key value pairs), or bulk-set from a JSON file.
    Set {
        /// key value key value ... (pairs of positional args)
        pairs: Vec<String>,
        /// Bulk-set from a JSON file.
        #[arg(long)]
        file: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum TagsCommand {
    /// List all tags in the library (flat).
    List {
        #[arg(long, default_value_t = 200)]
        limit: u64,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Rename a tag across all items.
    Rename {
        old: String,
        new: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Delete a tag library-wide.
    Delete {
        tag: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Add tags to one or more items.
    Add {
        keys: Vec<String>,
        #[arg(long = "tag", required = true)]
        tags: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Remove tags from one or more items.
    Remove {
        keys: Vec<String>,
        #[arg(long = "tag", required = true)]
        tags: Vec<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, clap::Args)]
struct ExportArgs {
    /// Item keys to export.
    keys: Vec<String>,
    /// Output format: bibtex, ris, csl-json, bibliography.
    #[arg(long, default_value = "bibtex")]
    format: String,
    /// Export all items from this collection (name or key).
    #[arg(long)]
    collection: Option<String>,
    /// Citation style URL (only for bibliography format).
    #[arg(long, default_value = "http://www.zotero.org/styles/apa")]
    style: String,
    /// Output HTML instead of plain text (only for bibliography format).
    #[arg(long)]
    html: bool,
    #[arg(long, default_value = DEFAULT_RPC_URL)]
    url: String,
}

#[derive(Debug, Subcommand)]
enum AnnotationsCommand {
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Delete an annotation by key.
    Delete {
        annotation_key: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum NotesCommand {
    /// List notes attached to a parent item.
    List {
        #[arg(long)]
        parent: String,
        #[arg(long, default_value_t = 50)]
        limit: u64,
        #[arg(long, default_value_t = 0)]
        offset: u64,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Get a single note by key.
    Get {
        note_key: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Update the content of an existing note.
    Update {
        note_key: String,
        #[arg(long)]
        content: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Delete a note by key.
    Delete {
        note_key: String,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Search notes by text content.
    Search {
        query: String,
        #[arg(long, default_value_t = 50)]
        limit: u64,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
}

#[derive(Debug, Subcommand)]
enum CollectionsCommand {
    /// List all collections in the user library (flat).
    List {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Print the collection hierarchy as a tree.
    Tree {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Get a single collection's metadata.
    Get {
        name_or_id: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// List all items in a collection.
    #[command(name = "get-items", visible_alias = "items")]
    GetItems {
        name_or_id: String,
        #[arg(long)]
        limit: Option<u64>,
        #[arg(long, default_value_t = 0)]
        offset: u64,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Show item/attachment/note/subcollection counts for a collection.
    Stats {
        name_or_id: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
    },
    /// Rename a collection.
    Rename {
        old_name: String,
        new_name: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Create a collection, optionally nested under a parent.
    Create {
        name: String,
        #[arg(long)]
        parent: Option<String>,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a collection.
    Delete {
        name_or_id: String,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Add existing items to a collection.
    #[command(name = "add-items")]
    AddItems {
        collection: String,
        item_keys: Vec<String>,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove items from a collection.
    #[command(name = "remove-items")]
    RemoveItems {
        collection: String,
        item_keys: Vec<String>,
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        url: String,
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
    let url = command_url(&cli.command);
    let mut client = ZoteroRpc::new(url);
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

fn rag_command_url(command: &RagCommand) -> String {
    match command {
        RagCommand::Providers => DEFAULT_RPC_URL.to_string(),
        RagCommand::Embed { .. } => DEFAULT_RPC_URL.to_string(),
        RagCommand::Status { url, .. } => url.clone(),
        RagCommand::Search { url, .. } => url.clone(),
    }
}

fn command_url(command: &Command) -> String {
    match command {
        Command::Ping { url }
        | Command::Rpc { url, .. }
        | Command::Push { url, .. }
        => url.clone(),
        Command::Ocr { command } => match command {
            OcrCommand::Providers => DEFAULT_RPC_URL.to_string(),
            OcrCommand::Run { .. } => DEFAULT_RPC_URL.to_string(),
            OcrCommand::Status { url, .. } => url.clone(),
            OcrCommand::Reindex { url, .. } => url.clone(),
            OcrCommand::Process { url, .. } => url.clone(),
        },
        Command::Rag { command } => rag_command_url(command),
        Command::System { command } => match command {
            SystemCommand::Version { url }
            | SystemCommand::Libraries { url }
            | SystemCommand::LibraryStats { url, .. }
            | SystemCommand::Schema { url, .. }
            | SystemCommand::CurrentCollection { url }
            | SystemCommand::Methods { url, .. } => url.clone(),
        },
        Command::Search(ref args) => match &args.management {
            Some(SearchManagementCommand::SavedSearches { url })
            | Some(SearchManagementCommand::CreateSaved { url, .. })
            | Some(SearchManagementCommand::DeleteSaved { url, .. }) => url.clone(),
            None => args.url.clone(),
        },
        Command::Items { command } => match command {
            ItemsCommand::Add { url, .. }
            | ItemsCommand::Update { url, .. }
            | ItemsCommand::Delete { url, .. }
            | ItemsCommand::Trash { url, .. }
            | ItemsCommand::Restore { url, .. }
            | ItemsCommand::MergeDuplicates { url, .. }
            | ItemsCommand::AddRelated { url, .. }
            | ItemsCommand::RemoveRelated { url, .. }
            | ItemsCommand::Get { url, .. }
            | ItemsCommand::List { url, .. }
            | ItemsCommand::FindDuplicates { url }
            | ItemsCommand::Recent { url, .. }
            | ItemsCommand::Fulltext { url, .. }
            | ItemsCommand::Related { url, .. }
            | ItemsCommand::CitationKey { url, .. }
            | ItemsCommand::Path { url, .. }
            | ItemsCommand::Attachments { url, .. }
            | ItemsCommand::FindPdfs { url, .. } => url.clone(),
        },
        Command::Collections { command } => match command {
            CollectionsCommand::List { url }
            | CollectionsCommand::Tree { url }
            | CollectionsCommand::Get { url, .. }
            | CollectionsCommand::GetItems { url, .. }
            | CollectionsCommand::Stats { url, .. }
            | CollectionsCommand::Rename { url, .. }
            | CollectionsCommand::Create { url, .. }
            | CollectionsCommand::Delete { url, .. }
            | CollectionsCommand::AddItems { url, .. }
            | CollectionsCommand::RemoveItems { url, .. } => url.clone(),
        },
        Command::Notes { command } => match command {
            NotesCommand::List { url, .. }
            | NotesCommand::Get { url, .. }
            | NotesCommand::Create { url, .. }
            | NotesCommand::Update { url, .. }
            | NotesCommand::Delete { url, .. }
            | NotesCommand::Search { url, .. } => url.clone(),
        },
        Command::Settings { command } => match command {
            SettingsCommand::Get { url, .. }
            | SettingsCommand::List { url }
            | SettingsCommand::Set { url, .. } => url.clone(),
        },
        Command::Tags { command } => match command {
            TagsCommand::List { url, .. }
            | TagsCommand::Rename { url, .. }
            | TagsCommand::Delete { url, .. }
            | TagsCommand::Add { url, .. }
            | TagsCommand::Remove { url, .. } => url.clone(),
        },
        Command::Export(ref args) => args.url.clone(),
        Command::Annotations { command } => match command {
            AnnotationsCommand::List { url, .. }
            | AnnotationsCommand::Create { url, .. }
            | AnnotationsCommand::CreateBatch { url, .. }
            | AnnotationsCommand::Locate { url, .. }
            | AnnotationsCommand::Delete { url, .. } => url.clone(),
        },
    }
}

fn run_ocr_command(command: OcrCommand, client: &mut impl RpcCaller) -> Result<String, String> {
    if let OcrCommand::Providers = &command {
        return format_json(&serde_json::json!({ "providers": ocr_provider_specs() }));
    }
    let value = match command {
        OcrCommand::Providers => unreachable!(),
        OcrCommand::Run {
            provider,
            input,
            file,
            item_key,
            attachment_key,
            mime_type,
            endpoint,
            api_key_env,
        } => run_ocr_run_command(OcrRunOptions {
            provider,
            input,
            file,
            item_key,
            attachment_key,
            mime_type,
            endpoint,
            api_key_env,
        })?,
        OcrCommand::Status { collection, .. } => run_ocr_status_command(client, collection)?,
        OcrCommand::Reindex { collection, key, stale_only, chunk_chars, .. } => {
            return run_ocr_reindex_command(client, collection, key, stale_only, chunk_chars);
        }
        OcrCommand::Process {
            provider,
            parent,
            attachment,
            source_url,
            result_dir,
            result_zip,
            provider_endpoint,
            api_key_env,
            poll_interval_seconds,
            timeout_seconds,
            chunk_chars,
            ..
        } => {
            let resolved_provider = match provider {
                Some(p) => p,
                None => fetch_ocr_provider_from_settings(client)?,
            };
            let resolved_env = api_key_env.unwrap_or_else(|| "ZOTRON_OCR_API_KEY".to_string());
            let needs_auth = result_dir.is_none() && result_zip.is_none();
            if needs_auth && env::var(&resolved_env).ok().filter(|v| !v.is_empty()).is_none() {
                let key = fetch_ocr_api_key_from_settings(client);
                if !key.is_empty() {
                    unsafe { env::set_var(&resolved_env, &key); }
                }
            }
            run_ocr_process_command(
                client,
                OcrProcessOptions {
                    provider: resolved_provider,
                    parent,
                    attachment,
                    source_url,
                    result_dir,
                    result_zip,
                    provider_endpoint,
                    api_key_env: resolved_env,
                    poll_interval_seconds,
                    timeout_seconds,
                    chunk_chars,
                },
            )?
        }
    };
    format_json(&value)
}

struct OcrProcessOptions {
    provider: String,
    parent: String,
    attachment: Option<String>,
    source_url: Option<String>,
    result_dir: Option<String>,
    result_zip: Option<String>,
    provider_endpoint: Option<String>,
    api_key_env: String,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
    chunk_chars: usize,
}

struct OcrRunOptions {
    provider: String,
    input: Option<String>,
    file: Option<String>,
    item_key: Option<String>,
    attachment_key: Option<String>,
    mime_type: Option<String>,
    endpoint: Option<String>,
    api_key_env: Option<String>,
}

fn run_ocr_run_command(options: OcrRunOptions) -> Result<Value, String> {
    let input: OcrRequestInput = match (options.input, options.file) {
        (Some(input), None) => read_json_input(&input)?,
        (None, Some(file)) => ocr_input_from_file(
            file,
            options.item_key,
            options.attachment_key,
            options.mime_type,
        )?,
        (Some(_), Some(_)) => {
            return Err("INVALID_ARGS: use either --input or --file, not both".to_string())
        }
        (None, None) => return Err("INVALID_ARGS: provide --input JSON or --file".to_string()),
    };
    let request = build_ocr_provider_request(&options.provider, &input)?;
    let payload = if request.command.is_empty() {
        let method = request
            .method
            .ok_or_else(|| format!("OCR provider {} missing HTTP method", request.provider))?;
        let auth_scheme = raw_ocr_provider_spec(&options.provider)?.auth;
        let mut transport =
            provider_http_transport_with_auth(options.api_key_env.as_deref(), auth_scheme)?;
        transport.post_json(&ProviderHttpInvocation {
            provider: request.provider.to_string(),
            style: request.style.to_string(),
            method: method.to_string(),
            url: options
                .endpoint
                .or_else(|| request.url.map(ToString::to_string)),
            auth_header_name: request.auth_header.map(ToString::to_string),
            auth_header_value: None,
            body: request.body,
        })?
    } else {
        let mut command_runner = StdProviderCommandRunner;
        command_runner.run_json(&request.command)?
    };
    let blocks = match parse_ocr_provider_response(
        request.provider,
        &payload,
        &input.item_key,
        &input.attachment_key,
    ) {
        Ok(blocks) => blocks,
        Err(err) => {
            if let Some(task) = ocr_async_task_result(request.provider, &payload) {
                return Ok(task);
            }
            return Err(err);
        }
    };

    Ok(serde_json::json!({
        "provider": request.provider,
        "blocks": blocks,
    }))
}

fn fetch_ocr_provider_from_settings(client: &mut impl RpcCaller) -> Result<String, String> {
    let settings = client.call("settings.getAll", None)?;
    let provider = settings
        .get("ocr.provider")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if provider.is_empty() {
        return Err("MISSING_CONFIG: ocr.provider not configured — set it in Zotero → Settings → Zotron → OCR Settings".to_string());
    }
    Ok(provider)
}

fn fetch_ocr_api_key_from_settings(client: &mut impl RpcCaller) -> String {
    client
        .call("settings.getRaw", Some(serde_json::json!({"key": "ocr.apiKey"})))
        .ok()
        .and_then(|raw| raw.get("ocr.apiKey").and_then(Value::as_str).map(String::from))
        .unwrap_or_default()
}

fn run_ocr_process_command(
    client: &mut impl RpcCaller,
    mut options: OcrProcessOptions,
) -> Result<Value, String> {
    let spec = raw_ocr_provider_spec(&options.provider)?;

    let attachment = match options.attachment.take() {
        Some(key) => key,
        None => resolve_first_pdf_attachment_key(client, &options.parent)?,
    };
    options.attachment = Some(attachment.clone());

    let attachment_path = resolve_attachment_path(client, &attachment)?;
    let storage_dir = attachment_path
        .parent()
        .ok_or_else(|| {
            format!(
                "ATTACHMENT_PATH_INVALID: attachment path has no parent directory: {}",
                attachment_path.display()
            )
        })?
        .to_path_buf();

    match spec.provider_key {
        "mineru" | "mineru-cli" => {
            if options.result_dir.is_some() && options.result_zip.is_some() {
                return Err("INVALID_ARGS: use either --result-dir or --result-zip, not both".to_string());
            }
            if options.source_url.is_some()
                && (options.result_dir.is_some() || options.result_zip.is_some())
            {
                return Err(
                    "INVALID_ARGS: --source-url cannot be combined with --result-dir/--result-zip"
                        .to_string(),
                );
            }
            let file_name = attachment_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("document.pdf")
                .to_string();
            let source = load_mineru_result_source(&options, &attachment_path, &file_name)?;
            let artifacts = persist_mineru_result_sidecars(
                &storage_dir, &options.parent, &attachment,
                &options.provider, &source, options.chunk_chars,
            )?;
            let embedding_count = embed_sidecar_chunks(
                client, &storage_dir, &options.parent, &attachment, &artifacts.chunks,
            );
            Ok(serde_json::json!({
                "provider": spec.provider_key,
                "status": "indexed",
                "item_key": options.parent,
                "attachment_key": attachment,
                "embeddings": embedding_count,
                "attachment_path": attachment_path,
                "storage_dir": storage_dir,
                "task_id": source.task_id,
                "state": source.state,
                "blocks": artifacts.block_count,
                "chunks": artifacts.chunk_count,
                "artifacts": artifacts.artifacts,
            }))
        }
        _ => {
            run_ocr_process_sync(
                client, &options, spec.provider_key,
                &attachment, &attachment_path, &storage_dir,
            )
        }
    }
}

fn run_ocr_process_sync(
    client: &mut impl RpcCaller,
    options: &OcrProcessOptions,
    provider: &str,
    attachment_key: &str,
    attachment_path: &Path,
    storage_dir: &Path,
) -> Result<Value, String> {
    let api_url = if let Some(endpoint) = &options.provider_endpoint {
        endpoint.clone()
    } else {
        let settings = client.call("settings.getAll", None)?;
        settings.get("ocr.apiUrl")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    if api_url.is_empty() {
        return Err(format!("MISSING_CONFIG: ocr.apiUrl not configured for provider {provider}"));
    }

    let api_key = {
        let from_env = if !options.api_key_env.is_empty() {
            env::var(&options.api_key_env).ok().filter(|v| !v.is_empty())
        } else {
            None
        };
        from_env.unwrap_or_else(|| {
            client.call("settings.getRaw", Some(serde_json::json!({"key": "ocr.apiKey"})))
                .ok()
                .and_then(|raw| raw.get("ocr.apiKey").and_then(Value::as_str).map(String::from))
                .unwrap_or_default()
        })
    };

    let pdf_bytes = fs::read(attachment_path)
        .map_err(|e| format!("READ_PDF_FAILED: {}: {e}", attachment_path.display()))?;

    const MAX_PDF_SIZE: usize = 100 * 1024 * 1024; // 100 MB
    if pdf_bytes.len() > MAX_PDF_SIZE {
        return Err(format!(
            "PDF_TOO_LARGE: {} is {} MB, max {} MB",
            attachment_path.display(),
            pdf_bytes.len() / (1024 * 1024),
            MAX_PDF_SIZE / (1024 * 1024),
        ));
    }

    let base64_pdf = format!("data:application/pdf;base64,{}", base64_encode(&pdf_bytes));

    let input = OcrRequestInput {
        content_base64: base64_pdf,
        file_name: attachment_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.pdf")
            .to_string(),
        mime_type: "application/pdf".to_string(),
        item_key: options.parent.clone(),
        attachment_key: attachment_key.to_string(),
        source_url: None,
        local_path: Some(attachment_path.to_string_lossy().to_string()),
        output_dir: None,
    };
    let request = build_ocr_provider_request(provider, &input)?;

    let payload = if request.command.is_empty() {
        let method = request
            .method
            .ok_or_else(|| format!("OCR provider {provider} missing HTTP method"))?;
        let spec = raw_ocr_provider_spec(provider)?;

        let mut transport = if !api_key.is_empty() {
            match spec.auth {
                "bearer" => UreqProviderHttpTransport::with_bearer_token(&api_key),
                "token" => UreqProviderHttpTransport::with_api_key(format!("token {api_key}")),
                _ => UreqProviderHttpTransport::new(),
            }
        } else {
            UreqProviderHttpTransport::new()
        };

        transport.post_json(&ProviderHttpInvocation {
            provider: request.provider.to_string(),
            style: request.style.to_string(),
            method: method.to_string(),
            url: Some(api_url),
            auth_header_name: request.auth_header.map(ToString::to_string),
            auth_header_value: None,
            body: request.body,
        })?
    } else {
        let mut runner = StdProviderCommandRunner;
        runner.run_json(&request.command)?
    };

    let blocks = parse_ocr_provider_response(provider, &payload, &options.parent, attachment_key)?;
    let chunks = zotron_types::chunks_from_blocks(&blocks, options.chunk_chars);

    let artifacts = vec![
        write_sidecar_json(
            storage_dir, &options.parent, attachment_key,
            MachineArtifactKind::OcrRaw, &payload,
        )?,
        write_sidecar_jsonl(
            storage_dir, &options.parent, attachment_key,
            MachineArtifactKind::Blocks, &blocks,
        )?,
        write_chunks_sidecar(
            storage_dir, &options.parent, attachment_key, &chunks,
        )?,
    ];

    let embedding_count = embed_sidecar_chunks(client, storage_dir, &options.parent, attachment_key, &chunks);

    Ok(serde_json::json!({
        "provider": provider,
        "status": "indexed",
        "item_key": options.parent,
        "attachment_key": attachment_key,
        "embeddings": embedding_count,
        "attachment_path": attachment_path,
        "storage_dir": storage_dir,
        "blocks": blocks.len(),
        "chunks": chunks.len(),
        "artifacts": artifacts,
    }))
}

/// Schema version written as the first line of chunks.v1.jsonl by reindex.
const CHUNK_SCHEMA_VERSION: u32 = 2;

fn run_ocr_reindex_command(
    client: &mut impl RpcCaller,
    collection: Option<String>,
    key: Option<String>,
    stale_only: bool,
    chunk_chars: usize,
) -> Result<String, String> {
    // Resolve sidecar paths using the same logic as RAG search.
    let keys: Vec<String> = key.into_iter().collect();
    let sidecars = resolve_sidecar_paths(
        client,
        collection.as_deref(),
        &keys,
    )?;

    if sidecars.is_empty() {
        return format_json(
            &serde_json::json!({
                "reindexed": 0,
                "skipped": 0,
                "message": "no sidecars found"
            }));
    }

    let mut reindexed: Vec<Value> = Vec::new();
    let mut skipped = 0usize;

    for (item_key, att_key, sidecar_root) in &sidecars {
        // sidecar_root is storage_dir/.zotron
        // storage_dir is sidecar_root's parent
        let storage_dir = match sidecar_root.parent() {
            Some(p) => p,
            None => {
                skipped += 1;
                continue;
            }
        };

        let chunks_path = sidecar_root.join(zotron_types::MachineArtifactKind::Chunks.sidecar_relative_path());
        if stale_only {
            if let Ok(f) = fs::File::open(&chunks_path) {
                use std::io::BufRead;
                let mut reader = std::io::BufReader::new(f);
                let mut first_line = String::new();
                if reader.read_line(&mut first_line).is_ok() {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&first_line) {
                        if v.get("schema_version").and_then(|v| v.as_u64()) == Some(CHUNK_SCHEMA_VERSION as u64) {
                            skipped += 1;
                            continue;
                        }
                    }
                }
            }
        }

        let blocks_path = sidecar_root.join(zotron_types::MachineArtifactKind::Blocks.sidecar_relative_path());
        let blocks_content = match fs::read_to_string(&blocks_path) {
            Ok(c) => c,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        let blocks: Vec<zotron_types::PdfEvidenceBlock> = blocks_content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        if blocks.is_empty() {
            skipped += 1;
            continue;
        }

        // Re-chunk (same chunk-size handling as `ocr process`).
        let chunks = zotron_types::chunks_from_blocks(&blocks, chunk_chars);

        // Write chunks via the unified writer (adds the schema-version header).
        write_chunks_sidecar(storage_dir, item_key, att_key, &chunks)?;

        // Re-embed.
        let embedding_count = embed_sidecar_chunks(client, storage_dir, item_key, att_key, &chunks);

        reindexed.push(serde_json::json!({
            "item_key": item_key,
            "attachment_key": att_key,
            "chunks": chunks.len(),
            "embeddings": embedding_count,
        }));
    }

    format_json(
        &serde_json::json!({
            "reindexed": reindexed.len(),
            "skipped": skipped,
            "items": reindexed,
        }))
}

fn embed_sidecar_chunks(
    client: &mut impl RpcCaller,
    storage_dir: &Path,
    item_key: &str,
    _attachment_key: &str,
    chunks: &[zotron_types::StructureChunk],
) -> usize {
    let Ok((provider, model, api_url, api_key)) = fetch_embedding_settings(client) else {
        return 0;
    };
    if provider.is_empty() || (api_key.is_empty() && provider != "ollama") {
        return 0;
    }
    let emb_chunks: Vec<EmbeddingChunkInput> = chunks
        .iter()
        .map(|c| EmbeddingChunkInput {
            chunk_key: c.chunk_key.clone(),
            text: c.text.clone(),
        })
        .collect();
    if emb_chunks.is_empty() {
        return 0;
    }
    // Batch in groups of 20 to avoid API limits. On ANY batch failure we must
    // NOT clobber a complete existing vectors file with a truncated partial
    // result — that silently degrades future retrieval while the caller reports
    // a normal count. We collect into a temp file and atomically rename only if
    // every batch succeeded; on partial failure we warn and leave the existing
    // (complete) file untouched.
    let batch_size = 20;
    let total_batches = emb_chunks.chunks(batch_size).count();
    let mut all_vectors: Vec<EmbeddingVector> = Vec::new();
    let mut failed = false;
    for (i, batch) in emb_chunks.chunks(batch_size).enumerate() {
        let input = EmbeddingRequestInput {
            item_key: item_key.to_string(),
            chunks: batch.to_vec(),
            model: if model.is_empty() { None } else { Some(model.clone()) },
            url: if api_url.is_empty() { None } else { Some(api_url.clone()) },
            input_type: Some("document".to_string()),
        };
        let request = match build_embedding_provider_request(&provider, &input) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} request build failed: {e}", i + 1);
                failed = true;
                break;
            }
        };
        let Some(url) = request.url.as_deref() else {
            eprintln!("warning: embedding batch {}/{total_batches} has no provider URL configured", i + 1);
            failed = true;
            break;
        };
        let mut http = ureq::post(url).set("Content-Type", "application/json");
        if let Some(auth) = request.auth_header {
            if !api_key.is_empty() {
                http = http.set(auth, &format!("Bearer {api_key}"));
            }
        }
        let resp = match http.send_json(&request.body) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} request failed: {e}", i + 1);
                failed = true;
                break;
            }
        };
        let payload: Value = match resp.into_json() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} response parse failed: {e}", i + 1);
                failed = true;
                break;
            }
        };
        match parse_embedding_provider_response(&provider, &payload, item_key, batch) {
            Ok(vectors) => all_vectors.extend(vectors),
            Err(e) => {
                eprintln!("warning: embedding batch {}/{total_batches} parse failed: {e}", i + 1);
                failed = true;
                break;
            }
        }
    }

    if failed {
        let filename = embedding_vector_filename(&provider, &model);
        let vectors_path = storage_dir.join(".zotron").join("embeddings").join(&filename);
        let preserved = if vectors_path.exists() {
            " — existing vectors left untouched"
        } else {
            ""
        };
        eprintln!(
            "warning: embedding incomplete ({} of {} chunks); not writing partial vectors{preserved}",
            all_vectors.len(),
            emb_chunks.len(),
        );
        // Signal partial/failed state to the caller (0 == nothing reliably written).
        return 0;
    }

    let count = all_vectors.len();
    if count > 0 {
        let filename = embedding_vector_filename(&provider, &model);
        let vectors_dir = storage_dir.join(".zotron").join("embeddings");
        if let Err(e) = fs::create_dir_all(&vectors_dir) {
            eprintln!("warning: cannot create embeddings dir {}: {e}", vectors_dir.display());
            return 0;
        }
        let vectors_path = vectors_dir.join(&filename);
        let mut out = String::new();
        for v in &all_vectors {
            if let Ok(line) = serde_json::to_string(v) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        // Atomic write: temp file in the same dir, then rename over the target.
        let tmp_path = vectors_dir.join(format!("{filename}.tmp"));
        if let Err(e) = fs::write(&tmp_path, &out) {
            eprintln!("warning: failed to write temp embeddings {}: {e}", tmp_path.display());
            return 0;
        }
        if let Err(e) = fs::rename(&tmp_path, &vectors_path) {
            eprintln!("warning: failed to persist embeddings to {}: {e}", vectors_path.display());
            let _ = fs::remove_file(&tmp_path);
            return 0;
        }
    }
    count
}

struct MineruResultSource {
    task_id: Option<String>,
    state: String,
    result_dir: PathBuf,
    raw_zip_bytes: Option<Vec<u8>>,
    task_status: Option<Value>,
    payload: Value,
    content_list_file: Option<PathBuf>,
    markdown: Option<String>,
}

struct PersistedOcrArtifacts {
    block_count: usize,
    chunk_count: usize,
    artifacts: Vec<Value>,
    chunks: Vec<zotron_types::StructureChunk>,
}

fn resolve_attachment_path(
    client: &mut impl RpcCaller,
    attachment_key: &str,
) -> Result<PathBuf, String> {
    let payload = client.call(
        "attachments.getPath",
        Some(serde_json::json!({"key": attachment_key})),
    )?;
    let raw_path = payload
        .get("path")
        .and_then(Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| {
            format!("ATTACHMENT_PATH_NOT_FOUND: attachment {attachment_key} has no local PDF path")
        })?;
    Ok(PathBuf::from(local_path_from_zotero_path(raw_path)))
}

/// Resolve the first PDF attachment key for a parent item via `attachments.list`.
fn resolve_first_pdf_attachment_key(
    client: &mut impl RpcCaller,
    parent_key: &str,
) -> Result<String, String> {
    let response = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": parent_key})),
    )?;
    // The XPI returns {items: [...], total: N}.
    let attachments = response
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| response.as_array())
        .ok_or_else(|| {
            format!("NO_PDF_ATTACHMENT: no attachments found for item {parent_key}")
        })?;
    for attachment in attachments {
        if is_pdf_attachment(attachment) {
            if let Some(key) = attachment.get("key").and_then(Value::as_str) {
                return Ok(key.to_string());
            }
        }
    }
    Err(format!(
        "NO_PDF_ATTACHMENT: no PDF attachment found for item {parent_key}"
    ))
}

fn load_mineru_result_source(
    options: &OcrProcessOptions,
    attachment_path: &Path,
    file_name: &str,
) -> Result<MineruResultSource, String> {
    if let Some(result_dir) = options.result_dir.as_deref() {
        return mineru_result_source_from_dir(PathBuf::from(result_dir), None, None, None);
    }
    if let Some(result_zip) = options.result_zip.as_deref() {
        let zip_path = PathBuf::from(result_zip);
        let zip_bytes = fs::read(&zip_path)
            .map_err(|err| format!("read MinerU result zip {}: {err}", zip_path.display()))?;
        let result_dir = extract_zip_bytes_to_temp("zotron-mineru-result", &zip_bytes)?;
        return mineru_result_source_from_dir(result_dir, Some(zip_bytes), None, None);
    }

    let Some(source_url) = options
        .source_url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    else {
        return submit_mineru_local_file(options, attachment_path, file_name);
    };
    let input = OcrRequestInput {
        item_key: options.parent.clone(),
        attachment_key: options.attachment.clone().expect("attachment resolved"),
        file_name: file_name.to_string(),
        mime_type: "application/pdf".to_string(),
        content_base64: format!("url:{source_url}"),
        source_url: Some(source_url.to_string()),
        local_path: None,
        output_dir: None,
    };
    let task = submit_mineru_task(
        &options.provider,
        &input,
        options.provider_endpoint.clone(),
        &options.api_key_env,
    )?;
    let task_id = task
        .get("data")
        .and_then(|data| data.get("task_id"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU submit response missing data.task_id".to_string())?
        .to_string();
    let auth_header = provider_auth_header_value(&options.api_key_env, "bearer")?;
    let status = poll_mineru_task(
        options.provider_endpoint.as_deref(),
        &task_id,
        &auth_header,
        options.poll_interval_seconds,
        options.timeout_seconds,
    )?;
    let zip_url = status
        .pointer("/data/full_zip_url")
        .or_else(|| status.pointer("/data/result/full_zip_url"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU completed task missing data.full_zip_url".to_string())?;
    let zip_bytes = download_bytes(zip_url)?;
    let result_dir = extract_zip_bytes_to_temp("zotron-mineru-result", &zip_bytes)?;
    mineru_result_source_from_dir(result_dir, Some(zip_bytes), Some(status), Some(task_id))
}

fn submit_mineru_local_file(
    options: &OcrProcessOptions,
    attachment_path: &Path,
    file_name: &str,
) -> Result<MineruResultSource, String> {
    let auth_header = provider_auth_header_value(&options.api_key_env, "bearer")?;
    let upload_request = create_mineru_file_upload(
        options.provider_endpoint.as_deref(),
        file_name,
        options.attachment.as_deref().expect("attachment resolved"),
        &auth_header,
    )?;
    let upload_url = upload_request
        .pointer("/data/file_urls/0")
        .or_else(|| upload_request.pointer("/data/fileUrls/0"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU upload URL response missing data.file_urls[0]".to_string())?;
    let batch_id = upload_request
        .pointer("/data/batch_id")
        .or_else(|| upload_request.pointer("/data/batchId"))
        .and_then(Value::as_str)
        .ok_or_else(|| "MinerU upload URL response missing data.batch_id".to_string())?
        .to_string();
    let bytes = fs::read(attachment_path)
        .map_err(|err| format!("read attachment PDF {}: {err}", attachment_path.display()))?;
    put_bytes(upload_url, &bytes)?;
    let status = poll_mineru_batch(
        options.provider_endpoint.as_deref(),
        &batch_id,
        &auth_header,
        options.poll_interval_seconds,
        options.timeout_seconds,
    )?;
    let zip_url = mineru_batch_zip_url(&status)
        .ok_or_else(|| "MinerU completed batch missing full_zip_url".to_string())?;
    let zip_bytes = download_bytes(&zip_url)?;
    let result_dir = extract_zip_bytes_to_temp("zotron-mineru-result", &zip_bytes)?;
    mineru_result_source_from_dir(result_dir, Some(zip_bytes), Some(status), Some(batch_id))
}

fn create_mineru_file_upload(
    endpoint: Option<&str>,
    file_name: &str,
    data_id: &str,
    auth_header: &str,
) -> Result<Value, String> {
    let url = mineru_file_urls_url(endpoint);
    let body = serde_json::json!({
        "files": [{"name": file_name, "data_id": data_id}],
        "model_version": "vlm",
        "is_ocr": false,
        "enable_formula": true,
        "enable_table": true,
        "language": "ch",
        "page_ranges": "1-200",
    });
    ureq::post(&url)
        .set("Authorization", auth_header)
        .send_json(body)
        .map_err(|err| format!("POST {url} failed: {err}"))?
        .into_json::<Value>()
        .map_err(|err| format!("POST {url} returned invalid JSON: {err}"))
}

fn put_bytes(url: &str, bytes: &[u8]) -> Result<(), String> {
    ureq::put(url)
        .send_bytes(bytes)
        .map_err(|err| format!("PUT {url} failed: {err}"))?;
    Ok(())
}

fn submit_mineru_task(
    provider: &str,
    input: &OcrRequestInput,
    endpoint: Option<String>,
    api_key_env: &str,
) -> Result<Value, String> {
    let request = build_ocr_provider_request(provider, input)?;
    let method = request
        .method
        .ok_or_else(|| "MinerU provider missing HTTP method".to_string())?;
    let mut transport = provider_http_transport_with_auth(Some(api_key_env), "bearer")?;
    transport.post_json(&ProviderHttpInvocation {
        provider: request.provider.to_string(),
        style: request.style.to_string(),
        method: method.to_string(),
        url: endpoint.or_else(|| request.url.map(ToString::to_string)),
        auth_header_name: request.auth_header.map(ToString::to_string),
        auth_header_value: None,
        body: request.body,
    })
}

fn poll_mineru_task(
    endpoint: Option<&str>,
    task_id: &str,
    auth_header: &str,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let url = mineru_task_status_url(endpoint, task_id);
    let started = Instant::now();
    loop {
        let status = get_json_with_auth(&url, auth_header)?;
        let state = status
            .pointer("/data/state")
            .or_else(|| status.pointer("/data/status"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        match state {
            "done" | "finished" | "success" => return Ok(status),
            "failed" | "error" => return Err(format!("MinerU task {task_id} failed: {status}")),
            _ => {
                if started.elapsed() >= Duration::from_secs(timeout_seconds) {
                    return Err(format!(
                        "MinerU task {task_id} timed out after {timeout_seconds}s with state {state}"
                    ));
                }
                thread::sleep(Duration::from_secs(poll_interval_seconds.max(1)));
            }
        }
    }
}

fn mineru_task_status_url(endpoint: Option<&str>, task_id: &str) -> String {
    let base = endpoint
        .unwrap_or("https://mineru.net/api/v4/extract/task")
        .trim_end_matches('/');
    if base.ends_with("/extract/task") {
        format!("{base}/{task_id}")
    } else {
        format!("{base}/extract/task/{task_id}")
    }
}

fn mineru_file_urls_url(endpoint: Option<&str>) -> String {
    let base = mineru_api_base(endpoint);
    format!("{base}/file-urls/batch")
}

fn mineru_batch_status_url(endpoint: Option<&str>, batch_id: &str) -> String {
    let base = mineru_api_base(endpoint);
    format!("{base}/extract-results/batch/{batch_id}")
}

fn mineru_api_base(endpoint: Option<&str>) -> String {
    let base = endpoint
        .unwrap_or("https://mineru.net/api/v4/extract/task")
        .trim_end_matches('/');
    if let Some(stripped) = base.strip_suffix("/extract/task") {
        return stripped.to_string();
    }
    if let Some(stripped) = base.strip_suffix("/extract") {
        return stripped.to_string();
    }
    base.to_string()
}

fn poll_mineru_batch(
    endpoint: Option<&str>,
    batch_id: &str,
    auth_header: &str,
    poll_interval_seconds: u64,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let url = mineru_batch_status_url(endpoint, batch_id);
    let started = Instant::now();
    loop {
        let status = get_json_with_auth(&url, auth_header)?;
        let state = mineru_batch_state(&status).unwrap_or("unknown");
        match state {
            "done" | "finished" | "success" => return Ok(status),
            "failed" | "error" => return Err(format!("MinerU batch {batch_id} failed: {status}")),
            _ => {
                if started.elapsed() >= Duration::from_secs(timeout_seconds) {
                    return Err(format!(
                        "MinerU batch {batch_id} timed out after {timeout_seconds}s with state {state}"
                    ));
                }
                thread::sleep(Duration::from_secs(poll_interval_seconds.max(1)));
            }
        }
    }
}

fn mineru_batch_state(status: &Value) -> Option<&str> {
    status
        .pointer("/data/extract_result/0/state")
        .or_else(|| status.pointer("/data/extractResult/0/state"))
        .or_else(|| status.pointer("/data/state"))
        .and_then(Value::as_str)
}

fn mineru_batch_zip_url(status: &Value) -> Option<String> {
    status
        .pointer("/data/extract_result/0/full_zip_url")
        .or_else(|| status.pointer("/data/extractResult/0/full_zip_url"))
        .or_else(|| status.pointer("/data/full_zip_url"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn provider_auth_header_value(api_key_env: &str, auth_scheme: &str) -> Result<String, String> {
    let token = env::var(api_key_env)
        .map_err(|_| format!("missing provider credential env var {api_key_env}"))?;
    let token = token.trim();
    if token.is_empty() {
        return Err(format!(
            "provider credential env var {api_key_env} is empty"
        ));
    }
    Ok(match auth_scheme {
        "bearer" if token.starts_with("Bearer ") => token.to_string(),
        "bearer" => format!("Bearer {token}"),
        "token" if token.starts_with("token ") => token.to_string(),
        "token" => format!("token {token}"),
        _ => token.to_string(),
    })
}

fn get_json_with_auth(url: &str, auth_header: &str) -> Result<Value, String> {
    ureq::get(url)
        .set("Authorization", auth_header)
        .call()
        .map_err(|err| format!("GET {url} failed: {err}"))?
        .into_json::<Value>()
        .map_err(|err| format!("GET {url} returned invalid JSON: {err}"))
}

fn download_bytes(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|err| format!("download {url} failed: {err}"))?;
    let mut bytes = Vec::new();
    response
        .into_reader()
        .read_to_end(&mut bytes)
        .map_err(|err| format!("read download {url}: {err}"))?;
    Ok(bytes)
}

fn extract_zip_bytes_to_temp(prefix: &str, zip_bytes: &[u8]) -> Result<PathBuf, String> {
    let dir = unique_temp_path(prefix);
    fs::create_dir_all(&dir).map_err(|err| format!("create temp dir {}: {err}", dir.display()))?;
    let zip_path = dir.with_extension("zip");
    fs::write(&zip_path, zip_bytes)
        .map_err(|err| format!("write temp zip {}: {err}", zip_path.display()))?;
    let output = ProcessCommand::new("unzip")
        .arg("-q")
        .arg("-o")
        .arg(&zip_path)
        .arg("-d")
        .arg(&dir)
        .output()
        .map_err(|err| format!("run unzip: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "unzip {} failed: {}",
            zip_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(dir)
}

fn unique_temp_path(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()))
}

fn mineru_result_source_from_dir(
    result_dir: PathBuf,
    raw_zip_bytes: Option<Vec<u8>>,
    task_status: Option<Value>,
    task_id: Option<String>,
) -> Result<MineruResultSource, String> {
    let (payload, content_list_file) = mineru_payload_from_result_dir(&result_dir)?;
    let markdown = find_first_file_by_name(&result_dir, "full.md")
        .map(|path| {
            fs::read_to_string(&path)
                .map_err(|err| format!("read native markdown {}: {err}", path.display()))
        })
        .transpose()?;
    Ok(MineruResultSource {
        task_id,
        state: "done".to_string(),
        result_dir,
        raw_zip_bytes,
        task_status,
        payload,
        content_list_file,
        markdown,
    })
}

fn mineru_payload_from_result_dir(result_dir: &Path) -> Result<(Value, Option<PathBuf>), String> {
    let v2 = find_first_file_with_suffix(result_dir, "_content_list_v2.json");
    if let Some(path) = v2 {
        let value = read_json_file(&path)?;
        return Ok((serde_json::json!({"content_list_v2": value}), Some(path)));
    }
    let content_list = find_first_file_with_suffix(result_dir, "_content_list.json");
    if let Some(path) = content_list {
        let value = read_json_file(&path)?;
        return Ok((serde_json::json!({"content_list": value}), Some(path)));
    }
    let layout = find_first_file_by_name(result_dir, "layout.json");
    if let Some(path) = layout {
        return Ok((read_json_file(&path)?, Some(path)));
    }
    let markdown = find_first_file_by_name(result_dir, "full.md");
    if let Some(path) = markdown {
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("read native markdown {}: {err}", path.display()))?;
        return Ok((serde_json::json!({"result": text}), Some(path)));
    }
    Err(format!(
        "MinerU result directory {} missing content_list_v2/content_list/layout/full.md",
        result_dir.display()
    ))
}

fn read_json_file(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("parse JSON {}: {err}", path.display()))
}

fn persist_mineru_result_sidecars(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    provider: &str,
    source: &MineruResultSource,
    chunk_chars: usize,
) -> Result<PersistedOcrArtifacts, String> {
    let blocks = parse_ocr_provider_response(provider, &source.payload, item_key, attachment_key)?;
    let chunks = zotron_types::chunks_from_blocks(&blocks, chunk_chars);
    let assets = copy_mineru_assets(&source.result_dir, storage_dir)?;
    let raw_bundle = serde_json::json!({
        "provider": provider,
        "item_key": item_key,
        "attachment_key": attachment_key,
        "task_id": source.task_id,
        "state": source.state,
        "task_status": source.task_status,
        "content_list_file": source.content_list_file,
        "payload": source.payload,
    });

    let mut artifacts = Vec::new();
    artifacts.push(write_sidecar_json(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::OcrRaw,
        &raw_bundle,
    )?);
    artifacts.push(write_sidecar_jsonl(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::Blocks,
        &blocks,
    )?);
    artifacts.push(write_chunks_sidecar(
        storage_dir,
        item_key,
        attachment_key,
        &chunks,
    )?);
    if let Some(markdown) = source.markdown.as_deref() {
        artifacts.push(write_sidecar_bytes(
            storage_dir,
            item_key,
            attachment_key,
            MachineArtifactKind::OcrNativeMarkdown,
            markdown.as_bytes(),
        )?);
    }
    artifacts.push(write_sidecar_json(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::OcrNativeAssets,
        &assets,
    )?);
    if let Some(bytes) = source.raw_zip_bytes.as_deref() {
        artifacts.push(write_extra_sidecar_bytes(
            storage_dir,
            ".zotron/ocr/latest.raw.zip",
            bytes,
        )?);
    }

    Ok(PersistedOcrArtifacts {
        block_count: blocks.len(),
        chunk_count: chunks.len(),
        artifacts,
        chunks,
    })
}

fn write_sidecar_json(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    value: &Value,
) -> Result<Value, String> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    write_sidecar_bytes(storage_dir, item_key, attachment_key, kind, &bytes)
}

fn write_sidecar_jsonl<T: serde::Serialize>(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    values: &[T],
) -> Result<Value, String> {
    let mut out = String::new();
    for value in values {
        out.push_str(&serde_json::to_string(value).map_err(|err| err.to_string())?);
        out.push('\n');
    }
    write_sidecar_bytes(storage_dir, item_key, attachment_key, kind, out.as_bytes())
}

/// Write the Chunks sidecar with a `{"schema_version":N}` header line followed
/// by one chunk per line. The header lets `ocr reindex --stale-only` detect
/// freshly-produced (current-schema) sidecars and skip re-embedding them.
/// This is the single writer for the Chunks artifact — `ocr process` (sync +
/// MinerU) and `ocr reindex` all go through here so the on-disk format stays
/// consistent.
fn write_chunks_sidecar(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    chunks: &[zotron_types::StructureChunk],
) -> Result<Value, String> {
    let mut out = String::new();
    out.push_str(&format!("{{\"schema_version\":{CHUNK_SCHEMA_VERSION}}}\n"));
    for chunk in chunks {
        out.push_str(&serde_json::to_string(chunk).map_err(|err| err.to_string())?);
        out.push('\n');
    }
    write_sidecar_bytes(
        storage_dir,
        item_key,
        attachment_key,
        MachineArtifactKind::Chunks,
        out.as_bytes(),
    )
}

fn write_sidecar_bytes(
    storage_dir: &Path,
    item_key: &str,
    attachment_key: &str,
    kind: MachineArtifactKind,
    bytes: &[u8],
) -> Result<Value, String> {
    let record = write_machine_artifact_sidecar(storage_dir, item_key, attachment_key, kind, bytes)
        .map_err(|err| format!("write sidecar {:?}: {err}", kind))?;
    Ok(serde_json::json!({
        "kind": kind,
        "relative_path": record.relative_path,
        "absolute_path": record.absolute_path,
    }))
}

fn write_extra_sidecar_bytes(
    storage_dir: &Path,
    relative_path: &str,
    bytes: &[u8],
) -> Result<Value, String> {
    let absolute_path = storage_dir.join(relative_path);
    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    fs::write(&absolute_path, bytes)
        .map_err(|err| format!("write sidecar {}: {err}", absolute_path.display()))?;
    Ok(serde_json::json!({
        "kind": "ocr_raw_zip",
        "relative_path": relative_path,
        "absolute_path": absolute_path,
    }))
}

fn copy_mineru_assets(result_dir: &Path, storage_dir: &Path) -> Result<Value, String> {
    let mut images = Vec::new();
    for file in collect_files(result_dir)? {
        if !is_image_file(&file) {
            continue;
        }
        let relative = file.strip_prefix(result_dir).unwrap_or(&file).to_path_buf();
        let destination = storage_dir.join(".zotron").join("ocr").join(&relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::copy(&file, &destination).map_err(|err| {
            format!(
                "copy MinerU asset {} to {}: {err}",
                file.display(),
                destination.display()
            )
        })?;
        images.push(serde_json::json!({
            "source_relative": relative,
            "sidecar_relative": PathBuf::from(".zotron").join("ocr").join(&relative),
            "absolute_path": destination,
        }));
    }
    Ok(serde_json::json!({
        "provider": "mineru",
        "images": images,
    }))
}

fn is_image_file(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "png" | "jpg" | "jpeg" | "webp" | "gif"
    )
}

fn find_first_file_with_suffix(root: &Path, suffix: &str) -> Option<PathBuf> {
    collect_files(root).ok()?.into_iter().find(|path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(suffix))
    })
}

fn find_first_file_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    collect_files(root).ok()?.into_iter().find(|path| {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name == name)
    })
}

fn collect_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    collect_files_into(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_files_into(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|err| format!("read dir {}: {err}", root.display()))? {
        let entry = entry.map_err(|err| format!("read dir entry {}: {err}", root.display()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| format!("stat {}: {err}", path.display()))?;
        if file_type.is_dir() {
            collect_files_into(&path, files)?;
        } else if file_type.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn ocr_async_task_result(provider: &str, payload: &Value) -> Option<Value> {
    let data = payload.get("data")?;
    let task_id = data.get("task_id").and_then(Value::as_str)?;
    Some(serde_json::json!({
        "provider": provider,
        "status": "submitted",
        "task_id": task_id,
        "state": data.get("state").and_then(Value::as_str).unwrap_or("submitted"),
        "result_url": data.get("full_zip_url").or_else(|| data.get("markdown_url")).cloned(),
        "raw": payload,
    }))
}

fn ocr_input_from_file(
    file: String,
    item_key: Option<String>,
    attachment_key: Option<String>,
    mime_type: Option<String>,
) -> Result<OcrRequestInput, String> {
    let item_key = item_key
        .ok_or_else(|| "INVALID_ARGS: --item-key is required when using --file".to_string())?;
    let attachment_key = attachment_key.ok_or_else(|| {
        "INVALID_ARGS: --attachment-key is required when using --file".to_string()
    })?;
    let path = PathBuf::from(&file);
    let bytes = fs::read(&path).map_err(|err| format!("read {file}: {err}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf")
        .to_string();
    let mime_type = mime_type.unwrap_or_else(|| guess_mime_type(&path).to_string());
    Ok(OcrRequestInput {
        item_key,
        attachment_key,
        file_name,
        mime_type,
        content_base64: base64_encode(&bytes),
        source_url: None,
        local_path: Some(file),
        output_dir: None,
    })
}

fn guess_mime_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => "application/pdf",
    }
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn run_ocr_status_command(
    client: &mut impl RpcCaller,
    collection: String,
) -> Result<Value, String> {
    let collection_key = find_collection_in_tree(client, &collection)?
        .and_then(|node| node.get("key").cloned())
        .ok_or_else(|| format!("COLLECTION_NOT_FOUND: Collection not found: {collection:?}"))?;
    let raw = paginate_rpc(
        client,
        "collections.getItems",
        serde_json::json!({"key": collection_key}),
        500,
    )?;
    let items = raw
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| raw.as_array())
        .ok_or_else(|| "collections.getItems returned non-array/non-items result".to_string())?
        .clone();

    let mut has_ocr = 0usize;
    for item in &items {
        let item_key = item.get("key").cloned().unwrap_or(Value::Null);
        if has_ocr_artifact(client, &item_key)? || has_ocr_note(client, &item_key)? {
            has_ocr += 1;
        }
    }

    Ok(serde_json::json!({
        "collection": collection,
        "total": items.len(),
        "has_ocr": has_ocr,
        "missing_ocr": items.len() - has_ocr,
    }))
}

fn has_ocr_artifact(client: &mut impl RpcCaller, item_key: &Value) -> Result<bool, String> {
    if let Some(item_key) = item_key.as_str() {
        // Legacy external store lookup for artifacts produced before the
        // per-attachment hidden sidecar became the default.
        if machine_artifact_exists_for_item(
            machine_artifact_store_root(),
            item_key,
            MachineArtifactKind::Chunks,
        ) {
            return Ok(true);
        }
    }

    let attachments = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": item_key.clone()})),
    )?;
    Ok(attachments.as_array().is_some_and(|attachments| {
        attachments.iter().any(|attachment| {
            let has_sidecar_chunks = attachment
                .get("path")
                .and_then(Value::as_str)
                .map(local_path_from_zotero_path)
                .as_deref()
                .map(Path::new)
                .and_then(Path::parent)
                .is_some_and(|dir| {
                    machine_artifact_exists_in_sidecar(dir, MachineArtifactKind::Chunks)
                });
            if has_sidecar_chunks {
                return true;
            }

            // Read-only fallback for old Zotero-visible artifact attachments.
            attachment
                .get("title")
                .and_then(Value::as_str)
                .is_some_and(|title| title.ends_with("zotron-chunks.jsonl"))
        })
    }))
}

pub(crate) fn local_path_from_zotero_path(path: &str) -> String {
    if is_wsl() && path.as_bytes().get(1) == Some(&b':') {
        return ProcessCommand::new("wslpath")
            .arg("-u")
            .arg(path)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|converted| converted.trim().to_string())
            .filter(|converted| !converted.is_empty())
            .unwrap_or_else(|| path.to_string());
    }
    path.to_string()
}

fn has_ocr_note(client: &mut impl RpcCaller, item_key: &Value) -> Result<bool, String> {
    let notes = client.call(
        "notes.list",
        Some(serde_json::json!({"parentKey": item_key.clone()})),
    )?;
    Ok(notes.as_array().is_some_and(|notes| {
        notes.iter().any(|note| {
            note.get("tags")
                .and_then(Value::as_array)
                .is_some_and(|tags| tags.iter().any(tag_is_ocr))
        })
    }))
}

fn tag_is_ocr(tag: &Value) -> bool {
    tag.as_str() == Some("ocr")
        || tag
            .get("tag")
            .and_then(Value::as_str)
            .is_some_and(|tag| tag == "ocr")
}

pub(crate) fn find_collection_in_tree(
    client: &mut impl RpcCaller,
    collection: &str,
) -> Result<Option<Value>, String> {
    let tree = client.call("collections.tree", None)?;
    let nodes = tree
        .as_array()
        .ok_or_else(|| "collections.tree returned non-array result".to_string())?;
    Ok(search_collection_tree(nodes, collection).cloned())
}

fn search_collection_tree<'a>(nodes: &'a [Value], collection: &str) -> Option<&'a Value> {
    for node in nodes {
        if node.get("key").and_then(Value::as_str) == Some(collection)
            || node.get("name").and_then(Value::as_str) == Some(collection)
        {
            return Some(node);
        }
        if let Some(children) = node.get("children").and_then(Value::as_array) {
            if let Some(found) = search_collection_tree(children, collection) {
                return Some(found);
            }
        }
    }
    None
}

fn run_command(command: Command, client: &mut impl RpcCaller) -> Result<String, String> {
    if let Command::Export(args) = command {
        return run_export(args, client);
    }

    let value = match command {
        Command::Ping { .. } => call_json(client, "system.ping", None)?,
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

fn run_push_command(
    json_file: String,
    pdf: Option<String>,
    collection: Option<String>,
    on_duplicate: String,
    dry_run: bool,
    client: &mut impl RpcCaller,
) -> Result<String, String> {
    if !matches!(on_duplicate.as_str(), "skip" | "update" | "create") {
        return Err(format!(
            "INVALID_ARGS: --on-duplicate must be skip|update|create, got {on_duplicate:?}"
        ));
    }

    let payload = if json_file == "-" {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|err| format!("read stdin: {err}"))?;
        input
    } else {
        fs::read_to_string(&json_file).map_err(|err| format!("read {json_file}: {err}"))?
    };
    let item_json = serde_json::from_str::<Value>(&payload)
        .map_err(|err| format!("INVALID_JSON: Could not parse JSON: {err}"))?;

    // Validate required fields
    match item_json.get("itemType").and_then(Value::as_str) {
        Some(s) if !s.is_empty() => {}
        _ => return Err("INVALID_ARGS: input JSON must include a non-empty \"itemType\" field".to_string()),
    }

    if dry_run {
        let collection_key = collection
            .as_deref()
            .map(|name| resolve_collection(client, name))
            .transpose()?;
        return format_json(
            &serde_json::json!({
                "ok": true,
                "dryRun": true,
                "wouldPush": {
                    "title": item_json.get("title").cloned().unwrap_or(Value::Null),
                    "itemType": item_json.get("itemType").cloned().unwrap_or(Value::Null),
                    "collectionKey": collection_key,
                    "pdfPath": pdf,
                    "onDuplicate": on_duplicate,
                }
            }));
    }

    let result = push_item(
        client,
        &item_json,
        pdf.as_deref(),
        collection.as_deref(),
        &on_duplicate,
    )?;
    format_json(&result)
}

fn push_item(
    client: &mut impl RpcCaller,
    item_json: &Value,
    pdf_path: Option<&str>,
    collection: Option<&str>,
    on_duplicate: &str,
) -> Result<Value, String> {
    let pdf_size = if let Some(path) = pdf_path {
        validate_pdf_magic(path)?
    } else {
        0
    };

    let collection_key = match collection {
        Some(name) => resolve_collection(client, name)?,
        None => resolve_current_collection(client)?,
    };

    let dup_id = find_duplicate(client, item_json)?;
    if let Some(dup_id) = dup_id.as_deref().filter(|_| on_duplicate == "skip") {
        if !is_library_root(&collection_key) {
            client.call(
                "collections.addItems",
                Some(serde_json::json!({"key": collection_key, "keys": [dup_id]})),
            )?;
        }
        let mut pdf_attached = false;
        if let Some(path) = pdf_path {
            if !item_has_pdf_attachment(client, dup_id)? {
                attach_pdf(client, dup_id, path)?;
                pdf_attached = true;
            }
        }
        return Ok(push_result(
            "skipped_duplicate",
            Some(dup_id.to_string()),
            pdf_attached,
            if pdf_attached { pdf_size } else { 0 },
            Value::Null,
        ));
    }

    let xpi_payload = to_xpi_payload(item_json, Some(&collection_key));
    let (item_key, status) =
        if let Some(dup_id) = dup_id.as_deref().filter(|_| on_duplicate == "update") {
            let mut params = serde_json::Map::new();
            params.insert("key".to_string(), Value::String(dup_id.to_string()));
            params.insert(
                "fields".to_string(),
                xpi_payload
                    .get("fields")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({})),
            );
            if let Some(creators) = xpi_payload.get("creators") {
                params.insert("creators".to_string(), creators.clone());
            }
            if let Some(tags) = xpi_payload.get("tags") {
                params.insert("tags".to_string(), tags.clone());
            }
            client.call("items.update", Some(Value::Object(params)))?;
            (dup_id.to_string(), "updated")
        } else {
            let created = client.call("items.create", Some(xpi_payload))?;
            let key = created
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("items.create returned unexpected shape: {created:?}"))?;
            (key.to_string(), "created")
        };

    let mut pdf_attached = false;
    if let Some(path) = pdf_path {
        if status != "updated" || !item_has_pdf_attachment(client, &item_key)? {
            attach_pdf(client, &item_key, path)?;
            pdf_attached = true;
        }
    }

    if status == "updated" && !is_library_root(&collection_key) {
        client.call(
            "collections.addItems",
            Some(serde_json::json!({"key": collection_key, "keys": [item_key]})),
        )?;
    }

    Ok(push_result(
        status,
        Some(item_key),
        pdf_attached,
        if pdf_attached { pdf_size } else { 0 },
        Value::Null,
    ))
}

fn validate_pdf_magic(path: &str) -> Result<u64, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("INVALID_PDF: cannot read {path}: {e}"))?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(format!(
            "INVALID_PDF: {path} does not start with %PDF- magic bytes"
        ));
    }
    Ok(bytes.len() as u64)
}

fn resolve_current_collection(client: &mut impl RpcCaller) -> Result<Value, String> {
    let selected = client.call("system.currentCollection", None)?;
    Ok(selected
        .get("key")
        .cloned()
        .unwrap_or_else(|| Value::Number(0.into())))
}

fn find_duplicate(
    client: &mut impl RpcCaller,
    item_json: &Value,
) -> Result<Option<String>, String> {
    if let Some(doi) = item_json
        .get("DOI")
        .and_then(Value::as_str)
        .filter(|doi| !doi.is_empty())
    {
        let hits = client.call("search.byIdentifier", Some(serde_json::json!({"doi": doi})))?;
        if let Some(key) = first_hit_key(&hits) {
            return Ok(Some(key));
        }
    }

    if let Some(title) = item_json
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| title.len() >= 10)
    {
        let hits = client.call(
            "search.quick",
            Some(serde_json::json!({"query": title, "limit": 20})),
        )?;
        if let Some(items) = response_items(&hits) {
            for item in items {
                if item.get("title").and_then(Value::as_str) == Some(title) {
                    if let Some(key) = item.get("key").and_then(Value::as_str) {
                        return Ok(Some(key.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

fn first_hit_key(response: &Value) -> Option<String> {
    response_items(response)?
        .first()?
        .get("key")?
        .as_str()
        .map(ToString::to_string)
}

fn response_items(response: &Value) -> Option<&Vec<Value>> {
    response
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| response.as_array())
}

fn to_xpi_payload(item_json: &Value, collection_key: Option<&Value>) -> Value {
    const NON_FIELD_KEYS: &[&str] = &[
        "itemType",
        "creators",
        "tags",
        "collections",
        "attachments",
        "relations",
        "notes",
        "id",
        "key",
        "version",
    ];

    let mut fields = serde_json::Map::new();
    if let Some(item) = item_json.as_object() {
        for (key, value) in item {
            if !NON_FIELD_KEYS.contains(&key.as_str()) && !value.is_null() && value != "" {
                fields.insert(key.clone(), value.clone());
            }
        }
    }

    let mut payload = serde_json::Map::new();
    payload.insert(
        "itemType".to_string(),
        item_json
            .get("itemType")
            .cloned()
            .unwrap_or_else(|| Value::String("journalArticle".to_string())),
    );
    payload.insert("fields".to_string(), Value::Object(fields));

    if let Some(creators) = item_json.get("creators").and_then(Value::as_array) {
        if !creators.is_empty() {
            payload.insert(
                "creators".to_string(),
                Value::Array(
                    creators
                        .iter()
                        .map(|creator| {
                            let mut c = serde_json::json!({
                                "firstName": creator.get("firstName").and_then(Value::as_str).unwrap_or(""),
                                "lastName": creator.get("lastName").and_then(Value::as_str).unwrap_or(""),
                                "creatorType": creator.get("creatorType").and_then(Value::as_str).unwrap_or("author"),
                            });
                            if let Some(fm) = creator.get("fieldMode").and_then(Value::as_u64) {
                                c["fieldMode"] = Value::from(fm);
                            }
                            c
                        })
                        .collect(),
                ),
            );
        }
    }

    if let Some(tags) = item_json.get("tags").and_then(Value::as_array) {
        if !tags.is_empty() {
            payload.insert(
                "tags".to_string(),
                Value::Array(
                    tags.iter()
                        .map(|tag| tag.get("tag").cloned().unwrap_or_else(|| tag.clone()))
                        .collect(),
                ),
            );
        }
    }

    if let Some(collection_key) = collection_key.filter(|key| !is_library_root(key)) {
        payload.insert(
            "collections".to_string(),
            Value::Array(vec![collection_key.clone()]),
        );
    }

    Value::Object(payload)
}

fn item_has_pdf_attachment(client: &mut impl RpcCaller, item_key: &str) -> Result<bool, String> {
    let attachments = client.call(
        "attachments.list",
        Some(serde_json::json!({"parentKey": item_key})),
    )?;
    Ok(has_pdf_attachment(&attachments))
}

fn attach_pdf(client: &mut impl RpcCaller, item_key: &str, path: &str) -> Result<(), String> {
    client.call(
        "attachments.add",
        Some(serde_json::json!({
            "parentKey": item_key,
            "path": zotero_path(path),
            "title": "Full Text PDF",
        })),
    )?;
    Ok(())
}

fn zotero_path(path: &str) -> String {
    let path = Path::new(path)
        .canonicalize()
        .unwrap_or_else(|_| Path::new(path).to_path_buf())
        .to_string_lossy()
        .into_owned();
    if is_wsl() {
        return ProcessCommand::new("wslpath")
            .arg("-w")
            .arg(&path)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|converted| converted.trim().to_string())
            .filter(|converted| !converted.is_empty())
            .unwrap_or(path);
    }
    path
}

fn is_wsl() -> bool {
    if env::var_os("WSL_DISTRO_NAME").is_some() {
        return true;
    }
    fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|release| release.to_ascii_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

fn is_library_root(value: &Value) -> bool {
    value.as_i64() == Some(0) || value.as_u64() == Some(0)
}

fn push_result(
    status: &str,
    zotero_item_key: Option<String>,
    pdf_attached: bool,
    pdf_size_bytes: u64,
    error: Value,
) -> Value {
    serde_json::json!({
        "status": status,
        "zotero_item_key": zotero_item_key,
        "pdf_attached": pdf_attached,
        "pdf_size_bytes": pdf_size_bytes,
        "error": error,
    })
}

fn run_search(
    args: SearchArgs,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let SearchArgs {
        query, fulltext, author, after, before, journal, tag,
        doi, isbn, issn, collection, limit, offset, ..
    } = args;

    let has_identifier = doi.is_some() || isbn.is_some() || issn.is_some();
    if has_identifier {
        let mut params = serde_json::Map::new();
        if let Some(doi) = doi { params.insert("doi".into(), Value::String(doi)); }
        if let Some(isbn) = isbn { params.insert("isbn".into(), Value::String(isbn)); }
        if let Some(issn) = issn { params.insert("issn".into(), Value::String(issn)); }
        let value = client.call("search.byIdentifier", Some(Value::Object(params)))?;
        return Ok(normalize_list_envelope(value, "items", None, 0));
    }

    if fulltext {
        let query = query.ok_or("INVALID_ARGS: --fulltext requires a search query")?;
        let mut params = serde_json::json!({"query": query, "limit": limit});
        if let (Some(col), Some(map)) = (collection, params.as_object_mut()) {
            map.insert("collection".into(), resolve_collection(client, &col)?);
        }
        let value = client.call("search.fulltext", Some(params))?;
        return Ok(normalize_list_envelope(value, "items", Some(limit), 0));
    }

    let has_filters = author.is_some() || after.is_some() || before.is_some()
        || journal.is_some() || tag.is_some();
    if has_filters {
        let mut conditions: Vec<Value> = Vec::new();
        if let Some(query) = &query {
            conditions.push(serde_json::json!({
                "field": "quicksearch-titleCreatorYear",
                "operator": "contains",
                "value": query,
            }));
        }
        if let Some(author) = author {
            conditions.push(serde_json::json!({
                "field": "creator", "operator": "contains", "value": author,
            }));
        }
        if let Some(after) = after {
            conditions.push(serde_json::json!({
                "field": "date", "operator": "isAfter", "value": after,
            }));
        }
        if let Some(before) = before {
            conditions.push(serde_json::json!({
                "field": "date", "operator": "isBefore", "value": before,
            }));
        }
        if let Some(journal) = journal {
            conditions.push(serde_json::json!({
                "field": "publicationTitle", "operator": "contains", "value": journal,
            }));
        }
        if let Some(tag) = tag {
            conditions.push(serde_json::json!({
                "field": "tag", "operator": "is", "value": tag,
            }));
        }
        let value = client.call(
            "search.advanced",
            Some(serde_json::json!({
                "conditions": conditions,
                "operator": "and",
                "limit": limit,
                "offset": offset,
            })),
        )?;
        return Ok(normalize_list_envelope(value, "items", Some(limit), offset));
    }

    let query = query.ok_or(
        "INVALID_ARGS: provide a search query, or use --doi/--isbn/--issn for identifier lookup"
    )?;
    let value = if let Some(col) = collection {
        let key = resolve_collection(client, &col)?;
        let response = client.call(
            "collections.getItems",
            Some(serde_json::json!({"key": key})),
        )?;
        collection_quick_search_response(&response, &query, limit)
    } else {
        filter_search_artifacts(client.call(
            "search.quick",
            Some(serde_json::json!({"query": query, "limit": limit})),
        )?)
    };
    Ok(normalize_list_envelope(value, "items", Some(limit), 0))
}

fn run_search_management_command(
    command: SearchManagementCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    match command {
        SearchManagementCommand::SavedSearches { .. } => Ok(normalize_list_envelope(
            client.call("search.savedSearches", None)?,
            "items",
            None,
            0,
        )),
        SearchManagementCommand::CreateSaved {
            name, condition, dry_run, ..
        } => {
            let conditions = condition
                .iter()
                .map(|raw| parse_search_condition(raw))
                .collect::<Result<Vec<_>, _>>()?;
            let params = serde_json::json!({"name": name, "conditions": conditions});
            if dry_run {
                Ok(dry_run_value("search.createSavedSearch", params))
            } else {
                Ok(client.call("search.createSavedSearch", Some(params))?)
            }
        }
        SearchManagementCommand::DeleteSaved {
            search_key, dry_run, ..
        } => {
            let params = serde_json::json!({"key": search_key});
            if dry_run {
                Ok(dry_run_value("search.deleteSavedSearch", params))
            } else {
                Ok(client.call("search.deleteSavedSearch", Some(params))?)
            }
        }
    }
}

fn filter_search_artifacts(mut value: Value) -> Value {
    let Some(items) = value.get_mut("items").and_then(Value::as_array_mut) else {
        return value;
    };
    items.retain(|item| match item.get("title").and_then(Value::as_str) {
        Some(title) => !is_zotron_evidence_artifact(title),
        None => true,
    });
    let total_items = items.len() as u64;
    if let Some(total) = value.get_mut("total") {
        *total = Value::from(total_items);
    }
    value
}

fn collection_quick_search_response(response: &Value, query: &str, limit: u64) -> Value {
    let mut matched = collection_items(response)
        .into_iter()
        .filter(|item| !item_is_evidence_artifact(item))
        .filter(|item| quick_item_matches(item, query))
        .collect::<Vec<_>>();
    let total = matched.len() as u64;
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    if matched.len() > limit {
        matched.truncate(limit);
    }
    serde_json::json!({"items": matched, "total": total})
}

fn item_is_evidence_artifact(item: &Value) -> bool {
    item.get("title")
        .and_then(Value::as_str)
        .is_some_and(is_zotron_evidence_artifact)
}

fn quick_item_matches(item: &Value, query: &str) -> bool {
    let terms = query
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .filter(|term| !term.is_empty())
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return true;
    }
    let mut haystack = String::new();
    append_search_text(item, &mut haystack);
    let haystack = haystack.to_lowercase();
    terms.iter().all(|term| haystack.contains(term))
}

fn append_search_text(value: &Value, out: &mut String) {
    match value {
        Value::String(text) => {
            out.push(' ');
            out.push_str(text);
        }
        Value::Number(number) => {
            out.push(' ');
            out.push_str(&number.to_string());
        }
        Value::Bool(value) => {
            out.push(' ');
            out.push_str(if *value { "true" } else { "false" });
        }
        Value::Array(items) => {
            for item in items {
                append_search_text(item, out);
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                append_search_text(item, out);
            }
        }
        Value::Null => {}
    }
}

fn parse_search_condition(raw: &str) -> Result<Value, String> {
    let mut parts = raw.split_whitespace();
    let field = parts.next();
    let operator = parts.next();
    let value = parts.collect::<Vec<_>>().join(" ");
    match (field, operator, value.is_empty()) {
        (Some(field), Some(operator), false) => Ok(serde_json::json!({
            "field": field,
            "operator": operator,
            "value": value,
        })),
        _ => Err(format!(
            "INVALID_ARGS: --condition must be 'field operator value', got: {raw:?}"
        )),
    }
}

const RPC_PAGINATION_SAFETY_CAP: usize = 10_000;
const RPC_PAGE_LIST_KEYS: [&str; 4] = ["items", "tags", "results", "data"];

pub(crate) fn paginate_rpc(
    client: &mut impl RpcCaller,
    method: &str,
    params: Value,
    page_size: usize,
) -> Result<Value, String> {
    let base = params
        .as_object()
        .ok_or_else(|| "params must be a JSON object".to_string())?;
    let mut out = Vec::new();
    let mut prev_page: Option<Vec<Value>> = None;
    let mut offset = 0usize;

    loop {
        let mut page_params = base.clone();
        page_params.insert("offset".to_string(), Value::Number(offset.into()));
        page_params.insert("limit".to_string(), Value::Number(page_size.into()));
        let response = client.call(method, Some(Value::Object(page_params)))?;

        let page = match extract_page(&response) {
            Some(page) => page,
            None if out.is_empty() => return Ok(response),
            None if response.is_object() => {
                return Err(format!(
                    "paginate: {method:?} returned a non-paginated dict after {} accumulated rows; aborting",
                    out.len()
                ));
            }
            None => {
                return Err(format!(
                    "paginate: {method:?} returned non-list/non-dict shape after {} accumulated rows; aborting",
                    out.len()
                ));
            }
        };

        if prev_page.as_ref() == Some(&page) {
            return Err(format!(
                "paginate: {method:?} returned identical pages — method likely ignores offset; aborting after {} rows",
                out.len()
            ));
        }

        let page_len = page.len();
        out.extend(page.clone());
        if page_len < page_size {
            return Ok(Value::Array(out));
        }
        if out.len() >= RPC_PAGINATION_SAFETY_CAP {
            out.truncate(RPC_PAGINATION_SAFETY_CAP);
            return Ok(Value::Array(out));
        }
        prev_page = Some(page);
        offset += page_size;
    }
}

fn extract_page(response: &Value) -> Option<Vec<Value>> {
    if let Some(page) = response.as_array() {
        return Some(page.clone());
    }
    let object = response.as_object()?;
    for key in RPC_PAGE_LIST_KEYS {
        if let Some(page) = object.get(key).and_then(Value::as_array) {
            return Some(page.clone());
        }
    }
    None
}

fn run_find_pdfs_command(
    client: &mut impl RpcCaller,
    collection: String,
    limit: usize,
) -> Result<Value, String> {
    let collection_key = resolve_collection(client, &collection)?;
    let response = client.call(
        "collections.getItems",
        Some(serde_json::json!({"key": collection_key})),
    )?;
    let items = collection_items(&response);

    let mut missing = Vec::new();
    for item in &items {
        let Some(item_key) = item.get("key").and_then(Value::as_str) else {
            continue;
        };
        let attachments = client.call(
            "attachments.list",
            Some(serde_json::json!({"parentKey": item_key})),
        )?;
        if !has_pdf_attachment(&attachments) {
            missing.push(item.clone());
        }
        if limit > 0 && missing.len() >= limit {
            break;
        }
    }

    let mut results = Vec::new();
    for item in &missing {
        let item_key = item
            .get("key")
            .and_then(Value::as_str)
            .ok_or_else(|| "missing item lacks key".to_string())?;
        let response = client.call(
            "attachments.findPDF",
            Some(serde_json::json!({"parentKey": item_key})),
        )?;
        let attachment = response.get("attachment").filter(|value| !value.is_null());
        results.push(serde_json::json!({
            "item_key": item_key,
            "title": item.get("title").cloned().unwrap_or(Value::Null),
            "found": attachment.is_some(),
            "attachment_key": attachment
                .and_then(|attachment| attachment.get("key"))
                .cloned()
                .unwrap_or(Value::Null),
        }));
    }

    Ok(serde_json::json!({
        "scanned": items.len(),
        "attempted": missing.len(),
        "results": results,
    }))
}

pub(crate) fn collection_items(response: &Value) -> Vec<Value> {
    if let Some(items) = response.get("items").and_then(Value::as_array) {
        return items.clone();
    }
    response.as_array().cloned().unwrap_or_default()
}

fn has_pdf_attachment(attachments: &Value) -> bool {
    attachments
        .as_array()
        .is_some_and(|attachments| attachments.iter().any(is_pdf_attachment))
}

fn is_pdf_attachment(attachment: &Value) -> bool {
    let content_type = attachment
        .get("contentType")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let path = attachment
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    matches!(
        content_type.as_str(),
        "application/pdf" | "application/x-pdf"
    ) || path.ends_with(".pdf")
}

fn run_system_command(
    command: SystemCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        SystemCommand::Version { .. } => client.call("system.version", None)?,
        SystemCommand::Libraries { .. } => client.call("system.libraries", None)?,
        SystemCommand::LibraryStats { library, .. } => {
            let params = library.map(|id| serde_json::json!({"id": id}));
            client.call("system.libraryStats", params)?
        }
        SystemCommand::Schema { item_type, .. } => {
            if let Some(item_type) = item_type {
                let fields = client.call("system.itemFields", Some(serde_json::json!({"itemType": item_type})))?;
                let creators = client.call("system.creatorTypes", Some(serde_json::json!({"itemType": item_type})))?;
                let field_names: Vec<Value> = fields.as_array().unwrap_or(&vec![])
                    .iter()
                    .filter_map(|f| f.get("field").cloned())
                    .collect();
                let creator_names: Vec<Value> = creators.as_array().unwrap_or(&vec![])
                    .iter()
                    .filter_map(|c| c.get("creatorType").cloned())
                    .collect();
                serde_json::json!({
                    "itemType": item_type,
                    "fields": field_names,
                    "creatorTypes": creator_names,
                })
            } else {
                let types = client.call("system.itemTypes", None)?;
                let type_names: Vec<Value> = types.as_array().unwrap_or(&vec![])
                    .iter()
                    .filter_map(|t| t.get("itemType").cloned())
                    .collect();
                Value::Array(type_names)
            }
        }
        SystemCommand::CurrentCollection { .. } => client.call("system.currentCollection", None)?,
        SystemCommand::Methods { method, .. } => {
            if let Some(method) = method {
                client.call("system.describe", Some(serde_json::json!({"method": method})))?
            } else {
                client.call("system.listMethods", None)?
            }
        }
    };
    Ok(value)
}

fn run_items_command(
    command: ItemsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        ItemsCommand::Add {
            doi,
            isbn,
            from_url,
            file,
            item_type,
            fields,
            collection,
            dry_run,
            ..
        } => {
            if let Some(doi) = doi {
                run_add_identifier_command(client, "items.addByDOI", "doi", doi, collection, dry_run)?
            } else if let Some(isbn) = isbn {
                run_add_identifier_command(client, "items.addByISBN", "isbn", isbn, collection, dry_run)?
            } else if let Some(from_url) = from_url {
                run_add_identifier_command(client, "items.addByURL", "url", from_url, collection, dry_run)?
            } else if let Some(file) = file {
                let mut params = serde_json::json!({"path": zotero_path(&file)});
                maybe_insert_collection(client, &mut params, collection)?;
                run_mutation_command(client, "items.addFromFile", params, dry_run)?
            } else if let Some(item_type) = item_type {
                let parsed_fields = parse_field_options(&fields)?;
                let mut params = serde_json::json!({"itemType": item_type});
                if !parsed_fields.is_empty() {
                    if let Some(map) = params.as_object_mut() {
                        map.insert("fields".to_string(), Value::Object(parsed_fields));
                    }
                }
                run_mutation_command(client, "items.create", params, dry_run)?
            } else {
                return Err("INVALID_ARGS: provide one of --doi, --isbn, --from-url, --file, or --type".into());
            }
        }
        ItemsCommand::Update {
            key,
            fields,
            dry_run,
            ..
        } => {
            let parsed_fields = parse_field_options(&fields)?;
            let mut params = serde_json::json!({"key": key});
            if !parsed_fields.is_empty() {
                if let Some(map) = params.as_object_mut() {
                    map.insert("fields".to_string(), Value::Object(parsed_fields));
                }
            }
            run_mutation_command(client, "items.update", params, dry_run)?
        }
        ItemsCommand::Delete { key, dry_run, .. } => run_mutation_command(
            client,
            "items.delete",
            serde_json::json!({"key": key}),
            dry_run,
        )?,
        ItemsCommand::Trash {
            items, dry_run, ..
        } => {
            if items.len() == 1 {
                run_mutation_command(
                    client,
                    "items.trash",
                    serde_json::json!({"key": items[0]}),
                    dry_run,
                )?
            } else {
                run_mutation_command(
                    client,
                    "items.batchTrash",
                    serde_json::json!({"keys": items}),
                    dry_run,
                )?
            }
        }
        ItemsCommand::Restore { item, dry_run, .. } => run_mutation_command(
            client,
            "items.restore",
            serde_json::json!({"key": item}),
            dry_run,
        )?,
        ItemsCommand::MergeDuplicates { keys, dry_run, .. } => {
            if keys.len() < 2 {
                return Err("INVALID_ARGS: need at least 2 keys to merge".to_string());
            }
            run_mutation_command(
                client,
                "items.mergeDuplicates",
                serde_json::json!({"keys": keys}),
                dry_run,
            )?
        }
        ItemsCommand::AddRelated {
            key,
            target,
            dry_run,
            ..
        } => run_mutation_command(
            client,
            "items.addRelated",
            serde_json::json!({"key": key, "targetKey": target}),
            dry_run,
        )?,
        ItemsCommand::RemoveRelated {
            key,
            target,
            dry_run,
            ..
        } => run_mutation_command(
            client,
            "items.removeRelated",
            serde_json::json!({"key": key, "targetKey": target}),
            dry_run,
        )?,
        ItemsCommand::Get { item, .. } => client.call("items.get", Some(serde_json::json!({"key": item})))?,
        ItemsCommand::List {
            limit,
            offset,
            sort,
            direction,
            trash,
            ..
        } => {
            if trash {
                let value = client.call(
                    "items.getTrash",
                    Some(serde_json::json!({"limit": limit, "offset": offset})),
                )?;
                normalize_list_envelope(value, "items", Some(limit), offset)
            } else {
                let mut params = serde_json::json!({
                    "limit": limit,
                    "offset": offset,
                    "direction": direction,
                });
                if let (Some(sort), Some(map)) = (sort, params.as_object_mut()) {
                    map.insert("sort".to_string(), Value::String(sort));
                }
                let value = client.call("items.list", Some(params))?;
                normalize_list_envelope(value, "items", Some(limit), offset)
            }
        }
        ItemsCommand::FindDuplicates { .. } => client.call("items.findDuplicates", None)?,
        ItemsCommand::Recent {
            limit,
            offset,
            recent_type,
            ..
        } => {
            if recent_type != "added" && recent_type != "modified" {
                return Err(format!(
                    "--type must be added or modified, got {recent_type:?}"
                ));
            }
            let value = client.call(
                "items.getRecent",
                Some(
                    serde_json::json!({"limit": limit, "offset": offset, "type": recent_type}),
                ),
            )?;
            normalize_list_envelope(value, "items", Some(limit), offset)
        }
        ItemsCommand::Fulltext { key, .. } => client.call("items.getFullText", Some(serde_json::json!({"key": key})))?,
        ItemsCommand::Related { key, .. } => normalize_list_envelope(
            client.call("items.getRelated", Some(serde_json::json!({"key": key})))?,
            "items",
            None,
            0,
        ),
        ItemsCommand::CitationKey { key, .. } => client.call("items.citationKey", Some(serde_json::json!({"key": key})))?,
        ItemsCommand::Path { key, .. } => localize_attachment_path_response(
            client.call("attachments.getPath", Some(serde_json::json!({"key": key})))?,
        ),
        ItemsCommand::Attachments { key, offset, .. } => {
            let value = client.call(
                "attachments.list",
                Some(serde_json::json!({"parentKey": key})),
            )?;
            let total = value
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len()) as u64;
            normalize_list_envelope(value, "items", Some(total), offset)
        }
        ItemsCommand::FindPdfs { collection, limit, .. } => {
            run_find_pdfs_command(client, collection, limit)?
        }
    };
    Ok(value)
}

fn run_add_identifier_command(
    client: &mut impl RpcCaller,
    method: &str,
    param_name: &str,
    param_value: String,
    collection: Option<String>,
    dry_run: bool,
) -> Result<Value, String> {
    let mut params = Value::Object(serde_json::Map::from_iter([(
        param_name.to_string(),
        Value::String(param_value),
    )]));
    maybe_insert_collection(client, &mut params, collection)?;
    run_mutation_command(client, method, params, dry_run)
}

fn run_mutation_command(
    client: &mut impl RpcCaller,
    method: &str,
    params: Value,
    dry_run: bool,
) -> Result<Value, String> {
    let value = if dry_run {
        serde_json::json!({
            "ok": true,
            "dryRun": true,
            "wouldCall": method,
            "wouldCallParams": params,
        })
    } else {
        client.call(method, Some(params))?
    };
    Ok(value)
}

fn parse_field_options(fields: &[String]) -> Result<serde_json::Map<String, Value>, String> {
    let mut parsed = serde_json::Map::new();
    for field in fields {
        let (key, value) = field
            .split_once('=')
            .ok_or_else(|| format!("INVALID_ARGS: --field must be key=value, got: {field:?}"))?;
        parsed.insert(key.to_string(), Value::String(value.to_string()));
    }
    Ok(parsed)
}

fn maybe_insert_collection(
    client: &mut impl RpcCaller,
    params: &mut Value,
    collection: Option<String>,
) -> Result<(), String> {
    let Some(collection) = collection else {
        return Ok(());
    };
    let collection = resolve_collection(client, &collection)?;
    let include = match &collection {
        Value::Null => false,
        Value::Number(number) => number.as_i64() != Some(0),
        _ => true,
    };
    if include {
        params
            .as_object_mut()
            .expect("mutation params are always objects")
            .insert("collection".to_string(), collection);
    }
    Ok(())
}

fn run_settings_command(
    command: SettingsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        SettingsCommand::Get { key, .. } => client.call("settings.get", Some(serde_json::json!({"key": key})))?,
        SettingsCommand::List { .. } => client.call("settings.getAll", None)?,
        SettingsCommand::Set {
            pairs,
            file,
            dry_run,
            ..
        } => {
            if let Some(file) = file {
                // --file mode: read JSON and call settings.setAll
                let raw = fs::read_to_string(&file)
                    .map_err(|err| format!("INVALID_JSON: Could not read JSON: {err}"))?;
                let settings: Value = serde_json::from_str(&raw)
                    .map_err(|err| format!("INVALID_JSON: Could not parse JSON: {err}"))?;
                if dry_run {

                        dry_run_value("settings.setAll", settings)
                } else {

                        client.call("settings.setAll", Some(settings))?
                }
            } else if pairs.len() == 2 {
                // Single key=value: settings.set
                let key = &pairs[0];
                let value = &pairs[1];
                let parsed_value = serde_json::from_str::<Value>(value)
                    .unwrap_or(Value::String(value.clone()));
                let params = serde_json::json!({"key": key, "value": parsed_value});
                if dry_run {

                        dry_run_value("settings.set", params)
                } else {

                        client.call("settings.set", Some(params))?
                }
            } else if pairs.len() > 2 && pairs.len() % 2 == 0 {
                // Multiple pairs: build a map and call settings.setAll
                let mut map = serde_json::Map::new();
                for chunk in pairs.chunks(2) {
                    let parsed = serde_json::from_str::<Value>(&chunk[1])
                        .unwrap_or(Value::String(chunk[1].clone()));
                    map.insert(chunk[0].clone(), parsed);
                }
                let settings = Value::Object(map);
                if dry_run {

                        dry_run_value("settings.setAll", settings)
                } else {

                        client.call("settings.setAll", Some(settings))?
                }
            } else {
                return Err(
                    "INVALID_ARGS: provide key value pairs (even number of args) or --file".into(),
                );
            }
        }
    };
    Ok(value)
}

fn run_tags_command(
    command: TagsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        TagsCommand::List { limit, .. } => {
            let value = client.call("tags.list", Some(serde_json::json!({"limit": limit})))?;
            normalize_list_envelope(value, "items", Some(limit), 0)
        }
        TagsCommand::Rename {
            old, new, dry_run, ..
        } => run_tag_mutation(
            client,
            "tags.rename",
            serde_json::json!({"oldName": old, "newName": new}),
            dry_run,
        )?,
        TagsCommand::Delete { tag, dry_run, .. } => run_tag_mutation(
            client,
            "tags.delete",
            serde_json::json!({"tag": tag}),
            dry_run,
        )?,
        TagsCommand::Add {
            keys, tags, dry_run, ..
        } => {
            if keys.len() == 1 {
                run_tag_mutation(
                    client,
                    "tags.add",
                    serde_json::json!({"key": keys[0], "tags": tags}),
                    dry_run,
                )?
            } else {
                run_tag_mutation(
                    client,
                    "tags.batchUpdate",
                    serde_json::json!({"keys": keys, "add": tags}),
                    dry_run,
                )?
            }
        }
        TagsCommand::Remove {
            keys, tags, dry_run, ..
        } => {
            if keys.len() == 1 {
                run_tag_mutation(
                    client,
                    "tags.remove",
                    serde_json::json!({"key": keys[0], "tags": tags}),
                    dry_run,
                )?
            } else {
                run_tag_mutation(
                    client,
                    "tags.batchUpdate",
                    serde_json::json!({"keys": keys, "remove": tags}),
                    dry_run,
                )?
            }
        }
    };
    Ok(value)
}

fn run_tag_mutation(
    client: &mut impl RpcCaller,
    method: &str,
    params: Value,
    dry_run: bool,
) -> Result<Value, String> {
    if dry_run {
        Ok(dry_run_value(method, params))
    } else {
        Ok(client.call(method, Some(params))?)
    }
}

fn dry_run_value(method: &str, params: Value) -> Value {
    serde_json::json!({
        "ok": true,
        "dryRun": true,
        "wouldCall": method,
        "wouldCallParams": params,
    })
}

fn run_annotations_command(
    command: AnnotationsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        AnnotationsCommand::List { parent, attachment, context, .. } => {
            let mut params = serde_json::json!({"parentKey": parent});
            if let Some(att) = attachment {
                params["attachmentKey"] = Value::String(att);
            }
            if let Some(ctx) = context {
                params["context"] = Value::Number(ctx.into());
            }
            let value = client.call("annotations.list", Some(params))?;
            let total = value
                .get("items")
                .and_then(Value::as_array)
                .map_or(0, |a| a.len()) as u64;
            normalize_list_envelope(value, "items", Some(total), 0)
        }
        AnnotationsCommand::Create {
            parent,
            attachment,
            annotation_type,
            position,
            quote,
            page,
            sort_index,
            text,
            comment,
            color,
            dry_run,
            ..
        } => {
            let annotation_type = annotation_type.unwrap_or_else(|| "highlight".to_string());
            if !matches!(
                annotation_type.as_str(),
                "highlight" | "note" | "underline" | "image" | "ink"
            ) {
                return Err(format!(
                    "INVALID_ARGS: --type must be highlight|note|underline|image|ink, got {annotation_type:?}"
                ));
            }
            let mut params = serde_json::Map::new();
            params.insert("parentKey".to_string(), Value::String(parent));
            if let Some(att) = attachment {
                params.insert("attachmentKey".to_string(), Value::String(att));
            }
            params.insert("type".to_string(), Value::String(annotation_type.clone()));
            params.insert("color".to_string(), Value::String(color));

            if let Some(ref quote_text) = quote {
                if !matches!(annotation_type.as_str(), "highlight" | "underline") {
                    return Err(format!(
                        "INVALID_ARGS: --quote is only valid for highlight|underline, got {annotation_type:?}"
                    ));
                }
                params.insert("quote".to_string(), Value::String(quote_text.clone()));
                if let Some(page_idx) = page {
                    params.insert(
                        "pageIndex".to_string(),
                        Value::Number(page_idx.into()),
                    );
                }
                // When --quote is given, --position is optional
                if let Some(raw) = position {
                    let pos = serde_json::from_str::<Value>(&raw)
                        .map_err(|err| format!("INVALID_JSON: Could not parse --position: {err}"))?;
                    validate_annotation_position(annotation_type.as_str(), &pos)?;
                    params.insert("position".to_string(), pos);
                }
            } else {
                let position = position
                    .ok_or_else(|| "INVALID_ARGS: --position JSON is required (or use --quote)".to_string())
                    .and_then(|raw| {
                        serde_json::from_str::<Value>(&raw)
                            .map_err(|err| format!("INVALID_JSON: Could not parse --position: {err}"))
                    })?;
                validate_annotation_position(annotation_type.as_str(), &position)?;
                params.insert("position".to_string(), position);
            }

            if let Some(sort_index) = sort_index {
                params.insert(
                    "sortIndex".to_string(),
                    parse_annotation_sort_index(sort_index)?,
                );
            }
            if let Some(text) = text {
                params.insert("text".to_string(), Value::String(text));
            }
            if let Some(comment) = comment {
                params.insert("comment".to_string(), Value::String(comment));
            }
            run_mutating_command(client, "annotations.create", Value::Object(params), dry_run)?
        }
        AnnotationsCommand::CreateBatch {
            parent,
            attachment,
            file,
            dry_run,
            ..
        } => {
            let input = if let Some(ref path) = file {
                std::fs::read_to_string(path)
                    .map_err(|e| format!("INVALID_ARGS: cannot read file {path}: {e}"))?
            } else {
                let mut buf = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)
                    .map_err(|e| format!("INVALID_ARGS: cannot read stdin: {e}"))?;
                buf
            };
            let annotations: Value = serde_json::from_str(&input)
                .map_err(|e| format!("INVALID_JSON: {e}"))?;
            if !annotations.is_array() {
                return Err("INVALID_ARGS: input must be a JSON array".to_string());
            }
            let mut params = serde_json::json!({
                "parentKey": parent,
                "annotations": annotations,
            });
            if let Some(att) = attachment {
                params["attachmentKey"] = Value::String(att);
            }
            run_mutating_command(client, "annotations.createBatch", params, dry_run)?
        }
        AnnotationsCommand::Locate {
            parent,
            attachment,
            quote,
            page,
            ..
        } => {
            let mut params = serde_json::json!({
                "parentKey": parent,
                "quote": quote,
            });
            if let Some(att) = attachment {
                params["attachmentKey"] = Value::String(att);
            }
            if let Some(page_idx) = page {
                params["pageIndex"] = Value::Number(page_idx.into());
            }
            client.call("annotations.locate", Some(params))?
        }
        AnnotationsCommand::Delete {
            annotation_key,
            dry_run,
            ..
        } => run_mutating_command(
            client,
            "annotations.delete",
            serde_json::json!({"key": annotation_key}),
            dry_run,
        )?,
    };
    Ok(value)
}

fn validate_annotation_position(annotation_type: &str, position: &Value) -> Result<(), String> {
    position
        .get("pageIndex")
        .and_then(Value::as_i64)
        .filter(|value| *value >= 0)
        .ok_or_else(|| {
            "INVALID_ARGS: --position must include a non-negative integer pageIndex".to_string()
        })?;

    if annotation_type == "ink" {
        let has_paths = position
            .get("paths")
            .and_then(Value::as_array)
            .is_some_and(|paths| !paths.is_empty());
        if !has_paths {
            return Err("INVALID_ARGS: ink --position must include non-empty paths".to_string());
        }
        return Ok(());
    }

    let valid_rects = position
        .get("rects")
        .and_then(Value::as_array)
        .is_some_and(|rects| !rects.is_empty() && rects.iter().all(is_annotation_rect));
    if !valid_rects {
        return Err(
            "INVALID_ARGS: --position must include non-empty rects of [x1, y1, x2, y2]".to_string(),
        );
    }
    Ok(())
}

fn is_annotation_rect(value: &Value) -> bool {
    value.as_array().is_some_and(|coords| {
        coords.len() == 4
            && coords
                .iter()
                .all(|coord| coord.as_f64().is_some_and(f64::is_finite))
    })
}

fn parse_annotation_sort_index(raw: String) -> Result<Value, String> {
    let parsed = serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| Value::String(raw));
    let valid = match &parsed {
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        Value::String(value) => {
            is_zotero_pdf_sort_index(value.trim())
                || (!value.trim().is_empty()
                    && value.trim().parse::<f64>().is_ok_and(f64::is_finite))
        }
        _ => false,
    };
    if valid {
        Ok(parsed)
    } else {
        Err(format!(
            "INVALID_ARGS: --sort-index must be a finite number or numeric string, got {parsed}"
        ))
    }
}

fn is_zotero_pdf_sort_index(value: &str) -> bool {
    let mut parts = value.split('|');
    matches!(
        (parts.next(), parts.next(), parts.next(), parts.next()),
        (Some(page), Some(offset), Some(y), None)
            if page.len() == 5
                && offset.len() == 6
                && y.len() == 5
                && page.chars().all(|ch| ch.is_ascii_digit())
                && offset.chars().all(|ch| ch.is_ascii_digit())
                && y.chars().all(|ch| ch.is_ascii_digit())
    )
}


fn localize_attachment_path_response(mut value: Value) -> Value {
    if let Some(path) = value.get("path").and_then(Value::as_str) {
        let local = local_path_from_zotero_path(path);
        if let Some(map) = value.as_object_mut() {
            map.insert("path".to_string(), Value::String(local));
        }
    }
    value
}

fn run_notes_command(
    command: NotesCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        NotesCommand::List {
            parent,
            limit,
            offset,
            ..
        } => {
            let value = client.call(
                "notes.list",
                Some(serde_json::json!({"parentKey": parent})),
            )?;
            normalize_list_envelope(value, "items", Some(limit), offset)
        }
        NotesCommand::Get { note_key, .. } => {
            client.call("notes.get", Some(serde_json::json!({"key": note_key})))?
        }
        NotesCommand::Create {
            parent,
            content,
            tags,
            dry_run,
            ..
        } => {
            let mut params = serde_json::Map::new();
            params.insert("parentKey".to_string(), Value::String(parent));
            params.insert("content".to_string(), Value::String(content));
            if !tags.is_empty() {
                params.insert(
                    "tags".to_string(),
                    Value::Array(tags.into_iter().map(Value::String).collect()),
                );
            }
            run_mutating_command(client, "notes.create", Value::Object(params), dry_run)?
        }
        NotesCommand::Update {
            note_key,
            content,
            dry_run,
            ..
        } => run_mutating_command(
            client,
            "notes.update",
            serde_json::json!({"key": note_key, "content": content}),
            dry_run,
        )?,
        NotesCommand::Delete {
            note_key, dry_run, ..
        } => {
            // Python CLI intentionally routes note deletion through items.delete.
            run_mutating_command(
                client,
                "items.delete",
                serde_json::json!({"key": note_key}),
                dry_run,
            )?
        }
        NotesCommand::Search { query, limit, .. } => {
            let value = client.call(
                "notes.search",
                Some(serde_json::json!({"query": query, "limit": limit})),
            )?;
            normalize_list_envelope(value, "items", Some(limit), 0)
        }
    };
    Ok(value)
}

fn run_mutating_command(
    client: &mut impl RpcCaller,
    method: &str,
    params: Value,
    dry_run: bool,
) -> Result<Value, String> {
    if dry_run {
        Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "wouldCall": method,
            "wouldCallParams": params,
        }))
    } else {
        client.call(method, Some(params))
    }
}

fn run_collections_command(
    command: CollectionsCommand,
    client: &mut impl RpcCaller,
) -> Result<Value, String> {
    let value = match command {
        CollectionsCommand::List { .. } => normalize_list_envelope(
            client.call("collections.list", None)?,
            "items",
            None,
            0,
        ),
        CollectionsCommand::Tree { .. } => client.call("collections.tree", None)?,
        CollectionsCommand::Get { name_or_id, .. } => {
            let key = resolve_collection(client, &name_or_id)?;
            client.call("collections.get", Some(serde_json::json!({"key": key})))?
        }
        CollectionsCommand::GetItems {
            name_or_id,
            limit,
            offset,
            ..
        } => {
            let key = resolve_collection(client, &name_or_id)?;
            let mut params = serde_json::json!({"key": key});
            if let Some(map) = params.as_object_mut() {
                if let Some(limit) = limit {
                    map.insert("limit".to_string(), Value::Number(limit.into()));
                }
                if offset > 0 {
                    map.insert("offset".to_string(), Value::Number(offset.into()));
                }
            }
            normalize_list_envelope(
                client.call("collections.getItems", Some(params))?,
                "items",
                limit,
                offset,
            )
        }
        CollectionsCommand::Stats { name_or_id, .. } => {
            let key = resolve_collection(client, &name_or_id)?;
            client.call("collections.stats", Some(serde_json::json!({"key": key})))?
        }
        CollectionsCommand::Rename {
            old_name,
            new_name,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &old_name, "rename")?;
            let params = serde_json::json!({"key": key, "name": new_name});
            if dry_run {
                return Ok(dry_run_value("collections.rename", params));
            }
            return client.call("collections.rename", Some(params));
        }
        CollectionsCommand::Create {
            name,
            parent,
            dry_run,
            ..
        } => {
            let mut params = serde_json::json!({"name": name});
            if let Some(parent) = parent {
                let parent_key = resolve_mutable_collection(client, &parent, "use as parent")?;
                if let Some(map) = params.as_object_mut() {
                    map.insert("parentKey".to_string(), parent_key);
                }
            }
            if dry_run {
                return Ok(dry_run_value("collections.create", params));
            }
            return client.call("collections.create", Some(params));
        }
        CollectionsCommand::Delete {
            name_or_id,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &name_or_id, "delete")?;
            let params = serde_json::json!({"key": key});
            if dry_run {
                return Ok(dry_run_value("collections.delete", params));
            }
            return client.call("collections.delete", Some(params));
        }
        CollectionsCommand::AddItems {
            collection,
            item_keys,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &collection, "add to")?;
            let params = serde_json::json!({"key": key, "keys": item_keys});
            if dry_run {
                return Ok(dry_run_value("collections.addItems", params));
            }
            return client.call("collections.addItems", Some(params));
        }
        CollectionsCommand::RemoveItems {
            collection,
            item_keys,
            dry_run,
            ..
        } => {
            let key = resolve_mutable_collection(client, &collection, "operate on")?;
            let params = serde_json::json!({"key": key, "keys": item_keys});
            if dry_run {
                return Ok(dry_run_value("collections.removeItems", params));
            }
            return client.call("collections.removeItems", Some(params));
        }
    };
    Ok(value)
}

fn resolve_export_keys(
    client: &mut impl RpcCaller,
    mut keys: Vec<String>,
    collection: Option<String>,
) -> Result<Vec<String>, String> {
    if let Some(name) = collection {
        let col_key = resolve_collection(client, &name)?;
        let response = client.call(
            "collections.getItems",
            Some(serde_json::json!({"key": col_key})),
        )?;
        let items = collection_items(&response);
        for item in items {
            if let Some(key) = item.get("key").and_then(Value::as_str) {
                if !keys.contains(&key.to_string()) {
                    keys.push(key.to_string());
                }
            }
        }
    }
    if keys.is_empty() {
        return Err("No item keys provided. Pass positional keys and/or --collection.".to_string());
    }
    Ok(keys)
}

fn run_export(args: ExportArgs, client: &mut impl RpcCaller) -> Result<String, String> {
    let keys = resolve_export_keys(client, args.keys, args.collection)?;
    match args.format.as_str() {
        "bibtex" => run_export_content_command(client, "export.bibtex", keys),
        "ris" => run_export_content_command(client, "export.ris", keys),
        "csl-json" => {
            let response =
                client.call("export.cslJson", Some(serde_json::json!({"keys": keys})))?;
            if let Some(content) = response.get("content") {
                format_json(content)
            } else {
                format_json(&response)
            }
        }
        "bibliography" => {
            let response = client.call(
                "export.bibliography",
                Some(serde_json::json!({"keys": keys, "style": args.style})),
            )?;
            if let Some(object) = response.as_object() {
                let field = if args.html { "html" } else { "text" };
                if object.contains_key("html") || object.contains_key("text") {
                    return raw_value_output(
                        object.get(field).unwrap_or(&Value::String(String::new())),
                    );
                }
            }
            format_json(&response)
        }
        other => Err(format!(
            "INVALID_ARGS: unknown format {other:?}, expected bibtex/ris/csl-json/bibliography"
        )),
    }
}

fn run_export_content_command(
    client: &mut impl RpcCaller,
    method: &str,
    keys: Vec<String>,
) -> Result<String, String> {
    let response = client.call(method, Some(serde_json::json!({"keys": keys})))?;
    if let Some(content) = response.get("content") {
        raw_value_output(content)
    } else {
        format_json(&response)
    }
}

pub(crate) fn resolve_collection(
    client: &mut impl RpcCaller,
    name_or_id: &str,
) -> Result<Value, String> {
    let trimmed = name_or_id.trim();
    if let Ok(id) = trimmed.parse::<i64>() {
        return Ok(Value::Number(id.into()));
    }

    let collections = client.call("collections.list", None)?;
    let items = collections
        .get("items")
        .and_then(Value::as_array)
        .or_else(|| collections.as_array())
        .ok_or_else(|| "collections.list returned non-array result".to_string())?;

    if let Some(collection) = items
        .iter()
        .find(|collection| collection.get("key").and_then(Value::as_str) == Some(trimmed))
    {
        return collection_key(collection);
    }

    let exact = items
        .iter()
        .filter(|collection| collection.get("name").and_then(Value::as_str) == Some(trimmed))
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return collection_key(exact[0]);
    }

    let needle = normalize_collection_name(trimmed);
    let fuzzy = items
        .iter()
        .filter(|collection| {
            collection
                .get("name")
                .and_then(Value::as_str)
                .map(normalize_collection_name)
                .is_some_and(|name| name.contains(&needle))
        })
        .collect::<Vec<_>>();

    match fuzzy.len() {
        1 => collection_key(fuzzy[0]),
        0 => Err(format!(
            "COLLECTION_NOT_FOUND: No collection named {trimmed:?}"
        )),
        _ => Err(format!(
            "COLLECTION_AMBIGUOUS: Multiple collections match {trimmed:?}"
        )),
    }
}

fn collection_key(collection: &Value) -> Result<Value, String> {
    collection
        .get("key")
        .cloned()
        .ok_or_else(|| "collection result is missing key".to_string())
}

fn resolve_mutable_collection(
    client: &mut impl RpcCaller,
    name_or_id: &str,
    operation: &str,
) -> Result<Value, String> {
    let key = resolve_collection(client, name_or_id)?;
    if key.as_i64() == Some(0) {
        return Err(format!(
            "COLLECTION_NOT_FOUND: {name_or_id:?} resolved to library root (cannot {operation})"
        ));
    }
    Ok(key)
}

fn normalize_collection_name(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
