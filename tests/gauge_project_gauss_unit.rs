//! TDD-HAL-IV.3 red test — `PROJECT_GAUSS` struct + Tikhonov-
//! regularized CG solver.
//!
//! These tests pin the contract for the Gauss projection verb the
//! Part IV symplectic flow calls at every leapfrog step (locked
//! decision IV-D: per-step cadence, no knob at launch). The projector
//! finds the Lagrange multiplier `λ ∈ ℝ^(V·3)` such that the cleaned
//! E field
//!
//! ```text
//!     E_clean = E_dirty − D_cov(U)^T · λ
//! ```
//!
//! satisfies `||G_cov(E_clean)||_inf ≤ cg_tol`. The normal equation
//!
//! ```text
//!     L_cov(U) · λ = G_cov(E_dirty),
//!     L_cov(U) = D_cov(U) · D_cov(U)^T + tikhonov · I
//! ```
//!
//! is solved by unpreconditioned Hestenes–Stiefel CG.
//!
//! Gate locked decisions in play:
//!   - IV-A: `ProjectGaussConfig::default()` returns
//!     `{ tikhonov: 1e-14, cg_tol: 1e-10, cg_max_iter: 200 }`. The
//!     1e-14 default matches Halcyon Python production, NOT the
//!     1e-12 spec default (which is reachable via an explicit struct
//!     literal).
//!   - IV-C: `SU2EField` is the sibling primitive; the projector
//!     mutates the buffer through `&mut SU2EField`. q0=0 invariant
//!     re-enforced on every write.
//!   - IV-E: no preconditioner. Buckyball cond(L_cov) ~ 16 — plain
//!     CG converges in O(10) iterations.
//!   - A2 row 2: same seed/β/U/E → same process → byte-identical
//!     `E_clean` (intra-binding determinism — load-bearing for the
//!     IV.10 gold gate).
//!
//! Group-erasure note: SU(2)-only at launch. The `L_cov(U)` operator
//! is built from `Ad(U_e)` which is SU(2)-specific (quaternion
//! sandwich); other groups return `UnsupportedGroup` at the
//! `project_gauss` entry. Future SU(3) ships a sibling
//! `project_gauss_su3` with its own Ad operator.

#![cfg(feature = "gauge")]

use gigi::gauge::{
    build_vertex_edge_incidence, compute_gauss_residual_covariant,
    e_field::{EFieldInit, SU2EField},
    max_inf_norm,
    project_gauss, ProjectGaussConfig,
    registry::{
        clear as clear_gauge, clear_e_registry, get_su2_e_mut,
        register_su2, register_su2_e, test_serial_lock,
    },
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
};
use gigi::lattice::{
    registry as lattice_registry, topology::truncated_icosahedron::buckyball,
};
use std::sync::{Arc, Mutex};

/// TDD-HAL-IV.3: `ProjectGaussConfig::default()` returns
/// `{ tikhonov: 1e-14, cg_tol: 1e-10, cg_max_iter: 200 }` — the
/// Halcyon-production-matching default per locked decision IV-A.
#[test]
fn tdd_hal_iv_3_project_gauss_config_default() {
    let cfg = ProjectGaussConfig::default();
    assert_eq!(
        cfg.tikhonov, 1e-14,
        "tikhonov default = 1e-14 (Halcyon production), NOT 1e-12 spec default"
    );
    assert_eq!(cfg.cg_tol, 1e-10, "cg_tol default = 1e-10");
    assert_eq!(cfg.cg_max_iter, 200, "cg_max_iter default = 200");
}

