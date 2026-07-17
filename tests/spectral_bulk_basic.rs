//! SPECTRAL_GAUGE `MODE MAGNETIC BULK k [AROUND σ] [IN [a,b]]` — the
//! dense interior-window slice (Hallie's 2026-07-17 RH ask, Part 1).
//!
//! RED phase: `BULK` is not in the grammar yet — every test that types
//! it fails on the pre-BULK tree (the parser rejects the trailing `BULK
//! …` clause), and the opt-in refusal test fails because the current
//! `SparseUnavailable` message does not yet name the `GIGI_DENSE_CEIL`
//! opt-in or the memory cost.
//!
//! WHY BULK exists (Hallie's correction, load-bearing): the June-30
//! sparse arm did shift-invert at σ = 0 → smallest-|λ| eigenvalues =
//! bottom of the spectrum = the YM mass gap. Riemann-Hypothesis /
//! number-variance statistics live in the BULK: a contiguous window of
//! consecutive levels at the spectral CENTER, not the edge. BULK is the
//! centering + slice on the ALREADY-sorted dense FULL spectrum — no
//! re-solve.
//!
//! Contract under test (pinned this run):
//!
//! - Auto-center := the POSITIONAL MEDIAN of the ascending spectrum, the
//!   eigenvalue at index ⌊V/2⌋. NOT the midrange (λ_min+λ_max)/2 — the
//!   median tracks where levels are densest under an asymmetric DOS, and
//!   an index-based center makes the window exactly the k consecutive
//!   levels straddling the center (the object number variance needs).
//! - `BULK k` returns the k contiguous centermost eigenvalues, ascending.
//!   `BULK k == FULL then center-slice` (parity, no re-solve).
//! - `BULK k AROUND σ` returns the k eigenvalues nearest σ by value
//!   (contiguous, since the spectrum is sorted).
//! - `BULK k IN [a,b]` returns ALL eigenvalues in the CLOSED interval
//!   [a,b], with k as a safety clamp (a≤λ≤b is inherently a contiguous
//!   consecutive-level window). a>b is a typed error.
//! - k > V clamps to V; k = 0 is a typed LIMIT-bounds error.
//! - BULK requires MODE MAGNETIC this phase (the magnetic complex
//!   spectrum is the RH object); the executor rejects BULK without it.
//! - BULK and FULL are mutually exclusive (parse error).
//! - Envelope: the BULK row carries `eigenvalues` (the window), plus
//!   `bulk` (tag), `bulk_center`, `bulk_center_index`, `bulk_lo`,
//!   `bulk_hi`, and `mode_used = "dense"`, on top of the Phase-1 fields.
//! - Opt-in 8192: the dense ceiling stays 4096 by default; a graph in
//!   (4096, 8192] is REFUSED unless `GIGI_DENSE_CEIL` opts in, and the
//!   refusal names the memory cost + the env knob (this test only pins
//!   the refusal message — the raised-ceiling resolution is unit-tested
//!   white-box in src/spectral.rs, no expensive V≈8000 solve here).
//!
//! Run with:
//!   `cargo test --features halcyon --test spectral_bulk_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── Fixture helpers (mirror tests/spectral_magnetic_basic.rs) ────────

fn make_theta_bundle(engine: &mut Engine, name: &str) {
    let schema = BundleSchema::new(name)
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("theta"));
    engine.create_bundle(schema).expect("create_bundle");
}

fn theta_edge(va: i64, vb: i64, theta: f64) -> Record {
    let mut rec = Record::new();
    rec.insert("vertex_a".to_string(), Value::Integer(va));
    rec.insert("vertex_b".to_string(), Value::Integer(vb));
    rec.insert("theta".to_string(), Value::Float(theta));
    rec
}

/// Cycle C_n with uniform per-edge flux `phi` (φ=0 → unit-weight
/// combinatorial Laplacian through the magnetic path).
fn insert_cycle(engine: &mut Engine, name: &str, n: usize, phi: f64) {
    let batch: Vec<Record> = (0..n)
        .map(|i| theta_edge(i as i64, ((i + 1) % n) as i64, phi))
        .collect();
    engine.batch_insert(name, &batch).expect("batch_insert");
}

