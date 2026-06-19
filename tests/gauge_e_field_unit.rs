//! TDD-HAL-IV.1 red test — E-field primitive (SU2EField sibling +
//! Lie buffer + INIT routines).
//!
//! These tests pin the contract for the SU(2) E-field primitive that
//! Part IV's symplectic flow consumes. SU2EField is a sibling struct
//! to SU2GaugeField — it does NOT impl `EdgeConnection` (the E field
//! has no group inverse / face-walk semantics) and ships its own
//! registry slot (`register_su2_e` / `get_su2_e_mut`) parallel to the
//! U field's. The buffer is a `(n_edges, 4)` quaternion-packed Lie
//! algebra storage layout with `q0 = 0` enforced as a hard invariant
//! (E is the imaginary-quaternion tangent direction).
//!
//! Gate locked decisions in play:
//!   - IV-B + IV-C: sibling struct + (n_edges, 4) q0=0 buffer.
//!   - q0=0 invariant enforced at every constructor entry point AND
//!     on every buffer mutation.
//!   - Maxwell–Boltzmann sigma: σ = sqrt(1.0 / (beta * 1.5)) — the
//!     Halcyon canonical_sigma packing for SU(2) (dim/2 = 3/2 = 1.5).
//!   - Per-edge MB draw: 3 standard normals via Box–Muller through
//!     `SmallRng`; q0 forced to 0; q_k = sigma * g_k for k = 1, 2, 3.
//!   - A2 row 1: same seed → byte-identical buffer (intra-binding
//!     bit-identity).

#![cfg(feature = "gauge")]

use gigi::gauge::{
    e_field::{EFieldHandle, EFieldInit, SU2EField},
    error::GaugeFieldError,
    group::Group,
    registry::{
        clear as clear_gauge, clear_e_registry, get_su2_e_mut, register_su2,
        register_su2_e, test_serial_lock,
    },
    su2_gauge_field::{GaugeFieldInit, SU2GaugeField},
};
use gigi::lattice::{
    registry as lattice_registry,
    topology::truncated_icosahedron::buckyball,
    Lattice,
};
use std::sync::{Arc, Mutex};

/// Build a tiny non-buckyball lattice for the cross-lattice mismatch
/// test. Two vertices, one edge, no faces — we only need a distinct
/// `n_edges` and a distinct `name`.
fn tiny_lattice(name: &str) -> Lattice {
    Lattice::new(
        name.to_string(),
        2,
        vec![(0, 1)],
        Vec::<Vec<usize>>::new(),
        None,
    )
}

/// TDD-HAL-IV.1: `SU2EField::new(..., EFieldInit::Zero, None)` returns
/// a (n_edges=90, 4) f64 buffer of zeros with the q0=0 invariant
/// satisfied trivially. Frozen-bytes shape check.
#[test]
fn tdd_hal_iv_1_e_field_zero_init() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_zero_init".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .expect("identity U init must succeed");

    let e = SU2EField::new(
        "E_zero".into(),
        &u_field,
        EFieldInit::Zero,
        None,
    )
    .expect("zero E init must succeed");

    assert_eq!(e.buffer.group, Group::SU2);
    assert_eq!(e.buffer.n_edges, 90);
    assert_eq!(e.buffer.repr_dim, 4);
    assert_eq!(e.buffer.data.len(), 360);
    assert_eq!(e.source_gauge_field, "U_zero_init");
    assert_eq!(e.source_lattice, bb.name);
    assert_eq!(e.init_kind, EFieldInit::Zero);
    assert_eq!(e.init_seed, None);

    // Every row [q0, q1, q2, q3] is exactly zero.
    for i in 0..90 {
        for j in 0..4 {
            assert_eq!(
                e.buffer.data[4 * i + j],
                0.0,
                "edge {i} component {j} not zero"
            );
        }
    }
}

/// TDD-HAL-IV.1: two MB inits with the same seed produce byte-identical
/// buffers. A2 row 1 intra-binding bit-identity contract.
#[test]
fn tdd_hal_iv_1_e_field_mb_byte_equal_same_seed() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_mb_bit".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();

    let a = SU2EField::new(
        "E_a".into(),
        &u_field,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .expect("MB init must succeed");
    let b = SU2EField::new(
        "E_b".into(),
        &u_field,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .expect("MB init must succeed");

    assert_eq!(a.buffer.data, b.buffer.data);
    assert_eq!(a.buffer.data.len(), 360);
}

/// TDD-HAL-IV.1: q0 = 0 on every edge of an MB-initialized buffer
/// (Lie-algebra invariant; E lives in su(2), the imaginary
/// quaternions).
#[test]
fn tdd_hal_iv_1_e_field_mb_q0_zero() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_q0_zero".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();

    let e = SU2EField::new(
        "E_q0".into(),
        &u_field,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(20260617),
    )
    .unwrap();

    for i in 0..90 {
        assert_eq!(
            e.buffer.data[4 * i],
            0.0,
            "edge {i} q0 not zero (Lie-algebra invariant violated)"
        );
    }
}

