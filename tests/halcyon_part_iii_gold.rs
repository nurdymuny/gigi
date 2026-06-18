//! TDD-HAL-III.8b — Gold gate. Halcyon Gate III contract via cross-
//! binding tests, anchored on the GIGI-internal canonical reference
//! `tests/fixtures/halcyon/part_iii/p_canonical.json` that gate III.8a
//! harvested.
//!
//! Per Bee's locked decision D1: cross-binding bit-identity against
//! Halcyon's NumPy PCG64 mock is impossible by design. The bit-identity
//! contract this gate enforces is **intra-GIGI** — same code, same seed,
//! same OS → byte-identical `f64` history. The fixture harvested in
//! III.8a is the sentinel; this gate fails loudly the moment the GIGI
//! side drifts from its own past.
//!
//! ── Map onto Halcyon mock's Gate III.A–E ──
//!
//! - **III.A** (PLAQUETTE per-face matches the kernel) — gate (a) below.
//!   Tests `SELECT PLAQUETTE OF U;` and `SELECT MEAN(PLAQUETTE OF U);`
//!   on an IDENTITY field. The per-face shape is `Vec<f64>` of length
//!   `F = 32` (D7); every entry is FP64-exact `1.0`; MEAN is FP64-exact
//!   `1.0`.
//!
//! - **III.B-zero** (Q_SURROGATE at identity is 0; HAAR keeps it in
//!   range) — gate (b) below. Tests `SELECT Q_SURROGATE OF U;` on an
//!   IDENTITY field (must be `|v| < 1e-12`) and on an INIT HAAR_RANDOM
//!   SEED 20260616 field (must land in `[0, F/2] = [0, 16]`, the
//!   buckyball range).
//!
//! - **III.B-reject** (E-field observables surface a typed error) —
//!   gate (c) below. `GIBBS_SAMPLE … MEASURE (HTotal)` returns an
//!   error string matching the regex `(?i)part iv|e field`. Mirrors
//!   the III.5 unit test surface at the parser+executor boundary.
//!
//! - **III.C** (GIBBS_SAMPLE short-run reproducibility in process) —
//!   gate (d) below. Two independent gauge fields on two independent
//!   lattices, both INIT IDENTITY + `BETA 2.5 N_SWEEPS 20 SEED
//!   20260616`, end with byte-identical link buffers (intra-binding
//!   bit-identity — Bee's locked decision 1).
//!
//! - **III.D** (production thermalization pass criterion) — gate (e)
//!   below. The load-bearing receipt: 2048-sweep production run
//!   reproduces the III.8a fixture byte-for-byte (every entry of
//!   `P_history` decodes via `f64::from_bits` to the same `u64` the
//!   fixture stores). Length / probability bounds + the diagnostics
//!   block are the structural envelope.
//!
//! ── Optionality ──
//!
//! Gated on the `halcyon` composite feature so the no-default-features
//! build stays byte-identical at 852/0 (Bee's optionality contract,
//! Part I/II constraint that carries through every Part III gate).

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

/// Path to the III.8a canonical-reference fixture, anchored to the
/// test crate's manifest dir so `cargo test` from anywhere finds it.
fn p_canonical_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_iii")
        .join("p_canonical.json")
}

/// Drop the lattice + gauge registries to a clean slate at test entry.
/// The III.5 + III.6 + III.7 tests all share the singleton registries;
/// holding `test_serial_lock()` for the test's lifetime serializes
/// against every other gauge test in the workspace and clearing here
/// gives this gate a known-clean starting point.
fn clear_registries() {
    gigi::gauge::registry::clear();
    gigi::lattice::registry::clear();
}

