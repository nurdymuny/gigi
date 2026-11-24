# Exploration: a quantum edge for Helike's channel in the sphere matrix model

24 November 2026. **EXPLORATORY: not a registered measurement and not an accepted convergence result.** Written to
decide what to pre-register next. Every number below is a finite-cutoff observation at cutoffs 10 and 12.

## Why I looked

The convergence run (`convergence-20261124T033143Z-aca2f5ca`) did not pass its residual gate, but its energies
and weights nearly settled, and they showed a pattern I recognised from *By Enough, But Not Too Much*. At g² = 0.5
and 1 the lowest excitation is a scalar, which Helike's directional channel H2 cannot reach. At g² = 2 it is the
spin-2 quintet, and H2 puts 99.36% of its response there, steady across cutoffs 8 to 14. The N = 8 scalar-lowest
reading of the first survey was a truncation artifact: with the adjusted width the quintet is lowest at every cutoff
from 8 to 14, 0.076 below the scalar. Classically, the twist is the floor only in a window of the field's strength.
Here the coupling seemed to play that role. So I scanned it.

## What the scan shows

`scripts/explore_ym_helike_edge.py`; width by the registered ground-energy rule at cutoff 10, with a wider grid
because g² = 2 had selected the old grid's edge. Gaps in units of 1/R, cutoff 12:

| g² | 0.5 | 0.75 | 1.0 | 1.25 | 1.5 | 1.75 | 2.0 | 2.5 | 3.0 | 4.0 | 6.0 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| lowest scalar Δ₀ | 1.561 | 1.958 | 2.271 | 2.527 | 2.744 | 2.932 | 3.097 | 3.377 | 3.610 | 3.986 | 4.543 |
| lowest spin-2 Δ₂ | 2.334 | 2.493 | 2.625 | 2.740 | 2.842 | 2.935 | 3.021 | 3.175 | 3.311 | 3.549 | 3.933 |
| Δ₂ − Δ₀ | +0.773 | +0.535 | +0.355 | +0.213 | +0.098 | +0.0035 | −0.076 | −0.203 | −0.299 | −0.437 | −0.609 |

- **A lower edge.** The spin-2 level, the one Helike's directions reach, becomes the lowest excitation at
  g²_c ≈ 1.76 (linear interpolation between 1.75 and 2.0; 1.772 at cutoff 10, 1.761 at cutoff 12).
- **No upper edge up to g² = 6.** The spin-4 level sits near twice the spin-2 gap and never undercuts; the
  tensor's lead grows with coupling. In this model the quantum version is "by enough"; a "not too much" has not
  appeared in this range.

## What the levels are made of

For each level: size ⟨tr qqᵀ⟩, orientation ⟨det q⟩ (the second classical vacuum sits at q = I/g, where det q =
g⁻³), and the weight in the top polynomial shell (the truncation tail).

- **The scalar is a "toward the other vacuum" mode.** Its ⟨det q⟩ is 3.8 times the ground state's at g² = 0.5,
  falling to 1.9 times at g² = 6. At weak coupling the second vacuum lies outside the ground state's spread and a
  shift toward it is cheap.
- **The spin-2 level is a shape mode.** Its ⟨det q⟩ is 1.1 to 1.2 times the ground state's at every coupling: a
  quadrupole deformation, which is what Helike's two quadratures and their rotations are.
- **Why the crossing.** As g grows the two classical vacua merge (q = I/g moves toward the origin), the scalar
  loses its "other vacuum" character and stiffens, and the shape mode becomes the floor. At the crossing the
  second vacuum's distance √3/g is about 0.75 of the ground state's radius √⟨tr qqᵀ⟩. This is a reading of the
  data, not yet a tested mechanism.
- **Why the residual gate fails.** The scalar's top-shell weight is about 100 times the tensor's (7×10⁻⁴ against
  6×10⁻⁶ at g² = 0.5): it lives away from the Gaussian's centre, toward the other vacuum, and needs high degree.

**A hypothesis this falsified.** I expected the scalar excitation to need a different width from the ground state.
It does not: at cutoff 10 its residual is smallest at the same α as the ground state's (α = 1.7 at g² = 1, 0.347;
α = 2.3 at g² = 2), and about 3.5 times larger at every width. A per-level width will not close the residual gate.

## Caveats

- Finite cutoffs 10 and 12; the crossing moved by 0.011 between them. Not converged in the registered sense.
- A scalar–tensor crossing as the coupling (equivalently, the sphere's size) changes resembles level crossings
  known for glueballs in finite volume (small-volume studies on the torus and the three-sphere). Before calling
  anything new, the literature has to be checked; I have not done that here.
- This is the homogeneous sector only (Helike's j = 0 member) on the round sphere. It is not the Berger setting of
  the Floor theorem.

## What I would pre-register next

1. **The quantum edge, confirmatory.** Held-out couplings around g² = 1.6 to 2.0 (not the scan's grid points),
   cutoffs 12, 14, 16 at the frozen width, with the registered thresholds, a prediction of g²_c ∈ [1.70, 1.82]
   fixed from this exploration, and a planted control (a model in which the crossing is absent).
2. **The bridge to the edge paper.** The classical Hessian of this potential on the circle-symmetric fields
   M = diag(a, b, b), with gauge and symmetry zero modes removed, gives a classical j = 0 edge in the (a, b) plane
   where Helike's |m| = 2 plane holds the floor. The quantum vacuum's distribution over singular values then says
   where the vacuum lives relative to that edge. Test: does the quantum floor switch (g²_c) when the vacuum's weight
   crosses the classical edge? The conventions must be matched to *By Enough* (operator, gauge slice) first.
3. **Berger.** The same scan in the homogeneous model on (t, 1, 1), where Helike is the proven classical floor
   for t ∈ [2.75, 10]: there the spatial rotation group breaks to the circle, and Helike's |m| = 2 component
   separates from m = 0 and ±1.
4. **Convergence.** The residual gate needs a basis that reaches toward the second vacuum (higher degree with the
   memory-bounded assembly, or a gauge-invariant weight that leans toward det q > 0), not a per-level width.

## GIGI

The decomposition here is linear algebra on the saved arrays. GIGI becomes useful as the evidence grows: the level
descriptors (gap, spin, size, orientation, tail, channel weights) are written in the evidence plan's level-bundle
shape, `research/helike-quantum-probe-20261123/exploration-edge-20261124/levels.csv` (176 levels, 22 cases), ready
for an isolated server import (YG00–YG02). A covariance decomposition of those descriptors across couplings,
cutoffs and, later, Berger stretches should separate the two families seen here, the orientation mode and the shape
mode, without being told which is which. That is a fair first test of GIGI as an analysis tool on this data.
