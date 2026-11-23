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
  certifies any fixed t, or a whole range of t (Result 5): t = 3 (the paper), t = 4 (Result 3), and every t in [2.75, 10] for j ≤ 12, 20, 24 (Result 5).

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

- Certified: t = 3 (the paper), t = 4 (Result 3), and every t in [2.75, 10] for j ≤ 12, 20, 24 (Result 5).
  Other sectors outside those, t < 2.75, and the threshold t* ≈ 2.698 are double-precision evidence.
- It is not a claim about Ricci flow along the family. The flow moves t, and how the spectrum moves with it needs
  the moving-slice bookkeeping that Sol's note warns about.

## Natural next steps

1. ~~Certify a second squashing.~~ Done for t = 4 (Result 3). Any other rational t runs the same way.
2. Characterise t* exactly. It is where a second eigenvalue branch crosses λ_t(4); find that branch in closed form.

## Result 5 (certified over a range): the floor law for every t in [2.75, 10]

**Script:** `scripts/ricci_su2_floor_interval_cert.py`. **Receipt:**
`research/su2-floor-family-20261119/interval_cert_2.75_10.json`.

**Statement.** For **every** t ∈ [2.75, 10] and every j ∈ {0, 1, …, 12, 20, 24}, λ_t(j) is the lowest
eigenvalue of H + GG* in the sector of spin j/2, with multiplicity exactly two. Its eigenvector lies in the gauge
slice, so λ_t(j) is the floor of the gauge-orthogonal spectrum there.

**Method.**

