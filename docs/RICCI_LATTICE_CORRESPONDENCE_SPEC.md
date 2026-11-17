# RF2B: Davis-Wilson lattice / Ricci correspondence

Date: 2026-11-17. Status: specification first; BW00-BW12 are NOT RUN.
RF2A verified a homogeneous continuum identity, not a lattice limit. This
gate remains OPEN until the operator, refinement and evidence gates below
pass. Partial passes must never be summarized as lattice correspondence.

## 1. Preserve the question

Use S3 = SU(2) and Gamma\SU(2), where Gamma is the binary icosahedral
group acting by LEFT translation. Metrics are the RF2A diagonal
left-invariant metrics `(1,1,1)`, `(3,1,1)`, `(0.5,1,2)`, `(5,1,1)`,
their declared positive scale multiples, and held-out positive metrics.
The structure constants remain `[e_i,e_j]=2 epsilon_ijk e_k`.

Test the *same connection map* and fixed time convention from RF2A.
Do not replace the question by a correlation of actions, a normalized curve,
a round critical point, a periodic torus, or a projection onto three modes.
There is no theorem of general Ricci-flow equivalence in this specification.

The scientific target is numerical consistency and convergence for this
declared smooth family before singularity, not a proof of a general limit.
An analytical truncation/stability argument is a separate required artifact
before claiming a convergence theorem rather than measured convergence.

## 2. Actual existing implementation

The native face walker uses the matrix representation

    M(q) = q0 I + i (q1 sigma_x + q2 sigma_y + q3 sigma_z),

whose quaternion product has MINUS cross product. It multiplies face links
left-to-right. The legacy `wilson_force_per_edge` and `drift_step` use PLUS
cross product. This apparent mismatch MUST be tested, not declared a bug:
left multiplication in the plus algebra is right multiplication in the
minus algebra. With right-trivialized variation

    U_e(s) = U_e compose exp(s eta),
    V(U) = (beta/4) sum_faces (1 - q0_face),

the proposed existing-force contract is

    dV/ds at zero = -2 sum_e dot(F_e, eta_e).

The factor two is fixed by the existing algebra metric, not fitted. Check
it against actual native holonomy and actual native drift on noncommuting
links with both edge orientations. If it fails, retain the exact mismatch
and halt integration claims. `SYMPLECTIC_FLOW` remains Hamiltonian, not
the new gradient flow. Its bit-identity path is not silently modified.

The existing `Lattice` contains a two-skeleton and an uninterpreted topology
label. Its `V-E+F` is not the Euler characteristic of a 3-manifold.
`LatticeWithMetric` currently describes surface areas, lengths and positions
in R3; it is not a 3D Whitney mass operator. Neither is sufficient alone.

## 3. Manifold mesh and refinement

Represent the cover as the boundary of the convex regular 600-cell in R4.
Its unit quaternion vertices are the 120 exact binary-icosahedral elements.
Initial counts must be `(V,E,F,T)=(120,720,1200,600)`; retain tetrahedra,
coordinates, orientation, provenance and the two-skeleton separately.
Radially project each flat boundary tetrahedron to S3:

    X_T(lambda) = (sum_a lambda_a p_a)/|sum_a lambda_a p_a|.

This construction, not a topology string or an Euler count, identifies S3.
Check nondegeneracy, face incidence two, coherent oriented boundary,
connectedness, vertex-link sphere tests, and `V-E+F-T=0`.

Refinement must be equivariant under the deck group. Use normalized edge
midpoints, four corner tetrahedra, and the normalized tetrahedron centroid
coned to the eight faces of the central octahedron: twelve children per
parent. No ID-dependent choice of an octahedron diagonal. Recheck orientation,
shape quality, maximum diameter and manifold incidence at every level.
Record actual h, not a claimed `2^-level` spacing. Minimum baseline levels
are 0,1,2,3; a resource limit leaves refinement gates open, never PASS.

Apply both exact deck generators to every vertex/edge/face/tetrahedron and
verify the induced permutations preserve the refined complex. Check free
orbits and retain the exact group certificate. Quotient evaluation may use
equivariant cover assembly divided by 120, but must bind all cell and edge
orbits and retain seam identifications. Do not pretend a one-vertex quotient
with parallel edges is representable by the current simple edge resolver.

## 4. Connection-to-link map, fixed before data

Base-space quaternions use the PLUS Hamilton product, hence bracket factor
two. For a directed mesh edge from p to q, take the unique short logarithm
`xi = log_plus(p^-1 q)` and the fixed path `p exp_plus(t xi)`.
This is a round great-circle path, not a claim that it is a geodesic of
every anisotropic metric.

For `x_i = (g_j+g_k-g_i)/sqrt(g_j*g_k)` from RF2A, define `a=x*xi`
componentwise. The inverse parallel transporter stored for native
left-to-right path composition is exactly

    U_pq = exp_matrix(sum_i a_i (-i sigma_i/2))
         = (cos(|a|/2), -sinc(|a|/2) a/2).

