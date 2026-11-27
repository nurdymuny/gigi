# Ground-weighted carrier geometry: bounded registered probe

Written before implementing or running this probe. The scalar residual enrichment did not repair the
registered reference-weight failure. This test asks whether the reference mismatch accompanies a
change in the excitation's energy direction, using the existing M8 geometry rather than a new selector.

## Objects and fixed questions

Use the saved Mq 12 ground and competitor at s=1.18, t=3, g^2=0.03 from
`research/flux-support-20261126/support.json`, LF digest
`15ed4b72a41ab4f1125fbe3a3bb3b6f35775c0ef077f1fc3f8b8f1a8b3969833`.
The source's parent receipt is pinned at
`b790b53f6186b6d3b1767c91cc1f087486d2db98c2bd291fce14db40524210a6`.
The seen reference score is 0.9128515410191 and dressed projection is 0.9740076181555388.
Neither is a newly predicted result. No state is re-solved, truncated, fitted, or selected to improve a score.

Read the actual configuration geometry from the exported Cholesky chart, frequencies and normal-mode
roles. Reconstruct G0 from the saved Cholesky coordinates, and M=J0 S from the saved chart differential.
Verify that the existing Python chart reproduces the exported origin and S. Keep the original metric,
measure, source potential and energy shift.
Fix the irrelevant global ground-state sign by requiring its overlap with the chart's Gaussian to be
positive (the coefficient of the zero-occupation basis vector). A zero overlap refuses; no pointwise
sign changes or absolute-value repair is allowed.

With G the saved ground state and psi the saved competitor, read f=psi/G. The object measured is its
gradient in the ground-weighted geometry, not the eigenvalue alone. The M8 identity is

    <psi,(H-E0)psi> = (1/2) integral grad(f)^T g grad(f) G^2 dmu
                     + <f^2 G,(H-E0)G>.

The residual term is retained; finite ground states are not declared exact. Positive energy shares use
M8's symmetric metric square root in its fixed background-normalized frame. Alignment uses the Gram
inverse of the ground-weighted centered physical coordinates, not division of the bare score by Z.

## Acceptance and resource limits

This is a one-point competitor diagnostic, not a carrier path, crossing test, domain certificate, or
amendment of the old grid gate. The old 0.95 rule remains FAIL at the saved point whatever this probe finds.

Use the existing M8 amendment-4 order rule: Mq 12 gives orders (14,16), one pair only. Use amendment-5
cuts 1e-12 and 1e-14, with the positivity requirement at 1e-12. No search for a passing cut or order.
Integrate in slabs without changing nodes, weights, or the global vacuum norm used to select the region.
First compute positivity and normalizations. If positivity fails at either order, report OPEN with the
offending weight and nodes and stop before the expensive energy reading. Do not divide through the bad
region, take an absolute value, discard significant nodes, or replace the ground state.

If positivity holds, run the unchanged M8 geometric readings with these existing criteria:
- Q0a energy identity relative error <=1e-4 at the fine order.
- Q0b share change <=1e-3 across the fixed order pair.
- A5 share cut change <=1e-3 at the fine order.
- Q2 alignment agrees with the saved independent dressed projection within 1e-6.
- Normalized positive/raw share sums agree with 1 to 1e-10; norms agree with saved coefficient norms to 1e-8.
- The competitor is ALIGNED only if its own C block is largest and its share >=0.5, as in M8.

The result is RESOLVED_ALIGNED, RESOLVED_NOT_ALIGNED, or OPEN, kept separately from the old gate.
At a single point it cannot establish persistence, transport, exact charge-sector membership, or absence
of an avoided crossing. High alignment does not itself prove why the old score is low.

Use no new eigensolve or full grid. Cap this probe at 900 seconds, with progress logged by slab, fixed
four BLAS threads, and at most one coarse/fine reading. On timeout preserve partial evidence and OPEN.

## Test-first checks

1. Exact width-deformed Gaussian: the old rescaled overlap is 8/9, while the same first-excitation
   geometry has own share 1, ground-weighted alignment 1, gap 2 and zero ground residual term.
2. A planted genuine mixture has the analytically prescribed energy shares and projection. Geometry
   must not label it pure merely because the ground state deformed.
3. Slab Hermite values and all six derivatives agree with the existing full product-grid implementation.
4. Partitioning the grid cannot change the global cut or positivity result. Dropping the global norm
   must make a fixture fail. No local per-slab renormalization.
5. Existing metric-frame invariance and inexact-ground residual tests remain passing.
6. Bad positivity, altered input hashes, wrong energy identity, mismatched alignment, missing rows,
   nonfinite evidence, or changed thresholds cannot yield RESOLVED_ALIGNED or retain a stale PASS.

Deliberately remove a derivative, the ground-weighted geometry, and the global cut normalization;
the corresponding gates must fail. Record all failures and passing focused checks before the real probe.
This geometric strategy is accepted at the real point only if its stated gates pass, not because its
unit tests pass. No production decision rule changes in this experiment.
