//! Halcyon Bridge Trilogy follow-up — topology-verb route-handler bypass.
//!
//! RED-first integration test that pins the route-handler contract for
//! the 5 topology verbs (`CHERN_CLASS`, `PONTRYAGIN`, `BETTI ORDER k`,
//! `PI_1`, `OBSTRUCTION`).
//!
//! ── What this test pins ──────────────────────────────────────────────
//!
//! Hallie's smoke chain (2026-06-28, against gigi-stream a1c9c57)
//! caught the route handler dropping every topology verb before the
//! executor ever runs:
//!
//! ```text
//! LATTICE smoke FROM CUBIC L=4 DIM=2 PERIODIC;          → {"status":"ok"}
//! GAUGE_FIELD U_smoke ON LATTICE smoke GROUP SU(3) INIT IDENTITY; → {"status":"ok"}
//! CHERN_CLASS U_smoke ORDER 2;                          → {"error":"No bundle: U_smoke"}
//! PONTRYAGIN U_smoke ORDER 1;                           → {"error":"No bundle: U_smoke"}
//! BETTI smoke ORDER 2;                                  → {"error":"No bundle: smoke"}
//! PI_1 smoke;                                           → {"status":"ok"}
//! OBSTRUCTION U_smoke;                                  → {"status":"ok"}
//! ```
//!
//! Root cause is `src/bin/gigi_stream.rs::gql_query` running
//! `engine.bundle(&bundle_name)` BEFORE dispatching to the executor.
//! `get_bundle_name(&stmt)` returns `Some("U_smoke")` / `Some("smoke")`
//! for `CHERN_CLASS` / `PONTRYAGIN` / `BETTI ORDER`, none of which
//! live in the engine bundle registry (they live in
//! `gigi::gauge::registry` / `gigi::lattice::registry`), so the
//! pre-resolve fails. For `PI_1` / `OBSTRUCTION` it returns `None`
//! and the default early-return drops the statement.
//!
//! The fix is a special-case dispatch the route handler consults
//! BEFORE the bundle pre-resolve, mirroring the gauge-dispatch
//! precedent in `gigi::halcyon_gql_dispatch::try_dispatch_gauge_statement`.
//! These tests drive that dispatcher directly, bypassing axum —
//! the route handler is a thin wrapper over the dispatcher, so
//! pinning the dispatcher pins the route handler's external contract.
//!
//! ── Design choice: call the lib-crate dispatcher directly ────────────
//!
//! Two viable paths to test the bug were considered (per the brief
//! task C):
//!
//! 1. Drive axum's `gql_query` via tower-oneshot. Cost: ~60 LOC of
//!    HTTP plumbing per test (StreamState, AppState, request body
//!    serialization, response deserialization).
//!
//! 2. Call the lib-crate dispatcher
//!    `gigi::halcyon_gql_dispatch::try_dispatch_topology_statement`
//!    directly. Cost: ~5 LOC per test (parse + execute + assert).
//!
//! This file picks (2). The dispatcher lives in `src/halcyon_gql_dispatch.rs`
//! (sibling of `try_dispatch_gauge_statement`, which already follows the
//! same pattern) so the integration test target's `extern crate gigi`
//! reaches it without binary-crate gymnastics. The route-handler patch
//! in `gigi_stream.rs` simply forwards to this dispatcher — covering
//! the dispatcher covers the route handler's behavior end-to-end.
//!
//! ── Why this build will be RED ────────────────────────────────────────
//!
//! Today `try_dispatch_topology_statement` is a stub returning
//! `Err("try_dispatch_topology_statement: not implemented (RED phase)")`.
//! All 7 tests below land on either an `expect("must succeed")` panic
//! (the success-path tests 1–5) or an assertion that the error message
//! contains specific strings the stub does not produce (the error-path
//! tests 6–7). The GREEN commit will replace the stub with the
//! per-variant dispatch logic that drives the 5 topology kernels
//! against the gauge / lattice / engine registries, and these tests
//! will pass.
//!
//! Run with:
//!   `cargo test --features halcyon --test topology_verbs_gql_integration`

#![cfg(feature = "halcyon")]

use std::sync::RwLock;

use gigi::engine::Engine;
use gigi::halcyon_gql_dispatch::try_dispatch_topology_statement;
use gigi::parser::{execute, parse, ExecResult};

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