/// Star K_{1,m}: centre vertex 0 wired to leaves 1..=m. θ = 0, so the
/// magnetic Laplacian is the plain combinatorial Laplacian with the
/// closed-form ASYMMETRIC spectrum {0, 1^(m-1), (m+1)}.
fn insert_star(engine: &mut Engine, name: &str, m: usize) {
    let batch: Vec<Record> = (1..=m)
        .map(|leaf| theta_edge(0, leaf as i64, 0.0))
        .collect();
    engine.batch_insert(name, &batch).expect("batch_insert");
}

/// Run a statement and return the single summary Record (the envelope).
fn run_row(engine: &mut Engine, gql: &str) -> Record {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse `{gql}` failed: {e}"));
    let result =
        execute(engine, &stmt).unwrap_or_else(|e| panic!("execute `{gql}` failed: {e}"));
    match result {
        ExecResult::Rows(mut rows) => {
            assert_eq!(rows.len(), 1, "single summary row expected for `{gql}`");
            rows.remove(0)
        }
        other => panic!("expected Rows for `{gql}`, got {other:?}"),
    }
}

/// Pull the `eigenvalues` Vector off a summary row.
fn eigs(row: &Record) -> Vec<f64> {
    match row.get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector, got {other:?}"),
    }
}

fn float_field(row: &Record, key: &str) -> f64 {
    match row.get(key) {
        Some(Value::Float(f)) => *f,
        other => panic!("expected Float `{key}`, got {other:?}"),
    }
}

fn int_field(row: &Record, key: &str) -> i64 {
    match row.get(key) {
        Some(Value::Integer(n)) => *n,
        other => panic!("expected Integer `{key}`, got {other:?}"),
    }
}

/// Full ascending spectrum via `FULL` on the same bundle (the reference
/// the BULK window is sliced from).
fn full_spectrum(engine: &mut Engine, bundle: &str) -> Vec<f64> {
    let row = run_row(
        engine,
        &format!("SPECTRAL_GAUGE {bundle} ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL;"),
    );
    eigs(&row)
}

/// The center window `[lo, hi)` the PINNED positional-median definition
/// selects: c = ⌊V/2⌋, half = ⌊k/2⌋, lo = clamp(c−half), hi = lo+k.
fn expected_window(v: usize, k: usize) -> (usize, usize, usize) {
    let k_eff = k.min(v);
    let c = v / 2;
    let half = k_eff / 2;
    let lo = c.saturating_sub(half).min(v - k_eff);
    (lo, lo + k_eff, c)
}

/// Closed-form combinatorial-Laplacian spectrum of C_n (θ=0), ascending.
fn cycle_spectrum(n: usize) -> Vec<f64> {
    let mut vals: Vec<f64> = (0..n)
        .map(|k| 2.0 - 2.0 * (2.0 * std::f64::consts::PI * k as f64 / n as f64).cos())
        .collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    vals
}

// ═══ Grammar ════════════════════════════════════════════════════════

/// (P1) `BULK k` (auto-center) parses.
#[test]
fn test_parse_bulk_auto_center() {
    parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8;")
        .expect("BULK k grammar must parse");
}

/// (P2) `BULK k AROUND σ` parses (σ may be a plain or signed decimal).
#[test]
fn test_parse_bulk_around_sigma() {
    parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 AROUND 2.5;")
        .expect("BULK k AROUND σ grammar must parse");
}

/// (P3) `BULK k IN [a,b]` parses.
#[test]
fn test_parse_bulk_in_interval() {
    parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 IN [1.0, 3.0];")
        .expect("BULK k IN [a,b] grammar must parse");
}

/// (P4) BULK and FULL are mutually exclusive — both orders reject.
#[test]
fn test_parse_bulk_and_full_mutually_exclusive() {
    parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 FULL;")
        .expect_err("BULK then FULL must be rejected");
    parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL BULK 8;")
        .expect_err("FULL then BULK must be rejected");
}

// ═══ Anchor (a) — k centermost, contiguous, closed-form slice ═══════

