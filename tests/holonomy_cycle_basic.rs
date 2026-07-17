//! HOLONOMY AROUND CYCLE — Poincaré Tier 1 readout verb (SU(2)).
//!
//! Math anchors H1..H7 for `HOLONOMY <field> AROUND CYCLE {AXIS <ax> AT
//! (c0, c1) | EDGES (e0, e1, ...)}`. Davis–Poincaré Thm 3.6: the SU(2)
//! holonomy around a non-contractible lattice loop (+ its order) is the
//! distinguishing observable for a lens space `L(p, q) = S³/ℤ_p`.
//!
//!   H1 identity loop      → holonomy (1,0,0,0), re_trace 1.0 exact, order 1.
//!   H2 lens wrap          → Ω exactly, re_trace = cos(2πq/p) to 1e-12,
//!                           order = p for (p,q) ∈ {(2,1),(3,1),(5,1),
//!                           (5,2),(7,1),(7,3)}. Core acceptance.
//!   H3 direction pin      → reversing a loop (reverse order + inverted
//!                           links) yields the conjugate quaternion; the
//!                           ordered product is direction-sensitive
//!                           (A·B ≠ B·A). Pins which way AXIS walks.
//!   H4 EDGES vs AXIS      → the AXIS z form at (x0,y0) produces the same
//!                           ordered edge set (hence same holonomy) as the
//!                           closed-form explicit EDGES list.
//!   H5 composite loop     → Ω_a·Ω_b is the ordered group product
//!                           (hand-computed), a genuine non-scalar
//!                           quaternion — not a product of re_traces.
//!   H6 non-SU(2) group    → typed error naming the group.
//!   H7 AXIS w/o lattice   → typed error naming the missing binding.
//!
//! Run: `cargo test --features halcyon --test holonomy_cycle_basic`
//!
//! RED at the base commit: `parse_holonomy` has only ON / NEAR arms, so
//! every `parse("HOLONOMY ... AROUND CYCLE ...")` returns Err and every
//! test fails at fixture setup. GREEN lands the `AROUND CYCLE` grammar,
//! the `Statement::HolonomyCycle` variant, and
//! `holonomy_cycle::execute_holonomy_cycle`.

#![cfg(feature = "halcyon")]

use std::sync::Arc;

use gigi::engine::Engine;
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::registry as gauge_registry;
use gigi::gauge::{
    DenseLinkBuffer, GaugeFieldHandle, GaugeFieldInit, Group, SU2GaugeField, SU3GaugeField,
};
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::Lattice;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{Record, Value};

// ── Fixtures ─────────────────────────────────────────────────────────

/// Open a fresh engine in a tempdir with both registries cleared. The
/// returned serial guard must live for the whole test — the gauge +
/// lattice registries are process-global singletons, so intra-file
/// parallel tests are serialized against each other.
fn open_engine() -> (Engine, tempfile::TempDir, std::sync::MutexGuard<'static, ()>) {
    let guard = gauge_registry::test_serial_lock();
    gauge_registry::clear();
    lattice_registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir, guard)
}

/// Row-major cubic site id: `v(x,y,z) = x + y·L + z·L²`.
fn site(x: usize, y: usize, z: usize, l: usize) -> usize {
    x + y * l + z * l * l
}

/// Lens wrap quaternion `Ω = (cos 2πq/p, 0, 0, sin 2πq/p)` = a σ₃-twist.
fn omega(p: usize, q: usize) -> [f64; 4] {
    let ang = 2.0 * std::f64::consts::PI * q as f64 / p as f64;
    [ang.cos(), 0.0, 0.0, ang.sin()]
}

/// Declare a periodic cubic lattice and return the materialized value.
fn declare_cubic(eng: &mut Engine, name: &str, l: usize, d: usize) -> Lattice {
    let gql = format!("LATTICE {name} FROM CUBIC L={l} DIM={d} PERIODIC;");
    let stmt = parse(&gql).expect("parse LATTICE");
    execute(eng, &stmt).expect("exec LATTICE");
    lattice_registry::get(name).expect("lattice registered")
}

