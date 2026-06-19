//! `SU2EField` — Part IV E-field sibling primitive.
//!
//! Closes TDD-HAL-IV.1. The E field is the conjugate momentum to the
//! SU(2) gauge field U; it lives in the Lie algebra su(2) (the
//! imaginary quaternions) and gets packed into a `(n_edges, 4)` row-
//! major `DenseLinkBuffer` with the `q0 = 0` invariant enforced at
//! every constructor entry AND every buffer mutation (Bee's locked
//! decision IV-C).
//!
//! Group-erasure note (Bee's locked decision IV-B): `SU2EField` is a
//! sibling struct to `SU2GaugeField`, NOT a field on it. E does not
//! implement `EdgeConnection` — E has no group inverse and no face-
//! walk semantics; it is a tangent-space object that the symplectic
//! flow integrates, not a connection the walker reads through. Future
//! U(1)/SU(3)/Z(N) E-fields will land as separate structs
//! (`U1EField`, `SU3EField`, …) parallel to their gauge counterparts;
//! the sibling registry slot (`register_su2_e` / `get_su2_e_mut`) is
//! the storage pattern they will mirror.
//!
//! Maxwell–Boltzmann sigma (Halcyon canonical_sigma packing): for
//! SU(2) `dim = 3`, so `σ = sqrt(1 / (β · 1.5))`. The per-edge sample
//! draws three standard normals via Box–Muller from four uniforms
//! (the fourth uniform's normal is consumed but discarded — fixed
//! RNG-state advance per edge is required for A2 row 1 bit-identity).
//!
//! `EFieldInit::FromField(other)` clones from another registered E
//! field. The lookup is the constructor's job (registry handle is
//! captured at construction time); the cross-lattice mismatch case
//! surfaces as `GaugeFieldError::EFieldSourceMismatch` so the parser
//! can refuse the declaration before any state mutates.

use std::sync::Arc;

use super::dense_link_buffer::DenseLinkBuffer;
use super::error::GaugeFieldError;
use super::group::Group;
use super::marsaglia_haar::{maxwell_boltzmann_su2, SmallRng};
use super::registry::GaugeFieldHandle;
use crate::lattice::EdgeId;

/// Initialization recipe for an `E_FIELD` declaration.
///
/// - `Zero` — every Lie-row is `(0, 0, 0, 0)`. No seed required.
/// - `MaxwellBoltzmann { beta }` — per-edge: three N(0,1) samples
///   scaled by `σ = sqrt(1 / (β · 1.5))`. SEED is mandatory; omission
///   surfaces as `GaugeFieldError::SeedRequired`.
/// - `FromField(name)` — clone an already-registered E field's
///   buffer. Source lattice MUST match the target U's lattice.
#[derive(Debug, Clone, PartialEq)]
pub enum EFieldInit {
    Zero,
    MaxwellBoltzmann { beta: f64 },
    FromField(String),
}

/// SU(2) E field bound to a declared `SU2GaugeField`.
///
/// `source_gauge_field` and `source_lattice` are the metadata the
/// executor + persistence layers need to round-trip the declaration
/// through SHOW E_FIELD. The buffer is `(n_edges, 4)` row-major with
/// `q0 = 0` on every row.
#[derive(Debug, Clone)]
pub struct SU2EField {
    /// User-facing field name (the `ident` in `E_FIELD ident …;`).
    pub name: String,
    /// Name of the U field this E binds to.
    pub source_gauge_field: String,
    /// Name of the lattice the source U is bound to.
    pub source_lattice: String,
    /// Dense per-edge Lie-algebra buffer (q0=0 invariant on every row).
    pub buffer: DenseLinkBuffer,
    /// How this field was initialized (round-tripped through SHOW).
    pub init_kind: EFieldInit,
    /// Seed used for `MaxwellBoltzmann` init (None for Zero / FromField).
    pub init_seed: Option<u64>,
}

/// Object-safe handle the sibling registry stores. Parallel to
/// `GaugeFieldHandle` but without `EdgeConnection` extension — E has
/// no group inverse / face-walk semantics.
pub trait EFieldHandle: Send + Sync {
    fn name(&self) -> &str;
    fn source_gauge_field(&self) -> &str;
    fn source_lattice(&self) -> &str;
    fn group(&self) -> Group;
    fn init_metadata(&self) -> (EFieldInit, Option<u64>);
    fn as_dense_buffer(&self) -> &DenseLinkBuffer;
}

impl EFieldHandle for SU2EField {
    fn name(&self) -> &str {
        &self.name
    }
    fn source_gauge_field(&self) -> &str {
        &self.source_gauge_field
    }
    fn source_lattice(&self) -> &str {
        &self.source_lattice
    }
    fn group(&self) -> Group {
        self.buffer.group
    }
    fn init_metadata(&self) -> (EFieldInit, Option<u64>) {
        (self.init_kind.clone(), self.init_seed)
    }
    fn as_dense_buffer(&self) -> &DenseLinkBuffer {
        &self.buffer
    }
}

