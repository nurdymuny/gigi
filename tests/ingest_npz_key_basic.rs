//! INGEST NPZ KEY <name> — integration tests.
//!
//! Pins the parser + executor contract for the `KEY <name>` clause on
//! `INGEST ... FORMAT NPZ ...`. The clause names a single member of a
//! multi-array NPZ archive so the caller can point at exactly the array
//! whose slices should become records.
//!
//! Grammar (unambiguous placement between `FORMAT NPZ` and any `AS`
//! clause or terminating `;`):
//!
//! ```text
//! INGEST <bundle> FROM '<path>' FORMAT NPZ KEY <name>
//!     [AS GAUGE_FIELD GROUP <group> ON LATTICE <lattice>] ;
//! ```
//!
//! Semantics:
//!
//! - `KEY <name>` selects exactly the named array from the archive; the
//!   rest of the archive's members are ignored.
//! - `KEY` absent + multi-member archive: preserved historical behavior
//!   (generic path emits multi-array records; the `AS GAUGE_FIELD` path
//!   errors with `MultiArrayNotAllowedForGaugeField`). This file
//!   exercises the generic path only; `AS GAUGE_FIELD` coverage lives
//!   in the vertex-emission test file.
//! - `KEY` absent + single-member archive: backward-compatible — the one
//!   member is used automatically.
//! - `KEY <name>` names a member not in the archive: hard error naming
//!   the requested key AND the archive's actual member names, so the
//!   caller can correct the statement without re-inspecting the file.
//!
//! Run with:
//!   `cargo test --test ingest_npz_key_basic`

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