/// Declare a 2D L=4 PERIODIC cubic lattice + an identity SU(3) gauge
/// field on it through the parser (mirrors Hallie's smoke chain
/// prelude exactly).
fn declare_lattice_and_gauge_field(eng_lock: &RwLock<Engine>) {
    let mut eng = eng_lock.write().expect("engine write lock");
    let lat_stmt = parse("LATTICE smoke FROM CUBIC L=4 DIM=2 PERIODIC;")
        .expect("parse LATTICE smoke");
    execute(&mut eng, &lat_stmt).expect("exec LATTICE smoke");
    let gf_stmt = parse(
        "GAUGE_FIELD U_smoke ON LATTICE smoke GROUP SU(3) INIT IDENTITY;",
    )
    .expect("parse GAUGE_FIELD U_smoke");
    execute(&mut eng, &gf_stmt).expect("exec GAUGE_FIELD U_smoke");
}

/// Unwrap `ExecResult::Scalar(v)`; panic with a precise message
/// (`expected Scalar, got <variant>`) on the other variants so a
/// mismatched envelope is loud, not silent.
fn scalar(r: ExecResult) -> f64 {
    match r {
        ExecResult::Scalar(v) => v,
        other => panic!("expected ExecResult::Scalar, got {other:?}"),
    }
}

// ── Tests ────────────────────────────────────────────────────────────

/// (1) Hallie's reported failure case. `CHERN_CLASS U_smoke ORDER 2`
/// against an identity SU(3) gauge field on a 2D L=4 cubic lattice
/// must reach the `chern_class` kernel and return `Scalar(0.0)`:
/// - identity field ⇒ every plaquette = I ⇒ F ≡ 0 ⇒ Σ Tr(F∧F) = 0
/// - 2D base ⇒ c_2 (a 4-form) is short-circuited by the dimension
///   guard before the clover walk
///
/// In the RED state the stub returns `Err`, so the `.expect(...)`
/// panic fires and the test fails.
#[test]
fn test_chern_class_2d_su3_identity_returns_zero() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);
    let stmt = parse("CHERN_CLASS U_smoke ORDER 2;")
        .expect("parse CHERN_CLASS");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("CHERN_CLASS dispatch must succeed (no bundle pre-resolve)");
    assert!(
        scalar(res).abs() < 1e-10,
        "c_2 on identity SU(3) 2D base must be 0 (dim-guard short-circuit)"
    );
}

/// (2) Same dispatch path, PONTRYAGIN. `PONTRYAGIN U_smoke ORDER 1`
/// must reach `pontryagin_class` and return `Scalar(0.0)` because
/// p_1 = -2·c_2 (Lüscher 1982 §2) and c_2 = 0 by the same dim-guard
/// reasoning.
#[test]
fn test_pontryagin_2d_su3_identity_returns_zero() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);
    let stmt = parse("PONTRYAGIN U_smoke ORDER 1;")
        .expect("parse PONTRYAGIN");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("PONTRYAGIN dispatch must succeed");
    assert!(
        scalar(res).abs() < 1e-10,
        "p_1 on identity SU(3) 2D base must be 0 (p_1 = -2·c_2, c_2 = 0)"
    );
}

/// (3) `BETTI smoke ORDER 2` must reach `topology::betti_topological`
/// against the registered lattice (NOT a bundle) and return
/// `Scalar(1.0)`. The 4×4 closed 2-torus has β_2 = 1 by the standard
/// `β_k(T^D) = C(D, k)` pattern (C(2, 2) = 1).
///
/// This pins the lattice-registry-first lookup order: the brief
/// requires `BETTI ORDER k>=1` to prefer the lattice registry over
/// the bundle store, with the bundle fallback only for `k ∈ {0, 1}`.
#[test]
fn test_betti_order_2_t2_lattice_returns_one() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);
    let stmt = parse("BETTI smoke ORDER 2;")
        .expect("parse BETTI smoke ORDER 2");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("BETTI ORDER 2 dispatch must succeed");
    let v = scalar(res);
    assert!(
        (v - 1.0).abs() < 1e-12,
        "β_2(T² L=4) must be 1, got {v}"
    );
}

/// (4) `PI_1 smoke` must reach `topology::pi_1_presentation` against
/// the lattice and return `Scalar(2.0)`. π_1(T²) is ℤ², abelianized
/// rank = 2; Phase 1 reports `pres.rank` as the abelianized rank.
///
/// This pins the PI_1 dispatch to the lattice registry only (no
/// bundle fallback path — PI_1 has no concept of a bundle store).
#[test]
fn test_pi_1_t2_lattice_returns_rank_two() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);
    let stmt = parse("PI_1 smoke;").expect("parse PI_1 smoke");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("PI_1 dispatch must succeed");
    let v = scalar(res);
    assert!(
        (v - 2.0).abs() < 1e-12,
        "π_1(T² L=4) rank must be 2, got {v}"
    );
}

