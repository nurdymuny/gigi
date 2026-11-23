# RF2F: compatible variational curvature recovery

Date: 2026-11-17. Initial status: NOT RUN. This is the authorized first repair
candidate after Russian Geom Work's review. RF2E's concurrently started
resolvent experiment is separate exploratory regularization, not this repair,
not a release gate, and not authorized chapter evidence. Preserve its files.

## Fixed experiment

Keep RF2B/C/D data, exact signed links, reference velocities, unit physical
clock, four n=1,2,3,5 shape-regular meshes, and original tangent mass unchanged.
BW05 remains: both finest measured-h rates >=0.5; final full relative error
<0.05; solve and q4/q6 differences below spatial error/10. No filtering, fitted
factor, reference-field basis, replaced input cochains or projected residual.
Use focused tests only; Abelian before Berger; report to Russian Geom Work
before the chapter may claim anything from this experiment.

## Representation and auxiliary variables

On each radial tetrahedron use degree-one polynomial two-forms in its affine
reference coordinates, transported by the differential-form/Piola map to S3.
The vector proxy is a three-component affine polynomial, 12 coefficients.
This is the tetrahedral BDM1 space, not a homogeneous Maurer-Cartan ansatz.
Its 12 degrees of freedom are three oriented flux moments per face:

    c_(f,j) = integral_face lambda_j F, j=0,1,2.

Lambda (lam-duh) is the face vertex barycentric coordinate. Sum of the three
moments equals the original integrated face flux f. Identify corresponding
moments on both sides of every face with their actual orientation and vertex
permutation. This gives globally matching normal traces, not merely matching
total fluxes. The map descends to the existing left deck quotient; verify each
deck transformation and refuse edge-signature collisions with different
geometric faces. Every quotient face must have two weighted incidences.

Let C sum the three moments of each face; J puts f/3 in each; Z maps two free
coordinates to (z0,z1,-z0-z1). Then C J=I, C Z=0. Every feasible field is

    c = J f + Z z,   f = D a,

with ORIGINAL edge integrals a and incidence D. Since BDM1 divergence is
constant on each cell, the original closed-cochain identity implies zero
cell divergence; matching traces makes this distributional closedness.

## Geometry-only polynomial recovery target

For each cell construct a gnomonic chart by inverting its four vertex columns
and dividing homogeneous barycentric coordinates by their sum. All adjoining
radial faces become flat triangles in that chart. Use the 11-dimensional
closed (divergence-free proxy) subspace of affine two-forms. Its basis is the
nullspace of the universal coefficient divergence row, independent of data,
metric parameters, theta_1, and the desired flow. Theta is pronounced thay-tuh.

Gather complete face-adjacency rings on the COVER, represented by pairs of
quotient-cell ID and exact deck element. Do not silently identify different
lifts of the same quotient cell. Try radii 1,2,3 in that order; choose the first
with adequate rank and conditioning. Refuse after radius 3; no degree/radius
search driven by the error. Reject any chart denominator <=1e-8.

Fit integrated fluxes, divided by chart face area for nondimensional row
scaling. Enforce the four CENTRAL face fluxes exactly (rank three on closed
two-forms). Minimize the remaining normalized flux residual. Use SVD with
relative rank tolerance 1e-11, central rank three, and free-fit rank eight.
Require both retained condition numbers <=1e6. Never regularize a deficient
fit. Record radius, conditioning and reconstruction amplification.

The result is a linear local map P_t from original face cochains to the 12
BDM1 moments of the cell. It reproduces every closed affine polynomial in a
flat chart when the data are its exact face integrals. The four exact moment
sums are retained. The patch target need not have matching higher moments
between cells; the following variational step enforces that compatibility.

## Variational elimination and actual reduced energy

Let H be the assembled BDM1 mass (aitch), using the SAME radial geometry and
quadrature, and let Q f be the assembled load of the piecewise target P_t f.
Select the compatible recovered field as the constrained closest field:

    minimize_c  1/2 sum_t ||c|_t - P_t f||_(H_t)^2, subject to C c=f.

