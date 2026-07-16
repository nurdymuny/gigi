//! SPECTRAL_GAUGE `MODE MAGNETIC` — U(1) Hermitian magnetic Laplacian
//! (Concept B, 2026-07-16).
//!
//! RED phase: `MODE MAGNETIC` is not in the grammar yet — every test
//! that types it fails with a parse error on the pre-Concept-B tree.
//!
//! Contract under test (Hallie's confirmed 2026-07-16 ask; extends
//! SPECTRAL_GAUGE_PHASE2_SPEC.md, which predates MAGNETIC):
//!
//! - For each undirected edge {u, v} whose record is (vertex_a = u,
//!   vertex_b = v, theta = θ), the ingested θ applies to the u → v
//!   direction:
//!       L[u][v] = −e^{+iθ},   L[v][u] = −e^{−iθ}   (Hermitian pair)
//!       L[u][u] += 1, L[v][v] += 1                 (unit edge weights)
//!   Eigenvalues of the Hermitian L are REAL — wire stays Vec<f64>.
//! - U(1) only this phase. Any other group errors with:
//!   "MODE MAGNETIC requires GROUP U(1) in this phase (matrix-valued
//!    magnetic Laplacians are a later phase)".
//! - Composes with FULL [LIMIT k] and WHERE.
//! - Without FULL, the wire envelope stays the λ₁-only 3-field shape.
//!
//! Closed-form anchor (exact to 1e-9): triangle C_3 with uniform flux
//! φ per edge (total flux Φ = 3φ) has magnetic-Laplacian eigenvalues
//!   2 − 2cos((Φ + 2πk)/3),  k = 0, 1, 2
//! (circulant symbol 2 − e^{iφ}ω − e^{−iφ}ω̄, ω = e^{2πik/3}).
//!
//! Symmetry-class acceptance (the scientific gate): on a fixed-seed
//! Erdős–Rényi graph (V = 512, mean degree ≈ 16) with i.i.d.
//! θ ~ U[0, 2π), the mean consecutive-spacing ratio
//! r̃ = mean(min(s_i, s_{i+1})/max(s_i, s_{i+1})) over the bulk of the
//! sorted spectrum must land within ±0.03 of the published ensemble
//! values (Atas, Bogomolny, Giraud, Roux, PRL 110, 084101, 2013):
//!   GOE ≈ 0.5307 (cos-weight real-symmetric mode, same graph)
//!   GUE ≈ 0.5996 (MODE MAGNETIC Hermitian mode)
//! Poisson ≈ 0.3863 is the failure smell for accidental localization.
//!
//! Run with:
//!   `cargo test --features halcyon --test spectral_magnetic_basic`
//! (add `-- --nocapture` to see the measured r̃ values)

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

// ── Fixture helpers ─────────────────────────────────────────────────

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

/// Cyclically-oriented triangle 0→1→2→0 with uniform phase `phi`.
fn insert_flux_triangle(engine: &mut Engine, name: &str, phi: f64) {
    let batch = vec![
        theta_edge(0, 1, phi),
        theta_edge(1, 2, phi),
        theta_edge(2, 0, phi),
    ];
    engine.batch_insert(name, &batch).expect("batch_insert");
}

/// Run a GQL statement and pull the eigenvalues Vector off the single
/// summary row.
fn eigenvalues_of(engine: &mut Engine, gql: &str) -> Vec<f64> {
    let stmt = parse(gql).unwrap_or_else(|e| panic!("parse `{gql}` failed: {e}"));
    let result =
        execute(engine, &stmt).unwrap_or_else(|e| panic!("execute `{gql}` failed: {e}"));
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows for `{gql}`, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "single summary row expected");
    match rows[0].get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector for `{gql}`, got {other:?}"),
    }
}

