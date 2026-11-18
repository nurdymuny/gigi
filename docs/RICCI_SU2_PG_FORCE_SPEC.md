# S2a: the two-ring Petrov–Galerkin force for SU(2) (spec)

2026-11-17, for Bee. Fixed **before** any measurement. The rung above S1
(`docs/RECOVER_CURVATURE_SU2_RESULTS.md`, covariant curvature at second order).
Research script, not an engine verb.

## Question

The native SU(2) Whitney force stalls at a 25.5% full-flow error on the Berger
field, with negative orders (RF2C, `docs/RICCI_SHAPE_REGULAR_RESULTS.md`). Does
the paper's move fix it, non-Abelian? The move keeps the native test variations
exactly and replaces only the trial curvature with the covariant two-ring
recovery.

## Contract: unchanged from RF2C

- Meshes n = 1, 2, 3, 5 on S³/2I, from the frozen inputs
  `output/ricci-shape-regular-20261117/berger/level*_berger_q{4,6}.{input,output}.json`.
- Berger metric (3, 1, 1), with the exact links the native probe recorded.
- The native tangent mass `B` (the output's `blocks`) and the native volume.
- The continuum target `ricci_lattice_convergence.reference_velocity`, the CG
  solve, and the volume-normalized `B`-norm error, all through the unchanged
  `ricci_lattice_convergence.measures`.
- BW05: final relative error below 5%; both final observed orders at least 0.5;
  numerical error (q4 against q6, and solver) below a tenth of the spatial error.

Only the gradient handed to `measures` changes.

## The force

Per cell, the native action is `E = 2 Σ_ij M_ij ⟨D_i, D_j⟩`. Here
`D_i = U_i − 1` is the face defect quaternion of faces 012, 013, 023 and 123, with
face 123 carried to vertex 0 by `U₀₁`, and `M` is the metric Whitney face mass.
The native gradient is

```
a[η] = 4 Σ_i ⟨ ∂D_i/∂η , h_i ⟩,    with h_i = Σ_j M_ij D_j
```

over the 18 right-trivialised link variations η. That is a test variation
(`∂D_i/∂η`) paired with a trial curvature (`h`).

**Petrov–Galerkin:** keep `∂D_i/∂η` exactly and replace the trial.

- **Vector part:** `h_i^vec = ∫_K ⟨P_K, W_i⟩_g`. `P_K` is the covariant two-ring
  recovery of the registered face defects (vector part) over the neighbourhood,
  one closed linear field per su(2) component, built as in S1. `W_i` is the
  Whitney face function. The pairing uses the Berger metric and the same
  quadrature as `cell_integrals`.
- **Scalar part:** `h_i^0 = Σ_j M_ij D_j^0`, kept native. It is `O(F²)` and
  contributes `O(F³)` to the gradient.

Assembly onto quotient links uses the native rule: weight times `liftᵀ`, with
`lift = −Ad(U)` on reversed edges.

## Controls (must pass before any result is read)

- **C1 energy replay.** The Python face defects and `cell_integrals` face mass
  reproduce the native total energy to 1e-10 relative.
- **C2 gradient replay.** The Python gradient with the native trial reproduces
  the native `gradient` to 1e-9 relative, on every level and both orders.
- **C3 Whitney trial.** Feeding the recovery step the cell's own Whitney field
  instead of the two-ring recovery reproduces C2 exactly. This proves the pairing
  is the native pairing.

## Gates

- **G-force.** BW05 on the PG gradient, q6. The final relative error and the
  orders are reported whatever they are.
- **G-num.** The q4 and q6 PG errors differ by less than a tenth of the error.
- **G-baseline.** The native error recomputed by this script equals RF2C's
  recorded 25.5066% at n = 5, q6.

## Mechanism removals

- **M-trial:** native trial (C2), which must reproduce the 25.5% floor.
- **M-register:** no register in the recovery. Expect degraded accuracy or a broken
  gauge structure.
- **M-ring:** one ring. Expect accuracy close to two rings, but S2b stability to
  fail as in the paper.

## Out of scope

- S2b, stability: the generalized spectrum of the linearised PG operator against
  `B`. That is the next spec.
- Flow and time integration.
- Any engine verb.
- Any mass, spectral or physical claim.
