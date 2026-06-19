//! `SU2GaugeField` — first production `EdgeConnection` impl.
//!
//! Closes TDD-HAL-II.3. Wraps a `DenseLinkBuffer` tagged `Group::SU2`
//! and the metadata that the GAUGE_FIELD declaration captured at
//! parse time (name of the bound lattice, init kind, optional seed)
//! so the executor can round-trip a `GAUGE_FIELD … INIT …` declaration
//! back through SHOW / persistence layers without rebuilding state.
//!
//! Architectural payoff (Bee's locked decision 6): the walker reads
//! through `&dyn EdgeConnection`, never the concrete `SU2GaugeField`
//! type. A future `U1GaugeField` is a separate struct with an
//! identical layout pattern (same `DenseLinkBuffer` wrapper, same
//! `EdgeConnection` impl shape, just `repr_dim = 1` and `read_element`
//! returns `GroupElement::U1`). Zero changes to `SU2GaugeField`, the
//! walker, the registry, the parser, or the HTTP routes when that
//! ships.

use super::dense_link_buffer::DenseLinkBuffer;
use super::edge_connection::EdgeConnection;
use super::error::GaugeFieldError;
use super::group::Group;
use super::group_element::GroupElement;
use crate::lattice::{EdgeId, EdgeOrientation, Lattice};

/// Initialization recipe for a GAUGE_FIELD declaration.
///
/// `Identity` and `HaarRandom` are the two primary launch paths;
/// `FromField` clones another field's buffer (used by the executor's
/// `INIT FROM_FIELD other` declaration and by the BundleStore
/// re-hydration path in TDD-HAL-II.4b).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaugeFieldInit {
    /// `INIT IDENTITY` — every edge is the group identity. SU(2)
    /// identity is `(1, 0, 0, 0)`. No seed required.
    Identity,
    /// `INIT HAAR_RANDOM SEED <u64>` — uniform on the group manifold,
    /// seeded by `GAUGE_FIELD::new`'s `seed` argument. SEED is
    /// mandatory; omission surfaces as `GaugeFieldError::SeedRequired`.
    HaarRandom,
    /// `INIT FROM_FIELD other` — clone the buffer of another declared
    /// field (resolved by name at executor time, not at construction
    /// time; this struct only records the source-field name).
    FromField(String),
}

/// SU(2) gauge field bound to a declared lattice.
///
/// First production `EdgeConnection` impl. The `buffer` carries the
/// per-edge SU(2) quaternions (scalar-first `(q0, q1, q2, q3)`),
/// `init_kind`/`init_seed` are the metadata the executor + persistence
/// layers need to round-trip the declaration.
#[derive(Debug, Clone)]
pub struct SU2GaugeField {
    /// User-facing field name (the `ident` in `GAUGE_FIELD ident …;`).
    pub name: String,
    /// Name of the lattice this field is bound to.
    pub lattice_name: String,
    /// Dense per-edge quaternion buffer, group-erased.
    pub buffer: DenseLinkBuffer,
    /// How this field was initialized (round-tripped through SHOW).
    pub init_kind: GaugeFieldInit,
    /// Seed used for `HaarRandom` init (None for `Identity`/`FromField`).
    pub init_seed: Option<u64>,
}