- Each slice [a, b] of t is certified once, for all t in it at the same time. The matrix
  A(t) − (λ_t(j) + δ) is enclosed with t and λ_t(j) as intervals (mpmath's interval context, 96 bits), realified
  and factored LDLᵀ at its midpoint.
- The perturbation bound is ε = ρ + backward error, where ρ = min(Frobenius, max row sum) of the enclosure radii.
  Both bound the 2-norm of a symmetric perturbation.
- Exactly four negative pivots and ε < δ give, for every t in the slice: at most two eigenvalues lie below
  λ_t(j) + δ − ε.
- λ_t(j) itself is an exact double eigenvalue for every t (Result 1). So it is the lowest, and every other
  eigenvalue lies above λ_t(j) + δ − ε > λ_t(j).

**Data.**

- 1,091 slices. The finest is 0.002 wide, near t = 2.75 where the gap above the floor is smallest: 0.16 for j = 4,
  0.18 for j = 1.
- Every slice has exactly four negative pivots and ε < δ. The smallest margin δ − ε is 0.0018 (j = 1).
- The slices tile [2.75, 10] exactly in every sector. No slice failed.
- Point check: a degenerate slice at t = 3 reproduces the published certificate (j = 12: four negative pivots,
  smallest pivot 21.29).

**Scope.** This proves the floor law across a whole range of shapes, for these fifteen sectors. It does not cover
every sector. At large j the Weitzenböck tail bound grows more slowly than λ_t(j) itself, so it cannot replace
per-sector certificates. The sectors chosen are all j ≤ 12, including j = 4, the first to fail below t*, plus the
Poincaré-surviving 20 and 24.

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

## Result 6: the resting fields on the stretched spheres, and no soft mode on them (t from 0.05 to 10)

**Scripts:** `scripts/ricci_su2_resting_fields.py` (multistart search) and `scripts/ricci_su2_meron_family_soft.py`
(continuation and sector spectra). **Receipts:** `research/su2-floor-family-20261119/resting_fields.json` and
`meron_family_soft.json`.

**The search.**

- Left-invariant SU(2) fields on g = (t, 1, 1) have 9 parameters. Their energy, gradient and Hessian are exact
  polynomials.
- By symmetric criticality (Palais), a critical point among left-invariant fields is a Yang–Mills field.
- Newton was run from 400 random starts at t = 1, 1.5, 2, 2.5, 3, 3.5, 4 and 5, and the solutions classified up to
  constant gauge rotation. Each time it found exactly three:
  - the trivial flat field (energy 0);
  - a Maurer–Cartan-type flat field (energy 0, singular values 2, 2, 2/√t);
  - **one non-flat resting field, the squashed meron.**
- This is a multistart search, not a proof that there are no others.

**The squashed meron.**

- It fits the ansatz m₀ = x·e₀, m₁ = y·e₁, m₂ = y·e₂, with every point checked critical in all nine directions.
- At t = 1 it is the meron (x = y = 1, energy 3/2).
- It continues smoothly in both directions: energy 1.13 at t = 1.5, 0.653 at t = 3 (x = 0.4741, y = 1.0858), 0.420 at
  t = 5, 0.223 at t = 10.
- Its left-invariant index is 1 throughout.

**Its spectrum** (H + GG* by spin sector, j ≤ 12, t ∈ [0.05, 10]).

- The Morse index is exactly 1 everywhere: one negative direction, in the left-invariant sector, inherited from the
  meron (Laquer's index 1 for a group manifold).
- **No sector crosses zero.** There is no soft mode and no bifurcation along this family in the range searched.
- All higher sectors soften as the sphere stretches. The j = 1 floor goes 1.00, 0.76, 0.62, 0.45, 0.28 at t = 1, 2,
  3, 5, 10. It falls more slowly than 1/t (t × floor rises from 1.0 to 2.8), so it reaches zero only in the limit,
  if at all.

**Reading.**

- The resting relative of the Berger field exists, but it does not breathe in and out with the stretch. It stays a
  one-way saddle with every other direction stiff.
- The twist law at the Berger field (Results 1–5) and this resting family are different objects. The twist goes
  free on a landscape that is not at rest. The resting field never lets a twist go free.
- Two places remain where a true soft-mode transition could live:
  - resting fields that are **not** left-invariant (the search above sees only left-invariant ones);
  - other squashings (t₁, t₂, t₃ all different), where the symmetry is lower.

## Result 7: unevenly stretched spheres (t₁, t₂, 1): the same three resting fields, still no soft mode

**Script:** `scripts/ricci_su2_resting_fields_general.py`. **Receipt:**
`research/su2-floor-family-20261119/resting_fields_general.json`.

**The search.**

- Only the shape matters, so t₃ = 1 is fixed.
- t₁ and t₂ range over {0.25, 0.5, 0.75, 1, 1.5, 2, 3, 5}, with t₁ ≤ t₂ (swapping them is a symmetry). That is 36
  shapes, with axis ratios up to 20.
- 300 random starts at each shape.

**The resting fields.**

- At every shape there are exactly three resting left-invariant fields up to gauge: two flat, and one non-flat
  relative of the meron. Its energy runs from 8.18 at (0.25, 0.25, 1) to 0.139 at (5, 5, 1).

**Its spectrum.**

- Its Morse index is exactly 1 at every shape: one negative direction, in the left-invariant sector.
- Every spin sector j = 1–8 stays positive.
- The closest approach to zero is j = 1 at 0.405, at (5, 5, 1). That is the flattened shape (0.2, 1, 1) with the
  axes renamed, which Result 6's one-axis scan already followed down to t = 0.05 with index 1 throughout.

**Reading.** Among left-invariant fields, the stretched spheres carry one non-flat resting field, the meron's
relative. It keeps the round meron's single unstable direction across the whole two-parameter family of shapes
sampled, and no twist or other mode goes soft on it.

**What remains open.**

- **Resting fields that are not left-invariant.** For example, fields invariant only under a circle subgroup, or
  under the right action. That is a PDE or ODE search, not an algebraic one.
- **Resting fields in other bundles.** On the Poincaré sphere, the non-trivial flat connections from
  representations of the binary icosahedral group in SU(2) are minima (energy zero), so they have no negative modes
  at all.

## Result 8: the floor is a lowest Landau level

See [`spectral-skeleton-2026-11-20.md`](spectral-skeleton-2026-11-20.md), §6. The twist satisfies X₊b = 0 and has
definite fibre <del>charge q = j − 2(t − 2)</del><ins>momentum −2(q − t), with q = j/2 + 2 the conserved fibre
charge (spectral-skeleton §7); X₊ annihilates no other mode (control, `landau_control.json`)</ins>, and

  λ_t(j) = (j − 2(t−2))²/t + 2(j + t) + (12 − 8t)  (fibre + Landau + curvature),

to round-off in 90 cases (`research/su2-floor-family-20261119/landau_check.json`).

## Result 9 (Proved): the floor law in every sector, for every t in [2.75, 10]

For every t ∈ [2.75, 10] and every j ≥ 0, λ_t(j) is the lowest eigenvalue of H + GG* in the sector of spin j/2, with
multiplicity exactly two, and the lowest gauge-orthogonal eigenvalue there. At t = 3 this settles every sector of
the original Floor Theorem, not only those that survive on the Poincaré sphere. Theorem 6.6 of the Floor paper, v2.

The proof has four parts.

1. **Charge blocks (exact).** `scripts/ricci_su2_charge_selection_symbolic.py`, receipt `charge_selection_symbolic.txt`.
   - In the charge basis of frame ⊗ Lie, Ω₀ = i·diag(c(t−2)/√t), Ω± shift c by exactly ±1, and R₀(t) preserves c.
   - So H + GG* preserves q = m + c, and each sector is the orthogonal sum of blocks of size at most 9.
   - The twist and its conjugate are the only states of charge ±(j/2 + 2).
2. **Sectors j ≤ 99 (interval certificates).** `scripts/ricci_su2_block_interval_cert.py`. Every non-twist block is
   enclosed for all t in a slice at once and certified positive definite after the shift λ_t(j) + η.
   - j ≤ 40: 943 blocks, 12,596 block-slices, smallest margin 1.7 × 10⁻⁵ (`block_cert_j40.json`).
   - 41 ≤ j ≤ 99: 4,307 blocks, 36,166 block-slices, smallest margin 2.9 × 10⁻³ (`block_cert_j41_99.json`).
   - No failures.
3. **Sectors j ≥ 100 (Gershgorin).** `scripts/ricci_su2_floor_large_j.py`, receipt `floor_large_j.json`.
   - The diagonal minus λ minus the Gershgorin radius is a closed form G(l, x, t) per state type. This is exact,
     identically in l, x and t, as is the mirror symmetry x → 2l − x (charge_selection receipt).
   - G > 0 is proved for l ≥ 50 by interval bisection:
     - x ≤ 20: G/l > 5.9 × 10⁻⁴, 373 boxes;
     - x ≥ 20, on a relaxed box: G/(lx) > 0.85, 9 boxes.
4. **The twist block** is 1 × 1 with entry λ_t(j) (Result 1 / Proposition 6.1).

**Not claimed.**
- No quantitative gap above the floor for every j. The large-j part gives a strict inequality, not a margin linear
  in j.
- Nothing outside [2.75, 10]. t* ≈ 2.698 remains numerical.


**Reissue (20 November, after the external adversarial review).** Every interval certificate has been reissued under the
outward error budget of `scripts/ricci_su2_outward.py`:
- Table 1 at t = 3;
- t = 4;
- the range certificate (15 sectors);
- the block certificates j ≤ 40 and 41 ≤ j ≤ 99.

The repair: radii are taken about the stored midpoints with outward rounding, and ρ, the backward-error term and ε
are accumulated in interval arithmetic. Results:
- All pass.
- Pivots, slice counts and pivot signs are unchanged.
- ρ grows slightly.
- The smallest margins quoted for Theorems 6.4 and 6.6 are unchanged at the precision quoted, except that the t = 4
  bound now reads ε < 8.8 × 10⁻⁵⁴.

The large-j proof does not use the conversion and was already outward.

**Second reissue (20 November, after the second adversarial review).** The certificates now bound the factorization
a posteriori: ‖M − LDLᵀ‖_F is evaluated in outward interval arithmetic from the stored factors, so no
rounding-error constant (Higham-type) is needed. All certificates were reissued again.
- Table 1: all certified, ε < 7 × 10⁻⁵⁶.
- t = 4: all certified, ε < 1.3 × 10⁻⁵⁵.
- Range certificate: 1,091 slices, all certified, minimum margin 0.0018.
- Block certificates: j ≤ 40 with 943 blocks and 12,596 block-slices, minimum margin 1.7 × 10⁻⁵; 41 ≤ j ≤ 99 with
  4,307 blocks and 36,166 block-slices, minimum margin 2.9 × 10⁻³.
- No failures.

**Name (21 November 2026).** The twist, the unique state of maximal charge
b = f (θ̂¹ + iθ̂²) ⊗ (τ₁ + iτ₂), is called **Helike** (Ἑλίκη, "she who turns"); see
[`the-twist-a-note-2026-11-21.md`](the-twist-a-note-2026-11-21.md). "The twist" remains the technical term.

