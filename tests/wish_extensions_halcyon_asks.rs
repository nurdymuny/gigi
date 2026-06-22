//! WISH extensions per Halcyon's WISH-extensions reply (2026-06-22).
//!
//! ASK 1: lift WISH off dim=2 via the `WishMetric` trait + registry
//!        (DESIGN WISH 1; this commit ships the trait surface + n-D
//!        dispatch entry point; full WishBundle/parallel_transport
//!        surface lands in the Halcyon follow-up).
//! ASK 2: WishTarget::Observable { name, value, err } with sigma-
//!        weighted convergence (DESIGN WISH 2).
//! ASK 3: per-segment capacity τ_i/κ_i populated on Granted, gated by
//!        WishConfig.compute_per_segment_capacity (DESIGN WISH 3).
//!
//! Locked constraints upheld by these tests:
//!   * 4 legacy 2D impls (S²/T²/CP¹/CurvaturePinch) behavior unchanged
//!     when `compute_per_segment_capacity` is left at default false.
//!   * `segment_capacities` is `None` by default — additive only.
//!   * `WishTarget::Observable` resolves through the legacy 2D
//!     evaluate_observable_2d dispatch (scalar_curvature, exp2phi,
//!     radius_chart).
//!   * `relaxation_solve_nd` dispatches by metric dim() so dim != 2
//!     no longer hits `UnsupportedDim` when a metric is registered.

#![cfg(all(feature = "imagine", feature = "wish"))]

use gigi::imagine::wish::{
    relaxation_solve, relaxation_solve_nd, relaxation_solve_target, CP1FubiniStudy,
    CurvaturePinch, IndeterminateReason, S2Stereographic, SolverKind, T2Flat, TrivialFlatND,
    WishConfig, WishMetric, WishMetric2D, WishMetric2DAdapter, WishMetricRegistry, WishOutcome,
    WishTarget,
};

fn open_budgets() -> WishConfig {
    let mut c = WishConfig {
        max_imagined_curvature: 1e9,
        max_accumulated_holonomy: 1e9,
        max_arc_length: 1e9,
        max_iterations: 5000,
        max_solve_ms: 60_000,
        grad_tol: 1e-5,
        energy_tol: 1e-12,
        ..Default::default()
    };
    c.solver = SolverKind::Relaxation { n_nodes: 32 };
    c
}

// ─── ASK 1: trait + registry + n-D dispatch ───────────────────────────────

#[test]
fn test_wish_metric_trait_n_d_dispatch() {
    // Registering a TrivialFlatND with dim=3 + invoking relaxation_solve_nd
    // proves the dim != 2 lift end-to-end.
    WishMetricRegistry::register("flat3d_test", || {
        Box::new(TrivialFlatND {
            name: "flat3d_test".into(),
            dim: 3,
        })
    });
    assert!(WishMetricRegistry::contains("flat3d_test"));
    let metric = WishMetricRegistry::get_factory("flat3d_test").expect("registered");
    assert_eq!(metric.dim(), 3);
    assert_eq!(metric.name(), "flat3d_test");

    let cfg = open_budgets();
    let seed = vec![0.0, 0.0, 0.0];
    let target = vec![1.0, 0.0, 0.0];
    let outcome = relaxation_solve_nd(&*metric, &seed, &target, &cfg)
        .expect("3D dispatch should succeed on flat metric");
    match outcome {
        WishOutcome::Granted {
            arc_length,
            path,
            ..
        } => {
            // Straight line in flat space → arc length = 1.0.
            assert!((arc_length - 1.0).abs() < 1e-9, "arc_length = {}", arc_length);
            assert_eq!(path[0].len(), 3, "path must be 3-D");
            assert_eq!(path.last().unwrap().len(), 3);
        }
        other => panic!("expected Granted for flat 3D dispatch, got {:?}", outcome_tag(&other)),
    }
}

#[test]
fn test_wish_metric_registry_list_contains_registered() {
    WishMetricRegistry::register("registry_list_probe", || {
        Box::new(TrivialFlatND {
            name: "registry_list_probe".into(),
            dim: 4,
        })
    });
    let names = WishMetricRegistry::list();
    assert!(
        names.iter().any(|n| n == "registry_list_probe"),
        "list() should include registered name, got: {:?}",
        names
    );
}

#[test]
fn test_wish_metric_2d_adapter_through_trait_object() {
    // The blanket adapter wraps S2Stereographic as a WishMetric and
    // exposes the same evaluate_observable surface.
    let s2 = WishMetric2DAdapter {
        name: "s2_adapter".to_string(),
        inner: S2Stereographic,
    };
    assert_eq!(s2.dim(), 2);
    assert_eq!(s2.name(), "s2_adapter");
    // Metric tensor at origin: exp(2*phi) = 4/(1+0)^2 = 4 → diag(4,4).
    let g = s2.metric_tensor(&[0.0, 0.0]);
    assert!((g[0] - 4.0).abs() < 1e-12);
    assert!((g[3] - 4.0).abs() < 1e-12);
    assert!(g[1].abs() < 1e-12 && g[2].abs() < 1e-12);
    // Observable: scalar_curvature on S² is 1 everywhere.
    let k = s2.evaluate_observable("scalar_curvature", &[0.3, 0.2]).unwrap();
    assert!((k - 1.0).abs() < 1e-12, "S² K = {}", k);
}

