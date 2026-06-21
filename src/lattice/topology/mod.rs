//! Canonical lattice constructors.
//!
//! Each submodule produces a ready-to-register `Lattice` for one
//! named graph shape. The `LATTICE name FROM CANONICAL_ID` GQL
//! shorthand dispatches to these constructors by uppercase identifier.
//!
//! Currently shipped:
//!
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
pub mod hints;
pub mod truncated_icosahedron;

pub use cubed_sphere::cubed_sphere;
pub use hints::topology_hint_for;
