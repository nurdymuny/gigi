# Fold-force: the Abelian force on folded stencils (results)

2026-11-18. The spec was fixed before measurement: `docs/RING_FOLD_FORCE_SPEC.md`. Research
script `scripts/ring_fold_force.py`, **not an engine run**. Receipts are in
`research/ring-fold-force-20261118/` (`fold/diagnostic.json`, `random/diagnostic.json`, and
the chosen sets per level).

## In plain English

Each cell kept only the second-ring neighbours the fold chose, two to four on average out of
about ten, and the paper's own force was rebuilt on those smaller stencils. **It passes both
of the paper's tests:**
- it converges (1.57% error on the finest mesh, against the full two rings' 1.45%);
- it is stable (no runaway eigenvalues).

The same number of neighbours picked at random also converges, but it is **unstable**: 26,
then 48, runaway modes. So the fold's *choice* is what keeps the force stable, not just the
count. That is Bee's graph-fold lesson carried all the way to the force: keep the
differences that change the answer.

## Controls (level 0, run before the experiment)

| Control | Result |
| --- | --- |
| C-full: the patched build with the full two-ring set reproduces the paper's two-ring operator | difference **0** |
| C-one: the patched build with the one-ring set reproduces the one-ring operator | difference **0** |
| C-chosen: recomputed greedy sets match the fold receipt | **0** mismatches of 120 |

## Results

Relative force error (q6) and the paper's BW05 gate (`numerical_gate`):

| Stencil | n = 1 | n = 2 | n = 3 | n = 5 | observed orders | BW05 |
| --- | ---: | ---: | ---: | ---: | --- | --- |
| **folded (the fold's choice)** | 6.90% | 7.03% | 3.34% | **1.57%** | −0.04, 1.87, 1.49 | **PASS** |
| random, same number | 20.60% | 8.03% | 5.87% | 3.47% | 1.43, 0.78, 1.04 | PASS |
| two rings (paper, for reference) | 6.22% | 3.83% | 2.79% | 1.45% | 0.73, 0.80, 1.29 | PASS |

S3, the paper's stability criterion (small-eigenvalue count = Whitney gauge dimension, and
every physical eigenvalue with positive real part):

| Stencil | n | gauge (PG / Whitney) | negative real parts | min Re λ (physical) | max \|Im λ\| | S3 |
| --- | --- | --- | ---: | ---: | ---: | --- |
| **folded** | 1 | 21 / 21 | **0** | 4.019 | 269 | **stable** |
| **folded** | 2 | 163 / 163 | **0** | 3.998 | 1,081 | **stable** |
| random | 1 | 21 / 21 | 26 | −2,952 | 350 | **unstable** |
| random | 2 | 163 / 163 | 48 | −5,038 | 801 | **unstable** |

**Claim rule (from the spec).**
- **"The folded force works":** BW05 and S3 both pass. **Met.**
- **"The choice matters for the force":** the random removal fails BW05 or S3 at some level.
  **Met:** random fails S3 at n = 1 and n = 2.

## Honest observations

- **The folded force is not better than two rings, only smaller.** Its errors are at or above
  the two-ring errors at every level (1.57% against 1.45% at n = 5). At n = 2 it is clearly
  worse (7.03% against 3.83%), and the first observed order is negative. The pre-asymptotic
  range is longer for the folded stencil.
- **Its spectrum is more non-normal.** The largest imaginary part is 1,081 at n = 2. The
  paper's two-ring stability table does not report |Im λ|, so the two cannot be compared from
  receipts here. The numerical range (the dissipativity certificate) was not computed.
- **The stencils differ from cell to cell**, and the saving shrinks under refinement (fold
  results: median 2 → 4 second-ring cells). Whether the folded stencil stays affordable on
  finer meshes is still open.
- **Random converges in the gate's sense**, but it is unstable. A force can pass BW05
  instantaneously and still have runaway modes. That is the paper's one-ring lesson again.

## What this says

The fold's claim, that a minority of well-chosen second-ring cells carries the
steadying, now holds at the level of the force, not just the fit. A smaller stencil that
reads the curve and stays stable exists, and it is found by Bee's matching-menu idea: keep
what changes the answer. It is research evidence for a cheaper stencil. It is not yet a
replacement for the paper's uniform two-ring rule.