With S=Z^T H Z, the unique auxiliary solution is

    z = S^-1 (Z^T Q - Z^T H J) f,
    R f = J f + Z z.

S (ess) is positive definite for an accepted mesh and positive metric; this
is a mass-matrix solve, not a frequency filter. No penalty parameter exists.
Report preconditioned solve diagnostics. The declared physical energy is

    E(a) = 1/2 (R D a)^T H (R D a).

It is NOT the fitting objective. Thus stationarity of the fitting objective
does not permit dropping derivatives of the recovered field. At fixed geometry

    gradient E = D^T R^T H R D a,
    R^T y = J^T y + (Q^T Z - J^T H Z) S^-1 Z^T y,
    velocity = -M1^-1 gradient E.

The original M1 (em one) is unchanged. Implement the displayed transpose,
including a second auxiliary solve. Do not retain the old codifferential.
Validate with independent constrained dense KKT elimination and finite
differences on a small planted fixture. Missing transpose and missing
auxiliary derivative must fail, not merely change a printed diagnostic.

Compatibility, polynomial reproduction and positive energy do not imply
strong consistency. The independent target remains convergence of the FULL
dual-mass action/load residual against all original discrete variations.

## Numerical and stability rules

CG uses the frozen rtol=1e-12, atol=1e-14, maxiter4000 and recomputed-residual
refusal. Validate the finite-dimensional reduced energy on tiny independent
fixtures, include direct sparse solve comparisons where feasible, and report
all solver work. Validate quadrature at orders 4 and 6 on every level.
On the smallest mesh independently replay auxiliary solves by sparse LU.
On all levels propagate the actual auxiliary, adjoint and final mass-solve
residual corrections through the displayed linear recovery and its transpose;
the correction-only solves use atol1e-25 (as in RF2C error estimation), with
unchanged rtol and iteration limit. Compare the resulting full velocity
correction with spatial error/10. This is an estimate, not interval arithmetic.
Patch SVD performs interpolation algebra only, never eigenmode selection.
On a fixed moment affine space the final variational step is an H-orthogonal
projection. Relative to the Whitney feasible lift it is nonexpansive; test
this property separately from local patch conditioning. Do not claim an
unproved mesh-independent recovery constant from a few condition numbers.

## Strict staged tests

1. Specification and behavioral RED: 12 moment basis unisolvence, exact
   closed-affine polynomial reproduction, constraint sums, explicit reduced
   adjoint, planted energy derivative, nullspace and orientation behavior.
2. GREEN and removal checks: no auxiliary solve, missing transpose/chain term,
   wrong face sign/permutation, wrong moment sum, singular fit and stale PASS.
3. On the SAME meshes verify global trace matching, closedness, deck lifts,
   mass agreement with the old assembly and all patch conditioning/refusals.
4. Independent controls: theta_1, theta_2, theta_3 on round S3, plus a nonconstant
   exact scalar gradient with zero curvature. None enters the recovery basis.
5. Four-level theta_1 FULL strong error, action/load/mass split, both measured-h
   orders, q4/q6 and solve budgets. Reuse BW05 verbatim. Any failure is NO-GO.
6. Only after that pass: a separate complete SU(2) construction and derivative
   spec, gauge/deck/sign/transport mutants, then the same Berger full gate.
   Differentiate every link-dependent transport, fit, constraint and mass.
7. Independent replay with source/data hashes; bad and incomplete evidence
   must invalidate old PASS before refusing. No commit, deployment or new
   chapter claim without the corresponding complete release gates.

## References and limits

[Arnold, Falk, Guzman and Tsogtgerel](https://sites.math.rutgers.edu/~falk/papers/dstar.pdf)
demonstrate that strong consistency of the Whitney codifferential can fail;
they do not prove our diagnosis or this proposed repair.
[BDM definition](https://defelement.org/elements/brezzi-douglas-marini.html)
specifies polynomial space, facet moments and normal continuity. This custom
radial-chart recovery requires its own tests. Finite numerical agreement is
not a proof of general lattice-to-continuum convergence.
