//! Halcyon L=24 workflow — Concept 2 RED tests.
//!
//! `INGEST <bundle> FROM '<path>' FORMAT NPZ AS GAUGE_FIELD GROUP <g>
//! ON LATTICE <l>` — the interpretation-clause path that turns a raw
//! NPZ produced by Halcyon's harvest into a bundle whose records carry
//! canonical base fields (config_id, mu, site_x, site_y, site_z, site_t)
//! and canonical fiber fields per group (SU(2)=q0..q3, SU(3)=re_00..im_22,
//! U(1)=theta, Z(N)=index).
//!
//! Every test in this file is expected to FAIL on the current tree
//! (RED). GREEN lands together with:
//!
//! - `parser::GaugeFieldInterpretation` struct
//! - `Statement::Ingest.as_gauge_field: Option<GaugeFieldInterpretation>`
//! - `parse_ingest` tail that reads `AS GAUGE_FIELD GROUP <g> ON LATTICE <l>`
//! - `ingest::execute_ingest_as_gauge_field(...)` public entry point
//! - `ingest::canonical_fiber_names(Group)` dispatcher
//! - New `IngestError` variants: `LatticeNotFound`, `FiberWidthMismatch`,
//!   `AxisCountMismatch`, `SiteAxisExtentMismatch`, `DirectionAxisMismatch`,
//!   `MultiArrayNotAllowedForGaugeField`
//! - `ingest::SU2_FIBER_NAMES` / `SU3_FIBER_NAMES` / `U1_FIBER_NAMES` /
//!   `ZN_FIBER_NAMES` / `SITE_AXIS_NAMES` constants
//!
//! Fixture synthesis: every test builds its own NPZ inline via the
//! same `npyz` writer path `tests/ingest_executor.rs` uses. No
//! committed binary blobs. All fixtures are L<=4 D<=2 (or L=4 D=2)
//! so the whole suite runs in <100ms.
//!
//! Depends on Concept 1 (LATTICE ... OBC AXIS <k>) for the `lattice.dim`
//! field the interpretation path reads. Until Concept 1 lands, the
//! tests here use PERIODIC lattices only, so the RED failure is
//! entirely about the INGEST interpretation surface, not the OBC
//! grammar.

#![cfg(feature = "lattice")]

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use gigi::engine::Engine;
use gigi::ingest::{
    canonical_fiber_names, execute_ingest_as_gauge_field, IngestError, IngestFormat,
    SITE_AXIS_NAMES, SU2_FIBER_NAMES, SU3_FIBER_NAMES, U1_FIBER_NAMES, ZN_FIBER_NAMES,
};
use gigi::gauge::Group;
use gigi::parser;
use gigi::types::Value;

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

// ── Helpers ─────────────────────────────────────────────────────────

