# V1: variational force from a constrained two-ring recovery (results; n = 1, interim)

2026-11-19. Spec: `docs/VARIATIONAL_RECOVERY_V1_SPEC.md` (c84c535, before measurement). Script
`scripts/ricci_variational_v1.py`; receipt `research/variational-v1-20261119/L0.json`. Research
script, **not an engine run**. n = 2 is queued (needs ~10 GB; it starts when the S2d run frees memory).

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
