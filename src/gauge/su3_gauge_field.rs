//! `SU3GaugeField` — second production `EdgeConnection` impl.
//!
//! Halcyon ITEM 3.1 Phase 1 (read-only ingest scope). Wraps a
//! `DenseLinkBuffer` tagged `Group::SU3` (repr_dim = 18, interleaved
//! real/imag pairs for a 3×3 complex matrix) and the metadata that the
//! `GAUGE_FIELD … GROUP SU(3) … INIT …` declaration captured at parse
//! time (name of the bound lattice, init kind, optional seed) so the
//! executor can round-trip the declaration back through SHOW /
//! persistence layers without rebuilding state.
//!
//! Architectural mirror of `SU2GaugeField`. The walker reads through
//! `&dyn EdgeConnection`, never the concrete `SU3GaugeField` type —
//! Bee's locked decision 6. The SU(3)-specific Cabibbo–Marinari
//! heatbath kernel (Phase 2, deferred) will use the `register_su3` /
//! `get_su3_mut` mutability escape in `super::registry`, parallel to
//! the SU(2) pattern. `GaugeFieldInit` is reused unchanged — the init
//! recipe vocabulary (Identity / HaarRandom / FromField) is
//! group-agnostic; only the per-edge bytes the constructor writes
//! differ.
//!
//! Representation per ITEM 3.1 §3.1:
//!   `[re_00, im_00, re_01, im_01, re_02, im_02,
//!     re_10, im_10, re_11, im_11, re_12, im_12,
//!     re_20, im_20, re_21, im_21, re_22, im_22]`
//! → 18 f64 = 144 bytes per link (matches Bee's
//! `inertia_damping/gauge_heatbath_gpu.py` conventions).

use super::dense_link_buffer::DenseLinkBuffer;
use super::edge_connection::EdgeConnection;
use super::error::GaugeFieldError;
use super::group::Group;
use super::group_element::GroupElement;
use super::su2_gauge_field::GaugeFieldInit;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice};

/// SU(3) gauge field bound to a declared lattice.
///
/// Second production `EdgeConnection` impl (Halcyon ITEM 3.1 Phase 1).
/// The `buffer` carries the per-edge 3×3 complex matrices as 18 f64
/// in interleaved real/imag pairs (row-major). `init_kind` /
/// `init_seed` are the metadata the executor + persistence layers
/// need to round-trip the declaration through SHOW.
#[derive(Debug, Clone)]
pub struct SU3GaugeField {
    /// User-facing field name (the `ident` in `GAUGE_FIELD ident …;`).
    pub name: String,
    /// Name of the lattice this field is bound to.
    pub lattice_name: String,
    /// Dense per-edge 3×3-complex-matrix buffer, group-erased.
    /// `buffer.group == Group::SU3`, `buffer.repr_dim == 18`.
    pub buffer: DenseLinkBuffer,
    /// How this field was initialized (round-tripped through SHOW).
    pub init_kind: GaugeFieldInit,
    /// Seed used for `HaarRandom` init (None for `Identity`/`FromField`).
    pub init_seed: Option<u64>,
}

impl SU3GaugeField {
    /// Materialize an SU(3) gauge field on `lattice` per the init
    /// recipe.
    ///
    /// - `Identity` → every edge is `I_3 = diag(1+0i, 1+0i, 1+0i)`;
    ///   `seed` is ignored.
    /// - `HaarRandom` → `seed` is mandatory; `None` returns
    ///   `Err(GaugeFieldError::SeedRequired)`. The Haar draw uses the
    ///   Mezzadri 2007 algorithm (complex Ginibre + QR + diagonal
    ///   phase normalization) — deterministic per `seed`.
    /// - `FromField(_)` → not materializable through this constructor
    ///   alone (the source-field lookup is the executor's job);
    ///   returns `Err(GaugeFieldError::FieldNotDeclared(name))` so the
    ///   executor knows to do its own resolution.
    pub fn new(
        name: String,
        lattice: &Lattice,
        init: GaugeFieldInit,
        seed: Option<u64>,
    ) -> Result<Self, GaugeFieldError> {
        let n_edges = lattice.n_edges();
        let buffer = match &init {
            GaugeFieldInit::Identity => DenseLinkBuffer::new_identity(Group::SU3, n_edges)?,
            GaugeFieldInit::HaarRandom => {
                let s = seed.ok_or(GaugeFieldError::SeedRequired)?;
                DenseLinkBuffer::new_haar(Group::SU3, n_edges, s)?
            }
            GaugeFieldInit::FromField(src) => {
                return Err(GaugeFieldError::FieldNotDeclared(src.clone()));
            }
            GaugeFieldInit::FluxRandom | GaugeFieldInit::FluxUniform => {
                // INIT FLUX is a U(1) bundle materialization
                // (gauge::u1_flux) — never an SU(3) link buffer.
                return Err(GaugeFieldError::FluxInitRequiresU1(Group::SU3));
            }
        };
        Ok(Self {
            name,
            lattice_name: lattice.name.clone(),
            buffer,
            init_kind: init,
            init_seed: seed,
        })
    }

