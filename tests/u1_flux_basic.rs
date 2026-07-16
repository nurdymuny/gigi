//! U(1) flux path — INGEST theta pin + `GAUGE_FIELD ... INIT FLUX`
//! (Concept C, 2026-07-16).
//!
//! Two halves:
//!
//! C.1 (PINNING, green on the pre-Concept-C tree): `INGEST ... AS
//! GAUGE_FIELD GROUP U(1) ON LATTICE l` already works end-to-end with
//! canonical fiber name `theta` (repr_dim 1) — pinned here TOGETHER
//! with the downstream SPECTRAL_GAUGE MODE MAGNETIC read, i.e. the
//! exact ingest → magnetic-spectrum leg of Hallie's RH loop.
//!
//! C.2 (RED): the flux-init path —
//!   GAUGE_FIELD name GROUP U(1) INIT FLUX RANDOM SEED <n> ON LATTICE <l>;
//!   GAUGE_FIELD name GROUP U(1) INIT FLUX UNIFORM <phi> ON LATTICE <l>;
//! RANDOM: i.i.d. θ ~ Uniform[0, 2π) from the house xorshift64*
//! SmallRng (gauge::marsaglia_haar), θ_k = 2π · uniform() drawn for
//! edge k in the LATTICE'S OWN edge order 0..n_edges (the same order
//! INIT HAAR walks) — reproducibility is part of the contract.
//! UNIFORM: every edge phase = phi.
//!
//! INIT FLUX (U(1)) MATERIALIZES A BUNDLE named after the field:
//! base fields (config_id = 0, edge_id, vertex_a, vertex_b) + fiber
//! (theta), one record per lattice edge, with (vertex_a → vertex_b) =
//! the lattice's own oriented edge — exactly the columns
//! SPECTRAL_GAUGE ON FIBER (theta) consumes, and the same θ-orientation
//! convention MODE MAGNETIC assembles (a → b carries e^{+iθ}). It does
//! NOT register a DenseLinkBuffer in the gauge registry (U(1) has no
//! link buffer this phase; the bundle IS the artifact).
//!
//! Grammar note: clause order is FLEXIBLE (probe S2 puts GROUP before
//! ON LATTICE and ON LATTICE at the tail; the Part-II canonical order
//! must keep parsing too).
//!
//! Errors: non-U(1) INIT FLUX; PERSIST + FLUX; FLUX RANDOM without
//! SEED (parse-time); target bundle already exists.
//!
//! Run with:
//!   `cargo test --features halcyon --test u1_flux_basic`

#![cfg(feature = "halcyon")]

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use gigi::engine::Engine;
use gigi::gauge::Group;
use gigi::ingest::{execute_ingest_as_gauge_field, IngestFormat};
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

mod common;

// ── Helpers ─────────────────────────────────────────────────────────

/// Single-array NPZ writer (same writer path the ingest suites use).
fn write_test_npz_single(path: &Path, array_name: &str, shape: &[u64], data: &[f64]) {
    let expected_len: u64 = shape.iter().product();
    assert_eq!(data.len() as u64, expected_len, "fixture shape mismatch");
    let file = File::create(path).expect("create test NPZ");
    let mut npz = NpzWriter::new(BufWriter::new(file));
    {
        let opts = npyz::zip::write::FileOptions::default()
            .compression_method(npyz::zip::CompressionMethod::Stored);
        let builder = npz.array::<f64>(array_name, opts).expect("start array");
        let mut writer = builder
            .default_dtype()
            .shape(shape)
            .begin_nd()
            .expect("begin_nd");
        for &v in data {
            writer.push(&v).expect("push f64");
        }
        writer.finish().expect("finish array");
    }
    npz.zip_writer().finish().expect("finish zip");
}

fn run(engine: &mut Engine, gql: &str) -> Result<ExecResult, String> {
    let stmt = parse(gql).map_err(|e| format!("parse `{gql}`: {e}"))?;
    execute(engine, &stmt)
}

fn run_ok(engine: &mut Engine, gql: &str) -> ExecResult {
    run(engine, gql).unwrap_or_else(|e| panic!("`{gql}` failed: {e}"))
}

