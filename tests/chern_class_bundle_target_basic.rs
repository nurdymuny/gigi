//! Halcyon CHERN_CLASS bundle-target extension — RED phase.
//!
//! Concept 3 of the Halcyon L=24 OBC workflow (Hallie 2026-06-28-evening
//! asks 2 + 3). Pins the parser grammar + executor semantics for
//! `CHERN_CLASS` when the target is a BUNDLE (not a gauge::registry
//! handle), with optional `ON LATTICE`, `PER <field>`, and
//! `INTO_COLUMN <col>` clauses.
//!
//! ── What each test asserts ───────────────────────────────────────────
//!
//! 1. `test_chern_class_bundle_no_lattice_errors_clearly`
//!    A bundle-target CHERN_CLASS without `ON LATTICE` must reject
//!    with a specific error naming that bundle records supply the
//!    fiber but the lattice supplies the cell complex — you cannot
//!    integrate curvature 2-forms without a cell complex to integrate
//!    on.
//!
//! 2. `test_chern_class_bundle_with_lattice_2d_su2_identity_returns_zero_row`
//!    Bundle target + `ON LATTICE l4_obc` + SU(2) identity fiber on
//!    a 2D L=4 OBC lattice returns `Rows` with a single row containing
//!    `chern_class_2 ≈ 0.0` and `q_rounded = 0`. 2D base ⇒ c_2 vanishes
//!    by dim-guard; result envelope is `Rows` not `Scalar` because the
//!    bundle path always returns a uniform per-group shape.
//!
//! 3. `test_chern_class_gauge_field_with_lattice_clause_errors_conflict`
//!    Gauge-field target + explicit `ON LATTICE` must reject: the
//!    gauge field already carries a lattice binding via
//!    `handle.lattice_name()`, adding another is a conflict.
//!
//! 4. `test_chern_class_per_config_id_returns_multiple_rows_stable_order`
//!    Bundle target + `PER config_id` on a 2-config synthetic SU(2)
//!    L=4 OBC bundle returns `Rows` with exactly 2 rows, one per
//!    config_id, in ascending config_id order (BTreeMap grouping).
//!    Each row carries `config_id`, `chern_class_2`, `q_rounded`.
//!
//! 5. `test_chern_class_into_column_writes_q_rounded_field_back`
//!    Bundle target + `PER config_id INTO_COLUMN q_rounded` writes
//!    the rounded sector back to the source bundle as a new BASE
//!    field value. After the CHERN_CLASS statement, a `COVER` scan
//!    of the bundle reads back the `q_rounded` column for each
//!    record consistent with the PER-group's rounded value.
//!
//! 6. `test_chern_class_into_column_without_per_parse_error`
//!    Parser rejects `INTO_COLUMN` without `PER` at parse time:
//!    the rounded sector needs a grouping key to write per-group.
//!
//! 7. `test_chern_class_into_column_undeclared_column_errors`
//!    Executor rejects `INTO_COLUMN` when the target column is not
//!    a declared BASE field on the bundle schema. Phase 1 policy is
//!    explicit schema evolution; the error message points at
//!    `ALTER BUNDLE ... ADD BASE q_rounded INT`.
//!
//! 8. `test_chern_class_gauge_field_target_unchanged_backwards_compat`
//!    Backwards-compat: the existing gauge-field-target grammar
//!    `CHERN_CLASS U_smoke ORDER 2 ON FIBER (q0..) GROUP SU(2)`
//!    returns `Scalar(f64)`, not `Rows`. Pins the two-path resolver
//!    at the executor boundary.
//!
//! 9. `test_chern_class_bundle_missing_target_errors_clearly`
//!    Neither a gauge field nor a bundle by that name → clear error
//!    listing the two possible sources.
//!
//! 10. `test_chern_class_per_missing_field_errors_clearly`
//!     Records missing the PER field → error naming that field.
//!
//! 11. `test_chern_class_bundle_su2_l4_obc_identity_zero_end_to_end`
//!     End-to-end shape of Hallie's L=24 workflow, shrunk to L=4
//!     D=2: LATTICE OBC + CREATE BUNDLE + INSERT identity SU(2)
//!     records + CHERN_CLASS bundle target PER + INTO_COLUMN
//!     round-trip. Every commit landing here is what unblocks the
//!     L=24 β=2.3 OBC sectoral SPECTRAL_GAUGE run once concepts 1
//!     + 2 land ahead of this.
//!
//! ── Why this build will be RED ────────────────────────────────────────
//!
//! At HEAD 8b30221 the `Statement::ChernClass` variant has only four
//! fields (`bundle`, `order`, `fiber_fields`, `group`). None of the
//! new clauses (`ON LATTICE`, `PER`, `INTO_COLUMN`) are in the
//! grammar. The executor only knows the gauge-field-target path.
//!
//! Every test below will fail — either at parse time (clauses
//! rejected as unexpected tokens), at execute time (bundle path not
//! implemented), or at result-envelope check (`Rows` vs. `Scalar`).
//!
//! The GREEN commit lands:
//!   * `Statement::ChernClass` gains `lattice`, `per_field`,
//!     `into_column: Option<String>` fields
//!   * `parse_chern_class` learns the three new clauses (any order
//!     after `ORDER k`)
//!   * `try_dispatch_topology_statement`'s ChernClass arm splits
//!     into a two-path resolver (gauge-field vs bundle) with
//!     PER grouping + INTO_COLUMN write-back
//!   * New `BundleEdgeConnectionAdapter` in `src/gauge/` reads
//!     records into a `dyn EdgeConnection` shape the kernel accepts
//!
//! Run with:
//!   `cargo test --features halcyon --test chern_class_bundle_target_basic`
//!
//! Note: the L=4 D=2 OBC fixture uses `LATTICE l4_obc FROM CUBIC
//! L=4 DIM=2 OBC AXIS 0;` (Concept 1 grammar). Until Concept 1
//! GREEN lands, the LATTICE declaration itself will fail to parse
//! and every test that depends on it will RED-fail at fixture
//! setup — that is the correct dependency ordering. Concept 1
//! must land BEFORE Concept 3 GREEN; Concept 3 RED tests are safe
//! to land now because they are gated behind the fixture setup
//! and produce a clean RED signal.

