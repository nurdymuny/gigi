//! L7.1 — U(1) line bundle with integrality check
//! (catalog §2.1 + IMPLEMENTATION_PLAN.md L7.1).
//!
//! Per the Dirac quantization condition: a U(1) line bundle
//! `L → M` with connection curvature `B` exists globally iff
//! `[B / 2π] ∈ H²(M; ℤ)`. The integer Chern number `c_1(L)` =
//! `(1 / 2π) ∮ B` is the topological invariant; non-integral
//! values force a Dirac string and the bundle cannot be defined
//! globally.
//!
//! ### Wu-Yang ground truth
//!
//! On `S²` with `B = q · sin(θ) dθ ∧ dφ`:
//! - North chart potential: `A_N = q (1 − cos θ) dφ`
//! - South chart potential: `A_S = −q (1 + cos θ) dφ`
//! - On the equator (`θ = π/2`): `A_N = q dφ`, `A_S = −q dφ`
//! - Difference `A_N − A_S = 2q dφ` ⇒ loop integral `4πq`
//! - Bundle is global iff `4πq / 2π = 2q ∈ ℤ`
//!
//! So `q = 0.5, 1.0, 1.5, ...` are admissible (Chern numbers
//! 1, 2, 3, ...); `q = 0.3, 1/π, 0.7` produce a Dirac obstruction.
//!
//! `validation_tests_v2.py::test_7_prequantization_integrality`
//! is the math ground truth — our Rust API must classify the same
//! `q` values the same way.
//!
//! ### Marcella consumption
//!
//! `bundle.line_bundle_chern_class() -> Result<ChernClass,
//! IntegralityError>` lets Marcella check whether her attached
//! `B` is globally well-defined before running quantized
//! holonomy / DHOOM Chern compression. Returning the
//! `IntegralityError` with the measured non-integral winding
//! gives her a precise debug signal.

#![cfg(feature = "kahler")]

use crate::geometry::forms::ClosedTwoForm;

/// The topological Chern number `c_1(L) = (1 / 2π) ∮ B` of a
/// `U(1)` line bundle. Always an integer when the bundle is
/// globally defined (Dirac quantization).
///
/// Sign convention matches catalog §2.1: positive `c_1` ⇒
/// "magnetic monopole" charge in the Wu-Yang sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChernClass(pub i64);

impl ChernClass {
    /// Construct from an integer with no validation. Use sparingly;
    /// most callers should go through `LineBundle::chern_class`
    /// which derives the integer from a curvature 2-form via the
    /// integrality check.
    pub const fn new(c: i64) -> Self {
        Self(c)
    }

    /// True iff the line bundle is topologically trivial
    /// (`c_1 = 0` ⇒ no monopole charge).
    pub const fn is_trivial(self) -> bool {
        self.0 == 0
    }
}

/// A `U(1)` line bundle constructed from transition cocycle data.
///
/// Stored data:
/// - The integer Chern class.
/// - The integral curvature norm `‖B‖_∫` that produced it (for
///   provenance / drift detection between bundle reads).
#[derive(Debug, Clone, PartialEq)]
pub struct LineBundle {
    /// Topological Chern number.
    pub chern: ChernClass,
    /// The measured integral `(1 / 2π) ∮ B` that was rounded to
    /// `chern`. Equals `chern.0` to within the integrality
    /// tolerance.
    pub integral_value: f64,
}

impl LineBundle {
    /// Construct from the integral of the curvature 2-form over a
    /// closed surface. Returns `Err(IntegralityError)` when the
    /// supplied integral is not within `tolerance` of an integer
    /// multiple of `2π` — i.e., the Wu-Yang transition function
    /// fails to close on the equator and the bundle has a Dirac
    /// string.
    ///
    /// Per the catalog §2.1 normalization the input is
    /// `g_alpha_beta = ∮ B` (NOT divided by 2π) — we do the
    /// division here so the input shape matches what
    /// `holonomy_debt` accumulates around a loop.
    pub fn from_transition_data(
        g_alpha_beta: f64,
        tolerance: f64,
    ) -> Result<Self, IntegralityError> {
        let winding = g_alpha_beta / (2.0 * std::f64::consts::PI);
        let nearest = winding.round();
        let deviation = (winding - nearest).abs();
        if deviation > tolerance {
            return Err(IntegralityError::DiracString {
                winding,
                deviation,
                tolerance,
            });
        }
        Ok(Self {
            chern: ChernClass(nearest as i64),
            integral_value: winding,
        })
    }

    /// Convenience constructor: from a constant `ClosedTwoForm`
    /// `B = b · dx ∧ dy` integrated over a circle of radius `r`
    /// in the plane.
    ///
    /// For Wu-Yang `B = q sin(θ) dθ ∧ dφ` on `S²` the equivalent
    /// integral over a hemisphere is `2π·2q = 4πq`, so callers
    /// pass `g_alpha_beta = 4πq` directly via
    /// `from_transition_data`.
    pub fn from_constant_two_form(
        b: &ClosedTwoForm,
        loop_area: f64,
        tolerance: f64,
    ) -> Result<Self, IntegralityError> {
        // For 2D constant B = b·dx∧dy, the magnitude is the
        // (0,1) entry of the underlying TwoForm matrix.
        if b.dim() != 2 {
            return Err(IntegralityError::DimensionUnsupported { dim: b.dim() });
        }
        let row = b.form().matrix();
        // 2x2 antisymmetric: row[1] is the (0,1) component.
        let b_magnitude = row[1];
        let integral = b_magnitude * loop_area;
        Self::from_transition_data(integral, tolerance)
    }

