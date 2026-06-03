//! BM25 / cosine / RRF / MMR scoring and result cutoffs.

use crate::evidence::StructureChunk;

/// Score chunks against a whitespace-tokenized query using BM25.
///
/// Returns `(chunk_index, score)` pairs sorted descending by score.
/// Only chunks with a positive score are included.
pub fn bm25_score_chunks(
    chunks: &[StructureChunk],
    query: &str,
    k1: f64,
    b: f64,
) -> Vec<(usize, f64)> {
    let terms = tokenize_query(query);
    if terms.is_empty() || chunks.is_empty() {
        return Vec::new();
    }
    let n = chunks.len() as f64;
    let avg_dl: f64 = chunks.iter().map(|c| c.text.len() as f64).sum::<f64>() / n;

    let mut df: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for chunk in chunks {
        let lower = chunk.text.to_lowercase();
        for term in &terms {
            if lower.contains(term.as_str()) {
                *df.entry(term.as_str()).or_default() += 1;
            }
        }
    }

    let mut scores: Vec<(usize, f64)> = chunks
        .iter()
        .enumerate()
        .filter_map(|(i, chunk)| {
            let lower = chunk.text.to_lowercase();
            let dl = lower.len() as f64;
            let mut score = 0.0_f64;
            for term in &terms {
                let tf = lower.matches(term.as_str()).count() as f64;
                if tf == 0.0 { continue; }
                let doc_freq = *df.get(term.as_str()).unwrap_or(&0) as f64;
                let idf = ((n - doc_freq + 0.5) / (doc_freq + 0.5) + 1.0).ln();
                score += idf * (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / avg_dl));
            }
            if score > 0.0 { Some((i, score)) } else { None }
        })
        .collect();
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores
}

/// Cosine similarity between two equal-length f64 vectors.
///
/// Returns 0.0 for empty, mismatched-length, or zero-norm inputs.
pub fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 { return 0.0; }
    dot / (norm_a * norm_b)
}

/// Reciprocal Rank Fusion of two ranked lists.
///
/// Each input is `(index, score)` sorted descending by score. The output
/// merges them using RRF with constant `k` and returns at most `limit`
/// entries sorted by combined RRF score.
pub fn rrf_merge(
    bm25_ranked: &[(usize, f64)],
    dense_ranked: &[(usize, f64)],
    k: f64,
    limit: usize,
) -> Vec<(usize, f64)> {
    let mut scores: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
    for (rank, &(idx, _)) in bm25_ranked.iter().enumerate() {
        *scores.entry(idx).or_default() += 1.0 / (k + rank as f64 + 1.0);
    }
    for (rank, &(idx, _)) in dense_ranked.iter().enumerate() {
        *scores.entry(idx).or_default() += 1.0 / (k + rank as f64 + 1.0);
    }
    let mut merged: Vec<(usize, f64)> = scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);
    merged
}

pub fn score_floor_filter(ranked: &[(usize, f64)], floor: f64) -> Vec<(usize, f64)> {
    ranked.iter().filter(|(_, s)| *s >= floor).copied().collect()
}

pub fn gap_cutoff(ranked: &[(usize, f64)], threshold: f64) -> Vec<(usize, f64)> {
    if ranked.len() <= 1 {
        return ranked.to_vec();
    }
    for i in 0..ranked.len() - 1 {
        let gap = ranked[i].1 - ranked[i + 1].1;
        if gap > threshold {
            return ranked[..=i].to_vec();
        }
    }
    ranked.to_vec()
}

/// Greedily keep top-ranked chunks until the cumulative token estimate exceeds
/// `budget`. `char_lens` must hold each chunk's character count (`text.chars().count()`),
/// the same unit the chunker uses to size chunks — NOT the UTF-8 byte length.
///
/// Tokens are estimated as 1 token per character. This is exact for CJK text
/// (~1 token/char) and a safe over-estimate for Latin text (~4 chars/token),
/// so the budget is never blown. The previous `bytes / 3` heuristic only held
/// for CJK and badly mis-budgeted Latin, where bytes == chars.
pub fn token_budget_filter(
    ranked: &[(usize, f64)],
    char_lens: &[usize],
    budget: usize,
) -> Vec<(usize, f64)> {
    let mut total = 0usize;
    let mut result = Vec::new();
    for &(idx, score) in ranked {
        let tokens = char_lens.get(idx).copied().unwrap_or(0);
        if !result.is_empty() && total + tokens > budget {
            break;
        }
        total += tokens;
        result.push((idx, score));
    }
    result
}

pub fn max_k_truncate(
    mut ranked: Vec<(usize, f64)>,
    max_k: usize,
) -> Vec<(usize, f64)> {
    ranked.truncate(max_k);
    ranked
}

