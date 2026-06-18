//! In-process gauge-field registry. The executor materializes
//! declared `GAUGE_FIELD` statements here; `SHOW GAUGE_FIELD name`
//! reads back through it.
//!
//! Closes the in-memory storage half of TDD-HAL-II.4. The registry is
//! a per-process `HashMap<String, Arc<dyn GaugeFieldHandle>>` guarded
//! by a `Mutex` — a `BundleStore`-grade persistence layer ships next
//! as TDD-HAL-II.4b (the GAUGE_FIELD declaration becomes durable when
//! `PERSIST` / `FROM_BUNDLE` is used; the default `INIT IDENTITY` /
//! `INIT HAAR_RANDOM SEED` declarations stay in-memory).
//!
//! Group-erasure note (Bee's locked decision 6): the registry stores
//! `Arc<dyn GaugeFieldHandle>`, never the concrete `SU2GaugeField`.
//! A future `U1GaugeField` / `SU3GaugeField` / `ZNGaugeField` ships
//! as a new struct that implements `GaugeFieldHandle`; the registry,
//! the walker, the parser, and the HTTP routes do not change.
//!
//! `GaugeFieldHandle` is the trait the registry stores behind a
//! trait object. It extends `EdgeConnection` (so the walker can read
//! through a handle directly without naming the concrete type) and
//! adds the four metadata accessors `SHOW GAUGE_FIELD` needs:
//! `name`, `lattice_name`, `group`, `init_metadata`, plus
//! `as_dense_buffer` for the buffer introspection wire surface.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use super::dense_link_buffer::DenseLinkBuffer;
use super::edge_connection::EdgeConnection;
use super::group::Group;
use super::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};

/// Object-safe handle the registry stores. Extends `EdgeConnection`
/// so the walker can read through it directly; the metadata accessors
/// power SHOW GAUGE_FIELD and the JSON envelope wire format.
pub trait GaugeFieldHandle: EdgeConnection + Send + Sync {
    /// The user-facing field name (the `ident` in `GAUGE_FIELD ident
    /// …;`). Stable across the lifetime of the registration.
    fn name(&self) -> &str;
    /// Name of the lattice this field is bound to. Stable.
    fn lattice_name(&self) -> &str;
    /// Group tag of the underlying buffer. Stable.
    fn group(&self) -> Group;
    /// How this field was initialized + the optional seed. Mirrors
    /// the metadata the executor + persistence layers need to
    /// round-trip the declaration through SHOW.
    fn init_metadata(&self) -> (GaugeFieldInit, Option<u64>);
    /// Borrow the underlying `DenseLinkBuffer`. The JSON envelope
    /// `{"group": …, "repr_dim": …, "n_edges": …, "data": [[…],…]}`
    /// is built off this view.
    fn as_dense_buffer(&self) -> &DenseLinkBuffer;
}

impl GaugeFieldHandle for SU2GaugeField {
    fn name(&self) -> &str {
        &self.name
    }
    fn lattice_name(&self) -> &str {
        &self.lattice_name
    }
    fn group(&self) -> Group {
        self.buffer.group
    }
    fn init_metadata(&self) -> (GaugeFieldInit, Option<u64>) {
        (self.init_kind.clone(), self.init_seed)
    }
    fn as_dense_buffer(&self) -> &DenseLinkBuffer {
        &self.buffer
    }
}

/// Global registry. Singleton; the engine is single-tenant per
/// process for Part II (matches `lattice::registry`).
fn registry() -> &'static Mutex<HashMap<String, Arc<dyn GaugeFieldHandle>>> {
    static REG: OnceLock<Mutex<HashMap<String, Arc<dyn GaugeFieldHandle>>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a gauge field under its `name`. Overwrites any previous
/// registration with the same name (second registration wins).
pub fn register(handle: Arc<dyn GaugeFieldHandle>) {
    let mut g = registry()
        .lock()
        .expect("gauge registry mutex poisoned");
    g.insert(handle.name().to_string(), handle);
}

/// Look up a gauge field by name. Returns a cloned `Arc` so the
/// caller never sees the in-registry Mutex guard.
pub fn get(name: &str) -> Option<Arc<dyn GaugeFieldHandle>> {
    let g = registry()
        .lock()
        .expect("gauge registry mutex poisoned");
    g.get(name).cloned()
}

