//! GAUGE_FIELD INIT FROM BUNDLE — chosen-field → gauge-registry seam.
//!
//! The missing half of the Poincaré / Navier–Stokes holonomy story: the
//! `GAUGE_FIELD … INIT …` surface offered IDENTITY / HAAR / FROM(field) /
//! FLUX(U1) only — there was no GQL path to load a *chosen* per-edge SU(2)
//! field into `gauge::registry`, so the lens-space p-sweep receipt was
//! gate-locked to unit tests (`holonomy_cycle_basic::h2_*`) and could not
//! run through the live bundle → registry → HOLONOMY path.
//!
//! `GAUGE_FIELD <name> GROUP SU(2) INIT FROM BUNDLE <bundle> ON LATTICE <l>`
//! reads an edge-endpoint bundle (base `edge_id, vertex_a, vertex_b`, fiber
//! `q0, q1, q2, q3` — the same schema INGEST AS GAUGE_FIELD / INIT FLUX use),
//! builds a registry `DenseLinkBuffer`, and registers it under `<name>` so
//! HOLONOMY reads it back with the SAME edge → slot mapping + orientation
//! convention CHERN_CLASS/HOLONOMY walk with.
//!
//! Anchors (per the ship spec):
//!   I1  ROUND-TRIP     chosen SU(2) per edge → INIT FROM BUNDLE → read back
//!                      via handle.edge_element(Forward) == stored Ω (1e-12).
//!   I1b ORIENTATION    a reverse-oriented record round-trips as Ω on the
//!                      declared va→vb direction (not Ω†) — the load-bearing
//!                      pin.
//!   I2  LENS p-SWEEP   twisted-BC bundle (z-wrap Ω, rest identity) → INIT
//!                      FROM BUNDLE → HOLONOMY AROUND CYCLE AXIS z →
//!                      re_trace = cos(2πq/p), order = p for (p,q) ∈
//!                      {(2,1),(3,1),(5,1),(5,2),(7,1),(7,3)}. THE receipt.
//!   I3  p=1 CONTROL    identity bundle → re_trace 1.0, order 1.
//!   I4  U(1) DEFERRAL  GROUP U(1) INIT FROM BUNDLE is a clean typed error
//!                      this phase (SU(2) ships; U(1) is a named fast-follow).
//!   I5  TYPED ERRORS   bundle-not-found / arity mismatch / non-lattice edge
//!                      / non-normalized quaternion / empty bundle — all
//!                      return typed errors (no panics).
//!
//! Run: `cargo test --features halcyon --test gauge_inject_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::edge_connection::EdgeConnection;
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::registry as gauge_registry;
use gigi::gauge::GaugeFieldHandle;
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::{EdgeOrientation, Lattice};
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── Fixtures ─────────────────────────────────────────────────────────

/// Open a fresh engine with both singleton registries cleared. The
/// returned serial guard must live for the whole test.
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

/// Lens wrap quaternion `Ω = (cos 2πq/p, 0, 0, sin 2πq/p)` — a σ₃-twist.
fn omega(p: usize, q: usize) -> [f64; 4] {
    let ang = 2.0 * std::f64::consts::PI * q as f64 / p as f64;
    [ang.cos(), 0.0, 0.0, ang.sin()]
}

/// Declare a periodic cubic lattice via the parser and return it.
fn declare_cubic(eng: &mut Engine, name: &str, l: usize, d: usize) -> Lattice {
    let gql = format!("LATTICE {name} FROM CUBIC L={l} DIM={d} PERIODIC;");
    let stmt = parse(&gql).expect("parse LATTICE");
    execute(eng, &stmt).expect("exec LATTICE");
    lattice_registry::get(name).expect("lattice registered")
}

/// The canonical SU(2) edge-endpoint bundle schema (matches INIT FLUX /
/// INGEST AS GAUGE_FIELD): base `config_id, edge_id, vertex_a, vertex_b`,
/// fiber `q0, q1, q2, q3`.
fn su2_bundle_schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
        .base(FieldDef::numeric("config_id"))
        .base(FieldDef::numeric("edge_id"))
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("q0"))
        .fiber(FieldDef::numeric("q1"))
        .fiber(FieldDef::numeric("q2"))
        .fiber(FieldDef::numeric("q3"))
}

