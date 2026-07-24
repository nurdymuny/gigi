//! PK-1 / Sasaki–contact geometry — the standard contact structure on ℝ³.
//!
//! The standard contact 1-form on ℝ³ is `α = dz − y·dx`. Its exterior
//! derivative `dα = dx ∧ dy` is a constant 2-form, and the pair
//! `(α, dα)` makes ℝ³ a contact manifold: `α ∧ dα` is a nowhere-zero
//! volume form (the non-degeneracy / contact condition).
//!
//! The **Reeb vector field** `R` is defined uniquely by the two
//! conditions
//!
//! > (i)  `α(R) = 1`
//! > (ii) `ι_R dα = 0`  (equivalently `dα(R, X) = 0` for every `X`).
//!
//! On this standard structure the unique solution is `R = ∂_z`. The
//! Reeb flow preserves `α` (`𝓛_R α = 0`), so `R` is the invariant
//! positional flow along a sequence — the geometric primitive behind
//! `SECTION … ALONG REEB FLOW`.
//!
//! Ground truth: `theory/post_kahler_directions/validation_tests.py
//! ::test_1_sasaki_contact_reeb_flow`. The unit tests below replicate
//! every assertion in that function to machine epsilon.

#![cfg(feature = "post_kahler_phase1")]

use std::fmt;

/// Error surfaced when a candidate field fails the Reeb defining
/// conditions or a form is evaluated on malformed input.
#[derive(Debug, Clone, PartialEq)]
pub enum ContactError {
    /// A candidate Reeb field violates `α(R) = 1` or `ι_R dα = 0`.
    NotReeb,
    /// The contact condition `α ∧ dα ≠ 0` fails at the tested point.
    Degenerate,
}

impl fmt::Display for ContactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContactError::NotReeb => write!(
                f,
                "vector field is not the Reeb field (needs α(R)=1 and ι_R dα=0)"
            ),
            ContactError::Degenerate => write!(
                f,
                "contact condition α ∧ dα = 0 (form is degenerate at this point)"
            ),
        }
    }
}

impl std::error::Error for ContactError {}

/// The standard contact 1-form `α = dz − y·dx` on ℝ³.
///
/// Points and tangent vectors are `[x, y, z]` / `[vx, vy, vz]`.
pub struct ContactOneForm;

impl ContactOneForm {
    /// `α` evaluated on a tangent vector at a base point:
    /// `α(p)(v) = v_z − y·v_x`.
    pub fn eval(point: [f64; 3], tangent: [f64; 3]) -> f64 {
        let y = point[1];
        tangent[2] - y * tangent[0]
    }

    /// The exterior derivative `dα = dx ∧ dy`, a constant 2-form
    /// (independent of the base point): `dα(u, v) = u_x·v_y − u_y·v_x`.
    pub fn d_alpha(u: [f64; 3], v: [f64; 3]) -> f64 {
        u[0] * v[1] - u[1] * v[0]
    }

    /// The Reeb vector field `R = ∂_z` of this contact form.
    pub fn reeb() -> ReebField {
        ReebField {
            vector: [0.0, 0.0, 1.0],
        }
    }

    /// The contact volume `(α ∧ dα)(v0, v1, v2)` at a base point, using
    /// the standard wedge expansion
    /// `α(v0)·dα(v1,v2) − α(v1)·dα(v0,v2) + α(v2)·dα(v0,v1)`.
    /// Non-zero on any positively-oriented frame ⇒ `α` is contact.
    pub fn contact_volume(point: [f64; 3], v0: [f64; 3], v1: [f64; 3], v2: [f64; 3]) -> f64 {
        Self::eval(point, v0) * Self::d_alpha(v1, v2)
            - Self::eval(point, v1) * Self::d_alpha(v0, v2)
            + Self::eval(point, v2) * Self::d_alpha(v0, v1)
    }

    /// Verify the contact (non-degeneracy) condition at `point` on the
    /// canonical frame `(R, ∂_x, ∂_y)`: `α ∧ dα ≠ 0`.
    pub fn is_contact_at(point: [f64; 3]) -> bool {
        let vol = Self::contact_volume(
            point,
            Self::reeb().vector,
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        );
        vol.abs() > 1e-12
    }
}

