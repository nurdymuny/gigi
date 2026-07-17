//! SPECTRAL ... MODE MATRIX — raw signed-symmetric spectrum (P-vs-NP).
//!
//! RED phase (2026-07-17): these tests pin the MODE MATRIX contract
//! Hallie needs for Bee's P-vs-NP signature — the fraction of NEGATIVE
//! eigenvalues of the SAT Hessian (solution-manifold curvature
//! instability). They fail on the pre-ship tree, where the plain
//! `SPECTRAL <b> ON FIBER (h) MODE MATRIX` grammar does not parse (the
//! ON FIBER branch expects `MODES k`, not `MODE MATRIX`).
//!
//! Contract under test (locked context 2026-07-17):
//!
//! - `SPECTRAL <bundle> ON FIBER (<h>) MODE MATRIX [DIAGONAL <d>] [FULL
//!   [LIMIT k]]` assembles the RAW signed symmetric matrix M from
//!   edge-endpoint records: off-diagonal record (vertex_a=i, vertex_b=j,
//!   i≠j) sets M[i][j]=M[j][i]=h; self-loop record (vertex_a==vertex_b=v)
//!   sets the diagonal M[v][v]=h (Option S). DIAGONAL <d> names an
//!   override column the self-loop reads instead of <h>. Missing
//!   diagonal → M[v][v]=0.
//! - This is NOT the Laplacian (L=D−W is PSD and loses the negatives).
//!   The raw matrix keeps its negative eigenvalues — the whole signal.
//! - Return: ONE row { eigenvalues (ascending), n_records_used,
//!   mode_used ("matrix"), n_negative = #{λ < −1e-9}, instability_fraction
//!   = n_negative / V }. n_negative/instability computed over the FULL
//!   spectrum, never windowed by LIMIT.
//! - MODE MATRIX requires NO GROUP; a stray GROUP clause is ignored.
//! - Dense V ≤ 4096; V > 4096 → typed SparseUnavailable-shaped error
//!   naming Phase 2.1. LIMIT 0 → typed error; k > V clamps; empty edge
//!   set → typed error.
//!
//! Run with:
//!   `cargo test --features halcyon --test spectral_matrix_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::Group;
use gigi::parser::{execute, parse, ExecResult, Statement};
use gigi::spectral::spectral_gauge_gap;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── Fixture helpers ─────────────────────────────────────────────────

/// Heap bundle with the Halcyon edge-endpoint schema + a single signed
/// scalar fiber `h_ij`.
fn make_hessian_bundle(engine: &mut Engine, name: &str) {
    let schema = BundleSchema::new(name)
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("h_ij"));
    engine
        .create_bundle(schema)
        .expect("create_bundle should succeed");
}

/// One edge-endpoint record. `va == vb` is a self-loop (diagonal).
fn edge(va: i64, vb: i64, h: f64) -> Record {
    let mut r = Record::new();
    r.insert("vertex_a".to_string(), Value::Integer(va));
    r.insert("vertex_b".to_string(), Value::Integer(vb));
    r.insert("h_ij".to_string(), Value::Float(h));
    r
}

/// Parse + execute a MODE MATRIX query, expecting the one-row envelope.
fn run_matrix(engine: &mut Engine, q: &str) -> Vec<Record> {
    let stmt = parse(q).unwrap_or_else(|e| panic!("MODE MATRIX must parse: {q}\n  err: {e}"));
    match execute(engine, &stmt).unwrap_or_else(|e| panic!("MODE MATRIX must execute: {q}\n  err: {e}")) {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows envelope, got {other:?}"),
    }
}

fn eigs(row: &Record) -> Vec<f64> {
    match row.get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector, got {other:?}"),
    }
}

fn n_negative(row: &Record) -> i64 {
    match row.get("n_negative") {
        Some(Value::Integer(n)) => *n,
        other => panic!("expected n_negative Integer, got {other:?}"),
    }
}

fn instability(row: &Record) -> f64 {
    match row.get("instability_fraction") {
        Some(Value::Float(f)) => *f,
        other => panic!("expected instability_fraction Float, got {other:?}"),
    }
}

// ── M1 — known positive-definite spectrum ───────────────────────────

