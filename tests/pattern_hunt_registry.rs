//! Phase 2 of the Pattern Hunt spec (Ask G — Patterns):
//! in-memory pattern registry on the Engine.
//!
//! Gates PH5–PH8 from `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §4.3.
//!
//! ### What this file covers
//!
//! - **PH5**  `DEFINE PATTERN p ...; SHOW PATTERNS` returns `p` in the result.
//! - **PH6a** `DEFINE PATTERN p ...; DEFINE PATTERN p ...` errors without
//!            `OR REPLACE` (typed collision error).
//! - **PH6b** `DEFINE OR REPLACE PATTERN p ...` overwrites silently.
//! - **PH7**  `HUNT p IN bundle` against a bundle whose schema lacks a
//!            field in `p.using` returns a typed `PatternFieldMissing`
//!            error, **not** a panic.
//! - **bonus** `DROP PATTERN p` removes the pattern from the registry
//!            (no spec gate name; complements PH5).
//! - **PH8**  Registry is non-transactional. `BEGIN; DEFINE PATTERN p;
//!            ROLLBACK` leaves `p` defined. Only compiled when
//!            `transactions` feature is also on.
//!
//! ### Domain-neutrality is load-bearing.
//!
//! All field, bundle, and pattern names are generic. Same discipline as
//! Phase 1: the registry must serve any consumer.

#![cfg(feature = "patterns")]

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult};
use tempfile::tempdir;

/// Spin up a fresh on-disk Engine in a tempdir. Tempdir leaks for the
/// duration of the test process — small leak, acceptable for tests.
fn fresh_engine() -> Engine {
    let dir = tempdir().expect("tempdir creation");
    let path = dir.into_path();
    Engine::open(&path).expect("engine open against fresh tempdir")
}

/// Convenience: parse + execute, returning either ExecResult or the
/// error message. Phase 2 statements should all parse cleanly (Phase 1
/// already proved that) — any Err here is from execute().
fn run(engine: &mut Engine, sql: &str) -> Result<ExecResult, String> {
    let stmt = parse(sql).map_err(|e| format!("parse failed on `{sql}`: {e}"))?;
    execute(engine, &stmt)
}

// ─── PH5 — DEFINE then SHOW PATTERNS lists the pattern ──────────────────────

#[test]
fn ph5_define_then_show_lists_the_pattern() {
    let mut engine = fresh_engine();
    run(&mut engine, "DEFINE PATTERN p AS field_a = 1").expect("DEFINE must succeed");
    let result = run(&mut engine, "SHOW PATTERNS").expect("SHOW must succeed");

    match result {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 1, "expected exactly one pattern row, got {}", rows.len());
            // Each pattern row should contain the pattern name as a field
            // value. The exact field name (`name`, `pattern`, etc.) is an
            // implementation choice; we just check the value "p" appears.
            let rendered = format!("{:?}", rows[0]);
            assert!(
                rendered.contains("\"p\"") || rendered.contains("'p'"),
                "expected pattern 'p' in row, got {rendered}"
            );
        }
        other => panic!("expected ExecResult::Rows, got {other:?}"),
    }
}

#[test]
fn ph5_show_patterns_returns_empty_when_no_patterns_defined() {
    let mut engine = fresh_engine();
    let result = run(&mut engine, "SHOW PATTERNS").expect("SHOW must succeed");
    match result {
        ExecResult::Rows(rows) => {
            assert!(rows.is_empty(), "expected empty registry, got {} rows", rows.len());
        }
        other => panic!("expected ExecResult::Rows, got {other:?}"),
    }
}

// ─── PH6a — collision errors without OR REPLACE ─────────────────────────────

#[test]
fn ph6a_define_pattern_collision_errors_without_or_replace() {
    let mut engine = fresh_engine();
    run(&mut engine, "DEFINE PATTERN p AS x = 1").expect("first DEFINE must succeed");
    let err = run(&mut engine, "DEFINE PATTERN p AS x = 2")
        .expect_err("second DEFINE PATTERN p must error without OR REPLACE");

    let msg = err.to_lowercase();
    assert!(
        msg.contains("exists")
            || msg.contains("already")
            || msg.contains("collision")
            || msg.contains("duplicate"),
        "collision error should mention pre-existence, got: {err}"
    );
}

// ─── PH6b — OR REPLACE overwrites silently ──────────────────────────────────

#[test]
fn ph6b_define_or_replace_pattern_overwrites_silently() {
    let mut engine = fresh_engine();
    run(&mut engine, "DEFINE PATTERN p AS x = 1").expect("first DEFINE");
    run(&mut engine, "DEFINE OR REPLACE PATTERN p AS x = 2")
        .expect("DEFINE OR REPLACE must succeed without error");

    // Registry should still hold exactly one pattern named p.
    match run(&mut engine, "SHOW PATTERNS").expect("SHOW") {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 1, "OR REPLACE produced {} rows (expected 1)", rows.len());
        }
        other => panic!("expected Rows, got {other:?}"),
    }
}

// ─── PH7 — HUNT against missing USING field returns typed error ─────────────

