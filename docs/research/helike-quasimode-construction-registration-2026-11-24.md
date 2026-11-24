# Quasimode construction: frozen cutoff, exact projector, integration contract

24 November 2026. REGISTERED BEFORE IMPLEMENTATION AND CONTROL MEASUREMENT.
This adds a certificate construction/audit; it changes neither the Hamiltonian nor any existing threshold.
The full grid and the LocalFock release remain held. Same controls: s=1.15, t=3, g^2=0.004,0.002; Mq=6,8.

## 1. Cutoff and physical functions

Retain each computed ConeRitz coefficient vector and its original physical function psi=chi/sqrt(nu).
The coefficients, chart and energy center are saved, not inferred later from a label. They define a trial
function; being computed by an eigensolver does not make it an exact eigenfunction.

Let D=det(G), D0=det(G0), N=||G||_F, N0=||G0||_F. Fix delta=0.01, **R=4 N0** in the same q-squared units
as G. Fix eta=h(D/(delta D0)) [1-h(N/R)], with the already specified smooth h:
a(v)=exp(-1/v) for v>0, zero otherwise; h(x)=a(x-1)/(a(x-1)+a(2-x)).
No cutoff fitting or post-result adjustment. The function is zero on the negative sheet and a neighborhood
of rank loss, and outside N<2R. Its extension is a smooth compactly supported Cartesian singlet.

Both transition shells are evaluated explicitly. The exact Cartesian identities used by the reduced evaluator are

    |grad D|^2 = 4 D ||cof(q)||_F^2,       Delta D = 2 ||cof(q)||_F^2,
    |grad N|^2 = 4 tr(G^3)/N^2,           Delta N = 14 tr(G)/N - 4 tr(G^3)/N^3,
    grad D . grad N = 4 D tr(G)/N.

These fix the mixed shell term in Delta eta; tests compare with independent nine-coordinate differentiation.
The full cutoff action is H(eta psi)=eta Hpsi - grad eta.B.grad psi - (Delta eta)psi/2.

## 2. Exact charge projector without pretending a finite angular sum is exact

Use R_theta=diag(1, [[cos(theta),-sin(theta)],[sin(theta),cos(theta)]]) and G -> R_theta G R_theta^T.
L=-i d/dtheta at zero; v+=G01+iG02 has charge +1 and w+=(G11-G22)/2+iG12 has charge +2.
The target projector is the **continuous** SO(2) integral, not an N-point replacement.

There is a direct bound that avoids angular aliasing. For an integer charge m, on this single-valued SO(2)
representation, distinct generator eigenvalues differ by at least one:

    ||(I-P_m) f||^2 <= ||(L-m) f||^2.

Since eta is invariant, it commutes with L and P_m. For normalized uncut psi, define genuine upper bounds
t >= ||(1-eta)psi||, c >= ||(L-m)psi||, r >= ||(H-E)psi||, ell >= ||[H,eta]psi||.
Then, provided t<1 and q_lower^2=(1-t)^2-c^2>0,

    ||P_m eta psi|| >= q_lower,
    ||(H-E) P_m eta psi|| / ||P_m eta psi|| <= (r+ell)/q_lower.

The same bound applies with an independently enclosed norm for an unnormalized vector. E is a real spectral
center, not automatically the projected Rayleigh quotient. No Temple formula may silently use it as one.
This constructs an exact-sector quasimode bound **if the input norms are enclosed**. It is not a claim that
the unprojected vector is pure charge. A nonpositive denominator refuses. Known finite Fourier controls test it.

For the near-degenerate real Helike pair, diagonalize the 2x2 generator matrix measured at the fixed coarse
integration order, choose its positive/negative eigenvectors, and then freeze both complex combinations for
the fine order. The two eigenvectors of that matrix must straddle zero; otherwise refuse. Evaluate the full
generator defects, never just the restricted matrix. Ground and competitor target m=0. The same real pair
is selected from the two strongest Helike harmonic-reference weights among the first 12 eigenpairs; require
both weights >=0.95 and include the original selected Helike level. Preserve and report all original weights.

## 3. Full operator evaluation and numerical integration audit

Use the existing six-dimensional log chart u=u0+Sz and nu=8|det S|exp(3u0+2u1+u2).
Write psi=f P, f=prod(w/pi)^(1/4) exp(-sum(w z^2)/2)/sqrt(nu), with normalized Hermite polynomial P.
Polynomial derivatives are exact lowering operations on the saved Hermite coefficients. Include derivatives
of f, the full cometric, its divergence, the physical measure, the sourced potential and both cutoff terms.
Never use the Galerkin H squared as the residual norm.

