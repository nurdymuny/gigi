//! Lattice + per-cell/per-edge metric data. DEC operators in Phase 2's
//! `lattice::dec` free functions.
//!
//! Phase 1 scope (AURORA reply 2, design.lattice_with_metric_decision
//! resolution): this module ships a MINIMAL wrapper struct that bundles
//! a [`Lattice`] together with the geometric data the ShallowWater
//! probe and downstream gauge consumers need (per-cell areas, per-edge
//! arc-lengths, optional dual-face areas). It deliberately does NOT
//! expose any Discrete Exterior Calculus (DEC) operators — `d_0`,
//! `delta_0`, `hodge_star_k` and friends land in Phase 2 as free
//! functions inside a new `src/lattice/dec/` module that CONSUMES this
//! wrapper. The Phase 1/2 boundary is held strictly additively: nothing
//! in Phase 2 should require breaking changes to the types declared
//! here.
//!
//! The A1 contract from the AURORA round-2 reply ("CUBED_SPHERE
//! constructor returns LatticeWithMetric") is honored by this Phase 1
//! surface; the constructor side of the wrapper is what cubed-sphere
//! and (zero-metric) truncated-icosahedron registry entries return.
//! The bare [`Lattice`] half is what continues to register via the
//! existing `lattice::registry::register()` path (callers unwrap via
//! [`LatticeWithMetric::lattice`] and clone), so the storage half of
//! TDD-HAL-I.8 is unchanged.

use crate::lattice::Lattice;

/// Wrapper bundling a [`Lattice`] with the per-cell and per-edge
/// metric data downstream DEC and ShallowWater consumers require.
///
/// Phase 1 surface only — no DEC operators are defined on this type.
/// See module docs for the Phase 1/2 boundary.
///
/// Stability: EVOLVING until gigi 0.1.0 tag.
/// Breaking changes only on minor version bumps (0.x → 0.(x+1)).
/// Patch versions (0.x.y → 0.x.(y+1)) are non-breaking on this surface.
#[derive(Debug, Clone, PartialEq)]
pub struct LatticeWithMetric {
    /// The combinatorial substrate. Bit-identical to whatever the
    /// underlying topology constructor produced; never mutated by this
    /// wrapper.
    lattice: Lattice,
    /// Per-cell (per-face) areas, indexed by face id in
    /// `lattice.faces`. Length MUST equal `lattice.n_faces()` when
    /// non-empty; a zero-length vector is the documented Phase 1
    /// "no real metric" placeholder used by the truncated-icosahedron
    /// constructor's wrapper.
    cell_areas: Vec<f64>,
    /// Per-edge arc-lengths, indexed by edge id in `lattice.edges`.
    /// Length MUST equal `lattice.n_edges()` when non-empty; same
    /// zero-length placeholder convention as `cell_areas`.
    edge_lengths: Vec<f64>,
    /// Optional dual-face areas (one per primal vertex), indexed by
    /// `VertexId`. Only Some(_) when the topology supplies a dual mesh
    /// (e.g. cubed-sphere); None means downstream consumers must
    /// compute it themselves or refuse.
    dual_face_areas: Option<Vec<f64>>,
}

impl LatticeWithMetric {
    /// Construct a wrapper from a [`Lattice`] and its metric vectors.
    ///
    /// `cell_areas.len()` and `edge_lengths.len()` are NOT validated
    /// against the lattice cardinalities here — Phase 1 leaves that
    /// to the constructor's own tests so the zero-metric placeholder
    /// path (both vectors empty) can be expressed without ceremony.
    /// Phase 2 may tighten this when the DEC operators land and need
    /// the lengths to match.
    pub fn from_lattice_and_metric(
        lattice: Lattice,
        cell_areas: Vec<f64>,
        edge_lengths: Vec<f64>,
        dual_face_areas: Option<Vec<f64>>,
    ) -> Self {
        Self {
            lattice,
            cell_areas,
            edge_lengths,
            dual_face_areas,
        }
    }

    /// Borrow the underlying combinatorial [`Lattice`]. The bridge to
    /// the existing `lattice::registry::register()` path — callers
    /// clone through this accessor when they need a bare `Lattice` to
    /// store.
    pub fn lattice(&self) -> &Lattice {
        &self.lattice
    }

