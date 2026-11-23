# Fold-force: the Abelian force on folded stencils (spec)

2026-11-18. Fixed **before** measurement. This is the next rung opened by
`docs/RING_FOLD_RESULTS.md`, whose force-rebuild gate (saving of at least 25% at every
level) passed with 78%, 75%, 70% and 60%. Its provenance is Bee's graph-fold article, via
idea 2 of `docs/research/small-edges-2026-11-18.md`.

## In plain English

The fold experiment found that each cell needs only two to four of its roughly ten
second-ring neighbours to read the curve almost as steadily as all of them. But steadiness
of the fit is only a stand-in. What matters is the force. This spec rebuilds the paper's
Abelian force with each cell using only its chosen neighbours, then runs the paper's own
two tests unchanged: does the force converge (BW05), and is it stable (S3)? As a check
that the choice matters, it does the same with the same number of neighbours picked at
random.

## Technical

**The folded fit.** For cell K, the chosen set C_K is the one-ring set plus the second-ring
cells picked by the greedy rule of `docs/RING_FOLD_SPEC.md`: add the candidate that most
lowers ‖R_K‖₂ until ‖R_K‖₂ ≤ 1.25 × its full two-ring value. The fit is
`ricci_pg_tworing.fit_cell_rings(…, rings = 2)` unchanged, with only its neighbourhood
(`neighbourhood_rings`) restricted to C_K, keeping the same transported placements and the
same insertion order. Everything else (build, mass, data, norm, the BW05 measurement and
S3) is the paper's own code path in `scripts/ricci_pg_tworing_gate.py`, replicated
line for line.

**Removal: random.** For each cell, the second-ring part of C_K is replaced by a random
subset of the candidates of the same size, with seed = cell index + 1000 × level (as in
the fold spec).

## Gates (fixed now)

- **BW05:** the paper's `numerical_gate` (`e2e/probes/ricci_shape_evidence.py`) on levels
  0–3 at q4 and q6, unchanged: final error below 5%, last two orders at least 0.5, and the
  quadrature and linear-solve estimates below error/10.
- **S3:** at n = 1, 2 (levels 0, 1), exactly the paper's criterion. The small-eigenvalue count
  equals the Whitney gauge dimension, and every physical eigenvalue has strictly positive
  real part. n = 3 is optional, if cost allows.
- **Claim rule.** "The folded force works" requires BW05 and S3 both to pass.
  - "The choice matters for the force" additionally requires the random removal to fail
    BW05 or S3 at some level.
  - If random passes both too, the finding is that the count of neighbours matters and the
    choice does not, and it is reported that way.
- **Reported against two rings:** the folded errors next to the paper's two-ring errors
  (6.22%, 3.83%, 2.79%, 1.45%), and the faces used.

## Controls

- **C-full.** With C_K set to the full two-ring set, the patched build reproduces the paper's
  two-ring operator at level 0 exactly (entrywise difference below 1e-12).
- **C-one.** With C_K set to the one-ring set, it reproduces the one-ring operator at level 0.
- **C-chosen.** Recomputing the greedy sets reproduces the fold receipt's k for every cell
  (`research/ring-fold-20261118/fold.json.gz`).

## Scope and honest cost

- **Scope:** the Abelian control only. SU(2) on folded stencils comes after.
- **Cost:** the greedy sets cost what the fold run did (about 50 minutes at n = 5). Each
  build costs what the paper's two-ring build costs. The stencils differ from cell to
  cell, which is a disclosed design change from the paper's same-radius-everywhere rule.
- Research script, not an engine verb.