Freeze tensor Gauss-Hermite orders **10 and 12 in each of six coordinates**, no point truncation or random
sampling. Evaluate in chunks of 32,768 for geometry and retain streaming result logs. Compare norm, centered
residual squared, charge defect squared, mass removed and commutator norm squared. Also report Rayleigh
estimates and the imaginary expectation component. A 1e-9*(1+|fine value|) discrepancy is a **new diagnostic
agreement check**, not a replacement for any existing scientific gate and not a quadrature error enclosure.
Keep separate outcomes NUMERICAL_AGREEMENT and NUMERICAL_DISAGREEMENT; neither is a certificate.

The controls use all four saved states (ground, two Helike partners, competitor). The Cartesian oracle uses
CPU float64 PyTorch automatic differentiation of q -> qq^T -> Cholesky -> z -> physical psi, not the reduced
cometric. Check Hpsi and H(eta psi) at the background, two fixed normal offsets, and the middle of each cutoff
shell. The existing oracle rule is 1e-9*(1+|Hpsi|); shell fixtures additionally require nonzero commutators.

Memory budget: 12 GiB for audit arrays, chunks as above. Per-rung wall budget: 900 seconds. Exceeding either
is a recorded refusal, not a reason to change integration orders or omit a channel. Save vectors and chart
data in NPZ, scalar checkpoints in JSON, and source snapshots before physical measurement.

## 4. Validated integration: frozen method, separate implementation gate

The numerical audit above cannot close the interval gate. The validated route is deterministic outward-rounded
box integration on the compact cutoff support in u, with exact derivatives and an interval extension of the
saved finite Hermite expression. Set U=2R and l=delta D0/U^2. The support satisfies
sqrt(l)<=r_i<=sqrt(U) and |a|,|b|,|c|<=sqrt(U); hence a finite box in u and its affine image in z covers it.
Use the physical measure, discard a box only when the cutoff is proved identically zero there, bisect the
largest normalized side (index order for ties), and prioritize the box of largest integral-width contribution.
Sum outward-rounded integrand ranges times volume; never treat interval point samples as integral bounds.

Freeze 128-bit ball arithmetic, at most 100,000 boxes per integrand, requested total absolute integral widths
1e-12 for squared residual/charge/leakage/tail and 1e-10 for normalization. No precision escalation or budget
increase without a new amendment. Matrix/chart constants and model parameters need explicit interval handling;
exact stored floating-point trial coefficients are permitted, but the original model must not be rounded to a
different model. Branch-cut, singular-box, rounding, normalization and exhaustion checks are mandatory.

**This validated backend is not yet implemented.** Until its independent oracle and mutation gates pass,
the evidence status must be UNRESOLVED_INTEGRATION, regardless of numerical agreement or zero sampled leakage.
The norm-bound formula alone cannot promote caller-supplied numbers into a certificate. No interval-certified
status or lower-bound/ordering result may be emitted by the numerical audit runner.

## 5. Frozen gates and strict TDD order

Preserve 0.003 refinement, 0.95 control weights, 1e-6 overlap error, 0.03 coefficient error, 0.9 grid weights,
0.015 switch tolerance, and the proposed 1e-10 cutoff-leakage gate. This phase does not erase the earlier
failed Mq=4 to 6 comparison. Separation hypotheses and the method amendment are not discharged by this work.

1. RED fixtures before implementation: cutoff endpoints/derivatives/invariance and nonzero shell terms;
   Hermite derivatives; Cartesian versus reduced H; positive/negative charges and the spectral projector bound;
   complete coverage/finite values and refusal to certify numerical-only evidence.
2. GREEN implementations on known polynomial/Gaussian controls, including a complex vector whose plain square
   incorrectly cancels and an off-space residual invisible to the compressed matrix.
3. Assertion-caught mutants: drop mixed shell term, drop either commutator contribution, omit measure drift,
   reverse charge sign, use plain complex squares, and promote numerical agreement to certification.
4. Commit the frozen construction and green focused tests before the real four-control audit. Log every stage.
5. Measure the same four controls. Preserve refusals and disagreements. The full grid remains held.

References: [SciPy Gauss-Hermite quadrature](https://docs.scipy.org/doc/scipy/reference/generated/scipy.special.roots_hermite.html)
and [Arb integration and failure contracts](https://arblib.org/acb_calc.html). Polynomial exactness does not apply
to a smooth cutoff times a log-chart exponential. One-dimensional complex integration is not a ready-made
six-dimensional validated integrator.
