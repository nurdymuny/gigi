# V1: variational force from a constrained two-ring recovery (results, n = 1 and n = 2)

2026-11-19. Spec: `docs/VARIATIONAL_RECOVERY_V1_SPEC.md` (c84c535, before measurement). Script
`scripts/ricci_variational_v1.py`; receipt `research/variational-v1-20261119/L0.json`. Research
script, **not an engine run**. Receipts `L0.json`, `L1.json`. **Verdict: V1-acc fails at n = 1 and n = 2; V1-stable
passes at both; V1b (`docs/VARIATIONAL_RECOVERY_V1B_SPEC.md`) runs next, as Bee directed.**

## In plain English

The constrained recovery does what it was built to do: it removes most of the error term that makes
the variational force diverge (the recovery's response to rough test variations), from 43.5% to
12.2%. But the smallest change that achieves this is about ten times the size of the original fit,
which multiplies the recovery's amplification by 6.5, and that amplification shows up in the other
error term instead. At n = 1 the new variational force is 440% wrong, against 45% before. It is
symmetric and stable. Whether this is a coarse-mesh effect (the change should shrink with refinement
if the conflict is first-order) or a structural cost is what n = 2 decides.

## Controls

| control | threshold | measured | verdict |
| --- | --- | --- | --- |
| C-repro: C = 0 reproduces the two-ring variational force | 45.4% (paper) | 0.454051 | pass |
| C-span (a): control in the held-out left-invariant span | 10⁻¹² | 9.8×10⁻¹⁶ | pass |
| C-span (b): control's distance from the enforced span | ≥ 10⁻³ | 1.8×10⁻⁴ (smallest cell) | **fail** |
| C-solve: column condition after the solve | 10⁻⁸ relative | 5.1×10⁻⁹ | pass |
| C-exact | 10⁻¹⁰ | by construction (Z, N null spaces) | pass |

C-span (b) fails: within one cell, the 23 enforced smooth fields reproduce the control to 1.8×10⁻⁴.
The hold-out removes the control exactly but not locally; any generic basis of smooth fields nearly
spans every smooth field at cell scale. The threshold encoded a wrong expectation. It means the
column condition also nearly constrains the control, as a generic smoothness condition should; it
does not mean it was tuned to it.

## Result at n = 1

| | two-ring variational (C = 0) | V1 constrained |
| --- | --- | --- |
| BW05 relative error | 45.4% | **439.8%** |
| reconstruction term (Prop. 1) | 4.7% | **432%** |
| test-variation term (Prop. 1) | **43.5%** | 12.2% |
| curvature relative error | 3.6% | 15.6% |
| fit amplification, median / max | 30.5 / 30.5 | 199 / 203 |
| ‖C_K‖ median / max | 0 | 288 / 289 |
| S3: small eigenvalues / negative / smallest physical | — | 21 (= gauge) / 0 / 4.03 |
| symmetry ‖K − Kᵀ‖/‖K‖ | — | 8×10⁻¹⁷ |

**V1-acc: FAIL at n = 1. V1-stable: PASS at n = 1.**

## Result at n = 2

| | two-ring variational (C = 0) | V1 constrained |
| --- | --- | --- |
| BW05 relative error | 276.3% | **61.6%** |
| reconstruction term | 3.7% | **61.6%** |
| test-variation term | **275.9%** | 1.2% |
| curvature relative error | 0.92% | 2.9% |
| fit amplification, median / max | 35.2 / 44.7 | 49.4 / 108 |
| ‖C_K‖ median / max | 0 | 65 / 149 |
| S3: small / negative / smallest physical | — | 163 (= gauge) / 0 / 3.9988 |
| symmetry | — | 1.3×10⁻¹⁶ |

Controls at n = 2: C-repro 276.34% (paper 276%), pass; C-span (a) 1.4×10⁻¹⁵, pass; C-span (b)
2.0×10⁻⁵, **fail** (as at n = 1); **C-solve 6.8×10⁻⁵, fail** (threshold 10⁻⁸; 21,356 constraint rows,
numerical rank 18,239 at the 10⁻¹² eigenvalue cut). The constraints are not met to the stated
tolerance at n = 2; the most likely cause is ill-conditioned constraint directions dropped by the
pseudo-inverse, to be checked (a looser cut, or an iterative solve, would show it).

**V1-acc: FAIL at n = 2** (61.6% is far below 276% but not below Whitney's 26%). **V1-stable: PASS.**

## Trend from n = 1 to n = 2

- The test-variation term, the one that makes the variational force diverge, is almost removed at
  n = 2: 275.9% → 1.2%.
- The cost of the change shrinks with refinement: the median change falls from 288 to 65 and the
  amplification grows by 1.4× instead of 6.5×.
- The reconstruction term (61.6%) is now the whole error. It is driven by the recovery's response to
  data, which is what V1b minimises.

## Reading

- The diagnosis of *Reading the Curve* is confirmed directly: the two-ring variational force's error is
  almost all test-variation term (43.5% of 45.4%).
- The column condition is satisfiable and cuts that term, so the route is not closed by inconsistency.
- The cost is amplification: a minimum-norm change in C is not a minimum-amplification change. The
  reconstruction term is ⟨RΦ − F, R Dη⟩ and carries R's amplification through the test functions.

## Next (decided after n = 2)

If the change shrinks under refinement, V1's problem is pre-asymptotic. If not, the objective should
penalise amplification directly (minimise ‖M'_K‖₂ or the H-norm of R'_K on rough data subject to the
same constraints), or the condition should be imposed only to the order the error analysis needs.

## V1b (stiffness objective), n = 1 and n = 2

Spec `docs/VARIATIONAL_RECOVERY_V1B_SPEC.md` (e67ea14). Receipts `research/variational-v1b-20261119/L{0,1}.json`.

| | n = 1 | n = 2 |
| --- | --- | --- |
| BW05 error (V1 for comparison) | 485% (440%) | **48.3%** (61.6%) |
| reconstruction / test-variation term | 479% / 10.2% | 48.1% / 1.1% |
| amplification median, before → after | 30.5 → 285 | 35.2 → 83.5 |
| trace K′ (V1 for comparison) | 64k (116k) | 132k (n/a) |
| C-solve residual | 3.1×10⁻⁹ | 6.7×10⁻⁵ (**fail**) |
| S3 | 21 / 0 negative / 4.014 | 163 / 0 negative / 3.9985 |

**V1b-acc FAIL at both levels; stable at both.** The count (`docs/research/variational-count-2026-11-19.md`)
explains why no objective could rescue V1: at two rings the change can only act on the field's own
second-order part. V2 (three rings, change kept off quadratic fields) is the response.