This is exact along the declared path because the left-invariant connection
is constant on it. Edge reversal must give the group inverse. Reject
nonfinite/nonunit endpoints, antipodal ambiguity and unsupported metrics;
never silently normalize arbitrary input or change the path. Identity
endpoints give identity links.

The adjoint map of `-i sigma_i/2` is `J_i v = unit_i cross v`, matching RF2A.
Small oriented triangle holonomy must approach the independently calculated
curvature, with its SIGN and factor of two checked against matrix ODEs.
Merely checking the trace cannot distinguish a reversed curvature vector.

## 5. Metric-aware action: an explicit extension, not a renamed legacy force

An unweighted sum of face traces on an arbitrary tetrahedral mesh is NOT
automatically the Yang-Mills action for g. In particular `Phi(c g)=Phi(g)`
gives identical links and identical legacy forces, whereas continuum
velocity scales by `1/c`. A missing metric cannot be repaired by a fitted
clock or by matching only one metric. Test this obstruction explicitly.

Use the pullback metric `G_T(lambda)=L_T^T diag(g) L_T`, where L_T is the
left Maurer-Cartan coframe of X_T. Integrate the full Whitney matrices:

    (M1_T)_ef = integral_T <w_e,w_f>_g dV_g,
    (M2_T)_ff' = integral_T <w_f,w_f'>_g dV_g.

Here `w_ij=lambda_i d lambda_j-lambda_j d lambda_i` and
`w_ijk=2(lambda_i d lambda_j wedge d lambda_k - ... + ...)` with fully
specified canonical orientations. Quadrature order is refined separately.
Check positive definiteness, tensor scaling and volume convergence.
Full off-diagonal entries are retained; a diagonal lump is not assumed.
The initial native cell kernel uses tensor Gauss-Legendre quadrature via
the Duffy map, orders 2 through 6, without fitted volume normalization.
Independent NumPy quadrature must check every reported matrix entry.

