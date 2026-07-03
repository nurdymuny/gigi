//! INGEST NPZ dtype auto-detect.
//!
//! The NPZ readers in `src/ingest.rs` read the `.npy` header dtype and
//! branch on element type: f64 members are read as-is; f32 members are
//! read as f32 then upconverted element-wise to f64 (mathematically
//! lossless — every f32 value has an exact f64 representation). Any
//! other element type surfaces `IngestError::FormatError` naming both
//! the observed dtype and the supported set (float32, float64).
//!
//! `test_ingest_npz_f64_unchanged` is the backward-compat anchor for
//! the pre-existing f64 path.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use gigi::engine::Engine;
use gigi::ingest::{execute_ingest, IngestError, IngestFormat};
use gigi::types::{FieldType, Value};

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

// GIGI_INGEST_DIR gate: sources are root-relative now.
mod common;

// ── Helpers ─────────────────────────────────────────────────────────

/// Write a single-array f32 NPZ. Same writer surface as the existing
/// fixture helpers in `tests/ingest_executor.rs` and
/// `tests/ingest_as_gauge_field_basic.rs`, differing only in the element
/// type parameter.
fn write_test_npz_f32(path: &Path, array_name: &str, shape: &[u64], data: &[f32]) {
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
        let builder = npz.array::<f32>(array_name, opts).expect("start array");
        let mut writer = builder
            .default_dtype()
            .shape(shape)
            .begin_nd()
            .expect("begin_nd");
        for &v in data {
            writer.push(&v).expect("push f32");
        }
        writer.finish().expect("finish array");
    }
    npz.zip_writer().finish().expect("finish zip");
}