/// Write a single-array NPZ file to `path`. The array has the given
/// shape and elements stored in row-major (C) order. Same writer as
/// `tests/ingest_executor.rs` uses so the two suites exercise the
/// same NPZ read path.
fn write_test_npz_single(path: &Path, array_name: &str, shape: &[u64], data: &[f64]) {
    let expected_len: u64 = shape.iter().product();
    assert_eq!(
        data.len() as u64,
        expected_len,
        "fixture mismatch: shape product {} != data len {}",
        expected_len,
        data.len()
    );
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

/// Two-array NPZ writer — used to exercise the multi-array-rejection
/// path for the GAUGE_FIELD interpretation.
fn write_test_npz_multi(path: &Path, arrays: &[(&str, &[u64], &[f64])]) {
    let file = File::create(path).expect("create test NPZ");
    let mut npz = NpzWriter::new(BufWriter::new(file));
    for (name, shape, data) in arrays {
        let expected_len: u64 = shape.iter().product();
        assert_eq!(data.len() as u64, expected_len, "fixture mismatch on `{}`", name);
        let opts = npyz::zip::write::FileOptions::default()
            .compression_method(npyz::zip::CompressionMethod::Stored);
        let builder = npz.array::<f64>(name, opts).expect("start array");
        let mut writer = builder
            .default_dtype()
            .shape(shape)
            .begin_nd()
            .expect("begin_nd");
        for &v in *data {
            writer.push(&v).expect("push f64");
        }
        writer.finish().expect("finish array");
    }
    npz.zip_writer().finish().expect("finish zip");
}

/// Open a fresh engine on a tempdir. Same idiom as tests/ingest_executor.rs.
fn open_engine() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

/// Declare a periodic L^D cubic lattice via the parser so it lands in
/// `lattice::registry`, matching what the runtime interpretation path
/// will look up. Uses the existing `LATTICE ... FROM CUBIC ... PERIODIC`
/// grammar so this file is decoupled from Concept 1's OBC extension.
fn declare_lattice(engine: &mut Engine, name: &str, l: usize, d: usize) {
    let decl = format!("LATTICE {name} FROM CUBIC L={l} DIM={d} PERIODIC;");
    let stmt = parser::parse(&decl).unwrap_or_else(|e| panic!("parse `{decl}` failed: {e}"));
    parser::execute(engine, &stmt).unwrap_or_else(|e| panic!("execute `{decl}` failed: {e}"));
}

/// Synthesize an all-identity SU(2) NPZ of shape
/// `(n_configs, D, L, L, ..., L, 4)` with `q = (1, 0, 0, 0)` at every
/// link. Total records after INGEST = n_configs * D * L^D.
fn write_su2_identity_npz(path: &Path, array_name: &str, n_configs: usize, d: usize, l: usize) {
    let sites_per_muconf: usize = (0..d).fold(1usize, |a, _| a * l);
    let n_links = n_configs * d * sites_per_muconf;
    let mut data: Vec<f64> = Vec::with_capacity(n_links * 4);
    for _ in 0..n_links {
        data.push(1.0); // q0
        data.push(0.0); // q1
        data.push(0.0); // q2
        data.push(0.0); // q3
    }
    let mut shape: Vec<u64> = Vec::with_capacity(d + 3);
    shape.push(n_configs as u64);
    shape.push(d as u64);
    for _ in 0..d {
        shape.push(l as u64);
    }
    shape.push(4); // SU(2) repr_dim
    write_test_npz_single(path, array_name, &shape, &data);
}

// ── Parser-level tests ─────────────────────────────────────────────

/// A full INGEST with the GAUGE_FIELD interpretation tail parses to a
/// Statement::Ingest with a `Some(GaugeFieldInterpretation { .. })`.
#[test]
fn test_ingest_gauge_field_parser_accepts_full_tail() {
    let src = "INGEST b FROM '/tmp/x.npz' FORMAT NPZ AS GAUGE_FIELD GROUP SU2 ON LATTICE l4;";
    let stmt = parser::parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    match stmt {
        parser::Statement::Ingest {
            bundle,
            source,
            format,
            key,
            as_gauge_field,
        } => {
            assert_eq!(bundle, "b");
            assert_eq!(source, "/tmp/x.npz");
            assert_eq!(format, "NPZ");
            assert!(key.is_none(), "no KEY clause → None");
            let interp = as_gauge_field.expect("GAUGE_FIELD interpretation clause present");
            assert_eq!(interp.group, Group::SU2);
            assert_eq!(interp.lattice_name, "l4");
        }
        other => panic!("expected Statement::Ingest, got {other:?}"),
    }
}

/// Without the AS clause the interpretation is None — backwards compat
/// with every existing INGEST test at the 889/0 lib floor.
#[test]
fn test_ingest_gauge_field_parser_defaults_to_none_without_as() {
    let src = "INGEST b FROM '/tmp/x.npz' FORMAT NPZ;";
    let stmt = parser::parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    match stmt {
        parser::Statement::Ingest { as_gauge_field, .. } => {
            assert!(
                as_gauge_field.is_none(),
                "backwards compat: no AS clause → None"
            );
        }
        other => panic!("expected Statement::Ingest, got {other:?}"),
    }
}

/// `AS GAUGE_FIELD` alone (no GROUP/ON LATTICE tail) is a parse error,
/// not a silent no-op. The error message names the missing keyword so
/// the user sees the surface, not a mystery statement-termination error.
#[test]
fn test_ingest_gauge_field_parser_errors_on_partial_tail() {
    let src = "INGEST b FROM '/tmp/x.npz' FORMAT NPZ AS GAUGE_FIELD;";
    let err = parser::parse(src).expect_err("partial tail must fail");
    assert!(
        err.to_uppercase().contains("GROUP"),
        "error should name missing GROUP keyword: {err}"
    );
}

// ── SU(2) canonical shape / count tests ────────────────────────────

/// SU(2) L=4 D=2, 2 configs → 2 * 2 * 4^2 = 64 records/config × 2 configs
/// on the SPEC's 32/config accounting, but the harvest layout is
/// (n_configs, D, L, L, fiber) so:
///   records = n_configs * D * L^D = 2 * 2 * 16 = 64 total.
#[test]
fn test_ingest_su2_synthetic_l4_records_correct_count() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path: PathBuf = tmp.path().join("su2_l4_d2.npz");
    // 2 configs × D=2 × L=4 → per-config records = D*L^D = 32.
    // Spec's inline example says "2×4^2×2×4 = 64 quaternion links →
    // 64 records/config" — that's total-records for BOTH configs.
    write_su2_identity_npz(&path, "su2_field", /*n_configs=*/ 2, /*d=*/ 2, /*l=*/ 4);
    declare_lattice(&mut engine, "l4", /*l=*/ 4, /*d=*/ 2);

    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "su2_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4",
        None,
    )
    .expect("INGEST AS GAUGE_FIELD succeeds on well-shaped SU(2) NPZ");

    let expected_records = 2 * 2 * 4usize.pow(2); // n_configs * D * L^D = 64
    assert_eq!(
        stats.records_emitted, expected_records,
        "records = n_configs * D * L^D = 2 * 2 * 16 = 64"
    );
    assert!(stats.bundle_created, "bundle auto-created from canonical schema");
}

