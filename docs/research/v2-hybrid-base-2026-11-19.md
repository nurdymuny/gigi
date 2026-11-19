# A third base for V2: own fluxes exact, quadratic bias only from the cell itself (19 November 2026)

Research note, written while the n = 2 measurement runs. Responds to Amendment 2 of
`docs/VARIATIONAL_RECOVERY_V2_SPEC.md` (the counter-review). Probe (exploratory, not gated):
`scripts/ricci_variational_baseprobe.py`; receipts `research/variational-baseprobe-20261119/L0_three_bases.json`
(n = 1) and `L1_three_bases.json` (n = 2). The earlier two-base receipts `L0.json`,
`L1.json`, cited by `v2-review-2026-11-19.md`, are unchanged. All numbers here have the change C = 0:
they describe the *base* V2 would freeze, not V2.

## The counter-review, and where this note agrees

Amendment 2 is right on every point:

- **V2q drops the cell's own fluxes.** Keeping only the linear part of the 26-field fit throws away the
  quadratic part's flux through the cell's own faces: a 4.75% defect (median, measured here too).
  *Reading the Curve* treats "own faces held exactly" as a design principle: the recovery refines
  the Whitney field rather than replacing it.
- **No linear output can both reproduce a quadratic field's own fluxes and read it as zero.**
- `lsqr` from x₀ = 0 returns the minimum-norm solution V1's objective asks for; stopping on the C-solve
  residual ties the solver to the control. Better than Amendment 1's version.
- The V2q removal arm keeps the comparison symmetric; the 26-dimensional trial space is rightly a
  separate design.

## The idea: zero is not the only unbiased answer

What made the linear three-ring base worse at n = 2 was bias drawn from the **outer** rings: a linear
fit over a wide neighbourhood reads the field's quadratic part as linear. Reading quadratic fields as
zero removes that bias but loses the own fluxes. A third base keeps the own fluxes and removes only the
outer-ring bias:

**hybrid:** fit the 26 closed linear and quadratic coefficients over three rings (own faces exact),
keep the linear part, and add back the minimum-norm closed linear field that carries the quadratic
part's flux through the cell's **own** faces:

    M_hybrid = M₂₆[:11] + pinv(A_lin) · A_quad · M₂₆[11:]

where A_lin, A_quad are the own-face flux rows of the linear and quadratic fields. Then

- own fluxes are reproduced exactly (the 26-field reproduces them, and the added field carries exactly
  the part the linear part misses);
- closed linear fields are reproduced exactly (their quadratic part is zero);
- a quadratic field is read as the cell's own-face representative of it, local to the cell, with no
  contribution from the outer rings.

## n = 1 (pre-asymptotic)

| base, three rings, C = 0 | PG force | variational | reconstruction term | test-variation term | curvature error | amplification | own-flux defect | linear exactness |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| linear (V2) | 4.02% | 39.0% | 1.49% | 37.6% | 3.60% | 21.1 | 4×10⁻¹⁶ | 8×10⁻¹⁵ |
| quadratic-aware (V2q) | 4.83% | 53.6% | 6.87% | 48.2% | 1.57% | 27.6 | **4.75%** | 3×10⁻¹⁵ |
| **hybrid** | 5.47% | 60.7% | 4.26% | 61.0% | **1.46%** | 26.8 | **5×10⁻¹⁶** | 3×10⁻¹⁵ |

The hybrid does what it was built to do: own fluxes exact (5×10⁻¹⁶), linear fields exact, and the best
curvature error of the three. Its reconstruction term sits between the other two. At n = 1 the linear
base still has the smallest reconstruction term, as in the two-base probe; n = 1 is pre-asymptotic here,
as everywhere in this work, and the ranking reversed at n = 2 last time.

## n = 2: the hybrid is best on every count

| base, C = 0 | PG force | variational | reconstruction term | test-variation term | curvature error | amplification (median / max) | own-flux defect |
| --- | --- | --- | --- | --- | --- | --- | --- |
| published (linear, two rings) | 3.83% | 276% | 3.67% | 276% | 0.92% | 35.2 / 44.7 | 4×10⁻¹⁶ |
| linear, three rings (V2) | 5.63% | 184% | 4.78% | 182% | 0.98% | 21.5 / 34.5 | 4×10⁻¹⁶ |
| quadratic-aware (V2q) | 1.19% | 249% | 1.29% | 249% | 0.41% | 16.4 / 19.0 | **4.8%** |
| **hybrid** | **0.85%** | **111%** | **0.92%** | **111%** | **0.36%** | **16.2 / 18.5** | **5×10⁻¹⁶** |

- The hybrid has the smallest reconstruction term (0.92%, against V2q's 1.29%) **and** keeps the cell's
  own fluxes exact. The departure Amendment 2 attached to V2q is not needed.
- It also starts V2 from the smallest test-variation term of any base (111%, against 182% to 276%), so
  the column condition has the least to remove.
- **By-product, a lead not a result:** the Petrov–Galerkin force with the hybrid base is 0.85% at n = 2,
  against the published 3.83%, with the own-face principle intact. It needs n = 3, 5 and the S3 spectra
  before it means anything for *Reading the Curve*.
- By the criterion stated before the run, this triggers **Amendment 3** (added to the V2 spec): a third
  arm, V2h.

## Running now (this machine)

| job | branch | state |
| --- | --- | --- |
| S2d one-ring removal, n = 3 | `feat/s2d-n3-spectrum` | running; the two-ring force already passed both n = 3 gates |
| S3c two-ring, n = 2, N = 40 and 80 | `feat/s3c-long-window` | running; decides P-long (two-ring) and P-cross |

Finished today and written up: V1 and V1b (fail; explained by the count), S3e (stiff modes are the cause
of the N = 4 failure; a second-order damping start-up halves it), S3d, S4, and S2d's n = 3 gates.