/// TDD-HAL-IV.1: marginal statistics on MB samples — across 20 seeds ×
/// 90 edges = 1800 samples, |mean(q_k)| < 0.05 for k=1,2,3 (symmetry)
/// and per-component variance is within 10% of σ² = 1/(β·1.5) for
/// β = 2.5 → σ² = 0.2667 (Halcyon canonical_sigma packing). Bounds
/// loose enough to be "right distribution" guards, not precision
/// PRNG audits.
#[test]
fn tdd_hal_iv_1_e_field_mb_marginal_stats() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_stats".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();

    let beta = 2.5_f64;
    let target_var = 1.0 / (beta * 1.5);
    let mut sums = [0.0_f64; 3];
    let mut sqs = [0.0_f64; 3];
    let mut n = 0_usize;
    for seed in 0..20_u64 {
        let e = SU2EField::new(
            format!("E_stats_{seed}"),
            &u_field,
            EFieldInit::MaxwellBoltzmann { beta },
            Some(seed + 1),
        )
        .unwrap();
        for edge in 0..90 {
            let base = 4 * edge;
            for k in 0..3 {
                let v = e.buffer.data[base + 1 + k];
                sums[k] += v;
                sqs[k] += v * v;
            }
            n += 1;
        }
    }
    for k in 0..3 {
        let mean = sums[k] / n as f64;
        let var = sqs[k] / n as f64 - mean * mean;
        assert!(
            mean.abs() < 0.05,
            "component {k}: |mean| = {} >= 0.05",
            mean.abs()
        );
        let rel = (var - target_var).abs() / target_var;
        assert!(
            rel < 0.10,
            "component {k}: var = {var}, target = {target_var}, rel = {rel} >= 0.10"
        );
    }
}

/// TDD-HAL-IV.1: FromField clones the buffer byte-for-byte from another
/// declared E field bound to the same source lattice.
#[test]
fn tdd_hal_iv_1_e_field_from_field_clones() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_clone".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    register_su2(u_field.clone());

    let e1 = SU2EField::new(
        "E1_clone_src".into(),
        &u_field,
        EFieldInit::MaxwellBoltzmann { beta: 1.0 },
        Some(1),
    )
    .unwrap();
    register_su2_e(Arc::new(Mutex::new(e1.clone())));

    let e2 = SU2EField::new(
        "E2_clone_dst".into(),
        &u_field,
        EFieldInit::FromField("E1_clone_src".into()),
        None,
    )
    .expect("FromField with matching source lattice must succeed");

    assert_eq!(e1.buffer.data, e2.buffer.data);
    assert_eq!(e1.buffer.data.len(), 360);
    assert_eq!(e2.source_gauge_field, "U_clone");
    assert_eq!(e2.source_lattice, bb.name);
}

/// TDD-HAL-IV.1: FromField across a lattice boundary surfaces the
/// typed `EFieldSourceMismatch` error. Build E1 on lattice A
/// (buckyball), then try to clone it into an E bound to a U field on
/// lattice B (tiny).
#[test]
fn tdd_hal_iv_1_e_field_source_mismatch() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();

    let bb = buckyball();
    lattice_registry::register(bb.clone());
    let tiny = tiny_lattice("tiny_other");
    lattice_registry::register(tiny.clone());

    // Source U + E on buckyball.
    let u_bb = SU2GaugeField::new(
        "U_on_bb".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    register_su2(u_bb.clone());
    let e_bb = SU2EField::new(
        "E_on_bb".into(),
        &u_bb,
        EFieldInit::MaxwellBoltzmann { beta: 1.0 },
        Some(7),
    )
    .unwrap();
    register_su2_e(Arc::new(Mutex::new(e_bb.clone())));

    // Target U on the tiny lattice.
    let u_tiny = SU2GaugeField::new(
        "U_on_tiny".into(),
        &tiny,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    register_su2(u_tiny.clone());

    let err = SU2EField::new(
        "E_should_fail".into(),
        &u_tiny,
        EFieldInit::FromField("E_on_bb".into()),
        None,
    )
    .expect_err("cross-lattice FromField must error");
    match err {
        GaugeFieldError::EFieldSourceMismatch {
            e_lattice,
            u_lattice,
        } => {
            assert_eq!(e_lattice, bb.name);
            assert_eq!(u_lattice, tiny.name);
        }
        other => panic!("expected EFieldSourceMismatch, got {other:?}"),
    }
}

/// TDD-HAL-IV.1: sibling registry round-trip — register, look up via
/// `get_su2_e_mut`, lock, and confirm the buffer round-trips
/// byte-for-byte.
#[test]
fn tdd_hal_iv_1_register_round_trip() {
    let _serial = test_serial_lock();
    clear_gauge();
    clear_e_registry();
    lattice_registry::clear();
    let bb = buckyball();
    lattice_registry::register(bb.clone());

    let u_field = SU2GaugeField::new(
        "U_reg_rt".into(),
        &bb,
        GaugeFieldInit::Identity,
        None,
    )
    .unwrap();
    let e_field = SU2EField::new(
        "E_reg_rt".into(),
        &u_field,
        EFieldInit::MaxwellBoltzmann { beta: 2.5 },
        Some(42),
    )
    .unwrap();
    let baseline = e_field.buffer.data.clone();

    register_su2_e(Arc::new(Mutex::new(e_field)));
    let got = get_su2_e_mut("E_reg_rt").expect("just registered");
    let guard = got.lock().expect("e field mutex");
    assert_eq!(guard.name, "E_reg_rt");
    assert_eq!(guard.source_gauge_field, "U_reg_rt");
    assert_eq!(guard.source_lattice, bb.name);
    assert_eq!(guard.buffer.data, baseline);
    assert_eq!(guard.buffer.data.len(), 360);
    // EFieldHandle accessors land too.
    let h: &SU2EField = &guard;
    assert_eq!(h.name(), "E_reg_rt");
    assert_eq!(h.source_gauge_field(), "U_reg_rt");
    assert_eq!(h.source_lattice(), bb.name);
    assert_eq!(h.group(), Group::SU2);
    let (kind, seed) = h.init_metadata();
    assert_eq!(kind, EFieldInit::MaxwellBoltzmann { beta: 2.5 });
    assert_eq!(seed, Some(42));
}
