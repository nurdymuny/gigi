# How I found and derived all of this: a research summary for review (17–23 November 2026)

This is the story of one week of my research, told from my own research notes, so that a reviewer working from
another machine can see how each result came to be: which question came first, what I computed, what failed, what I
wrote down before I looked, what the adversarial reviews found, and what changed because of them.

**Repository:** `nurdymuny/gigi`, branch `feat/reading-the-curve-update`. Every path below is relative to the
repository root.

**Where things live.**
- My research notes are in `docs/research/` (index: `docs/research/README.md`). Notes are never renamed or moved,
  and the registration and the results of every pre-registered test are separate files.
- Receipts (the raw outputs of the scripts) are in dated folders under `research/` (index: `research/README.md`).
- Scripts are in `scripts/`.
- Papers are in `docs/papers/`. The four that matter here:
  - *By Enough, But Not Too Much*, the edge paper (draft): `docs/papers/by-enough/paper.tex`;
  - *The Floor Theorem for the Yang–Mills Hessian at the Berger Field*, version 3 draft:
    `docs/papers/berger-spectrum/paper.tex`;
  - *Three Ways Across a Turn: Chord, Arc and Tangent*: `docs/papers/chord-arc-tangent/paper.tex`;
  - *Reading the Curve, Not Counting the Points*: `docs/papers/two-ring-pg/paper.tex`.

**Status tags.** I keep the tags the notes use, because they are the point of this document.
- **PROVED**: an exact symbolic identity, or a computer-assisted proof with rigorous (interval) error bounds. The
  19–20 November notes also say **Certified** for the interval proofs; I treat that as PROVED.
- **PROVED (T)**: used only in the edge paper. The proof runs through a code transcription of proved rows, and that
  transcription is checked in double precision, not symbolically (`docs/papers/by-enough/paper.tex`, §2,
  "What is proved, and the transcription condition (T)").
- **COMPUTED**: double precision, not certified. The 20 November notes also say **Numerical** or **Observed** for
  this.
- **CONJECTURE**: stated, not shown.
- **REFUTED / FAILED / counted against**: a claim that a pre-registered test, a proof or a verifier showed false.
- For bridges between ideas: **INCIDENCE / STRUCTURAL / RESONANCE / FELT**, and in the arc notes **IMAGINED** (an idea
  I may have but not yet believe), **QUOTED** (literature fetched, with a verbatim line) and **UNVERIFIED** (not
  fetched).

Nothing in this week's work is a statement about the quantum theory or a mass gap. Most of the fields studied are not
Yang–Mills critical points, so the spectra describe the landscape at a field, not a transition in time. The notes say
this repeatedly, and so do the papers.

---

## 1. Overview

**In plain English.** I was checking a lattice Yang–Mills force against an exact answer. The field I was using, the
Berger field, is what you get when you stretch a three-sphere along one family of circles and let the geometry itself
be the gauge field. To have an exact answer to measure against, I computed the second variation of the Yang–Mills
energy at that field (its Hessian) exactly, sector by sector. In every sector one particular disturbance turned out to
be an exact eigenvector, with a closed-form eigenvalue, and it was the cheapest disturbance in its sector. I named her
**Helike** ("she who turns"); the technical term is **the twist**.

Then I asked why she is the floor. I wrote down four simple explanations before testing each one, and all four
failed. So I stopped guessing and looked: I let the field around her vary in a two-parameter family (a strength x
along the long circle and y across it) and mapped where she is the floor. For H+GG\* she is not the floor of every sector at any x > 0 (proved); where she is the floor she sits where the
field along her own circle turns against her charge, **by enough, but not too much** (computed: one interval on every
grid; x = 0 is not settled; for the gauge-orthogonal spectrum the x > 0 exclusion is numerical). The edge of that region is an algebraic
wall, and the thresholds of my first Floor paper turned out to be exactly where the Berger field's path crosses that
wall. Then I tested the picture in other groups. In SU(3) stretched along the wrong circle she vanishes; along her own
charge circle she comes back. In SU(4) and Sp(2) she lives on the circle of her own SU(2) (the coroot of her highest
root), not on the principal circle I had bet on. A replication in G₂ and Sp(3) is half done.

**Precisely, what stands at the end of the week.**
1. **The Floor Theorem (PROVED).** At the Berger metric g = (t, 1, 1), in the sector of spin j/2, the twist
   b = f(θ̂¹ + iθ̂²) ⊗ (τ₁ + iτ₂) is an exact gauge-orthogonal eigenvector with eigenvalue
   λ_t(j) = [j(j+2) − 2(t−3)j − 2(t−2)(t+4)]/t, and it is the lowest eigenvalue of H + GG* in every sector, with
   multiplicity two, for every t ∈ [2.75, 10]. At t = 3 this is (j(j+2) − 14)/3, and on the Poincaré sphere the
   gauge-orthogonal spectrum below 142 is fully known
   (`docs/research/floor-family-squashing-2026-11-19.md`, Results 1, 3, 5, 9; `docs/papers/berger-spectrum/paper.tex`).
2. **The closed form across the family (PROVED).** For every circle-symmetric field M₀ = xτ₀, M₁ = yτ₁, M₂ = yτ₂ on
   (t, 1, 1), her eigenvalue is λ = (x + (j+2)/√t)² + 2√t·x − y², and its anatomy is exact: fibre momentum squared,
   plus a lowest-Landau-level term, plus a curvature term (`docs/research/twist-family-map-2026-11-21.md`,
   `docs/research/what-she-is-a-law-of-2026-11-21.md`).
3. **The edge (mixed).** The edge is a root of det(B_{j,q} − λ_j) = 0 for the competitor block. For the charge-0 block
   of sector 4 the determinant factors as −16384·P₁P₂, and P₂ restricted to the Berger path contains the degree-16
   polynomial of the Floor paper's threshold t_A exactly (PROVED; checked in Lean 4). The Berger path crosses the edge
   at t_A to 9 × 10⁻¹⁶ and at t* to 1.1 × 10⁻¹² (COMPUTED) (`docs/research/exact-edge-2026-11-21.md`).
4. **The region is bounded on both sides (PROVED for H + GG*), and one interval on every grid computed (COMPUTED).**
   For x > 0 high sectors always undercut her (PROVED for H + GG*); for very negative x a sector-0 state does
   (`docs/research/exact-edge-2026-11-21.md`, Correction; `docs/papers/by-enough/paper.tex`, §5).
5. **The floor on a domain (PROVED).** On D = {x ∈ [−3, −1.5], y ∈ [0.5, 3], t ∈ [3, 10]} she is the floor of every
   sector, with multiplicity two (`docs/research/exact-edge-2026-11-21.md`, last section; edge paper §6; Floor paper
   v3 draft, Theorem 7.3).
6. **Pre-registered tests.** Seven readings of "why she is the floor" or "where she lives" were written down and then
   failed or were counted against (Tests 2, 4, 5, 6; parts of A and B; C; the principal-circle bet in F). What held
   under pre-registration: her region is one interval (A, B, D), no sector 13–79 undercuts her inside it (A), she comes
   back in SU(3) along her own circle (D), the counter-rotating "sister" holds the floor for x > 0 (E), and she lives on
   her own SU(2)'s circle in SU(4) and Sp(2) (F). All COMPUTED.
7. **The basin and the arc (mixed).** When the Berger field relaxes under the Yang–Mills flow, the flow stays in her
   family, her price turns from negative to positive within flow time about 0.05 (the "door becomes a wall"), and the
   field lands on the flat field A = θ with Chern–Simons value exactly 1. On S³ the last thing settling is her
   "shadow", which an eigenvector check shows is her own state carried by the degree-one gauge change
   (`docs/research/the-basin-2026-11-22.md`, `docs/research/arc-deep-dive-2026-11-22.md`). Several imagined bridges
   (E₈ one-to-one, η-invariant, Morse strata, the principal circle) were checked against fetched literature or a
   pre-registered test and came back false as worded or downgraded.

