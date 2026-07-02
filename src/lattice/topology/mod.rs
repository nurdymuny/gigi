//! Canonical lattice constructors.
//!
//! Each submodule produces a ready-to-register `Lattice` for one
//! named graph shape. The `LATTICE name FROM CANONICAL_ID` GQL
//! shorthand dispatches to these constructors by uppercase identifier.
//!
//! Currently shipped:
//!
//! - `cubed_sphere` — six gnomonic panels stitched into a topological
//!   S²; parameterized by per-panel grid resolution C. V=6C²+2,
//!   E=12C², F=6C², χ=2.
//! - `cubic` — n-dimensional cubic lattice T^D (PERIODIC), V=L^D,
//!   E=L^D·D, F=L^D·D·(D-1)/2. The Halcyon §3.3 substrate
//!   (4D pure-gauge target: L=12, D=4 → V=20736, E=82944, F=124416).
//!   Phase 1 scope is PERIODIC only; OPEN deferred to Phase 2.
//! - `truncated_icosahedron` — the buckyball / Goldberg(1,1) / C₆₀
//!   cage: 60 vertices, 90 edges, 32 faces (12 pentagons + 20
//!   hexagons), Euler χ = 2, topology S². General fullerene math
//!   (not Halcyon-specific); Halcyon's heatbath fixtures index
//!   against this exact construction order, which is why the
//!   indexing matters here.
//!
//! Side modules:
//!
//! - `hints` — canonical (CANONICAL_ID → topology-hint) lookup table.
//!   Metadata-only registry consulted by callers that need to know the
//!   topology of a constructor without instantiating it. Future
//!   topologies extend the table by adding a single row.

pub mod cubed_sphere;
pub mod cubic;
pub mod hints;
pub mod truncated_icosahedron;

pub use cubed_sphere::cubed_sphere;
pub use cubic::cubic;
pub use hints::topology_hint_for;

/// Column-major site encoding for cubic lattices: `site_of(&[c0, c1,
/// ..., c_{D-1}], L) = sum_k c_k * L^k`, i.e. `c0` (site_x) is
/// least-significant. Matches the private encoding used inside
/// `cubic::cubic` (see the `site_of` closure there) and therefore
/// coincides with `Lattice::VertexId` numbering for the cubic
/// constructor. Callers that need to convert (config, coords) tuples
/// to lattice-native vertex ids — including the GAUGE_FIELD ingest
/// emitter, which stamps `vertex_a` / `vertex_b` on each record —
/// should route through this helper so the integer values stay
/// aligned with `lattice.vertex(id)` and `lattice.resolve_edge(a,b)`.
pub fn site_of_column_major(coords: &[usize], l: usize) -> usize {
    let mut s = 0usize;
    let mut stride = 1usize;
    for &c in coords {
        s += c * stride;
        stride *= l;
    }
    s
}
