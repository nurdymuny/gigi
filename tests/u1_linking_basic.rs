//! U(1) linking-number reading — INIT FROM BUNDLE U(1) + HOLONOMY U(1).
//!
//! Hallie's one open ask: the Navier–Stokes vortex linking-number reading
//! `∮_C A·dl = κ·Lk(C1, C2)` via HOLONOMY on a CHOSEN U(1) vortex field.
//! The chosen-field → registry seam (INIT FROM BUNDLE) shipped SU(2)-only;
//! this lights up the whole U(1) path (live U(1) group math + a U(1)
//! DenseLinkBuffer arm + INIT FROM BUNDLE U(1) + HOLONOMY U(1)).
//!
//! Anchors:
//!   U1-RT   ROUND-TRIP    a chosen theta field → INIT FROM BUNDLE → read
//!                         back each edge's θ EXACTLY (1e-12). Forward
//!                         stores θ; a reverse-declared record gives −θ
//!                         canonically so the declared read returns θ.
//!   U1-H    HOLONOMY φ     a z-cycle whose edge phases sum to Θ →
//!                         phase = Θ (raw, unwrapped), re_trace = cos Θ,
//!                         q = (cosΘ, 0, 0, sinΘ), group_used "U(1)".
//!                         Identity field → phase 0, re_trace 1.
//!   U1-LINK THE RECEIPT   a chosen U(1) field encoding a vortex of
//!                         circulation κ threading a linking column n
//!                         times → phase = n·κ (n=0→0, 1→κ, 2→2κ). A
//!                         control loop that does NOT link → phase 0.
//!   U1-ERR  TYPED ERRORS  fiber arity != 1 (a q0..q3 bundle → GROUP U(1)),
//!                         bundle-not-found, non-lattice edge, empty
//!                         bundle — all typed, no panics.
//!
//! Run: `cargo test --features halcyon --test u1_linking_basic`
//!
//! RED at the base commit: the GAUGE_FIELD executor errors `UnsupportedGroup`
//! on GROUP U(1) and HOLONOMY gates to SU(2), so every U(1) declaration and
//! read fails. GREEN lights the U(1) executor arm (`inject::u1_buffer_from_
//! bundle` → `U1GaugeField` → register) and the U(1) HOLONOMY arm.

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::edge_connection::EdgeConnection;
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::registry as gauge_registry;
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::{EdgeOrientation, Lattice};
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── Fixtures ─────────────────────────────────────────────────────────

/// Open a fresh engine with both singleton registries cleared. The serial
/// guard must live for the whole test (process-global registries).
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

/// Declare a periodic cubic lattice via the parser and return it.
fn declare_cubic(eng: &mut Engine, name: &str, l: usize, d: usize) -> Lattice {
    let gql = format!("LATTICE {name} FROM CUBIC L={l} DIM={d} PERIODIC;");
    let stmt = parse(&gql).expect("parse LATTICE");
    execute(eng, &stmt).expect("exec LATTICE");
    lattice_registry::get(name).expect("lattice registered")
}

/// The canonical U(1) edge-endpoint bundle schema (matches INIT FLUX /
/// INGEST AS GAUGE_FIELD): base `config_id, edge_id, vertex_a, vertex_b`,
/// fiber `theta`.
fn u1_bundle_schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
        .base(FieldDef::numeric("config_id"))
        .base(FieldDef::numeric("edge_id"))
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("theta"))
}

/// One U(1) edge record on the directed edge `va → vb`.
fn u1_record(edge_id: usize, va: usize, vb: usize, theta: f64) -> Record {
    let mut rec = Record::new();
    rec.insert("config_id".to_string(), Value::Integer(0));
    rec.insert("edge_id".to_string(), Value::Integer(edge_id as i64));
    rec.insert("vertex_a".to_string(), Value::Integer(va as i64));
    rec.insert("vertex_b".to_string(), Value::Integer(vb as i64));
    rec.insert("theta".to_string(), Value::Float(theta));
    rec
}

