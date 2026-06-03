//! `ImaginedRecord` and its required `ImaginedProvenance` enum.
//!
//! Per Marcella's feedback #1 on `IMAGINE_AND_WALK.md`: provenance is
//! **load-bearing**. Every imagined record carries an explicit
//! provenance describing how it was constructed, and the cite-render
//! contract requires that imagined records be visually distinct from
//! retrieved ones in any output that surfaces them.

use serde::{Deserialize, Serialize};

/// A record that does NOT exist in the substrate; it was constructed
/// by IMAGINE from existing records via geodesic extrapolation, halo
/// projection, or bridge composition.
///
/// The `provenance` field is REQUIRED at construction. There is no
/// public way to construct an `ImaginedRecord` without supplying
/// provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImaginedRecord {
    /// The synthesized fiber-space coordinates at this point.
    pub coords: Vec<f64>,

    /// Local Gaussian curvature at the imagined point, computed from
    /// the substrate's metric at this point (NOT from the imagined
    /// neighborhood — imagined records do not enter each other's K
    /// computation; that would introduce a fictitious self-reference).
    pub local_k: f64,

    /// Accumulated holonomy defect along the path from the seed
    /// record to this imagined point. Used by WALK's safety check
    /// against `max_accumulated_holonomy`.
    pub accumulated_holonomy: f64,

    /// REQUIRED provenance describing how this record was constructed.
    pub provenance: ImaginedProvenance,
}

/// How an `ImaginedRecord` was constructed. The variant determines
/// the cite-render prefix and the audit-log entry shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImaginedProvenance {
    /// Constructed by integrating the geodesic equation forward from
    /// `seed_record_id` along an initial direction for path length `s`.
    Geodesic {
        /// Identifier of the seed record in its source bundle.
        seed_record_id: String,
        /// Bundle name the seed came from.
        seed_bundle: String,
        /// Initial direction vector in the seed's local fiber coords.
        initial_direction: Vec<f64>,
        /// Total path length integrated.
        path_length: f64,
        /// Number of RK4 integrator steps taken to reach this point.
        integrator_steps: u32,
    },
    /// Constructed by projecting `seed_record` from another chart
    /// through the bridge transition map (intra-bundle halo
    /// population under sharding).
    Halo {
        source_chart: u32,
        target_chart: u32,
        seed_record_id: String,
        /// Empirical Lipschitz constant of the transition used.
        transition_lipschitz: f64,
    },
    /// Constructed by composing a bridge transition across two
    /// distinct atlases (cross-atlas join, Phase F target).
    Bridge {
        source_atlas: String,
        target_atlas: String,
        seed_record_id: String,
        bridge_id: String,
        /// Observed cocycle slack across the bridge at this projection.
        delta_cocycle_observed: f64,
    },
}

impl ImaginedRecord {
    /// Render this record with its provenance prefix per the cite
    /// contract in `IMAGINE_AND_WALK.md` §2.
    ///
    /// Marcella's discipline: an imagined record must never appear in
    /// citation output without a visual marker distinguishing it from
    /// a retrieved record. This method is the canonical render path.
    /// Callers that bypass it must add the prefix themselves.
    pub fn cite_render(&self, content_snippet: &str) -> String {
        match &self.provenance {
            ImaginedProvenance::Geodesic {
                seed_bundle,
                path_length,
                ..
            } => format!(
                "[imagined: projected from {} via geodesic, path_length={:.2}, \
                accumulated_holonomy={:.3}] {}",
                seed_bundle, path_length, self.accumulated_holonomy, content_snippet,
            ),
            ImaginedProvenance::Halo {
                source_chart,
                target_chart,
                transition_lipschitz,
                ..
            } => format!(
                "[imagined-halo: projected from chart_{} via transition into chart_{}, \
                lipschitz={:.2}] {}",
                source_chart, target_chart, transition_lipschitz, content_snippet,
            ),
            ImaginedProvenance::Bridge {
                source_atlas,
                target_atlas,
                bridge_id,
                delta_cocycle_observed,
                ..
            } => format!(
                "[imagined-bridge: {} → {} via {}, delta_cocycle={:.4}] {}",
                source_atlas, target_atlas, bridge_id, delta_cocycle_observed,
                content_snippet,
            ),
        }
    }

