//! Halcyon INGEST route-handler bypass — RED-phase integration tests.
//!
//! Pins the route-handler contract for the `INGEST` verb through the
//! `/v1/gql` POST surface. Same failure mode as Hallie's 2026-06-28
//! topology-verb smoke chain (fixed by 553a6c9 + 059a2c2 +
//! `try_dispatch_topology_statement`), rediscovered by Hallie's
//! 2026-07-01 afternoon smoke chain against gigi-stream v233:
//!
//! ```text
//! LATTICE l4_obc_verify FROM CUBIC L=4 DIM=4 OBC AXIS 0;      → {"status":"ok"}
//! INGEST su2_L4_obc_verify FROM '..._L4/raw_U_configs.npz'
//!     FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_obc_verify;
//!                                                            → HTTP 404
//!                                                              {"error":"No bundle:
//!                                                              su2_L4_obc_verify"}
//! ```
//!
//! Root cause is the same shape:
//! `src/bin/gigi_stream.rs::gql_query` runs `engine.bundle(&bundle_name)`
//! BEFORE the INGEST executor dispatches. `get_bundle_name(&stmt)`
//! returns `Some("su2_L4_obc_verify")` for `Statement::Ingest`, and the
//! bundle does not yet exist (INGEST is a bundle-CREATOR — it materializes
//! the bundle from the NPZ header via `ingest.rs:417-422` where
//! `ensure_bundle_compatible(..., allow_auto_create=true)` fires). So the
//! pre-resolve wall 404s before the executor code that would create the
//! bundle ever runs.
//!
//! The fix mirrors the topology-verb bypass: a special-case dispatch
//! the route handler consults BEFORE the bundle pre-resolve, matching
//! `Statement::Ingest` and forwarding to `parser::execute` (which
//! delegates to `crate::ingest::execute_ingest` /
//! `execute_ingest_as_gauge_field` per the `as_gauge_field` clause).
//! These tests drive that dispatcher directly, bypassing axum — the
//! route handler is a thin wrapper over the dispatcher, so pinning the
//! dispatcher pins the route handler's external contract.
//!
//! ── Design choice: call the lib-crate dispatcher directly ────────────
//!
//! Mirrors the choice made in `tests/topology_verbs_gql_integration.rs`:
//! testing `try_dispatch_ingest_statement` covers the route handler's
//! behavior end-to-end because the GREEN route-handler patch will be
//! a thin `if let Statement::Ingest = &stmt { forward to dispatcher }`
//! forwarder inserted immediately after the topology-verb bypass block
//! (`src/bin/gigi_stream.rs:12530`), before the bundle pre-resolve
//! at line 12553.
//!
//! ── Why this build will be RED ────────────────────────────────────────
//!
//! Today `try_dispatch_ingest_statement` is a stub returning
//! `Err("try_dispatch_ingest_statement: not implemented (RED phase)")`.
//! Tests 1, 2, 5 land on an `expect("must succeed")` panic. Test 3
//! asserts the error contains an INGEST-executor-produced string
//! ("not found" / "source file") which the stub does not produce, so
//! it fails as well. Test 4 (topology regression guard) STAYS GREEN
//! because it exercises `try_dispatch_topology_statement`, which is
//! already implemented — this test is included as a regression fence,
//! not a RED signal.
//!
//! GREEN commit will replace the stub with a call through to
//! `parser::execute` on the write-locked engine.
//!
//! Run with:
//!   `cargo test --features halcyon --test ingest_gql_bypass_basic`

#![cfg(feature = "halcyon")]

use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use std::sync::RwLock;

use gigi::engine::Engine;
use gigi::halcyon_gql_dispatch::{
    try_dispatch_ingest_statement, try_dispatch_topology_statement,
};
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::{FieldType, Value};

use npyz::npz::NpzWriter;
use npyz::WriterBuilder;

// ── Fixtures ─────────────────────────────────────────────────────────

/// Build a fresh `Engine` in a tempdir + a clean `RwLock` around it,
/// with both gauge and lattice registries cleared. Returns the lock
/// plus the `TempDir` guard (must live as long as the lock).
fn fresh_engine_and_registries() -> (RwLock<Engine>, tempfile::TempDir) {
    gigi::gauge::registry::clear();
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (RwLock::new(engine), dir)
}

/// Write a single-array NPZ file. Reuses the exact writer pattern from
/// `tests/ingest_executor.rs` — same NPZ shape the INGEST executor
/// reads, so the fixtures exercise the production code path.
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