/// [[2,−1],[−1,2]] via off-diag −1 + self-loop diagonals 2 → {1, 3},
/// n_negative 0.
#[test]
fn m1_known_spectrum_positive_definite() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_hessian_bundle(&mut engine, "m1");
    engine
        .batch_insert(
            "m1",
            &[edge(0, 1, -1.0), edge(0, 0, 2.0), edge(1, 1, 2.0)],
        )
        .expect("batch_insert");

    let rows = run_matrix(&mut engine, "SPECTRAL m1 ON FIBER (h_ij) MODE MATRIX FULL;");
    assert_eq!(rows.len(), 1, "single summary row");
    let v = eigs(&rows[0]);
    assert_eq!(v.len(), 2);
    assert!((v[0] - 1.0).abs() < 1e-12, "λ0 got {}", v[0]);
    assert!((v[1] - 3.0).abs() < 1e-12, "λ1 got {}", v[1]);
    assert_eq!(n_negative(&rows[0]), 0, "positive-definite: no negatives");
    assert!(instability(&rows[0]).abs() < 1e-12, "instability 0");
    match rows[0].get("mode_used") {
        Some(Value::Text(m)) => assert_eq!(m, "matrix"),
        other => panic!("expected mode_used Text \"matrix\", got {other:?}"),
    }
    match rows[0].get("n_records_used") {
        Some(Value::Integer(n)) => assert_eq!(*n, 3),
        other => panic!("expected n_records_used Integer, got {other:?}"),
    }
}

// ── M2 — negatives survive (THE core) ───────────────────────────────

/// [[0,1],[1,0]] via a single off-diagonal edge → {−1, +1},
/// n_negative 1, instability_fraction 0.5. A Laplacian would lose this.
#[test]
fn m2_negatives_survive() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_hessian_bundle(&mut engine, "m2");
    engine
        .batch_insert("m2", &[edge(0, 1, 1.0)])
        .expect("batch_insert");

    let rows = run_matrix(&mut engine, "SPECTRAL m2 ON FIBER (h_ij) MODE MATRIX FULL;");
    let v = eigs(&rows[0]);
    assert_eq!(v.len(), 2);
    assert!((v[0] + 1.0).abs() < 1e-12, "λ0 got {} (want −1)", v[0]);
    assert!((v[1] - 1.0).abs() < 1e-12, "λ1 got {} (want +1)", v[1]);
    assert_eq!(n_negative(&rows[0]), 1, "one negative eigenvalue survives");
    assert!(
        (instability(&rows[0]) - 0.5).abs() < 1e-12,
        "instability 1/2 = 0.5, got {}",
        instability(&rows[0])
    );
}

// ── M3 — diagonal correctness + shift ───────────────────────────────

/// Tridiagonal [[2,1,0],[1,2,1],[0,1,2]] via off-diags 1 + self-loop
/// diagonals 2 → {2−√2, 2, 2+√2}. Toggling the diagonal to 0 shifts the
/// whole spectrum by −2 → {−√2, 0, √2} (and flips one negative on).
#[test]
fn m3_diagonal_correctness_and_shift() {
    let mut engine = Engine::open_memory().expect("memory engine");
    let s2 = std::f64::consts::SQRT_2;

    // Diagonal = 2.
    make_hessian_bundle(&mut engine, "m3_two");
    engine
        .batch_insert(
            "m3_two",
            &[
                edge(0, 1, 1.0),
                edge(1, 2, 1.0),
                edge(0, 0, 2.0),
                edge(1, 1, 2.0),
                edge(2, 2, 2.0),
            ],
        )
        .expect("batch_insert");
    let r2 = run_matrix(&mut engine, "SPECTRAL m3_two ON FIBER (h_ij) MODE MATRIX FULL;");
    let v2 = eigs(&r2[0]);
    assert_eq!(v2.len(), 3);
    let want2 = [2.0 - s2, 2.0, 2.0 + s2];
    for (i, (g, w)) in v2.iter().zip(want2.iter()).enumerate() {
        assert!((g - w).abs() < 1e-12, "diag=2 λ{i}: got {g}, want {w}");
    }
    assert_eq!(n_negative(&r2[0]), 0);

    // Same off-diagonals, diagonal = 0 → spectrum shifts by −2.
    make_hessian_bundle(&mut engine, "m3_zero");
    engine
        .batch_insert("m3_zero", &[edge(0, 1, 1.0), edge(1, 2, 1.0)])
        .expect("batch_insert");
    let r0 = run_matrix(&mut engine, "SPECTRAL m3_zero ON FIBER (h_ij) MODE MATRIX FULL;");
    let v0 = eigs(&r0[0]);
    assert_eq!(v0.len(), 3);
    let want0 = [-s2, 0.0, s2];
    for (i, (g, w)) in v0.iter().zip(want0.iter()).enumerate() {
        assert!((g - w).abs() < 1e-12, "diag=0 λ{i}: got {g}, want {w}");
    }
    // Exact shift: every diag=2 eigenvalue is diag=0 eigenvalue + 2.
    for (a, b) in v2.iter().zip(v0.iter()) {
        assert!((a - (b + 2.0)).abs() < 1e-12, "shift by +2: {a} vs {b}+2");
    }
    assert_eq!(n_negative(&r0[0]), 1, "diag=0 flips one negative on");
}

