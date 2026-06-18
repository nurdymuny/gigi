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

use super::group::Group;
use super::group_element::GroupElement;

/// Typed error for buffer construction.
///
/// Lifted from `unimplemented_for_group!` (panic) to a return value
/// because the GAUGE_FIELD declaration path needs to surface
/// "unsupported group" as a normal user error (Bee's locked decision 5,
/// Halcyon G2.D regex anchor). Inner math (`compose`, `inverse`) keeps
/// the Part-I panic — reaching it from a well-typed buffer would be a
/// programming error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GaugeFieldError {
    /// Group variant compiles but has no live math at launch.
    UnsupportedGroup(Group),
}

impl std::fmt::Display for GaugeFieldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GaugeFieldError::UnsupportedGroup(g) => write!(
                f,
                "gauge: group {} is not implemented (Part II ships SU(2) math only; \
                 future groups land as separate EdgeConnection impls per the group-erasure plan)",
                g.label()
            ),
        }
    }
}

impl std::error::Error for GaugeFieldError {}

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
    /// quaternion identity). For every other group: returns
    /// `Err(GaugeFieldError::UnsupportedGroup(_))`.
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
            other => Err(GaugeFieldError::UnsupportedGroup(other)),
        }
    }

    /// Decode the row at `edge` into a `GroupElement`.
    ///
    /// Only `Group::SU2` has live math at launch. Other arms panic
    /// because reaching them from a well-typed buffer is a
    /// programming error — the `new_*` constructors above return
    /// `Err` for non-SU2 groups, so no such buffer can be observed
    /// here. Bee's locked decision 6.
    pub fn read_element(&self, edge: usize) -> GroupElement {
        let base = self.repr_dim * edge;
        match self.group {
            Group::SU2 => GroupElement::SU2 {
                q0: self.data[base],
                q1: self.data[base + 1],
                q2: self.data[base + 2],
                q3: self.data[base + 3],
            },
            Group::U1 => panic!(
                "read_element not implemented for Group::U1 - Part II ships SU(2) math only; \
                 future groups ship as separate EdgeConnection impls per group-erasure plan"
            ),
            Group::SU3 => panic!(
                "read_element not implemented for Group::SU3 - Part II ships SU(2) math only; \
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

    /// Non-SU2 constructors return the typed error (not a panic) so
    /// the GAUGE_FIELD declaration path can surface it to the user.
    /// Bee's locked decision 5.
    #[test]
    fn tdd_hal_ii_1_identity_rejects_non_su2_groups() {
        assert_eq!(
            DenseLinkBuffer::new_identity(Group::SU3, 10).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::SU3)
        );
        assert_eq!(
            DenseLinkBuffer::new_identity(Group::U1, 10).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::U1)
        );
        assert_eq!(
            DenseLinkBuffer::new_identity(Group::ZN { n: 5 }, 10).unwrap_err(),
            GaugeFieldError::UnsupportedGroup(Group::ZN { n: 5 })
        );
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
}
