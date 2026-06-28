//! `DenseLinkBuffer` — group-erased dense storage for a `GaugeField`.
//!
//! Closes the storage half of TDD-HAL-II.1. Shape is
//! `(n_edges, repr_dim)` row-major `Vec<f64>`; the `group` tag tells
//! `read_element` which `GroupElement` arm a row decodes into.
//!
//! Bee's locked decision 6: this buffer is group-erased, but only
//! SU(2) has live math at launch. Constructors that would materialize
//! a non-SU2 buffer return `GroupNotImplemented` (typed error, not a
//! panic) so the GAUGE_FIELD declaration surface can reject early
//! (decision 5). `read_element` on a non-SU2 buffer panics — that's a
//! programming-error path because the typed-error gate above keeps
//! such buffers from ever being constructed.

use super::error::GaugeFieldError;
use super::group::Group;
use super::group_element::GroupElement;
use super::marsaglia_haar::{haar_random_su2, haar_random_su3, SmallRng};

/// Group-erased dense link buffer. Shape `(n_edges, repr_dim)`
/// row-major. `group` is the per-buffer tag; every row decodes into
/// the corresponding `GroupElement` variant.
#[derive(Debug, Clone)]
pub struct DenseLinkBuffer {
    pub group: Group,
    pub n_edges: usize,
    pub repr_dim: usize,
    pub data: Vec<f64>,
}

impl DenseLinkBuffer {
    /// Build an identity-everywhere buffer.
    ///
    /// For `Group::SU2`: every row is `(1, 0, 0, 0)` (scalar-first
    /// quaternion identity). For `Group::SU3`: every row is the
    /// interleaved-pairs encoding of `I_3 = diag(1, 1, 1)` — real
    /// diagonal entries at row-offsets 0, 8, 16 are 1.0, every other
    /// slot is 0.0 (Halcyon ITEM 3.1 §3.1 representation). For every
    /// other group: returns `Err(GaugeFieldError::UnsupportedGroup(_))`.
    pub fn new_identity(group: Group, n_edges: usize) -> Result<Self, GaugeFieldError> {
        let repr_dim = group.repr_dim();
        match group {
            Group::SU2 => {
                let mut data = vec![0.0_f64; n_edges * repr_dim];
                for i in 0..n_edges {
                    data[repr_dim * i] = 1.0;
                }
                Ok(Self {
                    group,
                    n_edges,
                    repr_dim,
                    data,
                })
            }
            Group::SU3 => {
                // Halcyon ITEM 3.1: I_3 in interleaved row-major layout.
                // Real diagonal entries live at offsets 0, 8, 16 within
                // the 18-f64 per-link slot.
                let mut data = vec![0.0_f64; n_edges * repr_dim];
                for i in 0..n_edges {
                    let base = repr_dim * i;
                    data[base] = 1.0; // re_00
                    data[base + 8] = 1.0; // re_11
                    data[base + 16] = 1.0; // re_22
                }
                Ok(Self {
                    group,
                    n_edges,
                    repr_dim,
                    data,
                })
            }
            other => Err(GaugeFieldError::UnsupportedGroup(other)),
        }
    }