#![cfg(feature = "halcyon")]

use std::sync::RwLock;

use gigi::engine::Engine;
use gigi::halcyon_gql_dispatch::try_dispatch_topology_statement;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

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

/// Declare a 2D L=4 OBC cubic lattice named `l4_obc` (Concept 1
/// grammar — this parse will fail until Concept 1 GREEN lands,
/// which is the correct dependency ordering for a RED file that
/// runs alongside concurrent concept development).
fn declare_l4_obc_lattice(eng_lock: &RwLock<Engine>) {
    let mut eng = eng_lock.write().expect("engine write lock");
    let stmt = parse("LATTICE l4_obc FROM CUBIC L=4 DIM=2 OBC AXIS 0;")
        .expect("parse LATTICE l4_obc OBC AXIS 0 (Concept 1)");
    execute(&mut eng, &stmt).expect("exec LATTICE l4_obc");
}

/// Declare a 2D L=4 PERIODIC cubic lattice named `l4_periodic` +
/// an identity SU(2) gauge field on it. Used by the backwards-compat
/// gauge-field-target test which does NOT depend on Concept 1.
fn declare_periodic_lattice_and_gauge_field(eng_lock: &RwLock<Engine>) {
    let mut eng = eng_lock.write().expect("engine write lock");
    let lat_stmt = parse("LATTICE l4_periodic FROM CUBIC L=4 DIM=2 PERIODIC;")
        .expect("parse LATTICE l4_periodic");
    execute(&mut eng, &lat_stmt).expect("exec LATTICE l4_periodic");
    let gf_stmt = parse(
        "GAUGE_FIELD U_periodic ON LATTICE l4_periodic GROUP SU(2) INIT IDENTITY;",
    )
    .expect("parse GAUGE_FIELD U_periodic");
    execute(&mut eng, &gf_stmt).expect("exec GAUGE_FIELD U_periodic");
}

