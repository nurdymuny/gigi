# Bounded local push audit

2026-11-17. Diagnostic only; the repair and release gates remain parked/open.

## Fixed scope before observing contributions

- Original RF2D Abelian control only, level1 (n=2), quadrature6: 960 cells,
  1124 edge unknowns. No refinement sweep, Berger run, build or broad suite.
- Observe the midpoint of the prior crossing: original flag-chart coordinates
  (0.27, 0.23, 0.23). This is selected geometrically, not by the error size.
- Patch: the complete star of local edge0 of the containing tetrahedron.
  At most12 cells. Preserve every outside-cell contribution as a separate
  remainder; the mass inverse is global, not cell-local.
- Reuse the verified cached mass. At most two diagnostic adjoint solves with
  the existing solver/tolerances. These attribute a saved velocity; they do
  not generate a new flow candidate. One quadrature6 cell-accounting pass.
- Hard worker timeout180 seconds. No production or frozen-source edits.

## Attribution contract

Use each original oriented Whitney cell's mass M_T (em sub tee), stiffness
K_T (kay sub tee), exact edge data a_T (ay sub tee), and independently
integrated smooth continuum load L_T (ell sub tee). Actual force is -K_T a_T;
the sampled-reference load is M_T times the existing reference velocity.

For each of the two local sideways readouts ell (ell), solve M z = ell
(em zee equals ell). M is the ORIGINAL full assembled mass. Each cell's
contribution to the readout error is z_T dotted with
(-K_T a_T - M_T v_ref,T). Assembly signs and quotient weights are retained.
Split that contribution into:

1. Curvature/action load mismatch: -K_T a_T - L_T.
2. Reference interpolation mismatch: L_T - M_T v_ref,T.

All contributions must sum to the readout of the SAVED velocity minus the
reference. Arrows show contributions to this observation, NOT independent
physical cell trajectories. Normalization uses the known reference speed4,
identically for every cell; it does not fit or repair the answer.

## Focused evidence checks

Before the real-data pass, planted accounting fixtures must detect an omitted
contribution, a reversed assembly orientation, and nonfinite evidence.
Real-data checks: source hashes, assembled mass versus cached mass, actual
force versus cached mass times saved velocity, continuum load versus saved
independent load, readout reconstruction, two-way residual decomposition,
and whole-mesh attribution closure. Absolute readout tolerance1e-9; matrix
and load comparisons relative1e-10. Corrupt/missing evidence must not leave
a verified receipt. No passing receipt is a convergence or repair PASS.

## Interpretation limits

This can locate contributions to one saved point's error. It cannot alone
prove a general defect, diagnose an interface traction jump, or establish a
replacement operator. A force attribution is decomposition, not an intervention
proving causation. No smoothing, refitting, thresholds changes, or new repair.
