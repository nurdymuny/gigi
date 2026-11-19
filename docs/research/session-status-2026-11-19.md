# Status, 19 November 2026: what is done, what is running, what is open

Written for review on another machine. Each experiment lives on its own branch with its spec
committed before measurement; nothing below is merged into `feat/reading-the-curve` except where
noted. Bee's decisions of the day: the Davis–Wilson force **must be variational**; the strong-field
test is **deterministic in GIGI first**; a **constrained recovery** is acceptable for the
variational force.

## Branches at a glance

| branch | what | state | top commit |
| --- | --- | --- | --- |
| `feat/reading-the-curve` | papers, Russian edition, Zenodo files, S3b, exact derivative, prior art | merged | 1c4c893 (+ this note) |
| `claude/s4-strong-field` | strong-field energies (chord, arc, tangent, blend, adjoint) | **done** | 762de9d |
| `claude/variational-force-design` | variational force: design note, V1 spec, V1 results at n = 1 | n = 2 **running** | 53fda06 |
| `claude/s2d-n3-spectrum` | SU(2) spectrum and numerical range at n = 3 | partial, **running** | 659b394 |
| `claude/s3d-startup` | does the Cayley step's N = 4 failure come from undamped stiff modes? | 7/8, **running** | a05052d |
| `claude/s3c-long-window` | the SU(2) flow on a 5× longer window | 6/8, **running** | c2e0940 |

## Done today

**Merged into `feat/reading-the-curve`:**
- *Reading the Curve*: clean-up, new subtitle, abstract on one page, Zenodo description and metadata;
  cites *Three Ways* by DOI (10.5281/zenodo.24375338).
- *Three Ways*: Zenodo description (with the Russian edition and the Perelman chapter explained,
  books link), Russian edition in `docs/papers/chord-arc-tangent/ru/`, **prior-art search done and
  the claim narrowed** (`docs/research/prior-art-chord-tangent-2026-11-19.md`): cancelling Wilson's
  θ⁴ term is known (de Forcrand et al. 1998), the tangent energy is Lüscher's topology-conserving
  action at its boundary value, and the inequality is a known Wilker–Huygens type; the chord–tangent
  blend as a lattice energy, its form in U and its coefficients were not found.
