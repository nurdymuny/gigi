# The missing kinetic term now has a scale

## What I am keeping

The new passage asks a more useful question than simply saying that the kinetic term is unknown: how large would
it have to be to change the ordering? That is the point of the addition, and it stays. So do the first-person
narrative, the model, the registered controls and thresholds, the original 46 statements, and their mathematics.
The new statement remains COMPUTED: a local diagnostic estimate with an uncomputed kinetic coefficient, not a new
claim about the full physical spectrum.

This review starts from `d627989e`, including the two edits just pulled. It changes only the new threshold passage,
its register description, and its verification/supporting files. The older report and referee notes remain historical
sources; their wording is not silently rewritten.

## What the saved numbers say

I replayed `research/gigi-m3-20261127/result.json` without running a new field solver. At the sourced control
`t = 3, s = 1.15`, its mode frequencies give:

| Quantity | Replayed value |
|---|---:|
| Competitor minus Helike harmonic frequency | 0.01654957628440279 |
| Fast-potential coefficient, H_CC / omega_C - H_HH / omega_H | 1.258409747055742 |
| Kinetic threshold at g^2 = 0.002 | 7.325197889257137 |
| Kinetic threshold at g^2 = 0.004 | 3.187803818156439 |

The anharmonic difference is the quoted `-2.208` from Part V of the report, not newly recomputed here. The threshold
values use that rounded coefficient. Digits in this table reproduce saved arithmetic; they do not represent
physical accuracy or include the regulator-window spread. The rounded 7.33 and 3.19 in the narrative are retained.
The kappa in this passage is a kinetic coefficient difference. It is not the finite-counterterm parameter called
kappa in the earlier receipt.

## Where I sharpened the description

1. **A threshold for a stated estimate.** I name the displayed expression D_lin. Its sign orders these two local
   branches, not every state of the physical theory. For positive g^2 the condition is
   kappa < 0.01655 / g^2 - 0.950. Equality is a tie. This remains valid if 0.950 + kappa is zero or negative;
   division by that latter quantity does not. There is no finite-coupling remainder bound in this calculation.
2. **The root has its own coefficients.** If D(s,epsilon) = D0(s) + epsilon B(s) + o(epsilon) smoothly near a simple
   root, differentiating D(s*(epsilon),epsilon) = 0 gives ds*/d(epsilon) = -B(s0*) / D0'(s0*). Here
   B = a + p - kappa. The actual first-order derivative uses coefficients at s0*, not at the nearby control.
   Replacing them with their control values is an additional approximation. I retain it and name it.
3. **Trend is not exact reproduction.** The harmonic block gives s0* = 1.143567769390845 and
   D0'(s0*) = 2.569240136138929. Freezing the anharmonic coefficient at -2.208 gives these comparisons:

| g^2 | Frozen-control linear estimate | Registered reduced-model prediction | Difference |
|---|---:|---:|---:|
| 0.03 | 1.169349711323119 | 1.1674 | 0.001949711323119 |
| 0.05 | 1.186537672611301 | 1.1809 | 0.005637672611301 |
| 0.07 | 1.203725633899483 | 1.1931 | 0.010625633899483 |

Part V says the registered predictions use varying coefficients. These differences do not invalidate those
predictions, and I have changed none of their values or acceptance tolerances. They do invalidate saying that the
constant slope reproduces them exactly. Likewise, the estimated dressing lowers the switch only if kappa < 1.258.

4. **Born--Huang power counting is local.** With a smooth, gapped fast chart and bounded derivatives,
   Phi(q) = g^2 phi(2gq) has Hessian 4g^4 Hess(phi). Its leading common constant cancels between same-well
   excitation energies. It need not cancel between different wells. Neither calculation supplies a uniform
   ultraviolet bound. The doublet is treated in symmetry-preserving charge blocks, not declared nondegenerate.
5. **A held background is not automatically a dressed equilibrium.** The local Hessian calculation is at the
   specified background. If sources are held fixed instead, the fast potential can move the equilibrium by O(g),
   and the O(g) cubic derivative of the reduced potential then changes its stiffness by O(g^2). That contribution
   has not been included in the numeric estimate. For example, V(x) = x^2/2 + g x^3/3 + g x has its local stationary
   point at x = -g + O(g^3); V'' there is 1 - 2g^2 + O(g^4), although V''(0) = 1. This is an algebraic counterexample
   to silently equating the two prescriptions, not a replacement Yang--Mills model.

The physical switch still needs the complete matched effective Hamiltonian, a source prescription and remainder
control. Naming these does not remove the threshold's usefulness: it tells me what the present calculation measures
and what a future kinetic calculation must be compared against.

## External cross-check

Panati, Spohn and Teufel, *The time-dependent Born--Oppenheimer approximation* (2007), equations (43), (48) and the
discussion following (51), distinguish the Born--Huang scalar correction from the velocity-dependent mass tensor.
That supports keeping both mechanisms in the effective Hamiltonian. Their molecular theorem is not a proof of
this gauge-field reduction or its cutoff limit. The g-power counting above follows from my coordinate scaling,
not from importing their mass-ratio parameter.

Primary source: https://arxiv.org/abs/0712.4369 (full text: https://arxiv.org/pdf/0712.4369).
The paper's existing Teufel reference and bibliography are unchanged.

## Verification record

- `scripts/check_lowest_note_review.py`: 26 focused algebra, saved-evidence and document checks. The new prose
  check first failed on the unedited threshold statement; the arithmetic checks already passed. This is an
  editorial red/green check, not a failed physics experiment.
- Numerical checks bind the displayed D_lin coefficients to the saved receipt and quoted anharmonic coefficient.
  Deliberately changing its separation, halving its potential coefficient or reversing its kinetic sign is rejected.
  Separate controls cover denominator signs, a tied ordering, the metric-inverse factor, Born--Huang scaling,
  displaced equilibrium and a switch whose control-point derivative differs from its root derivative.
- `scripts/check_lowest_note_format.py`: eight format/preservation checks. All 46 baseline statement bodies,
  statuses, math fragments and the bibliography remain intact. The separately named addition has two numbered
  equations and an unchanged COMPUTED status.
- Logs, receipts and full arithmetic are in `research/lowest-note-threshold-review-20261127/`. The `red`, `edited`
  and `preservation` folders are intermediate checks against the then-current sources and the incoming PDF;
  `bound-equation` additionally checks the displayed coefficients before the rebuild.
  Only `release-review` and `release-format` identify the rebuilt release PDF. The build log is saved separately.
  The release is 24 pages, with stable references and no LaTeX warnings or overfull/underfull boxes. All pages
  were rendered and visually checked, especially the new equations and the appendix's continued table.
- No solver, grid, full GIGI suite or physical acceptance gate is run or closed by this revision.