**What a reviewer should know first.**
- The strongest results (items 1, 2, 4 for H + GG*, and 5) rest on exact algebra and interval certificates that were
  reissued twice after adversarial reviews (§2.3 below).
- Every pre-registration after Test 6 was itself put through an adversarial review before any data. Every amendment is
  dated relative to what I had seen, and four of them are not blind on some points (§2.9 lists them all).
- Test G is running. Its G₂ half is complete and reported in an interim note, unscored.

---

## 2. How it happened, day by day

### 2.1 Before the floor (17–18 November): a lattice force that needed an exact answer

**What I was doing.** My programme needs a discrete Yang–Mills force computed from edge data alone that is consistent,
stable, variational (the gradient of an energy) and faithful in time. *Reading the Curve* builds a two-ring
Petrov–Galerkin force that converges where the native Whitney force stalls (at 26% for an Abelian control, and at 25.5%
for SU(2) on a Berger metric), but it is not the gradient of an energy
(`docs/papers/two-ring-pg/paper.tex`, abstract; `docs/research/where-we-are-2026-11-19.md`). The SU(2) runs were done on
the Berger metric (3, 1, 1) on the Poincaré homology sphere S³/2I, and the lattice spectra had many negative eigenvalues
that I could not yet classify. The receipts from this phase are `research/su2-pg-stability-20261117/`,
`research/su2-flow-20261118/` and their neighbours (`research/README.md`).

**Two notes written before any measurement.**
- `docs/research/small-edges-2026-11-18.md` sets out three ideas from my Crouzeix, harmonic and graph-fold work for
  "the smaller edges" (small-scale roughness, and the smallest rim that still reads the curve). It is explicit that
  "nothing here is a result".
- `docs/research/unit-curvature-2026-11-18.md` records an idea of mine (one unit in every direction is a sphere, so the
  smallest real thing is curved) and computes one exact piece, the double-cover fold on the binary icosahedral group
  (`scripts/binary_icosahedral_fold.py`, `research/binary-icosahedral-fold-20261118/fold.json`). It says plainly:
  "No mass-gap claim is made." *Three Ways Across a Turn* is the companion geometry paper: chord, arc and tangent of a
  turn, with the chord's square as Wilson's lattice energy (`docs/papers/chord-arc-tangent/paper.tex`, abstract).

**A prediction that failed on 19 November (T1).** I predicted that Henrici's departure from normality of the linearised
force would track the growth of the two-ring force's max |Im λ| (104, 150, 418 at n = 1, 2, 3). It did not: the
relative departure *falls* (0.224, 0.168, 0.149) while max |Im λ| rises. I retracted my earlier reading that
"non-normality grows with refinement"; max |Im λ| measures spectral spread
(`docs/research/henrici-t1-results-2026-11-19.md`; `research/henrici-t1-20261119/t1.json`). COMPUTED.

### 2.2 19 November: an exact answer, and the floor appears

**The question.** My strategy note asked for an exact answer to measure the lattice against before trusting a number
(`docs/research/yang-mills-strategy-2026-11-19.md`). The Berger field is left-invariant, so its Hessian splits by
representation, and the smallest piece, the left-invariant sector, is a 9 × 9 matrix that can be computed exactly.

**What I computed** (`docs/research/su2-exact-sector-2026-11-19.md`; `scripts/ricci_su2_left_invariant_sector.py`;
`research/su2-left-invariant-sector-20261119/sector.json`):
- The nine exact eigenvalues at g = (3, 1, 1): −14/3 (×2), (29 − √1345)/3 (×2), 16 − 2√73, 8, (29 + √1345)/3 (×2),
  16 + 2√73. Both lattice forces match all nine, and their real parts converge towards them from n = 1 to n = 3.
- With the commutator dropped the sector is 4/3 (×6) and 12 (×3), all positive: **every growth direction comes from the
  field acting on itself**, not from the curved space.
- The same evening, the spin-6 sector (`scripts/ricci_su2_spin_sector.py`, `spin_sectors.json`), with two controls that
  passed (j = 0 reproduced to 8 × 10⁻¹⁵; round metric without commutator reproduces the coexact 1-form spectrum).

**The first error, caught the same day.** The note counted **five** physical growth directions. A gauge check
(`docs/research/su2-exact-sector-gauge-check-2026-11-19.md`; `scripts/ricci_su2_sector_gauge_check.py`;
`gauge_check.json`) found that three of the nine left-invariant directions are gauge (b_i = [M_i, ξ]), that the
eigenvalue 8 is pure gauge, and that the −2.558 pair is 40.5% gauge. **The physical growth directions are three:**
−14/3 (×2) and 16 − 2√73. On the gauge-orthogonal slice the sector is −14/3 (×2), 16 − 2√73, 22/3 (×2),
16 + 2√73. The correction sits at the top of the original note, which is otherwise kept as written. (At that point
22/3 was numerical to 10⁻¹⁰, "not derived symbolically"; the Floor paper later states the slice values as computed
exactly.)

**The closed form, then the gap** (addenda to the gauge-check note):
- **Closed form (PROVED, symbolic in j).** In every sector, b = f_top(θ̂¹ + iθ̂²) ⊗ (τ₁ + iτ₂) is an exact,
  gauge-orthogonal eigenvector with λ(j) = (j(j+2) − 14)/3 (`scripts/ricci_su2_sector_formula.py`,
  `sector_formula.txt`). That it is the *lowest* was at first numerical for j ≤ 40.
- **Minimality for j ≤ 120 and the gap.** A Weitzenböck form H + GG* = Σ XᵢᴴXᵢ + R₀ gives an all-j lower bound, and
  computation covers j ≤ 120 (`scripts/ricci_su2_sector_minimality.py`, `minimality.json`). On the Poincaré sphere the
  next physical value is 154/3, so the gap above the growth mode is 168/3 = 56, one third of 168, the first nonzero
  scalar eigenvalue there.
- **Literature (addenda 3 and 4).** A search found no prior publication of the closed form, the slice values, the
  Weitzenböck form with R₀ or the 168/3 gap. The nearest work is Kumar, Lechtenfeld and Picanço Costa on the *round*
  S³; my construction reproduces their meron spectrum −2, 1, 6, 7, 10 and Laquer's index 1. Bourguignon–Lawson §8 was
  behind a login, and the note says I should read it myself before deposit.

**The family across stretches** (`docs/research/floor-family-squashing-2026-11-19.md`):
- **Result 1 (PROVED, symbolic in j and t).** On every Berger sphere (t, 1, 1) the same field is an exact
  gauge-orthogonal eigenvector with λ_t(j) above (`scripts/ricci_su2_sector_formula_squashed.py`).
- **Result 2 (COMPUTED).** Below t* ≈ 2.69803 a lower eigenvalue appears, first in sector j = 4.
- **Result 3 (PROVED by interval certificate).** The floor theorem at a second stretch, t = 4
  (`scripts/ricci_su2_gap_certificates_t.py`, `gap_certificates_t4.json`). At t = 4 the sectors interleave rather than
  being separated by a gap.
- **Result 5 (PROVED by interval certificate).** For every t ∈ [2.75, 10] at once, in 15 sectors (j ≤ 12, 20, 24):
  1,091 slices, none failed, smallest margin 0.0018 (`scripts/ricci_su2_floor_interval_cert.py`,
  `interval_cert_2.75_10.json`).
- **Result 4: a prediction that failed.** I committed predictions P1–P3 before running a soft-mode flow test. With the
  operator frozen at the Berger field the twist law holds exactly (sign flips at the crossing; direction locks onto the
  twist, overlap 1.000). Under the real flow, P1 and P2 fail: the Berger field collapses towards a flat field within
  flow time ≈ 0.1 (energy 34.6 → 0.73), faster than any twist grows. The note also records that P1 "as written was
  ill-posed". The reading: the floor law is a law of the landscape, not a dynamical transition
  (`soft_mode_flow.json`).
- **Results 6–7 (COMPUTED, multistart, not a completeness proof).** The resting left-invariant fields on stretched
  spheres are two flat fields and one squashed meron, whose Morse index stays 1 with no sector going soft.