/// (A) C_8 (θ=0): BULK 4 returns exactly the 4 centermost eigenvalues
/// [2−√2, 2, 2, 2+√2], contiguous, matching the hand-sorted slice, with
/// the reported window and center pinned to the positional median.
#[test]
fn test_bulk_centermost_matches_closed_form_cycle_c8() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8");
    insert_cycle(&mut engine, "c8", 8, 0.0);

    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8 ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 4;",
    );
    let window = eigs(&row);
    let full = cycle_spectrum(8); // [0, .586, .586, 2, 2, 3.414, 3.414, 4]
    let (lo, hi, c) = expected_window(8, 4); // (2, 6, 4)

    assert_eq!(window.len(), 4, "BULK 4 returns 4 eigenvalues");
    for (i, (got, want)) in window.iter().zip(full[lo..hi].iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "centermost eigenvalue {i}: got {got}, want {want}"
        );
    }
    // Contiguity: strictly the middle slice, ascending, no gaps.
    for w in window.windows(2) {
        assert!(w[0] <= w[1] + 1e-12, "BULK window must ascend: {w:?}");
    }
    assert_eq!(int_field(&row, "bulk_lo"), lo as i64);
    assert_eq!(int_field(&row, "bulk_hi"), hi as i64);
    assert_eq!(int_field(&row, "bulk_center_index"), c as i64);
    assert!(
        (float_field(&row, "bulk_center") - full[c]).abs() < 1e-9,
        "center value must be the positional-median eigenvalue vals[V/2]"
    );
}

// ═══ Anchor (b) — parity: BULK k == FULL then center-slice ══════════

/// (B) On a genuinely COMPLEX magnetic spectrum (uniform-flux cycle
/// C_10, φ=0.3), BULK k is byte-for-byte the center slice of the FULL
/// spectrum — proving BULK re-centers the already-sorted spectrum
/// rather than re-solving anything.
#[test]
fn test_bulk_equals_full_then_center_slice() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c10flux");
    insert_cycle(&mut engine, "c10flux", 10, 0.3);

    let full = full_spectrum(&mut engine, "c10flux");
    assert_eq!(full.len(), 10);
    let (lo, hi, c) = expected_window(10, 4); // (3, 7, 5)

    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE c10flux ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 4;",
    );
    let window = eigs(&row);
    assert_eq!(window.len(), 4);
    for (i, (got, want)) in window.iter().zip(full[lo..hi].iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-12,
            "BULK must equal FULL center-slice exactly at index {i}: {got} vs {want}"
        );
    }
    assert_eq!(int_field(&row, "bulk_lo"), lo as i64);
    assert_eq!(int_field(&row, "bulk_hi"), hi as i64);
    assert_eq!(int_field(&row, "bulk_center_index"), c as i64);
    assert!((float_field(&row, "bulk_center") - full[c]).abs() < 1e-12);
}

// ═══ Anchor (c) — AROUND σ picks the k nearest σ ════════════════════

/// (C) C_8: `BULK 3 AROUND 3.5` returns the 3 eigenvalues nearest 3.5,
/// contiguous, matching an independent nearest-k computation.
#[test]
fn test_bulk_around_sigma_picks_k_nearest() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8around");
    insert_cycle(&mut engine, "c8around", 8, 0.0);

    let full = cycle_spectrum(8);
    let sigma = 3.5_f64;

    // Independent "k nearest σ" reference (stable — no ties at k=3 here).
    let mut by_dist: Vec<f64> = full.clone();
    by_dist.sort_by(|a, b| {
        (a - sigma)
            .abs()
            .partial_cmp(&(b - sigma).abs())
            .unwrap()
    });
    let mut nearest3 = by_dist[..3].to_vec();
    nearest3.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8around ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 3 AROUND 3.5;",
    );
    let window = eigs(&row);
    assert_eq!(window.len(), 3);
    for (i, (got, want)) in window.iter().zip(nearest3.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "AROUND 3.5 nearest-{i}: got {got}, want {want}"
        );
    }
    // The center used is σ itself.
    assert!((float_field(&row, "bulk_center") - sigma).abs() < 1e-9);
    // Contiguity: window is a slice of the sorted full spectrum.
    let lo = int_field(&row, "bulk_lo") as usize;
    let hi = int_field(&row, "bulk_hi") as usize;
    assert_eq!(hi - lo, 3);
    for (got, want) in window.iter().zip(full[lo..hi].iter()) {
        assert!((got - want).abs() < 1e-9, "AROUND window must be contiguous");
    }
}

// ═══ Anchor (d) — IN [a,b] semantics (all in the closed interval) ═══