/// Register an SU(2) gauge field bound to `lat`, identity everywhere
/// except the listed `(edge_id, quaternion)` overrides. Uses the
/// test-only `from_buffer` factory so we can plant twisted-BC links
/// directly (the GAUGE_FIELD grammar only offers IDENTITY / HAAR).
fn register_su2(name: &str, lat: &Lattice, overrides: &[(usize, [f64; 4])]) {
    let n_edges = lat.n_edges();
    let mut buf = DenseLinkBuffer::new_identity(Group::SU2, n_edges).expect("identity SU2 buffer");
    for &(eid, q) in overrides {
        let base = 4 * eid;
        buf.data[base] = q[0];
        buf.data[base + 1] = q[1];
        buf.data[base + 2] = q[2];
        buf.data[base + 3] = q[3];
    }
    let field =
        SU2GaugeField::from_buffer(name.to_string(), lat.name.clone(), buf, GaugeFieldInit::Identity, None);
    let h: Arc<dyn GaugeFieldHandle> = Arc::new(field);
    gauge_registry::register(h);
}

/// Run a HOLONOMY AROUND CYCLE statement and return its single row.
fn run_cycle(eng: &mut Engine, gql: &str) -> Record {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse '{gql}': {e}"));
    let res = execute(eng, &stmt).unwrap_or_else(|e| panic!("exec '{gql}': {e}"));
    match res {
        ExecResult::Rows(mut rs) => {
            assert_eq!(rs.len(), 1, "HOLONOMY AROUND CYCLE returns exactly one row");
            rs.pop().unwrap()
        }
        other => panic!("expected ExecResult::Rows, got {other:?}"),
    }
}

/// Run a HOLONOMY AROUND CYCLE statement expected to error.
fn run_cycle_err(eng: &mut Engine, gql: &str) -> String {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse '{gql}': {e}"));
    execute(eng, &stmt).expect_err("expected a typed error")
}

fn f(row: &Record, k: &str) -> f64 {
    match row.get(k) {
        Some(Value::Float(v)) => *v,
        other => panic!("field '{k}' not a Float: {other:?}"),
    }
}
fn int(row: &Record, k: &str) -> i64 {
    match row.get(k) {
        Some(Value::Integer(v)) => *v,
        other => panic!("field '{k}' not an Integer: {other:?}"),
    }
}
fn text(row: &Record, k: &str) -> String {
    match row.get(k) {
        Some(Value::Text(v)) => v.clone(),
        other => panic!("field '{k}' not Text: {other:?}"),
    }
}

// ── H1 — identity loop ───────────────────────────────────────────────

/// All links identity → holonomy (1,0,0,0), re_trace 1.0 EXACT, order 1.
/// (The p=1 trivial-twist control — the S³ analog.) Asserted through
/// BOTH the AXIS form (all-identity z-cycle) and the EDGES form.
#[test]
fn h1_identity_loop_is_trivial() {
    let (mut eng, _dir, _serial) = open_engine();
    let lat = declare_cubic(&mut eng, "lat_h1", 5, 3);
    register_su2("u_h1", &lat, &[]); // identity everywhere

    // AXIS form.
    let row = run_cycle(&mut eng, "HOLONOMY u_h1 AROUND CYCLE AXIS z AT (1, 2);");
    assert_eq!(f(&row, "q0"), 1.0, "identity q0 exact");
    assert_eq!(f(&row, "q1"), 0.0);
    assert_eq!(f(&row, "q2"), 0.0);
    assert_eq!(f(&row, "q3"), 0.0);
    assert_eq!(f(&row, "re_trace"), 1.0, "re_trace = q0 = 1.0 exact");
    assert_eq!(int(&row, "order_estimate"), 1, "identity → order 1");
    assert_eq!(text(&row, "group_used"), "SU(2)");

    // EDGES form on the same identity field — three arbitrary edges.
    let row = run_cycle(&mut eng, "HOLONOMY u_h1 AROUND CYCLE EDGES (0, 1, 2);");
    assert_eq!(f(&row, "q0"), 1.0);
    assert_eq!(int(&row, "order_estimate"), 1);
}

// ── H2 — lens wrap (core acceptance) ─────────────────────────────────

