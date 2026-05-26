//! L12 / memory pillar — EPISODIC and SEMANTIC.
//!
//! Two brain-like memory primitives from
//! `theory/brain_primitives/catalog.md`:
//!
//! - **§10 [`episodic_events`]** — change-point detection on a
//!   time-indexed value sequence via persistent H₀ (elder rule on
//!   the 1-D Vietoris-Rips MST). Long-lived H₀ bars correspond to
//!   topologically-stable "events" — moments where the sequence's
//!   value distribution discontinuously shifted.
//!
//! - **§11 [`semantic_gist`]** — wraps L6's
//!   [`crate::discrete::morse::MorseComplex`] under the brain-API
//!   name. The Morse compression of a bundle's Hodge complex IS its
//!   "gist" — the minimum-cell-count cell complex with the same
//!   cohomology. Sleep-cycle analog: re-compute periodically as
//!   new memories accumulate.
//!
//! Both are scale-aware primitives that complement the attention
//! pillar — attention picks the relevant *records*; memory primitives
//! pick the relevant *structure*.

#![cfg(feature = "kahler")]

use crate::bundle::BundleStore;
use crate::discrete::morse::MorseComplex;

/// A detected episodic event — a discrete shift in the time-indexed
/// value sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodicEvent {
    /// Index in the *sorted-by-value* time series. The two records
    /// at sorted indices `boundary_idx` and `boundary_idx + 1`
    /// straddle the change-point.
    pub boundary_idx: usize,
    /// The actual gap value `|v_{i+1} − v_i|` after sorting.
    pub gap: f64,
    /// `gap / median_gap` — a unitless persistence ratio. Larger
    /// means more topologically stable (more "event-like"). Catalog
    /// validation: a real change-point gives ratios > 100×;
    /// stationary noise stays under ~50×.
    pub persistence_ratio: f64,
}

/// L12 / §10 — Detect episodic events in a 1-D value sequence.
///
/// Builds the elder-rule persistence of the H₀ complex on the
/// sorted values: any gap larger than `min_persistence_ratio ×
/// median_gap` is reported as an event boundary. The boundary
/// indices reference the *sorted* sequence, so the caller can map
/// them back to original record IDs via an `argsort`.
///
/// Returns events sorted descending by persistence ratio (most
/// topologically stable first).
///
/// Empty input or fewer than 3 values → empty result.
///
/// Uses the default denominator floor (`DEFAULT_GAP_FLOOR_EPSILON
/// = 1e-6`). For tighter control over the floor — e.g. on data
/// known to be clustered (batched timestamps, etc.) — use
/// [`episodic_events_with_floor`] directly.
pub fn episodic_events(values: &[f64], min_persistence_ratio: f64) -> Vec<EpisodicEvent> {
    episodic_events_with_floor(values, min_persistence_ratio, DEFAULT_GAP_FLOOR_EPSILON)
}

/// Default relative-max-gap floor for the persistence-ratio
/// denominator. Per Marcella 2026-05-25 (REPLY_L13_5_FILTER_PROBE):
/// when input has clustered structure (e.g. batched timestamps in
/// bge ingest), `median(gap) → 0` and the unfloored
/// `ratio = gap / median` overflows to ~1e288. The floor caps
/// reported ratios at `≈ 1 / ε = 1e6` — still distinguishes "real
/// event" from "noise" at any caller's threshold.
///
/// Pass 0 to [`episodic_events_with_floor`] to disable (escape
/// hatch for callers with well-spaced data); the absolute 1e-300
/// guard remains.
pub const DEFAULT_GAP_FLOOR_EPSILON: f64 = 1e-6;

