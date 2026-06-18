//! In-process lattice registry. The executor materializes declared
//! `LATTICE` statements here; `SHOW LATTICE name` reads back through
//! it.
//!
//! Closes the storage half of TDD-HAL-I.8. The registry is a
//! per-process `HashMap<String, Lattice>` guarded by a `Mutex` —
//! a `BundleStore`-grade persistence layer is a Part-II follow-up
//! once `GAUGE_FIELD` lands on top.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use super::lattice::Lattice;

/// Global registry. Singleton; the engine is single-tenant per
/// process for Part I.
fn registry() -> &'static Mutex<HashMap<String, Lattice>> {
    static REG: OnceLock<Mutex<HashMap<String, Lattice>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a Lattice under its `name`. Overwrites any previous
/// registration with the same name.
pub fn register(lat: Lattice) {
    let mut g = registry().lock().expect("halcyon registry mutex poisoned");
    g.insert(lat.name.clone(), lat);
}

/// Look up a Lattice by name. Returns a clone for round-trip
/// stability — the caller never sees the in-registry Mutex guard.
pub fn get(name: &str) -> Option<Lattice> {
    let g = registry().lock().expect("halcyon registry mutex poisoned");
    g.get(name).cloned()
}

/// Clear the registry. Test-only convenience.
#[cfg(test)]
pub fn clear() {
    let mut g = registry().lock().expect("halcyon registry mutex poisoned");
    g.clear();
}

#[cfg(test)]
mod tests {
    use super::super::truncated_icosahedron::buckyball;
    use super::*;

    #[test]
    fn register_and_get_round_trip() {
        clear();
        let bb = buckyball();
        register(bb.clone());
        let got = get("buckyball").expect("buckyball was just registered");
        assert_eq!(got, bb);
    }
}
