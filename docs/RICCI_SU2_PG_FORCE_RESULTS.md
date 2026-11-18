# S2a: the two-ring Petrov–Galerkin force for SU(2): results

2026-11-17. Spec, fixed before measurement: `docs/RICCI_SU2_PG_FORCE_SPEC.md`.
Research script `scripts/ricci_su2_pg_force.py`, **not an engine run**. Receipts:
`research/su2-pg-force-20261117/`. The contract is unchanged from RF2C: frozen
native inputs and outputs, Berger metric (3, 1, 1), exact links, native tangent
mass and volume, and the continuum target and B-norm error from
`ricci_lattice_convergence.measures`. Only the gradient changes.

## Controls (all passed before any result was read)

| control | requirement | measured |
| --- | --- | --- |
| C1 energy replay | ≤ 1e-10 | 1.5×10⁻¹⁶ (n = 1) |
| C2 gradient replay (native trial against native gradient) | ≤ 1e-9 | 4.5×10⁻¹⁰ (central differences, ε = 1e-7) |
| C3 Whitney trial through the PG pairing | equals C2 | identical |
| G-baseline: native error recomputed | RF2C 26.2459% at n = 1 | 26.2459% |

## Result: relative full-flow error, Berger field, q6

| force | n = 1 | n = 2 | n = 3 | n = 5 | observed orders |
| --- | ---: | ---: | ---: | ---: | --- |
| native SU(2) Whitney (RF2C) | 26.25% | 24.74% | 24.96% | 25.51% | 0.04, −0.04, −0.05 |
| **two-ring Petrov–Galerkin** | 36.25% | 7.43% | 4.69% | **2.51%** | 2.37, **1.14, 1.23** |
| removal: one ring | 33.70% | 9.96% | 5.70% | 2.82% | 1.82, 1.39, 1.38 |
| removal: no register | 39.02% | 28.72% | 28.95% | 29.19% | 0.42, −0.03, −0.02 |

Orders are computed from the absolute B-norm error, as in BW05.

**BW05 numerical gate: PASS for the two-ring force.**
- **Final error:** 2.51% < 5%.
- **Final two orders:** 1.14 and 1.23, both ≥ 0.5.
- **Quadrature:** the q4 and q6 relative errors differ by at most 1.4×10⁻⁷, below a tenth of the error at every level.
- **Solver:** the largest relative linear residual is 8.7×10⁻¹³.

The native force does not pass. Its error stays at 25% and its orders are
negative.

## What the removals say

- **Register is essential.** Without it the force stays at about 29%, no better
  than the native floor and without convergence. The covariant transport is what
  lets the recovered curvature act as a force.
- **One ring is accurate as well, at 2.82% at n = 5,** just as in the Abelian
  paper. Accuracy does not separate one ring from two. In the paper, stability
  did. S2b answers that for SU(2).
- **n = 1 is pre-asymptotic for every recovery,** at 34–39%, because the stencil
  is large relative to the 120-cell space. It is excluded from the gate by
  BW05's final-two-orders rule.

## Not claimed

- **Stability (S2b):** the linearised PG operator's spectrum against `B` is not
  yet computed. The paper's lesson is that accuracy without stability is not a
  usable force.
- **The rest of the dynamics:** no flow, no time integration, and no metric other
  than Berger (3, 1, 1).
- **No engine verb, and no release gate.** Correspondence is not established.
  This is a numerical-gate result for the instantaneous force only.
