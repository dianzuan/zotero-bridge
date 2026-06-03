//! Block grouping into retrieval chunks and block-type predicates.

use crate::evidence::{PdfEvidenceBlock, StructureChunk};

pub fn chunks_from_blocks(blocks: &[PdfEvidenceBlock], max_chars: usize) -> Vec<StructureChunk> {
    let mut chunks = Vec::<StructureChunk>::new();
    let mut current = Vec::<PdfEvidenceBlock>::new();
    let mut current_chars = 0usize;
    let mut current_section = Vec::<String>::new();
    let attachment_key = blocks
        .first()
        .map(|block| block.attachment_key.clone())
        .unwrap_or_default();

    for block in blocks {
        // Running page furniture (headers/footers/page numbers/equation numbers)
        // is emitted as its own block by Paddle and MinerU and repeats on every
        // page ("downloaded from …", "GARY S. BECKER", "9", "(1)"). It carries no
        // retrieval value and pollutes chunk embeddings, so skip it here. The
        // block still lives in the Blocks sidecar for provenance; only chunking
        // and embedding drop it. Academic footnotes are NOT furniture — they hold
        // real content — and are deliberately excluded from this set.
        if is_furniture_type(&block.block_type) {
            continue;
        }

        if is_heading_type(&block.block_type) {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
            current_section = vec![block.text.clone()];
            continue;
        }

        let block_section = if block.section_path.is_empty() {
            current_section.clone()
        } else {
            block.section_path.clone()
        };

        if !current.is_empty() && !same_section(&current[0].section_path, &block_section) {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
        }

        let mut block = block.clone();
        block.section_path = block_section;

        if is_table_type(&block.block_type) || is_figure_type(&block.block_type) {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
            chunks.push(StructureChunk::from_blocks(
                format!("{attachment_key}:c{}", chunks.len()),
                std::slice::from_ref(&block),
            ));
            continue;
        }

        let block_chars = block.text.chars().count();
        let joined_chars = current_chars + if current.is_empty() { 0 } else { 2 } + block_chars;
        if max_chars > 0 && !current.is_empty() && joined_chars > max_chars {
            flush_chunk(&mut chunks, &mut current, &attachment_key);
            current_chars = 0;
        }

        current_chars += if current.is_empty() { 0 } else { 2 } + block_chars;
        current.push(block);
    }

    flush_chunk(&mut chunks, &mut current, &attachment_key);

    // Figure/image blocks (and other text-less blocks) yield chunks with empty
    // text. They have no retrieval value, inflate chunk counts, and break strict
    // embedding providers that reject blank inputs — drop them at the source.
    chunks.retain(|chunk| !chunk.text.trim().is_empty());
    chunks
}

fn flush_chunk(
    chunks: &mut Vec<StructureChunk>,
    current: &mut Vec<PdfEvidenceBlock>,
    attachment_key: &str,
) {
    if current.is_empty() {
        return;
    }
    let chunk_key = format!("{attachment_key}:c{}", chunks.len());
    chunks.push(StructureChunk::from_blocks(chunk_key, current));
    current.clear();
}

pub(crate) fn is_heading_type(block_type: &str) -> bool {
    matches!(
        block_type,
        "heading" | "title" | "doc_title" | "paragraph_title" | "section"
    )
}

pub(crate) fn normalize_block_type(block_type: &str) -> &str {
    match block_type {
        "title" | "doc_title" | "paragraph_title" | "heading" | "section" => "heading",
        "text" => "paragraph",
        "table" | "table_body" | "table_caption" | "table_footnote" => "table",
        "image" | "figure" | "image_caption" | "chart" => "figure",
        "formula" | "display_formula" | "inline_formula" => "formula",
        other => other,
    }
}

pub(crate) fn is_table_type(block_type: &str) -> bool {
    normalize_block_type(block_type) == "table"
}

/// Running page furniture shared across providers: GLM folds these into body
/// text, but Paddle (`header`/`footer`/`number`/`formula_number`) and MinerU
/// (`page_header`/`page_footer`/`page_number`) emit them as standalone blocks
/// that repeat on every page. `footnote`/`page_footnote` are intentionally
/// absent — academic footnotes carry real content.
fn is_furniture_type(block_type: &str) -> bool {
    matches!(
        block_type,
        "header"
            | "footer"
            | "page_header"
            | "page_footer"
            | "page_number"
            | "number"
            | "formula_number"
    )
}

fn is_figure_type(block_type: &str) -> bool {
    normalize_block_type(block_type) == "figure"
}

fn same_section(a: &[String], b: &[String]) -> bool {
    a == b
}