/// One SU(2) edge record in canonical (va → vb) orientation.
fn su2_record(edge_id: usize, va: usize, vb: usize, q: [f64; 4]) -> Record {
    let mut rec = Record::new();
    rec.insert("config_id".to_string(), Value::Integer(0));
    rec.insert("edge_id".to_string(), Value::Integer(edge_id as i64));
    rec.insert("vertex_a".to_string(), Value::Integer(va as i64));
    rec.insert("vertex_b".to_string(), Value::Integer(vb as i64));
    rec.insert("q0".to_string(), Value::Float(q[0]));
    rec.insert("q1".to_string(), Value::Float(q[1]));
    rec.insert("q2".to_string(), Value::Float(q[2]));
    rec.insert("q3".to_string(), Value::Float(q[3]));
    rec
}

/// Materialize a full SU(2) bundle for `lat`: one record per lattice edge
/// (emitted in the lattice's own `edges[k] = (u, v)` order → canonical
/// Forward orientation, exactly what Hallie's lens emitter and INIT FLUX
/// produce), identity everywhere except the `(edge_id → quaternion)`
/// overrides.
fn build_su2_bundle(
    eng: &mut Engine,
    bundle: &str,
    lat: &Lattice,
    overrides: &[(usize, [f64; 4])],
) {
    eng.create_bundle(su2_bundle_schema(bundle))
        .expect("create_bundle");
    let ident = [1.0, 0.0, 0.0, 0.0];
    let recs: Vec<Record> = lat
        .edges
        .iter()
        .enumerate()
        .map(|(eid, &(u, v))| {
            let q = overrides
                .iter()
                .find(|(e, _)| *e == eid)
                .map(|(_, q)| *q)
                .unwrap_or(ident);
            su2_record(eid, u, v, q)
        })
        .collect();
    eng.batch_insert(bundle, &recs).expect("batch_insert");
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

/// Execute an INIT FROM BUNDLE (or any) statement, expecting success.
fn run_ok(eng: &mut Engine, gql: &str) {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse '{gql}': {e}"));
    execute(eng, &stmt).unwrap_or_else(|e| panic!("exec '{gql}': {e}"));
}

/// Execute a statement expecting a typed String error (never a panic).
fn run_err(eng: &mut Engine, gql: &str) -> String {
    let stmt = match parse(gql) {
        Ok(s) => s,
        Err(e) => return e, // parse-level typed error is also acceptable
    };
    execute(eng, &stmt).expect_err("expected a typed error, got Ok")
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

fn unpack(g: GroupElement) -> [f64; 4] {
    match g {
        GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
        other => panic!("expected SU2, got {other:?}"),
    }
}

// ── I1 — round-trip (load-bearing) ───────────────────────────────────

/// A bundle carrying chosen unit-norm SU(2) elements per edge → INIT FROM
/// BUNDLE → each edge reads back via `handle.edge_element(eid, Forward)`
/// EXACTLY the stored quaternion (1e-12). Records emitted in canonical
/// (va → vb) orientation, so Forward reads == stored.
#[test]
fn i1_round_trip_forward_exact() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "rt_lat", 3, 3); // 81 edges

    // A handful of distinct unit-norm quaternions (rotations about x/y/z
    // and the half-half icosian) on specific edges; identity elsewhere.
    let c6 = (std::f64::consts::PI / 6.0).cos();
    let s6 = (std::f64::consts::PI / 6.0).sin();
    let c4 = (std::f64::consts::PI / 4.0).cos();
    let s4 = (std::f64::consts::PI / 4.0).sin();
    let c3 = (std::f64::consts::PI / 3.0).cos();
    let s3 = (std::f64::consts::PI / 3.0).sin();
    let overrides: &[(usize, [f64; 4])] = &[
        (0, [c6, s6, 0.0, 0.0]),
        (5, [c4, 0.0, s4, 0.0]),
        (10, [c3, 0.0, 0.0, s3]),
        (12, [0.5, 0.5, 0.5, 0.5]),
    ];
    build_su2_bundle(&mut eng, "rt_bundle", &lat, overrides);

    run_ok(
        &mut eng,
        "GAUGE_FIELD rt_field GROUP SU(2) INIT FROM BUNDLE rt_bundle ON LATTICE rt_lat;",
    );

    let handle = gauge_registry::get("rt_field").expect("field registered after INIT FROM BUNDLE");
    assert_eq!(handle.lattice_name(), "rt_lat");
    assert_eq!(handle.group(), gigi::gauge::Group::SU2);

    let ident = [1.0, 0.0, 0.0, 0.0];
    for eid in 0..lat.n_edges() {
        let expected = overrides
            .iter()
            .find(|(e, _)| *e == eid)
            .map(|(_, q)| *q)
            .unwrap_or(ident);
        let got = unpack(handle.edge_element(eid, EdgeOrientation::Forward));
        for j in 0..4 {
            assert!(
                (got[j] - expected[j]).abs() < 1e-12,
                "edge {eid} comp {j}: got {}, want {}",
                got[j],
                expected[j]
            );
        }
    }
}

