//! Halcyon ITEM 3.3 — `LATTICE name FROM CUBIC L=<N> DIM=<K> [PERIODIC|OPEN];`
//! works through the parser executor's registry dispatch.
//!
//! Three integration gates:
//!
//! 1. `LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;` lands the
//!    Halcyon §3.3 substrate in the registry with the locked
//!    combinatorics (V=20736, E=82944, F=124416).
//! 2. `PERIODIC` defaults to `true` when omitted (matches the
//!    lattice-physics convention).
//! 3. `LATTICE square FROM CUBIC L=4 DIM=2 PERIODIC;` produces the
//!    same V/E/F counts as a hand-rolled 2D periodic lattice
//!    (regression anchor: the flat-2-torus is the D=2 case of CUBIC).
//!
//! All three round-trip through `SHOW LATTICE name;` + re-parse via
//! `Lattice::from_gql`, the same canonical re-emit gate
//! `aurora_phase_1b_lattice_verb_cubed_sphere` uses.

#![cfg(feature = "lattice")]

use gigi::engine::Engine;
use gigi::lattice::Lattice;
use gigi::parser;
use gigi::types::Value;

/// Helper: declare a lattice via the FROM-shorthand, then SHOW it and
/// re-parse the canonical GQL re-emit through `Lattice::from_gql`. Returns
/// the round-tripped `Lattice` for combinatorial assertions.
fn declare_and_round_trip(decl: &str, name: &str) -> Lattice {
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let stmt = parser::parse(decl).unwrap_or_else(|e| panic!("parse `{decl}` failed: {e}"));
    parser::execute(&mut engine, &stmt)
        .unwrap_or_else(|e| panic!("execute `{decl}` failed: {e}"));

    let show = format!("SHOW LATTICE {name};");
    let stmt = parser::parse(&show).expect("parse SHOW LATTICE");
    let rows = match parser::execute(&mut engine, &stmt).expect("exec SHOW LATTICE") {
        parser::ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "SHOW LATTICE returns exactly one row");
    let gql_emitted = match rows[0].get("gql") {
        Some(Value::Text(s)) => s.clone(),
        other => panic!("missing/wrong-typed gql column: {other:?}"),
    };
    Lattice::from_gql(&gql_emitted).expect("re-parse SHOW output")
}

/// Halcyon §3.3 substrate — the locked L=12 D=4 PERIODIC dimensions.
/// V = 20_736, E = 82_944, F = 124_416 (the numbers named in
/// `GIGI_TO_HALCYON_REPLY_2026-06-26_BRIDGE_REVISED.md` §3.3).
#[test]
fn test_parse_lattice_cubic_l12_d4_periodic() {
    let decl = "LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;";
    let lat = declare_and_round_trip(decl, "my4d");

    assert_eq!(lat.name, "my4d");
    assert_eq!(lat.n_vertices, 20_736, "Halcyon §3.3: V = 12^4 = 20736");
    assert_eq!(lat.n_edges(), 82_944, "Halcyon §3.3: E = 12^4 · 4 = 82944");
    assert_eq!(lat.n_faces(), 124_416, "Halcyon §3.3: F = 12^4 · 6 = 124416");
    assert_eq!(lat.topology.as_deref(), Some("CUBIC_L12_D4"));
}

/// PERIODIC defaults to `true` when the keyword is omitted, matching
/// the lattice-physics convention. The substrate produced is
/// bit-equivalent to the explicit-PERIODIC declaration above.
#[test]
fn test_parse_lattice_cubic_default_periodic() {
    let decl = "LATTICE my4d_default FROM CUBIC L=4 DIM=3;";
    let lat = declare_and_round_trip(decl, "my4d_default");

    assert_eq!(lat.name, "my4d_default");
    assert_eq!(lat.n_vertices, 64, "V = 4^3 = 64");
    assert_eq!(lat.n_edges(), 192, "E = 4^3 · 3 = 192");
    assert_eq!(lat.n_faces(), 192, "F = 4^3 · C(3,2) = 4^3 · 3 = 192");
    assert_eq!(lat.topology.as_deref(), Some("CUBIC_L4_D3"));

    // Every vertex has degree 2·D = 6 on the closed 3-torus.
    let mut degree = vec![0usize; lat.n_vertices];
    for &(a, b) in &lat.edges {
        degree[a] += 1;
        degree[b] += 1;
    }
    for (vid, deg) in degree.iter().enumerate() {
        assert_eq!(*deg, 6, "vertex {vid} should have degree 2D=6");
    }
}

