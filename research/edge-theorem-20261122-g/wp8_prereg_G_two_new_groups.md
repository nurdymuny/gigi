# Pre-registration G: does she follow her own circle in two groups I have never looked at? (22–23 November 2026)

**Registered** (this commit), before any floor set, edge, margin or competitor of this test has been computed.

**Revision after review, before any data.** The first draft went to an adversarial review. It found one fatal problem
and seven major ones, and every one is fixed here. The full list and the fixes are at the end (§7). The biggest:
- **Fatal:** the truncation check left out the very sector type that set her edge in F (2θ_s). It is in now, with
  a check deep enough at every circle.
- **Major:**
  - my control for the short-root guess could not tell "the short root's circle" from "a chamber edge". A new circle,
    s5, splits them;
  - G₂'s far wall cannot test the short-root guess at all; it is now an honest replication of Sp(2)'s far wall;
  - several hypotheses could have passed vacuously; that is closed;
  - the new code had no independent check in the new groups; it has two now.

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
her, she should do it again. If F was a coincidence of two small groups, this is where it shows. And if she lives
*everywhere* on the arc, that is not a replication either, and G says so.

The second thing G does is test my newest guess, and it is riskier. In Sp(2) she also came back at the far wall of
the sweep, which happens to be the circle of the *short* highest root. That made me guess she lives wherever the
stretched circle is a highest root's own circle, short or long. The review showed me there are at least two other
readings of that one Sp(2) point:
- **the chamber edge:** she comes back on the corner rays of the chamber, whatever root they belong to;
- **the tie:** she comes back wherever the short root is tied for the top, whatever the circle.

Sp(3) can tell all three apart, with three circles:

| reading | at sS (the short root's circle, an edge) | at s4 (a face; same roots tied as sS) | at s5 (an edge, no root's circle) |
| --- | --- | --- | --- |
| **short root's circle** (my guess) | lives | does not | does not |
| **chamber edge** | lives | does not | lives |
| **short root tied at the top** | lives | lives | lives |

G₂ cannot test my guess. On the short root's circle in G₂, the short root is not even at the top; two long roots tie
above it. So G₂'s far wall is just a replication of Sp(2)'s far-wall return in another rank-2 group, and I score it as
that.

I already know two things that make the short-root guess shaky, and I am writing them down so nobody thinks I hid
them:
1. At Sp(2)'s far wall "she" is not purely the short root's twist. A diagnostic I ran after F splits her state into a
   part that is the short root's twist (a third to a half) and a part that mixes the two long top roots (§0).
2. In F, the field y_θ at Sp(2)'s far wall sat on the short root's plane, because the code breaks the tie that way.
   The short-twist part of her state sits exactly on that plane. So part of what I saw there may come from where y_θ
   was placed (§0, §1).

So I expect H_θ to hold, and I am not confident about the far walls. Every arm is scored the same way.

## 0. What I knew when I wrote this (disclosed in full)

- **Test F, complete** (`docs/research/edge-prereg-F-results-2026-11-22.md`): every number, both groups, all 36 points,
  both versions of the 9 wall points, and the receipt's rival and blocking sectors.
- **F's rival was 2θ_s.** In Sp(2) the sector that set her upper edge was "14" (= 2ω₂ = 2θ_s) at 6 of the 8 points
  where she lived. At the far wall the margin-setting check sector was "16" (= ω₁ + ω₂). The first draft left the
  analogues of both out of G; this revision puts them in (§2).
- **The far-wall diagnostic** (`scripts/edge_wp8_farwall_diagnostic.py`, receipt `farwall_diagnostic.json` in this
  folder), post hoc, on F's groups only. Trivial sector, x = −1.5, −1, −0.5, F's three shapes:
  - Sp(2) at 45°: her state is 32–46% short-root twist, and the rest is long–long cross states;
  - SU(4) at 45°: no twist part, only cross states;
  - at 0° in both groups: 100% her own twist.
  - **In F, y_θ at Sp(2)'s 45° wall sat on the short root θ_s** (`top_plane_used_for_y_theta` = the (−½, −½) plane),
    by the code's tie-break. The short-twist share sits on that plane.
