# Fold-Λ: the smallest stable piece of rim (results)

2026-11-18. The spec was fixed before measurement: `docs/RING_FOLD_SPEC.md`. Research
script `scripts/ring_fold.py`, **not an engine run**. Receipts are in
`research/ring-fold-20261118/`: `fold.summary.json`, plus the per-cell record (the
chosen cells and the greedy path for every cell) in `fold.json.gz`.

## In plain English

One ring is the smallest rim, and it mistakes roughness for curve. Two rings fix that, at
the cost of about ten extra cells per fit. This experiment asked how few of those ten are
really needed. The answer: **on average, two to four well-chosen cells read the curve
within 25% as steadily as all ten.** And which ones matters: the same number picked at
random fails in most cells. That is Bee's fold idea, measured. Most of the second ring
repeats what the fit already knows.

The saving shrinks as the mesh refines, though: the median number of cells needed grows
from 2 to 4. Whether it settles there or keeps growing is the open question.

## Controls

| Control | Result |
| --- | --- |
| C-fit: the rebuilt map reproduces `fit_cell_rings` for one ring and two rings, on every cell | relative difference **0** at every level |

## Result

Amplification here is ‖R_K‖₂, as in the paper. The target is 1.25 times the full
two-ring amplification for that same cell.

| n | cells | median one-ring | median two-ring | median folded | second-ring cells needed (median / max of ~10) | saving in cells | median faces: folded / two-ring | random subset meets target |
| --- | ---: | ---: | ---: | ---: | --- | ---: | --- | ---: |
| 1 | 120 | 251.8 | 30.5 | 35.0 | 2 / 2 | 78% | 22 / 40 | 4.2% |
| 2 | 960 | 125.4 | 35.2 | 40.6 | 2.5 / 7 | 75% | 23.5 / 44 | 17.4% |
| 3 | 3,240 | 125.8 | 34.6 | 40.0 | 3 / 8 | 70% | 25 / 44 | 11.9% |
| 5 | 15,000 | 127.0 | 34.2 | 39.6 | 4 / 8 | 60% | 28 / 44 | 9.4% |

How many second-ring cells each fit needed:

| n | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | | 120 | | | | | | |
| 2 | | 480 | 120 | 120 | | 180 | 60 | |
| 3 | 240 | 960 | 540 | 780 | 360 | 180 | 120 | 60 |
| 5 | 900 | 2,820 | 3,240 | 5,160 | 1,980 | 360 | 360 | 180 |

## Gates

- **Fold-Λ (reported).** The saving in cells is 78%, 75%, 70% and 60%. The saving in faces
  is 75%, 73%, 68% and 57%.
- **Removal: random selection. PASS at every level.** Random subsets of the same size meet
  the target in 4.2%, 17.4%, 11.9% and 9.4% of cells, all below the spec's 50%. So the
  claim is allowed: **the choice of cells matters, not just their number.**
- **Force-rebuild gate: OPEN.** The spec required a saving of at least 25% at every level,
  and the smallest is 60%. The next rung is to rebuild the force on the folded stencils
  and run BW05 and S3.

## Honest observations

- **The saving shrinks under refinement.** The median number of cells needed goes 2, 2.5,
  3, 4, and the tail reaches 8 of about 10 candidates. Four levels cannot tell whether
  this plateaus, so the folded stencil is not yet known to be mesh-uniform the way the
  full two-ring stencil is. That has to be settled before the folded force is claimed as
  better than two rings.
- **The stencil differs from cell to cell.** Neighbouring cells choose different rims, as
  the research note warned. A folded force would be a disclosed design change from the
  paper's same-radius-everywhere stencil.
- **Amplification is a lower-bound proxy for error.** A folded fit within 1.25× of the
  two-ring amplification is not yet shown to give a force within any stated tolerance of
  the two-ring force. That is what the next rung measures.
- **The target is relative.** The folded stencils sit at a median of about 40, against
  about 35 for two rings and 125–252 for one ring. They are much closer to two rings than
  to one.

## What this says about the idea

The fold question was whether most of the second ring repeats information. At every level
it does: a minority of well-chosen cells carries nearly all of the steadying, and an equal
number chosen blindly does not. That is the matching-menu lesson of Bee's graph-fold
article: keep the differences that change the answer, and let the rest go. *RESONANCE:*
the result is about least-squares conditioning, and it is not an equitable-partition
theorem.