/// (D1) C_8: `BULK k IN [a,b]` returns ALL eigenvalues in the closed
/// interval, contiguous. Three bracketings pin inclusion + exclusion.
#[test]
fn test_bulk_in_interval_returns_all_in_closed_interval() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8in");
    insert_cycle(&mut engine, "c8in", 8, 0.0);
    // full = [0, .586, .586, 2, 2, 3.414, 3.414, 4]

    // [1.5, 2.5] straddles only the two 2's.
    let w1 = eigs(&run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8in ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 IN [1.5, 2.5];",
    ));
    assert_eq!(w1.len(), 2, "[1.5,2.5] contains exactly the two 2's");
    for got in &w1 {
        assert!((got - 2.0).abs() < 1e-9, "got {got}, want 2.0");
    }

    // [0.5, 3.5] contains the two .586's, two 2's, two 3.414's = 6.
    let w2 = eigs(&run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8in ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 IN [0.5, 3.5];",
    ));
    assert_eq!(w2.len(), 6, "[0.5,3.5] contains 6 interior eigenvalues");
    for got in &w2 {
        assert!(*got >= 0.5 - 1e-9 && *got <= 3.5 + 1e-9, "{got} out of [0.5,3.5]");
    }

    // [3.9, 4.1] contains only the top eigenvalue 4.
    let w3 = eigs(&run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8in ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 IN [3.9, 4.1];",
    ));
    assert_eq!(w3.len(), 1);
    assert!((w3[0] - 4.0).abs() < 1e-9);
}

/// (D2) `IN [a,b]` with a > b is a typed error.
#[test]
fn test_bulk_in_interval_reversed_bounds_errors() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8rev");
    insert_cycle(&mut engine, "c8rev", 8, 0.0);

    let stmt = parse(
        "SPECTRAL_GAUGE c8rev ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 8 IN [3.0, 1.0];",
    )
    .expect("grammar accepts [a,b]; the executor rejects a>b");
    let err = execute(&mut engine, &stmt).expect_err("a>b must be a typed error");
    assert!(
        err.contains("interval") || err.contains("IN"),
        "reversed-interval error must name the interval: {err}"
    );
}

/// (D3) `IN [a,b]` k-clamp: when the interval holds more than k levels,
/// the safety clamp returns exactly k, all inside [a,b], contiguous.
#[test]
fn test_bulk_in_interval_k_clamp_invariants() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8clamp");
    insert_cycle(&mut engine, "c8clamp", 8, 0.0);

    // The whole spectrum lives in [-1, 10]; clamp to k=3.
    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8clamp ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 3 IN [-1.0, 10.0];",
    );
    let window = eigs(&row);
    assert_eq!(window.len(), 3, "k=3 clamps an over-full interval to 3");
    for got in &window {
        assert!(*got >= -1.0 && *got <= 10.0, "clamped window must stay in [a,b]");
    }
    let full = cycle_spectrum(8);
    let lo = int_field(&row, "bulk_lo") as usize;
    let hi = int_field(&row, "bulk_hi") as usize;
    assert_eq!(hi - lo, 3);
    for (got, want) in window.iter().zip(full[lo..hi].iter()) {
        assert!((got - want).abs() < 1e-9, "clamped window must be a contiguous slice");
    }
}

// ═══ Anchor (e) — k > V clamps; k = 0 errors ════════════════════════

/// (E1) BULK 100 on V=8 clamps to the whole spectrum.
#[test]
fn test_bulk_k_exceeds_v_clamps() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8big");
    insert_cycle(&mut engine, "c8big", 8, 0.0);

    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8big ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 100;",
    );
    let window = eigs(&row);
    assert_eq!(window.len(), 8, "k>V clamps to V");
    assert_eq!(int_field(&row, "bulk_lo"), 0);
    assert_eq!(int_field(&row, "bulk_hi"), 8);
}

/// (E2) BULK 0 is a typed LIMIT-bounds error.
#[test]
fn test_bulk_k_zero_errors() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8zero");
    insert_cycle(&mut engine, "c8zero", 8, 0.0);

    let stmt = parse("SPECTRAL_GAUGE c8zero ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 0;")
        .expect("grammar accepts BULK 0; the executor rejects k=0");
    let err = execute(&mut engine, &stmt).expect_err("BULK 0 must be rejected");
    assert!(err.contains("LIMIT") || err.contains("≥ 1"), "k=0 error must name the bound: {err}");
}

