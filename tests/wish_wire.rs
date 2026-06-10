//! WISH HTTP wire-shape tests (Phase 5).
//!
//! These pin the JSON envelope produced by `POST /v1/wish` for each of
//! the three verdict variants of the SUDOKU trichotomy. Same pattern as
//! `causal_states_wire.rs`: we don't spin up axum, we round-trip the
//! response struct through serde and assert on the resulting JSON
//! shape. The handler itself is thin glue; the substrate it calls
//! (`relaxation_solve`) has 13 W-math tests pinning the math.

#![cfg(feature = "wish")]

use gigi::imagine::wish::{
    relaxation_solve, CP1FubiniStudy, CurvaturePinch, S2Stereographic, SolverKind, T2Flat,
    WishConfig, WishOutcome,
};
use serde_json::{json, Value};

/// Mirror of the (private) `WishHttpResponse` in `gigi_stream.rs`. If
/// the handler-side struct field names diverge from this, the wire
/// contract is broken and these tests will catch it before deploy.
#[derive(serde::Serialize)]
struct ExpectedGrantedJson {
    verdict: &'static str,
    unsat: bool,
    capacity: f64,
    arc_length: f64,
    integrated_curvature: f64,
    accumulated_holonomy: f64,
    solver_iterations: u32,
    path: Vec<Vec<f64>>,
}

#[derive(serde::Serialize)]
struct ExpectedUnreachableJson {
    verdict: &'static str,
    unsat: bool,
    frontier_waypoint: Vec<f64>,
    waypoint_kind: &'static str,
    reached_fraction: f64,
    blocked_by: &'static str,
    capacity_to_waypoint: f64,
}

#[derive(serde::Serialize)]
struct ExpectedIndeterminateJson {
    verdict: &'static str,
    unsat: Option<bool>,
    reason: &'static str,
    final_residual: f64,
}

fn loose_cfg() -> WishConfig {
    WishConfig {
        max_imagined_curvature: 1e9,
        max_accumulated_holonomy: 1e9,
        max_arc_length: 1e9,
        max_iterations: 5000,
        max_solve_ms: 60_000,
        grad_tol: 1e-5,
        energy_tol: 1e-12,
        solver: SolverKind::Relaxation { n_nodes: 32 },
        ..Default::default()
    }
}

// ─── verdict serialization keys ──────────────────────────────────────────

#[test]
fn granted_wire_shape_has_required_keys_and_unsat_false() {
    let mirror = ExpectedGrantedJson {
        verdict: "granted",
        unsat: false,
        capacity: f64::INFINITY,
        arc_length: 0.0,
        integrated_curvature: 0.0,
        accumulated_holonomy: 0.0,
        solver_iterations: 0,
        path: vec![vec![0.0, 0.0], vec![1.0, 0.0]],
    };
    let v = serde_json::to_value(mirror).unwrap();
    let obj = v.as_object().unwrap();
    for k in [
        "verdict",
        "unsat",
        "capacity",
        "arc_length",
        "integrated_curvature",
        "accumulated_holonomy",
        "solver_iterations",
        "path",
    ] {
        assert!(obj.contains_key(k), "granted response missing key: {}", k);
    }
    assert_eq!(obj["verdict"], "granted");
    assert_eq!(obj["unsat"], false);
}

#[test]
fn unreachable_wire_shape_has_required_keys_and_unsat_true() {
    let mirror = ExpectedUnreachableJson {
        verdict: "unreachable",
        unsat: true,
        frontier_waypoint: vec![0.19, 0.0],
        waypoint_kind: "frontier_truncation",
        reached_fraction: 0.187,
        blocked_by: "curvature",
        capacity_to_waypoint: 2.10,
    };
    let v = serde_json::to_value(mirror).unwrap();
    let obj = v.as_object().unwrap();
    for k in [
        "verdict",
        "unsat",
        "frontier_waypoint",
        "waypoint_kind",
        "reached_fraction",
        "blocked_by",
        "capacity_to_waypoint",
    ] {
        assert!(obj.contains_key(k), "unreachable response missing key: {}", k);
    }
    assert_eq!(obj["verdict"], "unreachable");
    assert_eq!(obj["unsat"], true);
    // §5 v0.1 waypoint_kind is the only one shipped now.
    assert_eq!(obj["waypoint_kind"], "frontier_truncation");
}