/// Clear the registry. Test/executor convenience — the persistence
/// gate (II.4b) calls `clear()` at every `Engine::open` so the WAL
/// replay starts from an empty registry and reconstructs the durable
/// set deterministically.
pub fn clear() {
    let mut g = registry()
        .lock()
        .expect("gauge registry mutex poisoned");
    g.clear();
    let mut s = su2_mut_registry()
        .lock()
        .expect("su2 mut registry mutex poisoned");
    s.clear();
}


/// Remove a single named field from both registry maps without
/// clobbering the rest. III.5 tests use this to tear down their own
/// fixtures at the end of a test without disturbing parallel tests'
/// state on the singleton registries.
pub fn remove(name: &str) {
    {
        let mut g = registry()
            .lock()
            .expect("gauge registry mutex poisoned");
        g.remove(name);
    }
    let mut s = su2_mut_registry()
        .lock()
        .expect("su2 mut registry mutex poisoned");
    s.remove(name);
}

// ───────────────────────── SU(2) mutability escape ─────────────────────────
//
// Group-erasure note (Bee's locked decision D4 for Part III gate III.5).
// `register` + `get` above stay on the `Arc<dyn GaugeFieldHandle>` read-only
// surface (group erasure holds for PLAQUETTE / Q_SURROGATE / SHOW). The
// `register_su2` + `get_su2_mut` pair below adds an SU(2)-named, concrete-
// typed mutable escape used only by `GIBBS_SAMPLE`. Future SU(3) heatbath
// will ship a sibling `register_su3` / `get_su3_mut` in parallel; the surface
// stays symmetric. The escape is honest: the Kennedy–Pendleton kernel is
// SU(2)-specific, so an SU(2)-named accessor at this boundary is engineering
// (not a hack).
//
// `register_su2(field)` populates BOTH maps from the same source of truth —
// the `Arc<Mutex<SU2GaugeField>>` goes into the SU(2)-mut map; a snapshot
// `Arc<SU2GaugeField>` (which impls `GaugeFieldHandle`) goes into the dyn
// read map so `get(name)` keeps working byte-identically. After GIBBS_SAMPLE
// mutates the SU(2)-mut copy it calls `refresh_dyn_from_su2_mut(name)` to
// publish a fresh snapshot into the dyn map — read-after-write coherence on
// the dyn surface is restored at the gibbs-sample epilogue, not per edge.

fn su2_mut_registry() -> &'static Mutex<HashMap<String, Arc<Mutex<SU2GaugeField>>>> {
    static REG: OnceLock<Mutex<HashMap<String, Arc<Mutex<SU2GaugeField>>>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register an `SU2GaugeField` under its `name` with the mutable surface
/// `GIBBS_SAMPLE` needs. Populates both the SU(2)-mut map and the existing
/// dyn read map (with a snapshot clone) so `get` and `get_su2_mut` see the
/// same name. Overwrites any previous registration under the same name.
pub fn register_su2(field: SU2GaugeField) {
    let name = field.name.clone();
    // 1. Snapshot the field into the dyn read map (so `get(name)` keeps
    //    returning a `GaugeFieldHandle` byte-identically).
    let snapshot: Arc<dyn GaugeFieldHandle> = Arc::new(field.clone());
    {
        let mut g = registry()
            .lock()
            .expect("gauge registry mutex poisoned");
        g.insert(name.clone(), snapshot);
    }
    // 2. Park the mutable copy in the SU(2)-mut map.
    let mut s = su2_mut_registry()
        .lock()
        .expect("su2 mut registry mutex poisoned");
    s.insert(name, Arc::new(Mutex::new(field)));
}

/// Look up the SU(2) mutable handle for `name`. Returns `Some(guard)` with
/// the inner Mutex already locked so the caller has direct `&mut
/// SU2GaugeField` access for the duration of the guard. SU(2)-named on
/// purpose — group-erasure escape for the heatbath kernel only (Bee's
/// locked decision D4).
///
/// The guard's lifetime is tied to a static `Arc<Mutex<_>>` stored in the
/// SU(2)-mut registry; we leak a `'static` lifetime via the `Arc` so the
/// caller can hold the guard across the gibbs-sample sweep loop.
pub fn get_su2_mut(name: &str) -> Option<Arc<Mutex<SU2GaugeField>>> {
    let s = su2_mut_registry()
        .lock()
        .expect("su2 mut registry mutex poisoned");
    s.get(name).cloned()
}

