# The next levels change what we can honestly say

23 November 2026, Pacific time. **COMPUTED, conditional numerical evidence.**

The four review corrections are implemented and tested. All six registered existing-control cases completed
on the GPU and passed saved-state replay. The most useful result is not just a smaller number: at cutoff 12,
including the next scalar level's measured error prevents a tensor-lower verdict that ignoring that error
would have produced. At cutoff 14 that same control resolves conditionally without dropping the error.

**Software verification: PASS. Scientific convergence: UNRESOLVED. Full held-out crossing experiment: NOT RUN.**

## What changed

1. Assumed separation is never promoted to verified separation. A planted missing-eigenvalue counterexample
   retains `certified=false`, `separation_verified=false`, and unconditional `ORDER_UNRESOLVED`, even with zero
   residual on every retained state. Conditional ordering is a separate field.
2. The neighbor means the next DISTINCT same-spin energy multiplet. For spin two, all five members are retained;
   another vector inside the same quintet is not a higher level. Target and neighbor residuals are measured.
3. Dimensionless spin validation stays separate from the energy-valued residual. No spin tolerance is added to
   the energy error. Invalid spin, missing neighbors, incomplete multiplets and unresolved repeated copies refuse.
4. Width comparisons use explicitly named baselines. Against the expanded-grid selected widths, the largest
   per-level residual improvement in the saved N=10 table is 1.1089676171. The separate 1.3040361304 comparison
   is the g-squared-2 ground residual at old width 2.0 versus new width 2.3. The smallest excited residual in that
   table is 0.3470552370, approximately 0.35, not a literal lower bound of 0.35.

The [protocol](quantum-order-controls-protocol-2026-11-23.md), source and tests were committed and pushed as
`3d0e2c3354c7320e3aae12ca312fad5bfe357779` BEFORE numerical execution. The Hamiltonian, numerical precision,
separation checks and acceptance thresholds were not changed during the run.

## The measured neighbors

The controls use coupling squared 1 at width 1.7 and coupling squared 2 at width 2.3. The latter is the expanded
exploration grid's ground-energy optimum, not the older width 2.0. Width is fixed across cutoffs and states.

Each residual is the full-operator block norm, not merely the residual of the finite matrix eigensolve. Scalar
blocks have rank one; tensor blocks have rank five. These are floating-point estimates, not certified enclosures.

| Cutoff | Coupling squared | Scalar residual | Next scalar residual | Tensor residual | Next tensor residual | Conditional order after both checks |
| --- | --- | --- | --- | --- | --- | --- |
| 10 | 1 | 0.3470552370 | 0.4809420266 | 0.3883492777 | 1.0407421417 | Scalar lower |
| 12 | 1 | 0.1588797046 | 0.3791573934 | 0.1796959328 | 0.5236817364 | Scalar lower |
| 14 | 1 | 0.1114756647 | 0.1651048938 | 0.0834419178 | 0.2832373454 | Scalar lower |
| 10 | 2 | 0.4167585753 | 0.7528789668 | 0.4278749495 | 1.2853824817 | Unresolved: overlapping bounds |
| 12 | 2 | 0.2230103147 | 0.4697300545 | 0.2036145132 | 0.8631380311 | Unresolved: separation-sensitive |
| 14 | 2 | 0.1261146351 | 0.2609084411 | 0.1173826832 | 0.5461825079 | Tensor lower |

The ordering at coupling squared 2 is NOT stable across all three registered cutoffs. Decreasing residuals are
observed here; they do not prove convergence or verify the assumed spectral separation.

## A real-data demonstration of why the correction matters

At N=12, coupling squared 2, the measured scalar energy is 9.34503667186361. Its residual is
0.22301031472047803. The next scalar energy is 11.066881400221074, with residual 0.46973005447392374.

The registered separation estimate subtracts that next-level residual:

\[
\beta_s = 11.066881400221074 - 0.46973005447392374
        = 10.59715134574715.
\]

The initial conditional scalar lower estimate is 9.30531698683071. Halving the assumed separation lowers it to
9.26559730179781. The tensor upper endpoint plus the registered numerical margin is 9.269004938442281.
The initial inequality holds; the halved-separation inequality does not. The recorded answer is therefore
`ORDER_UNRESOLVED`, reason `SEPARATION_SENSITIVE`.

If the measured next-level residual were incorrectly replaced by zero, the halved-separation scalar lower
estimate would instead be 9.287268860803005, and the code would report conditional tensor-lower. That is an
ablation of the error term using the saved control, NOT an additional physical run or an accepted estimate.
It shows a concrete consequence of requiring the missing measurement.