/// Sorted closed-form magnetic spectrum of the uniform-flux triangle.
fn triangle_flux_spectrum(phi: f64) -> Vec<f64> {
    let total = 3.0 * phi;
    let mut vals: Vec<f64> = (0..3)
        .map(|k| 2.0 - 2.0 * ((total + 2.0 * std::f64::consts::PI * k as f64) / 3.0).cos())
        .collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    vals
}

// ── Grammar ─────────────────────────────────────────────────────────

/// (B1) MODE MAGNETIC composes with GROUP + FULL LIMIT — the exact
/// statement shape of live probe S3.
#[test]
fn test_parse_mode_magnetic_composes_with_full_limit() {
    parse("SPECTRAL_GAUGE rh_flux ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 8;")
        .expect("MODE MAGNETIC FULL LIMIT grammar must parse");
}

/// (B2) MODE anything-else is rejected with a message naming MAGNETIC
/// (solver selection dense/sparse stays internal this phase — R1).
#[test]
fn test_parse_mode_other_than_magnetic_rejected() {
    let err = parse("SPECTRAL_GAUGE b ON FIBER (theta) GROUP U(1) MODE SPARSE FULL;")
        .expect_err("MODE SPARSE must be rejected");
    assert!(
        err.contains("MAGNETIC"),
        "MODE error must name MAGNETIC as the only mode: {err}"
    );
}

// ── Closed-form anchors ─────────────────────────────────────────────

/// (B3) Triangle with uniform flux φ = 0.7 per edge: magnetic
/// eigenvalues match 2 − 2cos((Φ + 2πk)/3) to 1e-9.
#[test]
fn test_triangle_uniform_flux_closed_form() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c3flux");
    let phi = 0.7_f64;
    insert_flux_triangle(&mut engine, "c3flux", phi);

    let vals = eigenvalues_of(
        &mut engine,
        "SPECTRAL_GAUGE c3flux ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL;",
    );
    assert_eq!(vals.len(), 3);
    let expected = triangle_flux_spectrum(phi);
    for (i, (got, want)) in vals.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "triangle flux eigenvalue {i}: got {got}, want {want} (phi = {phi})"
        );
    }
}

/// (B4) Zero flux: MAGNETIC and the default cos-weight mode agree on
/// the same θ=0 cycle (both reduce to the unit-weight combinatorial
/// Laplacian of C_6: 2 − 2cos(2πk/6)).
#[test]
fn test_magnetic_at_zero_flux_matches_cos_weight_mode() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c6zero");
    let batch: Vec<Record> = (0..6)
        .map(|i| theta_edge(i as i64, ((i + 1) % 6) as i64, 0.0))
        .collect();
    engine.batch_insert("c6zero", &batch).expect("batch_insert");

    let magnetic = eigenvalues_of(
        &mut engine,
        "SPECTRAL_GAUGE c6zero ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL;",
    );
    let cos_mode = eigenvalues_of(
        &mut engine,
        "SPECTRAL_GAUGE c6zero ON FIBER (theta) GROUP U(1) FULL;",
    );
    assert_eq!(magnetic.len(), 6);
    assert_eq!(cos_mode.len(), 6);
    for (i, (m, c)) in magnetic.iter().zip(cos_mode.iter()).enumerate() {
        assert!(
            (m - c).abs() < 1e-9,
            "θ=0 spectra must agree: eigenvalue {i} magnetic {m} vs cos {c}"
        );
    }
}

