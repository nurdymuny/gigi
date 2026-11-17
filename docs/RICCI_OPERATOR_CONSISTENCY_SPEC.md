# RF2D: locating the shape-regular operator discrepancy

Date: 2026-11-17. Initial status: NOT RUN. Diagnostic, not a new flow claim.

## Frozen experiment

Preserve RF2B/RF2C sources, native executable, all raw results and manifests.
Use the same n=1,2,3,5 shape-regular S3 meshes and 120-fold left quotient.
The actual correspondence test remains Berger (3,1,1), with the SAME exact
links, continuum velocity, full mass norm, time scale and BW05 criteria:
both final orders >=0.5, final relative error <0.05, numerical error estimates
below one tenth of spatial error. No projected residual may replace this test.

Focused tests only. A passing numerical candidate, not merely a diagnostic
test pass, is required before broad regressions. No commit or deployment.
The retained native assembly binary SHA256 is
`71fb30dbbb5fbfcd01e7a516f72849bef9303cbae8b046f7fbd2fe7824b196ad`.
RF2C's eight exact native replays may be reused after checking their hashes,
source manifest and verified receipt. Do not rerun unchanged expensive work.

## Hypotheses, before measurement

1. A double-cover branch/sign loss is contaminating link or face data.
2. The implemented action derivative differs from its own energy derivative.
3. The tangent mass/interpolation is the main source of the full-flow error.
4. The discrete action derivative has a full-norm consistency defect even
   though its value and its action on smooth variations converge.
5. Such a defect already occurs in the linear Whitney codifferential, without
   a non-Abelian commutator or a finite-angle sign transition.

Do not declare one cause merely because the other tests pass. Quantify an
exact residual decomposition, and use a separate planted control to isolate
mechanisms. Multiple mechanisms or an unresolved result remain possible.

## Double cover: three angles are not interchangeable

For a unit native quaternion q=(q0,qv), define alpha (pronounced alpha)
as atan2(norm(qv),q0), in [0,pi]. The lifted SO(3) rotation angle is 2 alpha.
Ad(q)=Ad(-q), but q and -q are distinct SU(2) elements. Keep their signed q0.

For the U(1) eigenphase alpha, the paper's chord bridge is

    K^2 = ||q-I||^2 = 2(1-q0), S_lift=(1+q0)/2,
    d^2=(1-q0)/2, S_lift+d^2=1.

Here K is pronounced kay and S_lift is ess lift. The SO(3)-rotation-related
overlap q0^2=cos(alpha)^2 is a DIFFERENT observable. It must not be silently
substituted for S_lift. Read theory/double_cover_v3.tex, especially its
properties lemma and the remark that the stated sameness function does not
itself factor through the projective quotient.

Record per level: signed link/face q0 extrema, alpha maxima, minimum
S_lift, negative-q0 counts and distances to q0=0 and q=-I. The hemisphere
boundary q0=0 and the SU(2) logarithm cut at q=-I are different boundaries.
Neither boundary alone proves a thermodynamic/dynamical phase transition.

Controls, tested against the native action and adjoint:
- q=I and q=-I have equal adjoints but different fundamental holonomies.
- A smooth one-link path through alpha=pi/2 and alpha=pi remains a smooth
  polynomial native action; force positive-q0 canonicalization to expose
  the artificial jump it can introduce.
- Vertex signs s_i=+/-1 induce q_ij -> s_i q_ij s_j. Face products,
  action, right-coordinate derivatives and mass are unchanged.
- Flipping an arbitrary individual edge is NOT generally such a gauge
  change; triangular face signs can change. The native action must detect it.
- Simultaneous base antipodes (p,q)->(-p,-q) preserve the exact link. This
  base-space fact must not be confused with replacing a stored link by -q.

No factor of two may be inserted into the mapping or clock to improve fit.

## Exact residual accounting

Let B be the frozen full tangent mass, a the native action gradient, and
v the independently differentiated continuum link velocity. Then

    native velocity = -B^{-1} a,
    r = a+Bv.

B is pronounced bee; r is arr. Construct an independent weak continuum
load L (ell) using the SAME local trial field that defines the native mass.
For edge e with target j, that field in the cell-root frame is

    T_e eta = -2 Ad(U_0j) eta * Whitney_1_form_e.

Eta is pronounced ay-tuh. At quadrature position x, transport the analytic
left-invariant continuum velocity W(x) to the root by Ad(U_0x), using the
same declared round short path. Integrate its metric inner product with
T_e eta. Assemble with the unchanged reversed-edge lifts and orbit weights.
This produces L without using the measured native force to construct it.

Split exactly:

    r_action = a+L, r_mass = Bv-L, r = r_action+r_mass.

Solve B times e_action = r_action and B times e_mass = r_mass, using the
frozen CG tolerances. Report their full volume-normalized mass norms and
their inner product. Check vector reconstruction and the norm identity;
do not add norms as if the two pieces were orthogonal. Bound the total
between abs(norm(e_action)-norm(e_mass)) and their sum.

