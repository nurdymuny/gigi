//! Halcyon ITEM 3.2 — INGEST executor integration tests.
//!
//! End-to-end coverage that the NPZ ingest path lands records into
//! the gigi engine in the shape the Halcyon harvest pipeline expects.
//! Every fixture is synthesized in-process via `npyz` itself — no
//! committed binary blobs, no Python dependency.
//!
//! Test matrix:
//!
//! 1. `test_ingest_npz_small_float64_array_creates_bundle` —
//!    auto-create from a 10x4 array and assert 10 records.
//! 2. `test_ingest_npz_with_existing_compatible_bundle` — pre-create
//!    the bundle with the matching schema; assert records append
//!    without auto-create.
//! 3. `test_ingest_npz_with_existing_conflicting_bundle` — pre-create
//!    with an incompatible schema; assert `SchemaConflict`.
//! 4. `test_ingest_npz_file_not_found` — return `FileNotFound`.
//! 5. `test_ingest_npz_unsupported_format` — return
//!    `FormatNotSupported`.
//! 6. `test_ingest_npz_4d_array_record_count` — a `(3,3,3,3,9)`
//!    array stands in for the Halcyon `(L=12,12,12,12,9)` shape and
//!    validates the outer-axis record count + per-record vector
//!    length.
//! 7. `test_ingest_parser_end_to_end` — drive the full
//!    `INGEST … FROM '…' FORMAT NPZ;` statement through the parser
//!    executor and verify the engine has the records.
//! 8. `test_ingest_multi_array_npz` — multi-member archive tags each
//!    record with `array_name`.
//! 9. `test_ingest_npz_virtual_bundle_rejected` — `INGEST` into
//!    `__bundles__` is rejected by the virtual-bundle guard.

use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use gigi::engine::Engine;
use gigi::ingest::{execute_ingest, IngestError, IngestFormat};
use gigi::types::{BundleSchema, FieldDef, FieldType, Value};

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