/// Materialize a FULL U(1) theta bundle for `lat`: one record per lattice
/// edge in the lattice's own `edges[k] = (u, v)` order (canonical Forward
/// orientation — exactly what INIT FLUX / a lens emitter produce), θ = 0
/// everywhere except the `(edge_id → θ)` overrides.
fn build_u1_bundle(eng: &mut Engine, bundle: &str, lat: &Lattice, overrides: &[(usize, f64)]) {
    eng.create_bundle(u1_bundle_schema(bundle)).expect("create_bundle");
    let recs: Vec<Record> = lat
        .edges
        .iter()
        .enumerate()
        .map(|(eid, &(u, v))| {
            let theta = overrides
                .iter()
                .find(|(e, _)| *e == eid)
                .map(|(_, t)| *t)
                .unwrap_or(0.0);
            u1_record(eid, u, v, theta)
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

/// Execute a statement expecting success.
fn run_ok(eng: &mut Engine, gql: &str) {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse '{gql}': {e}"));
    execute(eng, &stmt).unwrap_or_else(|e| panic!("exec '{gql}': {e}"));
}

/// Execute a statement expecting a typed String error (never a panic).
fn run_err(eng: &mut Engine, gql: &str) -> String {
    let stmt = match parse(gql) {
        Ok(s) => s,
        Err(e) => return e,
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
fn text(row: &Record, k: &str) -> String {
    match row.get(k) {
        Some(Value::Text(v)) => v.clone(),
        other => panic!("field '{k}' not Text: {other:?}"),
    }
}

fn theta_of(g: GroupElement) -> f64 {
    match g {
        GroupElement::U1 { theta } => theta,
        other => panic!("expected U1, got {other:?}"),
    }
}

// ── U1-RT — round-trip (load-bearing) ────────────────────────────────

/// A U(1) theta bundle with chosen per-edge phases → INIT FROM BUNDLE →
/// each edge reads back via `handle.edge_element(eid, Forward)` EXACTLY the
/// stored θ (1e-12). Records emitted in canonical (va → vb) order, so
/// Forward reads == stored.
#[test]
fn u1_rt_forward_exact() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "rt_u1_lat", 3, 3); // 81 edges
    let overrides: &[(usize, f64)] =
        &[(0, 0.37), (5, -1.1), (17, 2.6), (40, 0.0), (63, 3.0)];
    build_u1_bundle(&mut eng, "rt_u1_bundle", &lat, overrides);
    run_ok(
        &mut eng,
        "GAUGE_FIELD rt_u1_field GROUP U(1) INIT FROM BUNDLE rt_u1_bundle ON LATTICE rt_u1_lat;",
    );

    let handle = gauge_registry::get("rt_u1_field").expect("field registered");
    // Every override edge reads back its exact θ (Forward).
    for &(eid, theta) in overrides {
        let got = theta_of(handle.edge_element(eid, EdgeOrientation::Forward));
        assert!((got - theta).abs() < 1e-12, "edge {eid} θ round-trip: got {got}, want {theta}");
    }
    // A non-override edge is identity (θ = 0).
    assert!(theta_of(handle.edge_element(1, EdgeOrientation::Forward)).abs() < 1e-12);
}

/// A record declared in the REVERSED (v → u) direction stores −θ in the
/// canonical slot (the U(1) inverse), so reading in the DECLARED direction
/// (Reverse) returns +θ — the intended circulation sign. Forward on that
/// slot returns −θ.
#[test]
fn u1_rt_reverse_returns_theta_not_minus() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "rev_u1_lat", 3, 3);
    let (u, v) = lat.edges[0];
    let theta = 0.9_f64;
    // Full bundle, but override edge 0's record to be declared REVERSED
    // (va = v, vb = u) carrying +θ on the v→u direction.
    eng.create_bundle(u1_bundle_schema("rev_u1_bundle")).expect("create_bundle");
    let recs: Vec<Record> = lat
        .edges
        .iter()
        .enumerate()
        .map(|(eid, &(a, b))| {
            if eid == 0 {
                u1_record(0, v, u, theta) // reversed declaration
            } else {
                u1_record(eid, a, b, 0.0)
            }
        })
        .collect();
    eng.batch_insert("rev_u1_bundle", &recs).expect("batch_insert");
    run_ok(
        &mut eng,
        "GAUGE_FIELD rev_u1_field GROUP U(1) INIT FROM BUNDLE rev_u1_bundle ON LATTICE rev_u1_lat;",
    );

    let handle = gauge_registry::get("rev_u1_field").expect("field registered");
    let (eid, orient) = lat.resolve_edge(v, u).expect("edge");
    assert_eq!(orient, EdgeOrientation::Reverse);
    // Declared (Reverse) read returns +θ.
    let declared = theta_of(handle.edge_element(eid, orient));
    assert!((declared - theta).abs() < 1e-12, "declared read = +θ: got {declared}");
    // Canonical Forward slot is −θ.
    let canon = theta_of(handle.edge_element(eid, EdgeOrientation::Forward));
    assert!((canon + theta).abs() < 1e-12, "canonical Forward = −θ: got {canon}");
}

// ── U1-H — holonomy phase ────────────────────────────────────────────

/// A z-cycle whose 5 z-edge phases sum to Θ → HOLONOMY returns phase = Θ
/// (raw, unwrapped), re_trace = cos Θ, q = (cosΘ, 0, 0, sinΘ), group_used
/// "U(1)". The load-bearing NS field is `phase` = ∮_C A·dl.
#[test]
fn u1_h_phase_is_edge_sum() {
    let (mut eng, _dir, _g) = open_engine();
    let l = 5usize;
    let (x0, y0) = (1usize, 2usize);
    let v = l * l * l; // 125
    let lat = declare_cubic(&mut eng, "h_u1_lat", l, 3);
    // z-axis block starts at 2·V; z-edge at (x0,y0,z) = 2·V + site(x0,y0,z).
    let zphases = [0.1_f64, 0.2, 0.3, 0.4, 0.5]; // sum = 1.5
    let big_theta: f64 = zphases.iter().sum();
    let overrides: Vec<(usize, f64)> = (0..l)
        .map(|z| (2 * v + site(x0, y0, z, l), zphases[z]))
        .collect();
    build_u1_bundle(&mut eng, "h_u1_bundle", &lat, &overrides);
    run_ok(
        &mut eng,
        "GAUGE_FIELD h_u1_field GROUP U(1) INIT FROM BUNDLE h_u1_bundle ON LATTICE h_u1_lat;",
    );

    let row = run_cycle(&mut eng, "HOLONOMY h_u1_field AROUND CYCLE AXIS z AT (1, 2);");
    assert!((f(&row, "phase") - big_theta).abs() < 1e-12, "phase = Σθ = {big_theta}");
    assert!((f(&row, "re_trace") - big_theta.cos()).abs() < 1e-12, "re_trace = cos Θ");
    assert!((f(&row, "q0") - big_theta.cos()).abs() < 1e-12, "q0 = cos Θ");
    assert!(f(&row, "q1").abs() < 1e-12, "q1 = 0");
    assert!(f(&row, "q2").abs() < 1e-12, "q2 = 0");
    assert!((f(&row, "q3") - big_theta.sin()).abs() < 1e-12, "q3 = sin Θ");
    assert_eq!(text(&row, "group_used"), "U(1)");
    // Θ = 1.5 is not a rational multiple of 2π → order sentinel 0.
    assert_eq!(int(&row, "order_estimate"), 0, "continuous U(1) → order sentinel 0");
}

/// Identity U(1) field → phase 0, re_trace 1, q = (1,0,0,0), order 1.
#[test]
fn u1_h_identity_is_trivial() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "hid_u1_lat", 5, 3);
    build_u1_bundle(&mut eng, "hid_u1_bundle", &lat, &[]); // all θ = 0
    run_ok(
        &mut eng,
        "GAUGE_FIELD hid_u1_field GROUP U(1) INIT FROM BUNDLE hid_u1_bundle ON LATTICE hid_u1_lat;",
    );
    let row = run_cycle(&mut eng, "HOLONOMY hid_u1_field AROUND CYCLE AXIS z AT (0, 0);");
    assert_eq!(f(&row, "phase"), 0.0, "identity → phase 0 exact");
    assert_eq!(f(&row, "re_trace"), 1.0, "identity → re_trace 1 exact");
    assert_eq!(f(&row, "q0"), 1.0);
    assert_eq!(f(&row, "q3"), 0.0);
    assert_eq!(int(&row, "order_estimate"), 1, "identity → order 1");
    assert_eq!(text(&row, "group_used"), "U(1)");
}