/// Write a single-array f64 NPZ (backward-compat fixture).
fn write_test_npz_f64(path: &Path, array_name: &str, shape: &[u64], data: &[f64]) {
    let expected_len: u64 = shape.iter().product();
    assert_eq!(data.len() as u64, expected_len);
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

/// Write a single-array int32 NPZ — the unsupported-dtype path.
fn write_test_npz_i32(path: &Path, array_name: &str, shape: &[u64], data: &[i32]) {
    let expected_len: u64 = shape.iter().product();
    assert_eq!(data.len() as u64, expected_len);
    let file = File::create(path).expect("create test NPZ");
    let mut npz = NpzWriter::new(BufWriter::new(file));
    {
        let opts = npyz::zip::write::FileOptions::default()
            .compression_method(npyz::zip::CompressionMethod::Stored);
        let builder = npz.array::<i32>(array_name, opts).expect("start array");
        let mut writer = builder
            .default_dtype()
            .shape(shape)
            .begin_nd()
            .expect("begin_nd");
        for &v in data {
            writer.push(&v).expect("push i32");
        }
        writer.finish().expect("finish array");
    }
    npz.zip_writer().finish().expect("finish zip");
}

fn open_engine() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

// ── dtype auto-detect tests ────────────────────────────────────────

/// f32 NPZ auto-upconverts to f64 on the AUTO_GENERIC INGEST path.
/// The reader reads the `.npy` dtype header, sees f32, upconverts every
/// element to f64, and lands records with the exact expected values.
///
/// Values are all exactly representable in f32 (small integers), so
/// f32 → f64 cast is bit-exact.
#[test]
fn test_ingest_npz_f32_upconverts_to_f64() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path: PathBuf = tmp.path().join("f32_small.npz");

    // 5 rows × 4 cols of exactly-representable f32 values.
    let data_f32: Vec<f32> = (0..20).map(|i| i as f32).collect();
    write_test_npz_f32(&path, "f32_arr", &[5, 4], &data_f32);

    let stats = execute_ingest(
        &mut engine,
        "f32_bundle",
        &common::ingest_rel(&path),
        IngestFormat::Npz,
        None,
    )
    .expect("INGEST succeeds on f32 NPZ with auto-detect + upconvert");

    assert_eq!(stats.records_emitted, 5, "5 outer-axis slices");
    assert!(stats.bundle_created);

    let bundle = engine.bundle("f32_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 5);
    // Emitted schema is a Vector(dims=4) fiber under the array's name.
    match &store.schema.fiber_fields[0].field_type {
        FieldType::Vector { dims } => assert_eq!(*dims, 4),
        other => panic!("expected Vector(dims=4), got {other:?}"),
    }

    // Collect every emitted vector and compare against the source data
    // upconverted the same way (`f32 as f64`). Cast is bit-exact for the
    // values we used, so equality is strict.
    let mut got: Vec<(i64, Vec<f64>)> = Vec::new();
    for rec in store.records() {
        let idx = match rec.get("row_idx") {
            Some(Value::Integer(i)) => *i,
            _ => panic!("record missing row_idx"),
        };
        let v = match rec.get("f32_arr") {
            Some(Value::Vector(v)) => v.clone(),
            _ => panic!("record missing f32_arr vector"),
        };
        got.push((idx, v));
    }
    got.sort_by_key(|(i, _)| *i);
    for (row_idx, v) in &got {
        assert_eq!(v.len(), 4);
        for (col, val) in v.iter().enumerate() {
            let expected = ((*row_idx as usize) * 4 + col) as f32 as f64;
            assert_eq!(*val, expected, "row {row_idx} col {col}");
        }
    }
}

/// f64 NPZ still ingests unchanged — this is the backward-compat anchor
/// that MUST pass at both RED and GREEN. Under the RED tree it exercises
/// the existing `into_vec::<f64>()` branch; under GREEN it exercises the
/// f64 arm of the dtype match.
#[test]
fn test_ingest_npz_f64_unchanged() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("f64_small.npz");

    // 5 rows × 4 cols of f64 values chosen so the exact bit pattern
    // matters — non-integer with a mantissa that would round differently
    // in f32.
    let data: Vec<f64> = (0..20).map(|i| (i as f64) * 0.1 + 1.234567890123).collect();
    write_test_npz_f64(&path, "f64_arr", &[5, 4], &data);

    let stats = execute_ingest(
        &mut engine,
        "f64_bundle",
        &common::ingest_rel(&path),
        IngestFormat::Npz,
        None,
    )
    .expect("INGEST succeeds on f64 NPZ (backward compat)");
    assert_eq!(stats.records_emitted, 5);

    let bundle = engine.bundle("f64_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    // Bit-exact roundtrip: emitted values equal the source Vec<f64>.
    let mut got: Vec<(i64, Vec<f64>)> = Vec::new();
    for rec in store.records() {
        let idx = match rec.get("row_idx") {
            Some(Value::Integer(i)) => *i,
            _ => panic!("missing row_idx"),
        };
        let v = match rec.get("f64_arr") {
            Some(Value::Vector(v)) => v.clone(),
            _ => panic!("missing f64_arr"),
        };
        got.push((idx, v));
    }
    got.sort_by_key(|(i, _)| *i);
    for (row_idx, v) in &got {
        for (col, val) in v.iter().enumerate() {
            let src = data[(*row_idx as usize) * 4 + col];
            assert_eq!(val.to_bits(), src.to_bits(), "row {row_idx} col {col}");
        }
    }
}

/// int32 NPZ errors with a clear message that names the dtype and the
/// supported set. The GREEN reader inspects the `.npy` header first, so
/// the message references the actual dtype string (e.g. "int32" or "<i4")
/// rather than the generic `npyz` decode error the RED path surfaces.
#[test]
fn test_ingest_npz_int32_errors_with_dtype_name() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("i32_arr.npz");
    let data: Vec<i32> = (0..12).collect();
    write_test_npz_i32(&path, "i32_arr", &[3, 4], &data);

    let err = execute_ingest(
        &mut engine,
        "i32_bundle",
        &common::ingest_rel(&path),
        IngestFormat::Npz,
        None,
    )
    .expect_err("INGEST must reject int32 with a dtype-aware error");
    let msg = err.to_string();
    let low = msg.to_ascii_lowercase();
    assert!(
        low.contains("int32") || low.contains("i4") || low.contains("<i4"),
        "error should name the observed dtype (int32 / <i4): {msg}"
    );
    assert!(
        low.contains("float32") || low.contains("float64") || low.contains("f4") || low.contains("f8"),
        "error should name the supported dtypes (float32/float64): {msg}"
    );
    // The error is either FormatError (recommended) or a new variant —
    // both flow through Display so the message contract is what we test.
    // A wrapped-npyz FormatError from `into_vec::<f64>()` at RED would
    // typically say "type mismatch" without naming the dtype string, so
    // this assertion fails until the header-first branch lands.
    match err {
        IngestError::FormatError(_) => (),
        other => panic!("expected FormatError with dtype name, got {other:?}"),
    }
}

