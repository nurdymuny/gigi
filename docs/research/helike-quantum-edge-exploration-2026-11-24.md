# Exploration: a quantum edge for Helike's channel in the sphere matrix model

24 November 2026. **EXPLORATORY: not a registered measurement and not an accepted convergence result.** Written to
decide what to pre-register next. Every number below is a finite-cutoff observation at cutoffs 10 and 12.

## Why I looked

The convergence run (`convergence-20261124T033143Z-aca2f5ca`) did not pass its residual gate, but its energies
and weights nearly settled, and they showed a pattern I recognised from *By Enough, But Not Too Much*. At g² = 0.5
and 1 the lowest excitation is a scalar, which Helike's directional channel H2 cannot reach. At g² = 2 it is the
spin-2 quintet, and H2 puts about 99.36% of its finite-basis response there across cutoffs 8 to 14. The N = 8
scalar-lowest reading of the first survey changes with the basis width: with the adjusted width the quintet is
lowest at every saved cutoff from 8 to 14. This shows finite-basis sensitivity, not yet a converged resolution
of that earlier ordering. Classically, the twist is the floor only in a window of the field's strength.
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
  g²_c ≈ 1.76 (linear interpolation between 1.75 and 2.0; 1.7699786411 at cutoff 10,
  1.7609417618 at cutoff 12). These are interpolation estimates, not measured roots or uncertainty intervals.
- **No upper edge observed at the sampled couplings through g² = 6.** The reported spin-4 level does not
  undercut the spin-2 level at those points. A second crossing between samples, beyond the sampled range, or
  after further refinement has not been excluded. A "not too much" has not appeared in these observations.

## What the levels are made of

For each level: size ⟨tr qqᵀ⟩, signed volume ⟨det q⟩ (a representative of the second classical vacuum orbit is
q = I/g, where det q = g⁻³), and weight in the highest retained polynomial shell. That shell is not the
omitted full-space residual or the discarded probe-response fraction. These are different measurements.

- **The scalar has a larger signed-volume expectation.** Its ⟨det q⟩ is 3.8 times the ground state's at g² = 0.5,
  falling to 1.9 times at g² = 6. A shift of probability toward the second vacuum is one possible interpretation;
  these means alone do not establish localization, a basin population, or a tunnelling mechanism.
- **The spin-2 level carries the quadrupole channel.** Its signed-volume ratio to the ground state is about
  1.12 to 1.21 at the sampled couplings. Its spin and H2 response establish the channel identification. Direct
  shape moments and their fluctuations are still needed to characterize the state's geometry.
- **A candidate explanation for the crossing.** In q coordinates, I/g approaches the origin as g increases,
  but the two vacuum orbits remain distinct at every finite g. In the original M = gq coordinates they stay at
  M = 0 and the orbit of M = I. At g² = 1.75, √3/g is about 0.75 of √⟨tr qqᵀ⟩ for the ground state. This
  ratio compares a distance with a root-mean-square size; it is not a localization test or evidence of merging.
- **A clue about the residual, not its demonstrated cause.** At cutoff 12 and g² = 0.5, the scalar's top-shell
  weight is about 114 times the tensor's (7.04×10⁻⁴ against 6.19×10⁻⁶). At g² = 1 and 2 the ratios are about
  5.57 and 1.36. The full residual also depends on the Hamiltonian's action and interference between shells.
  Shell weights alone do not prove why the residual gate fails or which basis change will repair it.

**The width hypothesis remains open.** The earlier wording claimed that a per-level width was ruled out.
The committed scan stores ground energy versus width, not the residual-versus-width table needed to check that
claim. Those measurements must be saved and replayed before drawing a conclusion. Even agreement of the best
widths on one grid would only establish that observation at those tested widths and cutoffs, not exclude every
per-level width or a different basis construction.

## Caveats

- Finite cutoffs 10 and 12; the interpolated crossing moved by 0.0090368794 between them. Not converged in the
  registered sense.
- Comparison with finite-volume glueball studies on the torus and the three-sphere is still needed. This scan
  does not establish novelty or agreement with any particular published crossing.
- Radius remains fixed at R = 1. This scan does not establish a running-coupling/size relation; changing g² is
  not automatically equivalent to changing the sphere's size under this contract.
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
4. **Convergence.** Test where the full residual is produced before choosing a repair. Higher degree with
   memory-bounded assembly, a different invariant envelope, and per-level width remain candidate approaches;
   none is established as necessary or sufficient by this scan.

The concrete next contract is the [confirmatory crossing and state-geometry specification](helike-quantum-crossing-preregistration-2026-11-23.md).
It is on review hold; the exploratory observations are not substituted for its held-out measurements.

## GIGI

The decomposition here is linear algebra on the saved arrays. GIGI becomes useful as the evidence grows: the level
descriptors (gap, spin, size, orientation, tail, channel weights) are written in the evidence plan's level-bundle
shape, `research/helike-quantum-probe-20261123/exploration-edge-20261124/levels.csv` (176 levels, 22 cases), ready
for an isolated server import (YG00–YG02). A covariance decomposition of those descriptors across couplings,
cutoffs and, later, Berger stretches should separate the two families seen here, the orientation mode and the shape
mode, without being told which is which. That is a fair first test of GIGI as an analysis tool on this data.
