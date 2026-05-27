use zotron_types::{score_floor_filter, gap_cutoff, token_budget_filter, min_max_k_clamp, mmr_select, PdfEvidenceBlock, chunks_from_blocks};
use std::collections::HashMap;

// === Dynamic Cutoff Tests ===

#[test]
fn score_floor_drops_low_scores() {
    let input = vec![(0, 0.9), (1, 0.5), (2, 0.08), (3, 0.02)];
    let result = score_floor_filter(&input, 0.1);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, 0);
    assert_eq!(result[1].0, 1);
}

#[test]
fn score_floor_keeps_all_when_above() {
    let input = vec![(0, 0.9), (1, 0.5)];
    let result = score_floor_filter(&input, 0.1);
    assert_eq!(result.len(), 2);
}

#[test]
fn gap_cutoff_cuts_at_largest_gap() {
    let input = vec![(0, 0.92), (1, 0.88), (2, 0.86), (3, 0.48), (4, 0.30)];
    let result = gap_cutoff(&input, 0.15);
    assert_eq!(result.len(), 3);
}

#[test]
fn gap_cutoff_flat_distribution_keeps_all() {
    let input = vec![(0, 0.80), (1, 0.78), (2, 0.76), (3, 0.74)];
    let result = gap_cutoff(&input, 0.15);
    assert_eq!(result.len(), 4);
}

#[test]
fn token_budget_stops_at_limit() {
    let texts = vec!["a".repeat(900), "b".repeat(900), "c".repeat(900)];
    let input: Vec<(usize, f64)> = vec![(0, 0.9), (1, 0.8), (2, 0.7)];
    let result = token_budget_filter(&input, &texts, 650);
    assert_eq!(result.len(), 2);
}

#[test]
fn min_max_k_clamp_enforces_bounds() {
    let short: Vec<(usize, f64)> = vec![(0, 0.9)];
    let long: Vec<(usize, f64)> = (0..25).map(|i| (i, 0.9 - i as f64 * 0.01)).collect();

    let result_min = min_max_k_clamp(short, 3, 20);
    assert_eq!(result_min.len(), 1);

    let result_max = min_max_k_clamp(long, 3, 20);
    assert_eq!(result_max.len(), 20);
}

// === MMR Tests ===

#[test]
fn mmr_removes_near_duplicate_chunks() {
    let ranked = vec![(0, 0.9), (1, 0.85), (2, 0.7)];
    let vectors: HashMap<usize, Vec<f64>> = [
        (0, vec![1.0, 0.0, 0.0]),
        (1, vec![0.99, 0.01, 0.0]),
        (2, vec![0.0, 1.0, 0.0]),
    ].into_iter().collect();

    let result = mmr_select(&ranked, &vectors, 0.7, 0.05);
    assert!(result.iter().any(|(idx, _)| *idx == 0));
    assert!(result.iter().any(|(idx, _)| *idx == 2));
}

#[test]
fn mmr_keeps_all_when_diverse() {
    let ranked = vec![(0, 0.9), (1, 0.8), (2, 0.7)];
    let vectors: HashMap<usize, Vec<f64>> = [
        (0, vec![1.0, 0.0, 0.0]),
        (1, vec![0.0, 1.0, 0.0]),
        (2, vec![0.0, 0.0, 1.0]),
    ].into_iter().collect();

    let result = mmr_select(&ranked, &vectors, 0.7, 0.05);
    assert_eq!(result.len(), 3);
}

#[test]
fn mmr_retains_chunks_without_vectors() {
    let ranked = vec![(0, 0.9), (1, 0.8)];
    let vectors: HashMap<usize, Vec<f64>> = [
        (0, vec![1.0, 0.0]),
    ].into_iter().collect();

    let result = mmr_select(&ranked, &vectors, 0.7, 0.05);
    assert_eq!(result.len(), 2);
}

// === Chunking Enhancement Tests ===

fn make_block(block_type: &str, text: &str, section_path: Vec<String>) -> PdfEvidenceBlock {
    PdfEvidenceBlock {
        block_key: String::new(),
        item_key: "ITEM".into(),
        attachment_key: "ATT".into(),
        page_idx: 0,
        block_type: block_type.into(),
        bbox: None,
        section_path,
        text: text.into(),
    }
}

#[test]
fn chunks_prepend_section_path_to_text() {
    let blocks = vec![
        make_block("heading", "3. Methods", vec!["3. Methods".into()]),
        make_block("paragraph", "We used logistic regression.", vec!["3. Methods".into()]),
    ];
    let chunks = chunks_from_blocks(&blocks, 1200);
    assert_eq!(chunks.len(), 1);
    assert!(
        chunks[0].text.starts_with("3. Methods: "),
        "chunk text should start with section path prefix, got: {}",
        &chunks[0].text[..50.min(chunks[0].text.len())]
    );
    assert!(chunks[0].text.contains("We used logistic regression."));
}

#[test]
fn chunks_with_nested_section_path_use_separator() {
    let blocks = vec![
        make_block("heading", "3.2 Data", vec!["3. Methods".into(), "3.2 Data".into()]),
        make_block("paragraph", "Collected from surveys.", vec!["3. Methods".into(), "3.2 Data".into()]),
    ];
    let chunks = chunks_from_blocks(&blocks, 1200);
    assert_eq!(chunks.len(), 1);
    assert!(chunks[0].text.starts_with("3. Methods > 3.2 Data: "));
}

#[test]
fn chunks_without_section_path_have_no_prefix() {
    let blocks = vec![
        make_block("paragraph", "Abstract text here.", vec![]),
    ];
    let chunks = chunks_from_blocks(&blocks, 1200);
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].text, "Abstract text here.");
}

#[test]
fn figure_blocks_become_standalone_chunks() {
    let blocks = vec![
        make_block("heading", "Results", vec!["Results".into()]),
        make_block("paragraph", "Figure 1 shows the trend.", vec!["Results".into()]),
        make_block("image", "Distribution of employment rates across regions", vec!["Results".into()]),
        make_block("paragraph", "The data confirms our hypothesis.", vec!["Results".into()]),
    ];
    let chunks = chunks_from_blocks(&blocks, 1200);
    assert!(chunks.len() >= 2, "figure should cause chunk split, got {} chunks", chunks.len());
    let fig_chunk = chunks.iter().find(|c| c.text.contains("Distribution of employment")).unwrap();
    assert_eq!(fig_chunk.block_keys.len(), 1);
}
