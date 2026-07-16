//! SPECTRAL / SPECTRAL_GAUGE `FULL [LIMIT k]` — Phase 2 dense path (Concept A).
//!
//! RED phase (2026-07-16): these tests pin the FULL contract Hallie
//! confirmed for the RH sweep loop. They fail on the Phase-1 tree,
//! where `spectral_gauge_gap(.., full=true, ..)` returns
//! `PhaseNotImplemented` and plain `SPECTRAL <b> FULL` ignores the
//! flag entirely.
//!
//! Contract under test (per SPECTRAL_GAUGE_PHASE2_SPEC.md §6 as
//! reconciled by the 2026-07-16 orchestrator rulings R1-R5):
//!
//! - `FULL` populates `eigenvalues: Some(vec)`, ascending ALGEBRAIC
//!   (R3 — deviation from the spec's ascending-by-|λ|, named in the
//!   ship report). `FULL` without `LIMIT` returns ALL eigenvalues.
//! - `FULL LIMIT k` returns the k smallest; k > V clamps to V;
//!   k = 0 is a typed error (LIMIT bounds).
//! - The `gap` field stays the Phase-1 λ₁ (first |λ| > 1e-9) whether
//!   or not FULL is present — Phase-1 consumers see no change.
//! - Dense path only up to V = 4096 (spec §6 dense/sparse threshold);
//!   FULL on V > 4096 surfaces a SparseUnavailable-shaped error naming
//!   Phase 2.1 (R2 — Lanczos deferred rather than rushed).
//! - Result struct gains `mode_used` (spec §6); dense path reports
//!   `SpectralGaugeMode::Dense` with `convergence: None` (R4).
//! - Wire envelope: `eigenvalues` (+ `mode_used`) appear ONLY when
//!   FULL is present; the λ₁-only envelope {gap, n_records_used,
//!   group_used} stays byte-identical without FULL (probe S6).
//!
//! Closed-form anchors (exact to 1e-9):
//! - Cycle C_n, uniform unit weights: λ_k = 2 − 2cos(2πk/n), k=0..n−1.
//! - Complete graph K_n, unit weights: {0, n, n, ..., n}.
//! Unit weights are produced through the U(1) path with θ = 0
//! (w_e = cos 0 = 1), which also pins the exact fiber the RH loop
//! reads (`ON FIBER (theta) GROUP U(1)`).
//!
//! Run with:
//!   `cargo test --features halcyon --test spectral_full_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::Group;
use gigi::parser::{execute, parse, ExecResult, Statement};
use gigi::spectral::spectral_gauge_gap;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── Fixture helpers (mirror tests/spectral_gauge_basic.rs) ─────────

/// Heap bundle with the Halcyon edge schema and a single U(1) fiber
/// column `theta`.
fn make_theta_bundle(engine: &mut Engine, name: &str) {
    let schema = BundleSchema::new(name)
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("theta"));
    engine
        .create_bundle(schema)
        .expect("create_bundle should succeed");
}

fn theta_edge(va: i64, vb: i64, theta: f64) -> Record {
    let mut rec = Record::new();
    rec.insert("vertex_a".to_string(), Value::Integer(va));
    rec.insert("vertex_b".to_string(), Value::Integer(vb));
    rec.insert("theta".to_string(), Value::Float(theta));
    rec
}

/// Insert a θ=0 cycle C_n (unit cos-weights).
fn insert_cycle(engine: &mut Engine, name: &str, n: usize) {
    let batch: Vec<Record> = (0..n)
        .map(|i| theta_edge(i as i64, ((i + 1) % n) as i64, 0.0))
        .collect();
    engine.batch_insert(name, &batch).expect("batch_insert");
}

/// Insert a θ=0 complete graph K_n (unit cos-weights).
fn insert_complete(engine: &mut Engine, name: &str, n: usize) {
    let mut batch = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            batch.push(theta_edge(i as i64, j as i64, 0.0));
        }
    }
    engine.batch_insert(name, &batch).expect("batch_insert");
}

fn theta_fiber() -> Vec<String> {
    vec!["theta".to_string()]
}

/// Closed-form combinatorial-Laplacian spectrum of C_n with unit
/// weights: 2 − 2cos(2πk/n), sorted ascending.
fn cycle_spectrum(n: usize) -> Vec<f64> {
    let mut vals: Vec<f64> = (0..n)
        .map(|k| 2.0 - 2.0 * (2.0 * std::f64::consts::PI * k as f64 / n as f64).cos())
        .collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    vals
}