/// A z-cycle at (x0,y0) with ONE wrap link Ω = (cos 2πq/p, 0, 0, sin
/// 2πq/p) and all interior z-links identity → holonomy = Ω exactly,
/// re_trace = cos(2πq/p) to 1e-12, order_estimate = p. The order tracks
/// π₁ = ℤ/p. This is the receipt Hallie's lens readout depends on.
#[test]
fn h2_lens_wrap_order_tracks_p() {
    let (mut eng, _dir, _serial) = open_engine();
    let l = 5usize;
    let (x0, y0) = (1usize, 2usize);
    let lat = declare_cubic(&mut eng, "lat_h2", l, 3);
    // The wrap edge along +z at (x0,y0): site(z=L-1) → site(z=0).
    let wrap = lat
        .resolve_edge(site(x0, y0, l - 1, l), site(x0, y0, 0, l))
        .expect("periodic z-wrap edge exists")
        .0;

    for (p, q) in [(2usize, 1usize), (3, 1), (5, 1), (5, 2), (7, 1), (7, 3)] {
        gauge_registry::remove("u_h2");
        register_su2("u_h2", &lat, &[(wrap, omega(p, q))]);
        let ang = 2.0 * std::f64::consts::PI * q as f64 / p as f64;

        let row = run_cycle(&mut eng, "HOLONOMY u_h2 AROUND CYCLE AXIS z AT (1, 2);");
        assert!(
            (f(&row, "q0") - ang.cos()).abs() < 1e-12,
            "(p={p},q={q}) q0 must equal cos(2πq/p)={}, got {}",
            ang.cos(),
            f(&row, "q0")
        );
        assert!(f(&row, "q1").abs() < 1e-12, "(p={p},q={q}) q1≈0");
        assert!(f(&row, "q2").abs() < 1e-12, "(p={p},q={q}) q2≈0");
        assert!(
            (f(&row, "q3") - ang.sin()).abs() < 1e-12,
            "(p={p},q={q}) q3 must equal +sin(2πq/p)={} (Ω not Ω†), got {}",
            ang.sin(),
            f(&row, "q3")
        );
        assert!(
            (f(&row, "re_trace") - ang.cos()).abs() < 1e-12,
            "(p={p},q={q}) re_trace = cos(2πq/p)"
        );
        assert_eq!(
            int(&row, "order_estimate"),
            p as i64,
            "(p={p},q={q}) order_estimate must equal p"
        );
        assert_eq!(text(&row, "group_used"), "SU(2)");
    }
}

// ── H3 — direction convention pin ────────────────────────────────────

/// Reversing a loop (reverse order + invert each link) yields the group
/// inverse = the conjugate quaternion (q0, −q1, −q2, −q3); and the
/// ordered product is direction-sensitive (A·B ≠ B·A). Together these
/// PIN which direction AXIS walks: because the forward-stored z-links
/// are read Forward (U, not U†), a +z AXIS walk returns Ω, and its
/// reverse would return Ω†. A silently-reversed convention would read
/// every class p as its inverse.
#[test]
fn h3_reverse_loop_is_conjugate_and_order_sensitive() {
    let (mut eng, _dir, _serial) = open_engine();
    let lat = declare_cubic(&mut eng, "lat_h3", 5, 3);

    // Two non-commuting rotations (θ = π/2 each): A = σ₃-twist about z,
    // B = σ₁-twist about x; plus their inverses on separate edges.
    let hp = std::f64::consts::FRAC_1_SQRT_2; // cos(π/4) = sin(π/4)
    let a = [hp, 0.0, 0.0, hp];
    let b = [hp, hp, 0.0, 0.0];
    let a_inv = [hp, 0.0, 0.0, -hp];
    let b_inv = [hp, -hp, 0.0, 0.0];
    register_su2(
        "u_h3",
        &lat,
        &[(0, a), (1, b), (2, b_inv), (3, a_inv)],
    );

    // Forward loop A·B.
    let fwd = run_cycle(&mut eng, "HOLONOMY u_h3 AROUND CYCLE EDGES (0, 1);");
    // Reversed loop: reverse order AND invert each link → B⁻¹·A⁻¹ = (A·B)⁻¹.
    let rev = run_cycle(&mut eng, "HOLONOMY u_h3 AROUND CYCLE EDGES (2, 3);");

    assert!((f(&rev, "q0") - f(&fwd, "q0")).abs() < 1e-12, "conj: q0 unchanged");
    assert!((f(&rev, "q1") + f(&fwd, "q1")).abs() < 1e-12, "conj: q1 negated");
    assert!((f(&rev, "q2") + f(&fwd, "q2")).abs() < 1e-12, "conj: q2 negated");
    assert!((f(&rev, "q3") + f(&fwd, "q3")).abs() < 1e-12, "conj: q3 negated");

    // Ordered product is direction-sensitive: B·A ≠ A·B (q2 flips sign).
    let ba = run_cycle(&mut eng, "HOLONOMY u_h3 AROUND CYCLE EDGES (1, 0);");
    assert!(
        (f(&ba, "q2") - f(&fwd, "q2")).abs() > 0.5,
        "B·A must differ from A·B (non-commuting) — q2 sign flip"
    );
}

