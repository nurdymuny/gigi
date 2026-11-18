# Fold-Λ: the smallest stable piece of rim (spec)

2026-11-18. Fixed **before** measurement. This is idea 2 of
`docs/research/small-edges-2026-11-18.md`. Its provenance is Bee's Good Will Hunting
graph-fold article (matching menus, AP = PB; site commit `8eb87c00`), with Chan–Godsil
§5 on partition refinement.

## In plain English

One ring is the smallest rim. It has exactly as much information as unknowns, so it
mistakes roughness for curve. Two rings fix that, but they bring many more faces, and
many of them probably repeat what the fit already knows. The fold question is which
second-ring cells actually steady the fit. This spec measures, for every cell, how few
second-ring cells are needed to read the curve almost as steadily as the full two
rings. It also checks that *which* cells we pick matters, not just how many.

## Technical

**Amplification.** As in the paper (`make_data.py: conditioning`), the amplification of
cell K's fit is ‖R_K‖₂, the spectral norm of the map from neighbourhood face fluxes
(chart units) to the 11 coefficients. The own faces are held exactly; all others are
fitted by least squares (null-space method, `ricci_pg_tworing.fit_cell_rings`).

**The fold.**

- **Stencils.** For cell K, the one-ring set N₁ and two-ring set N₂ come from
  `neighbourhood_rings`, with N₂'s transported placements. The candidates are
  C = N₂ \ N₁.
- **Greedy.** Starting from N₁, repeatedly add the candidate whose inclusion gives the
  smallest ‖R_K‖₂. Stop at the first stencil with ‖R_K‖₂ ≤ 1.25 × ‖R_K‖₂(N₂).
  Record k_K, the number of candidates added, and the face count.
- **Whole cells, not single faces.** Each cell's four fluxes sum to zero, so a single
  face is not an independent datum.

**Meshes.** The paper's Abelian meshes (`pg.load_cells`), levels 0–3 (n = 1, 2, 3, 5).
The fit map depends on geometry only, so no field data enters.

## Gates

- **C-fit (control).** The rebuilt map reproduces `fit_cell_rings` for both N₁ and N₂
  to 1e-12 relative on every cell.
- **Fold-Λ (reported).** The distribution of k_K and of the face count, against the
  full |C| and the full two-ring face count, per level. The **saving** is
  1 − median(k_K) / median(|C|).
- **Removal: random selection (fixed now).** For each cell, take a random subset of C of
  the same size k_K (seed = cell index + 1000 × level), and record whether it meets the
  1.25 target. The claim "the choice of cells matters" is made **only if** random
  subsets meet the target in fewer than 50% of cells, at every level. Otherwise the
  finding is that the count matters and the choice does not, and it is reported that way.
- **Force rebuild (next rung, not here).** Only if the saving is at least 25% at every
  level do we rebuild the force on folded stencils and run BW05 and S3.

## Scope

Amplification only, which is a lower-bound proxy for error (the paper's measured ratios
to 1/σ_min are 1.03–2.17). Research script, not an engine verb.
