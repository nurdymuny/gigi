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
/// ── Dispatch table ────────────────────────────────────────────────────
///
/// - `CHERN_CLASS bundle ORDER k`: resolves `bundle` through
///   `gigi::gauge::registry::get` (the "bundle" name in the parser is
///   the gauge-field name), then looks up the bound lattice through
///   `gigi::lattice::registry::get(handle.lattice_name())`, then calls
///   `chern_weil::chern_class` with the gauge field handle as the
///   `EdgeConnection` impl.
/// - `PONTRYAGIN bundle ORDER k`: same shape, delegates to
///   `chern_weil::pontryagin_class`.
/// - `PI_1 lattice`: resolves `lattice` through
///   `gigi::lattice::registry::get`, calls
///   `topology::pi_1_presentation`, returns `pres.rank` as `Scalar`.
/// - `BETTI bundle ORDER Some(k)`: prefers the lattice registry
///   (`bundle` is the lattice name in the parser), calling
///   `topology::betti_topological` on the cell complex; falls back to
///   the engine bundle store for `k ∈ {0, 1}` (legacy graph β path).
///   `order = None` is not handled here — the legacy bundle path in
///   `gigi_stream.rs::execute_gql_on_store_read` still handles that.
/// - `OBSTRUCTION bundle`: tries the engine bundle store first (the
///   path INGEST-declared configs follow); if no bundle by that name,
///   falls through to the gauge-field path that resolves the field +
///   lattice from the registries and computes c_2 directly through
///   `chern_weil::chern_class`. This lets `GAUGE_FIELD U ON LATTICE l`
///   targets reach the kernel even when no bundle named `U` exists.
///
/// ── Why a Result instead of Option<Result> ────────────────────────────
///
/// The caller (`gql_query`) gates this with a pattern match on the 5
/// topology Statement variants — by the time this function is invoked,
/// the variant is already known to be a topology verb, so the internal
/// catch-all branch is a true error path, not a "not my variant"
/// signal. Returning `Result` keeps the call site simpler than the
/// `Option<Result>` shape `try_dispatch_gauge_statement` uses.
pub fn try_dispatch_topology_statement(
    engine: &RwLock<Engine>,
    stmt: &Statement,
) -> Result<ExecResult, String> {
    match stmt {
        // ── CHERN_CLASS bundle ORDER k ────────────────────────────────
        // `bundle` here is the gauge-field name from `GAUGE_FIELD <name>
        // ON LATTICE ... GROUP ...`. Resolve through gauge::registry,
        // not engine.bundle().
        Statement::ChernClass {
            bundle,
            order,
            fiber_fields,
            group,
        } => {
            let (handle, lat, detected_group) =
                resolve_gauge_field_and_lattice(bundle, "CHERN_CLASS")?;
            let resolved_group = group.unwrap_or(detected_group);
            let fields_owned: Vec<String> = if fiber_fields.is_empty() {
                canonical_fiber_fields(resolved_group)
            } else {
                fiber_fields.clone()
            };
            let edge_conn: &dyn crate::gauge::edge_connection::EdgeConnection =
                handle.as_ref();
            let q = crate::chern_weil::chern_class(
                edge_conn,
                &lat,
                *order,
                &fields_owned,
                Some(resolved_group),
            )
            .map_err(|e| e.to_string())?;
            Ok(ExecResult::Scalar(q))
        }

        // ── PONTRYAGIN bundle ORDER k ─────────────────────────────────
        Statement::Pontryagin {
            bundle,
            order,
            fiber_fields,
            group,
        } => {
            let (handle, lat, detected_group) =
                resolve_gauge_field_and_lattice(bundle, "PONTRYAGIN")?;
            let resolved_group = group.unwrap_or(detected_group);
            let fields_owned: Vec<String> = if fiber_fields.is_empty() {
                canonical_fiber_fields(resolved_group)
            } else {
                fiber_fields.clone()
            };
            let edge_conn: &dyn crate::gauge::edge_connection::EdgeConnection =
                handle.as_ref();
            let p = crate::chern_weil::pontryagin_class(
                edge_conn,
                &lat,
                *order,
                &fields_owned,
                Some(resolved_group),
            )
            .map_err(|e| e.to_string())?;
            Ok(ExecResult::Scalar(p))
        }

        // ── PI_1 lattice ──────────────────────────────────────────────
        // `lattice` is the lattice name. The lattice registry is the
        // only source of truth — no bundle fallback (PI_1 has no
        // concept of a bundle store).
        #[cfg(feature = "lattice")]
        Statement::Pi1 { lattice } => {
            let lat = crate::lattice::registry::get(lattice).ok_or_else(|| {
                format!(
                    "PI_1: lattice '{}' not declared (use LATTICE {} \
                     FROM ... first)",
                    lattice, lattice
                )
            })?;
            let pres = crate::topology::pi_1_presentation(&lat);
            Ok(ExecResult::Scalar(pres.rank as f64))
        }

        // ── OBSTRUCTION bundle ────────────────────────────────────────
        // Two-path dispatch: prefer the engine bundle store (the path
        // INGEST-declared configs follow), fall back to the gauge-field
        // path that computes c_2 directly through chern_weil::chern_class.
        //
        // Math-parity contract (Phase 1):
        //   - Bundle path returns `res.class as f64` — an integer sector
        //     produced by `obstruction.rs::round_with_tolerance(witness,
        //     0.25)`.
        //   - Gauge-field path computes `c_2` directly. To converge with
        //     the bundle path bit-identically when the underlying physics
        //     is the same, the fallback applies the same rounding rule
        //     here (see OBSTRUCTION_QUANT_TOL below — mirrors the value
        //     in `obstruction.rs:58`; the kernel module is locked so the
        //     constant is duplicated with a cross-ref comment).
        //
        // Semantic-divergence note (the second math lens concern):
        //   The bundle path returns a structured `ObstructionResult` with
        //   `kind` / `has_obstruction` / `witness` / `class` fields. The
        //   gauge-field path returns only the bare scalar. Until Phase 2
        //   lands a unified `obstruction_with_default_from_gauge_field`
        //   entry point, downstream consumers that need the structured
        //   shape must use INGEST (the bundle path). The scalar values
        //   converge; the structured metadata does not. Documented at
        //   the call-site rather than in the wire envelope because
        //   `ExecResult::Scalar` carries no metadata slot.
        Statement::Obstruction { bundle } => {
            // First try the engine bundle store. `obstruction_with_default`
            // does its own bundle lookup and returns BundleNotFound if
            // the name doesn't resolve.
            //
            // Engine-lock errors (`PoisonError`) fall through to the
            // gauge-field path: a poisoned bundle lock should not mask
            // a healthy gauge-field result if the caller's name resolves
            // there. Only `UnsupportedObstruction` (group/dim pair is
            // explicitly out of scope) is surfaced eagerly; `ChernWeil` /
            // `LatticeMissing` from the bundle path also fall through so
            // the gauge-field path can still answer.
            let bundle_path = match engine.read() {
                Ok(eng) => Some(crate::obstruction::obstruction_with_default(&eng, bundle)),
                Err(_) => None,
            };
            if let Some(res) = bundle_path {
                match res {
                    Ok(r) => return Ok(ExecResult::Scalar(r.class as f64)),
                    // The bundle was found but the (group, base_dim) pair
                    // is unsupported — surface that as a typed error.
                    Err(
                        e @ crate::obstruction::ObstructionError::UnsupportedObstruction { .. },
                    ) => {
                        return Err(format!("OBSTRUCTION: {}", e));
                    }
                    // Bundle not found (or any other recoverable error)
                    // — fall through to the gauge-field registry path.
                    Err(_) => {}
                }
            }

            // Gauge-field path: bundle didn't resolve (or lock was
            // poisoned), try the gauge registry. This is the path
            // GAUGE_FIELD-only declarations (no INGEST) take.
            //
            // TODO Phase 2: route the gauge-field path through a unified
            // `obstruction::obstruction_with_default_from_gauge_field(handle)`
            // entry point so both paths produce a structured
            // `ObstructionResult` (kind / has_obstruction / witness /
            // class). Named blocking precondition: that helper must
            // infer base_dim from the bound lattice's dim rather than
            // the bundle-name prefix heuristic, since gauge-field
            // declarations carry a lattice handle directly.
            //
            // The error message is custom (not the shared
            // `resolve_gauge_field_and_lattice` text) because OBSTRUCTION
            // has two valid registries to point at (INGEST → bundle,
            // GAUGE_FIELD → gauge registry) and the unresolved case
            // should name both corrective verbs.
            let handle = crate::gauge::registry::get(bundle).ok_or_else(|| {
                format!(
                    "OBSTRUCTION: no bundle or gauge field named '{}' \
                     (use INGEST ... or GAUGE_FIELD {} ON LATTICE ... first)",
                    bundle, bundle
                )
            })?;
            let lattice_name = handle.lattice_name().to_string();
            let lat = crate::lattice::registry::get(&lattice_name).ok_or_else(|| {
                format!(
                    "OBSTRUCTION: lattice '{}' bound to gauge field '{}' not \
                     found (was it declared?)",
                    lattice_name, bundle
                )
            })?;
            let g = handle.group();
            let fields_owned = canonical_fiber_fields(g);
            let edge_conn: &dyn crate::gauge::edge_connection::EdgeConnection =
                handle.as_ref();
            // Phase 1 OBSTRUCTION semantics: report the appropriate
            // characteristic-class witness as a finite scalar. The
            // chern_class kernel applies the same dimension guard as
            // the bundle path (c_2 on D<4 ⇒ 0.0), so a 2D base + an
            // identity SU(N) field both short-circuit to 0.
            let q = crate::chern_weil::chern_class(
                edge_conn,
                &lat,
                2,
                &fields_owned,
                Some(g),
            )
            .map_err(|e| format!("OBSTRUCTION: {}", e))?;
            // Apply the same round-to-integer rule the bundle path uses
            // through `obstruction.rs::round_with_tolerance(_, 0.25)`.
            // Without this, identity-field 2D SU(N) lands at 0.0 on both
            // paths but a cooled-but-not-fully-thermalized config lands
            // at the raw witness here and at the quantized integer over
            // there. Quantizing here closes the divergence.
            let class = quantize_obstruction_witness(q);
            Ok(ExecResult::Scalar(class as f64))
        }

        // ── BETTI bundle ORDER Some(k) ────────────────────────────────
        // Lattice registry first (the parser stores the lattice name in
        // the `bundle` field for the ORDER path), then engine bundle
        // fallback for the legacy β_0/β_1 graph path.
        //
        // Math-divergence note (third math lens concern):
        //   The two paths are NOT interchangeable when ∂_2 has nontrivial
        //   rank. The lattice path computes β_k from the cell complex
        //   (uses ∂_2 and ∂_3 boundary maps); the bundle path returns
        //   β_0 + β_1 from the field-index graph's pure Euler-characteristic
        //   reduction (V − E + β_0, no faces). When a lattice carries
        //   2-cells, cell-complex β_1 = graph β_1 only if rank(∂_2) = 0;
        //   cell-complex β_0 always equals graph β_0 of the 1-skeleton.
        //   This dispatcher prefers the lattice path so no double-source
        //   ambiguity arises in practice — but maintainers extending the
        //   bundle fallback should know the two paths can disagree.
        #[cfg(feature = "lattice")]
        Statement::Betti {
            bundle,
            order: Some(k),
        } => {
            if let Some(lat) = crate::lattice::registry::get(bundle) {
                let b = crate::topology::betti_topological(&lat, *k)
                    .map_err(|e| format!("BETTI: {}", e))?;
                return Ok(ExecResult::Scalar(b as f64));
            }
            // Lattice didn't resolve — fall back to the bundle store
            // for the legacy graph β path. Only k ∈ {0, 1} is supported
            // through the bundle path (the bundle stores a field-index
            // graph, not a cell complex; β_k for k ≥ 2 requires
            // ∂_2 / ∂_3 boundary-rank arithmetic the graph path can't
            // express, and even for k ∈ {0, 1} the graph β only matches
            // cell-complex β when ∂_2 has rank 0 — i.e. the lattice has
            // no 2-cells).
            let eng = engine
                .read()
                .map_err(|e| format!("BETTI: engine lock poisoned: {}", e))?;
            let store = eng.bundle(bundle).ok_or_else(|| {
                format!(
                    "BETTI ORDER {}: no lattice or bundle named '{}' \
                     (use LATTICE {} FROM ... first, or INGEST a bundle)",
                    k, bundle, bundle
                )
            })?;
            let (b0, b1) = store.betti_numbers();
            match *k {
                0 => Ok(ExecResult::Scalar(b0 as f64)),
                1 => Ok(ExecResult::Scalar(b1 as f64)),
                other => Err(format!(
                    "BETTI ORDER {} requires a registered lattice — the \
                     bundle path only supports k ∈ {{0, 1}}",
                    other
                )),
            }
        }

        // Catch-all. The route handler gates this function on a 5-arm
        // pattern match so this branch is unreachable from production —
        // it only fires when programmatic callers invoke the dispatcher
        // with an unexpected variant.
        _ => Err(format!(
            "try_dispatch_topology_statement: not a topology variant \
             (got {:?})",
            std::mem::discriminant(stmt)
        )),
    }
}

