# The inherited boundary is matching, not a wall

24 November 2026. Local derivation and numerical verification for the registered
Cartesian finite model. No boundary-complete spectrum or larger grid was run.

## The answer

The original model allows a configuration to pass continuously through zero
determinant. At a generic rank-two configuration, nothing in the original
Cartesian kinetic energy inserts a wall. The positive and negative determinant
regions meet there. A regular quantum state must have matching values and
matching flux across that interface.

The choice is not arbitrary. Section 2 of the
[original registration](helike-quantum-probe-preregistration-2026-11-23.md)
already specifies the regular Cartesian Friedrichs realization, restricted to
colour SO(3) singlets, with no extra boundary interaction. The later positive-
sheet approximation does not inherit a closed Dirichlet, Neumann or constant-
Robin wall from that parent domain.

**What is resolved:** the local matching rule at generic rank two.
**What is not built:** a spectrum solver that implements the complete inherited
domain. The earlier interior basis stays unchanged and is not relabeled as
boundary-complete.

## Derivation from the actual geometry

The colour-invariant coordinates are the pair

\[
G=qq^T,\qquad d=\det q,\qquad d^2=\det G.
\]

For positive-definite G there are two SO(3) orbits, distinguished by the sign
of d. At rank two they meet. This is a concrete orientation double cover,
not a measured phase transition or a claim that every earlier double-cover
phenomenon is the same mechanism. Keeping only G discards the orientation
that the signed cubic term actually uses. The invariant-polynomial distinction
between SO(3) and O(3) is standard; see
[Dhont, Cassam-Chenai and Patras, section 3](https://arxiv.org/pdf/1511.01311).

Where the first two rows are independent, use the oriented orthonormal frame
they determine. A representative of the orbit is

\[
L=\begin{pmatrix}r_0&0&0\\a&r_1&0\\b&c&w\end{pmatrix},
\quad r_0,r_1>0,\quad w\in\mathbb R,
\quad u=(\log r_0,\log r_1,w,a,b,c).
\]

Now \(d=r_0r_1w\). The signed coordinate w crosses zero smoothly. The first
two independent rows make the SO(3) stabilizer trivial, so this is a regular
quotient patch at w=0. Also \(\nabla_q d=\operatorname{cof}q\ne0\) at rank two.
The interface is not a singularity of the parent Cartesian space.

The old Gram density and the coordinate Jacobian combine as

\[
\rho=(r_0r_1|w|)^{-1},\qquad
|\det(dG/du)|=8r_0^4r_1^3|w|,
\qquad \mu=8r_0^3r_1^2.
\]

The apparent singularity cancels. To calculate the exact cometric, form the
9 by 9 tangent matrix from the six derivatives of L and three infinitesimal
colour rotations of L. Its determinant has magnitude \(r_0^3r_1^2\), including
at w=0. If K is the first six rows of its inverse, then

\[
B=KK^T,\qquad
\Delta_{\rm red}=\mu^{-1}\partial_i(\mu B_{ij}\partial_j).
\]

The implemented divergence term includes
\(\nabla_u\log\mu=(3,2,0,0,0,0)\). It is not a flat-coordinate Laplacian.
At the interface the mixed normal components vanish and

\[
B_{ww}=1+\frac{c^2}{r_1^2}
+\frac{(ac-br_1)^2}{r_0^2r_1^2}>0.
\]

Let h increase into each half: \(\psi_+(x,h)=\psi(x,+h)\),
\(\psi_-(x,h)=\psi(x,-h)\), h>=0. The inherited conditions are

\[
\boxed{\psi_+(x,0)=\psi_-(x,0)},\qquad
\boxed{\mu B_{ww}(\partial_h\psi_++\partial_h\psi_-)(x,0)=0}.
\]

The derivative sum, not difference, follows from the opposite half-coordinate
orientations. Values match in the local H^1 form domain. Conormal flux matches
in the operator domain, by integration by parts without an interface delta
interaction. Smooth tests use ordinary derivatives; weak domain statements
use traces. Form and operator domains must not be conflated; the general
framework is in [Teschl, section 2.3](https://www.mat.univie.ac.at/~gerald/ftp/book-schroe/schroe2.pdf).

## Why neither usual wall is inherited

Consider smooth invariant functions

\[
\psi(q)=[A(G)+dC(G)]\exp\!\left(-\tfrac12\sum_i\alpha_iG_{ii}\right).
\]

At w=0 their trace is A times the envelope, and their signed-w derivative is
\(r_0r_1C\) times the envelope. A and C are independently selectable. In
particular, Gaussian and determinant-times-Gaussian probes give independent
trace and derivative data. Multiplying by an invariant cutoff equal to one
near the chosen interface patch puts the same local examples in the compactly
supported Cartesian core.

Mandatory zero value excludes the first type. Mandatory zero derivative
excludes the second. A fixed Robin relation cannot cover all their linear
combinations. These are counterexamples to those universal wall rules, not
claims that those rules define no mathematical operator.

The two sides cannot be replaced by independently evolving even/odd sectors
at the control couplings either:

\[
V(w)-V(-w)=-2g\Big(\sum_i\omega_i\Big)r_0r_1w\ne0.
\]

The kinetic reflection is an isometry, but the signed cubic does not preserve
that reflection. The boundary extensions studied in
[Acharyya and Balachandran](https://arxiv.org/pdf/1702.06430) are not imported:
the original experiment explicitly excludes their extra reduced extension
choices. Our conclusion is for the chosen regular Cartesian model, not a
classification of all models with a related differential expression.

## Recorded same-control measurements

The specification and implementation were pushed as
**ffc544190afea4474cc82b3d400b8a0d39ba3f73** before this recorded run.
The receipt reports clean declared sources and retains snapshots.
Settings are unchanged: s=1.15, t=3, g squared=0.004 and 0.002.

For each coupling, five regular invariant polynomial-Gaussian probes were
evaluated at w=-0.2, 0 and +0.2, keeping the first two diagonal coordinates
from the same background. Gaussian widths scale the probes for numerical
visibility; they are not fitted changes to the Hamiltonian. Direct nine-
coordinate differentiation supplies the independent oracle.

| g squared | Largest absolute Laplacian disagreement | Largest absolute Hamiltonian disagreement | Largest scaled Hamiltonian disagreement |
| --- | --- | --- | --- |
| 0.004 | 6.9278e-14 | 1.1642e-10 | 2.7934e-16 |
| 0.002 | 6.3949e-14 | 1.3970e-9 | 4.5251e-16 |

Scaled disagreement means absolute disagreement divided by
\(1+|H\psi|\), not by an energy gap. The frozen oracle check is
absolute disagreement <=1e-9 times \(1+|H\psi|\). Reporting the absolute
numbers too matters because some unnormalized polynomial probes are large.

| g squared | Measured V(+0.2)-V(-0.2) | Independent signed-cubic formula |
| --- | --- | --- |
| 0.004 | -13.940374018575653 | -13.940374018575609 |
| 0.002 | -19.714666001623186 | -19.714666001623140 |

All five interface probes have zero recorded value jump and conormal sum.
An additional smooth complex probe has nonzero normal probability current
per coordinate density, **0.1353352832366127**, agreeing with the planted
value exp(-2). Its opposite half-conormals cancel exactly in the recorded
floating-point evaluation. It is an unnormalized test wavefunction, not a
stationary eigenstate, a tunnelling rate or a new excitation measurement.

The evidence is in
[controls-ffc54419](../../research/qspace-boundary-20261124/controls-ffc54419/).
Its status is `PASS_LOCAL_MATCHING`, with both spectral certification and
registered-grid release explicitly false.

## Gates and unchanged controls

- 18 boundary tests pass: the original 16 plus two saved-evidence/replay
  checks added after the measured run.
- All 18 existing positive-cone tests pass, including the SAME weak-coupling
  coefficient and original-reference weight controls and their saved matrices.
- Nine deliberate changes are caught by assertions, with zero incidental
  errors: erased orientation, wrong density, flat substituted metric, omitted
  density drift, absolute determinant, ignored value jump, wrong half-normal
  sign, imposed Dirichlet wall and imposed Neumann wall.
- The initial RED log is an honest missing-module/API failure, not a claimed
  numerical failure. The deliberate-defect logs supply the behavioral
  counterexamples. All logs and the intermediate first pass are retained.
- No model function, old solver, physical threshold or original receipt was
  edited. No full Rust suite, live-server deployment or bigger grid ran.

The earlier coefficient results remain -0.5308701642 and -2.7511293048.
The earlier competitor refinement discrepancy remains 0.0034559546 > 0.003.
Neither is reinterpreted as a result from a newly boundary-complete solver.

## The next engineering requirement is now specific

The old one-sided log-Hermite family has zero trace at the interface. Its
closure in the local form norm cannot approximate a regular state with
nonzero trace, because the trace map is continuous on a compact regular
patch. Thus more functions of that same vanishing-boundary type do not repair
the missing part of the inherited form domain.

A completion must carry both determinant signs and permit independent
nonzero trace and normal-derivative data joined by the derived rule. One
globally regular starting family already represented in the repository's
invariant machinery is \([A(G)+dC(G)]\) times a Cartesian Gaussian. A local
signed-chart basis is another possible implementation, but its chart overlaps
and coverage of lower-rank configurations must be justified, not assumed.
Neither option is implemented as a new spectral solver by this change.

To replace the positive-sheet approximation with the original Cartesian
completion, record that domain correction explicitly while preserving the
physical energy formula and thresholds. Then test boundary-compatible basis
coverage, exact charge identification and the unchanged gap-convergence
requirement before any registered grid. A guessed wall or a favorable local
matching receipt cannot authorize that grid.
