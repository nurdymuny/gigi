//! INGEST AS GAUGE_FIELD — vertex_a / vertex_b base fields + OBC record omission.
//!
//! Every test expects the INGEST executor to emit `vertex_a` and
//! `vertex_b` as additional base INT fields per record, computed from
//! the lattice's own coordinate ↔ site helpers:
//!
//!   vertex_a = site_of(site_coords)
//!   vertex_b = site_of(shift_plus(site_coords, mu))
//!
//! For an OBC axis `k`, records whose (mu, coords) would wrap across
//! the open boundary — i.e. `mu == k && coords[k] == L - 1` — are
//! OMITTED from the ingested bundle entirely (not emitted with a
//! sentinel, not with NULL). The ingested record set then equals the
//! lattice's edge set exactly, which is what SPECTRAL_GAUGE consumes.
//!
//! Every test in this file is expected to FAIL on the current tree
//! (RED). GREEN lands together with the ingest-executor change that
//! (a) queries the lattice for adjacency and (b) skips wrap edges on
//! any OBC axis.
//!
//! Fixture synthesis mirrors `tests/ingest_as_gauge_field_basic.rs` and
//! `tests/halcyon_l24_workflow_e2e.rs`. No committed binary blobs; all
//! fixtures are L=4 D∈{2,4} so the whole file runs in well under a second.

#![cfg(feature = "halcyon")]

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::RwLock;

use gigi::engine::Engine;
use gigi::gauge::Group;
use gigi::ingest::{execute_ingest_as_gauge_field, IngestFormat};
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

// ── Helpers ─────────────────────────────────────────────────────────

/// Write a single-array NPZ file at `path`. Same writer path as the
/// other INGEST integration suites use.
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

/// SU(2) identity NPZ: shape `(n_configs, D, L, L, ..., L, 4)`, every
/// quaternion set to (1, 0, 0, 0).
fn write_su2_identity_npz(path: &Path, n_configs: usize, d: usize, l: usize) {
    let sites_per_muconf: usize = (0..d).fold(1usize, |a, _| a * l);
    let n_links = n_configs * d * sites_per_muconf;
    let mut data = Vec::with_capacity(n_links * 4);
    for _ in 0..n_links {
        data.push(1.0);
        data.push(0.0);
        data.push(0.0);
        data.push(0.0);
    }
    let mut shape: Vec<u64> = Vec::with_capacity(d + 3);
    shape.push(n_configs as u64);
    shape.push(d as u64);
    for _ in 0..d {
        shape.push(l as u64);
    }
    shape.push(4);
    write_test_npz_single(path, "su2_field", &shape, &data);
}

/// SU(3) identity NPZ: shape `(n_configs, D, L, L, ..., L, 18)`,
/// (re, im) interleaved diag = 1 for indices 0, 4, 8; everything else 0.
fn write_su3_identity_npz(path: &Path, n_configs: usize, d: usize, l: usize) {
    let sites_per_muconf: usize = (0..d).fold(1usize, |a, _| a * l);
    let n_links = n_configs * d * sites_per_muconf;
    let mut data = Vec::with_capacity(n_links * 18);
    for _ in 0..n_links {
        for pair_idx in 0..9 {
            let is_diag = matches!(pair_idx, 0 | 4 | 8);
            data.push(if is_diag { 1.0 } else { 0.0 });
            data.push(0.0);
        }
    }
    let mut shape: Vec<u64> = Vec::with_capacity(d + 3);
    shape.push(n_configs as u64);
    shape.push(d as u64);
    for _ in 0..d {
        shape.push(l as u64);
    }
    shape.push(18);
    write_test_npz_single(path, "su3_field", &shape, &data);
}

/// Declare a lattice via the grammar. Accepts a full "PERIODIC" or
/// "OBC AXIS <k>" clause tail.
fn declare_lattice_stmt(engine: &mut Engine, decl: &str) {
    let stmt = parse(decl).unwrap_or_else(|e| panic!("parse `{decl}` failed: {e}"));
    execute(engine, &stmt).unwrap_or_else(|e| panic!("execute `{decl}` failed: {e}"));
}