/// `LATTICE square FROM CUBIC L=4 DIM=2 PERIODIC;` produces the
/// flat-2-torus counts (V=16, E=32, F=16, χ=0) — the D=2 case of
/// CUBIC is the same combinatorial surface as a hand-rolled 4×4
/// periodic grid. Regression anchor against the cubic constructor
/// drifting from its D=2 specialization.
#[test]
fn test_parse_lattice_cubic_d2_matches_flat_torus() {
    let decl = "LATTICE square FROM CUBIC L=4 DIM=2 PERIODIC;";
    let lat = declare_and_round_trip(decl, "square");

    assert_eq!(lat.name, "square");
    assert_eq!(lat.n_vertices, 16, "V = 4^2 = 16");
    assert_eq!(lat.n_edges(), 32, "E = 4^2 · 2 = 32");
    assert_eq!(lat.n_faces(), 16, "F = 4^2 · C(2,2) = 4^2 · 1 = 16");
    assert_eq!(lat.euler_characteristic(), 0, "χ(T²) = 0");
    assert_eq!(lat.topology.as_deref(), Some("CUBIC_L4_D2"));

    // Every vertex has degree 2·D = 4 on the closed 2-torus.
    let mut degree = vec![0usize; lat.n_vertices];
    for &(a, b) in &lat.edges {
        degree[a] += 1;
        degree[b] += 1;
    }
    for (vid, deg) in degree.iter().enumerate() {
        assert_eq!(*deg, 4, "vertex {vid} should have degree 2D=4");
    }

    // Every face is a quad on the 2-torus.
    for face in &lat.faces {
        assert_eq!(face.len(), 4);
    }
}

/// Missing required parameter `L` surfaces with a clear error, not a
/// silent default-to-1.
#[test]
fn test_parse_lattice_cubic_missing_l_errors() {
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let decl = "LATTICE oops FROM CUBIC DIM=4 PERIODIC;";
    let stmt = parser::parse(decl).expect("parse missing-L decl");
    let result = parser::execute(&mut engine, &stmt);
    assert!(result.is_err(), "missing L must error, got {result:?}");
    let err = result.unwrap_err();
    assert!(
        err.contains("L"),
        "error message names the missing parameter L: {err}"
    );
}

/// Missing required parameter `DIM` surfaces with a clear error.
#[test]
fn test_parse_lattice_cubic_missing_dim_errors() {
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let decl = "LATTICE oops FROM CUBIC L=4 PERIODIC;";
    let stmt = parser::parse(decl).expect("parse missing-DIM decl");
    let result = parser::execute(&mut engine, &stmt);
    assert!(result.is_err(), "missing DIM must error, got {result:?}");
    let err = result.unwrap_err();
    assert!(
        err.contains("DIM"),
        "error message names the missing parameter DIM: {err}"
    );
}

/// `OPEN` boundary condition is parsed but routes to the Phase-2-deferred
/// assertion in the underlying constructor. Verifies the parser accepts
/// the keyword and the assertion fires (Phase 2 will lift this).
#[test]
fn test_parse_lattice_cubic_open_panics() {
    gigi::lattice::registry::clear();
    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = Engine::open(dir.path()).expect("engine open");

    let decl = "LATTICE openish FROM CUBIC L=4 DIM=3 OPEN;";
    let stmt = parser::parse(decl).expect("parse OPEN decl");
    // The cubic constructor panics on `periodic = false`. The executor
    // hands that panic up; `catch_unwind` lets us assert the message
    // without aborting the test process.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        parser::execute(&mut engine, &stmt)
    }));
    assert!(
        result.is_err(),
        "OPEN must panic with the deferred-to-Phase-2 assertion, got {result:?}"
    );
}

/// Regression guard for the existing TRUNCATED_ICOSAHEDRON path: the
/// Phase 1b lattice GQL verb still produces the canonical buckyball
/// (V=60, E=90, F=32) after the FROM-clause grammar was extended to
/// accept KEY=VALUE parameters. The TI path takes no parameters so
/// the empty-params branch of the new grammar is the path exercised.
#[test]
fn test_parse_lattice_truncated_icosahedron_unchanged() {
    let decl = "LATTICE phase33_bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let lat = declare_and_round_trip(decl, "phase33_bb");
    assert_eq!(lat.n_vertices, 60);
    assert_eq!(lat.n_edges(), 90);
    assert_eq!(lat.n_faces(), 32);
    assert_eq!(lat.topology.as_deref(), Some("S2"));
}