/// A candidate Reeb vector field on the standard contact ℝ³.
pub struct ReebField {
    /// The field's (constant) components `[Rx, Ry, Rz]`.
    pub vector: [f64; 3],
}

impl ReebField {
    /// Maximum deviation of `α(R)` from 1 over the supplied base points.
    /// The Reeb field satisfies `α(R) ≡ 1` everywhere; a field whose
    /// `α(R)` varies with position is not Reeb.
    pub fn alpha_defect(&self, points: &[[f64; 3]]) -> f64 {
        points
            .iter()
            .map(|&p| (ContactOneForm::eval(p, self.vector) - 1.0).abs())
            .fold(0.0, f64::max)
    }

    /// Maximum `|dα(R, X)|` over the supplied probe vectors — the
    /// interior-product condition `ι_R dα = 0`.
    pub fn dalpha_defect(&self, probes: &[[f64; 3]]) -> f64 {
        probes
            .iter()
            .map(|&x| ContactOneForm::d_alpha(self.vector, x).abs())
            .fold(0.0, f64::max)
    }

    /// True iff both defining Reeb conditions hold to `tol` over the
    /// supplied points/probes.
    pub fn is_reeb(&self, points: &[[f64; 3]], probes: &[[f64; 3]], tol: f64) -> bool {
        self.alpha_defect(points) < tol && self.dalpha_defect(probes) < tol
    }
}

// ── tests ─────────────────────────────────────────────────────────
// Mirror theory/post_kahler_directions/validation_tests.py::
// test_1_sasaki_contact_reeb_flow, assertion for assertion.

#[cfg(test)]
mod tests {
    use super::*;

    fn test_points() -> [[f64; 3]; 3] {
        [
            [1.0, 2.0, 0.0],
            [-3.0, 0.5, 7.0],
            [0.0, -1.4, -2.1],
        ]
    }

    fn test_vectors() -> [[f64; 3]; 3] {
        [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [3.0, -2.0, 5.0],
        ]
    }

    #[test]
    fn reeb_condition_i_alpha_of_r_is_one() {
        // (i) α(R) ≡ 1 to machine epsilon at every base point.
        let r = ContactOneForm::reeb();
        assert!(
            r.alpha_defect(&test_points()) < 1e-12,
            "α(R) must be identically 1"
        );
    }

    #[test]
    fn reeb_condition_ii_iota_r_dalpha_is_zero() {
        // (ii) ι_R dα ≡ 0 against arbitrary probe vectors.
        let r = ContactOneForm::reeb();
        assert!(
            r.dalpha_defect(&test_vectors()) < 1e-12,
            "ι_R dα must vanish"
        );
    }

    #[test]
    fn contact_condition_volume_is_unit() {
        // (α ∧ dα)(R, ∂x, ∂y) = 1 at (0.5, 0, 0.7) — a genuine volume form.
        let vol = ContactOneForm::contact_volume(
            [0.5, 0.0, 0.7],
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        );
        assert!((vol - 1.0).abs() < 1e-12, "contact volume should be 1, got {vol}");
        assert!(ContactOneForm::is_contact_at([0.5, 0.0, 0.7]));
    }

    #[test]
    fn negative_control_dx_is_not_reeb() {
        // X = ∂_x fails α(X) ≡ 1: α(X) = −y varies with the base point.
        let x = ReebField { vector: [1.0, 0.0, 0.0] };
        assert!(
            x.alpha_defect(&test_points()) > 0.5,
            "∂_x must fail the Reeb α(R)=1 condition"
        );
        assert!(!x.is_reeb(&test_points(), &test_vectors(), 1e-9));
    }

    #[test]
    fn reeb_preserves_alpha_flow_invariant() {
        // The Reeb flow preserves α: translating a point along R = ∂_z
        // leaves α(R) unchanged (=1). Sample along the flow.
        let r = ContactOneForm::reeb();
        for t in [-5.0, 0.0, 3.3, 100.0] {
            let flowed = [0.4, 1.9, 0.1 + t]; // moved along ∂_z
            assert!((ContactOneForm::eval(flowed, r.vector) - 1.0).abs() < 1e-12);
        }
    }
}