- **Result 9 (PROVED).** The floor law holds in **every** sector for every t ∈ [2.75, 10]: exact charge blocks of size
  ≤ 9 (`scripts/ricci_su2_charge_selection_symbolic.py`), interval certificates of every non-twist block for j ≤ 99
  (943 + 4,307 blocks, no failure), and a closed-form Gershgorin bound for j ≥ 100 proved by interval bisection
  (`scripts/ricci_su2_floor_large_j.py`). This is Theorem 6.6 of the Floor paper, version 2. The note is explicit about
  what is not claimed: no quantitative gap for every j, and nothing outside [2.75, 10]; t* stays numerical.

The status note that evening still put the *variational force* as "the whole open problem" and recommended pausing
side experiments (`docs/research/where-we-are-2026-11-19.md`). The floor started as a side check and became its own
line of work.

### 2.3 20 November: two adversarial reviews, and a hypothesis that could not fail

**The certificates were attacked twice and reissued twice** (`docs/research/floor-family-squashing-2026-11-19.md`,
"Reissue" and "Second reissue").
- After the first external adversarial review, every interval certificate (Table 1 at t = 3, t = 4, the range
  certificate, the block certificates j ≤ 40 and 41–99) was reissued under an outward error budget
  (`scripts/ricci_su2_outward.py`): radii about stored midpoints with outward rounding, ρ and ε accumulated in interval
  arithmetic. All passed; the t = 4 bound became ε < 8.8 × 10⁻⁵⁴.
- After the second review, the certificates bound the factorization *a posteriori* (‖M − LDLᵀ‖_F evaluated in outward
  interval arithmetic from the stored factors), so no Higham-type constant is needed. All reissued again; no failures.

**My theory, and how I turned it into tests** (`docs/research/spectral-skeleton-2026-11-20.md`). I wrote down what I
believe (§3 there, tagged Theory) and a "spectral-skeleton hypothesis": that the low spectrum of symmetric fields is
organised by extremal modes obeying exact laws. Before any test data I then wrote a pre-registration that first shows
**why the tests as I had written them could not fail**
(`docs/research/skeleton-tests-preregistration-2026-11-20.md`, §0–§1):
- "The floor persists and stays lowest under a small perturbation" holds for *any* isolated eigenvalue (Kato, Weyl,
  Davis–Kahan), skeleton or not.
- A counting argument shows that the twist is the unique state of maximal charge, so it is an exact gauge-orthogonal
  eigenvector for *any* circle-invariant background. **So exactness is automatic and is not evidence;** only being the
  floor is (design checks: `scripts/ricci_su2_skeleton_design_checks.py`).
- The note in `spectral-skeleton-2026-11-20.md` keeps the struck-out first versions of its tests visible beside the
  replacements.

**Test 2 (classification E / P / N, fixed in advance).** The scoped hypothesis: in every independent circle-invariant
family, the maximal-charge state is the floor of every listed sector on a range of positive length. The Berger family
itself is class P (code control: T_all = {2.70, …, 10.00} exactly).
- **Family 1, the squashed meron: class N.** The extremal state is the floor at no grid point in no sector; at t ≤ 1
  it is the *ceiling* of every sector (`docs/research/skeleton-test2-family1-results-2026-11-20.md`; protocol committed
  in 5063d460 before the script existed).
- **Families 2a and 2b, SU(3) with the Maurer–Cartan background: class N, both** (one 8-hour run crashed on a LAPACK
  non-convergence; the rerun used a gesvd fallback at 1 of 1,800 pairs, recorded)
  (`docs/research/skeleton-test2-family2-results-2026-11-20.md`).
- **Verdict:** the scoped skeleton hypothesis is counted against three times. Prediction G (alignment) was untested,
  because no family had a floor point.

**The wave test** (`docs/research/wave-test-preregistration-2026-11-20.md`, with Amendment 1 made after the script was
written and before any run; results `docs/research/wave-test-results-2026-11-20.md`). A tap on the frozen landscape:
causality (W1) passed at all four stretches; the late field sits in twist pairs (W3) passed; the pointwise growth rate
gate (W2, ±2%) **failed at three of four stretches** (worst +8.7% at t = 8). The note says "That is the pre-registered
outcome, and it stands."

### 2.4 21 November: four readings fail, then I look instead of guessing

**Tests 4 and 5 (each registered before any code or data).**
- **Test 4, alignment** (`docs/research/alignment-test-results-2026-11-21.md`): "a field yields along its twist when its
  field strength lines up with the twist's spin". On A = sθ, aligned at every grid point, the twist is the floor of the
  j = 0 sector only. **Class N in SU(2) and SU(3); "alignment suffices" counted against twice.** An exploratory reading
  after the result suggested she must also be *singled out* by the curvature term.
- **Test 5, stretch** (`docs/research/stretch-test-results-2026-11-21.md`): I registered that reading. At 35 points
  classified in advance as singled out, the twist is the floor of every sector at **none**. **"Singled out by the
  curvature term" is not sufficient.** The note's record table lists the three failed readings against what is proved.

**Publication and a name.** The Floor paper version 2 was published (doi:10.5281/zenodo.24412603; concept DOI
10.5281/zenodo.24386613), and I named the twist Helike (`docs/research/the-twist-a-note-2026-11-21.md`). In formal
writing the technical term stays "the twist".

**Looking instead of guessing** (`docs/research/twist-family-map-2026-11-21.md`, exploratory, not pre-registered).
- **The closed form (PROVED; numerically checked to 1.4 × 10⁻¹⁵ at 40 points):** for the circle-symmetric family
  m₀ = x e₀, m₁ = y e₁, m₂ = y e₂, λ(x, y, t, j) = (x + (j+2)/√t)² + 2√t·x − y². Lifting her from sector 0 to sector j
  costs (j² + 4j)/t + 2xj/√t, and **the second term is a discount if and only if x < 0**.
- **The map (COMPUTED, j ≤ 8, 9,065 cells):** of 3,294 cells where she is the floor of all nine sectors, every one has
  x ≤ 0. The Berger field sits on the right side at t = 3, 4, 6, and every background where she had failed (meron,
  A = sθ) has x > 0.

**Test 6, the edge test: both gates failed** (`docs/research/edge-test-preregistration-2026-11-21.md`;
`docs/research/edge-test-results-2026-11-21.md`; predictions committed in 580a170 before phase 2).
- I fitted x*(t) = c₁/t + c₂/t² at four training stretches and predicted held-out edges. **E1 failed:** interpolation
  held (6 of 6), extrapolation to t = 8 failed at two heights, where the edge went positive. **E2 failed:** the model's
  Berger crossing was 2.6315 against t* = 2.6980 (miss 0.067 > 0.05).
- Reported, not gated: at all 21 points her floor set (j ≤ 12) was a half-line [−3, x*] on a 0.05 grid. (Later this
  turned out to be an interval whose lower end lay below the scan; see below.)

**The exact edge** (`docs/research/exact-edge-2026-11-21.md`; `scripts/ricci_su2_exact_edge.py`; `exact_edge.json`).
Instead of fitting, I derived the charge-block rows in closed form in (x, y, t). The edge is where her 1 × 1 block
meets the lowest eigenvalue of the first competitor block.
- Validation (COMPUTED): blocks against full sector spectra agree to 1.5 × 10⁻¹⁵; the Berger path crosses the H + GG*
  edge at t = 2.7311222782381144 against t_A = 2.7311222782381135 (difference 9 × 10⁻¹⁶) and the gauge-orthogonal edge
  at 2.6980269405211 against t* = 2.6980269405200 (1.1 × 10⁻¹²); Test 6's 21 bisection edges are reproduced to
  ≤ 3 × 10⁻⁵.
- **The edge polynomial (PROVED):** the numerator of det(B₄,₀ − λ₄) (normalized as det(s²(B₄,₀ − λ₄))) factors over ℚ(√2, √3) into P₁ (degrees 5, 6, 12) and P₂ (7, 8, 16)
  in (x, y, √t); on the Berger path P₂ contains the degree-16 polynomial of t_A exactly, by polynomial division with zero
  remainder (`scripts/ricci_su2_edge_polynomial.py`, `edge_polynomial.txt`).
