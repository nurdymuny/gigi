# The SU(2) force's defect derivative: exact, not finite-difference (results)

2026-11-18. Spec: `docs/RICCI_SU2_EXACT_DEFECT_DERIVATIVE_SPEC.md`. Test:
`scripts/test_ricci_su2_exact_defect_derivative.py`. Change: `defect_derivatives` in
`scripts/ricci_su2_pg_force.py` (the former version is kept as `defect_derivatives_fd`).

## In plain English

The S3b Newton solve stalled at 3×10⁻¹⁰ because the force was noisy at that level. The
noise came from the way the force differentiated each face: by a finite difference with step
10⁻⁷, which multiplies the computer's rounding by about ten million. With the exact
derivative the noise falls to rounding level, Newton converges to 10⁻¹⁵, and the Python
force now matches the Rust engine to rounding error instead of to 4.5×10⁻¹⁰. The chord
reading U − 1, suspected in S3b's Amendment 1, was not the cause.

## Test: RED before, GREEN after

| check | threshold | before (finite difference) | after (exact) |
| --- | --- | --- | --- |
| D-agree: exact vs finite difference, every n = 1 cell | < 10⁻⁸ | (no exact version: error) | 2.3×10⁻⁹ |
| D-group: vs a fourth-order difference, random unit quaternions | < 10⁻⁹ | 2.7×10⁻⁹ (fail) | pass |
| D-engine: Python native gradient vs Rust engine, n = 1 | < 10⁻¹² | 4.5×10⁻¹⁰ (fail) | 7.6×10⁻¹⁵ |
| D-engine: same, n = 2 | < 10⁻¹² | 4.8×10⁻¹⁰ (fail) | 1.7×10⁻¹⁴ |
| D-noise: native gradient under a rounding-level nudge, n = 1 | < 10⁻¹² | 5.0–5.3×10⁻¹⁰ (fail) | 1.4–1.7×10⁻¹⁴ |
| D-noise: two-ring gradient, same | < 10⁻¹² | 4.6–4.9×10⁻¹⁰ | 0.9–1.1×10⁻¹⁴ |

D-agree at 2.3×10⁻⁹ is the old finite difference's own truncation error: the two compute the
same derivative.

## S3b's diagnostics, rerun unchanged against the fix

`research/su2-cayley-20261118/diagnostics/`, n = 1 (level 0), native, N = 8, first step:

| | Newton increments ‖Δ‖/‖Ω‖, iterations 1 to 8 | settles at |
| --- | --- | --- |
| before, CG mass solve | 1, 1.7e-3, 1.1e-5, 1.0e-7, 1.1e-9, 3.5e-10, 3.3e-10, 3.2e-10 | 3×10⁻¹⁰ |
| after, CG mass solve | 1, 1.7e-3, 1.1e-5, 1.0e-7, 1.0e-9, 1.2e-11, 1.8e-13, 7.8e-14 | 6.4×10⁻¹⁴ (CG's own tolerance) |
| after, direct mass solve | 1, 1.7e-3, 1.1e-5, 1.0e-7, 1.0e-9, 1.2e-11, 1.5e-13, 2.6e-15 | 1.1×10⁻¹⁵ |

`force_noise.py`: velocity moves 0.3–2.2×10⁻¹³ (was 3.3–3.8×10⁻¹⁰), gradient 1.4–1.7×10⁻¹⁴
(was 5.0–5.3×10⁻¹⁰).

Newton now passes S3b's original tolerance of 10⁻¹⁰ at iteration 6. Restoring it, or keeping
Amendment 1's 10⁻⁸, is S3b's decision; nothing here changes that spec.

## Why the chord was not the cause

At n = 1 the largest face angle is 0.094, so the scalar part of U − 1 is at most about
4×10⁻³ in size: forming it by subtraction loses a factor of a few hundred, a relative error
near 10⁻¹³ in that one component. The vector part has no cancellation. The measured
amplification, about 10⁶ to 10⁷, is 1/EPS for EPS = 10⁻⁷.

## What else this explains

The S2b and S2c linearised operators are built by differencing the gradient again (column
step 10⁻⁶). Gradient noise of 5×10⁻¹⁰ put an error of about 5×10⁻³ into those columns, and
about 0.5 at a column step of 10⁻⁸. That is why, in the step-size check of the S2b review, the
count of eigenvalues below 0.1 at n = 1 held at 46 (native) and 41 (two-ring) for steps 10⁻⁵
to 10⁻⁷ and dropped to 32 and 30 at 10⁻⁸. At the step S2b used, the gated quantities (no
rough modes, six negative physical modes, the five most negative eigenvalues to three
decimals) did not move. No committed receipt is rewritten.

## Downstream

Every SU(2) force moves by about 4×10⁻¹⁰ relative (4.5×10⁻¹⁰ native, 4.0×10⁻¹⁰ two-ring, at
n = 1), far below every reported digit of S2a–S3a. Receipts made after this commit carry it in
their commit hash.
