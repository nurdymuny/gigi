# Residual integration diagnosis and minimal correction

24 November 2026. Registered before the new operator expansion, tests and control measurements.
The model, saved vectors, energy centers, complex mixing, cutoff and all existing thresholds stay fixed.
Only the four existing controls at s=1.15, t=3, g^2=0.004/0.002 and Mq=6/8 are allowed. Full grid held.

## Evidence already available

In the saved e21087a1 receipt, every one of the twelve failed state comparisons fails only residual_sq
and its identical cutoff_residual_sq. Norm, charge, Rayleigh and other comparisons pass. The sampled
cutoff terms vanish, which is not a rigorous tail bound. All 160 pointwise differential-action checks pass.

An independent manufactured example exposes a limitation of the rule: normalized Hermite h_10 has
Gaussian squared norm 1, but evaluation at the roots of H_10 gives zero in exact arithmetic (2.7e-31 in
the current float calculation). Gauss-Hermite order n is exact for polynomial degrees through 2n-1,
not arbitrary residuals. The original trial norm has lower degree than the full residual squared.
The physical integrand also contains exponentials, to which polynomial exactness does not apply.

Hypothesis to test, not an assumed conclusion: the twelve disagreements arise from underintegration
of the same correct residual, not a changed model, state selection, cutoff term or differential operator.

## Finite algebraic representation

Retain psi=f P, with f the original Gaussian divided by sqrt(nu). In log coordinates u=u0+Sz, each
Laurent term in the unchanged operator is exp(alpha.z) times a polynomial. Differentiation and
multiplication preserve finite sums of this form. Build

    (H-E)psi / f = sum_a exp(alpha_a.z) Q_a(z).

Use normalized Hermite coefficients for each Q, not monomials with poorly scaled coefficients.
Physical differentiation is D_i=sum_k invS[k,i]*(partial_k-w_k*z_k)-measure_power[i]/2.
Retain BOTH the metric second derivatives and its full density-divergence first derivatives.
Use the original Laurent metric and potential as inputs. Do not square the compressed Ritz matrix.
An Mq+6 total-degree workspace suffices: two derivatives plus at most four off-diagonal factors.
Attempted nonzero coefficient truncation must refuse; no small coefficients may be dropped by tolerance.

For beta=alpha_a+alpha_b and mu_k=beta_k/(2*w_k), completing the square gives

    <exp(alpha_a.z)Q_a, exp(alpha_b.z)Q_b>_Gaussian
      = exp(sum_k beta_k^2/(4*w_k)) * <Q_a(z+mu), Q_b(z+mu)>_Gaussian.

Translate each finite Hermite polynomial by its finite Taylor series. Orthogonality reduces the final
inner product to a conjugated coefficient dot product. Keep diagonal and twice-real cross terms;
use compensated scalar summation of pair contributions. Report cancellation scale separately.
The formula is exact in exact arithmetic. Ordinary floating-point evaluation is NOT an interval enclosure.

This is the smallest proposed correction to the uncut residual integration: use finite completed-square
moments rather than raising an empirical quadrature order until a desired answer appears. The original
10/12 outputs remain preserved and are replayed for diagnosis, never overwritten.

## Gates before the same controls

1. Behavioral RED: high-degree Hermite alias fixture; exact translation coefficients; exponential norm;
   complex cross terms; independent high-order Gaussian oracle; physical differential action; no degree
   truncation; nonfinite/invalid input refusals; inability to turn numerical moments into a certificate.
2. GREEN with manufactured inputs. Compare the expanded action to the existing reduced and Cartesian
   evaluators, including points away from the origin. The existing 1e-9*(1+abs(value)) rule is unchanged.
3. Assertion-caught removal tests: translation, completing-square prefactor, conjugation, cross terms,
   density drift, derivative raising term, truncation refusal, and false numerical certification.
4. Commit the source and registration before physical replay. Read only the existing four NPZ states;
   check receipt/source/state hashes, centers, mixing and exact coverage. No new eigensolve or refitting.
5. On each saved state, reconstruct both old 10/12 tensor residual integrals with the independent
   exponential-polynomial representation and compare to the original numbers at the unchanged
   1e-9*(1+abs(reference)) rule. Compare the algebraic integral to both, recording signed errors.
6. Independently check the translated-polynomial inner product using degree-sufficient one-dimensional
   quadrature translation matrices, instead of Taylor translation coefficients. Compare the two full
   algebraic moment results at that same rule. No post-result order adjustment.

The independent translation matrix uses n=D+1 nodes for workspace degree D: products of shifted and
unshifted polynomials have degree at most 2D. The exponential has already been removed by completing
the square. This is polynomial exactness, not a claim about the original unshifted rule.

Budget: 900 seconds per control, 12 GiB estimated live arrays, no full-grid invocation. Log stages as
they finish. A mismatch, budget exhaustion, nonfinite or negative squared norm is a refusal, not a
zero replacement. Preserve original integration_status behavior: UNRESOLVED_INTEGRATION, certified=false.

## Scope of success

Matching operator actions and replayed old quadratures, together with agreement of two algebraic
integration paths, would identify quadrature underresolution for these twelve discrepancies and supply
corrected numerical uncut residuals. It would not certify cutoff leakage, projected norms, the physical
gap, the exterior tails, or spectral ordering. The validated-integration and separation gates stay open.
Do not substitute these uncut moments for cutoff integrals: the smooth cutoff is not a finite
exponential-polynomial. The compact-support/tail issue in the preceding handoff remains explicit.

Reference: [SciPy Gauss-Hermite contract](https://docs.scipy.org/doc/scipy/reference/generated/scipy.special.roots_hermite.html).