/// SU(2) L=4 D=2 → the bundle carries exactly the canonical fields:
///   base = [config_id, mu, site_x, site_y]
///   fiber = [q0, q1, q2, q3]
#[test]
fn test_ingest_su2_synthetic_l4_canonical_field_names() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("su2_names.npz");
    write_su2_identity_npz(&path, "su2_field", 1, 2, 4);
    declare_lattice(&mut engine, "l4_names", 4, 2);

    execute_ingest_as_gauge_field(
        &mut engine,
        "su2_names_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_names",
        None,
    )
    .expect("ingest");

    let bundle = engine
        .bundle("su2_names_bundle")
        .expect("bundle exists after ingest");
    let store = bundle.as_heap().expect("heap-resident");
    let base_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(
        base_names,
        vec![
            "config_id".to_string(),
            "mu".to_string(),
            "site_x".to_string(),
            "site_y".to_string(),
            "vertex_a".to_string(),
            "vertex_b".to_string(),
        ],
        "base fields = config_id, mu, site_x, site_y, vertex_a, vertex_b"
    );

    let fiber_names: Vec<String> = store
        .schema
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(
        fiber_names,
        vec![
            "q0".to_string(),
            "q1".to_string(),
            "q2".to_string(),
            "q3".to_string(),
        ],
        "fiber fields = q0, q1, q2, q3"
    );

    // One canonical record: config_id=0, mu=0, site=(0,0), q=(1,0,0,0).
    let mut found = false;
    for rec in store.records() {
        if let (Some(Value::Integer(0)), Some(Value::Integer(0)),
                Some(Value::Integer(0)), Some(Value::Integer(0)),
                Some(Value::Float(q0)))
            = (rec.get("config_id"), rec.get("mu"),
               rec.get("site_x"), rec.get("site_y"),
               rec.get("q0"))
        {
            assert!((q0 - 1.0).abs() < 1e-12);
            found = true;
            break;
        }
    }
    assert!(found, "expected identity record at (0,0,0,0) with q0=1.0");
}

/// SU(3) L=4 D=2 → 18 canonical fiber fields.
#[test]
fn test_ingest_su3_synthetic_l4_records_field_names() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("su3_l4.npz");
    // Shape (1, 2, 4, 4, 18) → 1 config, D=2, L=4, SU(3) repr_dim=18.
    let n_links = 1 * 2 * 4usize.pow(2);
    let mut data: Vec<f64> = Vec::with_capacity(n_links * 18);
    for _ in 0..n_links {
        // Identity SU(3): diagonal 1s, everything else 0. In the
        // (re, im) interleaved layout that's re_00=1, im_00=0,
        // re_11=1, ..., re_22=1.
        for pair_idx in 0..9 {
            let is_diag = matches!(pair_idx, 0 | 4 | 8); // (0,0), (1,1), (2,2)
            data.push(if is_diag { 1.0 } else { 0.0 }); // re
            data.push(0.0); // im
        }
    }
    write_test_npz_single(&path, "su3_field", &[1, 2, 4, 4, 18], &data);
    declare_lattice(&mut engine, "l4_su3", 4, 2);

    execute_ingest_as_gauge_field(
        &mut engine,
        "su3_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU3,
        "l4_su3",
        None,
    )
    .expect("SU(3) ingest");

    let bundle = engine.bundle("su3_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    let fiber_names: Vec<String> = store
        .schema
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    // Canonical SU(3): 18 fields re_00..im_22.
    assert_eq!(fiber_names.len(), 18, "SU(3) has 18 fiber fields");
    assert_eq!(fiber_names[0], "re_00");
    assert_eq!(fiber_names[1], "im_00");
    assert_eq!(fiber_names[17], "im_22");
}