/// Combined dtype auto-detect + vertex-emission: SU(2) f32 NPZ on an
/// OBC L=4 D=2 lattice runs through INGEST AS GAUGE_FIELD, upconverts
/// f32 to f64, AND emits vertex_a/vertex_b via lattice adjacency with
/// the OBC-wrap records omitted. Exercises the full stack: dtype
/// branch, lattice adjacency lookup, and OBC record omission.
#[cfg(feature = "lattice")]
#[test]
fn test_ingest_npz_f32_gauge_field_su2_full_chain() {
    use gigi::gauge::Group;
    use gigi::ingest::execute_ingest_as_gauge_field;
    use gigi::parser;

    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("su2_f32_l4.npz");

    // Shape (n_configs=1, D=2, L=4, L=4, fiber=4). Identity quaternion
    // (1, 0, 0, 0) at every link.
    let n_configs = 1usize;
    let d = 2usize;
    let l = 4usize;
    let n_links = n_configs * d * l.pow(d as u32);
    let mut data: Vec<f32> = Vec::with_capacity(n_links * 4);
    for _ in 0..n_links {
        data.push(1.0_f32); // q0
        data.push(0.0_f32); // q1
        data.push(0.0_f32); // q2
        data.push(0.0_f32); // q3
    }
    write_test_npz_f32(
        &path,
        "u_field",
        &[n_configs as u64, d as u64, l as u64, l as u64, 4],
        &data,
    );

    // OBC lattice on axis 0 via the `OBC AXIS <k>` grammar.
    let decl = format!(
        "LATTICE l4_obc_dtype FROM CUBIC L={l} DIM={d} OBC AXIS 0;"
    );
    let stmt = parser::parse(&decl).unwrap_or_else(|e| panic!("parse: {e}"));
    parser::execute(&mut engine, &stmt).unwrap_or_else(|e| panic!("execute: {e}"));

    // INGEST AS GAUGE_FIELD on the f32 NPZ. GREEN: dtype auto-detected
    // f32 → upconverted to f64 → lattice adjacency → vertex_a/vertex_b
    // populated → OBC-wrap records omitted.
    let stats = execute_ingest_as_gauge_field(
        &mut engine,
        "su2_f32_bundle",
        &common::ingest_rel(&path),
        IngestFormat::Npz,
        Group::SU2,
        "l4_obc_dtype",
        None,
    )
    .expect("INGEST AS GAUGE_FIELD succeeds on f32 SU(2) NPZ over OBC lattice");

    // OBC axis 0 drops (L-1)^0 × L^(D-1) = L = 4 wrap edges per (config, direction=axis-0).
    // Non-OBC-axis directions keep all L^D = 16 edges.
    //   direction 0 (OBC): 16 sites - 4 wrap = 12 edges per config
    //   direction 1 (periodic-in-OBC-context, but coord=L-1 in direction 1
    //     is still fine; only mu==obc_axis && coord[mu]==L-1 gets dropped).
    //     16 edges per config.
    // Total per config = 12 + 16 = 28. n_configs=1 → 28 records total.
    assert_eq!(
        stats.records_emitted, 28,
        "OBC-omitted records: 16 per direction, minus 4 wrap edges in the OBC axis"
    );

    let bundle = engine.bundle("su2_f32_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    let base_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    // Base fields = [config_id, mu, site_x, site_y, vertex_a, vertex_b].
    assert!(
        base_names.contains(&"vertex_a".to_string()),
        "base fields include vertex_a; got {base_names:?}"
    );
    assert!(
        base_names.contains(&"vertex_b".to_string()),
        "base fields include vertex_b; got {base_names:?}"
    );

    // Every record should carry a q0 = 1.0 (identity), with the value
    // materializing exactly under f32 → f64 cast.
    let mut sample_seen = false;
    for rec in store.records() {
        if let Some(Value::Float(q0)) = rec.get("q0") {
            assert_eq!(*q0, 1.0_f32 as f64);
            sample_seen = true;
            break;
        }
    }
    assert!(sample_seen, "at least one identity record with q0=1.0");
}
