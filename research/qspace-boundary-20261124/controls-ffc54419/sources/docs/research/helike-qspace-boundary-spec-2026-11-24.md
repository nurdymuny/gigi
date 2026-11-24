# Boundary inherited from the Cartesian q-space model

24 November 2026. Specification before implementation. Baseline `351007b0`.

## Frozen contract and scope

The [original quantum probe registration](helike-quantum-probe-preregistration-2026-11-23.md),
section 2, selects the regular Cartesian Friedrichs form on all of R^9,
restricted to colour SO(3) singlets, without a boundary interaction. The later
[flux registration](helike-flux-background-registration-2026-11-24.md) restricts
attention to det(q)>0 but does not give an autonomous boundary condition.
Do not silently identify these two domains.

Keep the sourced Berger differential expression, sources, normal frame, control
points s=1.15,t=3,g squared=0.004 and 0.002, coefficient tolerance 0.03, reference
weight requirement 0.95, overlap resolution 1e-6, registered gap tolerance 0.003,
registered weight requirement 0.9 and switch tolerance 0.015 unchanged.

This stage derives and tests the local interface rule inherited from the
original Cartesian domain. It does NOT replace the registered solver with an
unreviewed two-sheet spectral calculation. Re-run the existing interior controls
as regressions, labeled as such. No larger grid. Commit and push the specification,
implementation, tests and evidence before any larger calculation.

## The actual geometric object

For q a real 3 by 3 matrix, the colour action is q -> q R^T, R in SO(3).
The invariants are G=qq^T and d=det(q), with d^2=det(G). The pair (G,d), not
G alone, distinguishes the two orientations. Replacing d by |d| or quotienting
by O(3) changes the cubic potential and the Hilbert space.

At a rank-two matrix the SO(3) action is free: a rotation fixing two independent
row vectors is the identity. The rank-two quotient is locally regular, not a
physical hard wall. The signed determinant has a nonzero q-space gradient
there (the cofactor matrix), so its zero set is a smooth hypersurface locally.

Where the first two rows are independent, choose the positively oriented frame

    e0=q0/|q0|,
    a=q1.e0, r1=|q1-a e0|, e1=(q1-a e0)/r1, e2=e0 cross e1.

Set r0=|q0|, b=q2.e0, c=q2.e1, w=q2.e2. Then, modulo SO(3),

    L = [[r0,0,0],[a,r1,0],[b,c,w]],
    u = (log r0, log r1, w, a, b, c),  r0,r1>0, w real,
    d = r0 r1 w.

Unlike the earlier log-Cholesky chart, w is signed and may equal zero. This
chart covers both sides and their common rank-two interface. It is local:
other independent row pairs need overlapping charts. Rank <=1 requires the
inherited Cartesian regularity, not an invented scalar boundary rule.

## Exact measure and kinetic term

Off w=0 the Gram Jacobian is |det(dG/du)|=8 r0^4 r1^3 |w|.
With the existing rho=(det G)^(-1/2), the reduced density is

    mu(u)=8 r0^3 r1^2.

The apparent Gram singularity cancels. The factor 8 follows the previous
normalization; an overall orbit-volume constant has no effect on matching.

Construct the 9 by 9 tangent matrix D from the six columns vec(dL/du_i)
and the three colour-orbit columns vec(L E_k), with E_k the standard
antisymmetric generators. D is invertible at rank two in this patch. The
first six rows K of D^-1 give the gradients of u in q-space. Thus

    B = K K^T,
    Delta_red f = B_ij partial_i partial_j f + beta_i partial_i f,
    beta_i = sum_j partial_j B_ji + (3,2,0,0,0,0)_j B_ji.

This is derived from the original Euclidean Laplacian, not a new metric.
Compute the symbolic expressions once and validate them independently using
finite differences of the Cartesian coordinate map and direct Cartesian
Laplacians of smooth invariant test functions.

At w=0 all mixed B_wj with j != w vanish, and

    B_ww = 1 + c^2/r1^2 + (a c - b r1)^2/(r0^2 r1^2) > 0.

The potential remains the signed original V_sourced(L). In particular,

    V(w)-V(-w) = -2 g sum(omega) r0 r1 w.

The determinant reflection is not a symmetry at either nonzero control
coupling. An even/odd restriction cannot supply a closed half-space sector.

## Inherited matching, with the orientation fixed

The parent form core consists of smooth compactly supported Cartesian
singlets. Smooth invariant polynomial times Gaussian functions also give
regular test states of finite norm and energy. They cross the interface;
their traces do not generally vanish.

For a sufficiently regular state, let psi_plus(x,h)=psi(x,+h) and
psi_minus(x,h)=psi(x,-h), h>=0, with x the other five coordinates. Then

    psi_plus(x,0) = psi_minus(x,0),
    mu B_ww (partial_h psi_plus + partial_h psi_minus)(x,0) = 0.

Equivalently, signed-w conormal derivatives agree. The plus sign between
the two h-derivatives is essential: each h increases into its own half.
The outward conormals are their negatives, whose sum also vanishes.