/// Collect (edge_id, theta) pairs from a flux bundle, sorted by
/// edge_id.
fn collect_thetas(engine: &Engine, bundle: &str) -> Vec<(i64, f64)> {
    let bundle_ref = engine
        .bundle(bundle)
        .unwrap_or_else(|| panic!("bundle '{bundle}' missing"));
    let store = bundle_ref.as_heap().expect("heap-resident");
    let mut out: Vec<(i64, f64)> = store
        .records()
        .map(|rec| {
            let e = rec.get("edge_id").and_then(|v| v.as_i64()).expect("edge_id");
            let t = rec.get("theta").and_then(|v| v.as_f64()).expect("theta");
            (e, t)
        })
        .collect();
    out.sort_by_key(|(e, _)| *e);
    out
}

// ── C.1 — U(1) theta ingest PIN (green pre-Concept-C) ──────────────

/// PIN: U(1) NPZ ingest lands canonical `theta` fiber (repr_dim 1)
/// and the ingested bundle drives SPECTRAL_GAUGE MODE MAGNETIC FULL —
/// the ingest leg of Hallie's generate → INGEST → SPECTRAL loop.
/// (Named a pin: this was already green; Concept C must not regress
/// it.)
#[test]
fn test_u1_theta_ingest_end_to_end_pin() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("u1_flux_pin.npz");

    // Shape (1, 2, 4, 4, 1): 32 links, pseudo-random thetas.
    let data: Vec<f64> = (0..32).map(|i| (i as f64 * 0.7) % 6.28).collect();
    write_test_npz_single(&path, "u1_field", &[1, 2, 4, 4, 1], &data);

    run_ok(
        &mut engine,
        "LATTICE u1fluxpin_lat FROM CUBIC L=4 DIM=2 PERIODIC;",
    );
    execute_ingest_as_gauge_field(
        &mut engine,
        "u1fluxpin_bundle",
        &common::ingest_rel(&path),
        IngestFormat::Npz,
        Group::U1,
        "u1fluxpin_lat",
        None,
    )
    .expect("U(1) ingest must stay green (pin)");

    let bundle_ref = engine.bundle("u1fluxpin_bundle").expect("bundle exists");
    let store = bundle_ref.as_heap().expect("heap");
    let fiber_names: Vec<String> = store
        .schema
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(fiber_names, vec!["theta".to_string()], "canonical U(1) fiber");
    assert_eq!(Group::U1.repr_dim(), 1, "repr_dim pin");

    // The ingested bundle drives the magnetic spectrum directly.
    let result = run_ok(
        &mut engine,
        "SPECTRAL_GAUGE u1fluxpin_bundle ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 8;",
    );
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows, got {other:?}"),
    };
    let vals = match rows[0].get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector, got {other:?}"),
    };
    assert_eq!(vals.len(), 8, "LIMIT 8 on the ingested bundle");
    for w in vals.windows(2) {
        assert!(w[0] <= w[1] + 1e-12, "ascending");
    }
}

// ── C.2 — INIT FLUX grammar ─────────────────────────────────────────

/// Probe S2's exact clause order: GROUP before INIT, ON LATTICE at the
/// tail. RED until the GAUGE_FIELD grammar goes clause-order-flexible
/// and INIT FLUX lands.
#[test]
fn test_parse_init_flux_random_probe_s2_clause_order() {
    parse("GAUGE_FIELD rh_flux GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE l4_rh;")
        .expect("S2 clause order must parse");
}

/// Canonical Part-II clause order with INIT FLUX UNIFORM.
#[test]
fn test_parse_init_flux_uniform_canonical_order() {
    parse("GAUGE_FIELD f ON LATTICE l GROUP U(1) INIT FLUX UNIFORM 0.7;")
        .expect("canonical order + FLUX UNIFORM must parse");
}

/// FLUX RANDOM without SEED is a parse error naming SEED —
/// reproducibility is part of the contract, so the seed is mandatory.
#[test]
fn test_parse_init_flux_random_requires_seed() {
    let err = parse("GAUGE_FIELD f ON LATTICE l GROUP U(1) INIT FLUX RANDOM;")
        .expect_err("FLUX RANDOM without SEED must be rejected");
    assert!(err.contains("SEED"), "error must name SEED: {err}");
}

/// The Part-II canonical grammar (ON LATTICE first) keeps parsing for
/// every existing init — pin against the clause-order relaxation.
#[test]
fn test_old_gauge_field_grammar_still_parses_pin() {
    parse("GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;")
        .expect("Part-II canonical order must keep parsing (pin)");
    parse("GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT HAAR_RANDOM SEED 7 PERSIST;")
        .expect("HAAR + PERSIST canonical order must keep parsing (pin)");
}