    /// Borrow the per-cell area vector. Length is `n_faces()` for a
    /// fully-metric lattice, or `0` for the Phase 1 zero-metric
    /// placeholder (e.g. truncated-icosahedron's registry entry).
    pub fn cell_areas(&self) -> &[f64] {
        &self.cell_areas
    }

    /// Borrow the per-edge arc-length vector. Length is `n_edges()`
    /// for a fully-metric lattice, or `0` for the zero-metric
    /// placeholder.
    pub fn edge_lengths(&self) -> &[f64] {
        &self.edge_lengths
    }

    /// Borrow the per-vertex dual-face area vector, if the topology
    /// supplied one. `None` for any lattice whose constructor did not
    /// commit to a dual mesh.
    pub fn dual_face_areas(&self) -> Option<&[f64]> {
        self.dual_face_areas.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal smoke test: a hand-built `Lattice` round-trips through
    /// the wrapper and the four accessors hand back exactly what we
    /// put in.
    #[test]
    fn wrapper_round_trip_accessors() {
        let lat = Lattice::new(
            "smoke",
            4,
            vec![(0, 1), (1, 2), (2, 3), (3, 0)],
            vec![vec![0, 1, 2, 3]],
            Some("R2".to_string()),
        );
        let cell_areas = vec![1.0];
        let edge_lengths = vec![1.0, 1.0, 1.0, 1.0];
        let dual = Some(vec![0.25, 0.25, 0.25, 0.25]);

        let lwm = LatticeWithMetric::from_lattice_and_metric(
            lat.clone(),
            cell_areas.clone(),
            edge_lengths.clone(),
            dual.clone(),
        );

        assert_eq!(lwm.lattice(), &lat);
        assert_eq!(lwm.cell_areas(), cell_areas.as_slice());
        assert_eq!(lwm.edge_lengths(), edge_lengths.as_slice());
        assert_eq!(lwm.dual_face_areas(), Some(dual.as_ref().unwrap().as_slice()));
    }

    /// Phase 1 zero-metric placeholder: empty cell / edge vectors and
    /// `None` dual face areas. This is the shape the
    /// truncated-icosahedron registry entry returns from its Phase 1
    /// wrapper (Phase 2 owns assigning a real metric to the buckyball).
    #[test]
    fn zero_metric_placeholder_is_expressible() {
        let lat = Lattice::new(
            "empty_metric",
            3,
            vec![(0, 1), (1, 2), (2, 0)],
            vec![vec![0, 1, 2]],
            Some("S2".to_string()),
        );

        let lwm = LatticeWithMetric::from_lattice_and_metric(
            lat,
            Vec::new(),
            Vec::new(),
            None,
        );

        assert!(lwm.cell_areas().is_empty());
        assert!(lwm.edge_lengths().is_empty());
        assert!(lwm.dual_face_areas().is_none());
    }

    /// Phase-2-additivity sentinel: assert that the Phase 1 type has
    /// only the four accessor methods. If a Phase 2 PR accidentally
    /// adds a `d_0`/`delta_0`/`hodge_star_k` inherent method on this
    /// type instead of as a free function in `lattice::dec`, this
    /// test stays green (it can't observe added methods) — its job
    /// is to document the boundary in code review terms: any new
    /// inherent method on `LatticeWithMetric` should trigger a Phase
    /// 1/2 boundary discussion before landing.
    #[test]
    fn phase1_surface_documented() {
        // Compile-time check: the four accessors exist with the
        // exact signatures Phase 2's free DEC operators will consume.
        let lat = Lattice::new("doc", 1, Vec::new(), Vec::new(), None);
        let lwm = LatticeWithMetric::from_lattice_and_metric(
            lat,
            Vec::new(),
            Vec::new(),
            None,
        );
        let _l: &Lattice = lwm.lattice();
        let _c: &[f64] = lwm.cell_areas();
        let _e: &[f64] = lwm.edge_lengths();
        let _d: Option<&[f64]> = lwm.dual_face_areas();
    }
}
