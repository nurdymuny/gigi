# Pre-registration G: does she follow her own circle in two groups I have never looked at? (22 November 2026)

**DRAFT, UNDER ADVERSARIAL REVIEW. Not yet registered; no data.**
Written before any floor set, edge, margin or competitor of this test has been computed. **No Hessian spectrum of
G₂ or Sp(3) at any field has been looked at by me or by any session.** (The timing runs in §6 printed sizes and run
times only.)

**Labels.**
- **PROVED:** exact symbolic, or rigorous interval.
- **COMPUTED:** double precision.
- **CONJECTURE:** stated, not shown.

## In plain words first

Test F asked which circle she needs, and she answered: **her own**. In SU(4) and Sp(2) she lives on the circle her
highest root spans (θ^∨) and just inside it. She fades toward the group's principal circle and is gone past it. I had
bet on the principal circle, and I lost.

One result in two groups is a pattern, not a law. Both of those groups were chosen knowing SU(3), and I had seen Test
D before I wrote F. So the first thing G does is the honest thing: **ask the same question in two groups I have never
computed anything in**, G₂ and Sp(3), with every rule written down first. If her own circle is really what carries
her, she should do it again. If F was a coincidence of two small groups, this is where it shows.

The second thing G does is test my newest guess, and it is a riskier one. In Sp(2) she also came back at the far wall
of the sweep, which happens to be the circle of the *short* highest root. That made me guess she lives wherever the
stretched circle is a highest root's own circle, short or long. Both new groups have a short highest root, so G puts a
circle on it:
- **in G₂ the sweep ends exactly there**, at 30°, the way Sp(2)'s ended at 45°;
- **in Sp(3) the sweep ends somewhere else**, on a wall that is *not* a highest root's circle. That gives a control:
  my guess says yes at the short root's circle and no at that wall.

I already know two things that make the second guess shaky, and I am writing them down so nobody thinks I hid them:
1. At Sp(2)'s far wall "she" is not purely the short root's twist. A diagnostic I ran tonight, after F, splits her
   state into a part that is the short root's twist (a third to a half) and a part that mixes the two long top roots.
   (Section 0.)
2. In G₂ the short highest root is not even the top root on its own circle. Two long roots tie above it. So in G₂ the
   guess is a bare geometric statement ("the short root's circle"), with no "she is that root's twist" story behind it
   at all.

So I expect H_θ to hold and I am not confident about H_s. Both get scored the same way.

## 0. What I knew when I wrote this (disclosed in full)

- **Test F, complete** (`docs/research/edge-prereg-F-results-2026-11-22.md`): every number, both groups, all 36 points,
  and both versions of the 9 wall points.
- **The far-wall diagnostic** (`scripts/edge_wp8_farwall_diagnostic.py`, receipt `farwall_diagnostic.json` in this
  folder). It is post hoc and on F's groups only. Trivial sector, x = −1.5, −1, −0.5, F's three shapes:
  - Sp(2) at 45°: her state is 32–46% short-root twist, and the rest is long–long cross states;
  - SU(4) at 45°: no twist part, only cross states;
  - at 0° in both groups: 100% her own twist.
- **Geometry and timing of G₂ and Sp(3)** (structure only, §1 and §6). The arc numbers below.
- **The fast code** (`scripts/edge_wp8_fast.py`) was checked against F's code at three F points, which I had already
  seen. That is a code check, not new data.
- **Nothing else.** No floor, margin, edge or competitor in G₂ or Sp(3).