- So the Floor paper's thresholds are not special to the Berger field: they are its path's crossings of one wall.

**Her anatomy** (`docs/research/what-she-is-a-law-of-2026-11-21.md`; `scripts/ricci_su2_twist_anatomy.py`;
`twist_anatomy.json`). Four exact identities: her fibre momentum κ = (t − 2)/√t − x − j/√t (x enters by minimal
coupling); she is a lowest Landau level for every x, y, t; R₀β = (Ric⊥ − 2B)β with B = y² − 2√t·x; and
λ = κ² + (2j + t + y²) + (Ric⊥ − 2B). PROVED. The same note then states three hypotheses, H1 (the family law), H2 (her
rival is neutral and nearly blind to x) and H3 (the law carries to other groups), each with what would kill it. Its
competitor picture rests on three computed points. (Its "half-line" and "nearly blind" statements were later corrected
or counted against; the note itself was not edited. See §2.9.)

**Where I overreached, and the adversarial round that corrected me** (`docs/research/she-is-the-law-2026-11-21.md`,
"Correction"; `docs/research/edge-verification-findings-2026-11-21.md`; reports and verdicts in
`research/edge-theorem-20261121/workflow_reports.json`). I had written a one-sentence law, including the clause that on
strongly stretched spheres she wins "even when the field turns slightly with her". To turn the edge into a theorem I
organised an adversarial round: four work packages, each followed by a verifier instructed to refute it, and a
literature search that could only report sources actually fetched, with a verbatim quote, each re-fetched by a second
checker. What it found:
- **Held:** eight competitor blocks set the edge for j ≤ 12, each with an exact polynomial (WP1); an exact "iff" at the
  edge on two slices at t = 3 (WP3); a sharper large-sector bound for j ≥ 40 near the edge (WP2); and the (x, y) family
  is general up to constant gauge (WP4, PROVED). All 22 literature sources exist; the mechanism is the textbook lowest
  Landau level plus a gyromagnetic-ratio-2 term (Nielsen–Olesen 1978; Bolognesi et al. 2014); the closest prior art is
  Avramidi–Collopy 2012 on the round S³.
- **Corrected:** **the edge never goes positive.** For x > 0 a high sector always undercuts her: in block q = l the
  state of weight m = l and type (1, −1) has Rayleigh quotient minus λ equal to −8lx/√t + O(1) (PROVED, reproduced by
  two independent builders; e.g. at (0.1, √10, 10), sector 120 lies 0.58 below her). The positive edges were artefacts
  of the cut-off j ≤ 12.
- **Corrected:** **her region is an interval, not a half-line.** Her value grows like x², a sector-0 gauge-orthogonal
  state only linearly, so too strong a turn against her also loses. Every earlier scan had stopped at x = −3, which hid
  the lower end.
- **Bounded:** the all-sector edge is at most min(x*₁₂, 0); equality is a CONJECTURE for t ≥ 3 only, because near
  t = 2.01 a q = l block in sector 16 undercuts her first (refuted there by the verifier).
- **Demoted:** several of my "proved" labels became "computed" (sampled cutoffs, a ratio checked at four points).
- **Not ready:** the three drafted pre-registrations (A, B, C) needed fixes before registration (for example, her
  multiplicity is two, not one; one gate would wrongly fail at the newly found lower edge).

The sentence became: *she is the law that a field gives way first along the twist that spins with it, and does so
exactly when the field along her own circle turns against her charge — by enough, but not too much, as the shape of
space sets.* It is labelled a hypothesis in its scope (SU(2), circle-symmetric fields on the stretched spheres).

**The theorem on D (PROVED, 21 November).** On D = {x ∈ [−3, −1.5], y ∈ [0.5, 3], t ∈ [3, 10]} she is the floor of
H + GG* in every sector, with multiplicity two, and the lowest gauge-orthogonal eigenvalue
(`docs/research/exact-edge-2026-11-21.md`, last section):
- j ≥ 80: interval Gershgorin (`scripts/ricci_su2_family_large_j.py`, `family_large_j.json`);
- j < 80: a mean-value interval certificate over 1,640,935 boxes, no failure, about 64 CPU-hours
  (`scripts/ricci_su2_family_block_cert.py`, `family_block_cert_j80.json`).

D stops at x = −1.5 because plain Gershgorin needs large j near the edge (sampled cutoffs l ≈ 640, 160, 40 for
x ∈ [−2, −0.5], [−2, −1], [−3, −1.5]).

### 2.5 22 November: closing the proof pieces, and the edge paper's own review

The edge paper (*By Enough, But Not Too Much*) was put through a mathematical adversarial review: 36 findings, of which
the verifier confirmed 30 and partly confirmed 6 (`research/edge-paper-review-20261122/review.json`, with reviewer
scripts beside it). As I summarised it in `docs/research/the-two-hands-2026-11-22.md` (Part I §5): no false theorem;
a convention slip that also sits in published Floor v2 (the bracket factor, results unaffected); overclaims about which
spectrum a statement covers; and three links in the domain theorem that had been checked numerically, not proved. An
example finding (S4-1): a table caption called every positive edge an artefact "proved" in §5, but the x > 0 theorem
is proved only for H + GG*; three positive gauge-orthogonal entries are supported by computation, not proof.

The proof pieces that closed those links are in `research/edge-theorem-20261122-proofs/`:
- the Weitzenböck identity for the **whole family**, exactly (`weitzenbock_family.json`,
  `scripts/ricci_su2_weitzenbock_family_symbolic.py`);
- the rows used by the certificates proved equal, by sympy equality, to the rows of the charge-block lemma
  (`rows_transcription.json`, `scripts/ricci_su2_rows_transcription_check.py`);
- an a priori bound on the round-to-nearest final operations of the domain certificate: at most 1.64 × 10⁻¹⁵ against a
  smallest certified box bound of 1.2221 × 10⁻⁶ (`rounding_bound.md`, `rounding_bound.json`; finding S6-5). The note
  says plainly that "outward rounding" had overstated what the code does, and that the theorem does not change;
- `gershgorin_lemma.json`, `direct_rayleigh.json`, `lower_edge_closed_form.json` (I have not summarised these from a
  note; they are cited by the papers).

What remains is the transcription condition (T): the scaled near-end matrices of the slab proof and the slice
construction are checked numerically against the blocks (4.5 × 10⁻¹⁶ and 2.8 × 10⁻¹⁴), not symbolically. So the slab
theorem (j ≥ 40 near the edge) and the slice "iff" are labelled PROVED (T), while the theorem on D is PROVED
(`docs/papers/by-enough/paper.tex`, §2 "What is proved, and the transcription condition (T)", and §6). The edge paper
also points out that the smallest per-box bound of the D certificate, 1.22 × 10⁻⁶, is the slack of the last box of the
bisection, not a spectral margin; the actual margin on D is about 2.35 or more (COMPUTED).

The algebraic identities of the edge polynomial (the factorization, the Berger restriction, a root bracket of Q₁₆, and
a crossing bracket s² ∈ [2.73111, 2.73116]) and twelve identities of her price are checked in Lean 4: sixteen theorems
with no `sorry` (edge paper §4, "Lean formalisation"). The paper is explicit about what Lean does not check: that
`edgeDet` is the determinant, that the root is the edge, that the twist is the floor anywhere.

### 2.6 22 November: the pre-registered tests A to G

Tests A, B and C were drafted on 21 November, fixed after review, and registered at 232ab8d. From D on, every draft went
to an adversarial review before any data, and the count of fatal and major flaws it found is recorded in each
registration.