    /// **Test-only sugar** — build an `SU3GaugeField` from a
    /// pre-materialized `DenseLinkBuffer`, bypassing the INIT routines
    /// `new` runs. Mirrors `SU2GaugeField::from_buffer`.
    #[doc(hidden)]
    pub fn from_buffer(
        name: String,
        lattice_name: String,
        buffer: DenseLinkBuffer,
        init_kind: GaugeFieldInit,
        init_seed: Option<u64>,
    ) -> Self {
        Self {
            name,
            lattice_name,
            buffer,
            init_kind,
            init_seed,
        }
    }

    /// Halcyon ITEM 3.1 Phase 1 persistence arm: install `new_buffer`
    /// into `self.buffer.data` in place. Used by the WAL
    /// `OP_GAUGE_FIELD_SNAPSHOT` replay arm after `Engine::open` has
    /// rebuilt the field via the metadata-only `GAUGE_FIELD_DECLARE`
    /// pass. Mirrors `SU2GaugeField::replace_buffer`.
    ///
    /// Validates `new_buffer.len() == self.buffer.n_edges *
    /// self.buffer.repr_dim` (== n_edges * 18 for SU(3)) and rejects
    /// with `GaugeFieldError::BufferShapeMismatch` if the wire data
    /// disagrees with the field's declared shape. Group identity is
    /// the WAL replay's job — the snapshot payload's group tag is
    /// checked against `handle.group()` before this method is reached.
    pub fn replace_buffer(
        &mut self,
        new_buffer: Vec<f64>,
    ) -> Result<(), GaugeFieldError> {
        let expected = self.buffer.n_edges * self.buffer.repr_dim;
        if new_buffer.len() != expected {
            return Err(GaugeFieldError::BufferShapeMismatch {
                expected,
                got: new_buffer.len(),
            });
        }
        self.buffer.data = new_buffer;
        Ok(())
    }

    /// Read-only view of the raw interleaved real/imag buffer
    /// (`n_edges * 18` f64s). Phase 2 Cabibbo–Marinari and the
    /// plaquette dispatcher use this to access per-link matrix bytes
    /// without going through `read_element`'s GroupElement decode.
    pub fn buffer(&self) -> &[f64] {
        &self.buffer.data
    }

    /// Mutable view of the raw interleaved real/imag buffer. Reserved
    /// for the Phase 2 Cabibbo–Marinari kernel that mutates SU(3)
    /// links in place via the `register_su3` / `get_su3_mut`
    /// mutability escape in `super::registry`. Phase 1 callers must
    /// not mutate (read-only ingest scope).
    pub fn buffer_mut(&mut self) -> &mut [f64] {
        &mut self.buffer.data
    }
}

