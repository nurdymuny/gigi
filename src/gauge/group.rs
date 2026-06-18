//! `Group` — group-erased structure-group tag.
//!
//! Opens TDD-HAL-II.1. While `GroupElement` (Part I) is the per-edge
//! payload, `Group` is the per-buffer tag: it says which variant of
//! `GroupElement` every row of a `DenseLinkBuffer` decodes into. Group-
//! erased storage is the contract that lets a future U(1) / SU(3) /
//! Z(N) `EdgeConnection` impl ship without touching the SU(2) walker,
//! the parser, the registry, or the HTTP routes (see
//! `HALCYON_PART_I_GATES.md` Part II group-erasure note and Bee's
//! locked decision 6 for the launch surface).
//!
//! All four variants compile at launch; only `SU2` has live math.
//! Constructors that would materialize non-SU2 buffers return a typed
//! `GroupNotImplemented` error rather than panicking — the GAUGE_FIELD
//! declaration path lifts the error to the user (Bee's locked decision
//! 5), while inner math (`compose`, `inverse`) keeps the Part-I panic
//! because reaching it from a well-typed buffer is a programming error.

/// Group-erased structure-group tag carried by every
/// `DenseLinkBuffer`. Determines `repr_dim` and which arm of
/// `read_element` decodes a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    /// SU(2): 4 f64s per edge in scalar-first quaternion layout
    /// `(q0, q1, q2, q3)` with the determinant constraint
    /// `q0² + q1² + q2² + q3² = 1`. The only variant with implemented
    /// math at launch.
    SU2,
    /// SU(3): 9 f64s per edge (3×3 complex matrix flattened — exact
    /// real layout is fixed in a later gate). Compiles; constructors
    /// return `GroupNotImplemented`.
    SU3,
    /// U(1): 1 f64 per edge (angle θ). Compiles; constructors return
    /// `GroupNotImplemented`.
    U1,
    /// Z_N: 1 f64 per edge (discrete index packed as f64 to keep the
    /// buffer monomorphic). Compiles; constructors return
    /// `GroupNotImplemented`. `n` is the modulus.
    ZN { n: u32 },
}

impl Group {
    /// Floats per edge in the dense buffer for this group.
    ///
    /// - `SU2 = 4` (quaternion)
    /// - `SU3 = 9` (3×3 flattened)
    /// - `U1 = 1` (angle)
    /// - `ZN = 1` (discrete index as f64)
    pub fn repr_dim(self) -> usize {
        match self {
            Group::SU2 => 4,
            Group::SU3 => 9,
            Group::U1 => 1,
            Group::ZN { .. } => 1,
        }
    }

    /// Short label used in JSON envelopes, error messages, and the
    /// `GAUGE_FIELD` declaration surface. Matches Halcyon's mock
    /// expectation (the regex anchor `SU\(2\)` from the G2.D check
    /// per Bee's locked decision 5).
    pub fn label(self) -> &'static str {
        match self {
            Group::SU2 => "SU(2)",
            Group::SU3 => "SU(3)",
            Group::U1 => "U(1)",
            Group::ZN { .. } => "Z(N)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-HAL-II.1: `repr_dim` is the per-edge float count for every
    /// group variant (SU(2)=4, SU(3)=9, U(1)=1, Z(N)=1).
    #[test]
    fn tdd_hal_ii_1_group_repr_dim() {
        assert_eq!(Group::SU2.repr_dim(), 4);
        assert_eq!(Group::SU3.repr_dim(), 9);
        assert_eq!(Group::U1.repr_dim(), 1);
        assert_eq!(Group::ZN { n: 5 }.repr_dim(), 1);
    }

    /// Labels are stable string constants — Halcyon's `SU\(2\)` regex
    /// anchor and the JSON `"group"` field both rely on them.
    #[test]
    fn group_labels_are_stable() {
        assert_eq!(Group::SU2.label(), "SU(2)");
        assert_eq!(Group::SU3.label(), "SU(3)");
        assert_eq!(Group::U1.label(), "U(1)");
        assert_eq!(Group::ZN { n: 7 }.label(), "Z(N)");
    }

    /// `ZN` discriminates on `n` (different moduli are different
    /// groups under `Eq`).
    #[test]
    fn zn_eq_discriminates_on_modulus() {
        assert_ne!(Group::ZN { n: 3 }, Group::ZN { n: 5 });
        assert_eq!(Group::ZN { n: 4 }, Group::ZN { n: 4 });
    }
}