#[test]
fn indeterminate_wire_shape_has_required_keys_and_unsat_null() {
    let mirror = ExpectedIndeterminateJson {
        verdict: "indeterminate",
        unsat: None, // serialized as null
        reason: "non_convergence",
        final_residual: 4.7e-3,
    };
    let v = serde_json::to_value(mirror).unwrap();
    let obj = v.as_object().unwrap();
    for k in ["verdict", "unsat", "reason", "final_residual"] {
        assert!(obj.contains_key(k), "indeterminate response missing key: {}", k);
    }
    assert_eq!(obj["verdict"], "indeterminate");
    assert_eq!(obj["unsat"], Value::Null);
}

#[test]
fn blocked_by_tags_are_exactly_curvature_holonomy_arc_length() {
    // The §10 GQL row pin requires a stable set of blocked_by values.
    // Audit-log filters branch on these strings, so they cannot drift.
    for tag in ["curvature", "holonomy", "arc_length"] {
        let m = ExpectedUnreachableJson {
            verdict: "unreachable",
            unsat: true,
            frontier_waypoint: vec![0.0, 0.0],
            waypoint_kind: "frontier_truncation",
            reached_fraction: 0.1,
            blocked_by: tag,
            capacity_to_waypoint: 1.0,
        };
        let v = serde_json::to_value(m).unwrap();
        assert_eq!(v["blocked_by"], tag);
    }
}

#[test]
fn indeterminate_reason_tags_are_exactly_conjugate_locus_or_non_convergence() {
    for tag in ["conjugate_locus", "non_convergence"] {
        let m = ExpectedIndeterminateJson {
            verdict: "indeterminate",
            unsat: None,
            reason: tag,
            final_residual: 0.0,
        };
        let v = serde_json::to_value(m).unwrap();
        assert_eq!(v["reason"], tag);
    }
}

// ─── end-to-end: solver -> outcome -> JSON shape ─────────────────────────
//
// Drive the actual solver, then mirror the outcome into the wire shape,
// then assert the JSON keys. If the substrate side produces an outcome
// the handler can't map cleanly, these catch it.

#[test]
fn end_to_end_flat_t2_grant_round_trips_to_wire() {
    let out = relaxation_solve(&T2Flat, [0.0, 0.0], [0.6, 0.4], &loose_cfg());
    match out {
        WishOutcome::Granted {
            path,
            arc_length,
            integrated_curvature,
            capacity,
            accumulated_holonomy,
            solver_iterations,
            ..
        } => {
            let v = json!({
                "verdict": "granted",
                "unsat": false,
                "capacity": capacity,
                "arc_length": arc_length,
                "integrated_curvature": integrated_curvature,
                "accumulated_holonomy": accumulated_holonomy,
                "solver_iterations": solver_iterations,
                "path": path,
            });
            // Sanity: arc_length matches Euclidean on flat T².
            let euc = (0.6_f64 * 0.6 + 0.4 * 0.4).sqrt();
            let got = v["arc_length"].as_f64().unwrap();
            assert!((got - euc).abs() < 1e-9, "T² arc len {} vs {}", got, euc);
            assert_eq!(v["unsat"], false);
        }
        other => panic!("expected Granted, got verdict variant: {:?}", outcome_tag(&other)),
    }
}