/// Canonical fiber-field name list for a gauge group. Lifted from
/// `src/bin/gigi_stream.rs::canonical_fiber_fields` so the dispatcher
/// can synthesize the same default fiber list as the legacy executor
/// arms when the caller omits `ON FIBER`.
///
/// SU(2): `["q0", "q1", "q2", "q3"]` — quaternion scalar-first.
/// SU(3): `["m00_re", "m00_im", ..., "m22_re", "m22_im"]` — 9 complex
/// entries row-major, 18 floats total.
/// U(1): `["theta"]`.
/// Z(N): `["k"]`.
fn canonical_fiber_fields(group: crate::gauge::Group) -> Vec<String> {
    match group {
        crate::gauge::Group::SU2 => {
            vec!["q0".into(), "q1".into(), "q2".into(), "q3".into()]
        }
        crate::gauge::Group::SU3 => {
            let mut out = Vec::with_capacity(18);
            for i in 0..3 {
                for j in 0..3 {
                    out.push(format!("m{i}{j}_re"));
                    out.push(format!("m{i}{j}_im"));
                }
            }
            out
        }
        crate::gauge::Group::U1 => vec!["theta".into()],
        crate::gauge::Group::ZN { .. } => vec!["k".into()],
    }
}

/// OBSTRUCTION quantization tolerance. Mirrors the private constant
/// `OBSTRUCTION_QUANT_TOL` at `src/obstruction.rs:58`. The kernel module
/// is locked (per the route-handler fix scope), so the value is
/// duplicated here with a cross-reference rather than re-exported.
///
/// Provenance is documented in the kernel's docstring: 0.25 is the
/// empirical envelope where the Phase 1 calibrated SU(N) signature
/// lands on the synthetic single-instanton seed; it is a gate
/// threshold, NOT a topological convergence criterion. Phase 2's
/// Lüscher 16-plaquette clover charge will let the tolerance tighten
/// to ~0.05 on thermalized configs.
const OBSTRUCTION_QUANT_TOL: f64 = 0.25;

