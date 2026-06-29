//! Halcyon Part V P-1 — `/v1/gql` dispatch helper for gauge-feature
//! Statements.
//!
//! Closes the surface that `HALCYON_PART_V_SNAPSHOT_GATES.md` §2.5
//! flagged: `src/bin/gigi_stream.rs::gql_query` previously routed
//! gauge-feature statements (LATTICE / GAUGE_FIELD / SHOW GAUGE_FIELD
//! / GIBBS_SAMPLE / E_FIELD / SYMPLECTIC_FLOW / SHOW E_FIELD /
//! SELECT H_TOTAL / SELECT GAUSS_RESIDUAL_MAX / SELECT PLAQUETTE /
//! SELECT Q_SURROGATE / SHOW LATTICE / LATTICE FROM TRUNCATED_ICOSAHEDRON)
//! through `get_bundle_name(&stmt)`, which returned `None` for the
//! whole gauge family because none of them are bound to a single
//! GIGI bundle. The early-return path then emitted
//! `{"status":"ok"}` without ever calling `parser::execute`, so the
//! declaration silently dropped on the floor.
//!
//! This helper is the testable boundary the binary's `gql_query`
//! now consults BEFORE the bundle-name extraction. When the
//! statement is one of the gauge-feature variants, the helper
//! drives `parser::execute` against the supplied engine handle
//! and the process-global `lattice_registry` / `gauge_registry`
//! singletons. The caller (`gql_query`) lowers the
//! `Result<ExecResult, String>` through `exec_result_to_response`
//! for the JSON envelope.
//!
//! Optionality contract: every consumer of this module is
//! `#[cfg(feature = "gauge")]`-gated; the no-default-features
//! build does not see the module at all, so the 852/0 byte-identical
//! receipt for the optionality contract stays intact.

#![cfg(feature = "gauge")]

use std::sync::RwLock;

use crate::engine::Engine;
use crate::parser::{execute, ExecResult, Statement};

/// Returns `Some(...)` when `stmt` is a gauge-feature variant the
/// `/v1/gql` POST endpoint must dispatch through `parser::execute`,
/// `None` otherwise. The caller is the dispatcher of last resort —
/// when this returns `None`, the existing bundle-aware path in
/// `gql_query` takes over.
///
/// The 14 variants this matches are exactly the set the spec §2.5
/// names plus the implied siblings the receipt step 1 actually
/// reaches (`LatticeFromCanonical` is the variant the
/// `LATTICE bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';`
/// receipt parses into; `ShowLattice`, `SelectPlaquette`, and
/// `SelectQSurrogate` round out the gauge-substrate surface so a
/// follow-up Part V verb does not re-hit the same drop bug) plus
/// the new V.2 `Statement::Snapshot` arm (extends 13 → 14; locked
/// decision D-V-B keeps SNAPSHOT on `/v1/gql` only — no dedicated
/// HTTP route).
///
/// All variants are gated on `feature = "gauge"` upstream (Statement
/// definitions in `src/parser.rs` carry the same gate); the `cfg`
/// header on this module mirrors that so the helper compiles only
/// when the surface is reachable.
///
/// Hand-off contract: the engine handle is mutated by two arms now —
/// `GaugeField { persist: true, .. }` (durable PERSIST routing
/// through `engine.declare_gauge_field_durable`) and
/// `Snapshot { .. }` (always durable per locked decision D-V-D —
/// routes through `engine.snapshot_gauge_field_durable` which writes
/// `WalEntry::GaugeFieldSnapshot` and returns the SHA-256 + offset).
/// Every other arm operates on the process-global registries. The
/// caller passes the `Arc<RwLock<Engine>>` through unchanged so the
/// existing `StreamState::engine` field works without any signature
/// change.
pub fn try_dispatch_gauge_statement(
    engine: &RwLock<Engine>,
    stmt: &Statement,
) -> Option<Result<ExecResult, String>> {
    let matches_gauge_family = matches!(
        stmt,
        Statement::Lattice { .. }
            | Statement::LatticeFromCanonical { .. }
            | Statement::ShowLattice { .. }
            | Statement::GaugeField { .. }
            | Statement::ShowGaugeField { .. }
            | Statement::SelectPlaquette { .. }
            | Statement::SelectQSurrogate { .. }
            | Statement::GibbsSample { .. }
            | Statement::EField { .. }
            | Statement::SymplecticFlow { .. }
            | Statement::ShowEField { .. }
            | Statement::SelectHTotal { .. }
            | Statement::SelectGaussResidualMax { .. }
            | Statement::Snapshot { .. }
            | Statement::LoopDecl { .. }
            | Statement::LoopTransport { .. }
    );

    if !matches_gauge_family {
        return None;
    }

    // Acquire a write lock so the durable arms (declare-PERSIST and
    // SNAPSHOT) can route through `engine.declare_gauge_field_durable`
    // / `engine.snapshot_gauge_field_durable`. The non-durable arms
    // touch process-global registries and ignore the engine; the cost
    // of holding the write lock during a registry-only statement is a
    // few microseconds at most — acceptable for a P0 fix that closes
    // a real correctness bug.
    let mut eng = engine.write().expect("engine lock poisoned");
    Some(execute(&mut eng, stmt))
}

/// Halcyon Bridge Trilogy follow-up — topology-verb route-handler bypass.
///
/// Hallie's smoke chain (2026-06-28, gigi-stream a1c9c57) caught a
/// pre-resolve drop bug in `src/bin/gigi_stream.rs::gql_query`: the
/// bundle-name extraction at the top of the route handler fires
/// BEFORE the executor is dispatched, so the 5 topology verbs
/// (CHERN_CLASS / PONTRYAGIN / BETTI ORDER k / PI_1 / OBSTRUCTION)
/// hit either `{"error":"No bundle: <gauge-field-or-lattice>"}` (when
/// `get_bundle_name(&stmt)` returns the gauge/lattice name) or a
/// silent `{"status":"ok"}` envelope (when it returns `None`). The
/// fix mirrors `try_dispatch_gauge_statement`: a special-case dispatch
/// the route handler consults BEFORE the bundle pre-resolve, so the
/// topology verbs reach their kernels (`chern_weil::chern_class`,
/// `chern_weil::pontryagin_class`, `topology::betti_topological`,
/// `topology::pi_1_presentation`, `obstruction::obstruction_with_default`)
/// without ever touching the bundle registry.
///
/// **RED-PHASE STUB** — the GREEN commit will replace this body with
/// the per-variant dispatch logic that consults
/// `gigi::gauge::registry` / `gigi::lattice::registry` / the engine
/// bundle store as appropriate. For now this returns
/// `Err("try_dispatch_topology_statement: not implemented")` so the
/// integration tests at `tests/topology_verbs_gql_integration.rs`
/// compile but every assertion lands on a failing-Err branch.
pub fn try_dispatch_topology_statement(
    engine: &RwLock<Engine>,
    stmt: &Statement,
) -> Result<ExecResult, String> {
    // RED phase: signature pinned for the integration tests to call,
    // but the dispatch logic does not exist yet. Suppress unused-arg
    // warnings without changing the contract.
    let _ = (engine, stmt);
    Err("try_dispatch_topology_statement: not implemented (RED phase)".to_string())
}
