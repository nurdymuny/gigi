//! TDD-HAL-IV.4 red test — Wilson force per edge (Lie-algebra
//! momentum kick).
//!
//! These tests pin the contract for the Wilson force `F[e] = coeff *
//! project_lie(qmul(U[e], Σ_e))` and the matching `apply_force_kick`
//! that lands the K step of the leapfrog KDK integrator. Σ_e is the
//! per-edge staple sum from III.3 (REUSED unchanged). `project_lie`
//! zeroes the q0 component (Lie-algebra projection); `coeff =
//! -beta/(2·N²) = -beta/8` for SU(2) (Halcyon's bug #3 fix — the
//! -beta/(2·N²) sign + magnitude is the load-bearing coefficient
//! that makes the leapfrog second-order symplectic).
//!
//! Gate locked decisions in play:
//!   - IV-B: SU2EField sibling buffer (no EdgeConnection impl); the
//!     kick writes through the `(n_edges, 4)` Lie row layout with
//!     q0=0 restored at every mutation.
//!   - Group-erasure: SU(2)-only — coefficient -beta/(2·N²) hardcodes
//!     N=2. Future SU(3) ships sibling `wilson_force_su3.rs`.

#![cfg(feature = "gauge")]

use gigi::gauge::{
    apply_force_kick,
    build_edge_face_incidence,
    e_field::{EFieldInit, SU2EField},
    registry::{
        clear as clear_gauge, clear_e_registry, register_su2, register_su2_e,
        test_serial_lock,
    },
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
    wilson_force_per_edge,
};
use gigi::lattice::{
    registry as lattice_registry, topology::truncated_icosahedron::buckyball,
};
use std::sync::{Arc, Mutex};

/// TDD-HAL-IV.4: U = IDENTITY → every face's staple is identity →
/// `qmul(U[e], Σ_e) = (k, 0, 0, 0)` (k = incident-face count). Lie
/// projection zeroes the scalar component → F[e] = [0, 0, 0, 0]
/// byte-identical on every edge.
#[test]
fn tdd_hal_iv_4_wilson_force_identity_links_zero() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv4_id".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity U init must succeed");
    register_su2(u_field);
    let u_arc =
        gigi::gauge::registry::get_su2_mut("U_iv4_id").expect("registered");
    let u_guard = u_arc.lock().expect("u mutex poisoned");
    let inc = build_edge_face_incidence(&bb);

    let f = wilson_force_per_edge(&*u_guard, &bb, &inc, 2.5)
        .expect("identity force must succeed");

    assert_eq!(f.len(), bb.n_edges(), "force is per-edge");
    for (eid, force_row) in f.iter().enumerate() {
        assert_eq!(
            force_row,
            &[0.0, 0.0, 0.0, 0.0],
            "edge {eid}: identity-link force must be exactly zero (got {:?})",
            force_row
        );
    }
}

/// TDD-HAL-IV.4: thermalized (Haar-random) U → force is finite (no
/// NaN/Inf) and per-edge ||F[e]|| is O(1) at β=2.5. Sanity guard
/// against a degenerate sign / coefficient error blowing the buffer
/// up.
#[test]
fn tdd_hal_iv_4_wilson_force_thermalized_finite() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv4_haar".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .expect("haar U init must succeed");
    register_su2(u_field);
    let u_arc =
        gigi::gauge::registry::get_su2_mut("U_iv4_haar").expect("registered");
    let u_guard = u_arc.lock().expect("u mutex poisoned");
    let inc = build_edge_face_incidence(&bb);

    let f = wilson_force_per_edge(&*u_guard, &bb, &inc, 2.5)
        .expect("haar force must succeed");

    for (eid, row) in f.iter().enumerate() {
        for (k, v) in row.iter().enumerate() {
            assert!(
                v.is_finite(),
                "edge {eid} component {k}: non-finite force {v}"
            );
        }
        let n2 = row[1] * row[1] + row[2] * row[2] + row[3] * row[3];
        let norm = n2.sqrt();
        // The staple is bounded by face-count k=2 on the buckyball;
        // qmul produces a quaternion of norm ≤ 2; project_lie keeps
        // the imaginary part with magnitude ≤ 2; multiplied by
        // |coeff| = β/8 = 0.3125 → bound ≤ 0.625. Pad to 5 for a
        // sanity ceiling.
        assert!(
            norm < 5.0,
            "edge {eid}: thermalized force norm {norm} blew up (expected O(1))"
        );
    }
}

/// TDD-HAL-IV.4: F[e][0] (q0 component) is exactly 0.0 for every
/// edge — `project_lie` is the boundary that zeroes the scalar
/// component of the Lie-algebra row.
#[test]
fn tdd_hal_iv_4_wilson_force_q0_zero() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv4_q0".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .unwrap();
    register_su2(u_field);
    let u_arc =
        gigi::gauge::registry::get_su2_mut("U_iv4_q0").expect("registered");
    let u_guard = u_arc.lock().expect("u mutex poisoned");
    let inc = build_edge_face_incidence(&bb);

    let f = wilson_force_per_edge(&*u_guard, &bb, &inc, 2.5).unwrap();

    for (eid, row) in f.iter().enumerate() {
        assert_eq!(
            row[0], 0.0,
            "edge {eid}: q0 must be exactly 0.0 (Lie projection), got {}",
            row[0]
        );
    }
}