impl EdgeConnection for SU3GaugeField {
    fn edge_element(&self, edge: EdgeId, orientation: EdgeOrientation) -> GroupElement {
        let canonical = self.buffer.read_element(edge);
        match orientation {
            EdgeOrientation::Forward => canonical,
            EdgeOrientation::Reverse => canonical.inverse(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    /// Halcyon ITEM 3.1 Phase 1: identity SU(3) field on the buckyball
    /// materializes through the `new` constructor (gate lifted from
    /// the SU(2)-only era).
    #[test]
    fn su3_identity_init_on_buckyball() {
        let lat = buckyball();
        let field = SU3GaugeField::new(
            "U_su3_id".into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init must succeed");
        assert_eq!(field.init_kind, GaugeFieldInit::Identity);
        assert_eq!(field.init_seed, None);
        assert_eq!(field.buffer.group, Group::SU3);
        assert_eq!(field.buffer.n_edges, lat.n_edges());
        assert_eq!(field.buffer.repr_dim, 18);
        // Identity check on a couple of edges.
        for edge in [0_usize, 7, 89] {
            let g = field.buffer.read_element(edge);
            assert_eq!(g, GroupElement::su3_identity());
        }
    }

    /// HaarRandom init without a seed surfaces the typed `SeedRequired`
    /// error (mirrors SU(2) contract; same SEED-keyword check Halcyon's
    /// regex anchor matches).
    #[test]
    fn su3_haar_seed_required() {
        let lat = buckyball();
        let err = SU3GaugeField::new(
            "U_su3_haar".into(),
            &lat,
            GaugeFieldInit::HaarRandom,
            None,
        )
        .expect_err("HaarRandom without seed must error");
        assert_eq!(err, GaugeFieldError::SeedRequired);
        assert!(err.to_string().to_uppercase().contains("SEED"));
    }

    /// HaarRandom with a seed produces a buffer byte-identical to
    /// `DenseLinkBuffer::new_haar(SU3, …, seed)`. Bit-identity
    /// contract lifted from the storage layer to the field layer.
    #[test]
    fn su3_haar_init_round_trip() {
        let lat = buckyball();
        let field = SU3GaugeField::new(
            "U_su3_h".into(),
            &lat,
            GaugeFieldInit::HaarRandom,
            Some(20260626),
        )
        .expect("haar init with seed must succeed");
        let reference =
            DenseLinkBuffer::new_haar(Group::SU3, lat.n_edges(), 20260626).unwrap();
        assert_eq!(field.buffer.data, reference.data);
        assert_eq!(field.init_kind, GaugeFieldInit::HaarRandom);
        assert_eq!(field.init_seed, Some(20260626));
    }

    /// EdgeConnection: forward returns canonical element; reverse
    /// returns its inverse (conjugate transpose for SU(3)).
    #[test]
    fn su3_edge_connection_forward_vs_reverse() {
        let lat = buckyball();
        let field = SU3GaugeField::new(
            "U_su3_orient".into(),
            &lat,
            GaugeFieldInit::HaarRandom,
            Some(20260626),
        )
        .unwrap();
        let edge: usize = 0;
        let fwd = field.edge_element(edge, EdgeOrientation::Forward);
        let rev = field.edge_element(edge, EdgeOrientation::Reverse);
        // Forward ∘ Reverse = identity to FP64 tolerance.
        let prod = fwd.compose(&rev);
        match (prod, GroupElement::su3_identity()) {
            (GroupElement::SU3(p), GroupElement::SU3(i)) => {
                for k in 0..18 {
                    assert!(
                        (p[k] - i[k]).abs() < 1e-12,
                        "index {k}: |fwd · rev − I| = {}",
                        (p[k] - i[k]).abs()
                    );
                }
            }
            _ => panic!("expected SU3 variants"),
        }
    }

    /// `replace_buffer` succeeds when shape matches and rejects on
    /// shape mismatch.
    #[test]
    fn su3_replace_buffer_shape_check() {
        let lat = buckyball();
        let mut field = SU3GaugeField::new(
            "U_su3_repl".into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let n = field.buffer.n_edges * field.buffer.repr_dim;
        let right_sized: Vec<f64> = vec![0.0; n];
        assert!(field.replace_buffer(right_sized).is_ok());
        let wrong_sized: Vec<f64> = vec![0.0; n + 1];
        let err = field.replace_buffer(wrong_sized).unwrap_err();
        assert!(matches!(
            err,
            GaugeFieldError::BufferShapeMismatch { .. }
        ));
    }
}