/// L13.7 — Episodic-event detection with explicit denominator
/// floor.
///
/// Denominator formula: `denom = max(median, ε × max_gap, 1e-300)`.
/// At default ε = 1e-6, reported `persistence_ratio` is capped at
/// `≈ max_gap / (ε × max_gap) = 1 / ε = 1e6` — preserves ordering
/// across all gaps while preventing the overflow class
/// (Marcella's bge `ingested_at` 2.3e+288).
pub fn episodic_events_with_floor(
    values: &[f64],
    min_persistence_ratio: f64,
    gap_floor_epsilon: f64,
) -> Vec<EpisodicEvent> {
    if values.len() < 3 {
        return Vec::new();
    }
    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let gaps: Vec<f64> = sorted.windows(2).map(|w| w[1] - w[0]).collect();
    if gaps.is_empty() {
        return Vec::new();
    }
    let mut sorted_gaps = gaps.clone();
    sorted_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let raw_median = sorted_gaps[sorted_gaps.len() / 2];
    let max_gap = *sorted_gaps.last().unwrap_or(&0.0);
    // Same defensive-floor idiom as L13.6 σ² floor:
    //   denom = max(raw_median, ε × max_gap, 1e-300)
    let relative_floor = if gap_floor_epsilon > 0.0 {
        gap_floor_epsilon * max_gap
    } else {
        0.0
    };
    let denom = raw_median.max(relative_floor).max(1e-300);

    let mut events: Vec<EpisodicEvent> = gaps
        .iter()
        .enumerate()
        .filter_map(|(i, &g)| {
            let ratio = g / denom;
            if ratio >= min_persistence_ratio {
                Some(EpisodicEvent {
                    boundary_idx: i,
                    gap: g,
                    persistence_ratio: ratio,
                })
            } else {
                None
            }
        })
        .collect();
    events.sort_by(|a, b| {
        b.persistence_ratio
            .partial_cmp(&a.persistence_ratio)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    events
}

/// L12 / §13 (catalog "EXPLAIN") — geodesic path from a query
/// state to its nearest known record in the bundle.
///
/// Brain-primitive reading: "show me how the brain would get from
/// here to the closest thing it knows." For Friston-FEP free-energy
/// minimization on an isotropic Gaussian, the optimal-path-of-
/// descent from query `q` to the nearest sample `x*` is exactly
/// the straight-line interpolation in the metric-normalized
/// coordinates. For default Euclidean metric (what we use here),
/// that's literally linear interpolation `(1-t)q + t·x*` for `t`
/// in `[0, 1]`.
///
/// `n_steps` controls the path resolution. `n_steps + 1` points
/// are returned (initial query + n_steps forward toward the
/// target). With `n_steps = 0` you just get the endpoints.
///
/// Empty samples or zero-length query → empty path (and `None`
/// for nearest fields in the returned struct).
///
/// Usage:
/// - **Marcella**: "what's the closest memory to this novel query,
///   and what does the in-between look like?" — useful for
///   visualizing the bridge between a novel input and the model's
///   nearest training-distribution anchor.
/// - **PRISM**: "this transaction doesn't match anything cleanly —
///   what's the path from it to the most-similar known record?"
///   Provides interpretable "your transaction differs from
///   [closest match] by this gradient" output.
/// - **MIRADOR**: "this patient profile is unusual — here are the
///   intermediate cohort members along the path to the closest
///   known cohort."
pub fn explain(
    samples: &[Vec<f64>],
    query: &[f64],
    n_steps: usize,
) -> ExplanationPath {
    if samples.is_empty() || query.is_empty() {
        return ExplanationPath {
            query: query.to_vec(),
            nearest_record: None,
            nearest_index: None,
            nearest_distance: 0.0,
            path: Vec::new(),
            n_steps,
        };
    }
    // Find the nearest sample by Euclidean distance.
    let (nearest_idx, nearest_distance) = samples
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let d_sq: f64 = s
                .iter()
                .zip(query.iter())
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            (i, d_sq.sqrt())
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or((0, 0.0));
    let nearest = samples[nearest_idx].clone();

    // Linear interpolation query → nearest in (n_steps + 1) points.
    let path: Vec<Vec<f64>> = (0..=n_steps)
        .map(|i| {
            let t = if n_steps == 0 {
                1.0
            } else {
                i as f64 / n_steps as f64
            };
            query
                .iter()
                .zip(nearest.iter())
                .map(|(q, x)| (1.0 - t) * q + t * x)
                .collect()
        })
        .collect();

    ExplanationPath {
        query: query.to_vec(),
        nearest_record: Some(nearest),
        nearest_index: Some(nearest_idx),
        nearest_distance,
        path,
        n_steps,
    }
}

/// Geodesic-interpolation explanation returned by [`explain`].
#[derive(Debug, Clone, PartialEq)]
pub struct ExplanationPath {
    /// The query point as supplied.
    pub query: Vec<f64>,
    /// The nearest sample to the query (None if `samples` was empty).
    pub nearest_record: Option<Vec<f64>>,
    /// Index of the nearest sample in the original `samples` slice.
    pub nearest_index: Option<usize>,
    /// Euclidean distance from query to nearest sample.
    pub nearest_distance: f64,
    /// `n_steps + 1` points interpolating from `query` to
    /// `nearest_record`. Empty when no nearest record was found.
    pub path: Vec<Vec<f64>>,
    /// Step count as requested.
    pub n_steps: usize,
}

/// L12 / §11 — Semantic gist of a bundle (= Morse-compressed
/// representation of its Hodge complex, with cohomology preserved).
///
/// Returns `None` for the same cases `BundleStore::morse_compress`
/// returns `None` for (degenerate complex, too few records).
///
/// This is just a brain-API rename of the L6 morse_compress; the
/// semantic content is identical. The Morse complex's critical-cell
/// counts equal the Betti numbers — i.e. the *invariant topology*
/// of the bundle, with everything that was just "padding" stripped.
/// In brain terms: long-term semantic memory after the redundant
/// episodic detail has been pruned.
pub fn semantic_gist(store: &BundleStore) -> Option<MorseComplex> {
    store.morse_compress()
}

// ── tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── §10 EPISODIC ───────────────────────────────────────────

    #[test]
    fn episodic_detects_single_change_point() {
        // Two well-separated clusters: 20 values near 0, 20 near 5.
        let mut values = Vec::new();
        for i in 0..20 {
            values.push(0.0 + i as f64 * 0.01); // small steps
        }
        for i in 0..20 {
            values.push(5.0 + i as f64 * 0.01);
        }
        let events = episodic_events(&values, 50.0);
        assert!(
            !events.is_empty(),
            "expected at least one event, got none"
        );
        // The most-persistent event is the cluster boundary.
        let top = &events[0];
        assert!(
            top.persistence_ratio > 100.0,
            "top event ratio {} too small",
            top.persistence_ratio,
        );
        assert!(
            top.gap > 4.0,
            "top gap should be ~5 (cluster separation), got {}",
            top.gap,
        );
    }

    #[test]
    fn episodic_quiet_series_has_no_events() {
        // 100 monotonically increasing values with constant gap → no
        // event should be reported at threshold 50×.
        let values: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
        let events = episodic_events(&values, 50.0);
        assert!(
            events.is_empty(),
            "stationary series should have no events; got {} events",
            events.len(),
        );
    }

    #[test]
    fn episodic_two_change_points_both_detected() {
        let mut values = Vec::new();
        for i in 0..15 {
            values.push(0.0 + i as f64 * 0.01);
        }
        for i in 0..15 {
            values.push(10.0 + i as f64 * 0.01);
        }
        for i in 0..15 {
            values.push(50.0 + i as f64 * 0.01);
        }
        let events = episodic_events(&values, 50.0);
        assert!(
            events.len() >= 2,
            "expected ≥ 2 events, got {}",
            events.len()
        );
        // Top two should have gaps ≈ 10 and ≈ 40 (the biggest first
        // by persistence ratio).
        let gap_sum: f64 = events.iter().take(2).map(|e| e.gap).sum();
        assert!(
            gap_sum > 30.0,
            "top-2 gap sum should be ~50, got {}",
            gap_sum,
        );
    }

    #[test]
    fn episodic_empty_or_tiny_input_returns_empty() {
        assert!(episodic_events(&[], 50.0).is_empty());
        assert!(episodic_events(&[1.0], 50.0).is_empty());
        assert!(episodic_events(&[1.0, 2.0], 50.0).is_empty());
    }

    /// L13.7 — clustered-input pathology (Marcella's bge bundle).
    ///
    /// 50 values clustered tightly around two batches with tiny
    /// intra-cluster gaps + 2 large inter-cluster gaps. Without the
    /// floor, the inter-cluster gap (real, large) divided by the
    /// intra-cluster median (tiny, near machine zero) overflows.
    /// With the default ε = 1e-6 floor, the ratio is capped at
    /// ≈ 1e6 — large enough to fire on real events, finite enough
    /// to be a useful number.
    #[test]
    fn episodic_clustered_input_does_not_overflow() {
        let mut values: Vec<f64> = Vec::new();
        // Cluster A: 25 values within 1e-9 of t=0 (simulates a
        // batch ingest where timestamps are nominally same).
        for i in 0..25 {
            values.push(i as f64 * 1e-9);
        }
        // Cluster B: 25 values within 1e-9 of t=234000 (65 hours
        // later — matches Marcella's bge ingest case).
        for i in 0..25 {
            values.push(234_000.0 + i as f64 * 1e-9);
        }

        // Default function should not overflow.
        let events = episodic_events(&values, 1e3);
        assert!(
            !events.is_empty(),
            "real 65-hour gap should fire a persistence event"
        );
        let top = &events[0];
        assert!(
            top.persistence_ratio.is_finite(),
            "persistence_ratio {} should be finite (pre-floor was 2.3e288)",
            top.persistence_ratio
        );
        assert!(
            top.persistence_ratio < 1.0e9,
            "persistence_ratio {} should be capped under 1e9 (pre-floor was 2.3e288)",
            top.persistence_ratio
        );
        // Gap is the real one (~234000s).
        assert!(
            top.gap > 100_000.0,
            "gap {} should be the real 65h jump, not the noise floor",
            top.gap
        );
    }

    /// L13.7 — disabling the floor (`gap_floor_epsilon = 0`)
    /// reverts to the old behavior — overflow possible.
    #[test]
    fn episodic_zero_floor_still_works_for_well_spaced_data() {
        // Well-spaced data: floor or no floor, results match.
        let values: Vec<f64> = (0..30).map(|i| i as f64 * 0.5).collect();
        let with_floor = episodic_events_with_floor(&values, 50.0, 1e-6);
        let without_floor = episodic_events_with_floor(&values, 50.0, 0.0);
        assert_eq!(
            with_floor.len(),
            without_floor.len(),
            "well-spaced data: floored and unfloored should agree on event count"
        );
    }

    #[test]
    fn episodic_events_sorted_by_persistence_descending() {
        // Two events of different magnitudes.
        let mut values = Vec::new();
        for i in 0..10 {
            values.push(0.0 + i as f64 * 0.01);
        }
        for i in 0..10 {
            values.push(20.0 + i as f64 * 0.01); // gap of 20
        }
        for i in 0..10 {
            values.push(25.0 + i as f64 * 0.01); // gap of 5 from previous
        }
        let events = episodic_events(&values, 30.0);
        if events.len() >= 2 {
            assert!(
                events[0].persistence_ratio > events[1].persistence_ratio,
                "events not sorted descending: {} ≤ {}",
                events[0].persistence_ratio,
                events[1].persistence_ratio,
            );
        }
    }

    // ── §11 SEMANTIC ───────────────────────────────────────────
    //
    // semantic_gist is a thin wrapper over BundleStore::morse_compress
    // which has its own tests in src/discrete/morse.rs and src/bundle.rs.
    // We add a brain-API-layer test here to close the TDD gap noted
    // in Bee's 2026-05-26 cleanup audit — confirms the wrapper
    // returns a MorseComplex when the underlying store has enough
    // structure, and `None` when it doesn't.

    use crate::bundle::BundleStore;
    use crate::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};
    use crate::types::{BundleSchema, FieldDef, Record, Value};

    fn make_kahler_bundle(n_records: usize) -> BundleStore {
        let kahler = KahlerStructure::new(
            ComplexStructure::standard(1),
            ClosedTwoForm::new_constant(
                TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap(),
            ),
        );
        let schema = BundleSchema::new("semantic_test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(2.0))
            .fiber(FieldDef::numeric("y").with_range(2.0))
            .with_kahler(kahler);
        let mut store = BundleStore::new(schema);
        for i in 0..n_records {
            let mut rec = Record::new();
            rec.insert("id".into(), Value::Integer(i as i64));
            rec.insert("x".into(), Value::Float(i as f64 * 0.1));
            rec.insert("y".into(), Value::Float((i as f64 * 0.05).sin()));
            store.insert(&rec);
        }
        store
    }

    #[test]
    fn semantic_gist_returns_morse_complex_for_non_trivial_bundle() {
        let store = make_kahler_bundle(20);
        let m = semantic_gist(&store);
        assert!(
            m.is_some(),
            "20-record kahler bundle should produce a Morse complex"
        );
        let morse = m.unwrap();
        assert!(morse.cohomology_preserved(), "Morse compression must preserve cohomology");
        assert!(
            morse.n_critical() <= morse.n_original(),
            "critical-cell count {} should be ≤ original {}",
            morse.n_critical(),
            morse.n_original(),
        );
        assert!(morse.compression_ratio() >= 1.0);
    }

    #[test]
    fn semantic_gist_returns_none_for_too_small_bundle() {
        let store = make_kahler_bundle(1);
        assert!(
            semantic_gist(&store).is_none(),
            "single-record bundle is below Morse-compression threshold"
        );
    }

    // ── EXPLAIN (catalog §13) ────────────────────────────────

    #[test]
    fn explain_finds_nearest_sample() {
        let samples = vec![
            vec![0.0, 0.0],
            vec![5.0, 5.0],
            vec![10.0, 10.0],
        ];
        let query = vec![5.1, 4.9];
        let exp = explain(&samples, &query, 10);
        assert_eq!(exp.nearest_index, Some(1), "expected nearest to be index 1");
        assert_eq!(exp.nearest_record.as_deref(), Some(&[5.0, 5.0][..]));
        let expected_dist = ((5.1_f64 - 5.0).powi(2) + (4.9_f64 - 5.0).powi(2)).sqrt();
        assert!(
            (exp.nearest_distance - expected_dist).abs() < 1e-12,
            "expected distance ≈ {:.4}, got {:.4}",
            expected_dist,
            exp.nearest_distance,
        );
    }

    #[test]
    fn explain_path_has_n_steps_plus_one_points() {
        let samples = vec![vec![0.0, 0.0], vec![10.0, 0.0]];
        let query = vec![0.0, 0.0]; // exactly at sample 0
        let n = 5;
        let exp = explain(&samples, &query, n);
        assert_eq!(exp.path.len(), n + 1, "path has n+1 points");
    }

    #[test]
    fn explain_path_endpoints_are_query_and_nearest() {
        let samples = vec![vec![1.0, 2.0, 3.0]];
        let query = vec![7.0, 8.0, 9.0];
        let exp = explain(&samples, &query, 4);
        // First point of path == query.
        assert_eq!(exp.path[0], query);
        // Last point of path == nearest record.
        assert_eq!(*exp.path.last().unwrap(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn explain_path_is_linear_interpolation() {
        let samples = vec![vec![0.0, 0.0]];
        let query = vec![10.0, 0.0];
        let exp = explain(&samples, &query, 10);
        // Each successive point should advance by 1.0 along x.
        for (i, p) in exp.path.iter().enumerate() {
            let expected_x = 10.0 * (1.0 - i as f64 / 10.0);
            assert!(
                (p[0] - expected_x).abs() < 1e-12,
                "path point {} x = {} should be {}",
                i,
                p[0],
                expected_x,
            );
            assert!((p[1] - 0.0).abs() < 1e-12);
        }
    }

    #[test]
    fn explain_zero_steps_returns_endpoints_only() {
        let samples = vec![vec![5.0, 5.0]];
        let query = vec![0.0, 0.0];
        let exp = explain(&samples, &query, 0);
        // n_steps = 0 → one point (the endpoint).
        assert_eq!(exp.path.len(), 1);
        assert_eq!(exp.path[0], vec![5.0, 5.0]); // collapses to target
    }

    #[test]
    fn explain_empty_samples_returns_no_nearest() {
        let exp = explain(&[], &[1.0, 2.0], 5);
        assert!(exp.nearest_record.is_none());
        assert!(exp.nearest_index.is_none());
        assert!(exp.path.is_empty());
    }

    #[test]
    fn explain_picks_correct_record_among_many() {
        // 100 random-ish samples; verify EXPLAIN finds the actual
        // closest by independent direct search.
        let samples: Vec<Vec<f64>> = (0..100)
            .map(|i| {
                let t = i as f64 * 0.1;
                vec![t.cos(), t.sin()]
            })
            .collect();
        let query = vec![0.5, 0.0];
        let exp = explain(&samples, &query, 3);
        // Verify independently: walk all samples, find the one with
        // smallest Euclidean distance to query.
        let truth_idx = samples
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let d_sq: f64 = (s[0] - 0.5).powi(2) + s[1].powi(2);
                (i, d_sq)
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap()
            .0;
        assert_eq!(exp.nearest_index, Some(truth_idx));
    }
}
