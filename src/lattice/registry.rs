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

use super::Lattice;

/// Global registry. Singleton; the engine is single-tenant per
/// process for Part I.
fn registry() -> &'static Mutex<HashMap<String, Lattice>> {
    static REG: OnceLock<Mutex<HashMap<String, Lattice>>> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Register a Lattice under its `name`. Overwrites any previous
/// registration with the same name.
pub fn register(lat: Lattice) {
    let mut g = registry().lock().expect("lattice registry mutex poisoned");
    g.insert(lat.name.clone(), lat);
}

/// Look up a Lattice by name. Returns a clone for round-trip
/// stability — the caller never sees the in-registry Mutex guard.
pub fn get(name: &str) -> Option<Lattice> {
    let g = registry().lock().expect("lattice registry mutex poisoned");
    g.get(name).cloned()
}

/// Clear the registry. Convenience for tests + for the engine's
/// `do_replay` path which rebuilds the registry from WAL on every
/// `Engine::open` (TDD-HAL-II.4b durability gate).
pub fn clear() {
    let mut g = registry().lock().expect("lattice registry mutex poisoned");
    g.clear();
}

/// Snapshot every registered Lattice for compaction. The engine's
/// `compact_wal_to_schemas` re-emits one `WalEntry::LatticeDeclare`
/// per snapshot entry so the durable lattice set survives WAL rewrite.
/// Ordering is unspecified — the gauge-field emit consults the
/// resolved name, not iteration order.
pub fn all() -> Vec<Lattice> {
    let g = registry().lock().expect("lattice registry mutex poisoned");
    g.values().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::super::topology::truncated_icosahedron::buckyball;
    use super::*;

    #[test]
    fn register_and_get_round_trip() {
        clear();
        let bb = buckyball();
        register(bb.clone());
        let got = get("buckyball").expect("buckyball was just registered");
        assert_eq!(got, bb);
    }

    /// TDD-HAL-I.8 — executor + SHOW LATTICE round-trip.
    /// Declare a lattice via the shorthand form; issue SHOW LATTICE
    /// name; re-parse the SHOW output's `gql` column; assert
    /// structural equality with the original.
    #[test]
    fn tdd_hal_i_8_lattice_register_and_show() {
        use crate::engine::Engine;
        use crate::parser;
        use crate::types::Value;

        clear();
        // Use an isolated name to avoid colliding with other tests
        // racing for the singleton registry.
        let name = "tdd_hal_i_8_bb";

        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = Engine::open(dir.path()).expect("engine open");

        // 1. Declare via shorthand. Executor materializes via
        // truncated_icosahedron::buckyball() and renames.
        let decl = format!(
            "LATTICE {name} FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';"
        );
        let stmt = parser::parse(&decl).expect("parse LATTICE decl");
        match parser::execute(&mut engine, &stmt).expect("exec LATTICE decl") {
            parser::ExecResult::Ok => {}
            other => panic!("expected Ok, got {other:?}"),
        }

        // 2. SHOW LATTICE name; — read out the registered lattice.
        let show = format!("SHOW LATTICE {name};");
        let stmt = parser::parse(&show).expect("parse SHOW LATTICE");
        let rows = match parser::execute(&mut engine, &stmt)
            .expect("exec SHOW LATTICE")
        {
            parser::ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1, "SHOW LATTICE returns exactly one row");
        let row = &rows[0];
        let gql_emitted = match row.get("gql") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong-typed gql column: {other:?}"),
        };

        // 3. Round-trip — re-parse the emitted GQL via the Lattice
        //    algebra's own from_gql (the canonical re-emit
        //    parser).
        let lat = super::super::Lattice::from_gql(&gql_emitted)
            .expect("re-parse SHOW output");
        assert_eq!(lat.name, name);
        assert_eq!(lat.n_vertices, 60);
        assert_eq!(lat.n_edges(), 90);
        assert_eq!(lat.n_faces(), 32);
        assert_eq!(lat.topology.as_deref(), Some("S2"));

        // 4. Structural equality with the registered lattice
        //    (after the rename — the buckyball constructor names
        //    itself "buckyball" by default; the executor rebinds
        //    to the declared name).
        let registered = get(name).expect("registered lattice");
        assert_eq!(lat, registered);
    }

    /// Bit-identity contract: re-emitting an explicit-form
    /// declaration through SHOW LATTICE produces the same canonical
    /// re-emit body twice in a row.
    #[test]
    fn tdd_hal_i_8_explicit_form_round_trip() {
        use crate::engine::Engine;
        use crate::parser;
        use crate::types::Value;

        clear();
        let name = "tdd_hal_i_8_explicit";
        let dir = tempfile::tempdir().expect("tempdir");
        let mut engine = Engine::open(dir.path()).expect("engine open");
        let decl = format!(
            "LATTICE {name} \
             VERTICES 4 \
             EDGES ((0,1),(1,2),(2,0)) \
             FACES ((0,1,2));"
        );
        let stmt = parser::parse(&decl).expect("parse explicit LATTICE");
        parser::execute(&mut engine, &stmt).expect("exec explicit LATTICE");

        let show = format!("SHOW LATTICE {name};");
        let stmt = parser::parse(&show).expect("parse SHOW LATTICE");
        let rows = match parser::execute(&mut engine, &stmt)
            .expect("exec SHOW LATTICE")
        {
            parser::ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        let gql_first = match rows[0].get("gql") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong gql: {other:?}"),
        };

        // Round-trip the emitted form back through the parser +
        // executor.
        let stmt = parser::parse(&gql_first).expect("re-parse SHOW output");
        parser::execute(&mut engine, &stmt).expect("re-exec re-parsed LATTICE");

        // Issue SHOW LATTICE again — the canonical form is
        // bit-identical to the first emission.
        let stmt = parser::parse(&show).expect("parse SHOW LATTICE 2");
        let rows = match parser::execute(&mut engine, &stmt)
            .expect("exec SHOW LATTICE 2")
        {
            parser::ExecResult::Rows(r) => r,
            other => panic!("expected Rows, got {other:?}"),
        };
        let gql_second = match rows[0].get("gql") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("missing/wrong gql: {other:?}"),
        };
        assert_eq!(gql_first, gql_second);
    }
}