/// AXIS vs EDGES parity: the AXIS z form at (x0,y0) produces the same
/// ordered z-edge set (hence same phase) as the explicit EDGES list.
#[test]
fn u1_h_axis_equals_edges() {
    let (mut eng, _dir, _g) = open_engine();
    let l = 5usize;
    let (x0, y0) = (1usize, 2usize);
    let v = l * l * l;
    let lat = declare_cubic(&mut eng, "he_u1_lat", l, 3);
    let overrides: Vec<(usize, f64)> =
        (0..l).map(|z| (2 * v + site(x0, y0, z, l), 0.1 + z as f64 * 0.05)).collect();
    build_u1_bundle(&mut eng, "he_u1_bundle", &lat, &overrides);
    run_ok(
        &mut eng,
        "GAUGE_FIELD he_u1_field GROUP U(1) INIT FROM BUNDLE he_u1_bundle ON LATTICE he_u1_lat;",
    );
    let axis = run_cycle(&mut eng, "HOLONOMY he_u1_field AROUND CYCLE AXIS z AT (1, 2);");
    let ids: Vec<String> =
        (0..l).map(|z| (2 * v + site(x0, y0, z, l)).to_string()).collect();
    let edges_gql = format!("HOLONOMY he_u1_field AROUND CYCLE EDGES ({});", ids.join(", "));
    let edges = run_cycle(&mut eng, &edges_gql);
    for k in ["phase", "re_trace", "q0", "q3"] {
        assert!((f(&axis, k) - f(&edges, k)).abs() < 1e-15, "AXIS vs EDGES on {k}");
    }
}