#[test]
fn ph7_hunt_against_bundle_missing_using_field_returns_typed_error() {
    let mut engine = fresh_engine();

    // Create a bundle with field_a only. Domain-neutral naming.
    run(
        &mut engine,
        "CREATE BUNDLE things (id INT BASE, field_a FLOAT FIBER)",
    )
    .expect("CREATE BUNDLE must succeed");

    // Define a pattern that USES a field NOT present on `things`.
    run(
        &mut engine,
        "DEFINE PATTERN p AS field_b = 1 USING (field_b)",
    )
    .expect("DEFINE PATTERN must succeed");

    let err = run(&mut engine, "HUNT p IN things")
        .expect_err("HUNT must error when USING field absent on target bundle");

    let msg = err.to_lowercase();
    assert!(
        msg.contains("missing") || msg.contains("not found") || msg.contains("field_b"),
        "field-missing error should name the issue, got: {err}"
    );
    // Critical: it must NOT panic and must NOT be a generic "unimplemented"
    // — the validation must run before any "not yet executable" stub fires.
    assert!(
        !msg.contains("parser-only"),
        "PH7 must intercept BEFORE the Phase-1-only execute stub, got: {err}"
    );
}

// ─── bonus — DROP PATTERN removes from registry ─────────────────────────────

#[test]
fn drop_pattern_removes_the_pattern() {
    let mut engine = fresh_engine();
    run(&mut engine, "DEFINE PATTERN p AS x = 1").expect("DEFINE");
    run(&mut engine, "DROP PATTERN p").expect("DROP must succeed");

    match run(&mut engine, "SHOW PATTERNS").expect("SHOW") {
        ExecResult::Rows(rows) => {
            assert!(rows.is_empty(), "registry should be empty after DROP");
        }
        other => panic!("expected Rows, got {other:?}"),
    }
}

#[test]
fn drop_pattern_missing_name_is_ok_idempotent() {
    let mut engine = fresh_engine();
    // Dropping a non-existent pattern should succeed (idempotent).
    // Matches DROP TABLE IF EXISTS convention.
    run(&mut engine, "DROP PATTERN nope")
        .expect("DROP PATTERN on a missing pattern should be idempotent");
}

// ─── domain-neutrality smoke (mirrors Phase 1) ──────────────────────────────

#[test]
fn registry_serves_multiple_consumer_styles() {
    let mut engine = fresh_engine();
    // Four patterns, each from a different domain. All must coexist in
    // the same registry — the substrate doesn't specialize.
    for ddl in [
        // Vuln-hunt style:
        "DEFINE PATTERN int_overflow_alloc AS \
             has_alloc = 1 AND has_arith = 1 \
             WEIGHT (has_alloc * 3.0 + has_arith * 2.0) \
             USING (has_alloc, has_arith)",
        // Fraud-detection style:
        "DEFINE PATTERN suspicious_txn AS \
             amount > 10000 AND merchant_age_days < 30 \
             USING (amount, merchant_age_days)",
        // At-risk-student style:
        "DEFINE PATTERN attendance_concern AS \
             recent_absence_count > 3 \
             USING (recent_absence_count)",
        // Discourse-flow style:
        "DEFINE PATTERN coherence_break AS \
             transition_count > 5 \
             USING (transition_count)",
    ] {
        run(&mut engine, ddl).unwrap_or_else(|e| panic!("DEFINE failed: {e}"));
    }

    match run(&mut engine, "SHOW PATTERNS").expect("SHOW") {
        ExecResult::Rows(rows) => {
            assert_eq!(
                rows.len(),
                4,
                "registry should hold all four consumer-style patterns, got {}",
                rows.len()
            );
        }
        other => panic!("expected Rows, got {other:?}"),
    }
}

// ─── PH8 — transaction non-interaction (only with transactions feature) ─────
//
// DEFINE PATTERN is non-transactional in v0.1, mirroring the PREPARE
// precedent. BEGIN ... DEFINE PATTERN ... ROLLBACK leaves the pattern
// defined. This test compiles only when both features are on; without
// `transactions`, the gate is trivially satisfied (no BEGIN/ROLLBACK
// to interact with).

#[cfg(feature = "transactions")]
#[test]
fn ph8_define_pattern_survives_transaction_rollback() {
    let mut engine = fresh_engine();
    run(&mut engine, "BEGIN TRANSACTION").expect("BEGIN");
    run(&mut engine, "DEFINE PATTERN tx_p AS x = 1").expect("DEFINE inside tx");
    run(&mut engine, "ROLLBACK").expect("ROLLBACK");

    // After ROLLBACK, the pattern is STILL defined — DEFINE PATTERN is
    // non-transactional in v0.1.
    match run(&mut engine, "SHOW PATTERNS").expect("SHOW") {
        ExecResult::Rows(rows) => {
            let rendered = format!("{rows:?}");
            assert!(
                rendered.contains("tx_p"),
                "DEFINE PATTERN must survive ROLLBACK (non-transactional in v0.1): {rendered}"
            );
        }
        other => panic!("expected Rows, got {other:?}"),
    }
}