/// (B5) Tree gauge invariance: on a path graph (no cycles) every flux
/// assignment is gauge-equivalent to zero, so the magnetic spectrum
/// equals the unit-weight Laplacian of P_3: {0, 1, 3}. This pins both
/// the Hermitian assembly (real spectrum, conjugate pair off-diagonals)
/// and the unit-weight diagonal (|e^{iθ}| = 1, NOT cos θ).
#[test]
fn test_path_graph_flux_is_gauge_trivial() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "p3flux");
    let batch = vec![theta_edge(0, 1, 1.1), theta_edge(1, 2, 2.3)];
    engine.batch_insert("p3flux", &batch).expect("batch_insert");

    let vals = eigenvalues_of(
        &mut engine,
        "SPECTRAL_GAUGE p3flux ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL;",
    );
    let expected = [0.0, 1.0, 3.0];
    assert_eq!(vals.len(), 3);
    for (i, (got, want)) in vals.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "P_3 magnetic eigenvalue {i}: got {got}, want {want} — \
             unit-weight Hermitian assembly is broken (cos-weight leak?)"
        );
    }
}

// ── Error surface ───────────────────────────────────────────────────

/// (B6) Non-U(1) groups reject MODE MAGNETIC with the this-phase error.
#[test]
fn test_magnetic_requires_u1_error() {
    let mut engine = Engine::open_memory().expect("memory engine");
    let mut schema = BundleSchema::new("su2b")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"));
    for f in ["q0", "q1", "q2", "q3"] {
        schema = schema.fiber(FieldDef::numeric(f));
    }
    engine.create_bundle(schema).expect("create_bundle");
    let mut rec = Record::new();
    rec.insert("vertex_a".to_string(), Value::Integer(0));
    rec.insert("vertex_b".to_string(), Value::Integer(1));
    for f in ["q0", "q1", "q2", "q3"] {
        rec.insert(f.to_string(), Value::Float(if f == "q0" { 1.0 } else { 0.0 }));
    }
    engine.insert("su2b", &rec).expect("insert");

    let stmt = parse(
        "SPECTRAL_GAUGE su2b ON FIBER (q0, q1, q2, q3) GROUP SU(2) MODE MAGNETIC FULL;",
    )
    .expect("grammar accepts MODE MAGNETIC with any group; the executor rejects");
    let err = execute(&mut engine, &stmt)
        .expect_err("SU(2) + MODE MAGNETIC must be a typed error");
    assert!(
        err.contains(
            "MODE MAGNETIC requires GROUP U(1) in this phase \
             (matrix-valued magnetic Laplacians are a later phase)"
        ),
        "error must carry the exact this-phase message, got: {err}"
    );
}

// ── Envelope compatibility ──────────────────────────────────────────

/// (B7) MODE MAGNETIC without FULL keeps the λ₁-only 3-field envelope
/// (magnetic changes the operator, not the Phase-1 wire shape).
#[test]
fn test_magnetic_without_full_keeps_lambda1_envelope() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_theta_bundle(&mut engine, "c3gap");
    insert_flux_triangle(&mut engine, "c3gap", 0.4);

    let stmt = parse("SPECTRAL_GAUGE c3gap ON FIBER (theta) GROUP U(1) MODE MAGNETIC;")
        .expect("MODE MAGNETIC without FULL must parse");
    let result = execute(&mut engine, &stmt).expect("magnetic λ₁ path must run");
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(
        row.len(),
        3,
        "λ₁-only envelope must stay exactly 3 fields, got keys {:?}",
        row.keys().collect::<Vec<_>>()
    );
    // gap = smallest eigenvalue above tol of the flux triangle —
    // strictly positive because flux lifts the zero mode.
    match row.get("gap") {
        Some(Value::Float(g)) => {
            let expected = triangle_flux_spectrum(0.4)[0];
            assert!(
                (g - expected).abs() < 1e-9,
                "magnetic gap: got {g}, want {expected}"
            );
        }
        other => panic!("expected gap Float, got {other:?}"),
    }
}

// ── WHERE composition ───────────────────────────────────────────────

