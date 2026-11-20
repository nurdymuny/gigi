# B6 results: stability and numerical range from the exact-derivative K

2026-11-19. Spec: `docs/RICCI_SU2_EXACT_STABILITY_SPEC.md`. Script `scripts/ricci_su2_exact_stability.py`,
**research script runs, not engine runs**. Receipts: `research/su2-exact-stability-20261119/`.
Level index L = 0, 1, 2, 3 is n = 1, 2, 3, 5 (426, 3,372, 11,358, 52,530 unknowns).

## Summary

- **The two-ring force is stable at n = 5.** The left edge of its numerical range is −4.6661,
  certified by inertia: no generalized eigenvalue of (½(K+Kᵀ), B) lies below −10 or below −20.
  Since every eigenvalue lies in W, G-W at n = 5 also gives G-rough. Native: −4.6708.
- **The removal fails as required at every level.** The one-ring operator has 18,159 directions below
  −10 at n = 5 and a witness Rayleigh quotient of −131,683 (n = 3: −45,922; n = 2: −19,993;
  n = 1: −8,774). The gate measures roughness.
- **The finite-difference caveat is removable** (G-eps passes at n = 1, 2, all three forces).
- **What did not run:** dense B6 at n = 3. It was stopped after about five hours in the 180-angle
  sweep, before writing a receipt. So C-sweep and the n = 3 numerical radius and η are not in B6.
  The n = 3 stability numbers come from S2d's dense run (`research/su2-spectrum-n3-20261119/`,
  on `feat/reading-the-curve-update`), and C-sparse compares against those.

## Gates

| gate | level | native | two-ring PG | one ring (removal) |
|---|---|---|---|---|
| G-rough (no Re λ < −10) | n = 3 (S2d dense) | 0 | **0, pass** | 4,437, fails as required |
| G-W (left edge ≥ −10 and ≥ 2 × native) | n = 3 | −4.6763 | **−4.6656, pass** | −45,922, fails as required |
| G-W | n = 5 | −4.6708 | **−4.6661, pass** (bound −9.342) | witness −131,683, fails as required |
| G-eps (count and left edge unchanged over ε = 1e-5, 1e-6, 1e-7) | n = 1, 2 | pass | pass | pass |

G-eps: the count of eigenvalues with modulus < 1 is identical at all three ε, and the left edge
agrees to at most 1.2 × 10⁻¹⁰ relative (gate 10⁻⁶), for every force at both levels.

## n = 5 detail (sparse, inertia certificate)

| | inertia below −20 | below −10 | below 0 | left edge | slowest pair of (K, B), nearest −5 |
|---|---|---|---|---|---|
| native | 0 | 0 | 3,475 | −4.670847 | −4.6647 ± 0.0620 i |
| two-ring PG | 0 | 0 | 3,430 | −4.666091 | −4.6600 ± 0.0574 i |
| one ring | 18,121 | 18,159 | 21,668 | none (witness −131,683) | −4.6557 ± 0.0575 i |

No zero pivots in any factorisation. The smallest |pivot| at σ = −10 is 4.93 (two-ring) and 4.99
(native); at σ = 0 it is 5 × 10⁻⁶ to 7 × 10⁻⁶, which is why the gate line, not 0, carries the
certificate. LU fill: 812 million nonzeros for the two-ring factor (14.1 million in K); it fit in memory.

- **C-resid:** every returned eigenpair has relative residual ≤ 9.7 × 10⁻¹⁴ (shift-invert) and
  ≤ 2.0 × 10⁻¹² (the 20 eigenvalues of (K, B) near −5); the gate is 10⁻⁸.
- **C-hash:** each K is loaded against its B5 kbuild SHA-256 (the script refuses otherwise).

## The sparse method at n = 3 (C-sparse) against S2d's dense run

| | inertia below 0 vs dense count of S < 0 | sparse left edge vs dense λ_min(S) |
|---|---|---|
| native | 774 = 774 | −4.67628608431804 vs −4.676286084322474 (9 × 10⁻¹³ rel.) |
| two-ring PG | 772 = 772 | −4.665611154288831 vs −4.665611154269237 (4 × 10⁻¹² rel.) |
| one ring | 5,116 = 5,116 | witness −45,922.2827 = dense left edge −45,922.2827 |

Counts equal exactly, left edges well inside 10⁻⁸. The inertia count at σ = −10 (4,320 for one ring)
has no dense counterpart in the S2d receipt, which records Re λ(K) < −10 (4,437), a different count;
that half of C-sparse is not checked.

## n = 1, 2 against S2b/S2c (dense, exact K)

| level | force | worst eigenvalue shift vs S2b (rel. max\|λ\|) | left edge | rough | \|λ\| < 1 | η | F1 |
|---|---|---|---|---|---|---|---|
| n = 1 | native | 5.4e-6 | −4.6338 | 0 | 62 | 0.0014 | 1.0000 |
| n = 1 | two-ring | 8.7e-6 | −4.5628 | 0 | 62 | 0.134 | 1.0075 |
| n = 1 | one ring | 1.0e-6 | −8,774 | 207 | 62 | 0.116 | 1.0052 |
| n = 2 | native | 3.3e-7 | −4.6821 | 0 | 483 | 0.0003 | 1.0000 |
| n = 2 | two-ring | 1.5e-6 | −4.6644 | 0 | 481 | 0.142 | 1.0032 |
| n = 2 | one ring | 2.5e-7 | −19,993 | 1,451 | 483 | 0.176 | 1.0077 |

Verdicts unchanged from S2b/S2c. Controls C-chol (≤ 1.7 × 10⁻¹⁶) and C-W (no eigenvalue left of
the edge; support violations ≤ 3.3 × 10⁻¹¹) pass.

## What this closes, for *Reading the Curve*

- "Stability and numerical range are measured at n ≤ 2" → the left edge and the stability gate are
  now certified at n = 5; the full spectrum and η at n ≤ 3 (n = 3 from S2d). The numerical radius
  and η are not computed at n = 5 (spec).
- The S2b finite-difference caveat can be dropped: G-eps passes everywhere it was defined.
- Not closed: C-sweep at n = 3 (dense B6 n = 3 did not finish), time integration at n = 5.