/// Materialize an IDENTITY-init SU(2) field named `field_name` on a
/// freshly declared buckyball lattice named `lattice_name`, going
/// through the parser+executor path (so the test exercises the same
/// surface a Halcyon caller would). Re-publishes the field into the
/// SU(2)-mut registry (D4) so subsequent `GIBBS_SAMPLE` calls can lock
/// the mutable buffer. Mirrors the III.6 smoke / III.8a harvest
/// fix-up pattern.
fn declare_identity_field(
    engine: &mut gigi::engine::Engine,
    lattice_name: &str,
    field_name: &str,
) {
    let lat_decl = format!(
        "LATTICE {lattice_name} FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';"
    );
    let stmt = parse(&lat_decl).expect("parse LATTICE");
    execute(engine, &stmt).expect("exec LATTICE");

    let g_decl = format!(
        "GAUGE_FIELD {field_name} ON LATTICE {lattice_name} \
         GROUP SU(2) INIT IDENTITY;"
    );
    let stmt = parse(&g_decl).expect("parse GAUGE_FIELD");
    execute(engine, &stmt).expect("exec GAUGE_FIELD");

    // Re-publish through `register_su2` so `gibbs_sample` finds the
    // SU(2)-mut handle it needs (D4). The parser GAUGE_FIELD arm
    // registers via `register` (Arc<dyn>) only — same fix-up the III.6
    // smoke test + the III.8a harvest harness use.
    let lat = gigi::lattice::registry::get(lattice_name)
        .expect("lattice declared above");
    let su2 = gigi::gauge::SU2GaugeField::new(
        field_name.into(),
        &lat,
        gigi::gauge::GaugeFieldInit::Identity,
        None,
    )
    .expect("identity init");
    gigi::gauge::registry::register_su2(su2);
}

/// TDD-HAL-III.8b: Halcyon Gate III.A — IDENTITY init's PLAQUETTE
/// per-face is `Vec<f64>` of length `F = 32` (D7), every entry FP64-
/// exact `1.0`; MEAN reduction is FP64-exact `1.0` (`32/32 = 1.0` in
/// IEEE-754).
#[test]
fn tdd_hal_iii_8b_a_plaquette_per_face_matches_kernel() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    declare_identity_field(&mut engine, "iii_8b_a_bb", "U_iii_8b_a");

    // SELECT PLAQUETTE OF U; → per_face Vec<f64> of length 32.
    let stmt = parse("SELECT PLAQUETTE OF U_iii_8b_a;").expect("parse PLAQUETTE");
    let rows = match execute(&mut engine, &stmt).expect("exec PLAQUETTE per_face") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for per-face, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "per-face envelope is single-row");
    match rows[0].get("reduction") {
        Some(Value::Text(s)) => assert_eq!(s, "per_face"),
        other => panic!("missing/wrong reduction: {other:?}"),
    }
    let per_face: &Vec<f64> = match rows[0].get("per_face") {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong per_face column: {other:?}"),
    };
    assert_eq!(per_face.len(), 32, "buckyball has F=32 faces");
    for (i, q) in per_face.iter().enumerate() {
        // FP64-exact: IDENTITY face holonomy is `q0 = 1.0` byte-exact.
        assert_eq!(*q, 1.0, "face {i}: expected 1.0 byte-exact, got {q}");
    }

    // SELECT MEAN(PLAQUETTE OF U); → scalar f64 = 1.0 (32/32 FP64-exact).
    let stmt = parse("SELECT MEAN(PLAQUETTE OF U_iii_8b_a);")
        .expect("parse MEAN(PLAQUETTE)");
    let rows = match execute(&mut engine, &stmt).expect("exec PLAQUETTE mean") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for mean, got {other:?}"),
    };
    match rows[0].get("value") {
        Some(Value::Float(v)) => {
            // The spec asks `abs(MEAN - 1.0) < 1e-14`; on IDENTITY
            // the math reduces to `32.0 / 32.0` which is FP64-exact.
            // Use a tolerance assert to mirror the spec literally.
            assert!(
                (*v - 1.0).abs() < 1e-14,
                "IDENTITY MEAN(PLAQUETTE) must be 1.0 to 1e-14, got {v}"
            );
        }
        other => panic!("missing/wrong value column: {other:?}"),
    }
}

