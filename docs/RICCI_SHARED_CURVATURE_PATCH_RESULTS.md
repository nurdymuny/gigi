# Shared curvature on the original six-cell patch

2026-11-17. **Local construction verified; global repair NOT established.**
Spec: `RICCI_SHARED_CURVATURE_PATCH_SPEC.md`. This was the authorized bounded
follow-up, not a refinement sweep or a continuation of the full flow repair.

## What was constructed

The six neighboring cells now consult one common closed curvature polynomial,
instead of independently reconstructing six lowest-order curvature fields.
Its metric-dual field is computed from that polynomial and the original round
metric. No face trace is averaged, fitted independently, or overwritten.

The original signed edge observations, cover images, geometric mapping, clock,
Whitney tangent mass and acceptance tolerances are unchanged. We do change the
pointwise curvature reconstruction between observations. Preserving the old
pointwise field would preserve its known discontinuities.

The construction uses the same cells 0, 2, 3, 456, 458, 459. There are eight
vertices, nineteen oriented edges, eighteen faces and six cells. Incidence
ranks are 7, 12 and 6; both boundary-of-boundary products vanish exactly.

### Smallest within the declared family

The degree was chosen from geometry alone, before fitting the observations.
The entire twelve-dimensional discrete closed-flux space must be represented,
not merely the one control vector.

| Common curvature degree | Closed coefficients | Face-flux rank | Sufficient |
| --- | ---: | ---: | --- |
| Constant, 0 | 3 | 3 | No |
| Linear, 1 | 11 | 11 | No |
| Quadratic, 2 | 26 | 12 | Yes |

The quadratic map's closed-space residual is 2.403e-15. This is numerical
rank verification under the declared relative 1e-12 policy, not a proof that
quadratics are minimal among all conceivable constructions.

Among exact-flux fits, a small dense solve minimizes the original-metric
curvature energy evaluated with q6 quadrature. The mass condition number is
777,258.58; the reduced constraint-system condition number is 149.77, both
below the unchanged 1e12 cap. There is no regularization or target-force input.

Polynomial homotopy recovers a connection with this curvature. An exact
vertex-gradient correction restores every original edge integral without
changing curvature. The common-chart transition is projective: the code uses
its position-dependent Jacobian, not an affine approximation. Each cell
independently applies the two-form pullback and its original physical metric.

## Measured local result

All absolute acceptance tolerances remain 1e-9. The derivative comparison uses
the specified relative-to-max(1, derivative) error. These are maxima on the
retained original Abelian observations, unless stated otherwise.

| Check | Measured residual |
| --- | ---: |
| All nineteen original edge integrals | 5.551e-17 |
| All eighteen oriented face fluxes, including exterior faces | 1.648e-17 |
| Polynomial closure, coefficient by coefficient | 1.388e-16 |
| Curl of recovered potential versus curvature coefficients | 1.110e-16 |
| Independent edge circulation versus direct face integration | 4.120e-18 |
| Zero curl of the vertex-gradient correction | 5.551e-17 |
| Metric-dual constitutive identity | 7.373e-17 |
| Full energy derivative, finite differences on every edge | 5.457e-13 |
| Energy Hessian symmetry | 8.100e-13 |
| Constraint-null stationarity | 3.499e-17 |
| Largest internal shared-face physical trace jump, q6 | 1.136e-14 |
| Largest internal jump at seven off-quadrature points per face | 5.602e-15 |

All six internal faces pass both trace agreement and all three original
Whitney test moments. The old field fails on all six. The old maximum in this
receipt is 0.0917789; the previous audit sampled a differently oriented Duffy
node set and reported 0.0916634. These sampled maxima are not supremum norms;
no threshold or physical field was changed to obtain this small difference.

Three fixed-seed arbitrary edge vectors also preserve all edge and face data
and pass closure, curl and trace checks. Their largest internal trace residual
is 1.846e-11. Zero data and a pure vertex-gradient control yield zero curvature
within tolerance. These are general-purpose algebraic controls, not new
physical observations or additional manifolds.

The reported energy, 0.0009657146143616385, belongs to this patch and q6
reconstruction. It is not a full-manifold energy or evidence of flow accuracy.

## Boundary limitation: do not splice this into the old mesh

Agreement has been enforced INSIDE the patch. It does not impose agreement
with the unchanged cells OUTSIDE it. We evaluated those original exterior
neighbors, rather than comparing only with the old inside field.

| Largest physical boundary trace mismatch at the sampled nodes | Value |
| --- | ---: |
| Old inside versus unchanged outside | 0.10965440884831837 |
| Reconstructed inside versus unchanged outside | 3.738094878646989 |

The exterior integrated fluxes remain correct, but pointwise boundary behavior
is substantially worse. An isolated insertion could introduce new interface
terms and distributional defects; it is NOT an acceptable global repair.
This result establishes local compatibility with exact retained observations,
not closeness to the unknown true curvature or strong adjoint consistency.

**The next unresolved question, not attempted here:** can such a closed,
data-preserving reconstruction be extended compatibly between patches, with
controlled energy derivative relative to the ORIGINAL Whitney test space?
Internal continuity alone does not answer that question. Do not increase
degree/radius or rerun full flows automatically.

## Gates and evidence

- Saved RED receipts: nine missing algebra mechanisms, four missing patch
  constructions, and two evidence tests before their implementations.
- Final focused GREEN: fifteen tests on the same final sources.
- Six deliberate removals all trigger assertion failures: closure, exact flux
  enforcement, edge-gauge correction, two-form pullback, full derivative and
  stale-PASS cleanup. None is counted merely because the test crashed.
- Six real-receipt attacks reject changed coefficients, missing constraints,
  forged agreement, source drift, nonfinite data and duplicate JSON keys.
  Every attack removes its pre-existing VERIFIED marker.
- Fresh-process reconstruction/replay agrees with the retained receipt.
  The replay reruns the same numerical implementation from original inputs;
  separate curl, Stokes and finite-difference checks provide distinct oracles.

Evidence directory: `output/ricci-shared-curvature-20261117/`.
`final-gate.json` records source and artifact hashes. `diagnostic.json` retains
coefficients, potential, vertex gauge, input values, incidence matrices, rank
decisions, every face result and all controls. Its SHA256 is
`b37c290a021504763b4289f687a6f29326c72ce73ddd11f1377656c68e45c0fe`.

Base HEAD: `0a64e2f848693221bf25f330ea4fb19041249106`. The worktree is dirty;
source hashes, not HEAD alone, identify this experiment. Earlier RED/GREEN
receipts intentionally retain their earlier source hashes; the final gate
pins the final implementation.

Commands used:

```powershell
python scripts/ricci_shared_curvature_tdd.py red
python scripts/ricci_shared_curvature_tdd.py green
python scripts/ricci_shared_curvature_patch_tdd.py red
python scripts/ricci_shared_curvature_patch_tdd.py green
python scripts/ricci_shared_curvature_evidence_tdd.py red
python scripts/ricci_shared_curvature_evidence_tdd.py green
python scripts/ricci_shared_curvature_mutations.py
python scripts/ricci_shared_curvature_gate.py
```

RED commands were run before implementation. Receipt-producing commands refuse
to overwrite prior experiments. To replay the retained mathematical receipt:

```powershell
python scripts/ricci_shared_curvature_patch.py verify
```

No broad suite, Rust/native changes, server, full-mass solve, Berger experiment,
commit or deployment. BW05 remains OPEN with its original meshes, mapping,
mass, clock and thresholds. **Stop at this local result.**
