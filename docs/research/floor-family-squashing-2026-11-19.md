# The floor formula across the Berger family (19 November 2026, night)

**Research note.** This follows idea 5 in Sol Light's agenda ("an anchor for perturbing the geometry"): what happens
to the floor away from the squashing (3, 1, 1)? It is an exploration. One result is symbolic; the rest is double
precision and not certified.

## Result 1 (symbolic, every t > 0)

For the Berger metric g = (t, 1, 1), take the Levi-Civita field read as an SU(2) connection, and the same circular
top-weight field b = f_top(θ¹ + iθ²) ⊗ (τ₁ + iτ₂) as Proposition 4.1 of the Floor Theorem paper. In every sector of
spin j/2, b is an exact eigenvector of the Hessian, orthogonal to every gauge direction, with eigenvalue

  λ_t(j) = [ j(j+2) − 2(t−3) j − 2(t−2)(t+4) ] / t  =  j²/t − 2j + 8j/t − 2t − 4 + 16/t.

At t = 3 this is the Floor Theorem's (j(j+2) − 14)/3.

**Proof.** The same three facts as Proposition 4.1, now verified symbolically in both j and t
(`scripts/ricci_su2_sector_formula_squashed.py`, output in `research/su2-floor-family-20261119/formula_squashed.txt`):

- b is an exact eigenvector of the 9 × 9 top-weight block;
- it is orthogonal to the gauge directions;
- there is no leak into the next weight.

The Rayleigh value equals the formula identically.

**The rest of the plane block.** Its other three eigenvalues are also closed form:
- (j² + 4jt − 8j − 2t² − 4t + 16)/t;
- (j² + jt + 5t² − 4t ± t√(j² + t²))/t.

## Result 2 (double precision, j ≤ 40): where the branch is the floor

Script: `scripts/ricci_su2_floor_family_scan.py`; receipt: `research/su2-floor-family-20261119/scan.json`.

| t | Sectors j ≤ 40 where something lies below λ_t(j) on the slice |
| --- | --- |
| 1 (round), 1.5, 2 | every sector |
| 2.5 | 1, 4, 5, 6, 7 |
| 2.55 | 4, 5, 6 |
| 2.6 | 4, 5 |
| 2.65 | 4 |
| 2.7, 2.75, …, 3, 3.5, 4, 5, 6, 8, 10 | none |

- **The branch is present everywhere.** It appears in the computed slice spectrum at every t tested, to 1e-13.
- **Threshold.** Below t* ≈ 2.69803 a lower eigenvalue appears, first in the sector j = 4 (found by bisection).
- **Not a proof.** Above t*, the branch is the bottom of every sector up to j = 40 at every t tested up to 10. That
  is evidence, not a theorem. The Floor Theorem's certificate machinery (Weitzenböck bound plus interval inertia)
  could certify any fixed t; only t = 3 is certified.

## Consequences that follow from the formula (on the branch, for t ≥ t*)

- **The floor gap on the Poincaré sphere** is λ_t(12) − λ_t(0) = (240 − 24t)/t.
  - At t = 3 that is 168/3. "One third of the first scalar eigenvalue" is specific to t = 3, not a rule.
  - The gap closes at t = 10. Beyond that, spin 6 dips below the left-invariant floor.
- **The lowest sector moves with the squashing.** Over j, λ_t(j) is smallest at j = t − 4. For mild squashing the
  bottom is spin 0. At t = 5 it is j = 1 (the scan shows −11 at j = 1 against −10.8 at j = 0). For strong squashing
  the global floor sits in higher spin, so on the Poincaré sphere, which keeps only some sectors, which surviving
  sector is lowest changes with t.
- **The left-invariant floor** λ_t(0) = −2(t−2)(t+4)/t is negative for every t > 2, the growth directions from the
  commutator. It vanishes at t = 2, where the branch has already stopped being the floor.

## What this is not

- It does not extend the Floor Theorem's certified statement. That remains t = 3, sectors j = 0, 12, 20, 24, plus
  the tail bound.
- It is not a claim about Ricci flow along the family. The flow moves t, and how the spectrum moves with it needs
  the moving-slice bookkeeping that Sol's note warns about.

## Natural next steps

1. Certify a second squashing, say t = 4 (floor j²/4 − 8), with the existing certificate script parametrised in t.
2. Characterise t* exactly. It is where a second eigenvalue branch crosses λ_t(4); find that branch in closed form.
