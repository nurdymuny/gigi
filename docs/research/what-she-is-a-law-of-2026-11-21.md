# What she is a law of: a hypothesis, with what is proved under it (21 November 2026)

**Status.** This note has three layers, kept apart on purpose:

- **Proved** (exact symbolic identities in x, y, t, j): her eigenvalue taken apart into its physical pieces, across
  the whole circle-symmetric family (§2).
- **Computed** (double precision, sectors j ≤ 12, not certified): what the competitor that sets her edge is made of,
  and how each of them responds to the field (§3).
- **Hypothesised**: the law itself, at three heights, each with what would kill it (§4).

Script: `scripts/ricci_su2_twist_anatomy.py`. Receipt: `research/su2-floor-family-20261119/twist_anatomy.json`.
Builds on:
- `exact-edge-2026-11-21.md`, with its addendum: the edge polynomial and the large-j proof on a domain;
- `twist-family-map-2026-11-21.md`;
- the Floor paper's Proposition "the twist is a lowest Landau level" and the v3 draft's Section 7.

## 1. The question

The exact edge showed that the Berger thresholds are not special to the Berger field. t_A and t* are where the Berger
path crosses a wall in the space of fields (x, y, t). The wall is the object; the thresholds are its shadow on one
path. So the question is no longer "where does the Berger field stop being her floor?" It is: **what is the wall a
law of?**

## 2. Her eigenvalue, taken apart (proved)

**Setting.**
- SU(2) on the stretched sphere (t, 1, 1).
- Circle-symmetric field: m₀ = x e₀ along the long circle (the fibre), and m₁ = y e₁, m₂ = y e₂ across it.
- Weitzenböck form: H + GG* = Σᵢ Xᵢ*Xᵢ + R₀, with Xᵢ = ρᵢ/√gᵢ + Ωᵢ and R₀ = Ric ⊗ I − 2[F, ·].
- She is b = f ⊗ β: f is the top weight of sector j along the long axis, and β = (θ̂¹ + iθ̂²) ⊗ (τ₁ + iτ₂).

**Field strength of the family** (exact):

- F₁₂ = B e₀, with **B = y² − 2√t·x**;
- F₀₁ and F₀₂ have size |y(x − 2/√t)| and point across the fibre.

B is the curvature in the plane transverse to the fibre, pointing along the fibre. It is the family's
chromomagnetic field along her circle.

**Four identities, each exact in x, y, t and j:**

1. **Her fibre momentum.** X₀b = iκ b, with

   κ = (t − 2)/√t − x − j/√t.

   The three terms are three different things:
   - (t − 2)/√t is the Levi-Civita rotation of her frame;
   - **−x is the gauge field acting on her Lie-algebra charge;**
   - −j/√t is her orbital motion along the fibre.

   **The fibre component of the field enters her momentum by minimal coupling.** That is the formal home of x: it is
   the gauge potential along her circle, seen by her charge.
2. **She is a lowest Landau level across the whole family.** (Ω₁ + iΩ₂)β = 0 for every x, y and t. Together with
   the top weight this gives X₊b = 0. The paper proves it on the Berger path; it holds for every member.
3. **Her curvature term.** R₀β = (Ric⊥ − 2B)β exactly, with Ric⊥ = 4 − 2t. The −2B is the spin of her internal
   frame meeting the field along the fibre.
4. **Her eigenvalue is the sum of these:**

   **λ = κ² + (2j + t + y²) + (Ric⊥ − 2B)**

   | piece | what it is |
   | --- | --- |
   | κ² | fibre momentum squared |
   | 2j + t + y² | transverse kinetic term of a lowest Landau level |
   | Ric⊥ − 2B | curvature: Ricci across the fibre, and spin against the fibre field |

   This equals the published closed form (x + (j + 2)/√t)² + 2√t·x − y² identically. That form is a rearrangement of
   the same number and does not show the anatomy.

**On the Berger field**, x = (2 − t)/√t and y = √t:
- the gauge part of κ equals the frame part, as it must, since the Berger field is the Levi-Civita connection read as
  a gauge field;
- κ = (2t − 4 − j)/√t, the transverse term is 2(j + t), and the curvature term is 12 − 8t;
- this is the paper's Proposition, recovered as one slice;
- there B = 3t − 4.

**What the anatomy explains.**
- **x enters her twice:**
  - through κ, by minimal coupling;
  - through B, by her spin.
