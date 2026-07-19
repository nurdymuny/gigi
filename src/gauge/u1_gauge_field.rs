//! `U1GaugeField` — third production `EdgeConnection` impl (U(1)).
//!
//! The Navier–Stokes linking-number ship (2026-07-18). Wraps a
//! `DenseLinkBuffer` tagged `Group::U1` (repr_dim = 1, one phase θ per
//! edge) and the metadata that the `GAUGE_FIELD … GROUP U(1) … INIT FROM
//! BUNDLE …` declaration captured at parse time (name of the bound
//! lattice, init kind, optional seed) so the executor can round-trip the
//! declaration back through SHOW / persistence layers.
//!
//! Architectural mirror of `SU2GaugeField` / `SU3GaugeField` — the exact
//! new-struct-with-zero-registry/walker/parser-churn shape the group-
//! erasure plan anticipated (registry.rs group-erasure note). The walker
//! reads through `&dyn EdgeConnection`, never the concrete type. U(1)
//! needs NO mutability-escape sibling (`register_su2` / `register_su3`
//! exist only for the GIBBS_SAMPLE / Cabibbo–Marinari heatbath, which
//! U(1) does not run this phase): the plain `registry::register` +
//! `registry::get` dyn surface is exactly what HOLONOMY reads through.
//!
//! ORIENTATION: `edge_element(eid, Forward)` returns the canonical stored
//! phase `U1{θ}`; `edge_element(eid, Reverse)` returns its inverse
//! `U1{−θ}` (the U(1) group inverse). This is the same Forward = θ /
//! Reverse = −θ convention `u1_flux` MODE MAGNETIC and `inject` use, so a
//! chosen circulation reads back with the intended sign.

use super::dense_link_buffer::DenseLinkBuffer;
use super::edge_connection::EdgeConnection;
use super::error::GaugeFieldError;
use super::group::Group;
use super::group_element::GroupElement;
use super::su2_gauge_field::GaugeFieldInit;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice};

/// U(1) gauge field bound to a declared lattice.
///
/// Third production `EdgeConnection` impl (2026-07-18 linking ship). The
/// `buffer` carries the per-edge phases as a single f64 each
/// (`buffer.group == Group::U1`, `buffer.repr_dim == 1`). `init_kind` /
/// `init_seed` are the metadata the executor + SHOW layers need to
/// round-trip the declaration.
#[derive(Debug, Clone)]
pub struct U1GaugeField {
    /// User-facing field name (the `ident` in `GAUGE_FIELD ident …;`).
    pub name: String,
    /// Name of the lattice this field is bound to.
    pub lattice_name: String,
    /// Dense per-edge phase buffer, group-erased.
    /// `buffer.group == Group::U1`, `buffer.repr_dim == 1`.
    pub buffer: DenseLinkBuffer,
    /// How this field was initialized (round-tripped through SHOW).
    pub init_kind: GaugeFieldInit,
    /// Seed metadata (None for `Identity` / `FromBundle`).
    pub init_seed: Option<u64>,
}

