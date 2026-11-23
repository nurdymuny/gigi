# Bee's magnet idea: the geometry behind it, and two probes

2026-11-17. **Diagnostics, not gated features.** Python research
scripts; nothing ran through the GIGI engine. BW05 is not closed.

## The question

Bee: magnets lock things without forcing them, and they act *through* a layer
from opposite sides. She asked for the geometry behind the idea and whether it
supplies what the four earlier constructions lacked: a force that is
**reciprocal** (symmetric), **attracting in every mode** (no negative
eigenvalues), and **accurate**. Each earlier attempt had at most two of the three.

## The geometry (literature, verified by search)

- **Two meshes and a layer between them (STRUCTURAL).** Discretized magnetism
  puts flux on the faces of a primal mesh and the field on the edges of a dual
  mesh, coupled by a Hodge (material) operator: Yee, Weiland's Finite
  Integration Technique, Bossavit's *Geometry of Electromagnetism*.
- **The wall is a known no-go (RESONANCE).** Wardetzky, Mathur, Kälberer and
  Grinspun, *Discrete Laplace Operators: No Free Lunch* (SGP 2007): on general
  meshes no discrete Laplacian has symmetry, locality, linear precision and
  positive weights together. The proof uses Maxwell–Cremona reciprocal figures.
  Dropping **locality** leaves the other three attainable. Our constructions
  were all local. Caveats: their setting is scalar Laplacians on surfaces, their
  positivity is positive weights rather than a positive spectrum, and the full
  text could not be fetched (403/404), so this cites the paper's stated result.
- **Alignment without forcing.** Knöppel, Crane, Pinkall, Schröder, *Globally
  Optimal Direction Fields* (2013): fields align through the connection
  (magnetic) Laplacian rather than by imposed constraints.
- **A projection layer.** The variational multiscale method (Hughes 1995) and
  local projection stabilization (Braack and Burman) couple scales through
  orthogonal projections.

## Probe 1: a projection layer between the two sides

Receipt `output/ricci-pg-curvature-20261117/projection_layer_probe.json`,
script `scripts/ricci_projection_layer_probe.py`.

The force is the data's Whitney curvature, projected onto V, paired with the
original Whitney curvature of each test variation. V is the space of two-forms
whose left-invariant-frame components are continuous piecewise-linear, a layer
that locks across faces. The projection is self-adjoint, so the operator is
symmetric and positive semidefinite by construction. The test variations are
the original ones, and the operator is non-local.

Controls: the exact curvature projects onto itself to 1.2e-14; the operator is
symmetric to 3.8e-16.

| n | force error | projected curvature error | negative modes | zero modes (gauge) |
| --- | ---: | ---: | ---: | ---: |
| 1 | 1.73% | 1.60% | 0 | 79 (21) |
| 2 | 5.80% | 0.77% | 0 | 632 (163) |
| 3 | 5.77% | 0.45% | not computed | not computed |

**Result: reciprocal and never negative, and the floor falls from 26% to about
5.8%, but it is still a floor.** The projected curvature reaches only first
order (1.07, 1.31). The modes beyond gauge (58, then 469) have no restoring
force: the layer is too small to see them. Neutral, not unstable, but they fail
S3 as written. Caveat: this control's exact curvature is constant in the
left-invariant frame, which V represents exactly, so the probe tests the
mechanism and says nothing about generic fields.

## Probe 2: the other session's RF2H at level 0

Receipts `rf2h_forms_whitney.json` and `rf2h_forms_bdm.json`, script
`scripts/ricci_rf2h_forms_probe.py`, against RF2H code at commit 99882091.
Both RF2H constraints hold to about 1e-10 in every case.

| (Adj) field | Energy | Force error | Negative modes |
| --- | --- | ---: | ---: |
| Whitney | linearised (RF2H's reported case) | 44.27% | 48 / 121 |
| Whitney | quadratic | 836% | 0 |
| full BDM (the spec's pairing for the quadratic) | linearised | 1252% | 56 / 121 |
| full BDM | quadratic | 35,446% | 0 |

The first row reproduces RF2H's reported 0.44. The quadratic energy with the
full-field constraint is stable by construction and satisfies both consistency
conditions, yet is 354 times off at n=1. RF2H's spec notes that at n=1 the
stencil exceeds the quotient's injectivity radius, so n=1 may be outside the
regime its argument covers. **An n=2 build was attempted and did not finish in
25 minutes** (level 0 takes 59 s), so that question is unanswered and the
global solve's cost scaling is a real obstacle.

## Where this leaves the gap

The projection layer is the first construction with no negative modes that
also cut the floor substantially, but it still floors. The literature's next
lead is Maxwell's reciprocal figures: a mesh and its **orthogonal dual**, the
two layers on opposite sides. The catch: a dual construction wants both Hodge
layers taken from the same dual, while BW05 freezes the Whitney 1-form mass.
Whether that layer may change is a contract decision for Bee.