At N=14, coupling squared 2, the measured neighbor residual is included and both checks pass. The halved-
separation scalar lower estimate is 9.321663613042498; the tensor upper endpoint is 9.26777037268318, with
margin 0.00000009343531735369432. Their ordering survives the registered sensitivity check. The assumption
behind the separation is still an assumption; every unconditional ordering remains unresolved.

## What still fails

The original full-residual allowance is 0.01 times the larger of one and the absolute excitation gap.
At N=14:

| Coupling squared | Allowance | Scalar residual / allowance | Tensor residual / allowance |
| --- | --- | --- | --- |
| 1 | 0.0227002782 | 4.9108 | 3.6758 |
| 2 | 0.0301973759 | 4.1763 | 3.8872 |

Neither control passes the target-state residual gate. The tiny finite-matrix solver residuals, about
6 to 7 times 10^-17 at N=14 under the solver's normalization, do not contradict this: they test the finite
eigenproblem, whereas the full action tests the saved states under the untruncated oscillator Hamiltonian.
Solving the finite matrix accurately is not the same as converging the physical state.

This pilot does not measure the held-out couplings, construct N=16, implement the full state-geometry survey,
or establish a continuum Yang-Mills gap. It does not waive any QC00--QC14 gate in the
[crossing registration](helike-quantum-crossing-preregistration-2026-11-23.md).

## Tests and reproducibility

The focused software suite passed before launch and was repeated inside the run:

- 22 mathematical tests; 10 evidence and resource tests; 20 original tests; 31 amendment tests: **83 total**.
- **18 deliberate mechanism removals** each caused an assertion failure, with zero unexpected test errors.
- Behavioral RED receipts and original stubs are retained, including the documented fixture corrections.
- Small-cutoff full action agrees with a separate tuple/ladder-action oracle and the previous residual routine.
- Complete GPU spectra agree with CPU solutions; selected subspaces agree independent of eigenvector frame.
- Replay reconstructs the Hamiltonian, metric and Casimir, checks the full spectrum against a CPU eigensolve,
  and recalculates all 30 saved level-block residuals and their Gram matrices from the saved states.
- Tampering with reported residuals and their derived order together, or corrupting a saved residual Gram,
  is rejected. Incomplete, malformed or source-mismatched evidence replaces a prior verification PASS with FAIL.

No full Rust suite was run. The database engine and HTTP routes are unchanged. The large-case replay reuses
the production full-action routine; the separate ladder oracle is a small-cutoff test, not a second independent
large-cutoff implementation.

Run directory: [`controls-20261124T062338Z-1b0b1ad1`](../../research/quantum-order-controls-20261123/controls-20261124T062338Z-1b0b1ad1/).
The identifier is UTC; the run occurred on 23 November Pacific time. It contains the complete receipt,
verification PASS, source/spec/test snapshots and hashes, six matrix/state archives, immediate event and case
logs, stdout/stderr, resource samples and the repeated test receipts. Initial RED/GREEN evidence is in
[`tdd/`](../../research/quantum-order-controls-20261123/tdd/).

Hardware: NVIDIA GeForce RTX 5070 Laptop GPU, CUDA 12.8, Torch 2.11.0.dev20260108+cu128, float64.
Supervised elapsed time including prerequisites and replay: **595.625 seconds**. Peak sampled worker RSS:
**4,749,860,864 bytes** (4.42 GiB), below 8 GiB. Peak CUDA allocated: **429,377,536 bytes**;
peak CUDA reserved: **524,288,000 bytes** (0.49 GiB), below 4 GiB. No resource ceiling was raised.

From a checkout retaining the pinned sources, replay the saved run without generating new controls:

```powershell
python scripts/run_quantum_order_controls.py --verify research/quantum-order-controls-20261123/controls-20261124T062338Z-1b0b1ad1
```

`--run` instead starts a new six-case pilot and requires a clean checkout. It does not start the held-out survey.
If source files have since changed, restore/use the pinned source revision or a separate worktree; do not
rewrite the saved hashes to make an old receipt pass.

## Decision

Close the four-issue software correction gate and the six-control execution/replay gate. Keep the scientific
convergence and full crossing gates open. The pilot replaces a spacing-only feasibility guess with measured
neighbor errors, shows where the conditional ordering survives, and demonstrates one real control where
ignoring a measured error would overstate what the evidence supports.
