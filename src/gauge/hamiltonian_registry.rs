//! AURORA Phase 2 — Hamiltonian factory registry.
//!
//! Process-wide registry mapping a runtime name (e.g.
//! `"shallow_water_demo"`) to a `Box<dyn HamiltonianFactory>`. The
//! host binary (AURORA's downstream crate, or gigi's own
//! `gigi-stream`) calls `register()` once at startup, then the rest
//! of the system looks up by name via `with_factory()`.
//!
//! ── Q5 eager-init contract ──
//!
//! Per the AURORA Q5 resolution: NO lazy auto-population, NO
//! `OnceCell` populated on first use, NO thread-local hook. The
//! `OnceLock` here wraps only the `Mutex<HashMap>` allocation — the
//! `HashMap` inside starts empty and grows only via explicit
//! `register()` calls. `clear()` resets to the same empty state.
//!
//! ── WAL emission ──
//!
//! `register()` accepts an optional `&mut WalWriter` and, when
//! provided, emits a `HamiltonianDeclare` entry (op `0x0C`) with
//! `(name, kind_tag, group_tag, registered_at)`. WAL REPLAY of this
//! entry is explicitly OUT OF SCOPE for this workflow — the substrate
//! persists the declaration as a fact, but does not re-execute it at
//! replay (per Q5 the host binary must explicitly re-register at
//! startup; the WAL entry is for audit / introspection).
//!
//! ── Stability ──
//!
//! Every pub item carries the EVOLVING marker per
//! `docs/STABILITY_GUARANTEES.md`.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::action::HamiltonianFactory;
use crate::wal::WalWriter;

// ─────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────

/// Registry-operation error surface.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug)]
pub enum RegistryError {
    /// A factory is already registered under `name`. The substrate
    /// refuses last-write-wins by design — the first registration
    /// stays in place.
    DuplicateName { name: String },
    /// The WAL emission failed during `register()`. The registry
    /// entry is NOT inserted in this case, so a retry is safe.
    WalEmitFailed { source: std::io::Error },
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::DuplicateName { name } => write!(
                f,
                "hamiltonian registry: name '{name}' already registered \
                 (first-write-wins; clear() the registry to reset)"
            ),
            RegistryError::WalEmitFailed { source } => {
                write!(f, "hamiltonian registry: WAL emission failed: {source}")
            }
        }
    }
}

impl std::error::Error for RegistryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RegistryError::WalEmitFailed { source } => Some(source),
            _ => None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Storage
// ─────────────────────────────────────────────────────────────────────

/// Process-wide registry. `OnceLock` lazily allocates the `Mutex`
/// containing an empty `HashMap`; population happens ONLY through
/// explicit `register()` calls (Q5 eager-init contract). Initial state
/// is identical to post-`clear()` state.
static REGISTRY: OnceLock<Mutex<HashMap<String, Box<dyn HamiltonianFactory>>>> = OnceLock::new();

fn registry_lock() -> &'static Mutex<HashMap<String, Box<dyn HamiltonianFactory>>> {
    REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
}

// ─────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────

/// Register `factory` under `name`. If `wal_writer` is `Some`, emits a
/// `HamiltonianDeclare` entry to the WAL carrying
/// `(name, factory.kind_tag(), factory.group_tag(), registered_at)`.
///
/// Returns `Err(DuplicateName)` if `name` is already registered (the
/// existing entry is NOT overwritten). Returns `Err(WalEmitFailed)` if
/// WAL emission fails — in that case the in-memory entry is also NOT
/// inserted, so a retry is safe.
///
/// `registered_at` is a monotonic counter the caller supplies (the
/// gauge layer has no clock). For host binaries this is typically a
/// `u64` epoch milliseconds or a registration sequence number.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn register(
    name: impl Into<String>,
    factory: Box<dyn HamiltonianFactory>,
    wal_writer: Option<&mut WalWriter>,
    registered_at: u64,
) -> Result<(), RegistryError> {
    let name = name.into();
    let kind_tag = factory.kind_tag();
    let group_tag = factory.group_tag();

    let map = registry_lock();
    let mut guard = map.lock().expect("hamiltonian registry mutex poisoned");

    if guard.contains_key(&name) {
        return Err(RegistryError::DuplicateName { name });
    }

    // Emit WAL BEFORE inserting so that a WAL failure leaves the
    // registry untouched (retry-safe).
    if let Some(writer) = wal_writer {
        writer
            .log_hamiltonian_declare(&name, kind_tag, group_tag, registered_at)
            .map_err(|e| RegistryError::WalEmitFailed { source: e })?;
    }

    guard.insert(name, factory);
    Ok(())
}

/// Look up a factory by name and run `f` against it. Returns
/// `Some(f(...))` if registered, `None` otherwise.
///
/// Closure-pattern (rather than returning `&dyn HamiltonianFactory`)
/// keeps the mutex guard local and avoids exposing the lock lifetime
/// to callers — `HamiltonianFactory` does not need to be `Clone`.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn with_factory<R>(
    name: &str,
    f: impl FnOnce(&dyn HamiltonianFactory) -> R,
) -> Option<R> {
    let map = registry_lock();
    let guard = map.lock().expect("hamiltonian registry mutex poisoned");
    let factory = guard.get(name)?;
    Some(f(factory.as_ref()))
}

/// Returns `true` if a factory is registered under `name`.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn contains(name: &str) -> bool {
    let map = registry_lock();
    let guard = map.lock().expect("hamiltonian registry mutex poisoned");
    guard.contains_key(name)
}

/// Snapshot of every (name, kind_tag, group_tag) triple currently
/// registered. Useful for introspection HTTP endpoints and debug
/// dumps. Order is unspecified — callers that need stable order must
/// sort.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn list_registered() -> Vec<(String, &'static str, &'static str)> {
    let map = registry_lock();
    let guard = map.lock().expect("hamiltonian registry mutex poisoned");
    guard
        .iter()
        .map(|(name, factory)| (name.clone(), factory.kind_tag(), factory.group_tag()))
        .collect()
}

/// Drop every registration. Test-only knob — the production lifecycle
/// is "register once at startup, never clear".
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
pub fn clear() {
    let map = registry_lock();
    let mut guard = map.lock().expect("hamiltonian registry mutex poisoned");
    guard.clear();
}