// ── FULL returns the whole spectrum, ascending ──────────────────────

/// (A1) C_6 with unit weights: FULL returns all 6 eigenvalues
/// matching 2 − 2cos(2πk/6) to 1e-9, sorted ascending, and the gap
/// field still carries the Phase-1 λ₁ (= 1.0 for C_6).
#[test]
fn test_full_returns_all_eigenvalues_ascending_cycle_c6() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c6");
    insert_cycle(&mut engine, "c6", 6);

    let result = spectral_gauge_gap(&engine, "c6", &theta_fiber(), Group::U1, true, None, None)
        .expect("FULL on C_6 must succeed in Phase 2");
    let vals = result
        .eigenvalues
        .expect("FULL must populate eigenvalues");
    assert_eq!(vals.len(), 6, "FULL without LIMIT returns ALL eigenvalues");
    let expected = cycle_spectrum(6);
    for (i, (got, want)) in vals.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "C_6 eigenvalue {i}: got {got}, want {want}"
        );
    }
    // Ascending order.
    for w in vals.windows(2) {
        assert!(w[0] <= w[1] + 1e-12, "eigenvalues must ascend: {w:?}");
    }
    // gap = first |λ| > 1e-9 = 2 − 2cos(2π/6) = 1.0.
    assert!(
        (result.gap - 1.0).abs() < 1e-9,
        "gap must stay the Phase-1 λ₁, got {}",
        result.gap
    );
}

/// (A2) K_8 with unit weights: FULL returns {0, 8×7} to 1e-9.
#[test]
fn test_full_returns_complete_graph_k8_closed_form() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "k8");
    insert_complete(&mut engine, "k8", 8);

    let result = spectral_gauge_gap(&engine, "k8", &theta_fiber(), Group::U1, true, None, None)
        .expect("FULL on K_8 must succeed in Phase 2");
    let vals = result.eigenvalues.expect("FULL must populate eigenvalues");
    assert_eq!(vals.len(), 8);
    assert!(vals[0].abs() < 1e-9, "K_n lowest eigenvalue is 0, got {}", vals[0]);
    for (i, v) in vals.iter().enumerate().skip(1) {
        assert!(
            (v - 8.0).abs() < 1e-9,
            "K_8 eigenvalue {i}: got {v}, want 8.0"
        );
    }
}

// ── LIMIT k semantics ───────────────────────────────────────────────

/// (A3) FULL LIMIT 4 on C_10 returns exactly the 4 smallest
/// eigenvalues of the closed form.
#[test]
fn test_full_limit_k_truncates_to_k_smallest() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c10");
    insert_cycle(&mut engine, "c10", 10);

    let result =
        spectral_gauge_gap(&engine, "c10", &theta_fiber(), Group::U1, true, Some(4), None)
            .expect("FULL LIMIT 4 on C_10 must succeed");
    let vals = result.eigenvalues.expect("FULL must populate eigenvalues");
    assert_eq!(vals.len(), 4, "LIMIT 4 returns exactly 4 eigenvalues");
    let expected = cycle_spectrum(10);
    for (i, (got, want)) in vals.iter().zip(expected.iter().take(4)).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "C_10 eigenvalue {i}: got {got}, want {want}"
        );
    }
}

/// (A4) FULL LIMIT 0 is a typed bounds error (not a silent empty vec).
#[test]
fn test_full_limit_zero_errors() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c4z");
    insert_cycle(&mut engine, "c4z", 4);

    let err = spectral_gauge_gap(&engine, "c4z", &theta_fiber(), Group::U1, true, Some(0), None)
        .expect_err("FULL LIMIT 0 must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("LIMIT"),
        "LIMIT-bounds error must name LIMIT: {msg}"
    );
}

/// (A5) FULL LIMIT 99 on C_6 clamps to V = 6 eigenvalues.
#[test]
fn test_full_limit_exceeding_v_clamps() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c6clamp");
    insert_cycle(&mut engine, "c6clamp", 6);

    let result =
        spectral_gauge_gap(&engine, "c6clamp", &theta_fiber(), Group::U1, true, Some(99), None)
            .expect("FULL LIMIT > V must clamp, not error");
    let vals = result.eigenvalues.expect("FULL must populate eigenvalues");
    assert_eq!(vals.len(), 6, "LIMIT 99 on V=6 clamps to 6");
}