/// (B8) MODE MAGNETIC composes with WHERE sector filtering: two
/// disjoint flux triangles labelled by a sector column; filtering to
/// sector 0 reproduces the sector-0 closed form on 3 eigenvalues.
#[test]
fn test_magnetic_composes_with_where_sector_filter() {
    let mut engine = Engine::open_memory().expect("memory engine");
    let schema = BundleSchema::new("sectors")
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .base(FieldDef::numeric("sector"))
        .fiber(FieldDef::numeric("theta"));
    engine.create_bundle(schema).expect("create_bundle");

    let mut insert_sector = |va: i64, vb: i64, sector: i64, theta: f64| {
        let mut rec = Record::new();
        rec.insert("vertex_a".to_string(), Value::Integer(va));
        rec.insert("vertex_b".to_string(), Value::Integer(vb));
        rec.insert("sector".to_string(), Value::Integer(sector));
        rec.insert("theta".to_string(), Value::Float(theta));
        engine.insert("sectors", &rec).expect("insert");
    };
    // Sector 0: flux triangle phi = 0.9 on vertices 0-2.
    insert_sector(0, 1, 0, 0.9);
    insert_sector(1, 2, 0, 0.9);
    insert_sector(2, 0, 0, 0.9);
    // Sector 1: different triangle on vertices 100-102, junk flux.
    insert_sector(100, 101, 1, 2.2);
    insert_sector(101, 102, 1, 2.2);
    insert_sector(102, 100, 1, 2.2);

    let vals = eigenvalues_of(
        &mut engine,
        "SPECTRAL_GAUGE sectors WHERE sector = 0 ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL;",
    );
    assert_eq!(vals.len(), 3, "sector filter must reduce to one triangle");
    let expected = triangle_flux_spectrum(0.9);
    for (i, (got, want)) in vals.iter().zip(expected.iter()).enumerate() {
        assert!(
            (got - want).abs() < 1e-9,
            "sector-0 magnetic eigenvalue {i}: got {got}, want {want}"
        );
    }
}

// ── Symmetry-class acceptance (the scientific gate) ─────────────────

