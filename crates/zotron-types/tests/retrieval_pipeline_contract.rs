use zotron_types::{score_floor_filter, gap_cutoff, token_budget_filter, max_k_truncate, mmr_select, min_max_normalize, PdfEvidenceBlock, chunks_from_blocks};
use std::collections::HashMap;

// === min-max normalization tests ===

#[test]
fn min_max_normalize_empty_returns_empty() {
    let result = min_max_normalize(&[]);
    assert!(result.is_empty());
}

#[test]
fn min_max_normalize_single_element_maps_to_one() {
    let result = min_max_normalize(&[0.016]);
    assert_eq!(result, vec![1.0]);
}

#[test]
fn min_max_normalize_all_equal_avoids_division_by_zero() {
    // max == min: every element maps to 1.0, no NaN/inf.
    let result = min_max_normalize(&[0.5, 0.5, 0.5]);
    assert_eq!(result, vec![1.0, 1.0, 1.0]);
    assert!(result.iter().all(|v| v.is_finite()));
}

#[test]
fn min_max_normalize_linear_map() {
    // min -> 0, max -> 1, midpoint -> 0.5
    let result = min_max_normalize(&[0.0, 0.5, 1.0]);
    assert!((result[0] - 0.0).abs() < 1e-6);
    assert!((result[1] - 0.5).abs() < 1e-6);
    assert!((result[2] - 1.0).abs() < 1e-6);
}

#[test]
fn min_max_normalize_low_magnitude_rrf_scale() {
    // RRF-scale scores (~0.016) get stretched across [0,1].
    let result = min_max_normalize(&[0.016, 0.012, 0.008]);
    assert!((result[0] - 1.0).abs() < 1e-6, "max maps to 1.0");
    assert!((result[2] - 0.0).abs() < 1e-6, "min maps to 0.0");
    assert!((result[1] - 0.5).abs() < 1e-6, "midpoint maps to 0.5");
}

#[test]
fn min_max_normalize_nan_safe() {
    // A NaN must not poison min/max selection; finite values still normalize.
    let result = min_max_normalize(&[0.2, f32::NAN, 0.8]);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|v| v.is_finite()), "no NaN/inf in output: {result:?}");
}

#[test]
fn mmr_keeps_diverse_items_after_normalization() {
    // Simulates hybrid-no-reranker: low-magnitude RRF scores. Without
    // normalization, lambda*0.016 - (1-lambda)*sim is far below the 0.05 MMR
    // threshold and diverse items collapse to only the first (kept
    // unconditionally). After min-max normalization the top items map toward
    // 1.0 so diverse (orthogonal) candidates survive instead of collapsing.
    let raw_scores = [0.016f64, 0.014, 0.012];
    // Orthogonal vectors => maximally diverse, no redundancy to penalize.
    let v0: Vec<f64> = vec![1.0, 0.0, 0.0];
    let v1: Vec<f64> = vec![0.0, 1.0, 0.0];
    let v2: Vec<f64> = vec![0.0, 0.0, 1.0];
    let vectors: HashMap<usize, &[f64]> = [
        (0, v0.as_slice()), (1, v1.as_slice()), (2, v2.as_slice()),
    ].into_iter().collect();

    // Without normalization: raw RRF magnitudes are below the 0.05 floor.
    let raw_ranked: Vec<(usize, f64)> =
        raw_scores.iter().enumerate().map(|(i, s)| (i, *s)).collect();
    let raw_result = mmr_select(&raw_ranked, &vectors, 0.7, 0.05);
    assert_eq!(
        raw_result.len(), 1,
        "raw RRF-scale scores collapse to only the unconditionally-kept first item"
    );

    // With normalization: top items survive, diversity logic is restored.
    let normalized = min_max_normalize(
        &raw_scores.iter().map(|s| *s as f32).collect::<Vec<_>>(),
    );
    let norm_ranked: Vec<(usize, f64)> = normalized
        .iter()
        .enumerate()
        .map(|(i, s)| (i, *s as f64))
        .collect();
    let norm_result = mmr_select(&norm_ranked, &vectors, 0.7, 0.05);
    assert!(
        norm_result.len() > raw_result.len(),
        "normalization must keep more diverse items than the raw collapse, got {norm_result:?}"
    );
    assert!(
        norm_result.iter().any(|(idx, _)| *idx == 1),
        "the diverse second-ranked item must survive after normalization, got {norm_result:?}"
    );
}

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
    let text_lens = vec![900usize, 900, 900];
    let input: Vec<(usize, f64)> = vec![(0, 0.9), (1, 0.8), (2, 0.7)];
    let result = token_budget_filter(&input, &text_lens, 650);
    assert_eq!(result.len(), 2);
}

#[test]
fn max_k_truncate_enforces_bounds() {
    let short: Vec<(usize, f64)> = vec![(0, 0.9)];
    let long: Vec<(usize, f64)> = (0..25).map(|i| (i, 0.9 - i as f64 * 0.01)).collect();

    let result_short = max_k_truncate(short, 20);
    assert_eq!(result_short.len(), 1);

    let result_max = max_k_truncate(long, 20);
    assert_eq!(result_max.len(), 20);
}

// === MMR Tests ===

#[test]
fn mmr_removes_near_duplicate_chunks() {
    let ranked = vec![(0, 0.9), (1, 0.85), (2, 0.7)];
    let v0: Vec<f64> = vec![1.0, 0.0, 0.0];
    let v1: Vec<f64> = vec![0.99, 0.01, 0.0];
    let v2: Vec<f64> = vec![0.0, 1.0, 0.0];
    let vectors: HashMap<usize, &[f64]> = [
        (0, v0.as_slice()), (1, v1.as_slice()), (2, v2.as_slice()),
    ].into_iter().collect();

    let result = mmr_select(&ranked, &vectors, 0.7, 0.05);
    assert!(result.iter().any(|(idx, _)| *idx == 0));
    assert!(result.iter().any(|(idx, _)| *idx == 2));
}

#[test]
fn mmr_keeps_all_when_diverse() {
    let ranked = vec![(0, 0.9), (1, 0.8), (2, 0.7)];
    let v0: Vec<f64> = vec![1.0, 0.0, 0.0];
    let v1: Vec<f64> = vec![0.0, 1.0, 0.0];
    let v2: Vec<f64> = vec![0.0, 0.0, 1.0];
    let vectors: HashMap<usize, &[f64]> = [
        (0, v0.as_slice()), (1, v1.as_slice()), (2, v2.as_slice()),
    ].into_iter().collect();

    let result = mmr_select(&ranked, &vectors, 0.7, 0.05);
    assert_eq!(result.len(), 3);
}

#[test]
fn mmr_retains_chunks_without_vectors() {
    let ranked = vec![(0, 0.9), (1, 0.8)];
    let v0: Vec<f64> = vec![1.0, 0.0];
    let vectors: HashMap<usize, &[f64]> = [
        (0, v0.as_slice()),
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