/// TDD-HAL-III.8b: Halcyon Gate III.B (zero arm) — Q_SURROGATE at
/// IDENTITY is FP64-near-zero (every face's `arccos(q0=1.0)` is 0).
/// HAAR_RANDOM at SEED 20260616 stays in the buckyball range
/// `[0, F/2] = [0, 16]` (the geometric maximum of the angular
/// accumulator).
#[test]
fn tdd_hal_iii_8b_b_q_surrogate_at_identity_is_zero_and_in_range() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    declare_identity_field(&mut engine, "iii_8b_b_id_bb", "U_iii_8b_b_id");

    // IDENTITY arm: Q_SURROGATE = 0 to within FP64 roundoff at
    // arccos(1). The spec asks `abs(v) < 1e-12`.
    let stmt = parse("SELECT Q_SURROGATE OF U_iii_8b_b_id;")
        .expect("parse Q_SURROGATE identity");
    let rows = match execute(&mut engine, &stmt).expect("exec Q_SURROGATE identity") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for Q_SURROGATE, got {other:?}"),
    };
    match rows[0].get("value") {
        Some(Value::Float(v)) => {
            assert!(
                v.abs() < 1e-12,
                "IDENTITY Q_SURROGATE must be ~ 0 (< 1e-12), got {v}"
            );
        }
        other => panic!("missing/wrong value column: {other:?}"),
    }

    // HAAR_RANDOM arm: declare a fresh field on a fresh lattice (the
    // parser arm's `register` path is enough — Q_SURROGATE reads the
    // dyn handle, no SU(2)-mut needed).
    let lat_decl =
        "LATTICE iii_8b_b_haar_bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE for HAAR arm");
    execute(&mut engine, &stmt).expect("exec LATTICE for HAAR arm");

    let g_decl = "GAUGE_FIELD U_iii_8b_b_haar ON LATTICE iii_8b_b_haar_bb \
                  GROUP SU(2) INIT HAAR_RANDOM SEED 20260616;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD HAAR");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD HAAR");

    let stmt = parse("SELECT Q_SURROGATE OF U_iii_8b_b_haar;")
        .expect("parse Q_SURROGATE haar");
    let rows = match execute(&mut engine, &stmt).expect("exec Q_SURROGATE haar") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows for Q_SURROGATE haar, got {other:?}"),
    };
    match rows[0].get("value") {
        Some(Value::Float(v)) => {
            // Range bound `[0, F/2] = [0, 16]` follows from
            // `arccos(q0) ∈ [0, π]` and the `1/(2π)` normalization
            // (per-face contribution ≤ 1/2; F = 32 faces → max 16).
            // Inclusive bounds because the analytic supremum is
            // attainable in the limit of every face holonomy hitting
            // `q0 = -1`; FP64 clamp + arccos saturate at the bound
            // without overshoot.
            assert!(
                *v >= 0.0 && *v <= 16.0,
                "HAAR seed=20260616 Q_SURROGATE must be in [0, 16] \
                 (buckyball F/2 bound), got {v}"
            );
        }
        other => panic!("missing/wrong value column for HAAR arm: {other:?}"),
    }
}

/// TDD-HAL-III.8b: Halcyon Gate III.B (reject arm) — `GIBBS_SAMPLE
/// … MEASURE (H_TOTAL)` is a typed error before Part IV ships the E
/// field. The error string matches `(?i)part iv|e field`; the upstream
/// HTTP / parser layer surfaces the same anchor.
#[test]
fn tdd_hal_iii_8b_c_h_total_rejected_before_part_iv() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    declare_identity_field(&mut engine, "iii_8b_c_bb", "U_iii_8b_c");

    // GIBBS_SAMPLE with H_TOTAL must error before Part IV.
    let sample = "GIBBS_SAMPLE U_iii_8b_c BETA 2.5 N_SWEEPS 1 MEASURE_EVERY 1 \
                  MEASURE (H_TOTAL) SEED 20260616;";
    let stmt = parse(sample).expect("parse GIBBS_SAMPLE H_TOTAL");
    let err = execute(&mut engine, &stmt)
        .expect_err("MEASURE (H_TOTAL) must be a typed error before Part IV");
    let lower = err.to_lowercase();
    assert!(
        lower.contains("part iv") || lower.contains("e field"),
        "expected '(?i)part iv|e field' in error, got: {err}"
    );
}

