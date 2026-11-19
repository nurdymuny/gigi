# The SU(2) force's defect derivative: exact, not finite-difference (spec)

2026-11-18. Written after the S3b Newton stall (Amendment 1 of
`docs/RICCI_SU2_PG_FLOW_CAYLEY_SPEC.md`) and before any S3b trajectory result. It changes
one helper, `defect_derivatives` in `scripts/ricci_su2_pg_force.py`, which every SU(2)
research script imports (S2a force, S2b stability, S2c numerical range, S3 flow, S3a filter,
S3b Cayley). No gate, control, threshold or step-size rule of any of those specs changes.

## In plain English

The Python force needs, for every face, how the face's defect changes when one of its
links is turned a little. It has been getting that by turning the link by 10⁻⁷ each way and
dividing the difference by 2×10⁻⁷. Dividing by such a small number multiplies the last-digit
rounding of the computer by about ten million. That is the "force noise" S3b measured at
about 5×10⁻¹⁰, and it is why Newton could not get below 10⁻¹⁰.

The change uses the exact derivative instead. Turning a link inserts one factor τ into the
face's product of links, so the derivative is that product with the τ in place: no small
step, nothing to divide by. It is the same number to about 10⁻⁹ (the finite-difference
error), without the noise.

## Why the Amendment 1 hypothesis is set aside

Amendment 1 suspected the chord reading U − 1, by analogy with computing 1 − cos θ
directly. That cancellation is too small to explain the floor. At n = 1 the face angles are
at most 0.094, so the scalar part of U − 1 is about −a²/2 ≈ −2×10⁻³ to −4×10⁻³, and
subtracting 1 loses only a factor of about 500: a relative error near 10⁻¹³ in that
component, and none in the vector part, which is computed directly. The observed
amplification is about 10⁶ to 10⁷, which is 1/EPS for the finite-difference step EPS = 10⁻⁷.

## The change

`defect_derivatives(u)` returns dD[e, k, face, comp] for the right-trivialised variation
u_e → u_e exp(t τ_k), as now. Each face defect is a product of factors (a link or its
inverse); face 123 is additionally conjugated by u₀:

| face | product |
| --- | --- |
| 0, 1, 2 | u_a u_b u_c⁻¹ |
| 3 | u₀ u₃ u₅ u₄⁻¹ u₀⁻¹ |

By the product rule, the derivative is the sum over every occurrence of link e of the same
product with that factor replaced by its derivative: u_e τ_k for u_e, and the conjugate of
u_e τ_k for u_e⁻¹ (on unit quaternions the inverse is the conjugate, which is linear).
The finite-difference version is kept as `defect_derivatives_fd`, for comparison only.

## Checks (thresholds fixed now)

Script: `scripts/test_ricci_su2_exact_defect_derivative.py`. It must fail against the
current finite-difference helper (RED) and pass after the change (GREEN).

- **D-agree.** On every cell at n = 1, the exact and finite-difference derivatives agree to
  10⁻⁸ relative (the finite-difference truncation is about 10⁻⁹).
- **D-group.** On random unit quaternions (not mesh data), the exact derivative matches an independent
  fourth-order central difference with step 10⁻³ (truncation about 10⁻¹², rounding about
  10⁻¹³), to 10⁻⁹.
- **D-engine.** The Python native gradient matches the Rust engine's gradient (the frozen
  `gradient` in each output file, which the engine computes by forward-mode jets) to 10⁻¹²
  relative at n = 1 and n = 2. Today it agrees to 4.5×10⁻¹⁰ and 4.8×10⁻¹⁰ (S2a's C-grad).
- **D-noise.** Nudging every link at rounding level (relative 2.2×10⁻¹⁶, three seeds) moves
  the native and two-ring gradients at n = 1 by less than 10⁻¹² relative. Today: 4.6×10⁻¹⁰ to
  5.3×10⁻¹⁰.

## What it changes downstream

Every SU(2) force moves by about 4×10⁻¹⁰ relative (measured: 4.5×10⁻¹⁰ native, 4.0×10⁻¹⁰
two-ring, n = 1). Every gated number in S2a–S3a is reported to at most four significant
figures and sits orders of magnitude above that, so no committed receipt is rewritten and
no verdict can change. Receipts made after this change record it through the commit hash.

For S3b, the Newton tolerance could return to the original 10⁻¹⁰. Whether to do that is an
amendment for S3b's owner; this spec does not make it.

## Not changed

The Rust engine (already exact), the recovery, the mass, the metric, the integrators, any
spec's gates.
