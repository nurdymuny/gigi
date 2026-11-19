# S3a: why the native force does not stall in time (results)

2026-11-18. The spec was fixed before measurement and committed first:
`docs/RICCI_SU2_PG_FLOW_FILTER_SPEC.md` (commit 0c0a40c5). Research script
`scripts/ricci_su2_pg_flow_filter.py`, **not an engine run**. Receipts:
`research/su2-filter-20261118/L{0,1}_{native,pg}.json`.

## In plain English

The native force is about 25% wrong at every instant, yet its trajectory error kept
falling in S3. The reason is now measured: **99% of the native force's error points in stiff
directions**, the ones the lattice snaps back from within a small fraction of the time
window. A push there never builds up. Only the remaining 1%, in slow directions, carries
into the trajectory.

A simple linear model built on that split predicts every trajectory error S3 measured, for
both forces at both mesh levels, to within 6%. So the failed prediction was a prediction
about the wrong quantity: what matters in time is how wrong a force is in the slow
directions, and the instantaneous 25% is almost all in the fast ones.

The two-ring force's error is less concentrated in stiff directions (92% at n = 2), but it is
much smaller to begin with, and its slow part is shrinking faster.

## Controls

| Control | n = 1 native | n = 1 two-ring | n = 2 native | n = 2 two-ring | Threshold |
| --- | ---: | ---: | ---: | ---: | --- |
| C-S2a: ρ(0) against S2a | 26.246% (S2a 26.246%) | 36.247% (36.25%) | 24.744% (24.74%) | 7.427% (7.43%) | 5% relative |
| relative difference | 2.5×10⁻⁷ | 8.3×10⁻⁵ | 1.7×10⁻⁴ | 3.7×10⁻⁴ | |
| C-eps: ε = 10⁻⁵ against 10⁻⁶ | 4.9×10⁻⁸ | 4.9×10⁻⁸ | 4.9×10⁻⁸ | 4.9×10⁻⁸ | 10⁻⁶ |
| C-K: left edge against S2c | 1.1×10⁻¹³ | 7.3×10⁻¹⁴ | 1.6×10⁻¹² | 9.2×10⁻¹³ | 10⁻⁸ |

All pass. The instantaneous error is reproduced from a time difference of the exact links,
independently of S2a's continuum mapping.

## Result

| | n = 1 native | n = 1 two-ring | n = 2 native | n = 2 two-ring |
| --- | ---: | ---: | ---: | ---: |
| instantaneous error ρ, t = 0 → 0.02 | 26.2% → 25.8% | 36.2% → 34.8% | 24.7% → 24.3% | 7.4% → 5.6% |
| **slow share f_slow** (fraction of the error in directions with rate < 50) | 3.8% | 12.1% | **0.85%** | **7.6%** |
| slow part of the error, ρ·√f_slow | 5.13% | 12.60% | **2.28%** | **2.05%** |
| filter factor W = E_pred / E_naive | 0.289 | 0.415 | **0.095** | **0.284** |
| E(0.02) measured (S3) | 0.03493 | 0.06886 | 0.01127 | 0.00900 |
| E(0.02) predicted by the linear filter | 0.03704 | 0.07269 | 0.01176 | 0.00935 |
| worst model error over the four sample times | 6.1% | 5.6% | 4.3% | 4.0% |

E_naive is the error if nothing snapped back: 0.128, 0.175, 0.124, 0.033.

**G-model: PASS** for both forces at both levels (worst 6.1%, threshold 25%). The model error
grows with time at every level (1–2% at t = 0.005, 4–6% at t = 0.02), as expected from what
it leaves out: the operator is frozen at t = 0, and the term from the moving mass is dropped.

**H-wash at n = 2: PASS.** f_slow(native) = 0.85% < 25%, and W(native) = 0.095 is below half of
W(two-ring) = 0.284 (bound 0.142).

## What it means

- **The native force's 25% error is real, and it is stiff.** At n = 2, less than 1% of it lies
  in slow directions. The dynamics removes the rest within the window. Refinement makes the
  stiff directions stiffer, so W for native falls from 0.29 to 0.095 between n = 1 and n = 2.
  That is why the trajectory error converged while the instantaneous error stalled.
- **The stall prediction in Amendment 1 was about the wrong quantity.** It took the
  instantaneous error as the predictor of the trajectory error. Over a window this short,
  the predictor is the slow part of the error.
- **In the slow directions the two forces are now close.** At n = 2, the slow part of the error
  is 2.28% for native and 2.05% for two rings. Between n = 1 and n = 2 it falls with observed
  order 1.17 for native and 2.62 for two rings.
- **Forecast for n = 3, not a gate:** both trajectory errors keep falling, two rings faster.
  If the orders hold, the gap between them widens. A run at n = 3 (with the Cayley stepper)
  would test this.
- **On a longer window**, more directions count as slow (the threshold is 1/T), and native's
  stiff 25% begins to matter. The short window is part of why native does well here.

## Honest observations

- **The slow/fast split depends on a threshold**, rate < 1/T = 50. The slow subspace has 72 and
  83 directions at n = 1 and about 500 at n = 2. The filter factor W does not depend on a
  threshold, and it tells the same story.
- **For the two-ring force the split is approximate.** Its operator is non-normal, and an
  orthogonal projection onto an invariant subspace is only an approximation to the mode
  split. G-model does not use the split, and it passes.
- **This is a first-order model.** It does not replace S3's nonlinear runs, which remain the
  measurement. It explains them, to within 6%.
- **Not measured:** n ≥ 3, longer windows, other metrics, the engine path.