/// TDD-HAL-III.8b: Halcyon Gate III.C — two independent gauge fields,
/// same INIT IDENTITY + same lattice topology + same `(β, n_sweeps,
/// seed) = (2.5, 20, 20260616)`, end with byte-identical link buffers
/// (intra-binding bit-identity, Bee's locked decision 1).
///
/// Mirrors the III.5 in-module reproducibility test but exercises the
/// parser+executor surface twice (once per field) so the gate is the
/// integration-test-level receipt for cross-call determinism.
#[test]
fn tdd_hal_iii_8b_c_gibbs_sample_short_run_in_process_reproducible() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");

    // Field A on lattice A.
    declare_identity_field(&mut engine, "iii_8b_c_bb_a", "U_iii_8b_c_a");
    let sample = "GIBBS_SAMPLE U_iii_8b_c_a BETA 2.5 N_SWEEPS 20 MEASURE_EVERY 0 \
                  SEED 20260616;";
    let stmt = parse(sample).expect("parse GIBBS_SAMPLE A");
    let _ = execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE A");
    let buf_a = gigi::gauge::registry::get_su2_mut("U_iii_8b_c_a")
        .expect("U_iii_8b_c_a in SU(2)-mut map")
        .lock()
        .expect("lock A")
        .buffer
        .data
        .clone();

    // Field B on a separate lattice B (declared fresh; bit-identity
    // depends on topology shape, not on the lattice's user-facing
    // name).
    declare_identity_field(&mut engine, "iii_8b_c_bb_b", "U_iii_8b_c_b");
    let sample = "GIBBS_SAMPLE U_iii_8b_c_b BETA 2.5 N_SWEEPS 20 MEASURE_EVERY 0 \
                  SEED 20260616;";
    let stmt = parse(sample).expect("parse GIBBS_SAMPLE B");
    let _ = execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE B");
    let buf_b = gigi::gauge::registry::get_su2_mut("U_iii_8b_c_b")
        .expect("U_iii_8b_c_b in SU(2)-mut map")
        .lock()
        .expect("lock B")
        .buffer
        .data
        .clone();

    assert_eq!(
        buf_a, buf_b,
        "Two GIBBS_SAMPLE calls on identical INIT IDENTITY starts at \
         fixed (β=2.5, n_sweeps=20, seed=20260616) must produce \
         byte-identical buffer.data (intra-binding bit-identity, \
         locked decision 1)"
    );
}

