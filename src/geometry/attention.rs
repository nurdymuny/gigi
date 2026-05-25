//! L12 / attention pillar — ATTEND and FOCUS.
//!
//! Two brain primitives from `theory/brain_primitives/catalog.md`:
//!
//! - **§8 [`attend`]** — softmax over `-‖q − xᵢ‖² / 2σ²`. Identity
//!   with the normalized Gaussian kernel (Bishop, *PRML* §6.2). The
//!   brain's soft attention: distribute weight across all bundle
//!   records based on geodesic distance from the query, with
//!   exponentially decaying tails.
//!
//! - **§9 [`focus`]** — top-k attended records returned as a
//!   sub-bundle (indices + weights, sorted by attention). Hard
//!   attention / retrieval.
//!
//! Both are pure functions on sample vectors; no flow integration,
//! no PRNG. Cheap to call on hot retrieval paths.

#![cfg(feature = "kahler")]

/// L12 / §8 — Softmax attention over Euclidean distance.
///
/// Returns weights `α_i ∈ [0, 1]` with `Σ α_i = 1` such that
///
/// > `α_i = exp(−‖q − xᵢ‖² / 2σ²) / Σ_j exp(−‖q − xⱼ‖² / 2σ²)`
///
/// — equivalently the normalized Gaussian kernel at `q`. `bandwidth`
/// is `σ`; smaller bandwidths sharpen attention toward the nearest
/// record, larger ones spread it out.
///
/// Numerical stability: subtracts the max logit before exponentiating
/// (standard log-sum-exp trick).
///
/// Returns an empty vector if `samples` is empty or `bandwidth ≤ 0`.
pub fn attend(
    samples: &[Vec<f64>],
    query: &[f64],
    bandwidth: f64,
) -> Vec<f64> {
    if samples.is_empty() || bandwidth <= 0.0 {
        return Vec::new();
    }
    let two_bw_sq = 2.0 * bandwidth * bandwidth;
    let logits: Vec<f64> = samples
        .iter()
        .map(|s| {
            let d_sq: f64 = s
                .iter()
                .zip(query.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            -d_sq / two_bw_sq
        })
        .collect();
    let max_logit = logits.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut exps: Vec<f64> = logits.iter().map(|l| (l - max_logit).exp()).collect();
    let sum: f64 = exps.iter().sum();
    if sum == 0.0 {
        // All samples infinitely far → uniform.
        let n = samples.len() as f64;
        return vec![1.0 / n; samples.len()];
    }
    for e in &mut exps {
        *e /= sum;
    }
    exps
}

/// L12 / §9 — Top-k focus: indices + weights of the `k` highest-
/// attention records, sorted descending.
///
/// Returns `Vec<(index, weight)>` of length `min(k, samples.len())`.
/// Useful as a hard-attention shortlist: feed the indices back into
/// the bundle to materialize the focused records.
pub fn focus(
    samples: &[Vec<f64>],
    query: &[f64],
    bandwidth: f64,
    k: usize,
) -> Vec<(usize, f64)> {
    let weights = attend(samples, query, bandwidth);
    if weights.is_empty() {
        return Vec::new();
    }
    let mut idx_w: Vec<(usize, f64)> =
        weights.iter().enumerate().map(|(i, &w)| (i, w)).collect();
    // Sort descending by weight.
    idx_w.sort_by(|a, b| {
        b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal)
    });
    idx_w.truncate(k);
    idx_w
}

// ── tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn build_corpus() -> Vec<Vec<f64>> {
        vec![
            vec![0.0, 0.0],
            vec![0.5, 0.0],
            vec![1.0, 0.0],
            vec![2.0, 0.0],
            vec![3.0, 0.0],
        ]
    }

    // ── §8 ATTEND ──────────────────────────────────────────────

    #[test]
    fn attend_weights_sum_to_one() {
        let weights = attend(&build_corpus(), &[0.0, 0.0], 1.0);
        let sum: f64 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1e-12, "weights sum {} ≠ 1", sum);
    }

    #[test]
    fn attend_peaks_at_nearest_sample() {
        let weights = attend(&build_corpus(), &[0.0, 0.0], 1.0);
        let argmax = weights
            .iter()
            .enumerate()
            .fold((0_usize, f64::NEG_INFINITY), |acc, (i, &w)| {
                if w > acc.1 { (i, w) } else { acc }
            })
            .0;
        assert_eq!(argmax, 0, "expected weight peak at sample 0 (origin)");
    }

    #[test]
    fn attend_monotone_in_distance() {
        let weights = attend(&build_corpus(), &[0.0, 0.0], 1.0);
        for i in 0..weights.len() - 1 {
            assert!(
                weights[i] >= weights[i + 1],
                "weights not monotone at {}: {} < {}",
                i,
                weights[i],
                weights[i + 1],
            );
        }
    }

    #[test]
    fn attend_matches_gaussian_kernel_exactly() {
        let samples = build_corpus();
        let q = [0.3, 0.0];
        let bw = 0.7;
        let weights = attend(&samples, &q, bw);

        // Independent computation: raw kernel values then normalize.
        let raw: Vec<f64> = samples
            .iter()
            .map(|s| {
                let d_sq: f64 = s
                    .iter()
                    .zip(q.iter())
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                (-d_sq / (2.0 * bw * bw)).exp()
            })
            .collect();
        let raw_sum: f64 = raw.iter().sum();
        let expected: Vec<f64> = raw.iter().map(|r| r / raw_sum).collect();

        for i in 0..samples.len() {
            assert!(
                (weights[i] - expected[i]).abs() < 1e-12,
                "weight {} differs from kernel: {} vs {}",
                i,
                weights[i],
                expected[i],
            );
        }
    }

    #[test]
    fn attend_empty_or_zero_bandwidth_returns_empty() {
        assert!(attend(&[], &[0.0], 1.0).is_empty());
        assert!(attend(&build_corpus(), &[0.0, 0.0], 0.0).is_empty());
    }

    #[test]
    fn attend_far_query_stays_well_defined() {
        // Even at huge distances, weights should be finite, sum to 1,
        // and not produce NaN. The log-sum-exp trick guards against
        // overflow / underflow; the sample whose direction best
        // matches the query wins.
        let weights = attend(&build_corpus(), &[1e10, 1e10], 1.0);
        assert!(!weights.is_empty());
        let sum: f64 = weights.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-12,
            "weights should sum to 1 even at huge distances; sum = {}",
            sum,
        );
        for w in &weights {
            assert!(w.is_finite(), "weight is non-finite: {}", w);
            assert!(*w >= 0.0, "weight is negative: {}", w);
        }
    }

    // ── §9 FOCUS ───────────────────────────────────────────────

    #[test]
    fn focus_returns_top_k_in_order() {
        // Query at origin → expect indices [0, 1, 2] (the three
        // nearest in build_corpus()).
        let top3 = focus(&build_corpus(), &[0.0, 0.0], 1.0, 3);
        assert_eq!(top3.len(), 3);
        assert_eq!(top3[0].0, 0);
        assert_eq!(top3[1].0, 1);
        assert_eq!(top3[2].0, 2);
        // Weights must be sorted descending.
        assert!(top3[0].1 > top3[1].1);
        assert!(top3[1].1 > top3[2].1);
    }

    #[test]
    fn focus_k_larger_than_corpus_returns_all() {
        let all = focus(&build_corpus(), &[0.0, 0.0], 1.0, 100);
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn focus_k_zero_returns_empty() {
        let none = focus(&build_corpus(), &[0.0, 0.0], 1.0, 0);
        assert!(none.is_empty());
    }

    #[test]
    fn focus_off_center_query_picks_nearest() {
        // Query at (2.5, 0) → nearest is sample 3 (at x=2.0) then
        // sample 4 (at x=3.0). Sample 2 (at x=1.0) is farther but
        // closer than sample 0.
        let top2 = focus(&build_corpus(), &[2.5, 0.0], 0.5, 2);
        assert_eq!(top2[0].0, 3);
        assert_eq!(top2[1].0, 4);
    }
}
