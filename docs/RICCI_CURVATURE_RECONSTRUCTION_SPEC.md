# RF2E: curvature reconstruction and its actual discrete adjoint

Date: 2026-11-17. Status at specification: NOT RUN. Experimental, not deployed.

## Scope and frozen contract

RF2D located a strong-generator defect even in the Abelian Whitney control.
It did not locate an error in native differentiation of the existing action.
This experiment therefore proposes a NEW discretized energy, not a bug fix
to its automatic differentiation. Preserve every RF2B/C/D artifact and source.

The exact Levi-Civita link map, signed SU(2) representation, base meshes,
quadrature orders 4 and 6, tangent mass, continuum comparator, clock factor 1,
and BW05 acceptance criteria stay unchanged. Use the same n=1,2,3,5 meshes.
Both finest measured-h convergence rates must be >=0.5; finest full relative
error must be <0.05. Quadrature and solve errors must be below spatial error/10.
No projection of the acceptance residual, spectral cutoff, target-dependent
parameter, continuum velocity used as reconstruction input, or changed clock.
Focused tests only until an actual numerical candidate passes. No release or
commit from an Abelian pass alone. No claim of a general convergence theorem.

Start with an independently testable Abelian reconstruction. Berger is unlocked
only if that reconstruction passes the four-level gate. A nonlinear extension
must have its own complete energy derivative; applying a linear filter to the
old Berger force is not an acceptable extension.

## Candidate A: bias-corrected resolvent curvature reconstruction

M (pronounced em) is the fixed Whitney one-form mass; N (en) is the two-form
mass; D (dee) is incidence. K (kay) = D^T N D, A (ay) = M^-1 K. These symbols
are matrices, not the double-cover chord cost. Define the curvature operator
H (aitch) = D M^-1 D^T N. It is N-self-adjoint and nonnegative. H D = D A.

For positive epsilon (ep-si-lon), define

    P_epsilon(X) = (I + epsilon X)^-1,
    R_epsilon(X) = 2 P_epsilon(X) - P_epsilon(X)^2.

At scalar t=epsilon lambda >=0, the reconstruction multiplier is

    r(t) = (1+2t)/(1+t)^2 = 1 - (t/(1+t))^2.

This is a two-term, bias-corrected resolvent, not an eigenvalue selection.
It is strictly positive at every finite nonnegative eigenvalue, at most one,
has zero first derivative at t=0, and tends to zero as t tends to infinity.
No modes are discarded. For fixed lambda its bias is O(epsilon^2 lambda^2).

Reconstruct curvature f=D a as f_hat=R_epsilon(H) f. The declared energy and
its FULL derivative at fixed geometry are

    E_epsilon(a) = 1/2 f_hat^T N f_hat,
    grad E_epsilon(a) = D^T R_epsilon(H)^T N R_epsilon(H) D a,
    velocity = -M^-1 grad E_epsilon(a).

Both reconstruction factors matter. Omitting the adjoint factor computes a
different force. By H D = D A and the weighted adjoint identities,

    E_epsilon(a) = 1/2 (R_epsilon(A)a)^T K (R_epsilon(A)a),
    grad E_epsilon(a) = K R_epsilon(A)^2 a.

The implementation may use this edge-space identity without constructing H
or inverting N. Tests must compare against explicit independent face-space
construction on planted dense complexes. Existing exact edge data a, not a
replacement projection of the known answer, are fed into this operator.

### Geometry-only scale, fixed before measuring this candidate

Let L be the declared reference length (unit round S3 radius in the Abelian
control), h the frozen maximum geodesic edge length. Set

    epsilon = L^2 (h/L)^(3/2).

No adjustable coefficient or optimizer is exposed. This scale tends to zero,
but epsilon/h^2 tends to infinity. If an O(1) generator defect is carried by
lambda=O(h^-2) modes, the squared reconstruction attenuates that contribution
as O(h); fixed physical modes have O(h^3) bias. This is a motivation under
explicit spectral assumptions, NOT a proof that every error has this form.
The exponent targets first-order removal of mesh-scale contamination without
first-order low-frequency bias. It is not selected by a measured candidate
error or by the 5% threshold. Record all failures; do not search exponents.