/// A GAUGE_FIELD missing a required clause errors naming the missing
/// clause (flexible order must not weaken required-clause checking).
#[test]
fn test_gauge_field_missing_init_clause_errors() {
    let err = parse("GAUGE_FIELD f ON LATTICE l GROUP U(1);")
        .expect_err("missing INIT must be rejected");
    assert!(err.contains("INIT"), "error must name the missing INIT: {err}");
}

// ── C.2 — INIT FLUX semantics ───────────────────────────────────────

/// RANDOM SEED determinism: the same statement on two fresh engines
/// yields byte-identical theta columns; a different seed differs.
/// Also pins the draw contract: θ_k = 2π · uniform_k from the house
/// SmallRng (xorshift64*), edge order 0..n_edges.
#[test]
fn test_init_flux_random_seed_deterministic() {
    let mut e1 = Engine::open_memory().expect("engine 1");
    run_ok(&mut e1, "LATTICE u1fluxdet_lat FROM CUBIC L=4 DIM=2 PERIODIC;");
    run_ok(
        &mut e1,
        "GAUGE_FIELD u1fluxdet GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE u1fluxdet_lat;",
    );
    let t1 = collect_thetas(&e1, "u1fluxdet");

    let mut e2 = Engine::open_memory().expect("engine 2");
    run_ok(
        &mut e2,
        "GAUGE_FIELD u1fluxdet GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE u1fluxdet_lat;",
    );
    let t2 = collect_thetas(&e2, "u1fluxdet");

    assert_eq!(t1.len(), 32, "L=4 D=2 PERIODIC has 2·16 = 32 links");
    assert_eq!(t1, t2, "same seed → byte-identical theta column");

    // All phases in [0, 2π).
    for (e, t) in &t1 {
        assert!(
            (0.0..2.0 * std::f64::consts::PI).contains(t),
            "theta out of [0, 2π) at edge {e}: {t}"
        );
    }

    // Draw contract: edge 0 gets the FIRST uniform of SmallRng(42),
    // scaled by 2π. Pins RNG choice + edge order byte-stably.
    let mut rng = gigi::gauge::marsaglia_haar::SmallRng::seed_from_u64(42);
    let expected0 = 2.0 * std::f64::consts::PI * rng.uniform();
    assert!(
        (t1[0].1 - expected0).abs() < 1e-15,
        "edge 0 must carry 2π·uniform₀ of SmallRng(42): got {}, want {expected0}",
        t1[0].1
    );

    // Different seed → different column.
    let mut e3 = Engine::open_memory().expect("engine 3");
    run_ok(
        &mut e3,
        "GAUGE_FIELD u1fluxdet GROUP U(1) INIT FLUX RANDOM SEED 43 ON LATTICE u1fluxdet_lat;",
    );
    let t3 = collect_thetas(&e3, "u1fluxdet");
    assert_ne!(t1, t3, "different seed must change the flux pattern");
}

/// UNIFORM phi stamps every edge with exactly phi, one record per
/// lattice edge, endpoints = the lattice's own oriented edges.
#[test]
fn test_init_flux_uniform_sets_every_edge_phi() {
    let mut engine = Engine::open_memory().expect("engine");
    run_ok(&mut engine, "LATTICE u1fluxuni_lat FROM CUBIC L=4 DIM=2 PERIODIC;");
    run_ok(
        &mut engine,
        "GAUGE_FIELD u1fluxuni GROUP U(1) INIT FLUX UNIFORM 0.7 ON LATTICE u1fluxuni_lat;",
    );
    let thetas = collect_thetas(&engine, "u1fluxuni");
    assert_eq!(thetas.len(), 32);
    for (e, t) in &thetas {
        assert!(
            (t - 0.7).abs() < 1e-15,
            "edge {e}: UNIFORM 0.7 must stamp exactly 0.7, got {t}"
        );
    }
    // Endpoint columns exist and are lattice vertex ids.
    let bundle_ref = engine.bundle("u1fluxuni").expect("bundle");
    let store = bundle_ref.as_heap().expect("heap");
    for rec in store.records() {
        let va = rec.get("vertex_a").and_then(|v| v.as_i64()).expect("vertex_a");
        let vb = rec.get("vertex_b").and_then(|v| v.as_i64()).expect("vertex_b");
        assert!((0..16).contains(&va), "vertex_a in 0..16, got {va}");
        assert!((0..16).contains(&vb), "vertex_b in 0..16, got {vb}");
    }
}

