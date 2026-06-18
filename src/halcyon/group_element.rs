//! `GroupElement` — group-erased element of a structure group.
//!
//! Closes the group-erasure half of TDD-HAL-I.3. The enum carries
//! every group tag the spec admits — `SU(2)`, `U(1)`, `Z_N` — but
//! only the `SU2` arm has implemented math at launch. The `U1` and
//! `ZN` variants exist so the buffer layout and the trait are
//! group-erased at the type level (Q2 from the engine-owner reply
//! and `HALCYON_PART_I_GATES.md` Part II scope); their math is a
//! Part-II/V follow-up.
//!
//! Quaternion convention is the one pinned in
//! `tests/fixtures/halcyon/buckyball_gold_provenance.json` (scalar-
//! first `(q0, q1, q2, q3)` with `q0 = cos(θ/2)`); see the module
//! docstring on `halcyon::mod` for the full product and exponent
//! rules.

/// Group-erased element of a structure group. Only `SU2` has
/// implemented math at launch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupElement {
    /// SU(2) element in scalar-first quaternion form
    /// `(q0, q1, q2, q3)` with `q0 = cos(θ/2)`,
    /// `(q1, q2, q3) = sin(θ/2)·n_hat`. The constraint
    /// `q0² + q1² + q2² + q3² = 1` is the SU(2) determinant.
    SU2 { q0: f64, q1: f64, q2: f64, q3: f64 },
    /// U(1) element by angle. Compiles but every method panics
    /// with `unimplemented_for_group!("U1")` — Part-V wish per
    /// the gate spec.
    U1 { theta: f64 },
    /// Z_N element. Same panic-at-use contract as U1.
    ZN { k: u32, n: u32 },
}

impl GroupElement {
    /// SU(2) identity quaternion `(1, 0, 0, 0)`. Convenience because
    /// the identity literal appears in every test in this module.
    pub fn su2_identity() -> Self {
        GroupElement::SU2 {
            q0: 1.0,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
        }
    }

    /// Multiply two group elements. Both must be the same variant;
    /// mixed-group composition is a programming error and panics.
    ///
    /// Quaternion product (left-action, matches
    /// `davis-wilson-lattice/.../buckyball_action.py::face_holonomy`):
    ///
    /// ```text
    /// c0 = a0·b0 - a·b
    /// c_vec = a0·b_vec + b0·a_vec - a × b
    /// ```
    ///
    /// (Hamilton convention with the `-a × b` sign — `c = a*b`, not
    /// `b*a`.)
    pub fn compose(&self, other: &GroupElement) -> GroupElement {
        match (self, other) {
            (
                GroupElement::SU2 { q0: a0, q1: a1, q2: a2, q3: a3 },
                GroupElement::SU2 { q0: b0, q1: b1, q2: b2, q3: b3 },
            ) => {
                let c0 = a0 * b0 - (a1 * b1 + a2 * b2 + a3 * b3);
                // a × b
                let cx = a2 * b3 - a3 * b2;
                let cy = a3 * b1 - a1 * b3;
                let cz = a1 * b2 - a2 * b1;
                let c1 = a0 * b1 + b0 * a1 - cx;
                let c2 = a0 * b2 + b0 * a2 - cy;
                let c3 = a0 * b3 + b0 * a3 - cz;
                GroupElement::SU2 {
                    q0: c0,
                    q1: c1,
                    q2: c2,
                    q3: c3,
                }
            }
            (GroupElement::U1 { .. }, GroupElement::U1 { .. }) => {
                unimplemented_for_group("U1")
            }
            (GroupElement::ZN { .. }, GroupElement::ZN { .. }) => {
                unimplemented_for_group("ZN")
            }
            _ => panic!(
                "GroupElement::compose: cannot compose elements of different group variants"
            ),
        }
    }

    /// Group inverse. For SU(2) quaternions: conjugate
    /// `(q0, -q1, -q2, -q3)` (the determinant constraint
    /// `q0² + …² = 1` makes the conjugate the inverse).
    pub fn inverse(&self) -> GroupElement {
        match self {
            GroupElement::SU2 { q0, q1, q2, q3 } => GroupElement::SU2 {
                q0: *q0,
                q1: -*q1,
                q2: -*q2,
                q3: -*q3,
            },
            GroupElement::U1 { .. } => unimplemented_for_group("U1"),
            GroupElement::ZN { .. } => unimplemented_for_group("ZN"),
        }
    }

    /// Real part of the trace, normalized to the `[-1, 1]` plaquette
    /// range. For SU(2): `Re tr(U) / 2 = q0`. This is the per-face
    /// plaquette value Halcyon's reference implementation publishes
    /// in `inertia_damping/buckyball_observables.py`.
    pub fn re_trace_half(&self) -> f64 {
        match self {
            GroupElement::SU2 { q0, .. } => *q0,
            GroupElement::U1 { .. } => unimplemented_for_group("U1"),
            GroupElement::ZN { .. } => unimplemented_for_group("ZN"),
        }
    }
}

#[cold]
fn unimplemented_for_group(group: &'static str) -> ! {
    panic!(
        "halcyon::group_element: math for group {group} is not implemented (Part I only ships SU(2); see HALCYON_PART_I_GATES.md Part II scope)"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn su2_identity_compose_is_identity() {
        let i = GroupElement::su2_identity();
        let r = i.compose(&i);
        match r {
            GroupElement::SU2 { q0, q1, q2, q3 } => {
                assert_eq!(q0, 1.0);
                assert_eq!(q1, 0.0);
                assert_eq!(q2, 0.0);
                assert_eq!(q3, 0.0);
            }
            _ => panic!("expected SU2"),
        }
    }

    #[test]
    fn su2_inverse_of_identity_is_identity() {
        let i = GroupElement::su2_identity();
        assert_eq!(i.inverse(), i);
    }

    #[test]
    fn su2_compose_with_inverse_is_identity() {
        // Some non-trivial element: rotation by θ=π/3 about z-axis.
        let q0 = (std::f64::consts::PI / 6.0).cos(); // cos(θ/2) with θ=π/3
        let q3 = (std::f64::consts::PI / 6.0).sin();
        let g = GroupElement::SU2 { q0, q1: 0.0, q2: 0.0, q3 };
        let g_inv = g.inverse();
        let p = g.compose(&g_inv);
        match p {
            GroupElement::SU2 { q0, q1, q2, q3 } => {
                assert!((q0 - 1.0).abs() < 1e-14);
                assert!(q1.abs() < 1e-14);
                assert!(q2.abs() < 1e-14);
                assert!(q3.abs() < 1e-14);
            }
            _ => panic!("expected SU2"),
        }
    }

    #[test]
    #[should_panic(expected = "U1")]
    fn u1_compose_panics() {
        let a = GroupElement::U1 { theta: 0.1 };
        let b = GroupElement::U1 { theta: 0.2 };
        let _ = a.compose(&b);
    }

    #[test]
    #[should_panic(expected = "ZN")]
    fn zn_inverse_panics() {
        let g = GroupElement::ZN { k: 1, n: 4 };
        let _ = g.inverse();
    }
}