// ── U1-LINK — the NS vortex-linking receipt ──────────────────────────

/// THE RECEIPT: a chosen U(1) field encoding a vortex of circulation κ
/// threading a linking column n times → HOLONOMY phase = n·κ exactly
/// (n=0→0, 1→κ, 2→2κ), re_trace = cos(n·κ). A control loop that does not
/// link the vortex reads phase 0. This is `∮_C A·dl = κ·Lk(C1, C2)` — the
/// vortex linking number read as U(1) holonomy. Lk = phase/κ, client-side.
#[test]
fn u1_link_phase_is_kappa_times_lk() {
    let (mut eng, _dir, _g) = open_engine();
    let l = 5usize;
    let v = l * l * l;
    let kappa = 0.37_f64; // continuous circulation quantum
    // Linking column at (2, 2); control column at (0, 0).
    let (lx, ly) = (2usize, 2usize);
    let (cx, cy) = (0usize, 0usize);
    let z_edge = |z: usize, x: usize, y: usize| 2 * v + site(x, y, z, l);
    let lat = declare_cubic(&mut eng, "link_u1_lat", l, 3);

    for n in [0usize, 1, 2] {
        // n flux quanta of κ thread the linking column (z-edges 0..n).
        let overrides: Vec<(usize, f64)> =
            (0..n).map(|z| (z_edge(z, lx, ly), kappa)).collect();
        let bname = format!("link_u1_bundle_n{n}");
        build_u1_bundle(&mut eng, &bname, &lat, &overrides);
        gauge_registry::remove("link_u1_field");
        run_ok(
            &mut eng,
            &format!(
                "GAUGE_FIELD link_u1_field GROUP U(1) INIT FROM BUNDLE {bname} ON LATTICE link_u1_lat;"
            ),
        );

        // The linking loop: z-cycle at (lx, ly) → phase = n·κ.
        let row = run_cycle(
            &mut eng,
            &format!("HOLONOMY link_u1_field AROUND CYCLE AXIS z AT ({lx}, {ly});"),
        );
        let want = n as f64 * kappa;
        assert!(
            (f(&row, "phase") - want).abs() < 1e-12,
            "n={n}: phase must be n·κ = {want}, got {}",
            f(&row, "phase")
        );
        assert!(
            (f(&row, "re_trace") - want.cos()).abs() < 1e-12,
            "n={n}: re_trace = cos(n·κ)"
        );
        assert_eq!(text(&row, "group_used"), "U(1)");

        // The control loop at (cx, cy) does NOT link the vortex → phase 0.
        let ctrl = run_cycle(
            &mut eng,
            &format!("HOLONOMY link_u1_field AROUND CYCLE AXIS z AT ({cx}, {cy});"),
        );
        assert_eq!(f(&ctrl, "phase"), 0.0, "n={n}: control column links 0 → phase 0");
        assert_eq!(f(&ctrl, "re_trace"), 1.0, "n={n}: control re_trace 1");
    }
}

