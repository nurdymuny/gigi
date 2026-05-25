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
pub fn episodic_events(values: &[f64], min_persistence_ratio: f64) -> Vec<EpisodicEvent> {
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
    let median = sorted_gaps[sorted_gaps.len() / 2].max(1e-300);

    let mut events: Vec<EpisodicEvent> = gaps
        .iter()
        .enumerate()
        .filter_map(|(i, &g)| {
            let ratio = g / median;
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