/// U(1) L=4 D=2 → single `theta` fiber field.
#[test]
fn test_ingest_u1_synthetic_l4_theta_field() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("u1_l4.npz");
    // Shape (1, 2, 4, 4, 1) → 32 links × 1 float = 32 f64s.
    let data: Vec<f64> = (0..32).map(|i| i as f64 * 0.1).collect();
    write_test_npz_single(&path, "u1_field", &[1, 2, 4, 4, 1], &data);
    declare_lattice(&mut engine, "l4_u1", 4, 2);

    execute_ingest_as_gauge_field(
        &mut engine,
        "u1_bundle",
        &path,
        IngestFormat::Npz,
        Group::U1,
        "l4_u1",
        None,
    )
    .expect("U(1) ingest");

    let bundle = engine.bundle("u1_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    let fiber_names: Vec<String> = store
        .schema
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(fiber_names, vec!["theta".to_string()]);
}

/// Z(N) L=4 D=2 → single `index` fiber field.
#[test]
fn test_ingest_zn_synthetic_l4_index_field() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("zn_l4.npz");
    let data: Vec<f64> = (0..32).map(|i| (i % 5) as f64).collect();
    write_test_npz_single(&path, "zn_field", &[1, 2, 4, 4, 1], &data);
    declare_lattice(&mut engine, "l4_zn", 4, 2);

    execute_ingest_as_gauge_field(
        &mut engine,
        "zn_bundle",
        &path,
        IngestFormat::Npz,
        Group::ZN { n: 5 },
        "l4_zn",
        None,
    )
    .expect("Z(N) ingest");

    let bundle = engine.bundle("zn_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    let fiber_names: Vec<String> = store
        .schema
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(fiber_names, vec!["index".to_string()]);
}

// ── Shape / axis-count / fiber-width error tests ──────────────────

/// Wrong fiber width for the declared group → clear error naming
/// expected vs got + the group label.
#[test]
fn test_ingest_fiber_width_mismatch_errors_clearly() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("bad_fiber.npz");
    // Declare SU(2) (expects fiber=4) but fixture has fiber=9 (SU(3) old).
    let data: Vec<f64> = vec![0.0; 1 * 2 * 4 * 4 * 9];
    write_test_npz_single(&path, "bad_fiber", &[1, 2, 4, 4, 9], &data);
    declare_lattice(&mut engine, "l4_bad_fiber", 4, 2);

    let err = execute_ingest_as_gauge_field(
        &mut engine,
        "bad_fiber_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_bad_fiber",
        None,
    )
    .expect_err("fiber width mismatch must error");
    match err {
        IngestError::FiberWidthMismatch { group, expected, got } => {
            assert_eq!(group, "SU(2)");
            assert_eq!(expected, 4);
            assert_eq!(got, 9);
        }
        other => panic!("expected FiberWidthMismatch, got {other:?}"),
    }
}

/// Lattice name not found in registry → LatticeNotFound with the name.
#[test]
fn test_ingest_lattice_not_found_errors() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("ghost_lat.npz");
    write_su2_identity_npz(&path, "su2_field", 1, 2, 4);

    let err = execute_ingest_as_gauge_field(
        &mut engine,
        "bad_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "not_a_lattice",
        None,
    )
    .expect_err("undeclared lattice must error");
    match err {
        IngestError::LatticeNotFound { name } => {
            assert_eq!(name, "not_a_lattice");
        }
        other => panic!("expected LatticeNotFound, got {other:?}"),
    }
}