impl U1GaugeField {
    /// Materialize a U(1) gauge field on `lattice` per the init recipe.
    ///
    /// - `Identity` → every edge is `θ = 0` (`e^{i·0} = 1`); `seed` ignored.
    /// - `FromBundle(_)` → resolved by the executor (which holds the engine
    ///   handle to read the bundle records); a bare `new` surfaces a typed
    ///   `BundleNotFound` (mirrors `SU2GaugeField::new`).
    /// - `FromField(_)` → not supported through this constructor
    ///   (`FieldNotDeclared`).
    /// - `HaarRandom` / `FluxRandom` / `FluxUniform` → U(1) has no link-
    ///   buffer HAAR/FLUX path: random / flux U(1) phases materialize as a
    ///   *theta bundle* (`gauge::u1_flux`, INIT FLUX) and are injected via
    ///   INIT FROM BUNDLE. The executor routes those before construction;
    ///   a bare `new` reaching here returns `UnsupportedGroup` (never a
    ///   panic).
    pub fn new(
        name: String,
        lattice: &Lattice,
        init: GaugeFieldInit,
        seed: Option<u64>,
    ) -> Result<Self, GaugeFieldError> {
        let n_edges = lattice.n_edges();
        let buffer = match &init {
            GaugeFieldInit::Identity => DenseLinkBuffer::new_identity(Group::U1, n_edges)?,
            GaugeFieldInit::FromBundle(bundle) => {
                return Err(GaugeFieldError::BundleNotFound(bundle.clone()));
            }
            GaugeFieldInit::FromField(src) => {
                return Err(GaugeFieldError::FieldNotDeclared(src.clone()));
            }
            GaugeFieldInit::HaarRandom
            | GaugeFieldInit::FluxRandom
            | GaugeFieldInit::FluxUniform => {
                return Err(GaugeFieldError::UnsupportedGroup(Group::U1));
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

    /// Build a `U1GaugeField` from a pre-materialized `DenseLinkBuffer`
    /// (the INIT FROM BUNDLE path — the executor resolves the chosen
    /// per-edge phases via `gauge::inject::u1_buffer_from_bundle` and
    /// hands the buffer here). Mirrors `SU2GaugeField::from_buffer` /
    /// `SU3GaugeField::from_buffer`.
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
}

impl EdgeConnection for U1GaugeField {
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

    /// Identity U(1) field materializes: repr_dim = 1, every edge θ = 0.
    #[test]
    fn u1_identity_init_on_buckyball() {
        let lat = buckyball();
        let field =
            U1GaugeField::new("u1_id".into(), &lat, GaugeFieldInit::Identity, None)
                .expect("identity init must succeed");
        assert_eq!(field.buffer.group, Group::U1);
        assert_eq!(field.buffer.n_edges, lat.n_edges());
        assert_eq!(field.buffer.repr_dim, 1);
        for edge in [0_usize, 7, 89] {
            assert_eq!(field.buffer.read_element(edge), GroupElement::U1 { theta: 0.0 });
        }
    }

    /// EdgeConnection: Forward returns the stored phase; Reverse returns
    /// its inverse (−θ). Plant a phase and check both arms.
    #[test]
    fn u1_edge_connection_forward_vs_reverse() {
        let lat = buckyball();
        let mut field =
            U1GaugeField::new("u1_orient".into(), &lat, GaugeFieldInit::Identity, None).unwrap();
        field.buffer.write_u1_row(7, 0.8);
        match field.edge_element(7, EdgeOrientation::Forward) {
            GroupElement::U1 { theta } => assert!((theta - 0.8).abs() < 1e-12),
            other => panic!("expected U1, got {other:?}"),
        }
        match field.edge_element(7, EdgeOrientation::Reverse) {
            GroupElement::U1 { theta } => assert!((theta + 0.8).abs() < 1e-12, "Reverse = −θ"),
            other => panic!("expected U1, got {other:?}"),
        }
    }

    /// FromBundle / FromField / HaarRandom through the bare `new` surface
    /// typed errors (never panics) — the executor intercepts the real
    /// paths.
    #[test]
    fn u1_new_non_identity_inits_are_typed_errors() {
        let lat = buckyball();
        assert!(matches!(
            U1GaugeField::new("x".into(), &lat, GaugeFieldInit::FromBundle("b".into()), None),
            Err(GaugeFieldError::BundleNotFound(_))
        ));
        assert!(matches!(
            U1GaugeField::new("x".into(), &lat, GaugeFieldInit::FromField("f".into()), None),
            Err(GaugeFieldError::FieldNotDeclared(_))
        ));
        assert!(matches!(
            U1GaugeField::new("x".into(), &lat, GaugeFieldInit::HaarRandom, Some(1)),
            Err(GaugeFieldError::UnsupportedGroup(Group::U1))
        ));
    }
}
