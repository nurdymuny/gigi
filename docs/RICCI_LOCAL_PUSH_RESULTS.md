# Local push audit: shared-face imbalance located

2026-11-17. **Diagnostic result only. No repair; correspondence and release
gates unchanged.** This extends the parked handoff with a bounded investigation
explicitly requested by Bee, not a restart of the repair program.

## Scope

Original RF2D Abelian control, level1/n=2/q6 only. Readout at the fixed original
chart position (0.27,0.23,0.23), chosen before inspecting contributions.
Containing cell2, local edge0/global edge103. Complete six-cell edge star:
0, 2, 3, 456, 458, 459 (figure labels C1 through C6).

The existing full mass coupling is global. Two adjoint solves on its cached
1124-unknown matrix attribute the SAVED velocity to individual cells. The
960-cell coarse case supplies an explicitly retained outside remainder; the
six-cell patch is not falsely treated as an isolated system. A second,
solve-free pass separates interiors and faces. No refinement sweep, Berger
run, nonlinear trajectory, new flow candidate, full suite, commit or deploy.
Measured computational passes: 3.93 seconds and 4.06 seconds, excluding
startup, tests and figure production.

## Observation

All numbers below are signed contributions to the two sideways readouts,
normalized by intended speed4. They are NOT independent cell trajectories,
and vector components must be summed before taking a magnitude.

| Contribution | Side direction1 | Side direction2 |
|---|---:|---:|
| Shared-face/interface terms | -0.201704072461 | +0.218286903040 |
| Cell-interior discrepancy | +0.001295850978 | -0.000913759549 |
| Reference interpolation discrepancy | -0.000185550329 | -0.000743747236 |
| Observed saved-velocity error | -0.200593771812 | +0.216629396256 |

Thus, at this point, the interface terms account for almost all the sideways
error, with small interior and interpolation offsets. This is an exact
accounting of this finite example, not a general convergence result.

At the first shared face in the preselected patch (face0, C1/C4), the first
sideways component gives an easily inspected pair:

- Smooth reference: -1.403078037942266 and +1.403078037942265; they cancel.
- Original discretization: -1.391273155029535 and +1.409415245515281;
  the remaining contribution is +0.018142090485747.

The picture displays ALL six internal shared-face pairs, not only this pair.
The pair was selected by face order, not by largest error. These six internal
pairs contribute (+0.017270496198, +0.017402524357); pairs crossing the patch
boundary contribute (-0.063047335485, -0.141280918402); remaining faces
contribute (-0.155927233173, +0.342165297086). The outside accounting matters.

## What was separated

For the original Whitney curvature, its metric-dual field is evaluated on
both sides of each shared face. Integration by parts splits the same energy
derivative into interior and boundary terms. Pairing boundaries exposes
remainders that cancel for the known smooth field but survive for the
discretized field. The test field, orientation, deck identifications and mass
coupling are identical in both comparisons.

This locates the dominant contribution in **shared-face transfer of the
metric-dual curvature**, not a missing topological connection. It does not
establish which replacement fixes that transfer or whether the entire frozen
contract admits the required convergence. Do not simply delete boundary terms:
they are part of the actual energy derivative, and removing them fails the
accounting identity. No smoothing or discarded modes are justified here.

## Verification retained

- Independent assembled mass versus cached mass: relative difference2.50e-16.
- Reassembled force versus cached mass times SAVED velocity: relative6.13e-13.
- Independent direct per-cell force agreement: maximum absolute1.78e-14.
- Interior plus boundary versus direct force: maximum absolute7.55e-15.
- Smooth paired-face cancellation over1920 shared faces: maximum4.80e-14.
- Whole action-error split closure: absolute1.02e-14.
- Attribution versus SAVED point error: absolute7.83e-13.
- Eight focused diagnostic tests passed after initial unimplemented failures.
  Four explicit mechanism removals each caused the required assertion failure:
  assembly orientation, closure checking, boundary terms, outward normals.
  Real-data missing-cell and wrong-orientation evidence was also rejected.
- Dropping boundary contributions changes the reconstructed sideways action
  error by0.297209866738 in normalized-speed units. It is not a valid repair.

No full regression or customer-facing endpoint claim is made.

## Evidence and scripts

`output/ricci-local-push-20261117/` retains source/data hashes, local forces,
adjoint readouts, per-cell and per-face arrays, checks and rejected evidence:

- `receipt.json`, `attributions.npz`
- `interface-receipt.json`, `interfaces.npz`
- `focused-checks.json`
- `shared-face-pushes-v3.png`, `plot-receipt-v3.json`

Original commands (fresh output only; reruns refuse to overwrite receipts):

```text
python scripts/ricci_local_push_audit.py
python scripts/ricci_push_interface_audit.py
python scripts/ricci_push_focused_checks.py
python scripts/plot_ricci_local_pushes.py
```

Specs: `RICCI_LOCAL_PUSH_AUDIT.md` and the separately frozen
`RICCI_LOCAL_PUSH_INTERFACE_ADDENDUM.md`. Prior RF2D/F/G evidence unchanged.
The exact consistency question in `RICCI_HANDOFF.md` still must be answered
before another repair attempt. **Stop here.**
