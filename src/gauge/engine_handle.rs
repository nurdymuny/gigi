//! Module-global engine handle for the LATTICE + GAUGE_FIELD HTTP
//! surface (TDD-HAL-II.6b).
//!
//! The HTTP router built by `gauge::http::build_router` is intentionally
//! stateless — Bee's locked decision 4 (reach = both embedded + HTTP)
//! and the existing II.6 surface both lean on the process-singleton
//! lattice + gauge registries (`lattice::registry`, `gauge::registry`).
//! When II.6b adds the `persist: bool` request-body field, the handler
//! has to reach an owned `&mut Engine` to call
//! `declare_lattice_durable` / `declare_gauge_field_durable`. Threading
//! the engine through axum State would force every consumer of
//! `build_router::<S>()` to rebuild around a concrete state type and
//! would break the existing test harness in
//! `tests/halcyon_part_ii_http.rs` which uses `build_router::<()>()`.
//!
//! Option (b) from the II.6b discovery: a process-global handle the
//! gigi-stream binary installs once at startup. Matches the
//! lattice + gauge registry pattern (mirrors of `OnceLock`-style
//! singletons that live alongside the rest of the gauge module).
//!
//! Companion to the engine handle: an authoritative set of names of
//! lattices that were declared durably *through the HTTP surface*. The
//! engine itself does not track this — its WAL contains a
//! `LatticeDeclare` entry per durable declaration, but scanning the WAL
//! on every gauge-field POST would be wasteful, and (more importantly)
//! at replay time `Engine::open` re-populates `lattice::registry` from
//! WAL alone, so we cannot distinguish in-memory-only vs durable by
//! inspecting the registry. Tracking it here at decl time is the cheap
//! fix: the HTTP gauge-field handler can fail fast (gate (e)) when a
//! `persist: true` field references a non-durable lattice.
//!
//! Bee's locked decision 8: the engine_handle module-global is
//! installed by `gigi_stream::main` exactly once, alongside `Engine::open`
//! and the readiness atomic. Tests use `clear_for_test()` to reset
//! between cases.

use crate::engine::Engine;
use std::collections::HashSet;
use std::io;
use std::sync::{Arc, Mutex, OnceLock, RwLock};

/// Internal state: an optional engine handle plus the set of durably-
/// declared lattice names tracked at the HTTP layer.
struct HandleState {
    engine: Option<Arc<RwLock<Engine>>>,
    durable_lattices: HashSet<String>,
}

impl HandleState {
    fn new() -> Self {
        Self {
            engine: None,
            durable_lattices: HashSet::new(),
        }
    }
}

fn state() -> &'static Mutex<HandleState> {
    static STATE: OnceLock<Mutex<HandleState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(HandleState::new()))
}

/// Install the engine handle. Idempotent: re-installing replaces the
/// previous handle and clears the durable-lattice tracker, which
/// matches the "fresh process" semantics tests rely on. Production
/// (gigi-stream) calls this exactly once after `Engine::open` succeeds.
pub fn install(engine: Arc<RwLock<Engine>>) -> io::Result<()> {
    let mut s = state()
        .lock()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("engine_handle poisoned: {e}")))?;
    s.engine = Some(engine);
    s.durable_lattices.clear();
    Ok(())
}

/// Run a closure against the installed engine under its `RwLock` write
/// guard. Returns `None` if no engine has been installed yet (the HTTP
/// handler treats this as a 500 with a "no engine" message). The inner
/// closure's return value is propagated as the handler's result.
pub fn with_engine_mut<R>(f: impl FnOnce(&mut Engine) -> R) -> Option<R> {
    let s = state().lock().ok()?;
    let engine = s.engine.clone()?;
    drop(s);
    let mut guard = engine.write().ok()?;
    Some(f(&mut *guard))
}

/// Record that a lattice has been declared durably through the HTTP
/// surface. The gauge-field POST handler uses this to fail fast (gate
/// (e)) when `persist: true` references a non-durable lattice.
pub fn mark_lattice_durable(name: &str) {
    if let Ok(mut s) = state().lock() {
        s.durable_lattices.insert(name.to_string());
    }
}

/// Was this lattice declared durably through the HTTP surface in the
/// current process lifetime?
pub fn is_lattice_durable(name: &str) -> bool {
    state()
        .lock()
        .map(|s| s.durable_lattices.contains(name))
        .unwrap_or(false)
}

/// Reset both the engine handle and the durable-lattice tracker.
/// Tests call this between cases to simulate a fresh process. Not used
/// in production paths.
pub fn clear_for_test() {
    if let Ok(mut s) = state().lock() {
        s.engine = None;
        s.durable_lattices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Install a stub engine + read it back via `with_engine_mut`.
    /// Verifies the OnceLock plumbing actually wires the handle through.
    #[test]
    fn tdd_hal_ii_6b_install_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let engine = Arc::new(RwLock::new(Engine::open(dir.path()).expect("engine open")));
        install(engine.clone()).expect("install");

        let saw = with_engine_mut(|e| e.bundle_names().len());
        assert!(saw.is_some(), "engine must be reachable after install");

        clear_for_test();
        let after_clear = with_engine_mut(|_| 1);
        assert!(after_clear.is_none(), "clear must drop the handle");
    }

    /// Durable-lattice tracker: mark + look up.
    #[test]
    fn tdd_hal_ii_6b_durable_lattice_tracker() {
        clear_for_test();
        assert!(!is_lattice_durable("nope"));
        mark_lattice_durable("bb");
        assert!(is_lattice_durable("bb"));
        clear_for_test();
        assert!(
            !is_lattice_durable("bb"),
            "clear must drop the durable-lattice set"
        );
    }
}
