# S2d: the linearised SU(2) force at n = 3: spectrum and numerical range (results)

2026-11-19. Spec: `docs/RICCI_SU2_PG_SPECTRUM_N3_SPEC.md` (4938db6, before measurement). Script
`scripts/ricci_su2_pg_spectrum.py`; receipts `research/su2-spectrum-n3-20261119/` (with every eigenvalue,
`*.eigs.npy`). Research script, **not an engine run**.

## In plain English

The two-ring SU(2) force stays stable at the third mesh, with 11,358 unknowns: no runaway modes, and
a numerical range whose left edge matches the native force's, so its lopsided spectrum still hides no
transient growth. The one-ring force fails again, worse than at the coarser meshes, as the Abelian
one ring does. This extends *Reading the Curve*'s stability results, measured at n ≤ 2, to n = 3.

## Results

| n | force | rough (Re λ < −10) | min Re λ | W-left | max \|Im λ\| |
| --- | --- | --- | --- | --- | --- |
| 1 | native | 0 | −4.475 | −4.634 | 0.80 |
| 1 | two-ring | 0 | −4.394 | −4.563 | 103.9 |
| 2 | native | 0 | −4.650 | −4.682 | 0.63 |
| 2 | two-ring | 0 | −4.632 | −4.664 | 150.3 |
| 3 | native | 0 | −4.661 | −4.676 | 0.64 |
| 3 | **two-ring** | **0** | −4.651 | **−4.666** | 418.0 |
| 3 | one ring (removal) | **4,437** | −45,543 | −45,922 | 1,305.6 |

The n = 1, 2 rows are this script's reproduction runs (C-repro).

## Gates and controls

| | threshold | measured | verdict |
| --- | --- | --- | --- |
| C-repro (n = 1, 2 vs S2b/S2c) | rough count exact; min Re λ, W-left to 10⁻³ | counts exact; 4×10⁻⁵ | pass |
| C-col (all runs) | 10⁻⁸ | ≤ 5.8×10⁻¹² | pass |
| C-chol | 10⁻¹³ | 1.8×10⁻¹⁶ | pass |
| C-W | eigenvalues right of W-left | 0 violations | pass |
| **G-rough, n = 3** | no two-ring Re λ < −10 | 0 | **PASS** |
| **G-W, n = 3** | two-ring W-left ≥ 2 × native and ≥ −10 | −4.666 ≥ −9.353 | **PASS** |
| **Removal, one ring** | expected to fail G-rough | 4,437 rough, min −45,543 | **fails, as required** |

## Notes

- The one-ring instability grows with refinement, as in the Abelian case: 207, 1,451, 4,437 rough
  modes and a most negative real part of −8,621, −19,829, −45,543 at n = 1, 2, 3.
- **Trend to watch:** the two-ring operator's non-normality grows with refinement (max |Im λ| = 104,
  150, 418), while its numerical range stays at the native's. Transient growth is bounded at every
  level measured; whether the imaginary extent keeps growing at n = 5 is not measured here.
- Since the exact defect derivative, K carries no finite-difference gradient noise; the n = 1, 2 values
  differ from S2b's by about 4×10⁻⁵ relative for that reason.
- Not measured: the numerical-range polygon, radius and step bounds at n = 3; n = 5; the flow.