- **Two timing runs on G's groups** (`scripts/edge_wp8_timing.py`, log `timing.log`, and `scripts/edge_wp8_reach.py`).
  They built G points in memory: g0 and s0, shape (½, ½, ¼), x = −1.2, every main sector. They **printed sizes, times,
  block counts and the dimension of her top-charge space in each sector**:
  - g0: 1, 2, 1, 3, 4 in sectors 1, 7, 14, 27, 77;
  - s0: 1, 1, 4 in sectors 1, 6, 14.

  They printed no eigenvalue, margin or spectrum. A third timing run of the check evaluator printed times only.
- **Structure of G₂ and Sp(3)** (geometry, ties, zero roots, y_θ placement, and each sector's reach), all in §1 and
  §2.
- **The fast code** (`scripts/edge_wp8_fast.py`):
  - its block evaluator was checked against F's dense code at three F points (already-public data), agreeing to
    5 × 10⁻¹⁵;
  - its matrix-free check evaluator was developed and checked **only on F's groups**, agreeing with the block
    evaluator to 4 × 10⁻¹⁴.
- **Nothing else.** No floor, margin, edge or competitor in G₂ or Sp(3).

**What is chosen knowing F.**
- The shapes (F's three), t = 3, the scoring rule ("lives" = at least 2 of 3 shapes), and the arc construction are
  F's, unchanged on purpose, so G is a replication and not a new design.
- The hypotheses are F's results turned into predictions. That is the point.
- The check sectors were chosen knowing F's rivals, and **toward** including them. That makes "lives" harder to earn.

## 1. The families and the circles

**Groups:** G₂ (dimension 14, rank 2) and Sp(3) (dimension 21, rank 3), built as matrix algebras exactly as in the
principal-circle survey (`scripts/arc_principal_survey.py`): G₂ as the stabiliser in so(7) of the 3-form, Sp(3) in its
6-dimensional defining representation. ⟨X, Y⟩ = −2 tr XY in the defining representation; orthonormal torus; roots are
the eigenvalues of i·ad. (The normalisation differs between the groups, |θ|² = ½ in G₂ and 1 in Sp(3). The verdicts do
not depend on it; φ\* is compared only within a group.)

**The arc**, as in F: e_K = cos a · θ^∨/|θ^∨| + sin a · e_⊥, where e_⊥ is the unit direction of ρ^∨ with its θ^∨ part
removed. a = 0 is θ^∨ (her own circle); the arc passes ρ^∨ and runs on to the first wall. Off the arc, Sp(3) gets two
more circles.

| group | circle | angle from θ^∨ | what it is | zero roots | tied at the top | y_θ on |
| --- | --- | --- | --- | --- | --- | --- |
| G₂ | **g0** | 0° | θ^∨, her own circle (a wall) | 1 | θ alone | θ |
| G₂ | g1 | 9.553° | half way to ρ^∨ | 0 | θ alone | θ |
| G₂ | **g2** | 19.107° | ρ^∨, the principal circle | 0 | θ alone | θ |
| G₂ | g3 | 24.553° | half way from ρ^∨ to the wall | 0 | θ alone | θ |
| G₂ | **g4** | 30° | the arc's end wall, which is θ_s^∨ (a corner ray); θ_s is **not** at the top here | 1 | two long roots | the other long root (Weyl-equivalent to θ here) |
| Sp(3) | **s0** | 0° | θ^∨ (a corner ray) | 4 | θ alone | θ |
| Sp(3) | s1 | 16.156° | half way to ρ^∨ | 0 | θ alone | θ |
| Sp(3) | **s2** | 32.312° | ρ^∨ | 0 | θ alone | θ |
| Sp(3) | s3 | 39.410° | half way from ρ^∨ to the wall | 0 | θ alone | θ |
| Sp(3) | **s4** | 46.508° | the arc's end wall: a **face**, parallel to no coroot | 1 | two long, and θ_s | θ_s |
| Sp(3) | **sS** | 45° (off the arc) | θ_s^∨, the short highest root's circle: a **corner ray** (ω₂^∨) | 2 | two long, and θ_s (the same three as s4) | θ_s |
| Sp(3) | **s5** | 54.736° (off the arc) | ω₃^∨: a **corner ray**, parallel to no coroot | 3 | three long and three short (θ_s among them) | θ_s |

All COMPUTED from root data (structure only), and rechecked by gate G-K3 before any data.

Notes, so nothing is hidden:
- **s0 is far more singular** than F's 0° walls: 4 roots pair to zero there, against 1 at F's 0° walls and at g0. A
  failure of H_θ in Sp(3) alone would be confounded with that.
- **Sp(3)'s first inside angle (16.16°) is close to F's ρ^∨ angle (18.43°).** H_near is about the *fraction of the
  way* to ρ^∨ (the first inside point is half way, as in F), not the absolute angle.
- **y_θ at the walls is pinned** to the placements in the table and gated (G-K3). At s4, sS and s5 it sits on θ_s, as
  it did at Sp(2)'s far wall in F. So the far-wall arms replicate F's placement. They do not test a different one.

**Metric:** g = t on e_K, 1 elsewhere, **t = 3**.

**Fields (F's diagonal sub-family):** x on e_K, free; w·I on the Cartan complement; y_θ·I on the plane in the table;
y·I on the other root planes.

**Shapes (w, y, y_θ):** (½, ½, ¼), (½, ½, ¾), (½, 1, 1), F's three, unchanged.

**Her state and her copies:** F's rule as finally implemented (registration, C1, Amendment 2 and its correction): the
cheapest exact state of top Q_K charge; her copies are the top-charge states tied with it (to 10⁻⁹ relative), plus
the mirror states tied with them.

## 2. Sectors

Built from the highest weights of the defining representation (`edge_wp8_fast.sector_specs`); the adjoint from the
structure constants. Each sector's highest weight is gated (G-K2) by its top charge, not just its dimension, because
dimensions collide (Sp(3)'s 14 and 14′; G₂'s 77 and 77′).

| group | main sectors | truncation check |
| --- | --- | --- |
| G₂ | 1, 7 (ω₁ = θ_s), 14 (adjoint, ω₂ = θ), 27 (2ω₁ = 2θ_s) | 77 (3ω₁), 64 (ω₁+ω₂), **77′ (2ω₂ = 2θ)** |
| Sp(3) | 1, 6 (ω₁), 14 (ω₂ = θ_s), 21 (adjoint, 2ω₁ = θ), 14′ (ω₃) | 56 (3ω₁), 64 (ω₁+ω₂), **90 (2ω₂ = 2θ_s)**, 126 (4ω₁) |

**Reach of the check.** For each sector, its largest charge under e_K, divided by q_K (COMPUTED, structure only):

| | g0 | g1 | g2 | g3 | g4 | s0 | s1 | s2 | s3 | s4 | sS | s5 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| deepest check sector | 2.00 | 2.19 | 2.40 | 2.53 | 2.67 | 2.00 | 2.00 | 2.00 | 2.00 | 2.00 | 2.00 | 2.00 |

The check now reaches at least 2·q_K at every circle, as F's Sp(2) check did at 0°.

**What is still left out:** every irreducible not listed, including G₂'s 182 (4ω₁) and the larger Sp(3) ones. By the
asymmetry in §4, "lives" could in principle still be undone by a missing sector; "does not live" cannot.

**How the check sectors are evaluated.** They are too big for block methods (N up to 55,566). So they are evaluated
matrix-free, on the exact gauge-orthogonal slice:
- the operator is A = P H P + β(1 − P), where P projects onto ker G\* minus her copies;
- the slice projector uses G\*G, which is small;
- the lowest eigenvalues come from a Lanczos solve (`edge_wp8_fast.check_margin`).

Gate G-X checks this against the block method inside the run, in G's own groups (§5).

## 3. Measurements (F's)

- **The floor set I** over the main sectors: a 0.25 grid on the bounding interval, brentq to 10⁻⁸ at sign changes; if
  I is empty on the grid, a golden-section search for the best margin.
- **The truncation check** at the grid x in I (or the golden point), over main + check sectors. "Survives" = at least
  one checked x where she is below everything in every sector.
  - **Implementation note:** the checked x are taken in order of decreasing main-sector margin, and the check stops
    at the first x that survives. Within an x it stops at the first sector that blocks her. "Survives" is exactly
    F's definition; only the number of evaluations changes. Every evaluation made is recorded.
- **φ\*:** `phi_grid_best`, the best grid margin, reported at every point.
- **Rival above her edge**, its sector; the blocking sector where I is empty. Descriptive.
- **Descriptive, not scored:** at every circle where her top space is degenerate (g4, s4, sS, s5), her state is split
  across root-plane pairs in the trivial sector at x = −1, as in the diagnostic: the share that is a twist (same plane
  on both sides) and the share that is cross.

## 4. Hypotheses and predictions (fixed now)

**Score per (group, circle):** the number of shapes, out of 3, where I is nonempty and survives. **Lives:** ≥ 2.
**Does not live:** 0. **Marginal:** 1.

**The replication of F (each per group, G₂ and Sp(3)):**

| hypothesis | prediction |
| --- | --- |
| **H_θ** | lives at her own circle (g0; s0) |
| **H_notP** | H_θ holds **and** the principal circle (g2; s2) is not "lives" (score ≤ 1). If she lives nowhere, H_notP is not supported. |
| **H_near** | on the arc, φ\* is largest at angle 0 or the first inside angle in ≥ 2 of 3 shapes. A shape counts only if it has at least one finite φ\* and its top two values differ by more than 10⁻⁶·max(1, \|φ\*\|); otherwise it counts against. |
| **F-pattern** | H_θ ∧ H_notP ∧ H_near, in that group |

- **F replicates** in a group if F-pattern holds there. "She lives at all five arc circles" is recorded as
  **non-replication** (her own circle is then not special), even though H_θ would hold.

**The far walls:**

| hypothesis | prediction |
| --- | --- |
| **H_wall2** (G₂ only) | lives at g4: Sp(2)'s far-wall return replicated in another rank-2, non-simply-laced arc |
| **H_s** (Sp(3) only; my guess) | lives at sS, does not live at s4, does not live at s5 |
| **H_edge** (Sp(3) only) | lives at sS and at s5, does not live at s4 |
| **H_tie** | lives at sS, s4 and s5, and does **not** live at g4 (the short root is not at the top there) |

- They are mutually exclusive by construction; at most one can be supported.

**How a hypothesis is decided:**
- It is **supported** if every cell it names has the verdict it needs.
- It is **refuted** if some cell has the opposite verdict ("lives" where it needs "does not live", or the reverse).
- Otherwise (a marginal score in a needed cell, and no contradiction) it is **not decided**.
- **Asymmetry, as in F:** "does not live" is robust to more sectors; "lives" is only as good as the truncation check.

**G-side** (F's side clause): at every point with I nonempty, x_hi ≤ 0, scored where r_hi > 0 and the SU(2)
calibration edge is ≤ 0; elsewhere "not informative". The calibration uses:
- the actual angle φ between e_K and θ^∨ (45° at sS, 54.736° at s5);
- the field actually on θ's plane (y_θ where θ carries it; y at s4, sS and s5, where y_θ sits on θ_s).

## 5. Gates (all stop the run before any prediction is read)

| gate | what must hold |
| --- | --- |
| **G-K0 (code)** | the block code reproduces F's dense code at F points (Sp(2) 0° and 45°, SU(4) 0°), every main sector: the gauge-orthogonal spectrum, λ_b, copy count and φ, to 10⁻⁹ relative. Rerun at **every** start, including resumes |
| **G-K1 (F reproduced)** | one complete F point recomputed by the G driver end to end (Sp(2), 0°, (½, ½, ¼)) gives F's receipt values for I, φ\* and "survives", to 10⁻⁸. Rerun at every start |
| **G-K2 (group data)** | brackets close to 10⁻¹³; every sector satisfies ρ([X, Y]) = [ρX, ρY] to 10⁻¹²; dimensions as in §2; **each sector's top charge under e_K equals the reach in §2**, to 10⁻⁹ |
| **G-K3 (geometry)** | the angles, zero-root counts, tie sets and y_θ placements of §1; g4 parallel to θ_s^∨ and sS parallel to θ_s^∨, to 10⁻⁹; s4 and s5 parallel to no coroot; the tie sets at s4 and sS equal |
| **G-B (independent block check)** | at every circle, for the largest main sector and the adjoint: on three random charge blocks and her top columns, at three x, the block code's U\*HU equals U\*(Hv) from the matrix-free H to 10⁻¹⁰ relative |
| **G-X (check evaluator)** | at every circle, for the largest main sector, at the first checked x (or x = −1.2 if none): the matrix-free margin equals the block margin to 10⁻⁹ relative |
| **G-S5** | at every circle, for one main sector: the quadratic pieces against a **direct** matrix-free H at three x (probe vectors), to 10⁻¹²; and the check-sector x-pieces likewise |
| **G-sym** | commutation of Q with H on random probes, ≤ 10⁻¹⁰ relative |
| **G-quad, G-lin, G-mult, G-bound** | as F's gates of the same names, with F's tolerances |
| **G-gauge** | her state is gauge-orthogonal: \|G\*v\| ≤ 10⁻⁹·max(1, \|λ\|), at the G-mult x values and in the check sectors |

## 6. Cost and runtime

Measured on the run machine (16 cores, 32 GB):

| sectors | N | cost |
| --- | --- | --- |
| G₂ main 1 / 7 / 14 / 27 | 196 / 1372 / 2744 / 5292 | prepare 0.7 / 6 / 26 / 125 s; evaluate 0.0 / 0.2 / 1.0 / 5.4 s per x |
| Sp(3) main 1 / 6 / 14 | 441 / 2646 / 6174 | prepare 6 / 69 / 335 s; evaluate 0.0 / 1.3 / 11 s per x |
| check sectors (matrix-free) | 12,544 – 55,566 | G₂ 77 (N 15,092): first evaluation 159 s (it builds the cached x-pieces and the charge basis), then 47 s per x; 8.6 GB. The Sp(3) check sectors are larger and slower in proportion. |

- Two processes, one per group (`... OUT.json g2` and `... OUT.json sp3`), each running every gate itself, scored together by `--merge`. BLAS threads allowed. Results are the same to rounding, and G-K0 and G-K1 run under the run's own
  thread setting.
- 12 circles × 3 shapes = **36 points**, all at t = 3.
- There is no grid fallback: if it is slow, it runs longer. Thresholds do not change.
- Script: `scripts/edge_wp8_two_new_groups.py`, using `scripts/edge_wp8_fast.py`.
- Receipt: `research/edge-theorem-20261122-g/wp8_two_new_groups.json`.

## 7. The review, and what changed

| finding | fix |
| --- | --- |
| **F1 (fatal):** F's margin-setting sectors (2θ_s, ω₁+ω₂) had no analogue in G; §2's justification was wrong | added G₂ 64, 77′ and Sp(3) 64, 90, 126 as check sectors, evaluated matrix-free; §2 rewritten |
| **M1:** uneven, shallow check | reach ≥ 2·q_K at every circle (table in §2) |
| **M2:** s4 confounded with "chamber edge" | added s5 = ω₃^∨; three readings, three predictions; zero roots recorded |
| **M3:** g4 cannot test H_s | g4 is now H_wall2, a replication; H_s is Sp(3) only; H_tie predicts "does not live" at g4 |
| **M4:** vacuous passes | H_notP needs H_θ; one contrast for each far-wall reading; H_near tie and finiteness rule; F-pattern; "lives everywhere" = non-replication |
| **M5:** circular G-S5; no independent check in the new groups | G-S5 against a direct matrix-free H; new gates G-B and G-X |
| **M6:** y_θ placement unpinned; F's placement undisclosed | pinned and gated (§1); F's placement disclosed (§0) |
| **M7:** disclosure gaps | timing printouts disclosed; timing scripts and log committed |
| m1 decide() text vs code | text now matches the code's rule |
| m2 tie set and y_θ not gated | in G-K3 |
| m3 resume skipped gates | G-K0 and G-K1 run at every start |
| m4 gstar not gated | G-gauge |
| m5 s0 more singular | stated in §1 |
| m6 H_near scaling | stated in §1 |
| m7 calibration field at s4, sS | uses the field on θ's plane |
| m8 sector identity | top-charge gate in G-K2 |
| m9 cost placeholders | filled in §6 |
| m10 block rounding | negligible, as the review says; unchanged |
| m11 adjoint path untested | exercised by G-B |