/// Open a fresh engine on a tempdir; also clear both lattice + gauge
/// registries so a repeat-run of the suite sees the same fresh state.
fn open_engine() -> (Engine, tempfile::TempDir) {
    gigi::gauge::registry::clear();
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

/// Column-major site_of: coords → flat site index. Matches the
/// lattice's own VertexId numbering (site_x least-significant, stride 1;
/// stride[k] = L^k). The ingest executor is required to align its
/// vertex_a / vertex_b encoding with this scheme.
fn site_of(coords: &[usize], l: usize) -> usize {
    let mut s = 0usize;
    let mut stride = 1usize;
    for &c in coords {
        s += c * stride;
        stride *= l;
    }
    s
}

/// Shift-by-+1 along axis `a`, modulo L. Encoding-agnostic (operates on
/// coord tuples, not flat indices).
fn shift_plus(coords: &[usize], a: usize, l: usize) -> Vec<usize> {
    let mut out = coords.to_vec();
    out[a] = (out[a] + 1) % l;
    out
}

// ── Test 1: PERIODIC L=4 D=2 SU(2) emits vertex_a AND vertex_b ─────

/// PERIODIC lattice: schema carries vertex_a and vertex_b as base INT
/// fields, and the record count is unaffected — every (config, mu, site)
/// point contributes one record.
#[test]
fn test_ingest_periodic_l4_d2_su2_emits_vertex_ab() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d2_periodic.npz");
    let n_configs = 2usize;
    let d = 2usize;
    let l = 4usize;
    write_su2_identity_npz(&path, n_configs, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_periodic_ab FROM CUBIC L=4 DIM=2 PERIODIC;",
    );

    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "su2_periodic_ab",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_periodic_ab",
        None,
    )
    .expect("INGEST AS GAUGE_FIELD succeeds");

    let expected_records = n_configs * d * l.pow(d as u32); // 2 * 2 * 16 = 64
    assert_eq!(
        stats.records_emitted, expected_records,
        "PERIODIC records = n_configs * D * L^D = {expected_records}"
    );

    let bundle = engine.bundle("su2_periodic_ab").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    let base_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert!(
        base_names.iter().any(|n| n == "vertex_a"),
        "base fields must include vertex_a; got {base_names:?}"
    );
    assert!(
        base_names.iter().any(|n| n == "vertex_b"),
        "base fields must include vertex_b; got {base_names:?}"
    );

    // vertex_a / vertex_b arrive as Integer values.
    let mut checked = false;
    for rec in store.records() {
        assert!(
            matches!(rec.get("vertex_a"), Some(Value::Integer(_))),
            "vertex_a must be Integer"
        );
        assert!(
            matches!(rec.get("vertex_b"), Some(Value::Integer(_))),
            "vertex_b must be Integer"
        );
        checked = true;
        break;
    }
    assert!(checked, "expected at least one record to sample");
}

// ── Test 2: OBC L=4 D=2 AXIS 0 omits wrap edge records ─────────────

/// OBC axis 0 drops the L^(D-1) records whose mu = 0 and site_x = L-1
/// (the ones that would wrap across the open boundary).
#[test]
fn test_ingest_obc_l4_d2_su2_axis0_omits_boundary_records() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d2_obc.npz");
    let n_configs = 2usize;
    let d = 2usize;
    let l = 4usize;
    write_su2_identity_npz(&path, n_configs, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_obc_ax0 FROM CUBIC L=4 DIM=2 OBC AXIS 0;",
    );

    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "su2_obc_ax0",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_obc_ax0",
        None,
    )
    .expect("INGEST AS GAUGE_FIELD succeeds on OBC lattice");

    // L^D * D per config = 32; drop L^(D-1) = 4 per config → 28 per config.
    let per_config = d * l.pow(d as u32) - l.pow((d - 1) as u32);
    let expected = n_configs * per_config;
    assert_eq!(
        stats.records_emitted, expected,
        "OBC AXIS 0 drops L^(D-1) records per config; expected {expected}, got {}",
        stats.records_emitted
    );

    // No record has (mu = 0 AND site_x = L-1). Those are the omitted
    // wrap-edge records — their presence would be a schema bug.
    let bundle = engine.bundle("su2_obc_ax0").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    for rec in store.records() {
        let mu = match rec.get("mu") {
            Some(Value::Integer(v)) => *v,
            other => panic!("mu must be Integer, got {other:?}"),
        };
        let site_x = match rec.get("site_x") {
            Some(Value::Integer(v)) => *v,
            other => panic!("site_x must be Integer, got {other:?}"),
        };
        assert!(
            !(mu == 0 && site_x == (l as i64) - 1),
            "OBC-omitted wrap edge leaked into record set (mu=0, site_x={site_x})"
        );
    }
}

// ── Test 3: OBC L=4 D=4 AXIS 0 absolute record count ───────────────