/// Escape a filesystem path for embedding in a GQL string literal.
/// Windows paths carry `\` which the GQL parser reads as escape
/// sequences; normalize to forward slashes.
fn gql_path_lit(p: &Path) -> String {
    p.to_string_lossy().replace('\\', "/")
}

// ── Tests ────────────────────────────────────────────────────────────

/// (1) Hallie's reported failure case (2026-07-01). An `INGEST` verb
/// against a bundle name that does NOT exist in the engine bundle
/// store must reach the INGEST executor and auto-create the bundle
/// from the NPZ header. Under the pre-resolve wall bug, the route
/// handler returns HTTP 404 `{"error":"No bundle: <name>"}` before
/// the executor ever runs. The dispatcher contract: bypass the
/// pre-resolve, forward to `parser::execute`, return `ExecResult::Ok`.
///
/// Under the RED stub, `try_dispatch_ingest_statement` returns
/// `Err("...not implemented (RED phase)")`, so the `.expect(...)`
/// panic fires and the test fails.
#[test]
fn test_ingest_gql_fresh_bundle_name_bypasses_pre_resolve() {
    let (eng, _dir) = fresh_engine_and_registries();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let npz_path = tmp.path().join("fresh.npz");

    // 4x3 float64 array — small, fast, unambiguous.
    let data: Vec<f64> = (0..12).map(|i| i as f64).collect();
    write_test_npz_single(&npz_path, "fresh", &[4, 3], &data);

    let query = format!(
        "INGEST fresh_bundle FROM '{}' FORMAT NPZ;",
        gql_path_lit(&npz_path)
    );
    let stmt = parse(&query).expect("parse INGEST");

    // Sanity precondition: fresh_bundle does NOT exist. If this
    // assertion ever fails, the fixture is stale — investigate before
    // weakening.
    {
        let g = eng.read().expect("read engine");
        assert!(
            g.bundle("fresh_bundle").is_none(),
            "fresh_bundle must NOT exist before INGEST (else the test \
             is not exercising the fresh-name path)"
        );
    }

    let result = try_dispatch_ingest_statement(&eng, &stmt).expect(
        "INGEST dispatch on fresh bundle name must succeed — the executor \
         auto-creates the bundle from the NPZ header (no bundle pre-resolve)",
    );

    // Result envelope: parser::execute on Statement::Ingest returns
    // ExecResult::Ok on the auto-create path.
    match &result {
        ExecResult::Ok => (),
        other => panic!("expected ExecResult::Ok, got {other:?}"),
    }

    // Sanity postcondition: the bundle now exists with the inferred
    // schema (row_idx Numeric base + `fresh` Vector(dims=3) fiber).
    {
        let g = eng.read().expect("read engine");
        let bundle =
            g.bundle("fresh_bundle").expect("bundle materialized by INGEST");
        let store = bundle.as_heap().expect("heap-resident");
        assert_eq!(store.len(), 4, "4 outer-axis slices → 4 records");
        assert_eq!(store.schema.fiber_fields[0].name, "fresh");
        match &store.schema.fiber_fields[0].field_type {
            FieldType::Vector { dims } => assert_eq!(*dims, 3),
            other => panic!("expected Vector(dims=3), got {other:?}"),
        }
    }
}