The first condition is the trace condition in the local form domain H^1.
The second is the operator-domain flux condition, obtained by integration
by parts with no delta-function interaction at the interface. For weak
operator-domain states, interpret it as conormal trace matching, not an
assertion of classical differentiability of every vector in the form domain.

In Gram variables let h=w^2 and F_plus/minus(x,h)=psi(x,+/-sqrt(h)).
Then values match and lim_{h->0+} sqrt(h)(partial_h F_plus+partial_h F_minus)=0.
Do not demand that each Gram derivative remain bounded. A smooth q-space
state containing d is proportional to +/-sqrt(h) in these variables.

For psi=[A(G)+d C(G)] exp(-sum alpha_i G_ii/2), the shared trace is A times
the envelope and the signed-w derivative at the interface is r0 r1 C times
the envelope. These can be chosen independently. Hence neither Dirichlet,
Neumann nor a fixed scalar Robin condition on the positive side alone is
inherited for all Cartesian states. Eliminating the negative side would
require its energy-dependent boundary response, not a guessed constant.

The existing one-sided log-Hermite trial functions have zero boundary trace.
Their zero extensions are admissible interior tests, but their form closure
cannot approximate a state with nonzero interface trace on a compact regular
patch. This follows from continuity of the H^1 trace map. Passing weak-coupling
controls does not remove that missing part of the physical form domain.

## Implementation and tests, before writing the implementation

Add a small independent signed-interface module; do not edit the old solver,
its receipts, or the original model files. Its matching checker takes complex
values and half-coordinate gradients, returns both residuals and a local
verdict, rejects nonfinite/malformed input, and never advertises a global
spectral certificate. Pointwise matching samples are not a whole-function
domain verifier. Use a fixed absolute tolerance 1e-10 for the matching tests.

Add the following gates, including deliberately broken variants afterward:

1. Signed chart: proper-rotation invariance, opposite determinant signs with
   the same Gram matrix, smooth passage through w=0, and explicit refusal of
   an invalid first-row-pair patch. Finite chart inputs only.
2. Cartesian differential checks: tangent inverse/gradient metric against
   independent finite differences (1e-8); positive B at w=0; exact density
   and the displayed B_ww identity (1e-10).
3. Positive-chart agreement away from the interface: metric and density
   match the earlier log-Cholesky chart after the actual coordinate change
   (1e-10). No replacement of the kinetic energy by a flat Laplacian.
4. Operator agreement on both signs and at rank two: the reduced Laplacian
   and sourced Hamiltonian agree with independently differentiated q-space
   invariant polynomial-Gaussian probes (absolute/relative 1e-9).
5. Smooth crossing: both a nonzero trace and a nonzero derivative are
   accepted; complex states can carry nonzero probability flux across it.
   Equal flux alone must not excuse a discontinuous value.
6. Bad interface data: reject a jump, a cusp, the wrong half-normal sign,
   NaN/infinity, a non-interface point, and malformed gradient arrays.
7. No invented wall: smooth Cartesian probes refute mandatory zero value,
   mandatory zero derivative and a universal fixed Robin relation.
8. Signed cubic: preserve the exact nonzero V(w)-V(-w) at the SAME two
   controls. Neither det(q)->|det(q)| nor independent parity blocks pass.
9. Re-run the existing positive-cone control tests and saved-state replay,
   retaining the prior 0.003456 > 0.003 refinement discrepancy. These remain
   interior controls, not newly boundary-complete spectra.

Preserve RED logs, GREEN logs, mutations, exact requests/settings and source
hashes. A test killed by an intentional defect must fail an assertion rather
than crash for an unrelated reason. No large spectral run is part of this
derivation gate. A local matching PASS cannot clear HOLD_DOMAIN.

## Literature and limits

The coordinate and matching derivation above is for this repository's chosen
Cartesian realization. It is not a new family of self-adjoint extensions.

- [Balachandran, Vaidya and de Queiroz, 1412.7900](https://arxiv.org/pdf/1412.7900)
  supplies the external finite matrix model.
- [Acharyya and Balachandran, 1702.06430](https://arxiv.org/pdf/1702.06430)
  studies alternative reduced boundary extensions. Those extensions are
  explicitly excluded by the original local registration; no edge-state
  boundary parameter is imported here.
- [Dhont, Cassam-Chenai and Patras, 1511.01311](https://arxiv.org/pdf/1511.01311)
  distinguishes SO(3) and O(3) invariant structure. The signed determinant
  must remain in the SO(3) description.
- [Teschl, Mathematical Methods in Quantum Mechanics, section 2.3](https://www.mat.univie.ac.at/~gerald/ftp/book-schroe/schroe2.pdf)
  provides the form/operator-domain framework; it does not prove this
  model-specific coordinate calculation or spectral convergence.

The global reduced domain is defined by lifting back to the original
Cartesian form domain, including lower-rank strata. The local interface
calculation is not a classification of every reduced self-adjoint extension,
an atlas-completeness proof, or a continuum Yang-Mills result.