/// A reverse-oriented record (vertex_a, vertex_b swapped relative to the
/// lattice's canonical `edges[eid]`) still round-trips as Ω on the DECLARED
/// va → vb direction — NOT Ω†. The buffer's canonical slot holds Ω.inverse()
/// so `edge_element(eid, resolve_orient(va,vb))` returns the intended Ω.
/// This is the orientation pin: a mis-registered seam would return Ω† and
/// silently flip every π₁ class.
#[test]
fn i1b_reverse_orientation_returns_omega_not_dagger() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "or_lat", 3, 3);

    // Pick a real lattice edge and its canonical direction.
    let eid = 7usize;
    let (u, v) = lat.edges[eid];
    // A chosen z-rotation with a non-trivial vector part (so Ω ≠ Ω†).
    let ang = 2.0 * std::f64::consts::PI / 5.0;
    let om = [(ang / 2.0).cos(), 0.0, 0.0, (ang / 2.0).sin()];

    // Emit the record REVERSED: declare it on v → u carrying Ω.
    eng.create_bundle(su2_bundle_schema("or_bundle"))
        .expect("create_bundle");
    let mut recs: Vec<Record> = lat
        .edges
        .iter()
        .enumerate()
        .filter(|(e, _)| *e != eid)
        .map(|(e, &(a, b))| su2_record(e, a, b, [1.0, 0.0, 0.0, 0.0]))
        .collect();
    // The reversed record: va=v, vb=u (opposite of edges[eid]=(u,v)).
    recs.push(su2_record(eid, v, u, om));
    eng.batch_insert("or_bundle", &recs).expect("batch_insert");

    run_ok(
        &mut eng,
        "GAUGE_FIELD or_field GROUP SU(2) INIT FROM BUNDLE or_bundle ON LATTICE or_lat;",
    );
    let handle = gauge_registry::get("or_field").expect("registered");

    // Reading in the DECLARED direction (v → u = Reverse of canonical) must
    // return Ω exactly.
    let (reid, rorient) = lat.resolve_edge(v, u).expect("edge exists");
    assert_eq!(reid, eid);
    assert_eq!(rorient, EdgeOrientation::Reverse);
    let read_declared = unpack(handle.edge_element(reid, rorient));
    for j in 0..4 {
        assert!(
            (read_declared[j] - om[j]).abs() < 1e-12,
            "declared-direction read comp {j}: got {}, want {}",
            read_declared[j],
            om[j]
        );
    }

    // And the canonical Forward slot holds Ω† (conjugate): q0 same, vector
    // negated.
    let canon = unpack(handle.edge_element(eid, EdgeOrientation::Forward));
    assert!((canon[0] - om[0]).abs() < 1e-12);
    assert!((canon[3] + om[3]).abs() < 1e-12, "vector part must be negated (Ω†)");
}

// ── I2 — lens p-sweep (THE receipt, now live) ────────────────────────

/// The headline: a twisted-BC SU(2) bundle (z-wrap link at (x0,y0) carries
/// Ω = (cos 2πq/p, 0, 0, sin 2πq/p), all other links identity) → INIT FROM
/// BUNDLE → HOLONOMY AROUND CYCLE AXIS z AT (x0,y0) → re_trace = cos(2πq/p)
/// to 1e-12, order_estimate = p. Reproduces the H2 unit anchor through the
/// FULL live path (bundle → registry → HOLONOMY). Closes the gate-lock.
#[test]
fn i2_lens_p_sweep_order_tracks_p_live() {
    let (mut eng, _dir, _g) = open_engine();
    let l = 5usize;
    let (x0, y0) = (0usize, 0usize);
    let lat = declare_cubic(&mut eng, "lens5", l, 3);
    // The +z wrap edge at (x0,y0): site(z=L-1) → site(z=0).
    let wrap = lat
        .resolve_edge(site(x0, y0, l - 1, l), site(x0, y0, 0, l))
        .expect("periodic z-wrap edge exists")
        .0;

    let cases: &[(usize, usize)] = &[(2, 1), (3, 1), (5, 1), (5, 2), (7, 1), (7, 3)];
    for &(p, q) in cases {
        // Fresh bundle + field per case.
        let bname = format!("lens5_p{p}_q{q}_bundle");
        build_su2_bundle(&mut eng, &bname, &lat, &[(wrap, omega(p, q))]);
        gauge_registry::remove("lens5_field");
        run_ok(
            &mut eng,
            &format!(
                "GAUGE_FIELD lens5_field GROUP SU(2) INIT FROM BUNDLE {bname} ON LATTICE lens5;"
            ),
        );
        let row = run_cycle(&mut eng, "HOLONOMY lens5_field AROUND CYCLE AXIS z AT (0, 0);");
        let ang = 2.0 * std::f64::consts::PI * q as f64 / p as f64;
        assert!(
            (f(&row, "re_trace") - ang.cos()).abs() < 1e-12,
            "(p={p},q={q}) re_trace = cos(2πq/p): got {}, want {}",
            f(&row, "re_trace"),
            ang.cos()
        );
        assert_eq!(
            int(&row, "order_estimate"),
            p as i64,
            "(p={p},q={q}) order_estimate must equal p"
        );
    }
}