    /// The topological Chern class of this bundle.
    pub fn chern_class(&self) -> ChernClass {
        self.chern
    }
}

/// Failure modes for `LineBundle` construction. The
/// `DiracString` variant carries the measured non-integral
/// winding so Marcella can see how far the bundle is from being
/// globally defined.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum IntegralityError {
    /// The transition data is not integral — Dirac string locus.
    /// `winding = ∮ B / 2π` was not close enough to an integer.
    #[error(
        "Dirac string: winding {winding} deviates from nearest integer by \
         {deviation} (tolerance {tolerance})"
    )]
    DiracString {
        winding: f64,
        deviation: f64,
        tolerance: f64,
    },
    /// The 2-form has a dimension the constructor cannot integrate
    /// without a domain mesh.
    #[error(
        "constant-form integration is only supported for dim = 2; got dim = {dim}"
    )]
    DimensionUnsupported { dim: usize },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::forms::TwoForm;

    /// Default integrality tolerance for the Wu-Yang test cases.
    const TOL: f64 = 1e-10;

    /// Positive — Wu-Yang integer charge: `2q ∈ ℤ` ⇒ bundle
    /// constructs globally. Matches `test_7_prequantization_integrality`.
    #[test]
    fn wu_yang_integer_charge_constructs_globally() {
        // Integer Chern cases from the Python ref:
        //   q ∈ {0.5, 1.0, 1.5, 2.0, 3.0} ⇒ Chern numbers {1, 2, 3, 4, 6}.
        // Wu-Yang loop integral = 4π·q (= A_N − A_S over equator).
        for (q, expected_chern) in [
            (0.5_f64, 1_i64),
            (1.0, 2),
            (1.5, 3),
            (2.0, 4),
            (3.0, 6),
        ] {
            let integral = 4.0 * std::f64::consts::PI * q;
            let lb = LineBundle::from_transition_data(integral, TOL)
                .expect("integer Chern: must construct");
            assert_eq!(
                lb.chern_class().0,
                expected_chern,
                "q = {} should give Chern = {}; got {}",
                q,
                expected_chern,
                lb.chern_class().0
            );
        }
    }

    /// Negative — Wu-Yang non-integer charge: returns
    /// `IntegralityError::DiracString` with the measured winding
    /// so Marcella can see the obstruction magnitude.
    #[test]
    fn wu_yang_non_integer_charge_returns_dirac_string() {
        for q in [0.3, 1.0 / 3.0, 1.0 / std::f64::consts::PI, 0.7] {
            let integral = 4.0 * std::f64::consts::PI * q;
            let err = LineBundle::from_transition_data(integral, TOL)
                .expect_err("non-integer Chern: must fail");
            match err {
                IntegralityError::DiracString {
                    winding,
                    deviation,
                    ..
                } => {
                    // 2q = 2 × 0.3 = 0.6 → winding = 0.6, deviation
                    // from nearest integer (1) = 0.4. All four cases
                    // are well-clear of TOL.
                    assert!(
                        deviation > TOL,
                        "deviation must exceed tolerance; got {}",
                        deviation
                    );
                    // Sanity: winding should be ≈ 2q.
                    assert!(
                        (winding - 2.0 * q).abs() < 1e-12,
                        "winding {} should equal 2q = {}",
                        winding,
                        2.0 * q
                    );
                }
                _ => panic!("expected DiracString error, got {:?}", err),
            }
        }
    }

    /// Positive — `from_constant_two_form` round-trips through the
    /// integral form. `B = 0.5 dx ∧ dy` integrated over loop area
    /// `4π` gives `2π` → Chern = 1.
    #[test]
    fn from_constant_two_form_constructs_integer_chern() {
        // The 2x2 antisymmetric matrix [[0, 0.5], [-0.5, 0]] has
        // b_magnitude = 0.5. Loop area 4π ⇒ integral = 2π ⇒ Chern 1.
        let tf = TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric");
        let cf = ClosedTwoForm::new_constant(tf);
        let lb = LineBundle::from_constant_two_form(&cf, 4.0 * std::f64::consts::PI, TOL)
            .expect("integer construction");
        assert_eq!(lb.chern_class().0, 1);
    }

    /// Negative — non-2D form is rejected.
    #[test]
    fn from_constant_two_form_rejects_non_2d() {
        let tf = TwoForm::new(
            vec![
                0.0, 1.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -1.0,
                0.0,
            ],
            4,
        )
        .expect("antisymmetric 4x4");
        let cf = ClosedTwoForm::new_constant(tf);
        let r = LineBundle::from_constant_two_form(&cf, 1.0, TOL);
        assert!(matches!(
            r,
            Err(IntegralityError::DimensionUnsupported { dim: 4 })
        ));
    }

    /// Positive — Chern class basic API.
    #[test]
    fn chern_class_api() {
        assert!(ChernClass::new(0).is_trivial());
        assert!(!ChernClass::new(1).is_trivial());
        assert_eq!(ChernClass::new(-3).0, -3);
    }

    /// Negative — zero integral with tolerance gives Chern = 0
    /// (trivial bundle, not an error).
    #[test]
    fn zero_integral_gives_trivial_chern() {
        let lb = LineBundle::from_transition_data(0.0, TOL).expect("trivial");
        assert!(lb.chern_class().is_trivial());
        assert_eq!(lb.chern_class().0, 0);
    }
}