// ── M4 — symmetry / no double count ─────────────────────────────────

/// A single (i,j) edge mirrors to M[j][i]; supplying BOTH (i,j) and
/// (j,i) with equal h does NOT double-count (M stays the intended
/// matrix, not 2×).
#[test]
fn m4_symmetry_no_double_count() {
    // Single edge (0,1,3) → M=[[0,3],[3,0]] → {−3, 3}.
    let mut engine = Engine::open_memory().expect("memory engine");
    make_hessian_bundle(&mut engine, "m4_single");
    engine
        .batch_insert("m4_single", &[edge(0, 1, 3.0)])
        .expect("batch_insert");
    let rs = run_matrix(&mut engine, "SPECTRAL m4_single ON FIBER (h_ij) MODE MATRIX FULL;");
    let vs = eigs(&rs[0]);
    assert!((vs[0] + 3.0).abs() < 1e-12 && (vs[1] - 3.0).abs() < 1e-12, "single: {vs:?}");

    // Both (0,1,3) and (1,0,3) → still M=[[0,3],[3,0]] → {−3, 3}, NOT
    // [[0,6],[6,0]] → {−6, 6}.
    make_hessian_bundle(&mut engine, "m4_both");
    engine
        .batch_insert("m4_both", &[edge(0, 1, 3.0), edge(1, 0, 3.0)])
        .expect("batch_insert");
    let rb = run_matrix(&mut engine, "SPECTRAL m4_both ON FIBER (h_ij) MODE MATRIX FULL;");
    let vb = eigs(&rb[0]);
    assert!(
        (vb[0] + 3.0).abs() < 1e-12 && (vb[1] - 3.0).abs() < 1e-12,
        "both endpoints must NOT double-count, got {vb:?} (double-count would be ±6)"
    );
}

// ── M5 — no-GROUP + MATRIX ≠ Laplacian ──────────────────────────────

