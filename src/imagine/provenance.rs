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

/// The target a WISH was solved toward — either chart coordinates or
/// a record in another bundle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WishTargetProvenance {
    /// The wish targeted a specific chart-coordinate point.
    Coords(Vec<f64>),
    /// The wish targeted a record in some bundle.
    Record { bundle: String, record_id: String },
}

/// Which budget refused a wish, used by the waypoint cite-render and
/// the audit-log entry on `Unreachable` outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WishBlockReason {
    Curvature,
    Holonomy,
    ArcLength,
}

/// When a wish is `Unreachable`, WISH returns a frontier-truncation
/// waypoint — the furthest in-budget node along the attempted path
/// (per WISH_SPEC_v0.1.md §6.1). The waypoint is rendered with the
/// `[wished-waypoint:` prefix, distinct from a granted endpoint, so
/// no consumer mistakes a refused wish for a satisfied one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WishWaypointInfo {
    /// Geodesic arc-length fraction reached toward the target
    /// (per §6.2: τ(seed → frontier) / τ(full attempted candidate)).
    pub reached_fraction: f64,
    /// Which budget the attempted path busted.
    pub blocked_by: WishBlockReason,
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
    /// Constructed by solving the geodesic BVP from a seed toward a
    /// target (per WISH_SPEC_v0.1.md §7). When `waypoint` is `None`,
    /// this record is the granted endpoint of the wish; when `Some`,
    /// it's a frontier-truncation waypoint from an `Unreachable`
    /// outcome and the cite-render uses the `[wished-waypoint:`
    /// prefix instead of `[wished:`. The asymmetry is load-bearing:
    /// no consumer may present a waypoint as a granted destination.
    Wished {
        /// Seed record id (the wish's starting point).
        seed_record_id: String,
        /// Bundle the seed was drawn from.
        seed_bundle: String,
        /// What the wish was aimed at.
        target: WishTargetProvenance,
        /// Geodesic arc length τ of the solved path.
        geodesic_arc_length: f64,
        /// Integrated curvature K crossed by the solved path.
        integrated_curvature: f64,
        /// Davis capacity C = τ / K — the wish's robustness score.
        capacity: f64,
        /// |endpoint − target| at solver convergence (granted: ≈ 0
        /// because endpoints are fixed in relaxation; waypoint: the
        /// residual at the budget-cutoff node).
        bvp_residual: f64,
        /// L-BFGS iteration count, for the §3.3 cost ledger.
        solver_iterations: u32,
        /// When `Some`, this is a frontier-truncation waypoint.
        /// When `None`, this is a granted endpoint.
        waypoint: Option<WishWaypointInfo>,
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
            ImaginedProvenance::Wished {
                seed_bundle,
                target,
                capacity,
                bvp_residual,
                waypoint,
                ..
            } => {
                let target_label = render_target_label(target);
                match waypoint {
                    None => format!(
                        "[wished: from {} toward {}, C=τ/K={:.2}, residual={:.1e}] {}",
                        seed_bundle, target_label, capacity, bvp_residual,
                        content_snippet,
                    ),
                    Some(wp) => format!(
                        "[wished-waypoint: from {}, reached {:.2} toward {}, \
                        blocked={}] {}",
                        seed_bundle, wp.reached_fraction, target_label,
                        block_reason_tag(wp.blocked_by), content_snippet,
                    ),
                }
            }
        }
    }

    /// Provenance variant tag for audit logs. Distinguishes
    /// `wished` (granted endpoint) from `wished_waypoint` (refused
    /// wish's frontier-truncation node) so audit-log readers can
    /// filter on outcome without re-parsing the cite-render string.
    pub fn provenance_kind(&self) -> &'static str {
        match &self.provenance {
            ImaginedProvenance::Geodesic { .. } => "geodesic",
            ImaginedProvenance::Halo { .. } => "halo",
            ImaginedProvenance::Bridge { .. } => "bridge",
            ImaginedProvenance::Wished { waypoint: None, .. } => "wished",
            ImaginedProvenance::Wished { waypoint: Some(_), .. } => "wished_waypoint",
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

fn render_target_label(target: &WishTargetProvenance) -> String {
    match target {
        WishTargetProvenance::Record { record_id, .. } => record_id.clone(),
        WishTargetProvenance::Coords(c) => {
            // Compact summary: "coords[d=N]" if the vector is long,
            // otherwise inline the values. Keeps the cite-render
            // bounded length regardless of substrate dimension.
            if c.len() > 4 {
                format!("coords[d={}]", c.len())
            } else {
                let parts: Vec<String> = c.iter().map(|x| format!("{:.2}", x)).collect();
                format!("coords[{}]", parts.join(", "))
            }
        }
    }
}

fn block_reason_tag(r: WishBlockReason) -> &'static str {
    match r {
        WishBlockReason::Curvature => "curvature",
        WishBlockReason::Holonomy => "holonomy",
        WishBlockReason::ArcLength => "arc_length",
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

    fn sample_wished_granted_record() -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![0.8, 0.4],
            local_k: 0.22,
            accumulated_holonomy: 0.21,
            provenance: ImaginedProvenance::Wished {
                seed_record_id: "rec_12".into(),
                seed_bundle: "geometry_of_flight".into(),
                target: WishTargetProvenance::Record {
                    bundle: "geometry_of_flight".into(),
                    record_id: "rec_88".into(),
                },
                geodesic_arc_length: 1.12,
                integrated_curvature: 0.33,
                capacity: 3.41,
                bvp_residual: 2.0e-7,
                solver_iterations: 47,
                waypoint: None,
            },
        }
    }

    fn sample_wished_waypoint_record() -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![0.55, 0.25],
            local_k: 0.18,
            accumulated_holonomy: 0.14,
            provenance: ImaginedProvenance::Wished {
                seed_record_id: "rec_12".into(),
                seed_bundle: "geometry_of_flight".into(),
                target: WishTargetProvenance::Record {
                    bundle: "geometry_of_flight".into(),
                    record_id: "rec_88".into(),
                },
                geodesic_arc_length: 0.68,
                integrated_curvature: 0.21,
                capacity: 3.24,
                bvp_residual: 8.5e-3,
                solver_iterations: 53,
                waypoint: Some(WishWaypointInfo {
                    reached_fraction: 0.62,
                    blocked_by: WishBlockReason::ArcLength,
                }),
            },
        }
    }

    fn sample_wished_coords_target_record() -> ImaginedRecord {
        ImaginedRecord {
            coords: vec![0.8, 0.4, 0.1],
            local_k: 0.19,
            accumulated_holonomy: 0.07,
            provenance: ImaginedProvenance::Wished {
                seed_record_id: "rec_seed".into(),
                seed_bundle: "icarus_state".into(),
                target: WishTargetProvenance::Coords(vec![0.80, 0.40, 0.10]),
                geodesic_arc_length: 0.94,
                integrated_curvature: 0.28,
                capacity: 3.36,
                bvp_residual: 3.0e-7,
                solver_iterations: 31,
                waypoint: None,
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
        assert!(sample_wished_granted_record().is_imagined());
        assert!(sample_wished_waypoint_record().is_imagined());
    }

    #[test]
    fn imagined_record_serde_round_trips() {
        for r in [
            sample_geodesic_record(),
            sample_halo_record(),
            sample_bridge_record(),
            sample_wished_granted_record(),
            sample_wished_waypoint_record(),
            sample_wished_coords_target_record(),
        ] {
            let json = serde_json::to_string(&r).unwrap();
            let back: ImaginedRecord = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back);
        }
    }

    // ─── Wished provenance: cite-render prefix discipline ─────────────────

    #[test]
    fn wished_granted_render_carries_wished_prefix() {
        let r = sample_wished_granted_record();
        let s = r.cite_render("...content...");
        assert!(
            s.starts_with("[wished: from geometry_of_flight toward rec_88"),
            "missing wished-granted prefix: {}",
            s
        );
        assert!(s.contains("C=τ/K=3.41"), "capacity missing or mis-rendered: {}", s);
        assert!(s.contains("residual=2.0e-7"), "residual missing: {}", s);
    }

    #[test]
    fn wished_waypoint_render_carries_distinct_waypoint_prefix() {
        // Load-bearing per WISH_SPEC_v0.1.md §7: a refused wish's
        // frontier-truncation waypoint MUST NOT render as a granted
        // endpoint. The `[wished-waypoint:` prefix is the visual
        // separation the response pipeline branches on.
        let r = sample_wished_waypoint_record();
        let s = r.cite_render("...content...");
        assert!(
            s.starts_with("[wished-waypoint: from geometry_of_flight"),
            "missing waypoint prefix: {}",
            s
        );
        assert!(s.contains("reached 0.62 toward rec_88"), "reached_fraction or target missing: {}", s);
        assert!(s.contains("blocked=arc_length"), "blocked_by tag missing or wrong: {}", s);
        // Explicit negative check: a waypoint must never look like a
        // grant when the audit log or response pipeline scans for the
        // bare `[wished:` prefix.
        assert!(
            !s.starts_with("[wished: "),
            "waypoint accidentally rendered as granted: {}",
            s
        );
    }

    #[test]
    fn wished_coords_target_renders_with_coord_summary() {
        let r = sample_wished_coords_target_record();
        let s = r.cite_render("...content...");
        assert!(
            s.contains("toward coords[0.80, 0.40, 0.10]"),
            "coords target should render as a short coord list: {}",
            s
        );
    }

    #[test]
    fn wished_provenance_kind_distinguishes_granted_from_waypoint() {
        // Audit-log filter discipline: a downstream auditor that wants
        // to count refused wishes shouldn't have to parse cite_render
        // strings. The provenance_kind() tag is the canonical answer.
        assert_eq!(sample_wished_granted_record().provenance_kind(), "wished");
        assert_eq!(
            sample_wished_waypoint_record().provenance_kind(),
            "wished_waypoint"
        );
    }

    #[test]
    fn wished_waypoint_renders_for_each_block_reason() {
        // Cite-render must produce the right tag for each block reason,
        // since the response pipeline may branch on which budget refused
        // the wish.
        let make_wp = |b: WishBlockReason| ImaginedRecord {
            coords: vec![0.0],
            local_k: 0.0,
            accumulated_holonomy: 0.0,
            provenance: ImaginedProvenance::Wished {
                seed_record_id: "s".into(),
                seed_bundle: "b".into(),
                target: WishTargetProvenance::Coords(vec![1.0]),
                geodesic_arc_length: 1.0,
                integrated_curvature: 1.0,
                capacity: 1.0,
                bvp_residual: 1.0e-3,
                solver_iterations: 1,
                waypoint: Some(WishWaypointInfo {
                    reached_fraction: 0.5,
                    blocked_by: b,
                }),
            },
        };
        assert!(make_wp(WishBlockReason::Curvature).cite_render("x").contains("blocked=curvature"));
        assert!(make_wp(WishBlockReason::Holonomy).cite_render("x").contains("blocked=holonomy"));
        assert!(make_wp(WishBlockReason::ArcLength).cite_render("x").contains("blocked=arc_length"));
    }
}