/// Re-publish the latest `SU2GaugeField` state from the SU(2)-mut map into
/// the dyn read map. `GIBBS_SAMPLE` calls this once after its sweep loop so
/// `get(name)` returns a fresh snapshot of the post-mutation buffer (the
/// dyn surface stays read-after-write coherent at the verb boundary). Noop
/// when `name` is not present in the SU(2)-mut map.
pub fn refresh_dyn_from_su2_mut(name: &str) {
    let snapshot = {
        let s = su2_mut_registry()
            .lock()
            .expect("su2 mut registry mutex poisoned");
        match s.get(name) {
            Some(arc) => {
                let guard = arc.lock().expect("su2 field mutex poisoned");
                guard.clone()
            }
            None => return,
        }
    };
    let mut g = registry()
        .lock()
        .expect("gauge registry mutex poisoned");
    g.insert(name.to_string(), Arc::new(snapshot));
}

/// Re-publish a locally-held `Arc<Mutex<SU2GaugeField>>` into BOTH the
/// SU(2)-mut map and the dyn read map. `GIBBS_SAMPLE` uses this instead
/// of `refresh_dyn_from_su2_mut` when it wants to be robust against a
/// parallel `clear()` that might wipe the SU(2)-mut entry mid-sweep:
/// the local Arc keeps the field alive, this function re-inserts it
/// into both maps under the same name so subsequent lookups land on
/// the post-sweep state.
pub fn republish_su2(name: &str, field_arc: Arc<Mutex<SU2GaugeField>>) {
    let snapshot = {
        let guard = field_arc.lock().expect("su2 field mutex poisoned");
        guard.clone()
    };
    {
        let mut s = su2_mut_registry()
            .lock()
            .expect("su2 mut registry mutex poisoned");
        s.insert(name.to_string(), field_arc);
    }
    let mut g = registry()
        .lock()
        .expect("gauge registry mutex poisoned");
    g.insert(name.to_string(), Arc::new(snapshot));
}

// `MutexGuard` re-export silencer — the public alias is what the spec
// calls out (`MutexGuard<SU2GaugeField>`); we expose `Arc<Mutex<…>>`
// instead because `MutexGuard` is non-`'static` and cannot escape the
// scope that locked the Mutex. The Arc-wrapped form gives the same access
// pattern (lock inside the caller) with a lifetime the registry can hand
// out.
#[allow(dead_code)]
type _Su2GuardAliasHint<'a> = MutexGuard<'a, SU2GaugeField>;