/// In-test deterministic RNG — xorshift64* (the house algorithm), kept
/// local so the fixture is self-contained and seed-stable forever.
struct TestRng(u64);
impl TestRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn uniform(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

/// Mean consecutive-spacing ratio r̃ over the bulk of a sorted
/// spectrum. `trim_frac` of the eigenvalues is dropped at EACH edge
/// before gaps are formed (spectrum-edge effects are not universal).
fn mean_spacing_ratio(sorted_vals: &[f64], trim_frac: f64) -> f64 {
    let n = sorted_vals.len();
    let lo = (n as f64 * trim_frac).floor() as usize;
    let hi = n - lo;
    let bulk = &sorted_vals[lo..hi];
    let gaps: Vec<f64> = bulk.windows(2).map(|w| w[1] - w[0]).collect();
    let mut ratios = Vec::with_capacity(gaps.len().saturating_sub(1));
    for w in gaps.windows(2) {
        let (a, b) = (w[0], w[1]);
        if a > 1e-12 && b > 1e-12 {
            ratios.push(a.min(b) / a.max(b));
        }
    }
    assert!(
        ratios.len() > 100,
        "need bulk statistics, got only {} ratios",
        ratios.len()
    );
    ratios.iter().sum::<f64>() / ratios.len() as f64
}

/// (B9) THE GATE — fixed-seed random-flux U(1) Erdős–Rényi graphs
/// (V = 512, mean degree 16, seeds 20260716/1/2/3): under MODE
/// MAGNETIC the bulk r̃ averaged over the 4 graphs must land within
/// ±0.03 of GUE 0.5996; the SAME 4 graphs under the default
/// cos-weight mode must average within ±0.03 of GOE 0.5307.
/// Published anchors: Atas et al., PRL 110, 084101 (2013).
///
/// ESTIMATOR NOTE (2026-07-16 investigation receipt): the gate is the
/// MEAN over 4 fixed-seed graphs because the single-graph r̃ estimator
/// at V = 512 has seed-to-seed scatter σ ≈ 0.02 (shipped-fixture
/// cos-weight values across seeds 20260716/1/2/3/4: 0.5567, 0.5165,
/// 0.5151, 0.5207, 0.5465 — 5-seed mean 0.5311, σ = 0.0192, i.e. ON
/// the GOE anchor; the earlier-quoted set with mean 0.5335 came from
/// an intermediate fixture during the investigation). A
/// single-seed ±0.03 window is a ~1.5σ criterion that fails ~13% of
/// seeds under CORRECT physics. Averaging 4 fixed seeds shrinks the
/// estimator σ to ≈ 0.01, making ±0.03 a ≈3σ gate. This is variance
/// reduction on the estimator — the anchors and the ±0.03 tolerance
/// are unchanged. Do NOT widen the tolerance; if this gate fails,
/// investigate (Hermiticity, symmetry breaking, spectrum edges).
#[test]
fn test_goe_vs_gue_spacing_ratio_gate() {
    let v = 512usize;
    let p = 16.0 / (v as f64 - 1.0);
    let seeds: [u64; 4] = [20260716, 1, 2, 3];
    let trim = 0.10;

    let mut goe_rs = Vec::new();
    let mut gue_rs = Vec::new();
    for seed in seeds {
        let mut engine = Engine::open_memory().expect("memory engine");
        let bundle = format!("rmt_{seed}");
        make_theta_bundle(&mut engine, &bundle);

        let mut rng = TestRng::new(seed);
        let mut batch: Vec<Record> = Vec::new();
        for i in 0..v {
            for j in (i + 1)..v {
                if rng.uniform() < p {
                    let theta = 2.0 * std::f64::consts::PI * rng.uniform();
                    batch.push(theta_edge(i as i64, j as i64, theta));
                }
            }
        }
        assert!(
            batch.len() > 3000,
            "fixture sanity: expected ≈4100 edges, got {} (seed {seed})",
            batch.len()
        );
        engine.batch_insert(&bundle, &batch).expect("batch_insert");

        let gue_vals = eigenvalues_of(
            &mut engine,
            &format!("SPECTRAL_GAUGE {bundle} ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL;"),
        );
        let goe_vals = eigenvalues_of(
            &mut engine,
            &format!("SPECTRAL_GAUGE {bundle} ON FIBER (theta) GROUP U(1) FULL;"),
        );
        assert_eq!(gue_vals.len(), goe_vals.len());

        let r_gue = mean_spacing_ratio(&gue_vals, trim);
        let r_goe = mean_spacing_ratio(&goe_vals, trim);
        println!("seed {seed}: cos-weight r̃ = {r_goe:.4}, magnetic r̃ = {r_gue:.4}");
        goe_rs.push(r_goe);
        gue_rs.push(r_gue);
    }

    let r_goe = goe_rs.iter().sum::<f64>() / goe_rs.len() as f64;
    let r_gue = gue_rs.iter().sum::<f64>() / gue_rs.len() as f64;
    println!(
        "measured 4-seed mean spacing ratios: cos-weight r̃ = {r_goe:.4} \
         (GOE anchor 0.5307), magnetic r̃ = {r_gue:.4} (GUE anchor 0.5996), \
         Poisson anchor 0.3863"
    );

    assert!(
        (r_goe - 0.5307).abs() <= 0.03,
        "cos-weight mode 4-seed mean r̃ = {r_goe:.4} not within ±0.03 of GOE 0.5307 \
         (per-seed: {goe_rs:?}; Poisson 0.3863 would mean localization; \
          do NOT widen the tolerance — investigate)"
    );
    assert!(
        (r_gue - 0.5996).abs() <= 0.03,
        "MODE MAGNETIC 4-seed mean r̃ = {r_gue:.4} not within ±0.03 of GUE 0.5996 \
         (per-seed: {gue_rs:?}; 0.5307 would mean time-reversal symmetry was NOT \
          broken — check the Hermitian assembly; do NOT widen the tolerance)"
    );
}