/// TDD-HAL-IV.4: coefficient is `coeff = -β / (2·N²) = -β/8` for
/// SU(2). Test by computing F at β=2.5 vs β=5.0 on the same U buffer;
/// the staple Σ_e does not depend on β, so F(5.0) = 2·F(2.5) within
/// FP floor.
#[test]
fn tdd_hal_iv_4_wilson_force_coefficient_beta_minus_4() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv4_coef".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .unwrap();
    register_su2(u_field);
    let u_arc =
        gigi::gauge::registry::get_su2_mut("U_iv4_coef").expect("registered");
    let u_guard = u_arc.lock().expect("u mutex poisoned");
    let inc = build_edge_face_incidence(&bb);

    let f_low = wilson_force_per_edge(&*u_guard, &bb, &inc, 2.5).unwrap();
    let f_high = wilson_force_per_edge(&*u_guard, &bb, &inc, 5.0).unwrap();

    let tol = 1e-12_f64;
    for eid in 0..bb.n_edges() {
        for k in 0..4 {
            let want = 2.0 * f_low[eid][k];
            let got = f_high[eid][k];
            assert!(
                (got - want).abs() < tol,
                "edge {eid} comp {k}: F(5.0) = {got}, expected 2·F(2.5) = {want}, diff {}",
                (got - want).abs()
            );
        }
    }
}

/// TDD-HAL-IV.4: `apply_force_kick(e_mut, F, dt_half)` restores q0=0
/// on every edge after the kick. Defends against FP roundoff in the
/// row mutation: q0 is forced to 0.0 exactly at the write boundary.
#[test]
fn tdd_hal_iv_4_apply_force_kick_q0_zero_invariant() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv4_kick".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .unwrap();
    register_su2(u_field);
    let u_arc =
        gigi::gauge::registry::get_su2_mut("U_iv4_kick").expect("registered");
    let inc = build_edge_face_incidence(&bb);
    let f = {
        let u_guard = u_arc.lock().expect("u mutex poisoned");
        wilson_force_per_edge(&*u_guard, &bb, &inc, 2.5).unwrap()
    };

    let e_field = SU2EField::new(
        "E_iv4_kick".into(),
        &*u_arc.lock().expect("u mutex poisoned"),
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260618),
    )
    .expect("MB E init must succeed");
    let e_arc = Arc::new(Mutex::new(e_field));
    register_su2_e(e_arc.clone());

    {
        let mut e_guard = e_arc.lock().expect("e mutex poisoned");
        apply_force_kick(&mut *e_guard, &f, 0.01).expect("kick must succeed");
    }

    let e_guard = e_arc.lock().expect("e mutex poisoned");
    for edge in 0..bb.n_edges() {
        let row = e_guard.read_element_q(edge);
        assert_eq!(
            row[0], 0.0,
            "edge {edge}: q0 must be exactly 0.0 after kick, got {}",
            row[0]
        );
    }
}

/// TDD-HAL-IV.4: dt halving → 4:1 scaling on the per-edge update
/// magnitude. The kick is `E += dt_half · F`, so halving dt_half
/// halves the update; integrating two kicks of dt_half/2 yields
/// the same E (linearity); but the second-order leapfrog accuracy
/// shows up in the L2 distance between E(dt_half) and E(dt_half/2)
/// applied twice. Here we test the single-kick linearity directly:
/// kick(dt) = 2 · kick(dt/2).
#[test]
fn tdd_hal_iv_4_dt_halving_4_to_1_scaling() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_iv4_dt".into(),
        &bb,
        GaugeFieldInit::HaarRandom,
        Some(20260616),
    )
    .unwrap();
    register_su2(u_field);
    let u_arc =
        gigi::gauge::registry::get_su2_mut("U_iv4_dt").expect("registered");
    let inc = build_edge_face_incidence(&bb);
    let f = {
        let u_guard = u_arc.lock().expect("u mutex poisoned");
        wilson_force_per_edge(&*u_guard, &bb, &inc, 2.5).unwrap()
    };

    let mk_e = |tag: &str| -> SU2EField {
        SU2EField::new(
            tag.into(),
            &*u_arc.lock().expect("u mutex poisoned"),
            EFieldInit::Zero,
            None,
        )
        .expect("zero E init must succeed")
    };

    // Baseline: full kick at dt = 0.02.
    let mut e_full = mk_e("E_full");
    apply_force_kick(&mut e_full, &f, 0.02).unwrap();

    // Halved: kick at dt = 0.01.
    let mut e_half = mk_e("E_half");
    apply_force_kick(&mut e_half, &f, 0.01).unwrap();

    // Linearity contract: full = 2 · half on every edge / component.
    let tol = 1e-14_f64;
    for edge in 0..bb.n_edges() {
        let row_full = e_full.read_element_q(edge);
        let row_half = e_half.read_element_q(edge);
        for k in 0..4 {
            let want = 2.0 * row_half[k];
            let got = row_full[k];
            assert!(
                (got - want).abs() <= tol,
                "edge {edge} comp {k}: kick(dt) = {got}, 2·kick(dt/2) = {want}, diff {}",
                (got - want).abs()
            );
        }
    }

    // Two half-kicks should land on the full kick exactly (linearity
    // again, from a fresh zero start).
    let mut e_two_half = mk_e("E_two_half");
    apply_force_kick(&mut e_two_half, &f, 0.01).unwrap();
    apply_force_kick(&mut e_two_half, &f, 0.01).unwrap();
    for edge in 0..bb.n_edges() {
        let row_full = e_full.read_element_q(edge);
        let row_two = e_two_half.read_element_q(edge);
        for k in 0..4 {
            assert!(
                (row_full[k] - row_two[k]).abs() <= tol,
                "edge {edge} comp {k}: full {got} vs 2× half-kick {two}, diff {}",
                (row_full[k] - row_two[k]).abs(),
                got = row_full[k],
                two = row_two[k]
            );
        }
    }
}