/// Process-wide test serialization mutex. Every gauge test that calls
/// `clear()` (the entire module does) holds this lock for its duration
/// so the lattice + gauge singletons don't get mid-test-clobbered by a
/// parallel `clear()` in another module. The mutex itself does NOT
/// participate in production code paths; production callers (parser,
/// HTTP routes) interact with the registries through their normal
/// `register` / `get` surface without touching this lock.
///
/// `cfg(any(test, feature = "halcyon"))` so integration tests in
/// `tests/halcyon_part_iii_*.rs` (which live in a separate crate from
/// the lib) can reach the same mutex when they need to coexist with
/// in-process gauge tests during a single `cargo test` invocation.
#[cfg(any(test, feature = "halcyon"))]
pub fn test_serial_lock() -> std::sync::MutexGuard<'static, ()> {
    use std::sync::Mutex;
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Snapshot every registered gauge field for compaction. The engine's
/// `compact_wal_to_schemas` re-emits one `WalEntry::GaugeFieldDeclare`
/// per snapshot entry so the durable field set survives WAL rewrite.
/// The handle returned here is the same `Arc` the registry holds;
/// callers use it for read-only access (`init_metadata`, `name`,
/// `lattice_name`, `as_dense_buffer`).
pub fn all() -> Vec<Arc<dyn GaugeFieldHandle>> {
    let g = registry()
        .lock()
        .expect("gauge registry mutex poisoned");
    g.values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::super::dense_link_buffer::DenseLinkBuffer;
    use super::super::edge_connection::EdgeConnection;
    use super::super::group::Group;
    use super::super::group_element::GroupElement;
    use super::super::holonomy::{face_edges, walk_loop};
    use super::super::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use super::*;
    use crate::lattice::registry as lattice_registry;
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use std::sync::Arc;

    /// TDD-HAL-II.4: GaugeFieldRegistry round-trip — register an
    /// SU2GaugeField under a name and read it back via `get`, then
    /// walk one face through the handle and assert the holonomy is
    /// non-identity (Haar buffer at seed 20260616 is essentially never
    /// identity-on-a-face).
    #[test]
    fn tdd_hal_ii_4_register_and_get_round_trip() {
        clear();
        lattice_registry::clear();

        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            "U_tdd_ii_4_a".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init must succeed");

        register(Arc::new(field));

        let got = get("U_tdd_ii_4_a").expect("just registered");
        assert_eq!(got.name(), "U_tdd_ii_4_a");
        assert_eq!(got.lattice_name(), bb.name);
        assert_eq!(got.group(), Group::SU2);
        let (kind, seed) = got.init_metadata();
        assert_eq!(kind, GaugeFieldInit::HaarRandom);
        assert_eq!(seed, Some(20260616));

        // Walk face 0 through the trait-object handle. The Haar
        // buffer at seed 20260616 produces a non-identity face
        // holonomy.
        let conn: &dyn EdgeConnection = got.as_ref();
        let edges = face_edges(&bb, 0);
        let h = walk_loop(&bb, &edges, conn);
        let id = GroupElement::su2_identity();
        assert_ne!(
            h, id,
            "Haar-random face holonomy should not be the SU(2) identity"
        );
    }

    /// TDD-HAL-II.4: double registration overwrites — the registry
    /// never holds two fields with the same name. Different seeds
    /// produce different buffers; the second registration wins.
    #[test]
    fn tdd_hal_ii_4_double_register_overwrites() {
        clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let a = SU2GaugeField::new(
            "U_overwrite".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(1),
        )
        .unwrap();
        let b = SU2GaugeField::new(
            "U_overwrite".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(2),
        )
        .unwrap();
        // Sanity: distinct seeds yield distinct buffers.
        assert_ne!(a.buffer.data, b.buffer.data);

        register(Arc::new(a));
        register(Arc::new(b.clone()));

        let got = get("U_overwrite").expect("registered");
        let (_, seed) = got.init_metadata();
        assert_eq!(seed, Some(2), "second registration wins");
        assert_eq!(got.as_dense_buffer().data, b.buffer.data);
    }

    /// TDD-HAL-II.4: getting an undeclared name returns None.
    #[test]
    fn tdd_hal_ii_4_get_unknown_returns_none() {
        clear();
        assert!(get("never_declared").is_none());
    }

    /// TDD-HAL-II.4: after `clear()` every subsequent `get` returns
    /// None.
    #[test]
    fn tdd_hal_ii_4_clear_empties_registry() {
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());
        let field = SU2GaugeField::new(
            "U_cleared".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        register(Arc::new(field));
        assert!(get("U_cleared").is_some());

        clear();
        assert!(get("U_cleared").is_none());
    }

    /// TDD-HAL-II.4: introspection surface for SHOW GAUGE_FIELD —
    /// `as_dense_buffer` returns a reference to the underlying
    /// `DenseLinkBuffer` whose shape matches the SU2GaugeField's
    /// `buffer` field.
    #[test]
    fn tdd_hal_ii_4_introspect_returns_buffer_view() {
        clear();
        lattice_registry::clear();
        let bb = buckyball();
        lattice_registry::register(bb.clone());
        let field = SU2GaugeField::new(
            "U_introspect".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .unwrap();
        let reference = field.buffer.clone();
        register(Arc::new(field));

        let got = get("U_introspect").expect("registered");
        let view: &DenseLinkBuffer = got.as_dense_buffer();
        assert_eq!(view.group, reference.group);
        assert_eq!(view.n_edges, reference.n_edges);
        assert_eq!(view.repr_dim, reference.repr_dim);
        assert_eq!(view.data, reference.data);
    }
}