/// Array ndim ≠ 1 + 1 + D + 1 → AxisCountMismatch with the expected/got
/// ndim and lattice.dim.
#[test]
fn test_ingest_axis_count_mismatch_errors() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("bad_ndim.npz");
    // Declared lattice is D=2 → expected ndim = 1 + 1 + 2 + 1 = 5.
    // Give it a 3-axis array (n_configs, sites_flat, fiber). Must reject.
    let data: Vec<f64> = vec![1.0; 1 * 32 * 4];
    write_test_npz_single(&path, "flat_arr", &[1, 32, 4], &data);
    declare_lattice(&mut engine, "l4_bad_ndim", 4, 2);

    let err = execute_ingest_as_gauge_field(
        &mut engine,
        "bad_ndim_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_bad_ndim",
        None,
    )
    .expect_err("axis-count mismatch must error");
    match err {
        IngestError::AxisCountMismatch { expected_ndim, got_ndim, lattice_dim } => {
            assert_eq!(expected_ndim, 5);
            assert_eq!(got_ndim, 3);
            assert_eq!(lattice_dim, 2);
        }
        other => panic!("expected AxisCountMismatch, got {other:?}"),
    }
}

/// mu-axis extent ≠ lattice.dim → DirectionAxisMismatch.
#[test]
fn test_ingest_direction_axis_mismatch_errors() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("bad_mu.npz");
    // D=2 lattice, but array axis 1 has extent 3.
    let data: Vec<f64> = vec![0.0; 1 * 3 * 4 * 4 * 4];
    write_test_npz_single(&path, "bad_mu", &[1, 3, 4, 4, 4], &data);
    declare_lattice(&mut engine, "l4_bad_mu", 4, 2);

    let err = execute_ingest_as_gauge_field(
        &mut engine,
        "bad_mu_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_bad_mu",
        None,
    )
    .expect_err("direction axis mismatch must error");
    match err {
        IngestError::DirectionAxisMismatch { expected_d, got } => {
            assert_eq!(expected_d, 2);
            assert_eq!(got, 3);
        }
        other => panic!("expected DirectionAxisMismatch, got {other:?}"),
    }
}

/// Site-axis extents disagree with each other → SiteAxisExtentMismatch.
/// (Concept 1 lattices are L-uniform; a non-uniform site axis is the
/// clearest error to surface.)
#[test]
fn test_ingest_site_axis_extent_mismatch_errors() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("bad_site.npz");
    // D=2 lattice, but site axes are (4, 5) — inconsistent.
    let data: Vec<f64> = vec![0.0; 1 * 2 * 4 * 5 * 4];
    write_test_npz_single(&path, "bad_site", &[1, 2, 4, 5, 4], &data);
    declare_lattice(&mut engine, "l4_bad_site", 4, 2);

    let err = execute_ingest_as_gauge_field(
        &mut engine,
        "bad_site_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_bad_site",
        None,
    )
    .expect_err("non-uniform site axis must error");
    match err {
        IngestError::SiteAxisExtentMismatch { .. } => { /* ok */ }
        other => panic!("expected SiteAxisExtentMismatch, got {other:?}"),
    }
}

/// The generic AUTO_GENERIC INGEST path is UNTOUCHED by this concept.
/// A parse of the plain form + execute_ingest still emits one record
/// per outer slice — the 889/0 lib floor stays green.
#[test]
fn test_ingest_generic_still_works_without_as_clause() {
    use gigi::ingest::execute_ingest;
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("generic.npz");
    let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
    write_test_npz_single(&path, "generic", &[3, 4], &data);

    let stats = execute_ingest(&mut engine, "generic_bundle", &path, IngestFormat::Npz, None)
        .expect("generic ingest still works");
    assert_eq!(stats.records_emitted, 3);
    assert!(stats.bundle_created);
}

/// A multi-member NPZ is rejected under the GAUGE_FIELD interpretation
/// — Halcyon's harvest convention is one array per file, and silently
/// accepting multiple arrays would drop or double-count links.
#[test]
fn test_ingest_gauge_field_multi_array_rejected() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("multi.npz");
    let a: Vec<f64> = vec![1.0; 1 * 2 * 4 * 4 * 4];
    let b: Vec<f64> = vec![0.0; 1 * 2 * 4 * 4 * 4];
    write_test_npz_multi(
        &path,
        &[
            ("first", &[1, 2, 4, 4, 4], &a),
            ("second", &[1, 2, 4, 4, 4], &b),
        ],
    );
    declare_lattice(&mut engine, "l4_multi", 4, 2);

    let err = execute_ingest_as_gauge_field(
        &mut engine,
        "multi_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_multi",
        None,
    )
    .expect_err("multi-array NPZ must be rejected for GAUGE_FIELD");
    match err {
        IngestError::MultiArrayNotAllowedForGaugeField { got } => {
            assert_eq!(got, 2);
        }
        other => panic!("expected MultiArrayNotAllowedForGaugeField, got {other:?}"),
    }
}