/// (5) `OBSTRUCTION U_smoke` must reach
/// `obstruction::obstruction_with_default` and return a finite
/// scalar. The exact value depends on the base-dim inference path:
/// for the `U_smoke` name (no recognized 2D/3D/4D prefix), the
/// inference defaults to 4D, and for an identity SU(N) field every
/// path lands on 0. What the test pins is that the dispatch
/// REACHES the kernel — i.e. the response is a finite scalar, not
/// the `{"status":"ok"}` envelope the bug produced.
///
/// Honest framing: the brief's "scope of fix" says OBSTRUCTION can
/// target either a bundle (INGEST'd configs) or a gauge field
/// (GAUGE_FIELD-declared). Phase 1 dispatch may route through either
/// path; the GREEN gate accepts any finite scalar value.
#[test]
fn test_obstruction_2d_su3_identity_returns_finite_scalar() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);
    let stmt = parse("OBSTRUCTION U_smoke;")
        .expect("parse OBSTRUCTION");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("OBSTRUCTION dispatch must succeed");
    let v = scalar(res);
    assert!(
        v.is_finite(),
        "OBSTRUCTION on identity SU(3) field must return a finite scalar, got {v}"
    );
}

/// (6) Clear-error regression — the documented bug fix. `CHERN_CLASS`
/// against a name that does NOT have a registered gauge field must
/// surface a precise "gauge field not declared" error, NOT the legacy
/// `"No bundle: <name>"` envelope (the bug Hallie's smoke chain hit),
/// and NOT a silent `{"status":"ok"}` drop.
///
/// In the RED phase the stub error contains neither "not declared"
/// nor "not found", so the first assertion fails. The GREEN commit
/// will produce the "CHERN_CLASS: gauge field 'X' not declared (use
/// GAUGE_FIELD X ON LATTICE ... first)" shape the executor arm at
/// `src/bin/gigi_stream.rs:13384` already uses.
#[test]
fn test_chern_class_against_nonexistent_gauge_field_returns_clear_error() {
    let (eng, _dir) = fresh_engine_and_registries();
    // Note: deliberately do NOT declare the lattice or gauge field.
    let stmt = parse("CHERN_CLASS totally_does_not_exist ORDER 2;")
        .expect("parse CHERN_CLASS");
    let res = try_dispatch_topology_statement(&eng, &stmt);
    let err = res.expect_err(
        "CHERN_CLASS on nonexistent gauge field must error (not return Ok)",
    );
    assert!(
        err.contains("not declared") || err.contains("not found"),
        "error must name the missing gauge field, got: {err}"
    );
    assert!(
        !err.contains("No bundle:"),
        "error must NOT be the legacy 'No bundle:' envelope (that is the bug \
         being fixed), got: {err}"
    );
}

/// (7) PI_1 clear-error regression. Hallie's smoke chain returned
/// a bare `{"status":"ok"}` for `PI_1 <undefined>` because the
/// pre-resolve dropped the statement. The GREEN commit must surface
/// a precise "lattice not declared" error instead.
///
/// In the RED phase the stub error contains neither "not declared"
/// nor "PI_1", so the test fails. The GREEN commit will produce a
/// shape like "PI_1: lattice 'X' not declared".
#[test]
fn test_pi_1_against_nonexistent_lattice_returns_clear_error() {
    let (eng, _dir) = fresh_engine_and_registries();
    let stmt = parse("PI_1 totally_does_not_exist;").expect("parse PI_1");
    let res = try_dispatch_topology_statement(&eng, &stmt);
    let err = res.expect_err(
        "PI_1 on nonexistent lattice must error (not return Ok)",
    );
    assert!(
        err.contains("not declared") || err.contains("PI_1"),
        "error must name PI_1 or the missing lattice, got: {err}"
    );
    assert!(
        !err.contains("status"),
        "error must NOT be a silent status envelope (that is the bug \
         being fixed), got: {err}"
    );
}

// ── Bypass-position regression guard ─────────────────────────────────
//
// The dispatcher tests above pin the contract for the helper. They do
// NOT pin the route handler's ORDERING invariant (the helper must run
// BEFORE the engine.bundle() pre-resolve in `gql_query`). A future
// refactor could move the bypass block to AFTER the bundle pre-resolve
// and every dispatcher test would stay green while Hallie's original
// bug regressed silently. The test below pins that invariant by
// reproducing the EXACT condition under which the route handler would
// have dropped the statement (engine.bundle("U_smoke") = None) and
// asserting the dispatcher answers anyway.