/// Min-max normalize a slice of scores into the [0,1] range.
///
/// Maps `min -> 0.0` and `max -> 1.0` via a linear transform. This lets
/// downstream relevance thresholds (e.g. the MMR cutoff) operate on a stable
/// [0,1] scale regardless of the upstream score origin (RRF ~0.016, raw BM25,
/// cosine, or 0..1 reranker scores).
///
/// Edge cases:
/// - empty input -> empty output
/// - single element -> `[1.0]`
/// - all-equal (`max == min`) -> all `1.0` (no division by zero)
/// - NaN values are ignored when computing min/max and passed through as the
///   max-mapped value (`1.0`) so they never produce NaN/inf in the output.
pub fn min_max_normalize(scores: &[f32]) -> Vec<f32> {
    if scores.is_empty() {
        return Vec::new();
    }
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    for &s in scores {
        if s.is_nan() {
            continue;
        }
        if s < min {
            min = s;
        }
        if s > max {
            max = s;
        }
    }
    // All-NaN, single element, or all-equal: collapse to 1.0 (no /0).
    let range = max - min;
    if !range.is_finite() || range <= f32::EPSILON {
        return vec![1.0; scores.len()];
    }
    scores
        .iter()
        .map(|&s| {
            if s.is_nan() {
                1.0
            } else {
                (s - min) / range
            }
        })
        .collect()
}

pub fn diversity_filter(
    ranked: &[(usize, f64)],
    vectors: &std::collections::HashMap<usize, &[f64]>,
    lambda: f64,
    threshold: f64,
) -> Vec<(usize, f64)> {
    if ranked.is_empty() {
        return Vec::new();
    }
    let mut selected: Vec<(usize, f64)> = vec![ranked[0]];

    for &(idx, score) in &ranked[1..] {
        let Some(vec_candidate) = vectors.get(&idx) else {
            selected.push((idx, score));
            continue;
        };
        let max_sim = selected
            .iter()
            .filter_map(|(sel_idx, _)| vectors.get(sel_idx))
            .map(|sel_vec| cosine_similarity(vec_candidate, sel_vec))
            .fold(0.0f64, f64::max);
        let mmr_score = lambda * score - (1.0 - lambda) * max_sim;
        if mmr_score > threshold {
            selected.push((idx, score));
        }
    }
    selected
}

fn tokenize_query(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || "，。、；：\u{201c}\u{201d}\u{2018}\u{2019}【】（）".contains(c))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.len() > 1)
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod retrieval_tests {
    use super::*;

    #[test]
    fn bm25_ranks_exact_match_highest() {
        let chunks = vec![
            StructureChunk {
                chunk_key: "c1".into(), item_key: "I1".into(), attachment_key: "A1".into(),
                block_keys: vec![], section_path: vec![], text: "就业弹性的测量方法与实证分析".into(),
                page_range: [0, 0], page_start: None, page_end: None, evidence_refs: vec![],
            },
            StructureChunk {
                chunk_key: "c2".into(), item_key: "I1".into(), attachment_key: "A1".into(),
                block_keys: vec![], section_path: vec![], text: "数字经济对产业结构的影响分析".into(),
                page_range: [1, 1], page_start: None, page_end: None, evidence_refs: vec![],
            },
            StructureChunk {
                chunk_key: "c3".into(), item_key: "I2".into(), attachment_key: "A2".into(),
                block_keys: vec![], section_path: vec![], text: "就业弹性在不同行业的差异".into(),
                page_range: [0, 0], page_start: None, page_end: None, evidence_refs: vec![],
            },
        ];
        let results = bm25_score_chunks(&chunks, "就业弹性 测量", 1.2, 0.75);
        assert!(!results.is_empty());
        // c1 should rank first (has both terms)
        assert_eq!(results[0].0, 0);
        // c3 should rank second (has 就业弹性 but not 测量)
        assert_eq!(results[1].0, 2);
        // c2 should not appear (has neither term)
        assert!(results.len() == 2 || results[2].1 == 0.0);
    }

    #[test]
    fn bm25_empty_query_returns_empty() {
        let chunks = vec![StructureChunk {
            chunk_key: "c1".into(), item_key: "I1".into(), attachment_key: "A1".into(),
            block_keys: vec![], section_path: vec![], text: "some text".into(),
            page_range: [0, 0], page_start: None, page_end: None, evidence_refs: vec![],
        }];
        assert!(bm25_score_chunks(&chunks, "", 1.2, 0.75).is_empty());
        assert!(bm25_score_chunks(&[], "query", 1.2, 0.75).is_empty());
    }

    #[test]
    fn cosine_similarity_identical_vectors() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_orthogonal_vectors() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        assert!(cosine_similarity(&a, &b).abs() < 1e-10);
    }

    #[test]
    fn cosine_similarity_empty_returns_zero() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    #[test]
    fn rrf_merge_combines_rankings() {
        let bm25 = vec![(0, 5.0), (1, 3.0), (2, 1.0)];
        let dense = vec![(2, 0.9), (0, 0.7), (3, 0.5)];
        let merged = rrf_merge(&bm25, &dense, 60.0, 10);
        // idx 0 appears in both lists (rank 0 in bm25, rank 1 in dense) — should be top
        assert_eq!(merged[0].0, 0);
        // idx 2 also appears in both (rank 2 in bm25, rank 0 in dense)
        assert!(merged.iter().any(|(idx, _)| *idx == 2));
    }

    #[test]
    fn rrf_merge_respects_limit() {
        let bm25 = vec![(0, 5.0), (1, 3.0), (2, 1.0)];
        let dense = vec![(3, 0.9), (4, 0.7)];
        let merged = rrf_merge(&bm25, &dense, 60.0, 2);
        assert!(merged.len() <= 2);
    }
}