**Test A: the edge at 48 held-out points** (`docs/research/edge-prereg-A-results-2026-11-22.md`; registration
`research/edge-theorem-20261121/wp4_prereg_A_heldout.md`; `scripts/edge_wp4_heldout.py`; `wp4_heldout.json`).
- **A gate stopped the run first.** The known edge is a root of P₂ at all 18 known rows, but **not the largest real
  root**: P₂ has further roots where a higher eigenvalue of B₄,₀ meets hers. My registered rule was wrong. **Amendment
  1** (9098bd9), written before any held-out point was computed, selects the largest root where the *lowest* eigenvalue
  meets her. "The edge is the largest real root of P₂" is counted against, and the paper says so.
- **A1 FAILED at 4 of 38** (held at 34, to 1.35 × 10⁻¹²): three times the sector-1 block (1, −3/2) reaches her first at
  small stretch and strong cross field, which my own competitor map had already shown and I had drawn the box over
  anyway; once the cut-off sector (12, −7) sets a positive edge.
- **A2 held** (10 of 10), **A3 held** (no sector 13 ≤ j ≤ 79 undercuts her at 48 points; smallest margin +0.52),
  **A4 held** (one interval on a 0.01 grid at 48 points). COMPUTED.

**Test B: 200 random circle-symmetric fields** (`docs/research/edge-prereg-B-results-2026-11-22.md`; registration
`wp4_prereg_B_general_fields.md`; Amendment B1, 4d486f2, before any B-II point; `wp4_general_fields.json`).
- B-I (code control): every random member behaves as its reduction, worst 2.2 × 10⁻¹⁴.
- **B-half held at 193 of 193:** one interval, no holes, over a far wider box than I had scanned.
- **B-S FAILED at 129 of 193:** the rival is not nearly blind to x (median slope ratio 0.67). Neutral rivals are flat
  (median 0.35), charged rivals steep (median 0.74); the most common rival is the charged (1, ±3/2). H2's "nearly blind"
  and neutrality clauses are counted against. The edge paper adds that the 0.5 threshold was chosen after seeing three
  ratios, so a pass would have been weak evidence.

**Test C: SU(3) stretched along hypercharge, with the field along that circle freed**
(`docs/research/edge-prereg-C-results-2026-11-22.md`; registration `wp4_prereg_C_su3.md`; `scripts/edge_wp4_su3_free_fibre.py`;
`wp4_su3_free_fibre.json`).
- Two amendments, both before any floor was computed: **C1**, her value comes in multiplets of the non-abelian U(2)
  (my gate expected 1–2 copies and saw 4–10; I derived the multiplet count *after seeing the counts*, and the amendment
  says so); **C2**, other states that tie her are competitors, not copies.
- **C-P1 weak FAILED:** at none of 20 shapes is there any x where she is the floor of all nine sectors. The sentence,
  and H3 beyond SU(2), are counted against in this family. A post-hoc diagnostic, labelled as such, shows she loses in
  every nonzero sector by margins that grow with the sector.
- I wrote in that note that the natural next family stretches the circle of her own charge, and that this "would be a
  new hypothesis, chosen after this failure".

**Test D: SU(3) along her own charge circle H = diag(1, 0, −1)** (registration
`research/edge-theorem-20261122-d/wp5_prereg_D_su3_own_circle.md`, revised after a review that found one fatal and four
major flaws, registered at 8adea29; interim `docs/research/edge-prereg-D-interim-2026-11-22.md`, unscored, 11 of 20
shapes; results `docs/research/edge-prereg-D-results-2026-11-22.md`; `wp5_su3_own_circle.json`).
- D was chosen after C failed, and says so. I wrote the interim note early so I could not read the second half through
  the first.
- **D-P1 weak and strict held** (19 of 20 shapes; all 16 with t ≥ 3); **D-ext held** overall (one t = 2 shape is
  truncation-sensitive and is disclosed); **D-P2 held** (15 of 15 scorable: her region at x < 0); **D-P3 held** (one
  interval). **D-P4 FAILED** again: the charged rival (4,0) with charge −7 rides the field (ratios 0.61–0.76).