/// MODE MATRIX needs no GROUP; a GROUP clause is ignored. And the raw
/// matrix spectrum differs from the D−W Laplacian on the same edge:
/// the matrix keeps a negative eigenvalue, the Laplacian is PSD.
#[test]
fn m5_no_group_and_matrix_is_not_laplacian() {
    let mut engine = Engine::open_memory().expect("memory engine");

    // Raw matrix on a single edge (0,1,1): [[0,1],[1,0]] → {−1, 1}.
    make_hessian_bundle(&mut engine, "raw");
    engine
        .batch_insert("raw", &[edge(0, 1, 1.0)])
        .expect("batch_insert");
    let rm = run_matrix(&mut engine, "SPECTRAL raw ON FIBER (h_ij) MODE MATRIX FULL;");
    let vm = eigs(&rm[0]);
    assert!((vm[0] + 1.0).abs() < 1e-12, "matrix keeps the negative: {vm:?}");
    assert_eq!(n_negative(&rm[0]), 1);

    // The D−W Laplacian of the SAME single edge (θ=0 U(1) gauge path):
    // L=[[1,−1],[−1,1]] → {0, 2}, PSD — no negatives.
    let schema = BundleSchema::new("lap")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("theta"));
    engine.create_bundle(schema).expect("create_bundle");
    let mut r = Record::new();
    r.insert("vertex_a".to_string(), Value::Integer(0));
    r.insert("vertex_b".to_string(), Value::Integer(1));
    r.insert("theta".to_string(), Value::Float(0.0));
    engine.insert("lap", &r).expect("insert");
    let lap = spectral_gauge_gap(
        &engine,
        "lap",
        &["theta".to_string()],
        Group::U1,
        true,
        None,
        None,
    )
    .expect("gauge Laplacian FULL");
    let vl = lap.eigenvalues.expect("FULL eigenvalues");
    assert!(
        vl.iter().all(|&x| x > -1e-9),
        "Laplacian must be PSD (no negatives): {vl:?}"
    );
    assert!(
        (vm[0] - vl[0]).abs() > 0.5,
        "MATRIX ≠ Laplacian: matrix min {} vs Laplacian min {}",
        vm[0],
        vl[0]
    );

    // No-GROUP ergonomic: MODE MATRIX parses without GROUP.
    assert!(
        parse("SPECTRAL raw ON FIBER (h_ij) MODE MATRIX FULL").is_ok(),
        "MODE MATRIX must not require GROUP"
    );
    // A GROUP clause present is ignored (identical result).
    let rg = run_matrix(&mut engine, "SPECTRAL raw ON FIBER (h_ij) MODE MATRIX GROUP U(1) FULL;");
    assert_eq!(eigs(&rg[0]), vm, "GROUP clause must be ignored");
}

// ── M6 — PNP plumbing (instability ordering) ────────────────────────

/// Two synthetic block-diagonal Hessians: a "2-SAT-like" one with 1
/// negative eigenvalue (instability 0.25) and a "3-SAT-like" one with 3
/// (instability 0.75). instability_3 > instability_2 with ratio 3 —
/// pins the end-to-end signal path (synthetic, not a real SAT result).
#[test]
fn m6_pnp_plumbing_instability_ordering() {
    let mut engine = Engine::open_memory().expect("memory engine");

    // P-like: blocks [[0,1],[1,0]] (→ {−1,1}) ⊕ [[2,1],[1,2]] (→ {1,3}).
    make_hessian_bundle(&mut engine, "p_like");
    engine
        .batch_insert(
            "p_like",
            &[
                edge(0, 1, 1.0),
                edge(2, 3, 1.0),
                edge(2, 2, 2.0),
                edge(3, 3, 2.0),
            ],
        )
        .expect("batch_insert");
    let rp = run_matrix(&mut engine, "SPECTRAL p_like ON FIBER (h_ij) MODE MATRIX FULL;");
    assert_eq!(n_negative(&rp[0]), 1);
    assert!((instability(&rp[0]) - 0.25).abs() < 1e-12, "P instability 0.25");

    // NP-like: blocks [[0,1],[1,0]] (→ {−1,1}) ⊕ [[−2,1],[1,−2]] (→ {−3,−1}).
    make_hessian_bundle(&mut engine, "np_like");
    engine
        .batch_insert(
            "np_like",
            &[
                edge(0, 1, 1.0),
                edge(2, 3, 1.0),
                edge(2, 2, -2.0),
                edge(3, 3, -2.0),
            ],
        )
        .expect("batch_insert");
    let rn = run_matrix(&mut engine, "SPECTRAL np_like ON FIBER (h_ij) MODE MATRIX FULL;");
    assert_eq!(n_negative(&rn[0]), 3);
    assert!((instability(&rn[0]) - 0.75).abs() < 1e-12, "NP instability 0.75");

    assert!(
        instability(&rn[0]) > instability(&rp[0]),
        "3-SAT-like must be more unstable than 2-SAT-like"
    );
    assert!(
        ((instability(&rn[0]) / instability(&rp[0])) - 3.0).abs() < 1e-9,
        "constructed ratio 0.75/0.25 = 3"
    );
}

// ── M7 — LIMIT clamp + typed errors ─────────────────────────────────