// ── H4 — EDGES vs AXIS parity ────────────────────────────────────────

/// The AXIS z form at (x0,y0) produces the SAME ordered edge set (hence
/// same holonomy) as the closed-form explicit EDGES list `eid =
/// 2·V + site(x0,y0,z)` for z = 0..L. This cross-checks the search-based
/// AXIS enumeration (via resolve_edge) against the cubic edge-layout
/// formula (axis-major: axis a block starts at a·V).
#[test]
fn h4_axis_equals_explicit_edges() {
    let (mut eng, _dir, _serial) = open_engine();
    let l = 5usize;
    let (x0, y0) = (1usize, 2usize);
    let v = l * l * l; // 125
    let lat = declare_cubic(&mut eng, "lat_h4", l, 3);
    let wrap = lat
        .resolve_edge(site(x0, y0, l - 1, l), site(x0, y0, 0, l))
        .unwrap()
        .0;
    register_su2("u_h4", &lat, &[(wrap, omega(5, 1))]);

    let axis = run_cycle(&mut eng, "HOLONOMY u_h4 AROUND CYCLE AXIS z AT (1, 2);");

    // Closed-form z-links at (x0,y0): axis 2 block starts at 2·V.
    let ids: Vec<String> = (0..l)
        .map(|z| (2 * v + site(x0, y0, z, l)).to_string())
        .collect();
    let edges_gql = format!("HOLONOMY u_h4 AROUND CYCLE EDGES ({});", ids.join(", "));
    let edges = run_cycle(&mut eng, &edges_gql);

    for k in ["q0", "q1", "q2", "q3", "re_trace"] {
        assert!(
            (f(&axis, k) - f(&edges, k)).abs() < 1e-15,
            "AXIS vs EDGES parity on {k}: {} vs {}",
            f(&axis, k),
            f(&edges, k)
        );
    }
    assert_eq!(int(&axis, "order_estimate"), int(&edges, "order_estimate"));
    assert_eq!(int(&axis, "order_estimate"), 5, "p=5 fixture");
}

// ── H5 — composite loop is a genuine group product ───────────────────

/// A loop with two non-identity links Ω_a·Ω_b (σ₃-twist then σ₁-twist)
/// → holonomy = the ordered quaternion product (hand-computed), a
/// genuine non-scalar element with a non-zero cross-term q2 = −a3·b1.
/// This is NOT what multiplying the two re_traces would give (0.5), and
/// the non-zero vector part proves it is a real group walk, not a
/// scalar collapse.
#[test]
fn h5_composite_loop_is_ordered_group_product() {
    let (mut eng, _dir, _serial) = open_engine();
    let lat = declare_cubic(&mut eng, "lat_h5", 5, 3);
    let hp = std::f64::consts::FRAC_1_SQRT_2;
    let a = [hp, 0.0, 0.0, hp]; // σ₃-twist, θ=π/2
    let b = [hp, hp, 0.0, 0.0]; // σ₁-twist, θ=π/2
    register_su2("u_h5", &lat, &[(0, a), (1, b)]);

    let row = run_cycle(&mut eng, "HOLONOMY u_h5 AROUND CYCLE EDGES (0, 1);");
    // Hand-computed A·B = (a0b0, a0b1, −a3b1, b0a3) = (0.5, 0.5, −0.5, 0.5).
    assert!((f(&row, "q0") - 0.5).abs() < 1e-12, "q0");
    assert!((f(&row, "q1") - 0.5).abs() < 1e-12, "q1");
    assert!((f(&row, "q2") + 0.5).abs() < 1e-12, "q2 = −0.5 (the cross term)");
    assert!((f(&row, "q3") - 0.5).abs() < 1e-12, "q3");
    // Genuine group element: all three vector components are non-trivial.
    assert!(f(&row, "q1").abs() > 0.1 && f(&row, "q2").abs() > 0.1 && f(&row, "q3").abs() > 0.1);

    // Cross-check against the group-element compose directly.
    let expected =
        GroupElement::SU2 { q0: a[0], q1: a[1], q2: a[2], q3: a[3] }
            .compose(&GroupElement::SU2 { q0: b[0], q1: b[1], q2: b[2], q3: b[3] });
    if let GroupElement::SU2 { q0, q1, q2, q3 } = expected {
        assert!((f(&row, "q0") - q0).abs() < 1e-12);
        assert!((f(&row, "q1") - q1).abs() < 1e-12);
        assert!((f(&row, "q2") - q2).abs() < 1e-12);
        assert!((f(&row, "q3") - q3).abs() < 1e-12);
    }
}

