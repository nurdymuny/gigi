//! Phase 1b — `LATTICE foo FROM CUBED_SPHERE TOPOLOGY 'S2';` works through
//! the parser executor's registry dispatch.
//!
//! Background: Phase 1 (commit `f62e46c`) shipped the
//! `lattice::registry::get_constructor()` API and registered both
//! `TRUNCATED_ICOSAHEDRON` and `CUBED_SPHERE` constructors. The parser
//! executor's `Statement::LatticeFromCanonical` arm (`src/parser.rs:8773`)
//! still used a static match arm that only knew `TRUNCATED_ICOSAHEDRON`.
//! Phase 1b switches the arm to route through `get_constructor()`.
//!
//! RED state before Phase 1b: the executor returns
//! `Err("Unknown canonical lattice constructor: 'CUBED_SPHERE'. Part I
//! ships only TRUNCATED_ICOSAHEDRON.")` because the static match has
//! no arm for `CUBED_SPHERE`.
//!
//! GREEN state after Phase 1b: the executor calls
//! `registry::get_constructor("CUBED_SPHERE")`, invokes it with default
//! `ConstructorArgs` (panel_size=None → default 1), extracts the bare
//! `Lattice` from the returned `LatticeWithMetric` via `.lattice().clone()`,
//! and registers it through the same path TRUNCATED_ICOSAHEDRON uses.
//!
//! The bit-identity guard for TRUNCATED_ICOSAHEDRON through the new
//! dispatch path already lives in `tests/aurora_lattice_registry_dispatch.rs`
//! (`test_registry_dispatched_buckyball_bit_identical_to_direct`) and stays
//! green throughout Phase 1b. This file's tests cover the parser-side
//! routing: the LATTICE GQL verb reaches the registry, and CUBED_SPHERE
//! is constructable through it.

use gigi::engine::Engine;
use gigi::lattice::Lattice;
use gigi::parser;
use gigi::types::Value;

/// Phase 1b core: `LATTICE foo FROM CUBED_SPHERE TOPOLOGY 'S2';` succeeds.
#[test]
fn phase_1b_lattice_verb_cubed_sphere_succeeds() {
    gigi::lattice::registry::clear();
    let name = "phase_1b_cs_minimal";
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let decl = format!("LATTICE {name} FROM CUBED_SPHERE TOPOLOGY 'S2';");
    let stmt = parser::parse(&decl).expect("parse LATTICE CUBED_SPHERE decl");

    let result = parser::execute(&mut engine, &stmt);
    assert!(
        result.is_ok(),
        "expected LATTICE CUBED_SPHERE to dispatch via registry and succeed, \
         got {result:?}"
    );
    match result.unwrap() {
        parser::ExecResult::Ok => {}
        other => panic!("expected ExecResult::Ok, got {other:?}"),
    }

    // SHOW LATTICE returns the registered lattice with the expected
    // combinatorial shape for the default panel_size=1 cubed sphere.
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

    let lat = Lattice::from_gql(&gql_emitted).expect("re-parse SHOW output");
    assert_eq!(lat.name, name);
    // C=1 cubed sphere: V=6C²+2=8, E=12C²=12, F=6C²=6, χ = V-E+F = 2
    assert_eq!(lat.n_vertices, 8, "C=1 cubed sphere vertex count");
    assert_eq!(lat.n_edges(), 12, "C=1 cubed sphere edge count");
    assert_eq!(lat.n_faces(), 6, "C=1 cubed sphere face count");
    assert_eq!(lat.topology.as_deref(), Some("S2"));
}

/// Phase 1b regression: TRUNCATED_ICOSAHEDRON via the LATTICE GQL verb
/// still produces the canonical buckyball (V=60, E=90, F=32). The switch
/// to registry dispatch must not change the existing buckyball path.
#[test]
fn phase_1b_lattice_verb_truncated_icosahedron_unchanged() {
    gigi::lattice::registry::clear();
    let name = "phase_1b_bb_via_registry";
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let decl = format!("LATTICE {name} FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';");
    let stmt = parser::parse(&decl).expect("parse LATTICE TI decl");
    parser::execute(&mut engine, &stmt).expect("exec LATTICE TI");

    let show = format!("SHOW LATTICE {name};");
    let stmt = parser::parse(&show).expect("parse SHOW LATTICE");
    let rows = match parser::execute(&mut engine, &stmt).expect("exec SHOW") {
        parser::ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    let gql_emitted = match rows[0].get("gql") {
        Some(Value::Text(s)) => s.clone(),
        other => panic!("missing gql column: {other:?}"),
    };
    let lat = Lattice::from_gql(&gql_emitted).expect("re-parse");
    assert_eq!(lat.n_vertices, 60, "TRUNCATED_ICOSAHEDRON V via registry");
    assert_eq!(lat.n_edges(), 90, "TRUNCATED_ICOSAHEDRON E via registry");
    assert_eq!(lat.n_faces(), 32, "TRUNCATED_ICOSAHEDRON F via registry");
    assert_eq!(lat.topology.as_deref(), Some("S2"));
}

/// Phase 1b error message: unknown canonical names still produce a clear
/// error, and the error now reflects the actually-available constructors
/// (not the stale "Part I ships only TRUNCATED_ICOSAHEDRON" message).
#[test]
fn phase_1b_lattice_verb_unknown_canonical_returns_error() {
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let decl = "LATTICE oops FROM NONEXISTENT_TOPOLOGY TOPOLOGY 'X';";
    let stmt = parser::parse(decl).expect("parse LATTICE unknown decl");
    let result = parser::execute(&mut engine, &stmt);
    assert!(result.is_err(), "unknown canonical must error, got {result:?}");
    let err = result.unwrap_err();
    assert!(
        err.contains("NONEXISTENT_TOPOLOGY"),
        "error message names the bad canonical: {err}"
    );
}
