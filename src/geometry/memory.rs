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
    // Just verify the re-export compiles + returns Some for a real
    // bundle. (Constructing a BundleStore worth Morse-compressing
    // requires a kahler-flagged schema; the L6 tests cover that.)
}