// ── H6 — non-SU(2) group is a typed error ────────────────────────────

/// A non-SU(2) field (SU(3) here) → typed error naming the group. The
/// gate fires BEFORE the walker, which would otherwise panic on a
/// mixed-variant compose. (U(1) is the live-probe P6 analog.)
#[test]
fn h6_non_su2_group_is_typed_error() {
    let (mut eng, _dir, _serial) = open_engine();
    let lat = declare_cubic(&mut eng, "lat_h6", 5, 3);
    // Build an SU(3) identity field on the same lattice and register it.
    let field = SU3GaugeField::new("u_h6".to_string(), &lat, GaugeFieldInit::Identity, None)
        .expect("su3 identity field");
    let h: Arc<dyn GaugeFieldHandle> = Arc::new(field);
    gauge_registry::register(h);

    // EDGES form (no lattice needed) — the group gate must fire first.
    let err = run_cycle_err(&mut eng, "HOLONOMY u_h6 AROUND CYCLE EDGES (0, 1, 2);");
    assert!(
        err.contains("SU(2)") && err.contains("SU(3)"),
        "error must name the required + actual group, got: {err}"
    );
    // AXIS form on the same field errors identically (gate precedes lattice).
    let err2 = run_cycle_err(&mut eng, "HOLONOMY u_h6 AROUND CYCLE AXIS z AT (0, 0);");
    assert!(err2.contains("SU(3)"), "AXIS form: {err2}");
}

// ── H7 — AXIS with no bound lattice is a typed error ─────────────────

/// AXIS form on a field whose bound lattice is not registered → typed
/// error naming the missing binding. Also covers the non-cubic case
/// (a lattice that exists but is not CUBIC). The EDGES form needs no
/// lattice, so it is unaffected.
#[test]
fn h7_axis_without_lattice_is_typed_error() {
    let (mut eng, _dir, _serial) = open_engine();

    // (a) Field bound by name to a lattice that was never declared.
    let buf = DenseLinkBuffer::new_identity(Group::SU2, 8).unwrap();
    let ghost = SU2GaugeField::from_buffer(
        "u_h7".to_string(),
        "ghost_never_declared".to_string(),
        buf,
        GaugeFieldInit::Identity,
        None,
    );
    let h: Arc<dyn GaugeFieldHandle> = Arc::new(ghost);
    gauge_registry::register(h);

    let err = run_cycle_err(&mut eng, "HOLONOMY u_h7 AROUND CYCLE AXIS z AT (0, 0);");
    assert!(
        err.contains("ghost_never_declared") && err.contains("not found"),
        "error must name the missing bound lattice, got: {err}"
    );
    // The EDGES form does NOT need the lattice — it succeeds.
    let ok = run_cycle(&mut eng, "HOLONOMY u_h7 AROUND CYCLE EDGES (0, 1);");
    assert_eq!(text(&ok, "group_used"), "SU(2)");

    // (b) Bound lattice exists but is not CUBIC (buckyball, topology S2).
    lattice_registry::clear();
    gauge_registry::clear();
    let bb_stmt = parse("LATTICE bb_h7 FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';")
        .expect("parse buckyball lattice");
    execute(&mut eng, &bb_stmt).expect("exec buckyball lattice");
    let bb = lattice_registry::get("bb_h7").expect("buckyball registered");
    register_su2("u_h7b", &bb, &[]);
    let err_nc = run_cycle_err(&mut eng, "HOLONOMY u_h7b AROUND CYCLE AXIS z AT (0, 0);");
    assert!(
        err_nc.contains("CUBIC"),
        "non-cubic binding must error naming CUBIC, got: {err_nc}"
    );
}