At each tetrahedron choose a declared root vertex and transport every face
holonomy to that root using stored edge links. Let `D_f` be the quaternion
of the transported `H_f-I`. Define the covariant simplicial extension

    E_h(U;g) = 2 sum_T sum_ff' (M2_T)_ff' dot_R4(D_f,D_f').

The normalization follows the physical SO(3) algebra norm from RF2A. For
diagonal face mass it reduces to `4 sum_f weight_f (1-q0_f)`, i.e. the
legacy potential with beta=16 and those weights. Off-diagonal couplings
are a genuine extension, with additional derivatives through transports;
the legacy staple alone is not its gradient.

This design follows the metric-coupled Wilson-loop construction of
Christiansen and Halvorsen, [A simplicial gauge theory](https://arxiv.org/abs/1006.2059).
Their consistency result motivates this choice; it does not certify our
implementation, curved mesh, error norms or time integrator.

## 6. Discrete gradient and comparison

Differentiate the actual E_h with respect to ALL right-trivialized link
coordinates, including transports between origins. Validate with centered
directional differences over several epsilon values and an independent
matrix implementation. No calls to RF2A's continuum velocity may occur
inside the discrete action/gradient evaluator.

Construct a gauge-covariant reconstruction of edge tangents using Whitney
one-forms, transporting each edge's target-based tangent to the cell root.
Its physical coefficient is `-2 eta_e`; the edge metric therefore includes
the factor four. Let its assembled positive mass be B_h(U,g). Then solve

    B_h W_h = -dE_h.

Report solver residual, conditioning diagnostics and all refusals. The
continuum comparison is to the derivative of the ACTUAL link map,
`J_h(g)[W(g,Phi(g))]`, not to a fitted three-parameter average. Compare

    r_h = W_h - J_h W,
    error_h = sqrt(r_h^T B_h r_h / Volume_g).

Also report relative error for noncritical cases, max local error, covariance
under vertex gauges, and a weak derivative norm. A decreasing weak error
does not stand in for the full mass-norm gate. If full-norm interpolation
consistency fails, retain that finding and specify a projection/stability
repair before changing the gate. Do not remove transverse link components.

## 7. Time and quotient gates

Only after spatial operator convergence, integrate the native lattice
gradient with the RF metric supplied at time t, exactly as RF2A H2 requires.
Independently refine spatial level, time step, quadrature and linear-solve
tolerance. Use the same initial metrics and `[0,0.02]` window; keep the
fixed-background experiment as a negative control. No time-axis alignment.

For the quotient, check equivariance of the link map and force, orbit-aware
assembly and the cancellation of the factor 1/120 in energy and mass.
Deck orders 120 vs 1 remain a separate exact/native certificate, not
Levi-Civita holonomy and not a mesh label inferred from local curvature.

## 8. Acceptance gates

Each gate requires behavioral RED before its mechanism, then GREEN and an
actual mechanism-removal failure. Compilation errors are not mathematical
RED. Retain requests, inputs, raw results, commands and hashes.

| Gate | Required evidence |
|---|---|
| BW00 | Native action/legacy-force/right-drift derivative agreement on planted noncommuting links; wrong left/right convention fails |
| BW01 | Exact group-derived 600-cell, closed oriented tetrahedral cover, equivariant refinements, vertex links and measured h |
| BW02 | Native link adapter, inverse edges, independent matrix transports, vector curvature convergence, sign/half-factor mutants |
| BW03 | Full M1/M2, SPD, scaling and quadrature/volume refinement; detect omitted anisotropy/off-diagonal entries |
| BW04 | Actual covariant action and full link gradient versus independent oracle; local gauge invariance/covariance |
| BW05 | All-link mass-norm operator convergence on four predeclared levels and held-out metrics; no fitted constants; strong and weak errors separate |
| BW06 | Quotient permutations/seams, orbit-aware operators and exact/native 120-vs-1 certificate |
| BW07 | Moving-background space/time refinement against RF2A; fixed-background negative controls retained |
| BW08 | Native executable/server data path with row/value verification; missing metric refuses, never silently substitutes legacy weights |
| BW09 | Independent noncircular oracle; normalized and unnormalized cases; zero fields/round fields cannot be sole controls |
| BW10 | Removed orientation, half factor, mass weights, cross-face coupling, origin transport or solver check causes named failure |
| BW11 | Fail-closed verifier rejects false convergence, deleted levels, fitted clocks, missing cells, incomplete solve and stale PASS |
| BW12 | Complete default and halcyon regression, source/binary pinning and reproducible high-fidelity showcase |

Predeclared operator criterion: on each noncritical fixture, all four levels
must produce finite residuals; the two finest refinement intervals must both
reduce full mass-norm error with observed order at least 0.5, final relative
error below 0.05, and quadrature/time/solve errors at least ten times smaller
than spatial error. Report all values, including coarse nonasymptotic levels.
Round controls use an absolute criterion; they cannot satisfy BW05 alone.
These numerical thresholds are acceptance tests, not a rigorous error bound.

## 9. Engineering order and refusal boundary

First pin BW00 and implement the exact native link adapter and genuine mesh
fixtures (BW01/BW02). Next implement the covariant mass/action extension,
then full force, then convergence and time evolution. Keep old Hamiltonian
byte-identity behavior unchanged. The experimental extension must have its
own versioned contract before any public query route or deployment.

At finite mesh size, arbitrary choices of cell/face roots must not silently
break the deck action. Before quotient assembly, use the explicit group
average of the covariant action and tangent metric over the 120 deck
permutations (or an independently verified algebraically identical local
orbit reduction). Differentiate that SAME averaged action. A free choice
of vertex IDs is not evidence of exact quotient equivariance. This averaging
is fixed by the declared group, not fitted to a continuum result.

### Exact orbit reduction of the averaged operator

For invariant initial links, let P lift independent quotient edge tangents
to ALL cover edges. A deck map preserving an edge orientation copies its
three tangent components. A reversed edge uses `eta_rev=-Ad(U_e) eta_e`,
derived by differentiating `U_e^-1`; it is not merely a sign change.
For the unaveraged cover action gradient b and mass B, assemble

    B_q = P^T B P / 120,   b_q = P^T b / 120.

Because each deck tangent permutation R satisfies `R P=P`, these are also
the restriction of the explicitly group-averaged action and mass. Positivity
and equivariance imply that the full averaged solution is invariant; solving
`B_q w=-b_q` therefore determines its full cover solution `P w`. This keeps
three components for EVERY quotient edge, not only three homogeneous modes.
Report the number of cover and quotient degrees of freedom.

For efficiency, group tetrahedra into exact deck orbits. Retain the histogram
of local vertex orderings obtained from ALL 120 actual cell permutations;
evaluate each distinct ordering once, weighted by its count/120. The weights
sum to one per orbit. Geometry and links are left-translation invariant.
The reduced assembly must agree entry-by-entry with an unreduced cover
assembly on level zero. Also lift the solved velocity and check the FULL
averaged cover residual directly, including transverse directions. Omitting
an orbit member, a permutation weight or the reversed-edge adjoint must fail.

Continuum comparison uses an independent matrix exponential Frechet
derivative of the fixed link formula, with RF2A's velocity supplied only
to that oracle. The native cell action and tangent mass receive no continuum
velocity. Quadrature refinement must be measured independently of spatial
refinement. A diagnostic that fails these criteria is retained as a failure,
not converted to a correspondence claim.

No gate receipt may say `correspondence: PASS` after only holonomy, action,
projected-force or volume tests. If a required layer is missing, return
`NOT_RUN`/`BLOCKED`, identify it by name, and do not offer successful-looking
curve plots as a substitute. No commit or deployment before BW00-BW12 close.

The first held-out metric is `(1.3,0.8,2.1)`, fixed before its operator
measurement. The diagnostic sparse solver uses relative tolerance `1e-12`
and absolute tolerance `1e-14`; recomputed residuals must be at most three
times `max(1e-12*|rhs|,1e-14)`. Both residuals and the right-hand-side norm
are reported, so a near-zero round control is not judged by an unstable
relative residual alone. This external SciPy solver is a diagnostic, not a
shipped native time integrator.
