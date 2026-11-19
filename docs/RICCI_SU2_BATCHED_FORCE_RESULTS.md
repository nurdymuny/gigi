# The SU(2) force, batched across cells (results, CPU)

2026-11-19. Spec: `docs/RICCI_SU2_BATCHED_FORCE_SPEC.md` (committed `c5e4d72f` before any
batched code ran). Script: `scripts/ricci_su2_batched.py`. Receipts:
`research/su2-batched-20261119/`. Research script, **not an engine run**.

## In plain English

The batched force gives the same numbers as the per-cell loop, at every level and for both
forces. The native gradient is bit-identical. The two-ring gradient differs from the loop by
5×10⁻¹⁴ at most, which is rounding. Rerunning the S3b Cayley steps with it reproduces the
committed answers to 5×10⁻¹⁴, with the same number of Newton iterations at every step. It is
9 to 37 times faster. At n = 3 a two-ring force evaluation takes 2.4 s instead of 89 s.

## Parity gates (all pass)

Worst relative difference over two states (stored initial links, and the saved S3b two-ring
final state at that level) and two metrics (g(0), g(0.02)). Receipts: `parity_L{0,1,2}_{native,pg}.json`.

| level | force | P-grad (≤ 10⁻¹²) | P-mass (≤ 10⁻¹³) | P-vel (≤ 10⁻¹⁰) | P-angles (≤ 10⁻¹³) |
| --- | --- | --- | --- | --- | --- |
| n = 1 | native | 0 | 1.2×10⁻¹⁶ | 2.8×10⁻¹⁵ | 0 |
| n = 1 | two-ring | 2.2×10⁻¹⁴ | 1.2×10⁻¹⁶ | 1.6×10⁻¹³ | 0 |
| n = 2 | native | 0 | 1.2×10⁻¹⁶ | 3.6×10⁻¹⁶ | 0 |
| n = 2 | two-ring | 3.6×10⁻¹⁴ | 1.2×10⁻¹⁶ | 3.1×10⁻¹⁴ | 0 |
| n = 3 | native | 0 | 1.2×10⁻¹⁶ | 3.7×10⁻¹⁶ | 0 |
| n = 3 | two-ring | 5.4×10⁻¹⁴ | 1.2×10⁻¹⁶ | 4.2×10⁻¹⁴ | 0 |

**P-run.** The S3b Cayley step with N = 8, rerun on the batched force, against the committed
receipts `research/su2-cayley-20261118/L{0,1}_{native,pg}_N8.json`. Gate: E(0.02) to 10⁻⁹,
identical Newton counts.

| level | force | E(0.02) relative difference | Newton counts identical |
| --- | --- | --- | --- |
| n = 1 | native | 1.1×10⁻¹⁴ | yes (5 ×8) |
| n = 1 | two-ring | 3.0×10⁻¹⁵ | yes |
| n = 2 | native | 1.7×10⁻¹⁴ | yes |
| n = 2 | two-ring | 4.8×10⁻¹⁴ | yes |

## Timing (measured, not gated)

Seconds per force evaluation (gradient, mass and CG solve), one process, nothing else
running, best of three for the batched version, one evaluation for the loop.

| level | force | loop | batched | speed-up |
| --- | --- | --- | --- | --- |
| n = 1 | native | 2.48 | 0.091 | 27× |
| n = 1 | two-ring | 3.43 | 0.108 | 32× |
| n = 2 | native | 6.19 | 0.667 | 9× |
| n = 2 | two-ring | 20.7 | 0.776 | 27× |
| n = 3 | native | 16.2 | 1.72 | 9× |
| n = 3 | two-ring | 88.7 | 2.43 | 37× |

The S3b n = 3 runs reported 41 s (native) and 146 s (two-ring) per evaluation; those ran six
at a time, so the single-process loop numbers above are the fair comparison.

The native speed-up is smaller at n ≥ 2 because the loop was already cheap there: what is left
in both versions is the metric pieces and the CG mass solve, which this rewrite did not touch.

## Not done yet

The GPU step of the spec; rebuilding K₀ with the batched force (the n = 3 build took 6,672 s
with the loop).