// ── Phase-1 continuity ──────────────────────────────────────────────

/// (A6) The gap under FULL equals the gap without FULL on the same
/// bundle (identical λ₁ extraction rule).
#[test]
fn test_full_gap_field_matches_phase1_lambda1() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c7");
    insert_cycle(&mut engine, "c7", 7);

    let gap_only =
        spectral_gauge_gap(&engine, "c7", &theta_fiber(), Group::U1, false, None, None)
            .expect("Phase-1 gap path");
    let full = spectral_gauge_gap(&engine, "c7", &theta_fiber(), Group::U1, true, None, None)
        .expect("FULL path");
    assert_eq!(
        gap_only.gap, full.gap,
        "gap must be identical with and without FULL"
    );
    assert!(
        gap_only.eigenvalues.is_none(),
        "non-FULL keeps eigenvalues None (Phase-1 shape)"
    );
}

// ── Dense/sparse threshold (spec §6: V = 4096) ──────────────────────

/// (A7) FULL on a V > 4096 graph surfaces the SparseUnavailable-shaped
/// error naming Phase 2.1 (R2: Lanczos deferred, honest error instead).
/// Path graph on 4101 vertices — assembly must NOT be attempted.
#[test]
fn test_full_dense_threshold_v_gt_4096_errors() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "big_path");
    let batch: Vec<Record> = (0..4100)
        .map(|i| theta_edge(i as i64, i as i64 + 1, 0.0))
        .collect();
    engine.batch_insert("big_path", &batch).expect("batch_insert");

    let err =
        spectral_gauge_gap(&engine, "big_path", &theta_fiber(), Group::U1, true, Some(4), None)
            .expect_err("FULL above the dense threshold must error until sparse lands");
    let msg = err.to_string();
    assert!(
        msg.contains("4096"),
        "threshold error must name the 4096 boundary: {msg}"
    );
    assert!(
        msg.contains("Phase 2.1"),
        "threshold error must name the Phase 2.1 sparse deferral: {msg}"
    );
}

// ── Executor envelope (parser::execute — same arm shape as HTTP) ────

/// (A8) SPECTRAL_GAUGE ... FULL LIMIT 3 through parse + execute:
/// Rows[0] carries eigenvalues (Vector, len 3, ascending), mode_used
/// = "dense", and the Phase-1 fields are still present.
#[test]
fn test_executor_envelope_full_carries_eigenvalues_vector() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "ring8");
    insert_cycle(&mut engine, "ring8", 8);

    let stmt = parse("SPECTRAL_GAUGE ring8 ON FIBER (theta) GROUP U(1) FULL LIMIT 3;")
        .expect("FULL LIMIT grammar must parse");
    let result = execute(&mut engine, &stmt).expect("executor must run FULL in Phase 2");
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "single summary row");
    let row = &rows[0];
    let vals = match row.get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector in envelope, got {other:?}"),
    };
    assert_eq!(vals.len(), 3, "LIMIT 3 → 3 eigenvalues on the wire");
    for w in vals.windows(2) {
        assert!(w[0] <= w[1] + 1e-12, "wire eigenvalues must ascend");
    }
    match row.get("mode_used") {
        Some(Value::Text(m)) => assert_eq!(m, "dense"),
        other => panic!("expected mode_used Text, got {other:?}"),
    }
    assert!(row.contains_key("gap"), "Phase-1 gap field must remain");
    assert!(row.contains_key("n_records_used"));
    assert!(row.contains_key("group_used"));
}

/// (A9) Without FULL the wire envelope is byte-compatible with
/// Phase 1: exactly {gap, n_records_used, group_used}, no eigenvalues,
/// no mode_used (probe S6 pin — green today, must stay green).
#[test]
fn test_executor_envelope_without_full_unchanged() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "ring5");
    insert_cycle(&mut engine, "ring5", 5);

    let stmt = parse("SPECTRAL_GAUGE ring5 ON FIBER (theta) GROUP U(1);")
        .expect("Phase-1 grammar must parse");
    let result = execute(&mut engine, &stmt).expect("Phase-1 λ₁ path must run");
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(
        row.len(),
        3,
        "λ₁-only envelope must stay exactly {{gap, n_records_used, group_used}}, got keys: {:?}",
        row.keys().collect::<Vec<_>>()
    );
    assert!(row.contains_key("gap"));
    assert!(row.contains_key("n_records_used"));
    assert!(row.contains_key("group_used"));
}