impl SU2EField {
    /// Materialize an SU(2) E field bound to `source_gauge_field` per
    /// the init recipe.
    ///
    /// - `Zero` — every Lie-row is `(0, 0, 0, 0)`; `seed` is ignored.
    /// - `MaxwellBoltzmann { beta }` — `seed` is mandatory; `None`
    ///   returns `Err(GaugeFieldError::SeedRequired)`. Per-edge MB
    ///   sample via Box–Muller; q0 forced to 0 on every row.
    /// - `FromField(other)` — look the source E up via
    ///   `registry::get_su2_e_mut(other)`; if missing returns
    ///   `Err(GaugeFieldError::EFieldNotDeclared(other))`. If the
    ///   source E's lattice differs from the target U's lattice,
    ///   returns `Err(GaugeFieldError::EFieldSourceMismatch { … })`.
    pub fn new<H: GaugeFieldHandle + ?Sized>(
        name: String,
        source_gauge_field: &H,
        init: EFieldInit,
        seed: Option<u64>,
    ) -> Result<Self, GaugeFieldError> {
        let u_lattice = source_gauge_field.lattice_name().to_string();
        let u_name = source_gauge_field.name().to_string();
        let n_edges = source_gauge_field.as_dense_buffer().n_edges;
        let buffer = match &init {
            EFieldInit::Zero => DenseLinkBuffer::new_zero(Group::SU2, n_edges)?,
            EFieldInit::MaxwellBoltzmann { beta } => {
                let s = seed.ok_or(GaugeFieldError::SeedRequired)?;
                let mut buf = DenseLinkBuffer::new_zero(Group::SU2, n_edges)?;
                let mut rng = SmallRng::seed_from_u64(s);
                for edge in 0..n_edges {
                    let q = maxwell_boltzmann_su2(&mut rng, *beta);
                    buf.write_lie_row(edge, q);
                }
                buf
            }
            EFieldInit::FromField(src) => {
                let src_arc = super::registry::get_su2_e_mut(src)
                    .ok_or_else(|| GaugeFieldError::EFieldNotDeclared(src.clone()))?;
                let src_guard = src_arc
                    .lock()
                    .expect("source E field mutex poisoned");
                if src_guard.source_lattice != u_lattice {
                    return Err(GaugeFieldError::EFieldSourceMismatch {
                        e_lattice: src_guard.source_lattice.clone(),
                        u_lattice,
                    });
                }
                src_guard.buffer.clone()
            }
        };
        Ok(Self {
            name,
            source_gauge_field: u_name,
            source_lattice: u_lattice,
            buffer,
            init_kind: init,
            init_seed: seed,
        })
    }

    /// Read the Lie row at `edge` as `[0, q1, q2, q3]`. `q0` is
    /// guaranteed zero by the constructor + `write_element_q`
    /// invariants.
    pub fn read_element_q(&self, edge: EdgeId) -> [f64; 4] {
        self.buffer.read_lie_row(edge)
    }

    /// Write a Lie row, enforcing `q0 = 0` at the boundary
    /// (regardless of the caller's input).
    pub fn write_element_q(&mut self, edge: EdgeId, q: [f64; 4]) {
        self.buffer.write_lie_row(edge, q);
    }
}

// Silence "Arc unused" warning when this module is consumed without
// the registry-export surface in play.
#[allow(dead_code)]
type _ArcHint = Arc<()>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    /// `EFieldInit::Zero` round-trips: q0=0 everywhere AND every other
    /// component is zero too.
    #[test]
    fn zero_init_buffer_is_all_zeros() {
        let bb = buckyball();
        let u = SU2GaugeField::new(
            "U".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let e = SU2EField::new("E".into(), &u, EFieldInit::Zero, None).unwrap();
        assert_eq!(e.buffer.n_edges, 90);
        assert_eq!(e.buffer.data, vec![0.0; 360]);
    }

    /// `EFieldInit::MaxwellBoltzmann` without a seed errors with
    /// `SeedRequired`.
    #[test]
    fn mb_init_without_seed_errors() {
        let bb = buckyball();
        let u = SU2GaugeField::new(
            "U".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let err = SU2EField::new(
            "E".into(),
            &u,
            EFieldInit::MaxwellBoltzmann { beta: 2.5 },
            None,
        )
        .unwrap_err();
        assert_eq!(err, GaugeFieldError::SeedRequired);
    }

    /// `write_element_q` zeroes q0 regardless of input.
    #[test]
    fn write_element_q_enforces_q0_zero() {
        let bb = buckyball();
        let u = SU2GaugeField::new(
            "U".into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .unwrap();
        let mut e =
            SU2EField::new("E".into(), &u, EFieldInit::Zero, None).unwrap();
        e.write_element_q(3, [99.0, 1.0, 2.0, 3.0]);
        let row = e.read_element_q(3);
        assert_eq!(row, [0.0, 1.0, 2.0, 3.0]);
    }
}