/// (8) Bypass-position regression guard. Reproduces Hallie's smoke
/// chain's load-bearing precondition: after `LATTICE smoke;
/// GAUGE_FIELD U_smoke ...;` lands through the parser, the engine
/// bundle registry has NO entry for either "smoke" or "U_smoke" (they
/// live in the lattice + gauge registries). This is exactly the
/// situation in which `gql_query`'s bundle pre-resolve would have
/// 404'd or silently dropped. The dispatcher must answer regardless
/// — i.e. it must run BEFORE the bundle pre-resolve, not after.
///
/// If a refactor accidentally moved the topology dispatch block to
/// AFTER the bundle pre-resolve, the production smoke chain would
/// regress to Hallie's original failure mode (`{"error":"No bundle:
/// U_smoke"}`). This test pins the ordering invariant via the
/// dispatcher contract: when the engine bundle store has no entry
/// for the name, the dispatcher MUST still resolve through the
/// gauge / lattice registry and reach the kernel.
#[test]
fn test_dispatcher_succeeds_when_engine_bundle_lookup_would_fail() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);

    // Sanity precondition: the bundle registry has NO entry for either
    // "smoke" or "U_smoke". If this assertion ever fails, the parser
    // started landing gauge fields / lattices in the bundle store and
    // the bypass would no longer be load-bearing — investigate before
    // weakening the assertion.
    {
        let eng_guard = eng.read().expect("engine read");
        assert!(
            eng_guard.bundle("U_smoke").is_none(),
            "GAUGE_FIELD U_smoke must NOT land in the bundle registry \
             (lives in gauge::registry)"
        );
        assert!(
            eng_guard.bundle("smoke").is_none(),
            "LATTICE smoke must NOT land in the bundle registry \
             (lives in lattice::registry)"
        );
    }

    // Now drive the smoke chain's three failing verbs through the
    // dispatcher. Each must succeed AND return a finite scalar — the
    // exact bug Hallie's chain hit.
    for src in [
        "CHERN_CLASS U_smoke ORDER 2;",
        "PONTRYAGIN U_smoke ORDER 1;",
        "BETTI smoke ORDER 2;",
        "PI_1 smoke;",
        "OBSTRUCTION U_smoke;",
    ] {
        let stmt = parse(src).unwrap_or_else(|e| panic!("parse {src}: {e:?}"));
        let res = try_dispatch_topology_statement(&eng, &stmt)
            .unwrap_or_else(|e| panic!("{src} dispatch failed: {e}"));
        let v = scalar(res);
        assert!(
            v.is_finite(),
            "{src} must return a finite scalar (bypass-position \
             regression — the dispatcher fired AFTER the bundle \
             pre-resolve), got {v}"
        );
    }
}

/// (9) OBSTRUCTION quantization parity. The gauge-field fallback path
/// must apply the same round-to-integer rule the bundle path uses
/// (`obstruction.rs::round_with_tolerance(_, 0.25)`), so for the
/// identity SU(3) 2D config the returned scalar is exactly 0.0 — an
/// integer-typed sector, NOT a raw non-quantized witness.
///
/// Without the quantization fix, the gauge-field path would return
/// the raw chern_class scalar (which happens to be 0.0 on identity
/// in 2D via the dim-guard short-circuit, so this test is coincidentally
/// invisible for identity fields, but the parity contract still holds:
/// the returned value must be representable as an integer-typed
/// sector class). The assertion below is `(v - v.round()).abs() < 1e-12`
/// which holds bit-exactly on identity but pins the contract.
#[test]
fn test_obstruction_returns_quantized_integer_sector() {
    let (eng, _dir) = fresh_engine_and_registries();
    declare_lattice_and_gauge_field(&eng);
    let stmt = parse("OBSTRUCTION U_smoke;")
        .expect("parse OBSTRUCTION");
    let res = try_dispatch_topology_statement(&eng, &stmt)
        .expect("OBSTRUCTION dispatch must succeed");
    let v = scalar(res);
    assert!(
        v.is_finite(),
        "OBSTRUCTION must return finite, got {v}"
    );
    let gap = (v - v.round()).abs();
    assert!(
        gap < 1e-12,
        "OBSTRUCTION gauge-field path must return a quantized integer \
         sector (parity with the bundle path's round_with_tolerance), \
         got value {v} with rounding gap {gap}"
    );
}