/// TDD-HAL-IV.3: at U = IDENTITY + E = MaxwellBoltzmann seed
/// 20260617, the initial Gauss residual is small but nonzero. The
/// projector drives `||G_cov(E_clean)||_inf <= cg_tol` and converges
/// in a small number of CG iterations (cond(L_cov) is mild on identity
/// links).
#[test]
fn tdd_hal_iv_3_project_gauss_identity_no_op() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u = SU2GaugeField::new(
        "U_id_proj".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity U init");
    let e = SU2EField::new(
        "E_id_proj".into(),
        &u,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .expect("MB E init");
    register_su2(u);
    let u_handle = gigi::gauge::registry::get("U_id_proj").expect("registered U");
    register_su2_e(Arc::new(Mutex::new(e)));

    let inc = build_vertex_edge_incidence(&bb);
    let cfg = ProjectGaussConfig::default();

    let e_arc = get_su2_e_mut("E_id_proj").expect("registered E");
    let mut e_guard = e_arc.lock().expect("e field mutex poisoned");
    let diag = project_gauss(&mut e_guard, u_handle.as_ref(), &bb, &inc, cfg)
        .expect("project_gauss must succeed at identity U");

    // Convergence below cg_tol on the residual the projector reports.
    assert!(
        diag.final_gauss_residual_inf <= cfg.cg_tol,
        "final ||G_cov||_inf = {} > cg_tol = {}",
        diag.final_gauss_residual_inf,
        cfg.cg_tol
    );
    assert!(
        !diag.cg_did_not_converge,
        "CG should converge on identity-U + MB-E (mild conditioning)"
    );
    // Identity-U is the easy case — well under the diagnostic
    // budget. Loose bound (≤ 20 iters) so we are not pinning the
    // exact CG schedule.
    assert!(
        diag.cg_iterations > 0 && diag.cg_iterations <= 20,
        "cg_iterations = {} (expected 1..=20 on identity U)",
        diag.cg_iterations
    );

    // q0=0 invariant survives the projection.
    for edge in 0..bb.n_edges() {
        let q = e_guard.read_element_q(edge);
        assert_eq!(q[0], 0.0, "edge {edge} q0 must remain 0 after projection");
    }

    // The post-projection covariant residual recomputed independently
    // must agree with the projector's `final_gauss_residual_inf`.
    drop(e_guard);
    let e_post_handle = gigi::gauge::registry::get_su2_e("E_id_proj")
        .expect("registered E");
    let residual = compute_gauss_residual_covariant(
        u_handle.as_ref(),
        e_post_handle.as_ref(),
        &bb,
        &inc,
    )
    .expect("post-projection covariant residual");
    assert!(
        max_inf_norm(&residual) <= cfg.cg_tol,
        "recomputed ||G_cov(E_clean)||_inf = {} > cg_tol = {}",
        max_inf_norm(&residual),
        cfg.cg_tol
    );
}

/// TDD-HAL-IV.3: at thermalized U (GIBBS_SAMPLE 200 sweeps β=2.5
/// seed=20260616) + E = MaxwellBoltzmann seed 20260617, the projector
/// drives `||G_cov(E_clean)||_inf` below the production-canonical
/// target of 1e-9. Convergence in fewer than 200 iterations.
#[test]
fn tdd_hal_iv_3_project_gauss_thermalized() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u = SU2GaugeField::new(
        "U_therm_proj".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .expect("haar U init");
    register_su2(u);
    gigi::gauge::gibbs_sample(
        "U_therm_proj",
        2.5,
        200,
        200,
        vec![gigi::gauge::ObservableId::MeanPlaquette],
        Some(20260616),
    )
    .expect("gibbs_sample thermalization");

    let u_handle = gigi::gauge::registry::get("U_therm_proj").expect("registered U");

    // Build E off a fresh identity-U binding (E only needs n_edges /
    // lattice metadata from a U; the projection consumes the actual
    // thermalized U via u_handle).
    let e_template = SU2GaugeField::new(
        "U_template_proj".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity U for E binding");
    let e = SU2EField::new(
        "E_therm_proj".into(),
        &e_template,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .expect("MB E init");
    register_su2_e(Arc::new(Mutex::new(e)));

    let inc = build_vertex_edge_incidence(&bb);
    let cfg = ProjectGaussConfig::default();

    let e_arc = get_su2_e_mut("E_therm_proj").expect("registered E");
    let mut e_guard = e_arc.lock().expect("e field mutex poisoned");
    let diag = project_gauss(&mut e_guard, u_handle.as_ref(), &bb, &inc, cfg)
        .expect("project_gauss must succeed at thermalized U");

    // Production-canonical target: ||G_cov||_inf < 1e-9 after one
    // projector call (Halcyon Python production).
    assert!(
        diag.final_gauss_residual_inf < 1e-9,
        "final ||G_cov||_inf = {} >= 1e-9 production-canonical target",
        diag.final_gauss_residual_inf
    );
    assert!(
        !diag.cg_did_not_converge,
        "CG should converge at thermalized U within {} iters",
        cfg.cg_max_iter
    );
    assert!(
        diag.cg_iterations > 0 && diag.cg_iterations < cfg.cg_max_iter,
        "cg_iterations = {} (expected 0 < n < {})",
        diag.cg_iterations,
        cfg.cg_max_iter
    );

    // Initial residual is recorded too — must be strictly larger than
    // the final one (or equal if MB happened to already be Gauss-clean,
    // which it is not).
    assert!(
        diag.initial_gauss_residual_inf >= diag.final_gauss_residual_inf,
        "initial residual {} < final residual {} (projection should not increase)",
        diag.initial_gauss_residual_inf,
        diag.final_gauss_residual_inf
    );

    // q0=0 invariant survives.
    for edge in 0..bb.n_edges() {
        let q = e_guard.read_element_q(edge);
        assert_eq!(q[0], 0.0, "edge {edge} q0 must remain 0");
    }
}

/// TDD-HAL-IV.3: `diagnostics.cg_iterations` records the CG iteration
/// count. At the default config on thermalized U it is strictly
/// between 0 and `cg_max_iter`, and `cg_did_not_converge` is false.
#[test]
fn tdd_hal_iv_3_project_gauss_cg_iter_count_recorded() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u = SU2GaugeField::new(
        "U_iter".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .expect("haar U");
    register_su2(u);
    gigi::gauge::gibbs_sample(
        "U_iter",
        2.5,
        50,
        50,
        vec![gigi::gauge::ObservableId::MeanPlaquette],
        Some(20260616),
    )
    .expect("gibbs sweep");
    let u_handle = gigi::gauge::registry::get("U_iter").expect("U");

    let e_template = SU2GaugeField::new(
        "U_template_iter".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    let e = SU2EField::new(
        "E_iter".into(),
        &e_template,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .unwrap();
    register_su2_e(Arc::new(Mutex::new(e)));

    let inc = build_vertex_edge_incidence(&bb);
    let cfg = ProjectGaussConfig::default();

    let e_arc = get_su2_e_mut("E_iter").unwrap();
    let mut e_guard = e_arc.lock().unwrap();
    let diag = project_gauss(&mut e_guard, u_handle.as_ref(), &bb, &inc, cfg).unwrap();

    assert!(
        diag.cg_iterations > 0,
        "cg_iterations must be > 0 on non-clean E"
    );
    assert!(
        diag.cg_iterations < cfg.cg_max_iter,
        "cg_iterations = {} must be < cg_max_iter = {}",
        diag.cg_iterations,
        cfg.cg_max_iter
    );
    assert!(!diag.cg_did_not_converge);
    assert!(
        diag.cg_residual_final.is_finite(),
        "cg_residual_final must be finite, got {}",
        diag.cg_residual_final
    );
}

/// TDD-HAL-IV.3: non-convergence is a diagnostic, not a panic. Force
/// `cg_max_iter = 2` + `cg_tol = 1e-30` so CG cannot converge in time;
/// the projector returns Ok with `cg_did_not_converge = true` and the
/// output buffer stays finite (no NaN). A2 inference: non-convergence
/// is a diagnostic, NOT a regression trigger.
#[test]
fn tdd_hal_iv_3_project_gauss_non_convergence_no_panic() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u = SU2GaugeField::new(
        "U_nc".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .unwrap();
    register_su2(u);
    gigi::gauge::gibbs_sample(
        "U_nc",
        2.5,
        50,
        50,
        vec![gigi::gauge::ObservableId::MeanPlaquette],
        Some(20260616),
    )
    .unwrap();
    let u_handle = gigi::gauge::registry::get("U_nc").unwrap();

    let e_template = SU2GaugeField::new(
        "U_template_nc".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    let e = SU2EField::new(
        "E_nc".into(),
        &e_template,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .unwrap();
    register_su2_e(Arc::new(Mutex::new(e)));

    let inc = build_vertex_edge_incidence(&bb);
    let cfg = ProjectGaussConfig {
        tikhonov: 1e-14,
        cg_tol: 1e-30,
        cg_max_iter: 2,
    };

    let e_arc = get_su2_e_mut("E_nc").unwrap();
    let mut e_guard = e_arc.lock().unwrap();
    let diag = project_gauss(&mut e_guard, u_handle.as_ref(), &bb, &inc, cfg)
        .expect("project_gauss must return Ok even when CG does not converge");

    assert!(
        diag.cg_did_not_converge,
        "cg_max_iter=2 + cg_tol=1e-30 should not converge"
    );
    assert_eq!(diag.cg_iterations, cfg.cg_max_iter);
    // Output buffer stays finite — no NaN propagation.
    for edge in 0..bb.n_edges() {
        let q = e_guard.read_element_q(edge);
        for k in 0..4 {
            assert!(
                q[k].is_finite(),
                "edge {edge} component {k} is non-finite ({}) after partial CG",
                q[k]
            );
        }
        assert_eq!(q[0], 0.0, "q0=0 invariant under partial CG");
    }
}

/// TDD-HAL-IV.3: A2 row 2 — same inputs (U, E, lattice, config) into
/// `project_gauss` twice returns BYTE-IDENTICAL outputs (intra-binding
/// determinism). This is the load-bearing IV.10 gold-gate precondition.
#[test]
fn tdd_hal_iv_3_project_gauss_byte_identical_same_seed() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u = SU2GaugeField::new(
        "U_byte".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .unwrap();
    register_su2(u);
    gigi::gauge::gibbs_sample(
        "U_byte",
        2.5,
        100,
        100,
        vec![gigi::gauge::ObservableId::MeanPlaquette],
        Some(20260616),
    )
    .unwrap();
    let u_handle = gigi::gauge::registry::get("U_byte").unwrap();

    let inc = build_vertex_edge_incidence(&bb);
    let cfg = ProjectGaussConfig::default();

    // First call.
    let e_template = SU2GaugeField::new(
        "U_template_byte".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    let e1 = SU2EField::new(
        "E_byte_1".into(),
        &e_template,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .unwrap();
    register_su2_e(Arc::new(Mutex::new(e1)));
    let e1_arc = get_su2_e_mut("E_byte_1").unwrap();
    let diag1 = {
        let mut g = e1_arc.lock().unwrap();
        project_gauss(&mut g, u_handle.as_ref(), &bb, &inc, cfg).unwrap()
    };

    // Second call — identical inputs, fresh E.
    let e2 = SU2EField::new(
        "E_byte_2".into(),
        &e_template,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .unwrap();
    register_su2_e(Arc::new(Mutex::new(e2)));
    let e2_arc = get_su2_e_mut("E_byte_2").unwrap();
    let diag2 = {
        let mut g = e2_arc.lock().unwrap();
        project_gauss(&mut g, u_handle.as_ref(), &bb, &inc, cfg).unwrap()
    };

    // Diagnostics byte-identical.
    assert_eq!(diag1.cg_iterations, diag2.cg_iterations);
    assert_eq!(diag1.cg_did_not_converge, diag2.cg_did_not_converge);
    assert_eq!(diag1.cg_residual_final, diag2.cg_residual_final);
    assert_eq!(
        diag1.initial_gauss_residual_inf,
        diag2.initial_gauss_residual_inf
    );
    assert_eq!(
        diag1.final_gauss_residual_inf,
        diag2.final_gauss_residual_inf
    );

    // E buffer byte-identical.
    let g1 = e1_arc.lock().unwrap();
    let g2 = e2_arc.lock().unwrap();
    assert_eq!(
        g1.buffer.data, g2.buffer.data,
        "byte-identical post-projection E required for A2 row 2"
    );
}