/// A fresh bundle name is auto-created with the canonical GAUGE_FIELD
/// schema (base = config_id/mu/site_*; fiber = per-group canonical names).
#[test]
fn test_ingest_gauge_field_auto_creates_bundle_with_canonical_schema() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("auto.npz");
    write_su2_identity_npz(&path, "su2_auto", 1, 2, 4);
    declare_lattice(&mut engine, "l4_auto", 4, 2);

    // Bundle does not exist yet.
    assert!(engine.bundle("auto_bundle").is_none());

    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "auto_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_auto",
        None,
    )
    .expect("ingest");
    assert!(stats.bundle_created, "bundle was auto-created");

    let bundle = engine.bundle("auto_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    // 6 base fields (config_id, mu, site_x, site_y, vertex_a, vertex_b)
    // + 4 fiber fields (q0..q3).
    assert_eq!(store.schema.base_fields.len(), 6);
    assert_eq!(store.schema.fiber_fields.len(), 4);
}

/// A pre-existing bundle whose schema matches the canonical GAUGE_FIELD
/// schema is accepted (bundle_created = false), and records land.
#[test]
fn test_ingest_gauge_field_existing_bundle_compat_check() {
    use gigi::types::{BundleSchema, FieldDef};
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("pre.npz");
    write_su2_identity_npz(&path, "su2_pre", 1, 2, 4);
    declare_lattice(&mut engine, "l4_pre", 4, 2);

    // Pre-create the bundle with the canonical schema INGEST would
    // infer. Every field is Numeric — the emitter uses scalar Numeric
    // for both base and fiber columns so SPECTRAL_GAUGE ON FIBER can
    // read them directly. vertex_a / vertex_b carry the row-major
    // edge endpoints the ingest emitter attaches from the lattice.
    let schema = BundleSchema::new("pre_bundle")
        .base(FieldDef::numeric("config_id"))
        .base(FieldDef::numeric("mu"))
        .base(FieldDef::numeric("site_x"))
        .base(FieldDef::numeric("site_y"))
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .fiber(FieldDef::numeric("q0"))
        .fiber(FieldDef::numeric("q1"))
        .fiber(FieldDef::numeric("q2"))
        .fiber(FieldDef::numeric("q3"));
    engine.create_bundle(schema).expect("create_bundle");

    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "pre_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_pre",
        None,
    )
    .expect("ingest into existing compatible bundle");
    assert!(!stats.bundle_created, "existing compatible bundle reused");
    assert_eq!(stats.records_emitted, 1 * 2 * 4usize.pow(2));
}

// ── Canonical constants — self-check ─────────────────────────────

/// The four canonical fiber-name tables and the dispatcher agree with
/// `Group::repr_dim()`.
#[test]
fn test_canonical_fiber_names_lengths_match_repr_dim() {
    assert_eq!(SU2_FIBER_NAMES.len(), Group::SU2.repr_dim());
    assert_eq!(SU3_FIBER_NAMES.len(), Group::SU3.repr_dim());
    assert_eq!(U1_FIBER_NAMES.len(), Group::U1.repr_dim());
    assert_eq!(ZN_FIBER_NAMES.len(), Group::ZN { n: 3 }.repr_dim());

    assert_eq!(canonical_fiber_names(Group::SU2), &SU2_FIBER_NAMES[..]);
    assert_eq!(canonical_fiber_names(Group::SU3), &SU3_FIBER_NAMES[..]);
    assert_eq!(canonical_fiber_names(Group::U1), &U1_FIBER_NAMES[..]);
    assert_eq!(canonical_fiber_names(Group::ZN { n: 3 }), &ZN_FIBER_NAMES[..]);

    // Site-axis names cover 1D through 4D.
    assert!(SITE_AXIS_NAMES.len() >= 4);
    assert_eq!(SITE_AXIS_NAMES[0], "site_x");
    assert_eq!(SITE_AXIS_NAMES[1], "site_y");
    assert_eq!(SITE_AXIS_NAMES[2], "site_z");
    assert_eq!(SITE_AXIS_NAMES[3], "site_t");
}