/// TDD-HAL-III.8b: Halcyon Gate III.D — production thermalization run
/// (`BETA 2.5 N_SWEEPS 2048 SEED 20260616 MEASURE_EVERY 1 MEASURE
/// (MEAN(PLAQUETTE), Q_SURROGATE)`) reproduces the III.8a canonical
/// reference fixture byte-for-byte.
///
/// Structural envelope:
///   - `P_history.len() == 2048`
///   - `Q_history.len() == 2048`
///   - `P_history[0] < 1.0` (one sweep moves off identity)
///   - every Q in `[0, 16]`
///   - diagnostics: `seed = 20260616`, `beta = 2.5`,
///     `n_sweeps_completed = 2048`
///
/// Load-bearing intra-GIGI bit-identity receipt:
///   - for `i in 0..2048` the f64 `P_history[i]` decodes via
///     `to_bits()` to the same `u64` the III.8a fixture stores in
///     `p_history_bits[i]`. The decimal shadow `p_history_decimal`
///     is informational only.
///
/// This is the "the GIGI side is internally deterministic and
/// reproducible at fixed seed on this OS / this code" receipt the
/// production thermalization phase depends on. Cross-binding bit-
/// identity vs Halcyon's NumPy PCG64 mock is impossible by design
/// (locked decision 1) and explicitly out of scope; the provenance
/// side-car documents that drop.
#[test]
fn tdd_hal_iii_8b_d_production_thermalization_pass_criterion() {
    let _g = gigi::gauge::registry::test_serial_lock();
    clear_registries();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    declare_identity_field(&mut engine, "iii_8b_d_bb", "U_iii_8b_d");

    // Load the III.8a canonical reference. Bit patterns are the byte-
    // equality oracle (loaded via `f64::from_bits`); the decimal
    // shadow exists in the fixture for human inspection only.
    let body = fs::read_to_string(p_canonical_path()).unwrap_or_else(|e| {
        panic!(
            "read III.8a fixture at {}: {e}. Run \
             `cargo test --features halcyon --test harvest_part_iii_canonical \
              -- --ignored --nocapture` to regenerate.",
            p_canonical_path().display()
        )
    });
    let fixture: serde_json::Value =
        serde_json::from_str(&body).expect("parse p_canonical.json");
    let p_history_bits: Vec<u64> = fixture["p_history_bits"]
        .as_array()
        .expect("p_history_bits array")
        .iter()
        .map(|b| b.as_u64().expect("p_history_bits entry not u64"))
        .collect();
    assert_eq!(
        p_history_bits.len(),
        2048,
        "fixture p_history_bits must have 2048 entries (n_sweeps); \
         re-harvest III.8a if this trips"
    );

    // Drive the full 2048-sweep production run through the parser +
    // executor path so the bit-identity contract covers the same code
    // surface a Halcyon caller would exercise.
    let sample = "GIBBS_SAMPLE U_iii_8b_d BETA 2.5 N_SWEEPS 2048 MEASURE_EVERY 1 \
                  MEASURE (MEAN(PLAQUETTE), Q_SURROGATE) SEED 20260616;";
    let stmt = parse(sample).expect("parse GIBBS_SAMPLE production");
    let rows = match execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE production") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "GIBBS_SAMPLE returns one row");
    let row = &rows[0];

    // Diagnostics block — `seed`, `beta`, `n_sweeps_completed`.
    match row.get("seed") {
        Some(Value::Integer(n)) => assert_eq!(*n, 20260616),
        other => panic!("missing/wrong seed: {other:?}"),
    }
    match row.get("beta") {
        Some(Value::Float(b)) => assert_eq!(*b, 2.5),
        other => panic!("missing/wrong beta: {other:?}"),
    }
    match row.get("n_sweeps_completed") {
        Some(Value::Integer(n)) => assert_eq!(*n, 2048),
        other => panic!("missing/wrong n_sweeps_completed: {other:?}"),
    }

    // Measurement chains.
    let p_label = gigi::gauge::ObservableId::MeanPlaquette.label();
    let q_label = gigi::gauge::ObservableId::QSurrogate.label();
    let p_history: &Vec<f64> = match row.get(p_label) {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong {p_label} chain: {other:?}"),
    };
    let q_history: &Vec<f64> = match row.get(q_label) {
        Some(Value::Vector(v)) => v,
        other => panic!("missing/wrong {q_label} chain: {other:?}"),
    };
    assert_eq!(
        p_history.len(),
        2048,
        "MeanPlaquette chain length must be n_sweeps / measure_every = 2048"
    );
    assert_eq!(
        q_history.len(),
        2048,
        "QSurrogate chain length must be n_sweeps / measure_every = 2048"
    );

    // Structural envelope: P_history[0] < 1.0 (one sweep moves off
    // identity), every Q in [0, 16] (buckyball F/2 angular bound).
    assert!(
        p_history[0] < 1.0,
        "P_history[0] must be < 1.0 after one heatbath sweep off identity, \
         got {}",
        p_history[0]
    );
    for (i, q) in q_history.iter().enumerate() {
        assert!(
            *q >= 0.0 && *q <= 16.0,
            "Q_history[{i}] out of buckyball range [0, 16]: {q}"
        );
    }

    // **Load-bearing intra-GIGI bit-identity receipt.** Every f64 in
    // the run's `P_history` must decode via `to_bits` to the same
    // `u64` the III.8a fixture stores. Strict equality, no tolerance
    // — drift here means the GIGI side has lost determinism on this
    // OS / this code, which is the failure mode this gate exists to
    // surface.
    for i in 0..2048 {
        assert_eq!(
            p_history[i].to_bits(),
            p_history_bits[i],
            "P_history[{i}] bit pattern drift vs III.8a fixture: \
             run={:#x} fixture={:#x} (intra-GIGI bit-identity at fixed \
             seed broke; re-harvest III.8a or investigate RNG / sweep \
             order drift)",
            p_history[i].to_bits(),
            p_history_bits[i]
        );
    }
}