/// Create the target bundle schema Hallie's INGEST path emits:
/// `config_id, mu, site_x, site_y` as BASE fields and
/// `q0..q3` as FIBER (SU(2) quaternion components). Then INSERT
/// `n_configs × 2 × 4 × 4 = 32 · n_configs` records of the SU(2)
/// identity `(1, 0, 0, 0)` on every edge (mu = 0..1, site_x = 0..3,
/// site_y = 0..3).
fn create_su2_identity_bundle(
    eng_lock: &RwLock<Engine>,
    bundle_name: &str,
    n_configs: usize,
) {
    let mut eng = eng_lock.write().expect("engine write lock");
    let schema = format!(
        "CREATE BUNDLE {} (\
            config_id INT BASE, mu INT BASE, site_x INT BASE, site_y INT BASE, \
            q0 FLOAT FIBER, q1 FLOAT FIBER, q2 FLOAT FIBER, q3 FLOAT FIBER\
         )",
        bundle_name
    );
    let stmt = parse(&schema).expect("parse CREATE BUNDLE");
    execute(&mut eng, &stmt).expect("exec CREATE BUNDLE");

    for cid in 0..n_configs {
        for mu in 0..2 {
            for sx in 0..4 {
                for sy in 0..4 {
                    let insert = format!(
                        "INSERT INTO {bundle_name} \
                         (config_id, mu, site_x, site_y, q0, q1, q2, q3) \
                         VALUES ({cid}, {mu}, {sx}, {sy}, 1.0, 0.0, 0.0, 0.0)"
                    );
                    let stmt = parse(&insert).expect("parse INSERT identity");
                    execute(&mut eng, &stmt).expect("exec INSERT identity");
                }
            }
        }
    }
}

/// Unwrap `ExecResult::Rows`; panic with a precise message on the
/// other variants so a mismatched envelope is loud, not silent.
fn rows(r: ExecResult) -> Vec<gigi::types::Record> {
    match r {
        ExecResult::Rows(rs) => rs,
        other => panic!("expected ExecResult::Rows, got {other:?}"),
    }
}

/// Unwrap `ExecResult::Scalar`; loud on mismatch.
fn scalar(r: ExecResult) -> f64 {
    match r {
        ExecResult::Scalar(v) => v,
        other => panic!("expected ExecResult::Scalar, got {other:?}"),
    }
}

// ── Tests ────────────────────────────────────────────────────────────

/// (1) Bundle target without `ON LATTICE` must reject with a clear
/// message pointing the caller at the missing cell complex. This
/// pins the executor's bundle-path guard.
#[test]
fn test_chern_class_bundle_no_lattice_errors_clearly() {
    let (eng, _dir) = fresh_engine_and_registries();
    create_su2_identity_bundle(&eng, "b_no_lattice", 1);

    let stmt = parse(
        "CHERN_CLASS b_no_lattice ORDER 2 ON FIBER (q0, q1, q2, q3) GROUP SU(2);",
    )
    .expect("parse CHERN_CLASS bundle target (no lattice)");
    let err = try_dispatch_topology_statement(&eng, &stmt)
        .expect_err("bundle target without ON LATTICE must error");
    assert!(
        err.contains("ON LATTICE"),
        "error must name the missing ON LATTICE clause, got: {err}"
    );
    assert!(
        err.contains("b_no_lattice"),
        "error must name the offending bundle, got: {err}"
    );
    // Concept 3 GREEN provides the specific reason so the caller
    // understands WHY the clause is required (bundle records supply
    // the fiber; lattice supplies the cell complex). This wording is
    // what pins the RED file to the design's error contract — the
    // existing gauge-only executor does not say this.
    assert!(
        err.contains("bundle records") || err.contains("cell complex") ||
            err.contains("requires ON LATTICE"),
        "error must explain WHY ON LATTICE is required on a bundle target, got: {err}"
    );
}