    /// Build a Haar-random buffer (uniform on the group manifold).
    ///
    /// For `Group::SU2`: seeds GIGI's `SmallRng` (xorshift64*) with
    /// `seed`, then for `edge = 0..n_edges` writes the 4-tuple
    /// returned by `haar_random_su2` into rows
    /// `repr_dim * edge .. repr_dim * (edge + 1)`. This makes the
    /// output byte-identical across runs of the same binary on the
    /// same seed (Bee's locked decision 1). For every other group:
    /// returns `Err(GaugeFieldError::UnsupportedGroup(_))` — the
    /// uniform-on-S^3 sampler only applies to SU(2); future U(1) /
    /// SU(3) / Z(N) samplers land beside `haar_random_su2` in the
    /// `marsaglia_haar` module without touching this constructor's
    /// callers.
    pub fn new_haar(group: Group, n_edges: usize, seed: u64) -> Result<Self, GaugeFieldError> {
        let repr_dim = group.repr_dim();
        match group {
            Group::SU2 => {
                let mut rng = SmallRng::seed_from_u64(seed);
                let mut data = vec![0.0_f64; n_edges * repr_dim];
                for edge in 0..n_edges {
                    let q = haar_random_su2(&mut rng);
                    let base = repr_dim * edge;
                    data[base] = q[0];
                    data[base + 1] = q[1];
                    data[base + 2] = q[2];
                    data[base + 3] = q[3];
                }
                Ok(Self {
                    group,
                    n_edges,
                    repr_dim,
                    data,
                })
            }
            Group::SU3 => {
                // Halcyon ITEM 3.1: Mezzadri 2007 (complex Ginibre + QR
                // + det normalization). Per-edge RNG cadence is FIXED
                // at 18 uniforms (no rejection), which preserves the
                // bit-identity contract Bee's locked decision 1
                // demands. Same SmallRng (xorshift64*) as SU(2) so the
                // optionality contract (decision 7) holds — no extra
                // dependency for the SU(3) sampler.
                let mut rng = SmallRng::seed_from_u64(seed);
                let mut data = vec![0.0_f64; n_edges * repr_dim];
                for edge in 0..n_edges {
                    let m = haar_random_su3(&mut rng);
                    let base = repr_dim * edge;
                    // repr_dim == 18 for SU(3); copy the 18 f64s.
                    data[base..base + 18].copy_from_slice(&m);
                }
                Ok(Self {
                    group,
                    n_edges,
                    repr_dim,
                    data,
                })
            }
            other => Err(GaugeFieldError::UnsupportedGroup(other)),
        }
    }

    /// Build an all-zero buffer (no group identity injected).
    ///
    /// Used by the SU(2) E-field primitive (TDD-HAL-IV.1) where each
    /// row is a quaternion-packed Lie-algebra vector with `q0 = 0`
    /// (the identity component of a tangent vector is zero by
    /// definition). For `Group::SU2` the buffer is `(n_edges, 4)`
    /// row-major. Future U(1)/SU(3)/Z(N) E-fields will land beside
    /// `SU2EField` as separate structs; this constructor only knows
    /// about repr_dim, so it is group-agnostic at the storage layer.
    pub fn new_zero(group: Group, n_edges: usize) -> Result<Self, GaugeFieldError> {
        let repr_dim = group.repr_dim();
        match group {
            Group::SU2 => Ok(Self {
                group,
                n_edges,
                repr_dim,
                data: vec![0.0_f64; n_edges * repr_dim],
            }),
            other => Err(GaugeFieldError::UnsupportedGroup(other)),
        }
    }

    /// Write a row to the buffer with the `q0 = 0` Lie-algebra
    /// invariant forced at the write boundary.
    ///
    /// Used by `SU2EField` writers: `q[0]` is silently zeroed before
    /// the row is stored, regardless of what the caller passed in.
    /// This is the q0=0 invariant enforced at every mutation entry
    /// point (Bee's locked decision IV-C).
    pub fn write_lie_row(&mut self, edge: usize, mut q: [f64; 4]) {
        let base = self.repr_dim * edge;
        q[0] = 0.0;
        self.data[base] = q[0];
        self.data[base + 1] = q[1];
        self.data[base + 2] = q[2];
        self.data[base + 3] = q[3];
    }

    /// Read a Lie-algebra row as a 4-tuple `(0, q1, q2, q3)`.
    /// Companion to `write_lie_row`; the q0 slot is guaranteed zero
    /// by the write-side invariant (no defensive zeroing here).
    pub fn read_lie_row(&self, edge: usize) -> [f64; 4] {
        let base = self.repr_dim * edge;
        [
            self.data[base],
            self.data[base + 1],
            self.data[base + 2],
            self.data[base + 3],
        ]
    }