**What is chosen knowing F.** The shapes (F's three), t = 3, the scoring rule ("lives" = at least 2 of 3 shapes), and
the arc construction are F's, unchanged on purpose, so G is a replication and not a new design. The hypotheses are
F's results turned into predictions. That is the point: F's findings are the things being tested.

## 1. The families and the circles

**Groups:** G₂ (dimension 14, rank 2) and Sp(3) (dimension 21, rank 3), built as matrix algebras exactly as in the
principal-circle survey (`scripts/arc_principal_survey.py`): G₂ as the stabiliser in so(7) of the 3-form, Sp(3) in its
6-dimensional defining representation. ⟨X, Y⟩ = −2 tr XY in the defining representation; orthonormal torus; roots are
the eigenvalues of i·ad.

**The arc**, as in F: e_K = cos a · θ^∨/|θ^∨| + sin a · e_⊥, where e_⊥ is the unit direction of ρ^∨ with its θ^∨ part
removed. a = 0 is θ^∨ (her own circle); the arc passes ρ^∨ and runs on to the first wall.

| group | circle | angle a | what it is | regular? | top root unique? |
| --- | --- | --- | --- | --- | --- |
| G₂ | **g0** | 0° | θ^∨, her own circle (a wall) | no | yes |
| G₂ | g1 | 9.55° | half way to ρ^∨ | yes | yes |
| G₂ | **g2** | 19.11° | ρ^∨, the principal circle | yes | yes |
| G₂ | g3 | 24.55° | half way from ρ^∨ to the wall | yes | yes |
| G₂ | **g4** | 30° | the wall the arc ends on, which **is θ_s^∨**, the short highest root's circle | no | no (two long roots tie; θ_s itself is below them) |
| Sp(3) | **s0** | 0° | θ^∨ (a wall) | no | yes |
| Sp(3) | s1 | 16.16° | half way to ρ^∨ | yes | yes |
| Sp(3) | **s2** | 32.31° | ρ^∨ | yes | yes |
| Sp(3) | s3 | 39.41° | half way from ρ^∨ to the wall | yes | yes |
| Sp(3) | **s4** | 46.508° | the wall the arc ends on: **not** parallel to any coroot | no | no: the **same three roots tie** as at sS (two long, and θ_s) |
| Sp(3) | **sS** | 45° from θ^∨, **off the arc** | θ_s^∨, the short highest root's circle | no | no (three roots tie, as at Sp(2)'s far wall) |