/// (2) Bundle target + ON LATTICE + SU(2) identity on 2D L=4 OBC
/// returns `Rows` with a single row containing `chern_class_2 ≈ 0.0`
/// and `q_rounded = 0`. 2D dim-guard makes c_2 vanish.
#[test]
fn test_chern_class_bundle_with_lattice_2d_su2_identity_returns_zero_row() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);
    create_su2_identity_bundle(&eng, "b_id_l4_obc", 1);

    let stmt = parse(
        "CHERN_CLASS b_id_l4_obc ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2);",
    )
    .expect("parse CHERN_CLASS bundle + ON LATTICE (Concept 3)");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("bundle path must succeed");
    let rs = rows(res);
    assert_eq!(
        rs.len(),
        1,
        "no PER → single-row Rows envelope, got {}",
        rs.len()
    );
    let row = &rs[0];
    match row.get("chern_class_2") {
        Some(Value::Float(v)) => assert!(
            v.abs() < 1e-10,
            "c_2 on identity SU(2) 2D base must be 0, got {v}"
        ),
        other => panic!("chern_class_2 must be Float, got {other:?}"),
    }
    match row.get("q_rounded") {
        Some(Value::Integer(0)) => {}
        other => panic!("q_rounded must be Integer(0), got {other:?}"),
    }
}

/// (3) Gauge-field target with an explicit `ON LATTICE` clause is a
/// conflict — the gauge field already carries a lattice binding via
/// `handle.lattice_name()`. Executor must reject.
#[test]
fn test_chern_class_gauge_field_with_lattice_clause_errors_conflict() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_periodic_lattice_and_gauge_field(&eng);

    let stmt = parse(
        "CHERN_CLASS U_periodic ORDER 2 ON LATTICE l4_periodic \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2);",
    )
    .expect("parse CHERN_CLASS gauge target + ON LATTICE (Concept 3)");
    let err = try_dispatch_topology_statement(&eng, &stmt)
        .expect_err("gauge target + ON LATTICE must error as conflict");
    assert!(
        err.contains("already carries") || err.contains("conflict") || err.contains("gauge"),
        "error must flag the lattice-binding conflict, got: {err}"
    );
}

/// (4) Bundle target + PER config_id on a 2-config identity SU(2)
/// bundle returns exactly 2 rows in ascending config_id order.
#[test]
fn test_chern_class_per_config_id_returns_multiple_rows_stable_order() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);
    create_su2_identity_bundle(&eng, "b_per_cfg", 2);

    let stmt = parse(
        "CHERN_CLASS b_per_cfg ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2) PER config_id;",
    )
    .expect("parse CHERN_CLASS PER config_id (Concept 3)");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("PER dispatch must succeed");
    let rs = rows(res);
    assert_eq!(
        rs.len(),
        2,
        "PER config_id with 2 configs must yield 2 rows, got {}",
        rs.len()
    );
    // Ascending config_id order (BTreeMap grouping).
    let ids: Vec<i64> = rs
        .iter()
        .map(|r| match r.get("config_id") {
            Some(Value::Integer(i)) => *i,
            other => panic!("config_id must be Integer, got {other:?}"),
        })
        .collect();
    assert_eq!(
        ids,
        vec![0, 1],
        "rows must be ordered ascending by config_id, got {ids:?}"
    );
    // Each row carries the labeled scalar + rounded sector.
    for row in &rs {
        assert!(
            matches!(row.get("chern_class_2"), Some(Value::Float(_))),
            "each row must carry chern_class_2 Float"
        );
        assert!(
            matches!(row.get("q_rounded"), Some(Value::Integer(0))),
            "identity → q_rounded = 0 for every group"
        );
    }
}

