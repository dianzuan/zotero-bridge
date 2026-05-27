use zotron_types::{score_floor_filter, gap_cutoff, token_budget_filter, min_max_k_clamp};

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