Quadrature orders 4 and 6, all four levels. This changes no kernel or
acceptance threshold: L is an explanatory measurement, not a replacement
right-hand side. Validate its integration by planting a representable trial
field; then L must equal the independently assembled cell B times eta.

Smooth-variation checks may use the nine homogeneous tangents of the frozen
oracle. Label those checks weak/restricted; they never close BW05.

## Linear Abelian mechanism control

This is an additional diagnostic, NOT a replacement for the fixed Berger
mapping. On the same round S3 geometry, take a=theta_1, the first left
Maurer-Cartan one-form. Its exact edge integrals are the first components
of log(p^{-1}q); d theta_1=-2 theta_2 wedge theta_3 and
delta d theta_1=4 theta_1. Delta is pronounced del-tuh.

Assemble the ordinary scalar Whitney matrices M1,M2 with the same radial
geometry and quadrature. Let D be the oriented edge-to-face incidence:

    K=D^T M2 D, linear velocity=-M1^{-1} K a,
    exact velocity=-4 a.

Kay here denotes the linear stiffness matrix, not the preceding chord cost.
Use explicit names in code/results to avoid ambiguity. Check the native
SU(2) action's small-amplitude Hessian along a single internal axis against
this scalar matrix. That anchors the control to the actual action, rather
than to a look-alike unrelated discretization.

Measure full strong error and energy on all four levels, orders 4 and 6.
Measure its Rayleigh quotient, expected to approach 4, separately from the
full residual norm. Accurate energy/Rayleigh values do not establish an
accurate pointwise discrete generator on canonical interpolated data.

If a strong discrepancy persists as amplitude tends to zero, it cannot
require finite-angle phase crossing or noncommuting internal axes. This
does not exclude other nonlinear contributions in the original test.

## Strict focused gates

### HHE / Crouzeix diagnostic extension (before measurement)

User-requested companion: reuse the actual harmonic-conjugate kernel to
recover p(z)=(1-z)/2 from u(x,y)=(1-x)/2, with declared unit-disk bound
1+1e-10 and exact dyadic coefficients. Verify its derivative and conjugate.
This is a new polynomial for this operator, not a reuse of the 352-term
polynomial from a different matrix/domain.

For the coarse Abelian control, M=R^T R and A=M^{-1}K. Whiten first:
H=R A R^{-1}=R^{-T} K R^{-1}, a symmetric positive-semidefinite matrix.
With largest eigenvalue Lambda (pronounced lamb-duh), Z=2H/Lambda-I has
spectrum in [-1,1]. The HHE polynomial bounds the normalized relaxation
p(Z)=I-H/Lambda on this domain. Independently evaluate that matrix
polynomial, check symmetry and contraction, and compare with its unwhitened
Euclidean-norm appearance. Keep numerical eigenvalue residuals in evidence.

This checks metric-dependent amplification and a finite polynomial bound;
it does not change the flow step, repair the measured residual, establish a
nonlinear stability theorem, or certify an external conformal map/tail.
For this symmetric whitened operator the spectral theorem is sharper than
a general non-normal Crouzeix bound. Do not mistake apparent Euclidean
non-normality of M^{-1}K for non-normality in its physical mass norm.

| Gate | Required evidence |
|---|---|
| OC00 | Freeze prior data, sources, spec and binary; drift refuses |
| OC01 | Behavioral RED then GREEN for signed diagnostics, unit/input checks, residual identities and independent weak load |
| OC02 | Native center/gauge/phase path tests; forced sign erasure fails |
| OC03 | Native cell derivative and independent scalar Hessian, centered step refinement, mass/load planted field agreement |
| OC04 | Four-level Berger load/action/mass decomposition with independent quadrature and exact recombination |
| OC05 | Four-level infinitesimal Abelian control, both quadratures, correct scalar incidence and reverse orientation |
| OC06 | Wrong half-angle, absent transport, erased signs, wrong orientation, missing levels, fabricated evidence and stale PASS rejected |
| OC07 | Locate the tested mechanism or state explicitly what remains unresolved; no general convergence theorem |
| OC08 | Native HHE polynomial recovery, coarse mass-whitened matrix polynomial and disclosed scope |

No unconditional production fix is specified in advance. After localization,
the smallest candidate correction must receive a separate spec amendment,
behavioral failing tests and the unchanged full-flow gate. Do not relabel
a weak-consistency pass as a repaired full-flow result.

## Primary context

Christiansen and Halvorsen, [A simplicial gauge theory](https://arxiv.org/abs/1006.2059),
studies consistency of a gauge-invariant simplicial action. Arnold, Falk and
Winther, [Finite element exterior calculus: from Hodge theory to numerical stability](https://sites.math.rutgers.edu/~falk/papers/bulletin-orig.pdf),
distinguishes compatible variational formulations and operator domains.
These motivate separating norms and mechanisms; neither paper certifies
GIGI's implementation or replaces the frozen acceptance criteria.