The angles g1, g3, s1, s3 are the midpoints of the two legs. The regularity and uniqueness columns are COMPUTED
(structure only, `scripts/edge_wp7_groups.py` with the survey's builders); gate G-K3 rechecks them before any data.

**Why s4 is a good control.** At s4 and at sS the top is shared by exactly the same three roots, the two long ones
and the short highest root. What differs is only that sS *is* the short root's circle and s4 is not any root's
circle. So if she lives at sS and not at s4, it is the circle that matters and not which roots tie at the top. If she
lives at both, my guess is too narrow: what matters would be the tie, not the circle. Both outcomes are scored below.

**Metric:** g = t on e_K, 1 elsewhere, **t = 3**.

**Fields (F's diagonal sub-family):** x on e_K, free; w·I on the Cartan complement; y_θ·I on the plane of the root
highest for e_K (where several tie, the first in the code's order, recorded); y·I on the other root planes.

**Shapes (w, y, y_θ):** (½, ½, ¼), (½, ½, ¾), (½, 1, 1), F's three, unchanged.

**Her state and her copies:** F's rule as finally implemented (registration, C1, Amendment 2 and its correction): the
cheapest exact state of top Q_K charge; her copies are the top-charge states tied with it (to 10⁻⁹ relative), plus
the mirror states tied with them.

## 2. Sectors

Built from the highest weights of the defining representation (`edge_wp8_fast.sector_specs`), the adjoint from the
structure constants. The highest weight is taken under the same positive functional the code uses for roots.

| group | main sectors (dimension) | truncation check |
| --- | --- | --- |
| G₂ | 1, 7 (defining), 14 (adjoint), 27 (top of Sym²7) | 77 (top of Sym³7) |
| Sp(3) | 1, 6 (defining), 14 (top of Λ²6), 21 (adjoint = Sym²6), 14′ (top of Λ³6) | 56 (Sym³6) |

**Why these.** They are the analogues of F's: the defining representation and its first symmetric and antisymmetric
pieces, plus the adjoint, up to charge two in the defining weights; the truncation check is the next step of the Sym
tower, as in F. In F the sectors that set or blocked her margin were the small ones and the adjoint-sized ones (the
receipt's `rival_above` and `blocking` fields), never the truncation sectors except once.

**What is left out, stated now.** G₂'s 64 and larger, and Sp(3)'s 64 and larger, are not computed, for cost. By the asymmetry
in §4, a "lives" verdict could in principle be undone by a missing sector; "does not live" cannot.

## 3. Measurements (F's, unchanged)

- **The floor set I** over the main sectors: a 0.25 grid on the bounding interval, brentq to 10⁻⁸ at sign changes; if
  I is empty on the grid, a golden-section search for the best margin.
- **The truncation check** at every grid x in I (or the golden point), over main + extra sectors. "Survives" = at least
  one checked x where she is below everything in the extra sectors too.
- **φ\*:** `phi_grid_best`, the best grid margin; reported at every point.
- **Rival above her edge,** its sector; the blocking sector where I is empty. Descriptive.
- **Descriptive, not scored:** at every circle where her top space is degenerate (g4, s4 if so, sS), her state is
  split across root-plane pairs in the trivial sector at x = −1, as in the diagnostic: the share that is a twist
  (same plane on both sides) and the share that is cross.

## 4. Hypotheses and predictions (fixed now)

**Score per (group, circle):** the number of shapes, out of 3, where I is nonempty and survives. **Lives:** ≥ 2.
**Does not live:** 0. **Marginal:** 1.

| hypothesis | what it says | prediction (each must hold in **both** groups unless stated) |
| --- | --- | --- |
| **H_θ** (replication of F) | she lives on her own circle | lives at g0 and at s0 |
| **H_near** (replication of F's peak) | her margin is best at or next to her own circle | on the arc, φ\* is largest at angle 0 or the first inside angle (g0/g1, s0/s1) in ≥ 2 of 3 shapes |
| **H_notP** (replication of F's refutation) | the principal circle is not where she lives | g2 and s2 are **not** "lives" (score ≤ 1) |
| **H_s** (my new guess) | she lives on a highest root's own circle, short ones included | lives at g4 **and** at sS |
| **H_s-control** (the strict form of H_s) | and not on a wall that is no highest root's circle | does **not** live at s4 (score 0); Sp(3) only |
| **H_tie** (the rival reading of Sp(2)) | what brings her back at a wall is the short root tied at the top, not its circle | lives at **both** sS and s4; Sp(3) only |

- Supported means the prediction holds as written. A marginal score in a cell the prediction needs "lives" or "does
  not live" makes that hypothesis **not decided**.
- **The strict guess** "she lives exactly on highest roots' circles" is supported only if H_s and H_s-control both
  are.
- Each group is also reported on its own. A split is a finding, not a verdict.
- **Asymmetry, as in F:** "does not live" is robust to more sectors (more sectors only shrink I); "lives" is only as
  good as the truncation check.

**G-side** (F's side clause, unchanged): at every point with I nonempty, x_hi ≤ 0, scored where r_hi > 0 and the
SU(2) calibration edge is ≤ 0; elsewhere "not informative". The calibration uses the actual angle φ between e_K and
θ^∨ (for sS this is 45°), with F's map.

## 5. Gates (all stop the run before any prediction is read)

| gate | what must hold |
| --- | --- |
| **G-K0 (code)** | the fast code reproduces F's dense code at F points (Sp(2) 0° and 45°, SU(4) 0°), every main sector: the gauge-orthogonal spectrum, λ_b, copy count and φ, to 10⁻⁹ relative. (Already run once while building the code, at 5 × 10⁻¹⁵; rerun at the start of the run.) |
| **G-K1 (F reproduced)** | one complete F point recomputed by the G driver end to end (Sp(2), 0°, (½, ½, ¼)) gives F's receipt values for I, φ\* and "survives", to 10⁻⁸ |
| **G-K2 (group data)** | brackets close to 10⁻¹³; every sector satisfies ρ([X, Y]) = [ρX, ρY] to 10⁻¹²; dimensions as in §2 |
| **G-K3 (geometry)** | the angles, regularity and top-root uniqueness of §1; θ_s^∨ is 30° from θ^∨ in G₂ and 45° in Sp(3); g4 coincides with θ_s^∨ to 10⁻⁹; s4 is not parallel to any coroot of a highest root |
| **G-S5, G-sym, G-quad, G-lin, G-mult, G-bound** | as F's gates of the same names, with F's tolerances |

## 6. Cost and runtime

Measured on the run machine (16 cores, 32 GB), one prepare (three builds) per (sector, shape), and one evaluation
at one x, with the fast code:

| sector | N | prepare | one evaluation |
| --- | --- | --- | --- |
| G₂ 1 / 7 / 14 / 27 | 196 / 1372 / 2744 / 5292 | 0.7 s / 6 s / 26 s / 125 s | 0.0 / 0.2 / 1.0 / 5.4 s |
| G₂ 77 (check) | 15092 | TIMING_77 | |
| Sp(3) sectors | | TIMING_SP3 | |

- One process; BLAS threads allowed (results are the same to rounding; F-K0-type gates are rerun under the run's
  thread setting).

- 11 circles × 3 shapes = **33 points**, all at t = 3.
- There is no grid fallback: if it is slow, it runs longer. Thresholds do not change.
- Script: `scripts/edge_wp8_two_new_groups.py`, using `scripts/edge_wp8_fast.py`.
- Receipt: `research/edge-theorem-20261122-g/wp8_two_new_groups.json`.