// ── I3 — p=1 control ─────────────────────────────────────────────────

/// Identity field (no twist) → re_trace 1.0 exact, order 1. The S³ control.
#[test]
fn i3_p1_identity_control() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "ctrl_lat", 5, 3);
    build_su2_bundle(&mut eng, "ctrl_bundle", &lat, &[]); // identity everywhere
    run_ok(
        &mut eng,
        "GAUGE_FIELD ctrl_field GROUP SU(2) INIT FROM BUNDLE ctrl_bundle ON LATTICE ctrl_lat;",
    );
    let row = run_cycle(&mut eng, "HOLONOMY ctrl_field AROUND CYCLE AXIS z AT (0, 0);");
    assert_eq!(f(&row, "re_trace"), 1.0, "identity → re_trace 1.0 exact");
    assert_eq!(int(&row, "order_estimate"), 1, "identity → order 1");
}

// ── I4 — U(1) now live (the NS linking fast-follow landed) ───────────

/// GROUP U(1) INIT FROM BUNDLE now round-trips (2026-07-18 linking ship):
/// live U(1) group math + a U(1) DenseLinkBuffer arm landed, so a chosen
/// theta field injects and reads back its θ exactly. Full U(1) coverage
/// (round-trip / holonomy phase / the κ·Lk linking receipt / typed errors)
/// lives in `tests/u1_linking_basic.rs`; this fence just confirms the SU(2)
/// inject path is unregressed while U(1) went live alongside it.
#[test]
fn i4_u1_from_bundle_now_live() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "u1_lat", 3, 3);
    // A theta bundle (the U(1) shape), θ = 0.3 everywhere except edge 0.
    eng.create_bundle(
        BundleSchema::new("u1_bundle")
            .base(FieldDef::numeric("config_id"))
            .base(FieldDef::numeric("edge_id"))
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("theta")),
    )
    .expect("create_bundle");
    let recs: Vec<Record> = lat
        .edges
        .iter()
        .enumerate()
        .map(|(eid, &(u, v))| {
            let mut r = Record::new();
            r.insert("config_id".to_string(), Value::Integer(0));
            r.insert("edge_id".to_string(), Value::Integer(eid as i64));
            r.insert("vertex_a".to_string(), Value::Integer(u as i64));
            r.insert("vertex_b".to_string(), Value::Integer(v as i64));
            r.insert("theta".to_string(), Value::Float(if eid == 0 { 1.25 } else { 0.3 }));
            r
        })
        .collect();
    eng.batch_insert("u1_bundle", &recs).expect("batch_insert");

    run_ok(
        &mut eng,
        "GAUGE_FIELD u1_field GROUP U(1) INIT FROM BUNDLE u1_bundle ON LATTICE u1_lat;",
    );

    // The field registers as GROUP U(1) and reads back the chosen θ.
    let handle = gauge_registry::get("u1_field").expect("U(1) field registered");
    assert_eq!(handle.group().label(), "U(1)");
    let g0 = handle.edge_element(0, EdgeOrientation::Forward);
    match g0 {
        GroupElement::U1 { theta } => assert!((theta - 1.25).abs() < 1e-12, "edge 0 θ round-trip"),
        other => panic!("expected U1, got {other:?}"),
    }
}

// ── I5 — typed errors (no panics) ────────────────────────────────────