/// Non-U(1) INIT FLUX is a clear executor error naming U(1).
#[test]
fn test_init_flux_non_u1_errors() {
    let mut engine = Engine::open_memory().expect("engine");
    run_ok(&mut engine, "LATTICE u1fluxsu2_lat FROM CUBIC L=4 DIM=2 PERIODIC;");
    let err = run(
        &mut engine,
        "GAUGE_FIELD u1fluxsu2 GROUP SU(2) INIT FLUX RANDOM SEED 1 ON LATTICE u1fluxsu2_lat;",
    )
    .expect_err("SU(2) INIT FLUX must be rejected");
    assert!(
        err.contains("INIT FLUX requires GROUP U(1)"),
        "error must name the U(1) requirement: {err}"
    );
}

/// PERSIST composes with FLUX only as an explicit error this phase —
/// the materialized bundle is the durable artifact.
#[test]
fn test_init_flux_persist_rejected() {
    let mut engine = Engine::open_memory().expect("engine");
    run_ok(&mut engine, "LATTICE u1fluxper_lat FROM CUBIC L=4 DIM=2 PERIODIC;");
    let err = run(
        &mut engine,
        "GAUGE_FIELD u1fluxper GROUP U(1) INIT FLUX UNIFORM 0.1 ON LATTICE u1fluxper_lat PERSIST;",
    )
    .expect_err("PERSIST + FLUX must be rejected this phase");
    assert!(
        err.contains("PERSIST is not supported with INIT FLUX"),
        "error must name the PERSIST/FLUX exclusion: {err}"
    );
}

/// Re-running INIT FLUX into an existing bundle name is an error (an
/// init is a materialization, not an append).
#[test]
fn test_init_flux_existing_bundle_errors() {
    let mut engine = Engine::open_memory().expect("engine");
    run_ok(&mut engine, "LATTICE u1fluxdup_lat FROM CUBIC L=4 DIM=2 PERIODIC;");
    run_ok(
        &mut engine,
        "GAUGE_FIELD u1fluxdup GROUP U(1) INIT FLUX UNIFORM 0.2 ON LATTICE u1fluxdup_lat;",
    );
    let err = run(
        &mut engine,
        "GAUGE_FIELD u1fluxdup GROUP U(1) INIT FLUX UNIFORM 0.3 ON LATTICE u1fluxdup_lat;",
    )
    .expect_err("second INIT FLUX into the same bundle must error");
    assert!(
        err.contains("already exists"),
        "error must say the bundle already exists: {err}"
    );
}

// ── The local S1 → S2 → S3 loop ─────────────────────────────────────

/// End-to-end twin of live probes S1-S3: OBC lattice → INIT FLUX
/// RANDOM → SPECTRAL_GAUGE MODE MAGNETIC FULL LIMIT 8 returns 8
/// ascending reals, with n_records_used = the lattice's edge count
/// (L=4 D=2 OBC AXIS 0: 2·16 − 4 = 28 links).
#[test]
fn test_flux_to_magnetic_spectrum_s1_s2_s3_loop() {
    let mut engine = Engine::open_memory().expect("engine");
    run_ok(&mut engine, "LATTICE u1fluxloop_lat FROM CUBIC L=4 DIM=2 OBC AXIS 0;");
    run_ok(
        &mut engine,
        "GAUGE_FIELD u1fluxloop GROUP U(1) INIT FLUX RANDOM SEED 42 ON LATTICE u1fluxloop_lat;",
    );
    assert_eq!(
        collect_thetas(&engine, "u1fluxloop").len(),
        28,
        "OBC AXIS 0 drops the 4 wrap links: 32 − 4 = 28"
    );

    let result = run_ok(
        &mut engine,
        "SPECTRAL_GAUGE u1fluxloop ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 8;",
    );
    let rows = match result {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    let vals = match row.get("eigenvalues") {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("expected eigenvalues Vector, got {other:?}"),
    };
    assert_eq!(vals.len(), 8, "FULL LIMIT 8 → 8 eigenvalues");
    for w in vals.windows(2) {
        assert!(w[0] <= w[1] + 1e-12, "ascending reals");
    }
    match row.get("n_records_used") {
        Some(Value::Integer(n)) => assert_eq!(*n, 28, "28 flux links feed L_A"),
        other => panic!("expected n_records_used Integer, got {other:?}"),
    }
}
