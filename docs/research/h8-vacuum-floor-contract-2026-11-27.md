# Strict separation from the reduced vacuum floor: frozen contract

27 November 2026. Pure deduction first; no spectral matrix computation.

## Statement and dependencies

Keep the unsourced s = 1 reduced Hamiltonian, the original Euclidean metric,
Lebesgue measure and full-space Friedrichs realization. Fix a compact simple
real Lie algebra with positive Ad-invariant inner product, n = its dimension,
t > 0 and coupling g > 0. Let omega = (sqrt(t), 1/sqrt(t), 1/sqrt(t)),
S = sum omega, C = <X1,[X2,X3]>, W = sum omega_k ||Xk||^2 / 2 - g C,
and H = -Delta/2 + ||grad W||^2/2.

**Target:** E0 > n S / 2, for every fixed choice above. The existing
cubic-quartic estimate supplies confinement and compact resolvent first.
Factorization supplies the lower bound; equality would force exp(-W),
which must be shown nonintegrable on a positive-measure cone. No embedded
representation or new matrix computation is needed: a nonzero bracket
already supplies a direction where C > 0.

This is a ground-energy shift above a factorization floor, not E1 - E0.
No quantitative lower shift, uniform-in-parameter bound, finite-coupling
carrier ordering, continuum theorem, or new exponential upper estimate.
H7 and H8 remain open; the top-level count remains seven.

## Gates, in order

1. Freeze this statement and exact tests; record missing-proof and missing
   Lean-declaration failures before implementation.
2. Derive the constant Laplacian and the factorization sign, including the
   form-core extension. Use compact resolvent before excluding equality.
3. Prove the weak-kernel characterization on connected full configuration
   space and divergence on an open spherical cap, not a single ray.
4. Exact symbolic checks: full nine-coordinate SU(2) polynomial; generic
   row-trilinear cubics; Gaussian zero-coupling control; cone-tail identity;
   normalized free-particle dilation showing why attainment is necessary.
5. Lean checks the scalar algebra, finite sum-of-squares implication and
   conditional attained-bottom deduction. It does NOT formalize the
   global Lie-algebra estimate, compactness, weak-kernel theorem, or
   integration on the sphere. Report this boundary with the result.
6. Remove or corrupt mechanisms: wrong Laplacian shift, missing attainment,
   missing kernel exclusion, and too-strong tail bound must be rejected.
   Exact counterexamples cover zero coupling and a zero cubic as well.
7. Fresh output directories only; no stale PASS. Retain red and failed
   development receipts, compiler logs, pinned dependencies, source hashes,
   and focused prerequisite regressions. No full engine suite or GPU run.

## Publication

Add the proof and bounded verification to the research notes; update only
the relevant H8 lower bound and closed-set change log. Do not change the
paper, model, normalizations, thresholds, or any broad conjecture status.
Commit and push the existing feature branch after the focused gates pass.