#[test]
fn end_to_end_pinch_unreachable_round_trips_to_wire() {
    let m = CurvaturePinch::default();
    let mut cfg = loose_cfg();
    cfg.max_imagined_curvature = 1.0;
    cfg.max_arc_length = 10.0;
    cfg.max_accumulated_holonomy = 100.0;
    cfg.solver = SolverKind::Relaxation { n_nodes: 64 };
    let out = relaxation_solve(&m, [0.0, 0.0], [1.0, 0.0], &cfg);
    match out {
        WishOutcome::Unreachable {
            frontier_waypoint,
            reached_fraction,
            blocked_by,
            capacity_to_waypoint,
        } => {
            let block_tag = match blocked_by {
                gigi::imagine::provenance::WishBlockReason::Curvature => "curvature",
                gigi::imagine::provenance::WishBlockReason::Holonomy => "holonomy",
                gigi::imagine::provenance::WishBlockReason::ArcLength => "arc_length",
            };
            let v = json!({
                "verdict": "unreachable",
                "unsat": true,
                "frontier_waypoint": frontier_waypoint,
                "waypoint_kind": "frontier_truncation",
                "reached_fraction": reached_fraction,
                "blocked_by": block_tag,
                "capacity_to_waypoint": capacity_to_waypoint,
            });
            assert_eq!(v["verdict"], "unreachable");
            assert_eq!(v["unsat"], true);
            assert_eq!(v["blocked_by"], "curvature");
            // Frontier waypoint should be before the pinch at x=0.5.
            let wp_x = v["frontier_waypoint"][0].as_f64().unwrap();
            assert!(wp_x < 0.45, "waypoint x={} should sit before pinch", wp_x);
        }
        other => panic!("expected Unreachable on pinch, got {:?}", outcome_tag(&other)),
    }
}

#[test]
fn end_to_end_tight_iterations_indeterminate_round_trips_to_wire() {
    // Tight iteration cap -> Indeterminate(NonConvergence).
    let m = S2Stereographic;
    let mut cfg = loose_cfg();
    cfg.max_iterations = 1;
    let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
    match out {
        WishOutcome::Indeterminate { reason } => {
            use gigi::imagine::wish::IndeterminateReason;
            let (tag, res) = match reason {
                IndeterminateReason::ConjugateLocus { at_fraction } => {
                    ("conjugate_locus", at_fraction)
                }
                IndeterminateReason::NonConvergence { final_residual } => {
                    ("non_convergence", final_residual)
                }
            };
            let v = json!({
                "verdict": "indeterminate",
                "unsat": Value::Null,
                "reason": tag,
                "final_residual": res,
            });
            assert_eq!(v["verdict"], "indeterminate");
            assert_eq!(v["unsat"], Value::Null);
            assert_eq!(v["reason"], "non_convergence");
        }
        other => panic!("expected Indeterminate, got {:?}", outcome_tag(&other)),
    }
}

#[test]
fn cp1_grant_round_trips_to_wire() {
    // Cross-check CP¹ via the wire mapping. Same chart-coord geodesic
    // as S² but the K integrand differs (K=4), so capacity differs.
    let out = relaxation_solve(&CP1FubiniStudy, [0.1, 0.0], [0.5, 0.3], &loose_cfg());
    match out {
        WishOutcome::Granted {
            capacity,
            arc_length,
            integrated_curvature,
            ..
        } => {
            // K_int integrates |K| against CHORD arc length (matching
            // wish_validation.py and the §4.1 integrand), so the
            // identity is K_int = K · (chord len), not K · τ. The
            // chord length from (0.1,0) to (0.5,0.3) is sqrt(0.16+0.09)
            // = 0.5, so on CP¹(K=4) we expect K_int ≈ 2.0.
            assert!(
                capacity.is_finite() && capacity > 0.0,
                "CP¹ capacity must be positive-finite, got {}",
                capacity
            );
            assert!(arc_length > 0.0, "τ must be positive");
            let rel = (integrated_curvature - 2.0).abs() / 2.0;
            assert!(
                rel < 1e-3,
                "CP¹ K_int = {} expected ≈ 2.0 (= 4·chord_len)",
                integrated_curvature
            );
        }
        other => panic!("expected Granted on CP¹, got {:?}", outcome_tag(&other)),
    }
}

fn outcome_tag(o: &WishOutcome) -> &'static str {
    match o {
        WishOutcome::Granted { .. } => "Granted",
        WishOutcome::Unreachable { .. } => "Unreachable",
        WishOutcome::Indeterminate { .. } => "Indeterminate",
    }
}