/// (2) Backwards compat. When the target bundle DOES exist with a
/// compatible schema, the INGEST executor must still fire through the
/// dispatcher, and the dispatcher must not regress this path.
///
/// INGEST semantics note: the executor upserts records keyed on the
/// `row_idx` base field (`engine.batch_insert` in `src/engine.rs:1576`
/// dispatches to `BundleStore::batch_insert`, which hashes each record
/// by base-field values and overwrites collisions — see
/// `src/bundle.rs:1214`). Firing INGEST twice on the same file therefore
/// keeps the record count at 5, not 10 — same-row_idx records upsert.
/// The load-bearing check for this test is that the second dispatch
/// call returns `Ok` (proving the pre-resolve wall was bypassed on the
/// existing-bundle path too), NOT that the record count doubled.
///
/// Under the RED stub, dispatch returns Err — the test fails.
#[test]
fn test_ingest_gql_existing_bundle_name_still_works() {
    let (eng, _dir) = fresh_engine_and_registries();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let npz_path = tmp.path().join("append.npz");

    let data: Vec<f64> = (0..15).map(|i| i as f64).collect();
    write_test_npz_single(&npz_path, "existing", &[5, 3], &data);

    // Pre-create the bundle by driving one INGEST through the parser
    // directly (this bypasses the dispatcher — used as fixture
    // scaffolding, not as the test subject).
    {
        let mut g = eng.write().expect("write engine");
        let seed_query = format!(
            "INGEST existing_bundle FROM '{}' FORMAT NPZ;",
            gql_path_lit(&npz_path)
        );
        let seed_stmt = parse(&seed_query).expect("parse seed INGEST");
        execute(&mut g, &seed_stmt).expect("seed INGEST");
    }

    // Sanity: bundle exists with 5 records after the seed.
    {
        let g = eng.read().expect("read engine");
        let bundle = g.bundle("existing_bundle").expect("seed bundle");
        assert_eq!(
            bundle.as_heap().expect("heap-resident").len(),
            5,
            "seed INGEST populated 5 records"
        );
    }

    // Now the bundle exists. Fire a SECOND INGEST through the
    // dispatcher (the path under test). The dispatcher must forward
    // to the executor, which upserts by row_idx and returns Ok.
    let query = format!(
        "INGEST existing_bundle FROM '{}' FORMAT NPZ;",
        gql_path_lit(&npz_path)
    );
    let stmt = parse(&query).expect("parse second INGEST");
    let result = try_dispatch_ingest_statement(&eng, &stmt).expect(
        "INGEST dispatch on existing bundle name must succeed \
         (proves pre-resolve wall bypassed on existing-bundle path)",
    );
    match &result {
        ExecResult::Ok => (),
        other => panic!("expected ExecResult::Ok, got {other:?}"),
    }

    // Executor upserted the 5 records by row_idx (0..5). Count stays
    // at 5 — the load-bearing check is that dispatch didn't 404, which
    // it didn't (Ok above). Assert the same 5 records are still there
    // as a sanity fence.
    let g = eng.read().expect("read engine");
    let bundle = g.bundle("existing_bundle").expect("bundle still exists");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(
        store.len(),
        5,
        "second INGEST upserts by row_idx (same 5 keys), so count \
         stays 5 — proves the executor ran (and the pre-resolve was \
         bypassed) without regressing to a fresh auto-create"
    );
}

/// (3) Clear-error regression — the documented bug fix. When the NPZ
/// source path does not exist, the error must come from the INGEST
/// executor (something like "source file not found" / "not found")
/// NOT the legacy `"No bundle: <name>"` envelope. This is the specific
/// signature of the bug being fixed: killing the pre-resolve error
/// string == killing the bug.
///
/// This is the most load-bearing test — even if tests 1, 2, 5 flake on
/// fixture setup, THIS test pins the exact bug Hallie hit.
#[test]
fn test_ingest_gql_missing_file_returns_executor_error_not_no_bundle() {
    let (eng, _dir) = fresh_engine_and_registries();

    // Deliberately point at a phantom path.
    let query =
        "INGEST ghost_bundle FROM 'does-not-exist-anywhere.npz' FORMAT NPZ;";
    let stmt = parse(query).expect("parse INGEST");

    let result = try_dispatch_ingest_statement(&eng, &stmt);
    let err = result.expect_err(
        "INGEST with a missing source file must error (not return Ok)",
    );

    // The error MUST NOT be the legacy pre-resolve envelope — that is
    // the bug being fixed.
    assert!(
        !err.contains("No bundle:"),
        "error must NOT be the legacy 'No bundle:' envelope (that IS \
         the bug being fixed), got: {err}"
    );

    // The error SHOULD come from the INGEST executor. `IngestError`'s
    // Display for `FileNotFound` is:
    //     "INGEST: source file not found: <path>"
    // We match on the leading tokens rather than the full string so a
    // future error-message polish doesn't spuriously fail this gate.
    assert!(
        err.contains("not found")
            || err.contains("source file")
            || err.contains("INGEST"),
        "error must come from the INGEST executor (mentioning INGEST / \
         source file / not found), got: {err}"
    );
}

