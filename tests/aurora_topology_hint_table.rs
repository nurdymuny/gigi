//! AURORA Phase 1 — topology_hint const table (RED-first tests).
//!
//! Receipt: theory/aurora/GIGI_TO_AURORA_2026-06-19_v0_1_REPLY_2.md
//! § Phase 1 status board ("topology_hint const table"). ~30 LOC of
//! lookup metadata in src/lattice/topology/hints.rs. CUBED_SPHERE
//! and TRUNCATED_ICOSAHEDRON are reserved symmetrically — both
//! resolve to the S² hint string.
//!
//! Phase 1 scope: the table is metadata-only. It is NOT loaded by the
//! parser executor in Phase 1 (the executor still receives the hint
//! from the TOPOLOGY clause or the constructor's stamp). The table
//! exists so that callers — and downstream verifiers — can ask "what
//! topology does CANONICAL_ID belong to?" without instantiating the
//! constructor.
//!
//! Per the AURORA constraint sheet:
//!   - lookup is case-insensitive over the canonical identifier;
//!   - unknown identifiers return `None`, never a default — silent
//!     defaults hide drift between the table and the constructors.

#![cfg(feature = "lattice")]

use gigi::lattice::topology::hints;

/// CUBED_SPHERE is reserved as an S² topology in the hint table.
#[test]
fn test_topology_hint_s2_cubed_sphere_registered() {
    let hint = hints::lookup("CUBED_SPHERE");
    assert_eq!(
        hint,
        Some("S2"),
        "CUBED_SPHERE must resolve to S² in the hint table (got {hint:?})"
    );
}

/// TRUNCATED_ICOSAHEDRON is also an S² topology and is registered
/// symmetrically alongside CUBED_SPHERE in Phase 1.
#[test]
fn test_topology_hint_s2_truncated_icosahedron_registered() {
    let hint = hints::lookup("TRUNCATED_ICOSAHEDRON");
    assert_eq!(
        hint,
        Some("S2"),
        "TRUNCATED_ICOSAHEDRON must resolve to S² in the hint table (got {hint:?})"
    );
}

/// Unknown identifiers return None — never a silent default.
#[test]
fn test_topology_hint_unknown_returns_none() {
    let hint = hints::lookup("Z3/SOMETHING_WEIRD");
    assert_eq!(
        hint,
        None,
        "unknown identifier must return None, not a default (got {hint:?})"
    );
}
