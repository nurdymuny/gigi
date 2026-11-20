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
  certifies any fixed t; so far t = 3 (the paper) and t = 4 (Result 3) are certified.

## Result 3 (certified): the floor theorem at a second squashing, t = 4

`scripts/ricci_su2_gap_certificates_t.py` is the Floor Theorem's certificate script with the squashing as a
parameter. Run at t = 3 it reproduces the published receipt exactly: the same counts, the same R₀ and ω², the same
tail margin. Run at t = 4 (receipt `research/su2-floor-family-20261119/gap_certificates_t4.json`) it certifies the
following.

At the Berger metric g = (4, 1, 1):

1. **Every sector is bounded below.** In every sector of spin j/2, every gauge-orthogonal eigenvalue is at least
   (√(j(j+2)/4) − 6)² − 20 whenever √(j(j+2)/4) ≥ 6.
   - The Weitzenböck identity holds exactly in U(su(2)).
   - R₀ has eigenvalues −20 (twice), −8 (twice), 12 (three times) and 10 ± 2√33.
   - ω² = 36.
2. **The floor is exact in six sectors.** For j = 0, 12, 20, 24, 30, 32, the lowest gauge-orthogonal eigenvalue
   is λ₄(j) = j²/4 − 8: that is −8, 28, 92, 136, 217 and 248, each with multiplicity two. Every other eigenvalue
   of H + GG* is at least λ₄(j) + 1/3 − ε, with ε ≤ 8.7 × 10⁻⁵⁴ and smallest pivot 0.65.
3. **The low spectrum on the Poincaré sphere is fully known.** The gauge-orthogonal spectrum below 136 consists of:
   - the spin-0 values −8 (twice), 22 − 2√129 ≈ −0.716, 29/5 (twice) and 22 + 2√129 ≈ 44.72, computed exactly
     (`spin0_slice_t4.txt`);
   - the spin-6 values from 28 upward;
   - the spin-10 values from 92 upward.

   Spin 12 begins exactly at 136. The tail bound clears 136 from j = 36 on: exactly, √342 − 6 > √156, a margin of
   0.0032 in that form.

So at t = 4 there are still exactly three growth directions, −8 (twice) and 22 − 2√129, and the floor gap is
λ₄(12) − λ₄(0) = 36 = (240 − 24·4)/4. Unlike t = 3, spin 6 begins (at 28) below the top of the spin-0 spectrum
(44.72), so at t = 4 the sectors interleave rather than being separated by a gap.

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

- Certified statements exist at t = 3 (the paper) and t = 4 (Result 3) only. Minimality at other t, and the
  threshold t*, are double-precision evidence.
- It is not a claim about Ricci flow along the family. The flow moves t, and how the spectrum moves with it needs
  the moving-slice bookkeeping that Sol's note warns about.

## Natural next steps

1. ~~Certify a second squashing.~~ Done for t = 4 (Result 3). Any other rational t runs the same way.
2. Characterise t* exactly. It is where a second eigenvalue branch crosses λ_t(4); find that branch in closed form.

## Result 4: the soft-mode flow test (exact sector flow, no lattice)

**Script:** `scripts/ricci_su2_soft_mode_flow.py`, with predictions P1–P3 committed before the first run.
**Receipts:** `research/su2-floor-family-20261119/soft_mode_flow.json` and `soft_mode_controls.txt`.

**Crossings.** Each twist goes free where λ_t(j) = 0, at t_j = [√((j+2)² + 2(j+4)²) − (j+2)] / 2. That is
t₃ = 3.0453 and t₄ = 3.4031. At stretch t, every twist with j < (t − 4) + √(3t² − 4t) has negative cost.

**Setup.** The left-invariant background flows by the Yang–Mills gradient at fixed metric (t, 1, 1), starting from
the Berger field. A random spin-j disturbance flows by the gauge-fixed linearised flow, −(H + GG*)(A(τ)). Left
invariance keeps the sectors separate for all τ.

**Frozen background (the operator at the Berger field, held fixed).** The soft-mode law holds exactly in all six
runs (j = 3, 4; t = t_j − 0.05, t_j + 0.05, t_j + 0.5):

- **Sign.** The disturbance decays below t_j and grows above it, at rate −λ_t(j). The sign flips at the crossing.
- **Direction.** From a random start, the direction locks onto the twist: overlap 1.000 at τ = 2 in every run.

**Moving background (the actual flow).** The predictions P1 and P2 fail.

- The background is not at rest, and it collapses almost at once. At j = 3, t = 3.545, its energy falls from 34.6
  to 0.73 by τ = 0.1 and to about 0.02 by τ = 2, toward a flat field.
- The sector's floor flips from −3.27 to +3.98 within τ = 0.1.
- The twist overlap rises briefly (0.32 → 0.51 at τ = 0.1), then everything decays: log|b| falls by 7.5–10.6 over
  τ = 2 in all six runs.
- The strongest twist growth tested (rate 3.3, about 0.3 per e-fold) is several times slower than the background's
  collapse.

**P1 as written was ill-posed.** A random start's initial rate is its Rayleigh quotient, which the stiff modes
dominate. The frozen control answers what P1 meant to ask.

**Reading.**

- The twist law is a law of the *landscape* at the Berger field, and it holds there exactly: the frozen flow is the
  linearisation at that point.
- It is not, by itself, a dynamical transition. Starting from the Berger field, the background slides downhill
  faster than any twist can grow. This is Sol's caveat, now measured.
- A true phase-transition analogue needs a background **at rest**: a family of Yang–Mills critical points depending
  on t, whose Hessian has a twist going soft at some critical t. That is the natural next search: non-flat
  homogeneous Yang–Mills fields on the Berger spheres, the squashed relatives of the meron. There the soft-mode
  picture, a stiffness crossing zero linearly at a critical stretch, would be a genuine dynamical bifurcation.