/// (4) Regression guard: the existing topology-verb bypass
/// (`try_dispatch_topology_statement`, shipped 2026-06-29) must
/// continue to work. This test is a fence — it stays GREEN across the
/// RED-to-GREEN transition of the INGEST dispatcher because it
/// exercises a completely different dispatcher that's already
/// implemented.
///
/// If a refactor accidentally breaks the topology bypass while wiring
/// the INGEST bypass, this test catches it.
#[test]
fn test_topology_verbs_still_route_correctly() {
    let (eng, _dir) = fresh_engine_and_registries();

    // Set up the same 2D L=4 PERIODIC lattice + identity SU(3) gauge
    // field pattern the topology dispatcher tests use.
    {
        let mut g = eng.write().expect("write engine");
        let lat = parse("LATTICE smoke FROM CUBIC L=4 DIM=2 PERIODIC;")
            .expect("parse LATTICE");
        execute(&mut g, &lat).expect("exec LATTICE");
        let gf = parse(
            "GAUGE_FIELD U_smoke ON LATTICE smoke GROUP SU(3) INIT IDENTITY;",
        )
        .expect("parse GAUGE_FIELD");
        execute(&mut g, &gf).expect("exec GAUGE_FIELD");
    }

    // Drive PI_1 through the topology dispatcher — must succeed with
    // the abelianized rank of π_1(T²) = 2.
    let stmt = parse("PI_1 smoke;").expect("parse PI_1");
    let result = try_dispatch_topology_statement(&eng, &stmt)
        .expect("PI_1 dispatch must succeed (regression guard)");
    match result {
        ExecResult::Scalar(v) => assert!(
            (v - 2.0).abs() < 1e-12,
            "π_1(T² L=4) rank must be 2, got {v} (topology bypass regressed?)"
        ),
        other => {
            panic!("expected ExecResult::Scalar, got {other:?}")
        }
    }
}

/// (5) Sanity: successful INGEST auto-create path returns something
/// the caller can inspect (record count via bundle re-read). Pins that
/// the dispatcher forwards the full statement, not just a subset —
/// under a naive early-return implementation, records might be dropped.
///
/// Under the RED stub this test fails because the dispatcher returns
/// Err before any records land.
#[test]
fn test_ingest_gql_returns_row_count_on_success() {
    let (eng, _dir) = fresh_engine_and_registries();
    let tmp = tempfile::tempdir().expect("tempdir for fixture");
    let npz_path = tmp.path().join("rowcount.npz");

    // 7 records × 2-vector, deliberately not the same shape as tests 1/2.
    let data: Vec<f64> = (0..14).map(|i| i as f64 * 0.5).collect();
    write_test_npz_single(&npz_path, "rowcount", &[7, 2], &data);

    let query = format!(
        "INGEST rowcount_bundle FROM '{}' FORMAT NPZ;",
        gql_path_lit(&npz_path)
    );
    let stmt = parse(&query).expect("parse INGEST");

    let result = try_dispatch_ingest_statement(&eng, &stmt)
        .expect("INGEST dispatch must succeed");
    match result {
        ExecResult::Ok => (),
        other => panic!("expected ExecResult::Ok, got {other:?}"),
    }

    // Sanity: the record count in the materialized bundle is 7.
    let g = eng.read().expect("read engine");
    let bundle =
        g.bundle("rowcount_bundle").expect("bundle materialized by INGEST");
    let store = bundle.as_heap().expect("heap-resident");
    assert_eq!(store.len(), 7, "7 outer-axis slices → 7 records");

    // Iteration order is non-deterministic on `BaseStorage::Hashed`
    // (the storage the auto-create path lands on — HashMap keyed by
    // BasePoint). Collect + sort by row_idx before verifying record 0.
    let mut sorted_records: Vec<gigi::types::Record> = store.records().collect();
    sorted_records.sort_by_key(|rec| match rec.get("row_idx") {
        Some(Value::Integer(i)) => *i,
        _ => i64::MAX,
    });

    // Sanity: the row_idx=0 record carries vector [0.0, 0.5]
    // (data was `(0..14).map(|i| i as f64 * 0.5)` reshaped to (7, 2)
    // — outer slice 0 == [0.0, 0.5], row_idx=0).
    let first = sorted_records
        .first()
        .expect("bundle must have at least one record");
    match first.get("row_idx") {
        Some(Value::Integer(i)) => assert_eq!(*i, 0),
        other => panic!("expected row_idx=Integer(0), got {other:?}"),
    }
    match first.get("rowcount") {
        Some(Value::Vector(v)) => {
            assert_eq!(v.len(), 2);
            assert!((v[0] - 0.0).abs() < 1e-12);
            assert!((v[1] - 0.5).abs() < 1e-12);
        }
        other => panic!("expected rowcount=Vector, got {other:?}"),
    }
}
