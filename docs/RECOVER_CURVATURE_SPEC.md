# RECOVER_CURVATURE: spec

2026-11-17. The two-ring curvature recovery of the paper *Reading the Curve,
Not Counting the Points* (`docs/papers/two-ring-pg/`), made an engine verb.
Until now it existed only as `scripts/ricci_pg_tworing.py`.

## What it returns

For every tetrahedral cell of a closed mesh on a quotient of S³, it returns the
**closed linear curvature** `P_K(z) = c + B z` (`tr B = 0`, eleven coefficients)
recovered from the edge integrals of a 1-cochain. Each cell is solved by
equality-constrained least squares:

- the cell's own four face fluxes are held **exactly**;
- every other distinct face within `RINGS` face-steps is fitted in **least squares**;
- every neighbour is first carried onto the cell's sheet of the cover by the left
  quaternion multiplication fixed at a shared vertex.

It also returns the per-cell diagnostics the paper reports:

- `amplification`: the 2-norm of the linear map from neighbourhood fluxes to
  coefficients (`Λ_K`);
- `condition` and `rank` of the flux map;
- the neighbourhood size (`faces`, `cells`).

Coefficients are given in the cell's chart. The chart is a central projection
onto the affine 3-plane through the cell's vertices, with an orthonormal frame
from Gram–Schmidt on the edge vectors from vertex 0 (positive diagonal), centred
at the projected centroid and scaled by the cell's diameter. The frame is
canonical, so the coefficients are reproducible. Amplification, condition and
recovered fluxes do not depend on the frame's signs.

It does **not** compute the Petrov–Galerkin force, the mass matrices or the BW05
gate. Those stay in the research scripts.

## Surfaces

```
RECOVER_CURVATURE cells FROM edges ON value [RINGS 2];
POST /v1/bundles/{cells}/recover_curvature  {"edges": "...", "field": "value", "rings": 2}
```

Behind the `gauge` feature, which the production build enables via `halcyon`.

## Bundle shapes (refused by name if violated)

- **Cells bundle:** numeric `cell` = 0..N−1, each exactly once; numeric
  `v{i}_{k}` for vertex i ∈ 0..3 and quaternion component k ∈ 0..3, scalar first;
  numeric `e0..e5`, the global edge ids of local edges (0,1),(0,2),(0,3),(1,2),(1,3),(2,3);
  numeric `r0..r5`, 1 if the stored edge runs opposite to the local one.
- **Edges bundle:** numeric `edge` = 0..E−1, each exactly once, plus the
  numeric value field named in the request.

## Refusals

These are 422 over REST and an error over GQL, each naming the cell:

- a face not shared by exactly two cells (the mesh is not closed);
- shared vertices that do not coincide after transport (gap > 1e-12);
- a neighbourhood vertex outside the cell's projective chart;
- a flux map of rank < 11, or with condition number > 1e12;
- own face fluxes not reproduced to 1e-12 relative;
- `RINGS` outside 1..=3.

## Tests (as built)

Kernel, `tests/recover_curvature.rs`, on the 600-cell (built from the binary
icosahedral group, with shuffled local vertex order so `reverse` is exercised):

- **T1 exactness.** Planted closed linear fields are recovered to 1e-10, rings 1 and 2.
- **T2 one ring is exactly determined.** 16 faces and an interpolating residual; two
  rings read more faces, cannot interpolate rough data, and have lower amplification.
- **T3 own faces held.** On rough data the own-face residual is < 1e-11, and the
  misfit is carried by the rest.
- **T4 re-sheeting.** Each cell stored on its own random sheet (left
  multiplication): the recovery is unchanged to 1e-9.
- **T5 refusals.** Bad RINGS, open mesh, short cochain, a bent vertex
  (no coincidence) and a collapsed cell.

Parity, `e2e/probes/test_recover_curvature_parity.py`: the engine against the
paper's `fit_cell_rings` on the frozen meshes n = 1, 2, 3, 5, rings 1 and 2. It
compares coefficients (under a sign-canonical chart), amplification, condition,
faces and cells. The receipt is `research/recover-curvature-parity-20261117/parity.json`.

Live, `e2e/probes/live_recover_curvature.py`: a gigi-stream built from this
tree on a scratch port. The n = 1 mesh is ingested over HTTP; the script then
checks REST against Python, GQL with RINGS 1 (16 faces), GQL with the default
against REST, and the refusals (unknown key 422, missing bundle 404, missing
field 422, RINGS 9 422). The receipt is `research/recover-curvature-parity-20261117/live.json`.

## Mechanism removals (each turns a gate red)

- **M1 no transport** (`Mechanism.transport = false`): the re-sheeted 600-cell is
  refused for non-coincidence.
- **M2 no equality** (`Mechanism.equality = false`): the own-face residual exceeds
  1e-6, so T3 goes red.
- **M3 one ring forced:** T2 goes red, because two rings must read more than 16 faces. This follows by construction and was not run as a separate mutation.
- **M4 raw numpy chart:** parity coefficients disagree while amplification still
  agrees (`test_m4_...`). This shows the coefficient comparison really reads the
  coefficients.

## Implementation note

nalgebra 0.33's SVD reconstructed the rank-deficient 11×4 own-face matrix with
error 2.6e-3, which broke the equality constraint (T2, T3, T4 and T5 went red).
The kernel uses a one-sided Jacobi SVD instead; the matrices are at most about
60×11.