/// INIT FROM BUNDLE referencing a bundle the engine does not know → typed
/// bundle-not-found error.
#[test]
fn i5_bundle_not_found() {
    let (mut eng, _dir, _g) = open_engine();
    let _lat = declare_cubic(&mut eng, "nf_lat", 3, 3);
    let err = run_err(
        &mut eng,
        "GAUGE_FIELD nf_field GROUP SU(2) INIT FROM BUNDLE ghost_bundle ON LATTICE nf_lat;",
    );
    assert!(
        err.to_lowercase().contains("bundle") && err.contains("ghost_bundle"),
        "bundle-not-found must name the missing bundle: {err}"
    );
}

/// A bundle whose fiber columns don't match SU(2)'s repr_dim (here a theta
/// bundle pointed at GROUP SU(2)) → typed arity/column error.
#[test]
fn i5_fiber_arity_mismatch() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "ar_lat", 3, 3);
    eng.create_bundle(
        BundleSchema::new("theta_bundle")
            .base(FieldDef::numeric("config_id"))
            .base(FieldDef::numeric("edge_id"))
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("theta")),
    )
    .expect("create_bundle");
    let recs: Vec<Record> = lat
        .edges
        .iter()
        .enumerate()
        .take(3)
        .map(|(eid, &(u, v))| {
            let mut r = Record::new();
            r.insert("edge_id".to_string(), Value::Integer(eid as i64));
            r.insert("vertex_a".to_string(), Value::Integer(u as i64));
            r.insert("vertex_b".to_string(), Value::Integer(v as i64));
            r.insert("theta".to_string(), Value::Float(0.1));
            r
        })
        .collect();
    eng.batch_insert("theta_bundle", &recs).expect("batch_insert");

    let err = run_err(
        &mut eng,
        "GAUGE_FIELD ar_field GROUP SU(2) INIT FROM BUNDLE theta_bundle ON LATTICE ar_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("arity") || low.contains("fiber") || low.contains("q0") || low.contains("column"),
        "arity mismatch must name the fiber/arity problem: {err}"
    );
}

/// A record whose (vertex_a, vertex_b) is not a lattice edge → typed
/// non-lattice-edge error.
#[test]
fn i5_non_lattice_edge() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "nl_lat", 3, 3);
    build_su2_bundle(&mut eng, "nl_bundle", &lat, &[]);
    // Append one bogus record: vertices 0 and 999 are not an edge.
    let mut bogus = su2_record(0, 0, 999, [1.0, 0.0, 0.0, 0.0]);
    bogus.insert("edge_id".to_string(), Value::Integer(9999));
    eng.batch_insert("nl_bundle", std::slice::from_ref(&bogus))
        .expect("batch_insert bogus");

    let err = run_err(
        &mut eng,
        "GAUGE_FIELD nl_field GROUP SU(2) INIT FROM BUNDLE nl_bundle ON LATTICE nl_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("edge") && (low.contains("999") || low.contains("lattice")),
        "non-lattice edge must be flagged: {err}"
    );
}

/// A non-normalized SU(2) quaternion (|q| far from 1) → typed rejection,
/// NOT a silent renormalize (inverse==conjugate only holds for |q|=1).
#[test]
fn i5_non_normalized_quaternion() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "nn_lat", 3, 3);
    // Edge 3 carries a |q|=2 quaternion — far outside the 1e-6 tolerance.
    build_su2_bundle(&mut eng, "nn_bundle", &lat, &[(3, [2.0, 0.0, 0.0, 0.0])]);
    let err = run_err(
        &mut eng,
        "GAUGE_FIELD nn_field GROUP SU(2) INIT FROM BUNDLE nn_bundle ON LATTICE nn_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("norm") || low.contains("unit") || low.contains("normaliz"),
        "non-normalized quaternion must be flagged: {err}"
    );
}

/// An SU(2)-schema bundle with zero records → typed empty-bundle error
/// (an injection needs at least one edge to write).
#[test]
fn i5_empty_bundle() {
    let (mut eng, _dir, _g) = open_engine();
    let _lat = declare_cubic(&mut eng, "mt_lat", 3, 3);
    eng.create_bundle(su2_bundle_schema("mt_bundle"))
        .expect("create_bundle");
    // No inserts — bundle exists but is empty.
    let err = run_err(
        &mut eng,
        "GAUGE_FIELD mt_field GROUP SU(2) INIT FROM BUNDLE mt_bundle ON LATTICE mt_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("empty") || low.contains("no ") || low.contains("record"),
        "empty bundle must be flagged: {err}"
    );
}
