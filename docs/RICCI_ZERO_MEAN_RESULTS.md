# Option 1: Whitney plus a zero-mean correction — diagnostic result

2026-11-17, Claude. Chosen by Bee after `RICCI_PG_CURVATURE_RESULTS.md`.
**Run as a diagnostic, not a gated feature**, for the reason below. Python
research scripts; nothing ran through the GIGI engine. No commit.

**BW05 is not closed. Option 1 fails it.**

## Why it was run as a diagnostic

Before building, the sufficient condition in the assessment that motivated
option 1 was re-derived and found wrong (correction now in
`RICCI_CLAUDE_ASSESSMENT_2026_11_17.md`, section 4). Two independent problems:

1. **Accuracy.** A correction with zero mean on every cell keeps Whitney's cell
   means. Those are first-order accurate: metric-weighted mean error 3.56%,
   1.87%, 1.25% at n = 1, 2, 3, orders 0.93 and 0.98. The closed linear fit's
   means, for contrast, reach orders 1.88 and 1.92 (`cell_means.json`).
2. **Consistency.** Zero mean bounds the transpose term by order h times the
   order-one-over-h gain of rough variations: order one, not small.

The predictions, stated in the session **before** the run and recorded here
afterwards, were: curvature of order about one, and a force that stops growing
but floors.

## Construction

On each cell, Whitney's curvature plus the closed linear star fit with its own
metric-weighted cell mean removed. So the cell mean is exactly Whitney's, and the
variation inside the cell is the fit's. The force is the gradient of that
curvature's energy, the variational route. Script
`scripts/ricci_zero_mean_diagnostic.py`.

Controls, which refuse otherwise: with the correction dropped, the assembly
reproduces native `stiffness @ a` and RF2D's 29.8111% at n=1; the correction's
cell mean is zero to 1.8e-14 on every cell.

## Result

Receipts `zero_mean_diagnostic.json` (n = 1, 2, 3) and
`zero_mean_diagnostic_L3.json` (n=5).

| n | force error | rate | curvature error | rate |
| --- | ---: | ---: | ---: | ---: |
| 1 | 78.85% | | 4.67% | |
| 2 | 61.39% | 0.36 | 2.05% | 1.19 |
| 3 | 49.87% | 0.51 | 1.31% | 1.10 |
| 5 | 40.47% | 0.41 | 0.77% | 1.05 |

- **The curvature prediction held:** order about one, as Whitney's means force.
- **The force prediction did not hold.** It does not floor. It converges
  slowly, at rates 0.36, 0.51 and 0.41, and the rates are not decaying. The
  order-one bound is an upper bound that is not sharp here.
- **BW05 fails** three ways: 40.5% at n=5 against 5%; finest rate 0.41
  against 0.5; and worse than original Whitney (26%) at every level on these
  meshes. From 49.9% at n=3, reaching 5% at n=5 would need a final-step rate
  near 4.5.

Compared with the full closed linear fit used variationally (58%, 663%, 778%,
growing), removing the cell mean turned a divergent gradient into a slowly
convergent one. That is a real change in behaviour, but not a usable method.

## What this establishes

Across all four constructions now measured on the same data, mass and BW05
quantity, the force behaviour follows how the test variations are built:

| Construction | Test variations | Force, finest measured |
| --- | --- | --- |
| Whitney | Whitney | 26.1%, flat |
| Petrov-Galerkin, star fit | Whitney | **1.17%**, rate 1.32, but unstable |
| Variational, star fit | reconstructed, full | 778%, growing |
| Variational, zero-mean correction | reconstructed, mean held at Whitney's | 40.5%, rate ~0.4 |

The only consistent force so far kept the original Whitney test variations
untouched, and it is unstable. Every variational construction tested changes
the test variations, and none is consistent at BW05's level.

## Not claimed

No claim that no variational reconstruction can pass; four constructions were
measured, not all. No Berger, SU(2) or flow result. The order-one bound is
shown not to be sharp, not shown to be wrong in general.