// ── Plain SPECTRAL FULL [LIMIT k] ───────────────────────────────────

/// (A10) Plain `SPECTRAL <b> FULL LIMIT 5` parses and, through the
/// executor, returns a Rows envelope with the normalized-Laplacian
/// spectrum of the field-index graph. Anchor: an all-same-indexed-value
/// store is K_12; the normalized Laplacian of K_n has eigenvalues
/// {0, n/(n−1) × (n−1)}, so LIMIT 5 → [0, 12/11, 12/11, 12/11, 12/11].
#[test]
fn test_plain_spectral_full_limit_returns_normalized_spectrum() {
    let mut engine = Engine::open_memory().expect("memory engine");
    let schema = BundleSchema::new("flat12")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("color"))
        .index("color");
    engine.create_bundle(schema).expect("create_bundle");
    for i in 0..12 {
        let mut r = Record::new();
        r.insert("id".to_string(), Value::Integer(i));
        r.insert("color".to_string(), Value::Text("Red".to_string()));
        engine.insert("flat12", &r).expect("insert");
    }

    let stmt = parse("SPECTRAL flat12 FULL LIMIT 5;")
        .expect("plain SPECTRAL FULL LIMIT grammar must parse");
    let result = execute(&mut engine, &stmt).expect("plain SPECTRAL FULL must run");
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows envelope for SPECTRAL FULL, got {other:?}"),
    };
    assert_eq!(rows.len(), 1);
    let vals = match rows[0].get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector, got {other:?}"),
    };
    assert_eq!(vals.len(), 5, "LIMIT 5 → 5 eigenvalues");
    assert!(vals[0].abs() < 1e-9, "normalized K_n lowest is 0, got {}", vals[0]);
    let expected = 12.0 / 11.0;
    for (i, v) in vals.iter().enumerate().skip(1) {
        assert!(
            (v - expected).abs() < 1e-9,
            "normalized K_12 eigenvalue {i}: got {v}, want {expected}"
        );
    }
}

/// (A11) Plain `SPECTRAL <b>` without FULL keeps the Phase-1 Scalar
/// envelope (λ₁ of the normalized Laplacian) — no shape change.
#[test]
fn test_plain_spectral_without_full_stays_scalar() {
    let mut engine = Engine::open_memory().expect("memory engine");
    let schema = BundleSchema::new("flat6")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("color"))
        .index("color");
    engine.create_bundle(schema).expect("create_bundle");
    for i in 0..6 {
        let mut r = Record::new();
        r.insert("id".to_string(), Value::Integer(i));
        r.insert("color".to_string(), Value::Text("Red".to_string()));
        engine.insert("flat6", &r).expect("insert");
    }

    let stmt = parse("SPECTRAL flat6;").expect("plain SPECTRAL grammar");
    let result = execute(&mut engine, &stmt).expect("plain SPECTRAL must run");
    match result {
        ExecResult::Scalar(v) => {
            // K_6 normalized λ₁ = 6/5 = 1.2.
            assert!(
                (v - 1.2).abs() < 1e-6,
                "plain SPECTRAL λ₁ on K_6: got {v}, want 1.2"
            );
        }
        other => panic!("plain SPECTRAL without FULL must stay Scalar, got {other:?}"),
    }
}

// ── Grammar pins ────────────────────────────────────────────────────

/// (A12) SPECTRAL_GAUGE grammar: FULL LIMIT k round-trips through the
/// AST (this already parses on the Phase-1 tree — kept as a pin so the
/// Concept-B MODE clause cannot regress it).
#[test]
fn test_parse_spectral_gauge_full_limit_roundtrip() {
    let stmt = parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) FULL LIMIT 8")
        .expect("parse FULL LIMIT 8");
    match stmt {
        Statement::SpectralGauge { full, limit, group, .. } => {
            assert!(full);
            assert_eq!(limit, Some(8));
            assert_eq!(group, Some(Group::U1));
        }
        other => panic!("expected SpectralGauge, got {other:?}"),
    }
}