// ── U1-ERR — typed errors (no panics) ────────────────────────────────

/// A q0..q3 (SU(2)-shaped) bundle pointed at GROUP U(1) → typed fiber
/// arity mismatch (expected 1 `theta`, got 0). Symmetric to the SU(2)
/// arm rejecting a theta bundle.
#[test]
fn u1_err_fiber_arity_mismatch() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "are_u1_lat", 3, 3);
    eng.create_bundle(
        BundleSchema::new("q_bundle")
            .base(FieldDef::numeric("config_id"))
            .base(FieldDef::numeric("edge_id"))
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("q0"))
            .fiber(FieldDef::numeric("q1"))
            .fiber(FieldDef::numeric("q2"))
            .fiber(FieldDef::numeric("q3")),
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
            r.insert("q0".to_string(), Value::Float(1.0));
            r.insert("q1".to_string(), Value::Float(0.0));
            r.insert("q2".to_string(), Value::Float(0.0));
            r.insert("q3".to_string(), Value::Float(0.0));
            r
        })
        .collect();
    eng.batch_insert("q_bundle", &recs).expect("batch_insert");

    let err = run_err(
        &mut eng,
        "GAUGE_FIELD are_u1_field GROUP U(1) INIT FROM BUNDLE q_bundle ON LATTICE are_u1_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("arity") || low.contains("fiber") || low.contains("theta") || low.contains("column"),
        "U(1) arity mismatch must name the fiber/theta problem: {err}"
    );
}

/// GROUP U(1) INIT FROM BUNDLE referencing an unknown bundle → typed
/// bundle-not-found (no panic).
#[test]
fn u1_err_bundle_not_found() {
    let (mut eng, _dir, _g) = open_engine();
    let _lat = declare_cubic(&mut eng, "nfu_lat", 3, 3);
    let err = run_err(
        &mut eng,
        "GAUGE_FIELD nfu_field GROUP U(1) INIT FROM BUNDLE ghost_u1 ON LATTICE nfu_lat;",
    );
    assert!(
        err.to_lowercase().contains("bundle") && err.contains("ghost_u1"),
        "U(1) bundle-not-found must name the missing bundle: {err}"
    );
}

/// A record whose (vertex_a, vertex_b) is not a lattice edge → typed
/// non-lattice-edge error (no panic).
#[test]
fn u1_err_non_lattice_edge() {
    let (mut eng, _dir, _g) = open_engine();
    let lat = declare_cubic(&mut eng, "nlu_lat", 3, 3);
    build_u1_bundle(&mut eng, "nlu_bundle", &lat, &[]);
    let bogus = u1_record(9999, 0, 999, 0.5);
    eng.batch_insert("nlu_bundle", std::slice::from_ref(&bogus))
        .expect("batch_insert bogus");
    let err = run_err(
        &mut eng,
        "GAUGE_FIELD nlu_field GROUP U(1) INIT FROM BUNDLE nlu_bundle ON LATTICE nlu_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("edge") && (low.contains("999") || low.contains("lattice")),
        "U(1) non-lattice edge must be flagged: {err}"
    );
}

/// A U(1)-schema bundle with zero records → typed empty-bundle error.
#[test]
fn u1_err_empty_bundle() {
    let (mut eng, _dir, _g) = open_engine();
    let _lat = declare_cubic(&mut eng, "mtu_lat", 3, 3);
    eng.create_bundle(u1_bundle_schema("mtu_bundle")).expect("create_bundle");
    let err = run_err(
        &mut eng,
        "GAUGE_FIELD mtu_field GROUP U(1) INIT FROM BUNDLE mtu_bundle ON LATTICE mtu_lat;",
    );
    let low = err.to_lowercase();
    assert!(
        low.contains("empty") || low.contains("no ") || low.contains("record"),
        "U(1) empty bundle must be flagged: {err}"
    );
}
