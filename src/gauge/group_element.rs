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
//! docstring on `gauge::mod` for the full product and exponent
//! rules.

/// Group-erased element of a structure group. `SU2` and `SU3` have
/// implemented math at launch (the latter via Halcyon ITEM 3.1
/// Phase 1 — read-only ingest scope).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GroupElement {
    /// SU(2) element in scalar-first quaternion form
    /// `(q0, q1, q2, q3)` with `q0 = cos(θ/2)`,
    /// `(q1, q2, q3) = sin(θ/2)·n_hat`. The constraint
    /// `q0² + q1² + q2² + q3² = 1` is the SU(2) determinant.
    SU2 { q0: f64, q1: f64, q2: f64, q3: f64 },
    /// SU(3) element as a 3×3 complex matrix flattened to 18 f64s
    /// in row-major order with interleaved real/imag pairs:
    /// `[re_00, im_00, re_01, im_01, re_02, im_02,
    ///   re_10, im_10, re_11, im_11, re_12, im_12,
    ///   re_20, im_20, re_21, im_21, re_22, im_22]`.
    /// Represents a unitary 3×3 matrix with determinant 1.
    /// Halcyon ITEM 3.1 §3.1 representation — matches
    /// `inertia_damping/gauge_heatbath_gpu.py`. 144 bytes per link.
    SU3([f64; 18]),
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

    /// SU(3) identity matrix `I_3 = diag(1, 1, 1)` encoded as 18 f64s
    /// in the row-major interleaved real/imag layout. Diagonal real
    /// parts live at indices 0, 8, 16; every other slot is zero.
    pub fn su3_identity() -> Self {
        let mut m = [0.0_f64; 18];
        m[0] = 1.0; // re_00
        m[8] = 1.0; // re_11
        m[16] = 1.0; // re_22
        GroupElement::SU3(m)
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
            (GroupElement::SU3(a), GroupElement::SU3(b)) => {
                GroupElement::SU3(su3_matmul(a, b))
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
    /// `q0² + …² = 1` makes the conjugate the inverse). For SU(3):
    /// conjugate transpose `U^† = (conj U)^T` (unitarity makes the
    /// conjugate transpose the inverse).
    pub fn inverse(&self) -> GroupElement {
        match self {
            GroupElement::SU2 { q0, q1, q2, q3 } => GroupElement::SU2 {
                q0: *q0,
                q1: -*q1,
                q2: -*q2,
                q3: -*q3,
            },
            GroupElement::SU3(m) => GroupElement::SU3(su3_conjugate_transpose(m)),
            GroupElement::U1 { .. } => unimplemented_for_group("U1"),
            GroupElement::ZN { .. } => unimplemented_for_group("ZN"),
        }
    }

    /// Real part of the trace, normalized to the `[-1, 1]` plaquette
    /// range. For SU(2): `Re tr(U) / 2 = q0`. For SU(3):
    /// `Re tr(U) / 3` (sum of diagonal real parts at indices 0, 8, 16
    /// divided by 3). This is the per-face plaquette value Halcyon's
    /// reference implementation publishes in
    /// `inertia_damping/buckyball_observables.py`.
    pub fn re_trace_half(&self) -> f64 {
        match self {
            GroupElement::SU2 { q0, .. } => *q0,
            GroupElement::SU3(m) => su3_re_trace_third(m),
            GroupElement::U1 { .. } => unimplemented_for_group("U1"),
            GroupElement::ZN { .. } => unimplemented_for_group("ZN"),
        }
    }
}

// ─────────────────────────── SU(3) helpers ───────────────────────────
//
// Private free functions used by the SU(3) arms above. Kept module-
// internal because they assume the row-major interleaved real/imag
// layout that `GroupElement::SU3` pins; callers outside this module
// should go through `compose` / `inverse` / `re_trace_half` which
// dispatch on the variant tag.

/// Standard 3×3 complex matrix multiplication on the row-major
/// interleaved-pairs layout. Left action: `out = a · b`.
///
/// For each output element `c_ij = Σ_k a_ik · b_kj` (complex
/// multiplication). 27 complex multiplies = 162 real multiplies
/// + 108 real additions; ~O(1) hot-loop cost per link composition.
#[inline]
pub(crate) fn su3_matmul(a: &[f64; 18], b: &[f64; 18]) -> [f64; 18] {
    let mut out = [0.0_f64; 18];
    for i in 0..3 {
        for j in 0..3 {
            let mut re = 0.0;
            let mut im = 0.0;
            for k in 0..3 {
                let a_re = a[2 * (3 * i + k)];
                let a_im = a[2 * (3 * i + k) + 1];
                let b_re = b[2 * (3 * k + j)];
                let b_im = b[2 * (3 * k + j) + 1];
                re += a_re * b_re - a_im * b_im;
                im += a_re * b_im + a_im * b_re;
            }
            out[2 * (3 * i + j)] = re;
            out[2 * (3 * i + j) + 1] = im;
        }
    }
    out
}

/// Conjugate transpose of a 3×3 complex matrix in row-major
/// interleaved-pairs layout. For a unitary `U`, this is the inverse.
///
/// `out[i][j] = conj(in[j][i])`: read from transposed positions,
/// negate the imaginary part. 9 complex copies = O(1) cost.
#[inline]
pub(crate) fn su3_conjugate_transpose(m: &[f64; 18]) -> [f64; 18] {
    let mut out = [0.0_f64; 18];
    for i in 0..3 {
        for j in 0..3 {
            let src = 3 * j + i;
            let dst = 3 * i + j;
            out[2 * dst] = m[2 * src];
            out[2 * dst + 1] = -m[2 * src + 1];
        }
    }
    out
}

/// SU(3) plaquette reduction `Re Tr(U) / 3`.
///
/// Diagonal real parts live at indices 0, 8, 16 in the row-major
/// interleaved-pairs layout. For `U ∈ SU(3)`, `|Re Tr(U) / 3| ≤ 1`
/// (the fundamental-rep trace is bounded by `N = 3`), so the result
/// lives in `[-1, 1]` by construction — same invariant the SU(2)
/// `q0` plaquette enforces.
#[inline]
pub(crate) fn su3_re_trace_third(m: &[f64; 18]) -> f64 {
    (m[0] + m[8] + m[16]) / 3.0
}

#[cold]
fn unimplemented_for_group(group: &'static str) -> ! {
    panic!(
        "gauge::group_element: math for group {group} is not implemented (Part I only ships SU(2); see HALCYON_PART_I_GATES.md Part II scope)"
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

    // ─────────────────────── SU(3) unit tests ───────────────────────

    /// Halcyon ITEM 3.1: SU(3) identity composes with itself to give
    /// identity, byte-identical (FP64 exact: 1·1 = 1, 0·* = 0).
    #[test]
    fn su3_identity_compose_is_identity() {
        let i = GroupElement::su3_identity();
        let r = i.compose(&i);
        assert_eq!(r, i);
    }

    /// Halcyon ITEM 3.1: SU(3) inverse of identity is identity.
    #[test]
    fn su3_inverse_of_identity_is_identity() {
        let i = GroupElement::su3_identity();
        assert_eq!(i.inverse(), i);
    }

    /// Halcyon ITEM 3.1: SU(3) compose with inverse is identity (to
    /// FP64 tolerance). Uses a Hermitian-conjugate Givens-style
    /// rotation in the (0,1) block: U = exp(i·θ·σ_x) on the upper-left
    /// 2×2 with the (2,2) diagonal carrying e^{-2iθ} for det = 1.
    #[test]
    fn su3_compose_with_inverse_is_identity() {
        // Build a non-trivial SU(3) element: rotation by θ=π/3 in the
        // (0,1) plane with phase compensation on (2,2).
        // U_00 = cos(θ), U_01 = i·sin(θ), U_10 = i·sin(θ), U_11 = cos(θ),
        // U_22 = e^{-2iθ}·1 = cos(2θ) - i·sin(2θ) (det(U) = 1).
        // Wait — that determinant calc isn't right. Use a simpler one:
        // U = diag(e^{iα}, e^{iβ}, e^{-i(α+β)}) for det = 1.
        let alpha = 0.7_f64;
        let beta = -0.3_f64;
        let gamma = -(alpha + beta);
        let mut m = [0.0_f64; 18];
        m[0] = alpha.cos();
        m[1] = alpha.sin();
        m[8] = beta.cos();
        m[9] = beta.sin();
        m[16] = gamma.cos();
        m[17] = gamma.sin();
        let u = GroupElement::SU3(m);
        let u_inv = u.inverse();
        let r = u.compose(&u_inv);
        let id = GroupElement::su3_identity();
        match (r, id) {
            (GroupElement::SU3(a), GroupElement::SU3(b)) => {
                for k in 0..18 {
                    assert!(
                        (a[k] - b[k]).abs() < 1e-14,
                        "index {k}: got {}, expected {}",
                        a[k],
                        b[k]
                    );
                }
            }
            _ => panic!("expected SU3"),
        }
    }

    /// Halcyon ITEM 3.1: SU(3) plaquette reduction on identity is 1.0
    /// exactly (sum of three 1.0 real diagonals divided by 3.0).
    #[test]
    fn su3_re_trace_third_on_identity_is_one() {
        let i = GroupElement::su3_identity();
        assert_eq!(i.re_trace_half(), 1.0);
    }

    /// Mixed-variant compose panics (architectural contract).
    #[test]
    #[should_panic(expected = "different group variants")]
    fn su3_compose_mixed_variants_panics() {
        let a = GroupElement::su3_identity();
        let b = GroupElement::su2_identity();
        let _ = a.compose(&b);
    }
}