- Her slope is ∂λ/∂x = −2κ + 4√t. It is about 10 at every point computed near the edge.
- **The failed backgrounds were kinetic failures, not magnetic ones.** On A = sθ, B = 4s(s − 1), which is positive
  for s > 1: the spin term helps her there as it does on the Berger field. What differs is the sign of x, and so her
  fibre momentum. Tests 4 and 5 found the curvature term does not decide it; the anatomy shows where it went.

## 3. The competitor (computed, j ≤ 12)

For a cross field y around 1 and above, the edge is set by the lowest state of the charge-0 block of sector 4,
B₄,₀.

**What it is made of.** At all three points computed, its lowest state is a mirror-symmetric superposition dominated
by two states:
- orbital weight m = +2 with internal charges (c_f, c_l) = (−1, −1);
- m = −2 with (+1, +1).

In each, the orbital motion and the internal spin point in opposite directions and cancel, so the total fibre charge is
zero.

| point | weight in the two m = ±2 states | slope in x, competitor | slope in x, twist (j = 4) |
| --- | --- | --- | --- |
| Berger field at t* | 0.62 | −1.56 | 9.74 |
| t = 4, y = 2, x = −0.19 | 0.66 | −0.62 | 9.62 |
| t = 8, y = √8, x = 0.14 | 0.73 | +1.68 | 10.18 |

**She is a charged mode with her orbit and spin aligned. Her competitor is neutral, with its orbit and spin opposed.**
The field along the fibre moves her about six to fifteen times faster than it moves the competitor. The competitor is
nearly blind to x, though not exactly.

**Her region is a half-line in x.** φ(x) = λ_min(B₄,₀) − λ(j = 4) is strictly decreasing on x ∈ [−3, 3] at all 48
(t, y) pairs tested (t from 2 to 10, y from 0.5 to 3). One crossing per pair, so one edge.

**The edge is not a function of one invariant.** Along the 27 computed edges:
- B ranges from 0.2 to 10.3;
- κ₄ ranges from −2.2 to +1.7.

Neither alone fixes the wall. It is a balance of the two, with the stretch.

**A proof route that does not close on its own.** Bounding the competitor's slope by the operator norm ‖∂B₄,₀/∂x‖
works at 21 of 27 edge points. It fails at every edge point with y = √t and t ≥ 4. The actual slope is small because
of the competitor's eigenvector, not because of the matrix norm. A proof of the half-line needs that eigenvector, for
example in an eigenvector-aware interval certificate.

## 4. The hypothesis, at three heights

### H1: the family law (STRUCTURAL; the narrowest)

> *For SU(2) on (t, 1, 1) with a circle-symmetric field, the twist is the floor of every sector exactly when
> x < x\*(t, y). Here x\* is the largest real root in x of the edge polynomial P₂(x, y, √t) for a cross field around 1
> and above, and of the corresponding polynomials of the sector-2 and sector-3 blocks below that (not yet derived). Her region is a
> half-line in x.*

- **Proved toward it:**
  - her eigenvalue everywhere (§2);
  - the edge polynomial and its Berger restriction;
  - the large-j half of the all-sector statement on the domain D = x ∈ [−3, −1.5], y ∈ [0.5, 3], t ∈ [3, 10].
- **Running:** the small-j half on D.
- **Computed, not proved:** the half-line, and the edge for j ≤ 12.
- **Kills it:**
  - a second crossing in x (a hole in her region);
  - a sector above 12 that undercuts her inside the claimed region.
- **The Floor Theorem is H1 restricted to the Berger path.**

### H2: what the law is (STRUCTURAL for the anatomy; RESONANCE for the physical reading)

> *The twist is a charged, spin-aligned fluctuation, minimally coupled to the fibre component of the background
> field. Her competitor is a neutral one, with its orbit and spin opposed. She is the floor exactly where the field
> along the fibre, through her momentum and through her spin, pulls her below that neutral competitor. The wall is the
> point where a charged mode and a neutral mode trade places, and because their charges differ, the crossing is a true
> crossing.*

**Exact in it** (§2):
- the minimal coupling (−x in κ);
- the spin term (−2B);
- the lowest Landau level.

**A crossing, not an avoided one.** She is alone in her charge block, and her competitor carries charge 0. States in
different symmetry sectors may cross, while states in the same sector generically repel (the non-crossing rule of von
Neumann and Wigner). That is why the edge is sharp.

**Computed in it** (§3):
- the competitor is neutral with orbit and spin opposed;
- it is nearly blind to x.

**Kills it:** a background in the family where the competitor that sets the wall carries a nonzero total charge, or
where its slope in x is comparable to hers.