    /// Decode the row at `edge` into a `GroupElement`.
    ///
    /// `Group::SU2` and `Group::SU3` have live math at launch (the
    /// latter via Halcyon ITEM 3.1 Phase 1). Other arms panic because
    /// reaching them from a well-typed buffer is a programming error
    /// — the `new_*` constructors above return `Err` for unsupported
    /// groups, so no such buffer can be observed here. Bee's locked
    /// decision 6.
    pub fn read_element(&self, edge: usize) -> GroupElement {
        let base = self.repr_dim * edge;
        match self.group {
            Group::SU2 => GroupElement::SU2 {
                q0: self.data[base],
                q1: self.data[base + 1],
                q2: self.data[base + 2],
                q3: self.data[base + 3],
            },
            Group::SU3 => {
                // Halcyon ITEM 3.1: copy 18 f64s into the fixed-size
                // array the SU3 variant wraps. Interleaved row-major
                // real/imag pairs (same layout the writers emit).
                let mut m = [0.0_f64; 18];
                m.copy_from_slice(&self.data[base..base + 18]);
                GroupElement::SU3(m)
            }
            Group::U1 => panic!(
                "read_element not implemented for Group::U1 - Part II ships SU(2) math only; \
                 future groups ship as separate EdgeConnection impls per group-erasure plan"
            ),
            Group::ZN { .. } => panic!(
                "read_element not implemented for Group::ZN - Part II ships SU(2) math only; \
                 future groups ship as separate EdgeConnection impls per group-erasure plan"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-HAL-II.1: identity SU(2) buffer round-trips byte-for-byte
    /// to `GroupElement::SU2 { q0=1, q1=q2=q3=0 }` on every edge.
    ///
    /// Frozen-bytes check: for the buckyball edge count (90), the
    /// underlying `Vec<f64>` has length 360 and is laid out so
    /// `data[4*i] == 1.0`, `data[4*i+1..4*i+4] == 0.0` for every edge.
    /// This is the regression sentinel the Part-II gold gates will
    /// pin against (Bee's locked decision 1: gold is harvested from
    /// GIGI itself, not from NumPy).
    #[test]
    fn tdd_hal_ii_1_identity_round_trip_byte_equal() {
        let buffer = DenseLinkBuffer::new_identity(Group::SU2, 90)
            .expect("SU(2) identity buffer must succeed");

        assert_eq!(buffer.group, Group::SU2);
        assert_eq!(buffer.n_edges, 90);
        assert_eq!(buffer.repr_dim, 4);
        assert_eq!(buffer.data.len(), 360);

        let id = GroupElement::SU2 {
            q0: 1.0,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
        };
        for e in 0..90 {
            assert_eq!(buffer.read_element(e), id);
        }

        // Frozen-bytes check — strict f64 equality (this IS the
        // regression sentinel; no tolerances).
        for i in 0..90 {
            assert_eq!(buffer.data[4 * i], 1.0);
            for j in 1..4 {
                assert_eq!(buffer.data[4 * i + j], 0.0);
            }
        }
    }

    /// Non-implemented constructors return the typed error (not a
    /// panic) so the GAUGE_FIELD declaration path can surface it to
    /// the user. Bee's locked decision 5. Halcyon ITEM 3.1 lifts the
    /// SU(3) gate — only U(1) and Z(N) still error.
    #[test]
    fn tdd_hal_ii_1_identity_rejects_unimplemented_groups() {
        assert_eq!(
            DenseLinkBuffer::new_identity(Group::U1, 10).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::U1)
        );
        assert_eq!(
            DenseLinkBuffer::new_identity(Group::ZN { n: 5 }, 10).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::ZN { n: 5 })
        );
    }

    /// Halcyon ITEM 3.1: SU(3) identity buffer now constructs cleanly
    /// — gate lifted from "unsupported" to live math.
    #[test]
    fn su3_identity_buffer_constructs() {
        let buf = DenseLinkBuffer::new_identity(Group::SU3, 10).unwrap();
        assert_eq!(buf.group, Group::SU3);
        assert_eq!(buf.n_edges, 10);
        assert_eq!(buf.repr_dim, 18);
        assert_eq!(buf.data.len(), 180);
        // Identity layout check: re_00=1, re_11=1, re_22=1, all others 0.
        for edge in 0..10 {
            let base = 18 * edge;
            assert_eq!(buf.data[base], 1.0);
            assert_eq!(buf.data[base + 8], 1.0);
            assert_eq!(buf.data[base + 16], 1.0);
            for off in 0..18 {
                if off != 0 && off != 8 && off != 16 {
                    assert_eq!(buf.data[base + off], 0.0, "off={off}");
                }
            }
        }
    }

    /// Display impl must contain the group's stable label so the
    /// Halcyon G2.D `SU\(2\)` regex anchor can match.
    #[test]
    fn unsupported_group_display_contains_label() {
        let err = GaugeFieldError::UnsupportedGroup(Group::SU3);
        assert!(err.to_string().contains("SU(3)"));
    }

    /// Empty buffer is well-formed (n_edges = 0, data empty).
    #[test]
    fn identity_zero_edges_is_empty() {
        let buf = DenseLinkBuffer::new_identity(Group::SU2, 0).unwrap();
        assert_eq!(buf.data.len(), 0);
        assert_eq!(buf.n_edges, 0);
    }

    /// TDD-HAL-II.2: two `new_haar` builds with the same seed
    /// produce byte-identical buffers (strict f64 equality on
    /// `data`). This is the intra-binding bit-identity contract
    /// lifted up from the per-draw equality to the full buffer.
    #[test]
    fn tdd_hal_ii_2_dense_buffer_haar_reproducible() {
        let a = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616).unwrap();
        let b = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616).unwrap();
        assert_eq!(a.group, b.group);
        assert_eq!(a.n_edges, b.n_edges);
        assert_eq!(a.repr_dim, b.repr_dim);
        assert_eq!(a.data, b.data);
        // Shape check (90 edges × 4 floats).
        assert_eq!(a.data.len(), 360);
    }

    /// TDD-HAL-II.2: different seeds yield different buffers — guards
    /// against an accidental identity-seeding bug (e.g. seeding from
    /// `0` and the RNG state never advancing).
    #[test]
    fn tdd_hal_ii_2_dense_buffer_haar_different_seeds() {
        let a = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616).unwrap();
        let b = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260617).unwrap();
        assert_ne!(
            a.data, b.data,
            "different seeds must produce different Haar buffers"
        );
    }

    /// TDD-HAL-II.2: `new_haar` on a still-unimplemented group returns
    /// the typed error. Halcyon ITEM 3.1 lifts the SU(3) gate — only
    /// U(1) and Z(N) still error here.
    #[test]
    fn tdd_hal_ii_2_dense_buffer_haar_unsupported_group() {
        assert_eq!(
            DenseLinkBuffer::new_haar(Group::U1, 90, 0).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::U1)
        );
        assert_eq!(
            DenseLinkBuffer::new_haar(Group::ZN { n: 7 }, 90, 0).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::ZN { n: 7 })
        );
    }

    /// Halcyon ITEM 3.1: SU(3) Haar buffer constructs deterministically
    /// — same seed → byte-identical buffer (Mezzadri 2007).
    #[test]
    fn su3_haar_buffer_constructs_reproducibly() {
        let a = DenseLinkBuffer::new_haar(Group::SU3, 90, 20260626).unwrap();
        let b = DenseLinkBuffer::new_haar(Group::SU3, 90, 20260626).unwrap();
        assert_eq!(a.group, Group::SU3);
        assert_eq!(a.n_edges, 90);
        assert_eq!(a.repr_dim, 18);
        assert_eq!(a.data.len(), 90 * 18);
        assert_eq!(a.data, b.data);
    }

    /// TDD-HAL-II.2: every row of a Haar buffer is unit-norm in
    /// `(q0, q1, q2, q3)` to f64 rounding. Cross-check on the buffer
    /// surface (the marsaglia_haar tests already check the sampler
    /// directly; this guards the row-write path in `new_haar`).
    #[test]
    fn tdd_hal_ii_2_dense_buffer_haar_rows_unit_norm() {
        let buf = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616).unwrap();
        for edge in 0..90 {
            let b = 4 * edge;
            let n2 = buf.data[b] * buf.data[b]
                + buf.data[b + 1] * buf.data[b + 1]
                + buf.data[b + 2] * buf.data[b + 2]
                + buf.data[b + 3] * buf.data[b + 3];
            assert!(
                (n2 - 1.0).abs() < 1e-12,
                "edge {edge} not unit-norm: |q|^2 = {n2}"
            );
        }
    }
}