// ═══ Anchor (f) — center = median, NOT midrange (asymmetric DOS) ════

/// (F) Star K_{1,5} has the asymmetric spectrum {0,1,1,1,1,6}: the
/// positional median (vals[3] = 1) is far from the midrange
/// ((0+6)/2 = 3). BULK must center on the MEDIAN — this is the whole
/// reason "bulk" means "densest middle by count", not "arithmetic
/// middle of the range".
#[test]
fn test_bulk_center_is_median_not_midrange_asymmetric_dos() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "star5");
    insert_star(&mut engine, "star5", 5); // V=6

    let full = full_spectrum(&mut engine, "star5");
    assert_eq!(full.len(), 6);
    // Confirm the asymmetric closed form {0,1,1,1,1,6}.
    let expected = [0.0, 1.0, 1.0, 1.0, 1.0, 6.0];
    for (got, want) in full.iter().zip(expected.iter()) {
        assert!((got - want).abs() < 1e-9, "star spectrum {got} vs {want}");
    }
    let median = full[6 / 2]; // vals[3] = 1.0
    let midrange = (full[0] + full[5]) / 2.0; // 3.0
    assert!((median - 1.0).abs() < 1e-9 && (midrange - 3.0).abs() < 1e-9);
    assert!((median - midrange).abs() > 1.0, "fixture must be genuinely asymmetric");

    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE star5 ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 2;",
    );
    let center = float_field(&row, "bulk_center");
    assert!(
        (center - median).abs() < 1e-9,
        "BULK center must be the positional median {median}, got {center}"
    );
    assert!(
        (center - midrange).abs() > 1.0,
        "BULK center must NOT be the midrange {midrange}, got {center}"
    );
    assert_eq!(int_field(&row, "bulk_center_index"), 3);
}

// ═══ Magnetic prerequisite ══════════════════════════════════════════

/// (M) BULK without MODE MAGNETIC parses but is rejected at execute:
/// Part-1 BULK operates on the magnetic complex spectrum only.
#[test]
fn test_bulk_requires_magnetic() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8nomag");
    insert_cycle(&mut engine, "c8nomag", 8, 0.0);

    let stmt = parse("SPECTRAL_GAUGE c8nomag ON FIBER (theta) GROUP U(1) BULK 4;")
        .expect("grammar accepts BULK without MODE; the executor rejects");
    let err = execute(&mut engine, &stmt).expect_err("BULK without MAGNETIC must error");
    assert!(
        err.contains("MAGNETIC") && err.contains("BULK"),
        "error must name BULK + MAGNETIC: {err}"
    );
}

// ═══ Envelope ═══════════════════════════════════════════════════════

/// (V) The BULK row carries the window + center + [lo,hi) + bulk tag +
/// mode_used, on top of the Phase-1 {gap, n_records_used, group_used}.
#[test]
fn test_bulk_envelope_carries_center_window_and_tag() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c8env");
    insert_cycle(&mut engine, "c8env", 8, 0.0);

    let row = run_row(
        &mut engine,
        "SPECTRAL_GAUGE c8env ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 4;",
    );
    // Phase-1 fields survive.
    assert!(row.contains_key("gap"));
    assert!(row.contains_key("n_records_used"));
    assert!(row.contains_key("group_used"));
    // Window + mode.
    assert_eq!(eigs(&row).len(), 4);
    match row.get("mode_used") {
        Some(Value::Text(m)) => assert_eq!(m, "dense"),
        other => panic!("expected mode_used=dense, got {other:?}"),
    }
    // Bulk metadata.
    match row.get("bulk") {
        Some(Value::Bool(b)) => assert!(*b, "bulk tag must be true"),
        other => panic!("expected bulk:true, got {other:?}"),
    }
    let (lo, hi, c) = expected_window(8, 4);
    assert_eq!(int_field(&row, "bulk_lo"), lo as i64);
    assert_eq!(int_field(&row, "bulk_hi"), hi as i64);
    assert_eq!(int_field(&row, "bulk_center_index"), c as i64);
    // center is a Float and equals vals[c].
    assert!((float_field(&row, "bulk_center") - 2.0).abs() < 1e-9);
}