/// (5) INTO_COLUMN writes q_rounded back into the source bundle.
/// After execute, a COVER scan reads back the new BASE column value.
#[test]
fn test_chern_class_into_column_writes_q_rounded_field_back() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);
    create_su2_identity_bundle(&eng, "b_write_back", 2);

    // Precondition — the target column must exist as a BASE field on
    // the bundle schema. Phase 1 policy is explicit schema evolution;
    // Concept 3 GREEN adds an `ALTER BUNDLE ... ADD BASE q_rounded INT`
    // grammar or accepts the pre-existing column here.
    let alter = parse("ALTER BUNDLE b_write_back ADD BASE q_rounded INT;")
        .expect("parse ALTER BUNDLE ADD BASE q_rounded (Concept 3 dep)");
    {
        let mut eng_w = eng.write().expect("engine write lock");
        execute(&mut eng_w, &alter).expect("exec ALTER BUNDLE ADD BASE q_rounded");
    }

    let stmt = parse(
        "CHERN_CLASS b_write_back ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2) PER config_id \
         INTO_COLUMN q_rounded;",
    )
    .expect("parse CHERN_CLASS PER INTO_COLUMN (Concept 3)");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("INTO_COLUMN dispatch must succeed");
    let rs = rows(res);
    assert_eq!(rs.len(), 2, "PER over 2 configs → 2 rows");

    // Now read back the bundle via COVER and verify every record
    // has q_rounded = 0 (identity → sector 0 for every config).
    let cover_stmt = parse("COVER b_write_back").expect("parse COVER");
    let cover_res = {
        let mut eng_w = eng.write().expect("engine write lock");
        execute(&mut eng_w, &cover_stmt).expect("exec COVER")
    };
    let cover_rows = match cover_res {
        ExecResult::Rows(rs) => rs,
        other => panic!("expected Rows from COVER, got {other:?}"),
    };
    assert!(!cover_rows.is_empty(), "bundle must have records post-write");
    for r in &cover_rows {
        match r.get("q_rounded") {
            Some(Value::Integer(0)) => {}
            other => panic!(
                "every record must carry q_rounded = 0 after INTO_COLUMN, got {other:?}"
            ),
        }
    }
}

/// (6) Parser rejects INTO_COLUMN without PER — nothing to write to
/// otherwise.
#[test]
fn test_chern_class_into_column_without_per_parse_error() {
    let err = parse(
        "CHERN_CLASS b ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2) INTO_COLUMN q_rounded;",
    )
    .expect_err("INTO_COLUMN without PER must fail at parse time");
    assert!(
        err.contains("INTO_COLUMN") && err.contains("PER"),
        "parse error must name both clauses, got: {err}"
    );
}

/// (7) Executor rejects INTO_COLUMN when the target column is not a
/// declared BASE field on the bundle schema. Phase 1 policy: explicit
/// schema evolution; the error message points at ALTER BUNDLE.
#[test]
fn test_chern_class_into_column_undeclared_column_errors() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);
    create_su2_identity_bundle(&eng, "b_no_col", 1);
    // NOTE: no ALTER BUNDLE — q_rounded is NOT a declared column.

    let stmt = parse(
        "CHERN_CLASS b_no_col ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2) PER config_id \
         INTO_COLUMN q_rounded;",
    )
    .expect("parse INTO_COLUMN (undeclared column)");
    let err = try_dispatch_topology_statement(&eng, &stmt)
        .expect_err("INTO_COLUMN to undeclared column must error");
    assert!(
        err.contains("q_rounded"),
        "error must name the undeclared column, got: {err}"
    );
    assert!(
        err.contains("ALTER BUNDLE") || err.contains("BASE") || err.contains("declared"),
        "error must direct caller to explicit schema evolution, got: {err}"
    );
}

/// (8) Backwards compat: gauge-field target with the pre-existing
/// grammar returns `Scalar(f64)`, NOT `Rows`. Pins the two-path
/// resolver at the executor boundary.
#[test]
fn test_chern_class_gauge_field_target_unchanged_backwards_compat() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_periodic_lattice_and_gauge_field(&eng);

    let stmt = parse(
        "CHERN_CLASS U_periodic ORDER 2 ON FIBER (q0, q1, q2, q3) GROUP SU(2);",
    )
    .expect("parse existing gauge-field-target grammar");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("gauge-field target must still succeed");
    // 2D base + identity SU(2) → c_2 = 0.
    assert!(
        scalar(res).abs() < 1e-10,
        "c_2 on identity SU(2) 2D gauge field must be 0"
    );
}