/// Write a single-array NPZ file to `path`. The array has the given
/// shape and elements stored in row-major (C) order. Pure Rust — uses
/// the `npyz` crate that the executor itself depends on, so the
/// fixture exercises the same surface.
fn write_test_npz_single(
    path: &Path,
    array_name: &str,
    shape: &[u64],
    data: &[f64],
) {
    let expected_len: u64 = shape.iter().product();
    assert_eq!(
        data.len() as u64, expected_len,
        "fixture mismatch: shape product {} != data len {}",
        expected_len,
        data.len()
    );
    let file = File::create(path).expect("create test NPZ");
    let mut npz = NpzWriter::new(BufWriter::new(file));
    {
        let opts = npyz::zip::write::FileOptions::default()
            .compression_method(npyz::zip::CompressionMethod::Stored);
        let builder = npz
            .array::<f64>(array_name, opts)
            .expect("start array");
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

/// Write a two-array NPZ for the multi-array path.
fn write_test_npz_multi(
    path: &Path,
    arrays: &[(&str, &[u64], &[f64])],
) {
    let file = File::create(path).expect("create test NPZ");
    let mut npz = NpzWriter::new(BufWriter::new(file));
    for (name, shape, data) in arrays {
        let expected_len: u64 = shape.iter().product();
        assert_eq!(
            data.len() as u64, expected_len,
            "fixture mismatch on `{}`",
            name
        );
        let opts = npyz::zip::write::FileOptions::default()
            .compression_method(npyz::zip::CompressionMethod::Stored);
        let builder = npz
            .array::<f64>(name, opts)
            .expect("start array");
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

fn open_engine() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

#[test]
fn test_ingest_npz_small_float64_array_creates_bundle() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path: PathBuf = tmp.path().join("small.npz");

    // 10 rows × 4 cols of monotone-increasing f64.
    let data: Vec<f64> = (0..40).map(|i| i as f64).collect();
    write_test_npz_single(&path, "small", &[10, 4], &data);

    let stats =
        execute_ingest(&mut engine, "small_bundle", &path, IngestFormat::Npz)
            .expect("ingest succeeds");
    assert_eq!(stats.records_emitted, 10, "10 outer-axis slices");
    assert!(stats.bundle_created, "bundle auto-created when missing");
    assert!(stats.bytes_read > 0, "bytes_read populated from file size");

    // Verify the engine actually has those records under the expected
    // schema (row_idx Numeric base + `small` Vector(dims=4) fiber).
    let bundle = engine.bundle("small_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 10, "bundle holds 10 records");
    assert_eq!(store.schema.base_fields.len(), 1);
    assert_eq!(store.schema.base_fields[0].name, "row_idx");
    assert_eq!(store.schema.fiber_fields.len(), 1);
    assert_eq!(store.schema.fiber_fields[0].name, "small");
    match &store.schema.fiber_fields[0].field_type {
        FieldType::Vector { dims } => assert_eq!(*dims, 4, "Vector(dims=4)"),
        other => panic!("expected Vector(dims=4), got {other:?}"),
    }
}

#[test]
fn test_ingest_npz_with_existing_compatible_bundle() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("compat.npz");

    // Pre-create the bundle with the schema INGEST will infer.
    let mut vec_field = FieldDef::numeric("compat");
    vec_field.field_type = FieldType::Vector { dims: 3 };
    let schema = BundleSchema::new("compat_bundle")
        .base(FieldDef::numeric("row_idx"))
        .fiber(vec_field);
    engine.create_bundle(schema).expect("create_bundle");

    let data: Vec<f64> = (0..15).map(|i| i as f64).collect();
    write_test_npz_single(&path, "compat", &[5, 3], &data);

    let stats = execute_ingest(
        &mut engine,
        "compat_bundle",
        &path,
        IngestFormat::Npz,
    )
    .expect("ingest succeeds");
    assert_eq!(stats.records_emitted, 5);
    assert!(!stats.bundle_created, "existing bundle is reused");

    let bundle = engine.bundle("compat_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 5);
}

#[test]
fn test_ingest_npz_with_existing_conflicting_bundle() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("conflict.npz");

    // Pre-create bundle with a WRONG type on the array field (Numeric
    // instead of Vector). INGEST must reject.
    let schema = BundleSchema::new("conflict_bundle")
        .base(FieldDef::numeric("row_idx"))
        .fiber(FieldDef::numeric("conflict"));
    engine.create_bundle(schema).expect("create_bundle");

    let data: Vec<f64> = (0..6).map(|i| i as f64).collect();
    write_test_npz_single(&path, "conflict", &[2, 3], &data);

    let err = execute_ingest(
        &mut engine,
        "conflict_bundle",
        &path,
        IngestFormat::Npz,
    )
    .expect_err("should fail on incompatible schema");
    match err {
        IngestError::SchemaConflict { bundle, field, .. } => {
            assert_eq!(bundle, "conflict_bundle");
            assert_eq!(field, "conflict");
        }
        other => panic!("expected SchemaConflict, got {other:?}"),
    }
}

#[test]
fn test_ingest_npz_file_not_found() {
    let (mut engine, _dir) = open_engine();
    let phantom = PathBuf::from("does-not-exist-anywhere.npz");
    let err = execute_ingest(&mut engine, "ghost", &phantom, IngestFormat::Npz)
        .expect_err("missing file");
    match err {
        IngestError::FileNotFound(p) => assert_eq!(p, phantom),
        other => panic!("expected FileNotFound, got {other:?}"),
    }
}

#[test]
fn test_ingest_npz_unsupported_format() {
    let err = IngestFormat::from_name("CSV").expect_err("unsupported");
    match err {
        IngestError::FormatNotSupported { requested, supported } => {
            assert_eq!(requested, "CSV");
            assert_eq!(supported, vec!["NPZ".to_string()]);
        }
        other => panic!("expected FormatNotSupported, got {other:?}"),
    }
}

#[test]
fn test_ingest_npz_4d_array_record_count() {
    // Stand-in for the Halcyon (12,12,12,12,9) harvest shape — the
    // (3,3,3,3,9) version exercises the exact same code path with a
    // negligibly small footprint.
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("halcyon_smoke.npz");

    let shape = [3u64, 3, 3, 3, 9];
    let total: usize = shape.iter().map(|n| *n as usize).product();
    let data: Vec<f64> = (0..total).map(|i| i as f64).collect();
    write_test_npz_single(&path, "halcyon_smoke", &shape, &data);

    let stats = execute_ingest(
        &mut engine,
        "halcyon_smoke_bundle",
        &path,
        IngestFormat::Npz,
    )
    .expect("ingest succeeds");

    // Outer axis = 3 → 3 records. Inner length = 3 * 3 * 3 * 9 = 243.
    assert_eq!(stats.records_emitted, 3, "3 outer-axis slices");

    let bundle = engine
        .bundle("halcyon_smoke_bundle")
        .expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 3, "bundle holds 3 records");
    match &store.schema.fiber_fields[0].field_type {
        FieldType::Vector { dims } => assert_eq!(*dims, 3 * 3 * 3 * 9),
        other => panic!("expected Vector(dims=243), got {other:?}"),
    }

    // Verify at least one record carries the expected vector and
    // row_idx.
    let mut row_indices: Vec<i64> = Vec::new();
    let mut vec_lens: Vec<usize> = Vec::new();
    for rec in store.records() {
        if let Some(Value::Integer(i)) = rec.get("row_idx") {
            row_indices.push(*i);
        }
        if let Some(Value::Vector(v)) = rec.get("halcyon_smoke") {
            vec_lens.push(v.len());
        }
    }
    row_indices.sort();
    assert_eq!(row_indices, vec![0, 1, 2]);
    assert_eq!(vec_lens, vec![243, 243, 243]);
}

#[test]
fn test_ingest_parser_end_to_end() {
    // Drive the full INGEST statement through the parser executor,
    // not just the direct function call. This is the gate that proves
    // the parser arm at parser.rs:9485 is correctly wired to the new
    // module.
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("parser_e2e.npz");
    let data: Vec<f64> = (0..20).map(|i| i as f64).collect();
    write_test_npz_single(&path, "parser_e2e", &[5, 4], &data);

    // GQL string literal needs forward slashes on Windows so the
    // parser's Token::Str round-trip is portable.
    let path_str = path.to_string_lossy().replace('\\', "/");
    let stmt_src = format!(
        "INGEST e2e_bundle FROM '{}' FORMAT NPZ;",
        path_str
    );
    let stmt = gigi::parser::parse(&stmt_src)
        .unwrap_or_else(|e| panic!("parse failed: {e}"));
    let result = gigi::parser::execute(&mut engine, &stmt)
        .unwrap_or_else(|e| panic!("execute failed: {e}"));
    assert_eq!(result, gigi::parser::ExecResult::Ok);

    let bundle = engine.bundle("e2e_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 5, "5 records after parser-driven INGEST");
}

#[test]
fn test_ingest_multi_array_npz() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("multi.npz");

    let a: Vec<f64> = (0..6).map(|i| i as f64).collect();
    let b: Vec<f64> = (0..9).map(|i| -(i as f64)).collect();
    write_test_npz_multi(
        &path,
        &[
            ("a", &[2, 3], &a),
            ("b", &[3, 3], &b),
        ],
    );

    let stats =
        execute_ingest(&mut engine, "multi_bundle", &path, IngestFormat::Npz)
            .expect("ingest succeeds");
    // 2 outer slices in `a` + 3 outer slices in `b` = 5 records.
    assert_eq!(stats.records_emitted, 5);
    assert!(stats.bundle_created);

    let bundle = engine.bundle("multi_bundle").expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 5);

    // Schema should carry row_idx + array_name + a + b.
    let all_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .chain(store.schema.fiber_fields.iter())
        .map(|f| f.name.clone())
        .collect();
    assert!(all_names.contains(&"row_idx".to_string()));
    assert!(all_names.contains(&"array_name".to_string()));
    assert!(all_names.contains(&"a".to_string()));
    assert!(all_names.contains(&"b".to_string()));

    // Verify array_name distribution.
    let mut name_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for rec in store.records() {
        if let Some(Value::Text(s)) = rec.get("array_name") {
            *name_counts.entry(s.clone()).or_insert(0) += 1;
        }
    }
    assert_eq!(name_counts.get("a").copied().unwrap_or(0), 2);
    assert_eq!(name_counts.get("b").copied().unwrap_or(0), 3);
}

#[test]
fn test_ingest_npz_virtual_bundle_rejected() {
    // INGEST into the reserved `__bundles__` virtual bundle must
    // fail at the parser entry, BEFORE the executor touches the
    // file. This ensures the read-only-virtual-bundle policy is
    // uniform across every write verb.
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("virt.npz");
    let data: Vec<f64> = vec![1.0, 2.0];
    write_test_npz_single(&path, "virt", &[1, 2], &data);

    let path_str = path.to_string_lossy().replace('\\', "/");
    let stmt_src = format!(
        "INGEST __bundles__ FROM '{}' FORMAT NPZ;",
        path_str
    );
    let stmt = gigi::parser::parse(&stmt_src).expect("parse");
    let err = gigi::parser::execute(&mut engine, &stmt)
        .expect_err("must reject virtual bundle");
    assert!(
        err.contains("__bundles__") && err.contains("virtual"),
        "expected virtual-bundle reject, got: {err}"
    );
}