/// k > V clamps to V; LIMIT 0 is a typed error; an empty edge set is a
/// typed error; V > 4096 surfaces the SparseUnavailable-shaped error
/// naming Phase 2.1.
#[test]
fn m7_limit_clamp_and_typed_errors() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_hessian_bundle(&mut engine, "clamp");
    engine
        .batch_insert(
            "clamp",
            &[edge(0, 1, -1.0), edge(0, 0, 2.0), edge(1, 1, 2.0)],
        )
        .expect("batch_insert");

    // k > V clamps to V = 2.
    let rows = run_matrix(
        &mut engine,
        "SPECTRAL clamp ON FIBER (h_ij) MODE MATRIX FULL LIMIT 99;",
    );
    assert_eq!(eigs(&rows[0]).len(), 2, "LIMIT 99 on V=2 clamps to 2");
    // n_negative still over the FULL spectrum even with LIMIT.
    assert_eq!(n_negative(&rows[0]), 0);

    // LIMIT 0 → typed error naming LIMIT.
    let stmt = parse("SPECTRAL clamp ON FIBER (h_ij) MODE MATRIX FULL LIMIT 0;")
        .expect("LIMIT 0 must parse");
    let err = execute(&mut engine, &stmt).expect_err("LIMIT 0 must be rejected");
    assert!(err.contains("LIMIT"), "LIMIT-bounds error must name LIMIT: {err}");

    // Empty edge set → typed error.
    make_hessian_bundle(&mut engine, "empty");
    let stmt =
        parse("SPECTRAL empty ON FIBER (h_ij) MODE MATRIX FULL;").expect("must parse");
    let err = execute(&mut engine, &stmt).expect_err("empty edge set must error");
    assert!(!err.is_empty(), "empty bundle must error loudly: {err}");

    // V > 4096 → SparseUnavailable-shaped error naming Phase 2.1.
    make_hessian_bundle(&mut engine, "big");
    let batch: Vec<Record> = (0..4100)
        .map(|i| edge(i as i64, i as i64 + 1, 1.0))
        .collect();
    engine.batch_insert("big", &batch).expect("batch_insert");
    let stmt = parse("SPECTRAL big ON FIBER (h_ij) MODE MATRIX FULL;").expect("must parse");
    let err = execute(&mut engine, &stmt).expect_err("V>4096 must error until sparse lands");
    assert!(err.contains("4096"), "threshold error must name 4096: {err}");
    assert!(
        err.contains("Phase 2.1"),
        "threshold error must name the Phase 2.1 sparse deferral: {err}"
    );
}

// ── DIAGONAL <field> override column ────────────────────────────────

/// DIAGONAL <field> makes self-loop records read the named column for
/// the diagonal instead of the fiber field — proving Option S with an
/// override. h_ij=999 on the self-loops is ignored; hd=2 is used.
#[test]
fn diagonal_override_named_column() {
    let mut engine = Engine::open_memory().expect("memory engine");
    let schema = BundleSchema::new("dov")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("h_ij"))
        .fiber(FieldDef::numeric("hd"));
    engine.create_bundle(schema).expect("create_bundle");

    // off-diagonal: h_ij = −1 (hd unused).
    let mut off = Record::new();
    off.insert("vertex_a".to_string(), Value::Integer(0));
    off.insert("vertex_b".to_string(), Value::Integer(1));
    off.insert("h_ij".to_string(), Value::Float(-1.0));
    off.insert("hd".to_string(), Value::Float(0.0));
    engine.insert("dov", &off).expect("insert");

    // self-loops: h_ij = 999 (MUST be ignored), diagonal read from hd = 2.
    for v in 0..2 {
        let mut d = Record::new();
        d.insert("vertex_a".to_string(), Value::Integer(v));
        d.insert("vertex_b".to_string(), Value::Integer(v));
        d.insert("h_ij".to_string(), Value::Float(999.0));
        d.insert("hd".to_string(), Value::Float(2.0));
        engine.insert("dov", &d).expect("insert");
    }

    let rows = run_matrix(
        &mut engine,
        "SPECTRAL dov ON FIBER (h_ij) MODE MATRIX DIAGONAL hd FULL;",
    );
    let v = eigs(&rows[0]);
    // M = [[2,−1],[−1,2]] (diagonal from hd=2, NOT h_ij=999) → {1, 3}.
    assert!(
        (v[0] - 1.0).abs() < 1e-12 && (v[1] - 3.0).abs() < 1e-12,
        "DIAGONAL override must read hd, got {v:?}"
    );
    assert_eq!(n_negative(&rows[0]), 0);
}