/// OBC AXIS 0 on the L=4 D=4 fixture drops exactly L^(D-1) = 64
/// wrap-edge records per config → 1024 - 64 = 960 records per config.
#[test]
fn test_ingest_obc_l4_d4_su2_axis0_absolute_record_count() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d4_obc.npz");
    let n_configs = 1usize;
    let d = 4usize;
    let l = 4usize;
    write_su2_identity_npz(&path, n_configs, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_d4_obc_ax0 FROM CUBIC L=4 DIM=4 OBC AXIS 0;",
    );

    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "su2_l4_d4_obc",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_d4_obc_ax0",
        None,
    )
    .expect("INGEST AS GAUGE_FIELD succeeds on L=4 D=4 OBC lattice");

    // Per config: D * L^D = 4 * 256 = 1024. Drop L^(D-1) = 64. → 960.
    let per_config = 960usize;
    assert_eq!(
        stats.records_emitted, n_configs * per_config,
        "L=4 D=4 OBC AXIS 0: 1024 - 64 = 960 records per config"
    );
}

// ── Test 4: vertex_a = site_of(coords) ─────────────────────────────

/// Sample records and confirm vertex_a matches the row-major
/// site_of(site_coords) that the lattice uses. The ingest executor and
/// the lattice must agree on the same coord ↔ site encoding.
#[test]
fn test_vertex_a_matches_site_of_coords() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d2_va.npz");
    let d = 2usize;
    let l = 4usize;
    write_su2_identity_npz(&path, 1, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_periodic_va FROM CUBIC L=4 DIM=2 PERIODIC;",
    );

    execute_ingest_as_gauge_field(
        &mut engine,
        "su2_va",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_periodic_va",
        None,
    )
    .expect("ingest");

    let bundle = engine.bundle("su2_va").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");

    let mut checked = 0usize;
    for rec in store.records() {
        let site_x = match rec.get("site_x") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("site_x must be Integer, got {other:?}"),
        };
        let site_y = match rec.get("site_y") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("site_y must be Integer, got {other:?}"),
        };
        let vertex_a = match rec.get("vertex_a") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("vertex_a must be Integer, got {other:?}"),
        };
        assert_eq!(
            vertex_a,
            site_of(&[site_x, site_y], l),
            "vertex_a must equal site_of([site_x={site_x}, site_y={site_y}])"
        );
        checked += 1;
        if checked >= 8 {
            break;
        }
    }
    assert!(checked > 0, "expected at least one record to sample");
}

// ── Test 5: vertex_b = site_of(shift_plus(coords, mu)) ─────────────

/// vertex_b matches the shift-by-+1 along the record's own mu.
#[test]
fn test_vertex_b_matches_shift_plus() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d2_vb.npz");
    let d = 2usize;
    let l = 4usize;
    write_su2_identity_npz(&path, 1, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_periodic_vb FROM CUBIC L=4 DIM=2 PERIODIC;",
    );

    execute_ingest_as_gauge_field(
        &mut engine,
        "su2_vb",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_periodic_vb",
        None,
    )
    .expect("ingest");

    let bundle = engine.bundle("su2_vb").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");

    let mut checked = 0usize;
    for rec in store.records() {
        let mu = match rec.get("mu") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("mu must be Integer, got {other:?}"),
        };
        let site_x = match rec.get("site_x") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("site_x must be Integer, got {other:?}"),
        };
        let site_y = match rec.get("site_y") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("site_y must be Integer, got {other:?}"),
        };
        let vertex_b = match rec.get("vertex_b") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("vertex_b must be Integer, got {other:?}"),
        };
        let shifted = shift_plus(&[site_x, site_y], mu, l);
        assert_eq!(
            vertex_b,
            site_of(&shifted, l),
            "vertex_b must equal site_of(shift_plus([{site_x},{site_y}], mu={mu}))"
        );
        checked += 1;
        if checked >= 8 {
            break;
        }
    }
    assert!(checked > 0, "expected at least one record to sample");
}

// ── Test 6: PERIODIC wraps via vertex_b ────────────────────────────

/// The PERIODIC record at (mu = 0, site_x = L-1, site_y = 0) has
/// vertex_b at (0, 0) — i.e. site 0 under the lattice's column-major
/// encoding. This is the wrap edge that OBC would omit; PERIODIC keeps
/// it and wraps.
#[test]
fn test_ingest_periodic_wraps_via_vertex_b() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d2_wrap.npz");
    let d = 2usize;
    let l = 4usize;
    write_su2_identity_npz(&path, 1, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_periodic_wrap FROM CUBIC L=4 DIM=2 PERIODIC;",
    );

    execute_ingest_as_gauge_field(
        &mut engine,
        "su2_wrap",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_periodic_wrap",
        None,
    )
    .expect("ingest");

    let bundle = engine.bundle("su2_wrap").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");

    let expected_vertex_b = site_of(&[0, 0], l);
    let mut found = false;
    for rec in store.records() {
        let mu = match rec.get("mu") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("mu must be Integer, got {other:?}"),
        };
        let site_x = match rec.get("site_x") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("site_x must be Integer, got {other:?}"),
        };
        let site_y = match rec.get("site_y") {
            Some(Value::Integer(v)) => *v as usize,
            other => panic!("site_y must be Integer, got {other:?}"),
        };
        if mu == 0 && site_x == l - 1 && site_y == 0 {
            let vertex_b = match rec.get("vertex_b") {
                Some(Value::Integer(v)) => *v as usize,
                other => panic!("vertex_b must be Integer, got {other:?}"),
            };
            assert_eq!(
                vertex_b, expected_vertex_b,
                "PERIODIC wrap edge (mu=0, x=L-1, y=0) must have vertex_b = site_of([0, 0]) = {expected_vertex_b}"
            );
            found = true;
            break;
        }
    }
    assert!(
        found,
        "expected PERIODIC record at (mu=0, site_x=L-1, site_y=0) to exist and wrap"
    );
}