Under a uniform length rescaling s, h and L scale by s, eigenvalues by s^-2,
epsilon by s^2; the dimensionless multiplier is unchanged. Validate this on
planted matrices. Reject nonpositive or nonfinite h,L and h>L for this
specified asymptotic experiment instead of silently clipping.

### Solver contract

Compute P a by solving (M+epsilon K)x=M a with CG, the frozen rtol=1e-12,
atol=1e-14, maxiter=4000, positive diagonal preconditioner and a recomputed
residual. R requires two solves, R squared four. Solve M v=-grad E with the
same policy. Retain individual residuals and iterations. Independently
estimate velocity error by an independent sparse-LU replay of all five solves,
checking every direct-solve residual and measuring the complete velocity
difference in the full mass norm. This is a numerical error estimate, not an
interval enclosure; no CG success flag alone is sufficient. Dense spectral
evaluation is an oracle only on tiny test fixtures/coarse matrices, not the
production algorithm. Never shift a negative spectrum into acceptance.

## Required tests, behavioral RED before implementation

| Gate | Required evidence |
|---|---|
| CR00 | Hash and validate RF2C/D source/data/binaries, new spec and code; refuse drift; invalidate stale verification before any check |
| CR01 | Face-space reconstruction equals edge-space identity on a non-diagonal mass fixture; centered differences converge to the exact declared energy gradient |
| CR02 | One-sided reconstruction mutant, wrong transpose/metric, no reconstruction, and wrong scale fail planted tests; finite/nonpositive inputs and solver failures refuse |
| CR03 | Preserve incidence nullspace, orientation covariance, linearity, positive energy, weighted symmetry, exact low/high multiplier behavior, and physical length scaling |
| CR04 | Same four-level Abelian data, q4/q6, full mass error and both finest rates, no target used by solver; independent solve/quadrature error budget |
| CR05 | Only after CR04 passes: freeze a complete nonlinear Berger reconstruction and derivative, including derivatives of link-dependent transports, masses and reconstruction solves |
| CR06 | Native or independently verified nonlinear energy finite differences, signed double-cover and gauge tests; removed transport/chain-rule mechanisms must fail |
| CR07 | Berger same four-level full-flow gate, full residual accounting, q4/q6 numerical budgets; no filtered residual masquerading as full error |
| CR08 | Independent replay, counterfeit/missing cases/nonfinite data/stale PASS rejection, source receipts and commands; broaden regressions only after numerical success |

Passing CR01-03 proves implementation of the proposed linear energy, not its
usefulness. Passing CR04 proves only this Abelian control. Berger and general
correspondence remain NOT ESTABLISHED until their own gates pass. If Candidate
A fails, retain the failure and do not silently try a new parameter family.

## Nonlinear extension: design constraints, not an implemented formula

A nonlinear curvature reconstruction must use actual signed face holonomies
and gauge-covariant comparisons, not continuum curvature or the target force.
If R(U) or the curvature inner product depends on links U, the derivative of
1/2 <R(U)F(U), R(U)F(U)> includes derivatives of F, R and that inner product.
Freezing those terms is a different vector field. It must not be labeled the
gradient of this energy. The final tangent mass used to turn the derivative
into velocity remains the frozen B(U). The metric is held fixed during a
link variation. A complete extension will be specified BEFORE a Berger run.

Russian Geom Work has been asked to independently review the diagnosis and
suggest compatible local reconstruction or variational alternatives. Their
recommendation is advisory evidence, not a replacement for these tests.

## Mathematical context and limits

[Arnold, Falk and Winther, FEEC](https://sites.math.rutgers.edu/~falk/papers/bulletin-8-13-09.pdf)
and [Arnold and Guzman, local commuting projections](https://arxiv.org/abs/2104.00184)
explain why compatible projections and operator domains matter. Their results
do not prove this new resolvent experiment on GIGI's manifold meshes. The
finite-dimensional identities above follow from the displayed matrices and
will be checked independently; the asymptotic conjecture remains a test.

HHE/Crouzeix remains available for mass-aware operator-function checks. A
rational reconstruction is not itself the earlier HHE polynomial certificate.
Retain signed SU(2) data and all RF2D double-cover controls. Do not substitute
SO(3) overlap for signed fundamental holonomy or change the half-angle.