/// Round an OBSTRUCTION witness scalar to the integer sector the bundle
/// path would produce. Mirrors `obstruction.rs::round_with_tolerance`
/// at the locked kernel boundary so the gauge-field fallback path in
/// `try_dispatch_topology_statement` converges bit-identically with
/// the bundle path on (gauge_field, lattice) pairs that name the same
/// physical configuration through both registries.
///
/// The tolerance argument is currently unused (the kernel helper at
/// `obstruction.rs:517` ignores it too — it's documented as a
/// quantization-gap diagnostic for the structured `ObstructionResult`
/// path, not as a thresholded rounding rule). Keeping the parameter
/// preserves the kernel's signature for the Phase 2 unification.
fn quantize_obstruction_witness(q: f64) -> i64 {
    let _ = OBSTRUCTION_QUANT_TOL; // see docstring — kept for parity
    q.round() as i64
}

/// Resolve a gauge-field name to (handle, bound lattice, group) in one
/// shot, with a verb-tagged error message that names the missing
/// registry + the corrective verb. Shared by `CHERN_CLASS`,
/// `PONTRYAGIN`, and the `OBSTRUCTION` gauge-field fallback path so
/// the per-variant arms drop ~12 lines of registry-resolve prelude.
///
/// `verb` is the leading token of the GQL statement
/// (`"CHERN_CLASS"` / `"PONTRYAGIN"` / `"OBSTRUCTION"`) so error
/// messages carry the same verb the caller typed.
///
/// The returned handle is the same `Arc<dyn GaugeFieldHandle>` the
/// registry stores; callers borrow `handle.as_ref()` to get a
/// `&dyn EdgeConnection` for the kernel calls (`GaugeFieldHandle`
/// has `EdgeConnection` as a supertrait, so `as_ref()` coerces).
fn resolve_gauge_field_and_lattice(
    name: &str,
    verb: &str,
) -> Result<
    (
        std::sync::Arc<dyn crate::gauge::registry::GaugeFieldHandle>,
        crate::lattice::Lattice,
        crate::gauge::Group,
    ),
    String,
> {
    let handle = crate::gauge::registry::get(name).ok_or_else(|| {
        format!(
            "{}: gauge field '{}' not declared (use GAUGE_FIELD {} ON \
             LATTICE ... first)",
            verb, name, name
        )
    })?;
    let lattice_name = handle.lattice_name().to_string();
    let lat = crate::lattice::registry::get(&lattice_name).ok_or_else(|| {
        format!(
            "{}: lattice '{}' bound to gauge field '{}' not found \
             (was it declared?)",
            verb, lattice_name, name
        )
    })?;
    let group = handle.group();
    Ok((handle, lat, group))
}