/// Write a multi-array NPZ file with the given (name, shape, data) list.
fn write_test_npz_multi(
    path: &Path,
    arrays: &[(&str, &[u64], &[f64])],
) {
    let file = File::create(path).expect("create test NPZ");
    let mut npz = NpzWriter::new(BufWriter::new(file));
    for (name, shape, data) in arrays {
        let expected_len: u64 = shape.iter().product();
        assert_eq!(
            data.len() as u64,
            expected_len,
            "fixture mismatch on `{}`",
            name
        );
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

/// Write a single-array NPZ file.
fn write_test_npz_single(
    path: &Path,
    array_name: &str,
    shape: &[u64],
    data: &[f64],
) {
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

fn open_engine() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

/// Test 1 — KEY <name> selects the named member from a multi-member NPZ.
///
/// The archive holds `U` (2x3), `q_clover` (4x2), `q_rounded` (4x2).
/// `KEY U` is specified. The bundle emits exactly the slices from `U`
/// (2 records, one per outer-axis row), with the inner Vector fiber
/// dimension matching `U`'s inner shape (3). The other members do not
/// contribute records and do not appear in the emitted schema.
#[test]
fn test_ingest_npz_key_selects_named_array() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("multi_with_key.npz");

    let u_data: Vec<f64> = (0..6).map(|i| i as f64).collect();
    let q_clover_data: Vec<f64> = (0..8).map(|i| -(i as f64)).collect();
    let q_rounded_data: Vec<f64> = (0..8).map(|i| (i as f64) * 0.5).collect();
    write_test_npz_multi(
        &path,
        &[
            ("U", &[2, 3], &u_data),
            ("q_clover", &[4, 2], &q_clover_data),
            ("q_rounded", &[4, 2], &q_rounded_data),
        ],
    );

    let path_str = path.to_string_lossy().replace('\\', "/");
    let stmt_src = format!(
        "INGEST key_selected_bundle FROM '{}' FORMAT NPZ KEY U;",
        path_str
    );
    let stmt = parse(&stmt_src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    let result = execute(&mut engine, &stmt)
        .unwrap_or_else(|e| panic!("execute failed: {e}"));
    assert_eq!(result, ExecResult::Ok);

    let bundle = engine
        .bundle("key_selected_bundle")
        .expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(
        store.len(),
        2,
        "KEY U selects U's 2 outer-axis slices only"
    );

    let all_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .chain(store.schema.fiber_fields.iter())
        .map(|f| f.name.clone())
        .collect();
    assert!(
        all_names.contains(&"U".to_string()),
        "schema must contain U (the selected member); got {all_names:?}"
    );
    assert!(
        !all_names.contains(&"q_clover".to_string()),
        "schema must not contain unselected `q_clover`; got {all_names:?}"
    );
    assert!(
        !all_names.contains(&"q_rounded".to_string()),
        "schema must not contain unselected `q_rounded`; got {all_names:?}"
    );

    // Vector fiber `U` should have dims=3 (U's inner shape product).
    let u_field = store
        .schema
        .fiber_fields
        .iter()
        .find(|f| f.name == "U")
        .expect("U fiber field present");
    match &u_field.field_type {
        gigi::types::FieldType::Vector { dims } => {
            assert_eq!(*dims, 3, "U's inner Vector dims equal its inner shape")
        }
        other => panic!("expected Vector(dims=3) for U, got {other:?}"),
    }

    // Row indices 0 and 1 present.
    let mut row_indices: Vec<i64> = Vec::new();
    for rec in store.records() {
        if let Some(Value::Integer(i)) = rec.get("row_idx") {
            row_indices.push(*i);
        }
    }
    row_indices.sort();
    assert_eq!(row_indices, vec![0, 1]);
}

/// Test 2 — KEY absent + multi-array NPZ + GAUGE_FIELD interpretation
/// errors clearly.
///
/// The historical error `MultiArrayNotAllowedForGaugeField` must still
/// fire when the caller drops the KEY clause on a multi-member archive
/// bound for `AS GAUGE_FIELD` interpretation. The error string must
/// name the number of members observed AND suggest KEY <name> as the
/// remedy so the caller can add exactly the clause they omitted.
#[test]
fn test_ingest_npz_key_absent_multi_array_errors() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("multi_no_key.npz");

    let a_data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
    let b_data: Vec<f64> = vec![5.0, 6.0, 7.0, 8.0];
    let c_data: Vec<f64> = vec![9.0, 10.0, 11.0, 12.0];
    write_test_npz_multi(
        &path,
        &[
            ("alpha", &[2, 2], &a_data),
            ("beta", &[2, 2], &b_data),
            ("gamma", &[2, 2], &c_data),
        ],
    );

    let path_str = path.to_string_lossy().replace('\\', "/");
    // Direct executor call via GQL — no KEY clause. Expect a clear
    // error naming the observed member count AND mentioning KEY.
    let stmt_src = format!(
        "INGEST no_key_bundle FROM '{}' FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE some_lattice;",
        path_str
    );
    let stmt = parse(&stmt_src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    let err = execute(&mut engine, &stmt)
        .expect_err("must reject multi-array NPZ without KEY under GAUGE_FIELD");
    assert!(
        err.contains("3") || err.contains("multi"),
        "error should name member count or multi-array shape: {err}"
    );
    assert!(
        err.to_uppercase().contains("KEY"),
        "error should suggest KEY <name> remedy: {err}"
    );
}

/// Test 3 — KEY absent + single-array NPZ works (backward compat).
///
/// A single-member archive without a KEY clause must still ingest,
/// preserving today's behavior. This is the regression fence against
/// a GREEN patch that accidentally makes KEY mandatory.
#[test]
fn test_ingest_npz_key_absent_single_array_works() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("single_no_key.npz");

    let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
    write_test_npz_single(&path, "the_only_one", &[3, 4], &data);

    let path_str = path.to_string_lossy().replace('\\', "/");
    let stmt_src = format!(
        "INGEST single_compat_bundle FROM '{}' FORMAT NPZ;",
        path_str
    );
    let stmt = parse(&stmt_src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    let result = execute(&mut engine, &stmt)
        .unwrap_or_else(|e| panic!("execute failed: {e}"));
    assert_eq!(result, ExecResult::Ok);

    let bundle = engine
        .bundle("single_compat_bundle")
        .expect("bundle exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 3, "single-array NPZ still emits 3 records");

    let all_names: Vec<String> = store
        .schema
        .base_fields
        .iter()
        .chain(store.schema.fiber_fields.iter())
        .map(|f| f.name.clone())
        .collect();
    assert!(
        all_names.contains(&"the_only_one".to_string()),
        "schema retains the single member name: {all_names:?}"
    );
}

/// Test 4 — KEY <name> naming a missing array errors with the requested
/// name AND the available names.
///
/// The caller mistyped or expected a different archive layout. The
/// error must include the KEY value they supplied AND the actual
/// members so the fix is a one-shot correction, not a round trip to
/// re-inspect the NPZ.
#[test]
fn test_ingest_npz_key_unknown_array_name_errors() {
    let (mut engine, _dir) = open_engine();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let path = tmp.path().join("key_typo.npz");

    let u_data: Vec<f64> = (0..6).map(|i| i as f64).collect();
    let q_data: Vec<f64> = (0..8).map(|i| -(i as f64)).collect();
    write_test_npz_multi(
        &path,
        &[
            ("U", &[2, 3], &u_data),
            ("q_clover", &[4, 2], &q_data),
        ],
    );

    let path_str = path.to_string_lossy().replace('\\', "/");
    // Typo: `u_gauge` instead of `U`.
    let stmt_src = format!(
        "INGEST typo_bundle FROM '{}' FORMAT NPZ KEY u_gauge;",
        path_str
    );
    let stmt = parse(&stmt_src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    let err = execute(&mut engine, &stmt)
        .expect_err("must reject KEY naming a missing member");
    // The error must name the missing key value.
    assert!(
        err.contains("u_gauge"),
        "error should name the requested key `u_gauge`: {err}"
    );
    // The error must name each of the actual members so the caller can
    // correct the KEY without re-inspecting the file.
    assert!(
        err.contains("U"),
        "error should name available member `U`: {err}"
    );
    assert!(
        err.contains("q_clover"),
        "error should name available member `q_clover`: {err}"
    );
}