/// (9) Neither a gauge field nor a bundle by that name — error must
/// list both possible sources so the caller knows what to declare.
#[test]
fn test_chern_class_bundle_missing_target_errors_clearly() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);

    let stmt = parse(
        "CHERN_CLASS ghost_bundle ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2);",
    )
    .expect("parse CHERN_CLASS with unknown target");
    let err = try_dispatch_topology_statement(&eng, &stmt)
        .expect_err("unknown target must error");
    assert!(
        err.contains("ghost_bundle"),
        "error must name the missing target, got: {err}"
    );
    assert!(
        err.contains("GAUGE_FIELD") || err.contains("gauge") ||
            err.contains("CREATE BUNDLE") || err.contains("bundle"),
        "error must direct caller to both declaration paths, got: {err}"
    );
}

/// (10) PER on a field the records do not carry → clear error naming
/// the missing field.
#[test]
fn test_chern_class_per_missing_field_errors_clearly() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);
    create_su2_identity_bundle(&eng, "b_per_missing", 1);

    let stmt = parse(
        "CHERN_CLASS b_per_missing ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2) PER nonexistent_field;",
    )
    .expect("parse PER nonexistent_field");
    let err = try_dispatch_topology_statement(&eng, &stmt)
        .expect_err("PER on missing field must error");
    assert!(
        err.contains("nonexistent_field"),
        "error must name the missing field, got: {err}"
    );
}

/// (11) End-to-end shape of Hallie's L=24 workflow, shrunk to L=4
/// D=2: LATTICE OBC + CREATE BUNDLE + INSERT identity SU(2) records +
/// CHERN_CLASS bundle target PER config_id INTO_COLUMN q_rounded
/// round-trip. This is the shape of the acceptance witness in the
/// design's Phase 6 verification: if this passes on the live binary,
/// the L=24 workflow is unblocked end-to-end (modulo Hallie's Modal
/// data being fresh on her side).
#[test]
fn test_chern_class_bundle_su2_l4_obc_identity_zero_end_to_end() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_l4_obc_lattice(&eng);
    create_su2_identity_bundle(&eng, "test_su2_l4", 2);
    {
        let mut eng_w = eng.write().expect("engine write lock");
        let alter = parse("ALTER BUNDLE test_su2_l4 ADD BASE q_rounded INT;")
            .expect("parse ALTER BUNDLE ADD BASE q_rounded");
        execute(&mut eng_w, &alter).expect("exec ALTER BUNDLE ADD BASE q_rounded");
    }

    let stmt = parse(
        "CHERN_CLASS test_su2_l4 ORDER 2 ON LATTICE l4_obc \
         ON FIBER (q0, q1, q2, q3) GROUP SU(2) PER config_id \
         INTO_COLUMN q_rounded;",
    )
    .expect("parse full Concept-3 CHERN_CLASS");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("end-to-end must succeed");
    let rs = rows(res);
    assert_eq!(rs.len(), 2, "2 configs → 2 rows");
    for row in &rs {
        match row.get("chern_class_2") {
            Some(Value::Float(v)) => assert!(
                v.abs() < 1e-10,
                "2D SU(N) identity → c_2 = 0, got {v}"
            ),
            other => panic!("chern_class_2 must be Float, got {other:?}"),
        }
        match row.get("q_rounded") {
            Some(Value::Integer(0)) => {}
            other => panic!("q_rounded must be Integer(0), got {other:?}"),
        }
    }

    // Round-trip check: the write-back column reads back consistently.
    let cover = parse("COVER test_su2_l4").expect("parse COVER");
    let cover_res = {
        let mut eng_w = eng.write().expect("engine write lock");
        execute(&mut eng_w, &cover).expect("exec COVER")
    };
    let cover_rows = match cover_res {
        ExecResult::Rows(rs) => rs,
        other => panic!("expected Rows from COVER, got {other:?}"),
    };
    for r in &cover_rows {
        assert!(
            matches!(r.get("q_rounded"), Some(Value::Integer(0))),
            "every record must round-trip q_rounded = 0"
        );
    }
}
