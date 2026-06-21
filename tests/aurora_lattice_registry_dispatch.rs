//! AURORA Phase 1 — CC-2 registry-dispatch refactor (RED-first tests).
//!
//! Receipt: theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md
//! § CC-2 (commit ad306ec). The current executor (src/parser.rs lines
//! 8773–8813) dispatches `LATTICE name FROM <CANONICAL_ID>` via a
//! hard-coded match arm on `TRUNCATED_ICOSAHEDRON`. CC-2 replaces
//! that match with `lattice::registry::get_constructor(canonical)`,
//! returning a `Constructor` function pointer (or closure-equivalent)
//! that produces a `LatticeWithMetric` from `ConstructorArgs`.
//!
//! BIT-IDENTITY GATE (the load-bearing assertion of this file):
//! the buckyball returned via the registry must equal — field-for-field
//! — the buckyball returned by the existing direct `buckyball()`
//! constructor. If those two paths drift, every downstream Halcyon
//! gate (TDD-HAL-I.4 walker equivalence, IV.10 symplectic flow,
//! V.* heatbath gold) is at risk.
//!
//! Per the AURORA constraint sheet:
//!   - additive: do NOT modify existing pub methods that other tests
//!     depend on (the existing `buckyball()` constructor stays);
//!   - case-insensitive lookup (the parser uppercases canonical names);
//!   - unknown identifiers return None (silent defaults hide drift).

#![cfg(feature = "lattice")]

use gigi::lattice::registry;
use gigi::lattice::topology::truncated_icosahedron::buckyball;

/// The registry exposes a constructor for TRUNCATED_ICOSAHEDRON. The
/// call returns Ok with a LatticeWithMetric whose lattice half has
/// the canonical buckyball counts (V=60, E=90, F=32, χ=2).
#[test]
fn test_registry_get_constructor_truncated_icosahedron() {
    let ctor = registry::get_constructor("TRUNCATED_ICOSAHEDRON")
        .expect("TRUNCATED_ICOSAHEDRON must be registered in the constructor table");
    let args = registry::ConstructorArgs::default();
    let lwm = ctor(&args).expect("buckyball constructor must not fail with default args");
    let lat = lwm.lattice();
    assert_eq!(lat.n_vertices, 60, "buckyball V");
    assert_eq!(lat.n_edges(), 90, "buckyball E");
    assert_eq!(lat.n_faces(), 32, "buckyball F");
    assert_eq!(lat.euler_characteristic(), 2, "buckyball χ");
}

/// BIT-IDENTITY GATE. The buckyball produced through the registry
/// must equal the buckyball produced via the direct `buckyball()`
/// path — same name, vertex count, edges Vec (order + pairs),
/// faces Vec (order + cycles), and topology hint. This is the
/// regression anchor that proves the CC-2 refactor preserves the
/// existing Halcyon contract surface.
#[test]
fn test_registry_dispatched_buckyball_bit_identical_to_direct() {
    let direct = buckyball();

    let ctor = registry::get_constructor("TRUNCATED_ICOSAHEDRON")
        .expect("TRUNCATED_ICOSAHEDRON must be registered");
    let args = registry::ConstructorArgs::default();
    let via_registry = ctor(&args)
        .expect("registry-dispatched buckyball must build")
        .lattice()
        .clone();

    // PartialEq on Lattice is field-for-field (derived #[derive(PartialEq)]
    // in src/lattice/mod.rs:75). Anything that drifts trips this.
    assert_eq!(
        via_registry, direct,
        "registry-dispatched buckyball must be byte-identical to direct buckyball()"
    );

    // Re-emit the canonical GQL form on both sides; the strings
    // must be identical character-for-character.
    assert_eq!(
        via_registry.to_gql(),
        direct.to_gql(),
        "canonical GQL re-emit must match between registry and direct paths"
    );
}

/// CUBED_SPHERE is registered alongside TRUNCATED_ICOSAHEDRON. The
/// constructor accepts a `panel_size` parameter via ConstructorArgs.
#[test]
fn test_registry_get_constructor_cubed_sphere() {
    let ctor = registry::get_constructor("CUBED_SPHERE")
        .expect("CUBED_SPHERE must be registered in the constructor table");
    let args = registry::ConstructorArgs {
        panel_size: Some(3),
        ..Default::default()
    };
    let lwm = ctor(&args).expect("CUBED_SPHERE C=3 must build");
    let lat = lwm.lattice();
    // F = 6·C² = 54 for C=3.
    assert_eq!(lat.n_faces(), 54, "CUBED_SPHERE C=3 face count");
    // V − E + F = 2 on the sphere.
    let v = lat.n_vertices as i64;
    let e = lat.n_edges() as i64;
    let f = lat.n_faces() as i64;
    assert_eq!(v - e + f, 2, "CUBED_SPHERE Euler χ on the sphere");
}

/// Unknown canonical identifiers return None — never a silent default,
/// never a panic. The executor's error message ("unknown canonical
/// lattice identifier") should propagate from a None return here.
#[test]
fn test_registry_get_constructor_unknown_returns_none() {
    let ctor = registry::get_constructor("NONEXISTENT_TOPOLOGY");
    assert!(
        ctor.is_none(),
        "unknown canonical identifier must return None (got Some(_))"
    );
}