### H3: the general law (a prediction; the furthest out)

> *On a homogeneous circle bundle with a conserved fibre charge:*
> 1. *the top-charge, spin-aligned state is an exact gauge-orthogonal eigenvector;*
> 2. *its eigenvalue splits as fibre momentum squared, plus a lowest-Landau-level transverse term, plus Ricci across
>    the fibre minus twice the fibre field strength, with the fibre component of the gauge field entering her
>    momentum by minimal coupling;*
> 3. *where she is the floor, the boundary is set by a neutral competitor that is nearly blind to the fibre field,
>    so her region is a half-space in the fibre component of the field.*

**Evidence, by part:**
- **(1):** proved for the SU(2) family. Shown in SU(3) family 2a of Test 2, where the extremal state is an exact
  gauge-orthogonal eigenvector at every grid point and sector.
- **(2):** proved for the SU(2) family (§2).
- **(3):** computed for the SU(2) family only.

**What H3 does not say:**
- **It does not say her region is nonempty.** In the SU(3) Maurer–Cartan families there is no floor point at all,
  although in 2a the extremal state is exact. Those families fix the field; they give it no free component along the fibre to turn against
  her.
- **It does not say her region is x < 0.** The edge crosses zero at large stretch with a strong cross field.

**Kills it:**
- an SU(3) family with a free fibre field where (1) or (2) fails;
- a two-sided region;
- a competitor as sensitive to the fibre field as she is.

**The test I would pre-register next.** SU(3) on the stretched hypercharge circle (family 2b), with the field along
that circle freed as a parameter x:
- (a) her exactness and her anatomy at every grid point;
- (b) her region is a half-line in x wherever it is nonempty;
- (c) the competitor that sets its edge has total charge zero.

## 5. Where this sits

Not new, and cited as such:
- A charged vector mode in a chromomagnetic field, lowered by its spin coupling −2B: Nielsen and Olesen, "An unstable
  Yang–Mills field mode", *Nucl. Phys.* B144 (1978) 376–396.
- Savvidy's vacuum: Savvidy, *Phys. Lett.* B71 (1977) 133–134.
- In the Yang–Mills Hessian both come from the same term, −2[F, ·], that gives her −2B here. Calling her the
  Nielsen–Olesen mode of this geometry is a **RESONANCE**, not an identity: theirs is a flat space with a constant
  field, and hers is a compact squashed sphere with Ricci in the balance.
- Functions on S³ of fixed fibre charge are sections over S² of the Hopf line bundles. The transverse operator is then
  a Landau problem on the sphere: Wu and Yang, "Dirac monopole without strings: monopole harmonics", *Nucl. Phys.*
  B107 (1976) 365–380. Her lowest-Landau-level property is this picture made exact (§2, identity 2).
- The non-crossing rule: von Neumann and Wigner, *Phys. Z.* 30 (1929) 467–470.

What this work adds:
- the anatomy exact and gauge-orthogonal on squashed spheres;
- the wall as an explicit polynomial;
- the thresholds of the Floor Theorem as its crossings.

## 6. Ledger

| claim | status | where |
| --- | --- | --- |
| κ = (t − 2)/√t − x − j/√t, with −x the gauge part | proved | §2, P1 |
| X₊b = 0 across the family | proved | §2, P2 |
| R₀β = (Ric⊥ − 2B)β, F₁₂ = B e₀ | proved | §2, P3 |
| λ = κ² + (2j + t + y²) + (Ric⊥ − 2B) | proved | §2, P4 |
| Berger slice reproduces the paper's Landau proposition | proved | §2, P5 |
| edge polynomial P = P₁P₂; P₂ contains the t_A polynomial | proved | exact-edge addendum |
| the floor for j ≥ 80 on D | proved | exact-edge addendum |
| the floor for j < 80 on D | running | exact-edge addendum |
| competitor = neutral, orbit and spin opposed | computed | §3 |
| competitor nearly blind to x | computed (3 points) | §3 |
| half-line in x (block B₄,₀) | computed (48 pairs) | §3 |
| H1, H2, H3 | hypothesis | §4 |

## In my words

I have said the direction governs her. This is that, made exact:

- **The direction is not where the curvature points.** Tests 4 and 5 showed that.
- **It is how the field along her own circle meets her momentum and her spin.** She is the charged one, the one the
  field can carry. Her rival is neutral, and the field barely touches it.
- **The wall is where the two trade places.**

What is proved stays proved, the half-line stays computed until it is certified, and the law stays a hypothesis until
SU(3) has had its chance to break it.