- Her x² coefficient is exactly 1 (against C's 3/4), and the lift slope is (p + q)/√t. COMPUTED.

**Test E: the two hands** (registration `research/edge-theorem-20261122-e/wp6_prereg_E_two_hands.md`, revised after a
review, registered at 8be0713; results `docs/research/edge-prereg-E-results-2026-11-22.md`; `wp6_two_hands.json`). The
idea came from §2.7 below.
- **E1 held, 60 of 60:** for x > 0 the floor eigenspace is at least half "across-reaching" (c_l·m·x < 0) from some
  j₀ ≤ 272 on, usually j = 1.
- **E3 held, 60 of 60:** the sister's impurity is the first-order mixing in her own block, measured/predicted 0.86–1.00.
- **E2 not scored:** too few eligible pairs (4 against the 5 required); descriptively the ratios are 4.41–4.61 against a
  predicted 4.
- At t = 2 a different across-reaching hand holds the floor, not the sister (as the review had predicted).

**Test F: which circle?** (registration `research/edge-theorem-20261122-f/wp7_prereg_F_principal_circle.md`, revised
after a review that found two fatal and seven major flaws, registered at 6ad473c; results
`docs/research/edge-prereg-F-results-2026-11-22.md`; `wp7_principal_circle.json`, with the first, faulty receipt kept
beside it).
- **My bet:** she lives on the group's principal circle ρ^∨ (from `docs/research/the-arc-2026-11-22.md`, Part II.1). In
  SU(3) the principal circle, the coroot θ^∨ of her highest root and a regular circle coincide, so D could not tell
  them apart. F sweeps six angles in SU(4) and Sp(2).
- **H_θ supported** in both groups (lives at θ^∨, 0°); **H_P (principal) REFUTED**; **H_P-peak REFUTED** (her margin
  peaks at 9.22° in Sp(2) and 0° in SU(4), never at 18.43°); **H_reg REFUTED** (dead at 27.65° and 36.87°); **H_C
  REFUTED in Sp(2)** (she lives at the far wall, 45°; marginal in SU(4)); **F-side held, 13 of 13**.
- A post-hoc observation (Sp(2)'s far wall is the coroot of the short highest root) was first over-stated as "the twist
  of that root's SU(2)". A diagnostic the same evening (`scripts/edge_wp8_farwall_diagnostic.py`,
  `research/edge-theorem-20261122-g/farwall_diagnostic.json`) showed her state there is only 32–46% short-root twist;
  at SU(4)'s far wall it has no twist part at all. The correction is in the F note.

**Test G: replication in two groups never computed before, G₂ and Sp(3)** (registration
`research/edge-theorem-20261122-g/wp8_prereg_G_two_new_groups.md`, revised after a review that found one fatal and
seven major problems, registered at 333d97c; interim `docs/research/edge-prereg-G-interim-2026-11-23.md`).
- The fatal problem: the truncation check omitted the very sector type (2θ_s) that set her edge in F. It is in now.
- The far-wall guess is split into three readings (the short root's circle, a chamber edge, the short root tied at the
  top), told apart by three Sp(3) circles sS, s4, s5. G₂'s far wall is scored only as a replication.
- G₂, complete but **unscored**: she lives at θ^∨ (2 of 3 shapes) and half way to ρ^∨ (2 of 3), not at ρ^∨ or beyond (0),
  and is marginal at the far wall (1 of 3, margin 0.007). Every region lies at x < 0. Her margin is highest on her own
  circle at all three shapes. Sp(3) had produced no data point when the interim note was written; the process was
  stopped after two hours with swap nearly full and restarted alone after G₂ finished. Every gate passed in G₂.
- G will be scored once, at the merge, exactly as registered.

### 2.7 22 November: the two hands

**The question** came from Vygodsky's handbook (§94, p. 129): right and left coordinate systems drawn on two hands, and
handedness defined by where the watcher stands; and from the plain fact that you reach your left shoulder more easily
with your right hand (`docs/research/the-two-hands-2026-11-22.md`, Part II).

**What is exact** (PROVED from the closed-form rows):
- the circle symmetry isolates her (unique maximal charge, so an exact eigenvector);
- her mirror (charge −(l + 2)) has exactly her value, hence multiplicity two;
- the two lifts k = ±1 are gauge-equivalent with x → −x, and at y = 0 the spectrum is even in x (B-even 3.7 × 10⁻¹⁶);
- in each row the diagonal carries a linear term +4·c_l·x·l/√t, so **the cheap lift always goes to the state whose
  internal turning opposes the field along the circle**: her for x < 0, her counter-rotating "sister" for x > 0. The
  sister's −8xl/√t is exactly the state that proves she loses for x > 0.

The note keeps its limits: the sister is not shown to *be* the floor for x > 0 (that became Test E); Landau-level and
Aharonov–Casher parallels are RESONANCE until written term by term; the historical landmarks are to be fetched before
use. Part I of the same note records what Tests A–C changed in my book draft, including that "the rival is a blind
stranger" had to go.

### 2.8 22 November: the basin, the stress tests and the arc

**The basin** (`docs/research/the-basin-2026-11-22.md`; scripts and receipts in `docs/research/basin-2026-11-22/`).
- **A correction first.** My first answer was that the Berger field is a saddle and she is the way off it. That was
  wrong: the Floor paper says the Berger field is not a Yang–Mills critical point. Her negative price is a fact about
  the landscape.
- **The flow stays in her family** (STRUCTURAL; the symmetry argument is "not written out as a proof"), so it is a
  two-number ODE. That ODE reproduces the Floor paper's full flow: energy 34.71 against 34.6 at τ = 0, 0.729 against
  0.73 at τ = 0.1 (COMPUTED).
- **The door becomes a wall** (COMPUTED): her price turns positive at flow time 0.041–0.051, after which she can only
  settle; a frozen-rate estimate says she grows by only about 1.1 before that.
- **The landing** is the flat field A = θ for every t ≠ 1 sampled (COMPUTED), where her price is (j+4)²/t, and
  (j+2)²/t at A = 0 (PROVED).
- **On the Poincaré sphere** the fall lands in the irreducible basin whose holonomy is the defining representation of
  2I (holonomy PROVED, checked to 7.5 × 10⁻¹³). There she is never the floor; the floor of each surviving sector
  j ≥ 12 is j²/t, her A = 0 price two sectors down: her "shadow" (COMPUTED to 13 digits).
- **Addendum, checked against literature I read:** the spectrum at the bottom equals Henkel and Lauret's Berger 1-form
  spectrum (sectors 0–2), the floor at the bottom on S³ is min(4/t, 4t), and her own sector-0 state is in the six-fold
  floor 4/t. This corrected an earlier sentence ("a wall, but not the floor"), which is right sector by sector but not
  for the whole spectrum. Sector 0 at A = θ is 2t + 8 − 2√(t² + 8t), the shadow of a *different* state, and it is the
  overall lowest mode in the icosahedral basin (`scripts/basin_floor_at_bottom.py`,
  `research/basin-floor-20261122/floor_at_bottom.json`).
- **Addendum, part 2:** the wrap (the degree-one gauge change) built exactly from intertwiners; it is covariant only for
  the target A = θ = (2/√t, 2) (1.7 × 10⁻¹⁵). Her A = 0 eigenvector, carried by the wrap, is an exact eigenvector at θ
  with value (s + 2)²/t; in 22 of 27 cases the eigenspace is exactly her and her mirror, and in 5 she spans two
  dimensions of a larger degenerate space. **So the shadow is her, eigenvector by eigenvector** (COMPUTED;
  `scripts/basin_shadow_eigenvector_check.py`, `research/basin-shadow-20261122/shadow_eigenvectors.json`).

**Stress tests S, S6 and S7** (each registered and pushed before its script existed; scripts and receipts in
`docs/research/stress-2026-11-22/`). All are COMPUTED for sectors j ≤ 40 with my scripts, not interval certificates.
- **S** (`docs/research/stress-tests-results-2026-11-22.md`, registered at 951f2da0): **7 held, 2 failed.**
  - **S1b FAILED:** at A = 0 on S³ she is tied for the lowest wall overall (4/t), but other states undercut her inside
    sectors 1–5 at t = 3. My prediction had come from the Poincaré run, where those sectors do not survive.
  - **S3a FAILED:** under overload I predicted her price per s² tends to (4 − 4t)/t; it tends to −y², because she climbs
    to sector j ≈ s|x|√t − 2 so that her stride cancels the belt (a post-hoc follow-up, labelled so, checks this to
    s = 1000).
  - Held: the cut lands on A = 0 and her price only rises; the flip control; she is the floor at the Berger field from
    t = 2.75 to 50 and not for t ≤ 2.5; the t = 3 plane splits into exactly two basins (820 and 861 of 1,681 starts),
    with the squashed meron on the boundary.
- **S6, the current sweep** (`docs/research/current-sweep-results-2026-11-22.md`, registered at 78414bc6): both held.
  Two script corrections are disclosed (her mirror was first compared with her; exact ties at x = 0), and the first,
  stopped run's receipt was overwritten, deleted, and the sweep rerun from scratch.
- **S7, the tear** (`docs/research/tear-test-results-2026-11-22.md`, registered at a5be2123): all five held across eight
  quotient groups. Family 2 survives exactly for the cyclic groups; a cyclic tear keeps her shadow on the basin floor,
  a non-abelian tear hands it to the state that beat her at rest.

**The arc** (`docs/research/the-arc-2026-11-22.md`). Part I is my read of where the work stands; Part II is imagination
on purpose, tagged IMAGINED, with an honest bet for each direction (the principal circle, E₈ basins, an η-invariant for
the two hands, Morse doors, the spaghetti vacuum, the epicycle and geometric phase); Part III notes old roots, tagged
RESONANCE, "from memory and must be fetched". The principal-circle bet was pre-registered as Test F, and the note now
records that it was lost.

**The arc, checked** (`docs/research/arc-deep-dive-2026-11-22.md`; receipts in `research/arc-deepdive-20261122/`). Five
literature briefs, each quoting only what was fetched, a verifier that fixed nine items, and two computations:
- **C1, the winding of the fall (PROVED / COMPUTED).** On her family CS(x, y, t) = (t x² + 2y² − √t·x y²)/4, with
  CS(θ) = 1 (checked independently by quadrature to 10 digits). The family energy has exactly one more critical pair,
  the deformed meron, at CS ≈ ½ (exactly ½ only at t = 1). The fall lands at CS = 1 at every t ≠ 1 sampled and never
  crosses ½. So **the falling field carries winding one, and the last thing settling at the bottom is her shadow**, which
  the eigenvector check says is her. "Lands on the θ side of the pass" is a fact of the two-number ODE, not yet of the
  full flow.
- **C2, the principal circle in bigger groups (structure only, no floors).** ρ^∨ and θ^∨ coincide only in SU(2) and
  SU(3) (PROVED for classical families; COMPUTED for exceptional types to rank 8). This survey fed Test F's design.
- **What was REFUTED or downgraded:**
  - "Poincaré basins line up one-to-one with the E₈ dots" is **false as written** for SU(2): only 3 of the 9 McKay
    dots are flat SU(2) connections. What replaced it: the fall lands at CS = ±1/120 mod 1 on ⟨1,1,1⟩ (PROVED up to
    global sign). Calling it "the Fintushel–Stern minimum" waits on a sign convention I could not verify.
  - The η-invariant bet: **downgraded to low.** The Hessian uses no Hodge star, so it cannot carry an η; the two hands
    are a gauge pair, not an orientation pair. A by-product: H = (⋆d_A)² + S_F and her price λ = μ² − B (identity
    PROVED; her eigenvalues of ⋆d_A and S_F COMPUTED in 36 cases).
  - "The door-to-wall crossing is a Morse stratum boundary": **fails as worded** (a flow line never crosses a stratum
    boundary, and her sign change happens at a non-critical field).
  - The principal-circle bet lost its favourite status before F was scored; F then refuted it.
  - Five of the old roots were corrected against fetched sources (for example, the Fuxi–Nüwa silk paintings are Tang, not
    Han, and "mirror image" was my phrase). One was added: Clifford himself called the left and right isometries of S³
    "twists" (QUOTED from secondary sources; the primary scan returned 403).

**The same morning I also reread four of my older papers line by line** (the Davis manifold, the spectral branch, the
Non-Decoupling theorem, the Davis Duality, and the Lift Unification; notes `docs/research/*-review-2026-11-22.md`).
These are separate from the Helike work, and `docs/research/STATUS-live.md` lists them as not yet adversarially
reviewed. I mention them only because the basin and arc notes cite the spectral branch and the field equations at
INCIDENCE or RESONANCE.

### 2.9 Disclosure ledger: amendments, things seen before registration, and corrections

This is every place where the notes disclose that a rule changed or that I had seen something before writing a rule.

| where | what was disclosed | when relative to data | source |
| --- | --- | --- | --- |
| exact sector | "five physical growth directions" was wrong; three | same day; original text kept, correction at top | `docs/research/su2-exact-sector-2026-11-19.md` |
| soft-mode flow | P1 "as written was ill-posed"; P1, P2 failed | after the run | `docs/research/floor-family-squashing-2026-11-19.md`, Result 4 |
| Floor certificates | reissued twice under stricter error budgets | after two adversarial reviews | same note, "Reissue" |
| Landau anatomy | the fibre-charge formula in Result 8 is struck out and replaced | in place | same note, Result 8 |
| skeleton tests | the tests as first written could not fail; restated | before any Test 2/3 data | `docs/research/skeleton-tests-preregistration-2026-11-20.md` §0 |
| wave test | Amendment 1 | after the script was written, before any run | `docs/research/wave-test-results-2026-11-20.md` |
| the law sentence | "she wins even when x > 0" and "half-line" were wrong | after the adversarial verification round | `docs/research/she-is-the-law-2026-11-21.md`, Correction |
| A | Amendment 1 (which root of P₂) | after the A-code-2 gate stopped the run, before any held-out point | `docs/research/edge-prereg-A-results-2026-11-22.md` |
| B | Amendment B1 (tied mirror pair); B-S threshold chosen after seeing three ratios | before any B-II point | B results note; edge paper §8 |
| C | C1 multiplet count derived *after seeing the counts*; C2 ties | before any floor or edge | `docs/research/edge-prereg-C-results-2026-11-22.md` |
| D | chosen after C failed; interim written at 11 of 20 shapes | interim unscored | D interim and results notes |
| F | registered after I had looked at 14 of D's 20 shapes (15 complete); predictions not blind to D | before F data | F registration, line 22 |
| F, Amendment 1 | the peak test was adopted after F had already printed 3 of 36 points (Sp(2), 0°); reported as not blind on those | before F finished | F results note; `docs/research/STATUS-live.md` |
| F, Amendment 2 | rule at a degenerate wall, after the run stopped at the F-mult gate | after the stop, before further data | `wp7_prereg_F_amendment2_wall.md` |
| F, implementation correction | restored rule C1 at 9 wall points; *not blind* (whole first receipt seen); changed no verdict; both versions reported | after the full first receipt | F results note |
| F far wall | "the twist of that root's SU(2)" was unchecked and not true as stated | same evening | F results note, Correction |
| G | written knowing all of F and the far-wall diagnostic; two timing runs printed the dimension of her top-charge space in some sectors before registration | before any G data | G registration §0 |
| S6 | two script corrections; stopped run's receipt deleted and the sweep rerun | before the scored run | current-sweep results note |
| basin | "she is the way off a saddle" was wrong; "a wall, but not the floor" holds per sector only | same day | the basin note |
| arc | the principal-circle bet lost; E₈ one-to-one false; η and Morse bridges weakened | after the deep dive and Test F | arc and arc deep-dive notes |

---

## 3. Results table

| result | status | where it lives |
| --- | --- | --- |
| Left-invariant sector at (3, 1, 1): 9 exact eigenvalues; lattice matches all nine | PROVED (exact); lattice match COMPUTED | `docs/research/su2-exact-sector-2026-11-19.md`; `research/su2-left-invariant-sector-20261119/sector.json`; Floor paper §3 |
| Physical growth directions are three (−14/3 ×2, 16 − 2√73), all from the commutator | PROVED (corrected from five) | `docs/research/su2-exact-sector-gauge-check-2026-11-19.md`; `gauge_check.json` |
| (j(j+2) − 14)/3 is an exact gauge-orthogonal eigenvalue in every sector | PROVED | gauge-check note, addendum; `scripts/ricci_su2_sector_formula.py`; Floor paper Prop. 4.1 |
| Floor Theorem at t = 3: floor of sectors 0, 12, 20, 24; spectrum below 142 on S³/2I; gap 168/3 | PROVED (interval certificate) | Floor paper `thm:gap`; `minimality.json` |
| λ_t(j) exact on every Berger sphere | PROVED (symbolic in j, t) | `docs/research/floor-family-squashing-2026-11-19.md`, Result 1 |
| Floor at t = 4; low spectrum on S³/2I below 136 | PROVED (interval) | same note, Result 3; `gap_certificates_t4.json` |
| Floor of every sector for every t ∈ [2.75, 10] | PROVED (charge blocks + interval certificates j ≤ 99 + Gershgorin j ≥ 100) | same note, Result 9; Floor paper v2 Thm. 6.6 |
| t* ≈ 2.698 threshold (gauge-orthogonal) | COMPUTED | same note, Result 2; exact-edge note |
| Twist law holds at the frozen Berger field; real flow collapses first | COMPUTED; P1, P2 FAILED | same note, Result 4; `soft_mode_flow.json` |
| Resting left-invariant fields: two flat + squashed meron, index 1, no soft mode | COMPUTED (multistart) | same note, Results 6–7 |
| The twist is a lowest Landau level | PROVED (Berger path; and across the family) | Floor paper Prop. 6.2; `landau_symbolic.txt`; `twist_anatomy.json` |
| Exactness of the extremal mode is automatic for any circle-invariant background | PROVED (counting argument) | `docs/research/skeleton-tests-preregistration-2026-11-20.md` §1 |
| Scoped skeleton hypothesis (symmetry makes her the floor) | REFUTED (three families N) | Test 2 notes, 20–21 Nov |
| Wave test W2 (late growth rate ±2%) | FAILED at 3 of 4 stretches | `docs/research/wave-test-results-2026-11-20.md` |
| "Alignment suffices" | REFUTED (SU(2), SU(3) class N) | `docs/research/alignment-test-results-2026-11-21.md` |
| "Aligned and singled out suffices" | REFUTED (0 of 35) | `docs/research/stretch-test-results-2026-11-21.md` |
| Closed form λ = (x + (j+2)/√t)² + 2√t·x − y² across the family; lift discount iff x < 0 | PROVED | `docs/research/twist-family-map-2026-11-21.md`; edge paper `prop:closed`, `cor:lift` |
| Anatomy λ = κ² + (2j + t + y²) + (Ric⊥ − 2B), minimal coupling of x | PROVED | `docs/research/what-she-is-a-law-of-2026-11-21.md` §2; edge paper `prop:anatomy` |
| Edge model x* = c₁/t + c₂/t² (Test 6, E1 and E2) | FAILED | `docs/research/edge-test-results-2026-11-21.md` |
| Berger path crosses the edge at t_A (9 × 10⁻¹⁶) and t* (1.1 × 10⁻¹²) | COMPUTED | `docs/research/exact-edge-2026-11-21.md`; `exact_edge.json` |
| det(s²(B₄,₀ − λ₄)) = −16384·P₁P₂ (the normalized determinant; it differs from the unscaled one by s¹⁸); P₂ on the Berger path contains Q₁₆ (t_A) | PROVED; Lean 4 | exact-edge addendum; `edge_polynomial.txt`; edge paper §4 |
| "The edge is the largest real root of P₂" | REFUTED | Test A note; edge paper §1 |
| Family (x, y) is general up to constant gauge | PROVED | edge paper `prop:general`; `docs/research/edge-verification-findings-2026-11-21.md` |
| Weitzenböck identity and charge blocks, whole family | PROVED (symbolic) | `research/edge-theorem-20261122-proofs/weitzenbock_family.json`; edge paper `lem:weitz`, `lem:blocks` |
| For x > 0 she is never the floor of every sector | PROVED for H + GG*; COMPUTED for the gauge-orthogonal spectrum | exact-edge Correction; edge paper `thm:xpos` |
| Region bounded below (sector-0 state; cubic bound) | PROVED | edge paper `prop:lower`, `prop:lowblock` |
| Region is one interval | COMPUTED (A: 48 points; B: 193; D: every nonempty shape) | Tests A, B, D notes |
| All-sector edge = min(x*₁₂, 0) | CONJECTURE for t ≥ 3; REFUTED near t = 2 | exact-edge Correction; edge paper `conj:edge` |
| Exact "iff" on two slices at t = 3, j ≤ 12 | PROVED (T) | edge paper `thm:slice` |
| Every sector j ≥ 40 on slabs near the edge | PROVED (T) | edge paper `thm:slabs` |
| Theorem on D: floor of every sector, multiplicity two | PROVED | exact-edge note, last section; edge paper `thm:domain`; Floor v3 draft Thm. 7.3; `family_large_j.json`, `family_block_cert_j80.json`, `rounding_bound.md` |
| A1: edge is the P₂ root with competitor (4, 0) at 38 points | FAILED at 4 of 38 (held at 34) | `docs/research/edge-prereg-A-results-2026-11-22.md` |
| A3: no sector 13–79 undercuts her inside her region | COMPUTED, held 48 of 48 | same |
| H2: the rival is neutral and nearly blind to x | REFUTED (B-S 129 of 193; D-P4) | Test B and D notes |
| C: she is the floor somewhere in SU(3) stretched along hypercharge | FAILED (0 of 20 shapes) | `docs/research/edge-prereg-C-results-2026-11-22.md` |
| D: she is the floor in SU(3) stretched along her own circle | COMPUTED, held (19 of 20; 16 of 16 at t ≥ 3) | `docs/research/edge-prereg-D-results-2026-11-22.md` |
| E: for x > 0 an across-reaching hand holds the floor; sister's impurity first-order | COMPUTED, held (60 of 60); onset scaling not scored | `docs/research/edge-prereg-E-results-2026-11-22.md` |
| Cheap lift reaches across: discount ⇔ c_l·x < 0 | PROVED from the rows; the "purpose" reading STRUCTURAL | `docs/research/the-two-hands-2026-11-22.md` |
| F: she lives on θ^∨ in SU(4) and Sp(2) | COMPUTED, supported | `docs/research/edge-prereg-F-results-2026-11-22.md` |
| F: she lives on the principal circle; her margin peaks there; any regular circle works | REFUTED | same |
| G: replication in G₂ and Sp(3) | running; G₂ interim, unscored | `docs/research/edge-prereg-G-interim-2026-11-23.md`; `wp8G_g2.json` |
| Flow from the Berger field stays in her family | STRUCTURAL (not written as a proof) | `docs/research/the-basin-2026-11-22.md` |
| Door becomes a wall at flow time 0.04–0.05; landing on A = θ | COMPUTED | same; `docs/research/basin-2026-11-22/` |
| On S³/2I the fall lands in the irreducible (2I-holonomy) basin | landing COMPUTED; holonomy PROVED | same |
| The shadow at A = θ is her own A = 0 state carried by the wrap | COMPUTED (eigenvectors, 22 of 27 exact) | basin note, addendum part 2; `research/basin-shadow-20261122/` |
| CS on her family; CS(θ) = 1; fall carries winding one | PROVED (formula); COMPUTED (fall) | `docs/research/arc-deep-dive-2026-11-22.md` C1; `cs_fall.json` |
| On S³/2I the landing has CS = ±1/120 mod 1, on ⟨1,1,1⟩ | PROVED up to global sign; "minimum" conditional | arc deep dive II.2 |
| Poincaré basins ↔ E₈ dots one-to-one (SU(2)) | REFUTED as written | arc deep dive II.2 |
| Two hands carry an η-invariant | not supported (bet low) | arc deep dive II.3 |
| Door-to-wall is a Morse stratum boundary | REFUTED as worded | arc deep dive II.4 |
| H = (⋆d_A)² + S_F; her price λ = μ² − B | identity PROVED; eigenvalues COMPUTED | arc deep dive II.3 |
| Stress S (7 held, 2 failed), S6 (2 held), S7 (5 held) | COMPUTED, j ≤ 40 | stress, current-sweep, tear results notes |
| Henrici departure tracks max \|Im λ\| (T1) | FAILED; earlier reading retracted | `docs/research/henrici-t1-results-2026-11-19.md` |

---

## 4. What is still open

**The theorems.**
- **The all-sector edge as a theorem:** the conjecture edge = min(x*₁₂, 0) for t ≥ 3; the "iff" in every sector on
  (−1.5, x*]; sectors j > 12 on the slices; x ∈ (−0.02, 0) outside the slab theorem; small sectors j < 40 on the slab
  boxes (edge paper §9, item 1).
- **The transcription condition (T):** a symbolic check of the near-end matrices and the slice construction would turn
  the two PROVED (T) results into PROVED (edge paper §9, item 2).
- **The gauge-orthogonal edge in exact form**, and a gauge-orthogonal version of the x > 0 theorem (same item 1).
- **Near t = 2** the all-sector edge is unknown; t* itself is numerical (Floor note Result 9; edge paper §9, item 3).
- **The lower edge:** its closed-form cubic is checked at 48 points, not proved over a domain, and at some Test B fields
  blocks of sectors 1–3 set it. **The one-interval structure is computed, not proved** (edge paper §9, item 4).
- **Fields that mix sectors** (circle-invariant but not left-invariant, non-constant lifts): there her exactness is
  itself open (edge paper §9, item 5).
- **No quantitative gap above the floor for every j** in the Floor law (Floor note Result 9, "Not claimed").
- **Minimality beyond j = 120** at t = 3 was left open in the first argument; the later block-certificate route covers
  every sector on [2.75, 10].

**Other groups.**
- **Test G is running.** Sp(3) will separate three readings of the far-wall return (the short root's circle, a chamber
  edge, the short root tied at the top). Nothing is scored until the merge.
- Why she returns at Sp(2)'s far wall is open; the diagnostic shows a mixed state there.
- Tests C, D, F and G use thin slices of the left-invariant fields commuting with the circle (4 parameters out of, e.g.,
  76 in SU(5)) (arc deep dive C2).

**The two hands.** A proof of the asymptotic cross-reach for all l beyond an explicit l₀ "looks within reach" of the
Gershgorin machinery but is not done; E2's onset scaling is unscored (Test E note).

**The basin and the arc.**
- Prove that the flow stays in the family, so the winding-one answer holds for the full flow (arc deep dive, next steps
  6).
- The full Yang–Mills Morse index of the deformed meron, and a proof that its family index is 1 for all t (next steps 4).
- The spectral flow of the Hessian along the fall, with her share (next steps 5).
- The sign convention behind "the τ(P) minimum" on the Poincaré sphere (next steps 3).
- Whether any fall reaches the 2′ basin (CS 49/120) (arc deep dive II.2).
- Fetch-before-citing list: Nielsen–Olesen, Savvidy, Berry, Pancharatnam, Foucault, Kostant 1959, Atiyah–Bott 1983,
  Aharonov–Casher, and the remaining historical sources (arc deep dive, next steps 7 and 9; the two-hands ledger).
- Drawer: E₈ for SU(N ≥ 3); the spaghetti vacuum.

**The physics.** Nothing here is about dynamics beyond the computed flows, the quantum theory, or a mass gap. The
original critical path, a variational lattice force, was still open on 19 November
(`docs/research/where-we-are-2026-11-19.md`); this document does not report progress on it.

**Reviews still owed.** `docs/research/STATUS-live.md` lists the five reread notes of my older papers (and the Davis
Manifold second-edition draft) as written but not yet adversarially reviewed; the instruction there is that the
findings come first and the papers are revised only after.