- S3b Cayley step (n = 1, 2, and the other machine's n = 3), and the exact defect derivative.

**S4, strong field (`claude/s4-strong-field`, `docs/STRONG_FIELD_ENERGY_RESULTS.md`):**
- Discretization error is 3–15× larger than any compactness spread on these meshes, even with faces
  turning up to 2.9 rad: P-regime fails.
- With the native chord reading U − 1, the 2:1 blend cancels less than predicted; the de Forcrand
  adjoint action does far better.
- Exploratory, post hoc: reading the chord along the holonomy axis, 2 sin(a/2)·n, restores
  Proposition 2 on GIGI's face-coupled action (100–3,000× cancellation, matching 7a⁴/720).

## Running now, and how far each has got

| job | where | progress | finishes |
| --- | --- | --- | --- |
| V1 at n = 2 | `tmp/var`, `scripts/ricci_variational_v1.py 1` | started ~12:30 | ~1 h |
| S2d n = 3 two-ring, then one-ring | `tmp/paper`, `scripts/ricci_su2_pg_spectrum.py 2 pg/pg_onering` | native n = 3 done | a few hours |
| S3d n = 2 two-ring N = 8 | `tmp/s3d` | 7 of 8 runs done | < 1 h |
| S3c n = 2 two-ring N = 40, 80 | `tmp/su2` | 3/5 and 2/5 samples | a few hours |

These run on this machine (Bee's Mac) in the session's scratch worktrees. If the session ends before
they finish, rerun the missing receipts from each branch's spec and STATUS file; every receipt is
deterministic and every script refuses to overwrite.

## Results so far on the running branches (interim, not verdicts)

- **V1 at n = 1:** the column condition is satisfiable (residual 5×10⁻⁹) and cuts the test-variation
  term from 43.5% to 12.2%, but the minimum-norm change multiplies the recovery's amplification by
  6.5 (median 30.5 → 199) and the reconstruction term rises from 4.7% to 432%; total error 440%
  against 45% before. Symmetric and stable (gauge count 21, no negative modes). V1-acc **fails** at
  n = 1; n = 2 decides whether this is a coarse-mesh effect.
- **S2d:** reproduction of S2b/S2c at n = 1, 2 passes (4×10⁻⁵); n = 3 native has no rough modes and
  W-left −4.676. The gates need the two-ring n = 3 run.
- **S3d:** the start-up removes the native force's alternation (P-settle passes) but does not halve the
  first-step gap (native 12.5% vs bound 11.2%) and makes the two-ring N = 4 run much worse (67% at the
  first sample, against 13% without it). The stiff-mode hypothesis explains the alternation, not the
  failure.
- **S3c:** G-time passes at n = 1 and for native at n = 2. **P-long (native) passes**: native's order
  between n = 1 and n = 2 drops to 1.32 on the long window (threshold 1.38), as S3a predicted. The
  two-ring prediction and P-cross wait for the last two runs.

## Issues to know about

1. **S4's P-regime confounds field strength with mesh resolution.** Making the Berger metric strongly
   non-round stretches the cells (10:1 at A = 100), so "strong field" and "badly resolved mesh" are
   not separated. A strong field on a round metric would separate them.
2. **V1's hold-out is weak at cell scale (C-span failed).** The control field is held out exactly, but
   within one cell the 23 enforced smooth fields reproduce it to 1.8×10⁻⁴: no generic basis of smooth
   fields can exclude a smooth field locally. The column condition therefore nearly constrains the
   scored field, as a generic smoothness condition should. Read any V1 improvement with that in mind.
3. **V1's minimum-norm objective is the wrong objective.** It minimises the size of the change, not the
   amplification it causes; the n = 1 failure comes entirely through amplification.
4. **S3d's start-up is first order.** A backward-Euler half-step has its own O(h²) error, which at
   h = 0.005 can dominate the stiff-mode effect it was meant to remove. An L-stable *second-order*
   start-up (TR-BDF2) would be the fairer test.
5. **Two copies of `paper_ru.pdf`.** The upload folder `~/Documents/three-ways-ru-zenodo/` (Bee's Mac,
   outside git) has been refreshed to the corrected edition; the repo copy is authoritative.
6. **The Perelman chapter is deliberately not in git** (public repo, unpublished book material). The
   updated English and Russian chapter, and the 17 November originals, are only in
   `~/Documents/three-ways-ru-zenodo/chapter-source/` on Bee's Mac and in her Drive (English original).

## Open questions for Bee

1. **Variational force, next objective (after V1 n = 2).** If n = 2 also fails through amplification,
   the proposal is V1b: the same constraints, minimising the recovery's amplification (‖M'_K‖₂ or the
   energy of R'_K on rough data) instead of the size of the change. Agree?
2. **Notion of "smooth" for the variational condition.** V1 uses locally linear fields (ambient
   constant and linear closed 2-forms). Does the programme's physics (a Hamiltonian or symplectic
   structure, quantisation) dictate a specific energy instead?
3. **Monte Carlo in Halcyon for the chord–tangent energy?** S4 says the blend as published (with the
   native chord) does not justify it; if run, compare the axis-blend and the adjoint action on meshes
   where discretization error is below the compactness spread.
4. ***Three Ways* wording after S4.** Proposition 2 holds per face; on a face-coupled action it
   carries over only with the axis-chord reading. Add a sentence and cite S4 in the next version?
5. **S3d follow-up.** Test a second-order L-stable start-up, or close the question with the interim
   reading (stiff modes explain the alternation only)?
6. **Social and Zenodo:** the "Reading the Curve" new version is ready to post (description in
   `docs/papers/two-ring-pg/zenodo/`); it assumes the first upload was commit d3f167d.