impl SU2GaugeField {
    /// Materialize an SU(2) gauge field on `lattice` per the init
    /// recipe.
    ///
    /// - `Identity` → every edge is `(1, 0, 0, 0)`; `seed` is ignored.
    /// - `HaarRandom` → `seed` is mandatory; `None` returns
    ///   `Err(GaugeFieldError::SeedRequired)`.
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
            GaugeFieldInit::Identity => DenseLinkBuffer::new_identity(Group::SU2, n_edges)?,
            GaugeFieldInit::HaarRandom => {
                let s = seed.ok_or(GaugeFieldError::SeedRequired)?;
                DenseLinkBuffer::new_haar(Group::SU2, n_edges, s)?
            }
            GaugeFieldInit::FromField(src) => {
                return Err(GaugeFieldError::FieldNotDeclared(src.clone()));
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

    /// **Test-only sugar** — build an `SU2GaugeField` from a
    /// pre-materialized `DenseLinkBuffer`, bypassing the INIT
    /// routines `new` runs. Used by the TDD-HAL-II.7 gold-walker
    /// gate to load the Part I gold quaternion fixture directly
    /// into a field (the fixture is a frozen reference state, not
    /// the output of an INIT routine).
    ///
    /// Production code MUST go through `SU2GaugeField::new` with a
    /// `GaugeFieldInit` clause so the init recipe is recorded in
    /// the field's metadata and the executor / persistence layer
    /// can round-trip the declaration through SHOW. This factory
    /// is hidden from rustdoc to discourage production use.
    ///
    /// Gated on `#[doc(hidden)]` rather than `#[cfg(test)]` because
    /// the TDD-HAL-II.7 gate is an integration test (separate
    /// crate from the library) — `#[cfg(test)]` items are invisible
    /// across that boundary. The Halcyon spec section 6 marks this
    /// constructor as test-only sugar; production callers are
    /// expected to use `new`.
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

    /// TDD-HAL-V.3: install `new_buffer` into `self.buffer.data` in
    /// place. Used by the WAL `OP_GAUGE_FIELD_SNAPSHOT` replay arm
    /// after `Engine::open` has rebuilt the field via the metadata-only
    /// `GAUGE_FIELD_DECLARE` pass — the snapshot payload's buffer
    /// overwrites the freshly-materialized identity / Haar state with
    /// the post-thermalization state that was captured before close.
    ///
    /// Validates `new_buffer.len() == self.buffer.n_edges *
    /// self.buffer.repr_dim` and rejects with
    /// `GaugeFieldError::BufferShapeMismatch` if the wire data ever
    /// disagrees with the field's declared shape (defense in depth
    /// against a malformed payload that slipped past the SHA-256 gate
    /// upstream). Group identity is the WAL replay's job — the snapshot
    /// payload's group tag is checked against `handle.group()` before
    /// this method is reached (see `WalError::SnapshotGroupMismatch`).
    ///
    /// Idempotent — multiple snapshot entries for the same field
    /// replay last-write-wins, which is the semantics callers want
    /// (every `SNAPSHOT GAUGE_FIELD U PERSIST` overwrites the previous
    /// snapshot's buffer in place).
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
}

impl EdgeConnection for SU2GaugeField {
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
    use crate::gauge::holonomy::{face_edges, walk_loop};
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    /// TDD-HAL-II.3: identity SU2GaugeField on the buckyball returns
    /// the SU(2) identity through every face walk via &dyn
    /// EdgeConnection — the architectural payoff (walker is
    /// group-erased, reads through trait, not concrete type).
    #[test]
    fn tdd_hal_ii_3_field_walks_face_holonomy_identity() {
        let lat = buckyball();
        let field = SU2GaugeField::new(
            "U".into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init must succeed");

        // Walker reads through &dyn EdgeConnection — this is the
        // architectural contract (Bee's locked decision 6).
        let conn: &dyn EdgeConnection = &field;
        let id = GroupElement::SU2 {
            q0: 1.0,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
        };
        for fidx in 0..lat.n_faces() {
            let edges = face_edges(&lat, fidx);
            let h = walk_loop(&lat, &edges, conn);
            assert_eq!(h, id, "face {fidx}");
        }
    }

    /// TDD-HAL-II.3: HaarRandom init without a seed surfaces the
    /// typed `SeedRequired` error (not a panic). Display must contain
    /// the substring "SEED" so Halcyon's `match="SEED"` check hits.
    #[test]
    fn tdd_hal_ii_3_seed_required_typed_error() {
        let lat = buckyball();
        let err = SU2GaugeField::new(
            "X".into(),
            &lat,
            GaugeFieldInit::HaarRandom,
            None,
        )
        .expect_err("HaarRandom without seed must error");
        assert_eq!(err, GaugeFieldError::SeedRequired);
        // Case-insensitive substring match (the Halcyon test uses
        // match="SEED"; check the literal upper-case substring lands
        // in Display).
        let s = format!("{}", err);
        assert!(
            s.to_uppercase().contains("SEED"),
            "Display must contain 'SEED', got: {s}"
        );
    }

    /// TDD-HAL-II.3: the `UnsupportedGroup` variant exists and its
    /// Display contains "SU(2)" — that's the Halcyon G2.D regex
    /// anchor (Bee's locked decision 5).
    #[test]
    fn tdd_hal_ii_3_unsupported_group_typed_error() {
        assert!(
            format!("{}", GaugeFieldError::UnsupportedGroup(Group::U1))
                .contains("SU(2)")
        );
        assert!(
            format!("{}", GaugeFieldError::UnsupportedGroup(Group::SU3))
                .contains("SU(2)")
        );
    }

    /// TDD-HAL-II.3: HaarRandom init with a seed produces a buffer
    /// byte-identical to `DenseLinkBuffer::new_haar(SU2, n_edges,
    /// seed)`. This is the intra-binding bit-identity contract
    /// (Bee's locked decision 1) lifted from the storage layer to
    /// the field layer.
    #[test]
    fn tdd_hal_ii_3_haar_init_round_trip() {
        let lat = buckyball();
        let field = SU2GaugeField::new(
            "U_h".into(),
            &lat,
            GaugeFieldInit::HaarRandom,
            Some(20260616),
        )
        .expect("haar init with seed must succeed");
        let reference =
            DenseLinkBuffer::new_haar(Group::SU2, lat.n_edges(), 20260616).unwrap();
        assert_eq!(field.buffer.data, reference.data);
        assert_eq!(field.init_kind, GaugeFieldInit::HaarRandom);
        assert_eq!(field.init_seed, Some(20260616));
        assert_eq!(field.lattice_name, lat.name);
    }

    /// Identity init records the right metadata (init_kind = Identity,
    /// init_seed unused).
    #[test]
    fn identity_init_metadata() {
        let lat = buckyball();
        let field = SU2GaugeField::new(
            "U_id".into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        assert_eq!(field.init_kind, GaugeFieldInit::Identity);
        assert_eq!(field.init_seed, None);
        assert_eq!(field.buffer.group, Group::SU2);
        assert_eq!(field.buffer.n_edges, lat.n_edges());
        assert_eq!(field.buffer.repr_dim, 4);
    }

    /// FromField surfaces the typed `FieldNotDeclared` error so the
    /// executor knows to do source-field resolution itself (the
    /// constructor doesn't have a registry handle).
    #[test]
    fn from_field_returns_field_not_declared() {
        let lat = buckyball();
        let err = SU2GaugeField::new(
            "U_clone".into(),
            &lat,
            GaugeFieldInit::FromField("U_src".into()),
            None,
        )
        .expect_err("FromField without registry must error");
        assert_eq!(
            err,
            GaugeFieldError::FieldNotDeclared("U_src".into())
        );
    }

    /// EdgeConnection impl: forward orientation returns the canonical
    /// element; reverse orientation returns its inverse. Plant a
    /// non-trivial quaternion in the buffer and check both arms.
    #[test]
    fn edge_connection_forward_vs_reverse() {
        let lat = buckyball();
        let mut field = SU2GaugeField::new(
            "U".into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        // Half-turn about z-axis on edge 7.
        field.buffer.data[4 * 7] = 0.0;
        field.buffer.data[4 * 7 + 1] = 0.0;
        field.buffer.data[4 * 7 + 2] = 0.0;
        field.buffer.data[4 * 7 + 3] = 1.0;

        let fwd = field.edge_element(7, EdgeOrientation::Forward);
        let rev = field.edge_element(7, EdgeOrientation::Reverse);
        match (fwd, rev) {
            (
                GroupElement::SU2 { q0: f0, q1: f1, q2: f2, q3: f3 },
                GroupElement::SU2 { q0: r0, q1: r1, q2: r2, q3: r3 },
            ) => {
                // Forward is the canonical half-turn.
                assert_eq!(f0, 0.0);
                assert_eq!(f1, 0.0);
                assert_eq!(f2, 0.0);
                assert_eq!(f3, 1.0);
                // Reverse is the conjugate (q0, -q1, -q2, -q3).
                assert_eq!(r0, 0.0);
                assert_eq!(r1, 0.0);
                assert_eq!(r2, 0.0);
                assert_eq!(r3, -1.0);
            }
            _ => panic!("expected SU2 variants"),
        }
    }

    /// Trait object usage compiles: `Box<dyn EdgeConnection>` over a
    /// concrete `SU2GaugeField`. This is the architectural payoff —
    /// the walker, the HTTP routes, the registry never name the
    /// concrete type.
    #[test]
    fn su2_field_is_object_safe_edge_connection() {
        let lat = buckyball();
        let field = SU2GaugeField::new(
            "U".into(),
            &lat,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let boxed: Box<dyn EdgeConnection> = Box::new(field);
        let _ = boxed.edge_element(0, EdgeOrientation::Forward);
    }
}