    /// Provenance variant tag for audit logs.
    pub fn provenance_kind(&self) -> &'static str {
        match self.provenance {
            ImaginedProvenance::Geodesic { .. } => "geodesic",
            ImaginedProvenance::Halo { .. } => "halo",
            ImaginedProvenance::Bridge { .. } => "bridge",
        }
    }

    /// Always true. Provided per Marcella's response-pipeline contract
    /// (round-3 feedback #1): consumer code branching on
    /// "imagined vs retrieved" in a response path should use a method
    /// call rather than parsing the cite-render string or matching the
    /// `ImaginedProvenance` enum variant. The method is the canonical
    /// way to ask "should this record be rendered with an imagined
    /// prefix?" — and the answer is yes, because the type only exists
    /// for imagined records.
    ///
    /// Retrieved records live in different types (`crate::types::Record`,
    /// substrate `BundleRef::records`), which do NOT expose
    /// `is_imagined()`. The asymmetry is intentional — there is no
    /// silent default. Pattern-matched response items can call
    /// `.is_imagined()` only when it returns true.
    #[inline]
    pub fn is_imagined(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_geodesic_record() -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![0.31, 0.04, -0.12],
            local_k: 0.18,
            accumulated_holonomy: 0.04,
            provenance: ImaginedProvenance::Geodesic {
                seed_record_id: "rec_42".into(),
                seed_bundle: "geometry_of_flight".into(),
                initial_direction: vec![1.0, 0.0, 0.0],
                path_length: 0.31,
                integrator_steps: 31,
            },
        }
    }

    fn sample_halo_record() -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![0.42, -0.17],
            local_k: 0.25,
            accumulated_holonomy: 0.0,
            provenance: ImaginedProvenance::Halo {
                source_chart: 3,
                target_chart: 1,
                seed_record_id: "rec_99".into(),
                transition_lipschitz: 2.1,
            },
        }
    }

    fn sample_bridge_record() -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![1.4, -0.7, 0.3],
            local_k: 0.31,
            accumulated_holonomy: 0.02,
            provenance: ImaginedProvenance::Bridge {
                source_atlas: "marcella_corpus".into(),
                target_atlas: "prism_reconciliation".into(),
                seed_record_id: "rec_marcella_177".into(),
                bridge_id: "fin_semantics_v1".into(),
                delta_cocycle_observed: 0.018,
            },
        }
    }

    #[test]
    fn geodesic_render_carries_provenance_prefix() {
        let r = sample_geodesic_record();
        let s = r.cite_render("...content here...");
        assert!(s.starts_with("[imagined: projected from geometry_of_flight via geodesic"),
                "missing geodesic provenance prefix: {}", s);
        assert!(s.contains("path_length=0.31"));
        assert!(s.contains("accumulated_holonomy=0.040"));
    }

    #[test]
    fn halo_render_carries_provenance_prefix() {
        let r = sample_halo_record();
        let s = r.cite_render("...content...");
        assert!(s.starts_with("[imagined-halo: projected from chart_3 via transition into chart_1"),
                "missing halo provenance prefix: {}", s);
        assert!(s.contains("lipschitz=2.10"));
    }

    #[test]
    fn bridge_render_carries_provenance_prefix() {
        let r = sample_bridge_record();
        let s = r.cite_render("...content...");
        assert!(s.starts_with("[imagined-bridge: marcella_corpus → prism_reconciliation"),
                "missing bridge provenance prefix: {}", s);
        assert!(s.contains("via fin_semantics_v1"));
        assert!(s.contains("delta_cocycle=0.0180"));
    }

    #[test]
    fn provenance_kind_tags_correctly_for_audit_log() {
        assert_eq!(sample_geodesic_record().provenance_kind(), "geodesic");
        assert_eq!(sample_halo_record().provenance_kind(), "halo");
        assert_eq!(sample_bridge_record().provenance_kind(), "bridge");
    }

    #[test]
    fn is_imagined_returns_true_for_every_provenance_variant() {
        // Per Marcella round-3 feedback #1: response-path branching
        // must be a method call, not a string parse. The method
        // returns true for any ImaginedRecord regardless of provenance.
        assert!(sample_geodesic_record().is_imagined());
        assert!(sample_halo_record().is_imagined());
        assert!(sample_bridge_record().is_imagined());
    }

    #[test]
    fn imagined_record_serde_round_trips() {
        for r in [
            sample_geodesic_record(),
            sample_halo_record(),
            sample_bridge_record(),
        ] {
            let json = serde_json::to_string(&r).unwrap();
            let back: ImaginedRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back);
        }
    }
}