// ═══ Opt-in 8192 — refused by default, names the cost + env knob ════

/// (O) The dense ceiling stays 4096 by default; a graph in (4096, 8192]
/// is REFUSED unless `GIGI_DENSE_CEIL` opts in, and the refusal names
/// both the env knob and the memory cost (so an operator knows how to
/// raise the ceiling AND why it is off by default). Opting in to 8192
/// then UNBLOCKS L=20 (V=8000 < 8192) — proved via the public ceiling
/// resolver without an expensive V≈8000 solve. GIGI_DENSE_CEIL clamps to
/// the safe band [4096, 8192] (raises only, never lowers the floor,
/// never past 8192). All env mutation is serial in this ONE test so no
/// sibling test in this binary races on the var.
#[test]
fn test_optin_8192_ceiling_refusal_and_allow() {
    use gigi::spectral::{dense_ceiling, dense_full_allowed};

    // Start from a clean environment (no ambient opt-in).
    std::env::remove_var("GIGI_DENSE_CEIL");

    // ── Refused by default: the gate fires before assembly (cheap), and
    //   the message names 4096, Phase 2.1, the env knob, the memory cost,
    //   and the actual vertex count.
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "big_path");
    let batch: Vec<Record> = (0..4200)
        .map(|i| theta_edge(i as i64, i as i64 + 1, 0.0))
        .collect();
    engine.batch_insert("big_path", &batch).expect("batch_insert");
    let stmt = parse(
        "SPECTRAL_GAUGE big_path ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 4;",
    )
    .expect("FULL grammar parses");
    let err = execute(&mut engine, &stmt)
        .expect_err("V=4201 above the default 4096 ceiling must be refused");
    assert!(err.contains("4096"), "refusal must name the 4096 boundary: {err}");
    assert!(err.contains("Phase 2.1"), "refusal must name the sparse deferral: {err}");
    assert!(
        err.contains("GIGI_DENSE_CEIL"),
        "refusal must name the opt-in env knob: {err}"
    );
    assert!(
        err.contains("GB") || err.contains("memory"),
        "refusal must name the memory cost: {err}"
    );
    assert!(err.contains("4201"), "refusal must name the actual vertex count: {err}");

    // Default ceiling resolves to 4096.
    assert_eq!(dense_ceiling(), 4096, "default dense ceiling is 4096");
    assert!(dense_full_allowed(4096).is_ok(), "V=4096 is at the default ceiling");
    assert!(dense_full_allowed(4097).is_err(), "V=4097 refused by default");

    // ── Opt in to 8192 — this is the L=20 unblock (V=8000 < 8192).
    std::env::set_var("GIGI_DENSE_CEIL", "8192");
    assert_eq!(dense_ceiling(), 8192, "GIGI_DENSE_CEIL=8192 raises the ceiling");
    assert!(
        dense_full_allowed(8000).is_ok(),
        "opt-in must UNBLOCK L=20 (V=8000)"
    );
    assert!(dense_full_allowed(8192).is_ok(), "V=8192 is at the opt-in ceiling");
    assert!(dense_full_allowed(8193).is_err(), "V=8193 exceeds the 8192 opt-in max");

    // A mid-band opt-in is honored verbatim.
    std::env::set_var("GIGI_DENSE_CEIL", "6000");
    assert_eq!(dense_ceiling(), 6000);
    assert!(dense_full_allowed(6000).is_ok());
    assert!(dense_full_allowed(6001).is_err());

    // Clamps: below the floor snaps up to 4096; above the opt-in max
    // snaps down to 8192; unparseable falls back to the 4096 default.
    std::env::set_var("GIGI_DENSE_CEIL", "100");
    assert_eq!(dense_ceiling(), 4096, "opt-in can only RAISE, never lower the floor");
    std::env::set_var("GIGI_DENSE_CEIL", "999999");
    assert_eq!(dense_ceiling(), 8192, "opt-in can never exceed 8192");
    std::env::set_var("GIGI_DENSE_CEIL", "not-a-number");
    assert_eq!(dense_ceiling(), 4096, "unparseable falls back to the default");

    std::env::remove_var("GIGI_DENSE_CEIL");
}