// ─── ASK 2: Observable target convergence ─────────────────────────────────

#[test]
fn test_wish_target_observable_converges() {
    // Sigma-weighted convergence on S² scalar curvature: K = 1 everywhere,
    // so any geodesic endpoint hits value=1.0 within any positive err.
    let m = S2Stereographic;
    let cfg = open_budgets();
    let target = WishTarget::Observable {
        name: "scalar_curvature".into(),
        value: 1.0,
        err: 1e-6,
    };
    // Provide an anchor endpoint so the geodesic actually moves.
    let out = relaxation_solve_target(&m, [0.1, 0.0], &target, Some([0.5, 0.3]), &cfg);
    match out {
        WishOutcome::Granted { path, .. } => {
            let end = path.last().expect("non-empty path");
            assert!((end[0] - 0.5).abs() < 1e-3 && (end[1] - 0.3).abs() < 1e-3);
        }
        other => panic!(
            "expected Granted for trivially-satisfied observable, got {:?}",
            outcome_tag(&other)
        ),
    }
}

#[test]
fn test_wish_target_observable_radius_meaningful() {
    // On flat T², the radius_chart observable converges to the actual
    // Euclidean distance from the chart origin to the endpoint.
    let m = T2Flat;
    let cfg = open_budgets();
    let endpoint = [0.6, 0.0];
    let expected_radius = 0.6;
    let target = WishTarget::Observable {
        name: "radius_chart".into(),
        value: expected_radius,
        err: 1e-9,
    };
    let out = relaxation_solve_target(&m, [0.0, 0.0], &target, Some(endpoint), &cfg);
    match out {
        WishOutcome::Granted { path, .. } => {
            let end = path.last().expect("non-empty path");
            let r = (end[0] * end[0] + end[1] * end[1]).sqrt();
            assert!((r - expected_radius).abs() < 1e-6, "radius = {}", r);
        }
        other => panic!(
            "expected Granted for radius_chart observable, got {:?}",
            outcome_tag(&other)
        ),
    }
}

#[test]
fn test_wish_target_observable_outside_err_indeterminate() {
    // Aim at K = 5.0 on S² (K is 1.0 everywhere) with err = 0.1 →
    // residual is 4.0, well above err → Indeterminate.
    let m = S2Stereographic;
    let cfg = open_budgets();
    let target = WishTarget::Observable {
        name: "scalar_curvature".into(),
        value: 5.0,
        err: 0.1,
    };
    let out = relaxation_solve_target(&m, [0.1, 0.0], &target, Some([0.5, 0.3]), &cfg);
    match out {
        WishOutcome::Indeterminate {
            reason: IndeterminateReason::NonConvergence { final_residual },
        } => {
            // Sigma-weighted residual = |1 - 5| / 0.1 = 40.
            assert!(final_residual > 1.0, "sigma residual = {}", final_residual);
        }
        other => panic!(
            "expected Indeterminate{{NonConvergence}}, got {:?}",
            outcome_tag(&other)
        ),
    }
}

#[test]
fn test_wish_target_observable_unknown_name_indeterminate() {
    let m = S2Stereographic;
    let cfg = open_budgets();
    let target = WishTarget::Observable {
        name: "nonexistent_observable".into(),
        value: 0.0,
        err: 1.0,
    };
    let out = relaxation_solve_target(&m, [0.1, 0.0], &target, Some([0.5, 0.3]), &cfg);
    match out {
        WishOutcome::Indeterminate { .. } => {}
        other => panic!(
            "unknown observable must yield Indeterminate, got {:?}",
            outcome_tag(&other)
        ),
    }
}

// ─── ASK 3: per-segment capacity ──────────────────────────────────────────

#[test]
fn test_wish_phase4_per_segment_capacity_finite() {
    // Flag-on: per-segment capacities populated finite for CP¹ Fubini-Study
    // where K = 4 > 0 everywhere (no NaN risk).
    let m = CP1FubiniStudy;
    let mut cfg = open_budgets();
    cfg.compute_per_segment_capacity = true;
    let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
    match out {
        WishOutcome::Granted {
            segment_capacities,
            capacity,
            ..
        } => {
            let segs = segment_capacities.expect(
                "flag=true must populate segment_capacities",
            );
            assert!(!segs.is_empty(), "segment_capacities should be non-empty");
            for (i, c_i) in segs.iter().enumerate() {
                assert!(
                    c_i.is_finite() && *c_i > 0.0,
                    "segment {} capacity = {} should be finite & positive on CP¹",
                    i,
                    c_i
                );
            }
            // Whole-path capacity also finite.
            assert!(
                capacity.is_finite() && capacity > 0.0,
                "whole-path capacity = {} should be finite",
                capacity
            );
        }
        other => panic!(
            "expected Granted on CP¹ with flag=true, got {:?}",
            outcome_tag(&other)
        ),
    }
}