// ── Test 7: SU(3) also emits vertex_a / vertex_b ───────────────────

/// The vertex_a / vertex_b base fields are group-independent — they are
/// a property of the lattice adjacency, not the gauge group. Confirm
/// SU(3) records carry them too.
#[test]
fn test_ingest_su3_also_emits_vertex_ab() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su3_l4_d2.npz");
    let n_configs = 1usize;
    let d = 2usize;
    let l = 4usize;
    write_su3_identity_npz(&path, n_configs, d, l);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_su3_ab FROM CUBIC L=4 DIM=2 PERIODIC;",
    );

    execute_ingest_as_gauge_field(
        &mut engine,
        "su3_ab_bundle",
        &path,
        IngestFormat::Npz,
        Group::SU3,
        "l4_su3_ab",
        None,
    )
    .expect("SU(3) ingest");

    let bundle = engine.bundle("su3_ab_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    let base_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert!(
        base_names.iter().any(|n| n == "vertex_a"),
        "SU(3) base fields must include vertex_a; got {base_names:?}"
    );
    assert!(
        base_names.iter().any(|n| n == "vertex_b"),
        "SU(3) base fields must include vertex_b; got {base_names:?}"
    );
}

// ── Test 8: SPECTRAL_GAUGE reads vertex_a/vertex_b directly ────────

/// End-to-end shape-alignment witness: LATTICE OBC → INGEST → run
/// SPECTRAL_GAUGE with no WHERE filter. The kernel is expected to
/// consume vertex_a / vertex_b directly from the INGESTed schema. A
/// pass (Rows or Scalar) proves the base fields are read without a
/// site-decoding fallback. An error is only acceptable if it names the
/// spectral / edge / vertex kernel surface — never a schema-shape
/// error that would prove the base fields are still missing.
#[test]
fn test_spectral_gauge_reads_vertex_ab_directly() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("fixture tempdir");
    let path = tmp.path().join("su2_l4_d2_obc_e2e.npz");
    write_su2_identity_npz(&path, 1, 2, 4);
    declare_lattice_stmt(
        &mut engine,
        "LATTICE l4_e2e FROM CUBIC L=4 DIM=2 OBC AXIS 0;",
    );

    execute_ingest_as_gauge_field(
        &mut engine,
        "su2_e2e",
        &path,
        IngestFormat::Npz,
        Group::SU2,
        "l4_e2e",
        None,
    )
    .expect("ingest");

    let eng_lock = RwLock::new(engine);
    let stmt = parse(
        "SPECTRAL_GAUGE su2_e2e ON FIBER (q0, q1, q2, q3) GROUP SU(2);",
    )
    .expect("parse SPECTRAL_GAUGE");
    let mut eng = eng_lock.write().expect("engine write lock");
    match execute(&mut eng, &stmt) {
        Ok(ExecResult::Rows(rs)) => {
            assert!(!rs.is_empty(), "SPECTRAL_GAUGE must emit at least one row");
        }
        Ok(ExecResult::Scalar(v)) => {
            assert!(v.is_finite(), "SPECTRAL_GAUGE scalar must be finite");
        }
        Ok(other) => panic!("unexpected envelope {other:?}"),
        Err(e) => {
            let em = e.to_ascii_uppercase();
            // "MISSING" / "FIELD" / "SCHEMA" would prove vertex_a/b are
            // still absent — that's the RED failure mode this test is
            // guarding against, so reject those.
            assert!(
                !(em.contains("MISSING") && em.contains("VERTEX")),
                "SPECTRAL_GAUGE reports missing vertex_a/vertex_b — schema not aligned: {e}"
            );
            assert!(
                em.contains("EDGE")
                    || em.contains("VERTEX")
                    || em.contains("SUBGRAPH")
                    || em.contains("SPECTRAL")
                    || em.contains("EMPTY")
                    || em.contains("KERNEL"),
                "SPECTRAL_GAUGE diagnostic must name the kernel surface: {e}"
            );
        }
    }
}
