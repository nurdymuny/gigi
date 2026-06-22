//! Session-scoped path-handle registry — backs the two-form
//! `LET path = IMAGINE FROM ... TO ...; INTEGRATE OBSERVABLE <name>
//! ALONG path;` syntax per Hallie's WISH ASK 4 §4 sketch
//! (2026-06-22).
//!
//! Shape mirrors `gauge::loop_transport::loops_registry` and
//! `gauge::registry::REG`: a process-wide `OnceLock<Mutex<HashMap>>`
//! keyed by the identifier the LET clause bound. Lifetimes are
//! process-scoped (the WAL has no `LetPathBind` event — paths are
//! deterministic re-computations from seed / target / bundle, so the
//! registry is treated as a cache and rebuilt on demand).
//!
//! Out of scope for this first ship:
//!   * Cross-session persistence (deferred per design note).
//!   * Per-session isolation between concurrent callers (mirrors the
//!     existing gauge / loop registries, which share the same lift).

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::imagine::provenance::ImaginedRecord;

/// A bound IMAGINE/WISH path, queryable by the identifier the LET
/// clause assigned. Stored as an immutable snapshot so multiple
/// `INTEGRATE OBSERVABLE` calls against the same handle don't have to
/// re-run the geodesic.
#[derive(Clone, Debug)]
pub struct BoundPath {
    /// Records along γ. `records[0]` is the seed; subsequent entries
    /// are the integrator-emitted ImaginedRecord trajectory. The
    /// trapezoidal line integral consumes successive pairs.
    pub records: Vec<ImaginedRecord>,
    /// Bundle name the seed came from — surfaced for downstream
    /// `evaluate_observable` dispatch if/when a `WishBundle` is
    /// registered against it.
    pub bundle: String,
    /// How the path was constructed (IMAGINE geodesic vs WISH
    /// relaxation). LetPathFromWish lands in a follow-up commit; for
    /// now only `Imagine` populates here.
    pub source: PathSource,
}

/// Construction source of a `BoundPath`. The variants matter only for
/// audit-log rendering — the integration kernel treats both the
/// same.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathSource {
    /// Bound via `LET <ident> = IMAGINE FROM ... TO ... ;`.
    Imagine,
    /// Bound via `LET <ident> = WISH FROM ... TO OBSERVABLE { ... } ;`.
    /// Reserved for the WISH follow-up; no parser arm emits this yet.
    Wish,
}

fn registry() -> MutexGuard<'static, HashMap<String, BoundPath>> {
    static REG: OnceLock<Mutex<HashMap<String, BoundPath>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("path-handle registry mutex poisoned")
}

/// Bind a path under `ident`, overwriting any previous binding for
/// that name. (Mirrors the `gauge::loop_transport::register_loop`
/// semantics — last-writer wins.)
pub fn bind(ident: impl Into<String>, bound: BoundPath) {
    registry().insert(ident.into(), bound);
}

/// Fetch a bound path. Returns `None` if `ident` was never bound (or
/// was cleared via `clear` / `unbind`).
pub fn get(ident: &str) -> Option<BoundPath> {
    registry().get(ident).cloned()
}

/// Drop a single binding. No-op if missing.
pub fn unbind(ident: &str) {
    registry().remove(ident);
}

/// Wipe every binding. Test fixtures call this in their setup to
/// keep tests isolated from each other.
pub fn clear() {
    registry().clear();
}

/// Snapshot the currently-bound identifiers — used by introspection
/// helpers + the test harness for cross-checks.
pub fn list() -> Vec<String> {
    let mut names: Vec<String> = registry().keys().cloned().collect();
    names.sort();
    names
}