#[test]
fn test_wish_per_segment_capacity_default_off_byte_identical() {
    // Default (flag=false) → segment_capacities is None, matching
    // existing behavior. Critical for the byte-identical contract.
    let m = S2Stereographic;
    let cfg = open_budgets();
    assert!(
        !cfg.compute_per_segment_capacity,
        "default config must have flag=false"
    );
    let out = relaxation_solve(&m, [0.1, 0.0], [0.5, 0.3], &cfg);
    match out {
        WishOutcome::Granted {
            segment_capacities, ..
        } => {
            assert!(
                segment_capacities.is_none(),
                "default config must leave segment_capacities = None"
            );
        }
        other => panic!("expected Granted, got {:?}", outcome_tag(&other)),
    }
}

#[test]
fn test_wish_per_segment_capacity_flat_t2_is_nan() {
    // On flat T² where K ≡ 0, every segment's κ = 0 → per-segment
    // capacity is NaN (sentinel for "no curvature to ratio against").
    let m = T2Flat;
    let mut cfg = open_budgets();
    cfg.compute_per_segment_capacity = true;
    let out = relaxation_solve(&m, [0.0, 0.0], [0.6, 0.4], &cfg);
    match out {
        WishOutcome::Granted {
            segment_capacities, ..
        } => {
            let segs = segment_capacities.expect("flag=true populates segments");
            for c_i in segs.iter() {
                assert!(
                    c_i.is_nan(),
                    "flat T² segments should yield NaN capacity, got {}",
                    c_i
                );
            }
        }
        other => panic!("expected Granted on T², got {:?}", outcome_tag(&other)),
    }
}

// ─── ASK 1 follow-on: legacy 2D impls still behave correctly ──────────────

#[test]
fn test_wish_metric_2d_impls_still_work() {
    // The 4 legacy WishMetric2D impls (S²/T²/CP¹/CurvaturePinch) must
    // continue producing the same Granted-on-easy-targets that the
    // existing W1-W5 tests check.
    let cfg = open_budgets();

    // S²: small displacement → Granted.
    match relaxation_solve(&S2Stereographic, [0.1, 0.0], [0.2, 0.05], &cfg) {
        WishOutcome::Granted { arc_length, .. } => {
            assert!(arc_length > 0.0 && arc_length < 1.0);
        }
        o => panic!("S² simple solve failed: {:?}", outcome_tag(&o)),
    }

    // T²: chord init IS the geodesic.
    match relaxation_solve(&T2Flat, [0.0, 0.0], [0.6, 0.4], &cfg) {
        WishOutcome::Granted { arc_length, .. } => {
            let expected = (0.6_f64 * 0.6 + 0.4 * 0.4).sqrt();
            assert!((arc_length - expected).abs() < 1e-9);
        }
        o => panic!("T² solve failed: {:?}", outcome_tag(&o)),
    }

    // CP¹: tiny displacement → Granted.
    match relaxation_solve(&CP1FubiniStudy, [0.1, 0.0], [0.15, 0.05], &cfg) {
        WishOutcome::Granted { arc_length, .. } => {
            assert!(arc_length > 0.0);
        }
        o => panic!("CP¹ solve failed: {:?}", outcome_tag(&o)),
    }

    // CurvaturePinch on a path AVOIDING the pinch at x = x_center = 0.5
    // → Granted with finite arc length.
    let pinch = CurvaturePinch::default();
    let mut wide_cfg = cfg.clone();
    wide_cfg.max_imagined_curvature = 1e9;
    match relaxation_solve(&pinch, [0.0, 0.0], [0.0, 0.5], &wide_cfg) {
        WishOutcome::Granted { arc_length, .. } => {
            assert!(arc_length > 0.0 && arc_length.is_finite());
        }
        o => panic!("Pinch avoidance solve failed: {:?}", outcome_tag(&o)),
    }
}

// ─── Provenance sister-variant smoke test ─────────────────────────────────

#[test]
fn test_wish_target_observable_provenance_round_trip() {
    use gigi::imagine::WishTargetProvenance;
    let tgt = WishTarget::Observable {
        name: "sigma_a2".into(),
        value: 0.18,
        err: 0.02,
    };
    let prov: WishTargetProvenance = (&tgt).into();
    match prov {
        WishTargetProvenance::Observable { name, value, err } => {
            assert_eq!(name, "sigma_a2");
            assert!((value - 0.18).abs() < 1e-12);
            assert!((err - 0.02).abs() < 1e-12);
        }
        other => panic!("expected Observable variant, got {:?}", other),
    }
}

// ─── support ──────────────────────────────────────────────────────────────

fn outcome_tag(o: &WishOutcome) -> &'static str {
    match o {
        WishOutcome::Granted { .. } => "Granted",
        WishOutcome::Unreachable { .. } => "Unreachable",
        WishOutcome::Indeterminate { .. } => "Indeterminate",
    }
}
