# RF2C: mesh-only shape-regular refinement experiment

Date: 2026-11-17. Initial status: NOT RUN. Parent: RF2B / BW05.

## Frozen question and fast-fail order

Does replacing the deteriorating twelve-child mesh by a uniformly
shape-regular family fix the FULL link-flow discrepancy on the same S3
cover and binary-icosahedral left quotient?

Do not edit RF2B's mapping, native action, gradient, tangent mass, continuum
oracle, time convention, solve, error norm, fixtures, or numerical acceptance
thresholds. Keep all old evidence intact. The retained native assembly
executable is reused, not rebuilt with a changed discretization kernel:

    SHA256 71fb30dbbb5fbfcd01e7a516f72849bef9303cbae8b046f7fbd2fe7824b196ad

The user explicitly requested focused tests until a working result exists.
Therefore run geometry and contract gates first, then Berger `(3,1,1)`
through FOUR levels and quadrature orders 4 and 6. If Berger fails BW05,
stop this candidate and mark other metrics NOT RUN, not PASS. A failing
required fixture suffices for a no-go under the unchanged acceptance rule.
Only a Berger pass unlocks round `(1,1,1)`, triaxial `(0.5,1,2)`,
negative-scalar-curvature `(5,1,1)` and held-out `(1.3,0.8,2.1)`, followed
by the complete regressions. Do not choose a different sentinel after seeing
results. No public endpoint, commit, deployment or general flow theorem.

## The replacement mesh

Use the SAME 120 vertices and 600 facets from the frozen RF2B convex
600-cell. Make ONE affine barycentric subdivision of each original facet.
Its 24 tetrahedra are complete flags

    vertex < edge < triangular face < original tetrahedron.

Their four vertices are the affine barycenters, ordered by face dimension
0,1,2,3. This order is intrinsic and invariant under every deck transform.
No floating coordinate rounding or global vertex-ID order decides a
diagonal. Barycentric subdivision is NOT repeated during refinement.

On every fixed flag tetrahedron, use ordered edgewise subdivision, expressed
as the Freudenthal-Kuhn triangulation of

    O = {0 <= x3 <= x2 <= x1 <= 1}.

For integer n, intersect the 1/n cube grid with O. A retained tetrahedron
has vertices `i/n`, `(i+e_p1)/n`, `(i+e_p1+e_p2)/n`, `(i+1)/n`, for a
coordinate permutation p, provided all four vertices lie in O. Its
barycentric coordinates in the flag are

    lambda = (1-x1, x1-x2, x2-x3, x3).

There are exactly n^3 children per flag. Face restrictions agree with the
same ordered construction on that face. Dimension ordering of flags makes
those restrictions agree between all adjacent original facets and under
the deck group. Reference: Edelsbrunner and Grayson,
[Edgewise Subdivision of a Simplex](https://www.graysonfamily.org/dan/Papers/Esd-scan.pdf),
Discrete & Computational Geometry 24 (2000), 707-719.

Represent every new vertex by EXACT integer barycentric weights on the
original 120 vertices, with common denominator 12*n. For a flag ordered
by its four original vertices and integer barycentric numerators l0..l3:

    w = (12*l0+6*l1+4*l2+3*l3,
                 6*l1+4*l2+3*l3,
                      4*l2+3*l3, 3*l3).

Merge nodes by these sparse integer keys, omitting zero weights. Construct
affine points first and normalize once to the unit S3. Do not repeatedly
normalize intermediate subdivisions. Final tetrahedra get the same positive
R4 orientation convention as RF2B. The native curved-cell parameterization
remains `normalize(sum lambda_i u_i)` with unit vertices u_i.

Predeclared four resolutions are n = **1, 2, 3, 5**, giving **14,400;
115,200; 388,800; 1,800,000 tetrahedra**. These are four new meshes, not
the old level indices relabeled. Report actual maximum angular edge length;
compute observed orders using it, not assumed powers of two. The largest
resolution is chosen before flow evaluation, to reach a spacing comparable
to the old finest mesh without an unnecessary eightfold jump.

## Level-independent shape bound

This is a geometric argument, separate from the numerical flow test.
Let L be an original chord-edge length, c=(1+sqrt(5))/4 the adjacent-vertex
dot product, and rho the distance from zero to an original facet plane:

    L^2 = 2(1-c),     rho^2 = (1+3c)/4 = (7+3*sqrt(5))/16.

Successive edges of an affine flag orthoscheme are orthogonal, with squared
lengths L^2/4, L^2/12, L^2/24. All edgewise children are translates of one
of six fixed linear images of a unit Kuhn simplex, scaled by 1/n. Thus

    D_flat^2 = 3*L^2/(8*n^2),
    V_flat = L^3/(144*sqrt(2)*n^3),
    D_flat^3 / V_flat = 27*sqrt(3).

All affine points lie in the same supporting plane and have norm in
[rho,1]. Radial normalization on that facet is 1/rho-Lipschitz. For a
child with columns y_i and normalized columns u_i, determinant scaling gives

    |det[u0,u1,u2,u3]| = 6*rho*V_flat / product_i |y_i|
                      >= 6*rho*V_flat.

The distance from zero to the affine span of the u_i is at most one,
so their chord-tetrahedron volume is at least rho*V_flat. Its four faces
have total area at most 2*D_chord^2. With inradius r=3V/area:

    D_chord/r <= 2*D_flat^3/(3*rho^4*V_flat)
               = 18*sqrt(3)/rho^4 < 43.

This bound is independent of n. It is conservative, not a fitted observed
maximum. The same determinant and edge bounds give supporting-plane
distance `d_child >= 2*rho^4/(9*sqrt(3)) > 0`. The native radial chart is
therefore uniformly nondegenerate; its Jacobian singular values scale as
1/n with level-independent constants. Each fixed positive metric merely
multiplies these bounds by its fixed metric eigenvalue factors.

Independently verify the orthogonal Gram entries, child determinants and
volumes, integer facet restrictions, radial determinant relation, and the
bound in tests. A floating shape table alone is not this argument.
Operational geometry gate: measured maximum D/r <= 14 on each of the four
meshes, in addition to the uniform mathematical bound <43. The cap is fixed
before flow measurements and is substantially below RF2B's finest 19.67.

## Unchanged operator and acceptance

Reuse the frozen native binary and `reference_velocity`, `solve`, and
`measures` from RF2B. New geometry is passed through the identical wire
input. Any source/binary change in these frozen paths must refuse before
running. A general mesh-to-orbit adapter must reproduce the OLD adapter's
complete output exactly on the old level-zero and level-one fixtures.
This isolates geometry from orbit bookkeeping.

Use all three tangent components of EVERY quotient edge, and lift via the
unchanged `eta_reverse=-Ad(U) eta` rule. Verify reduced assembly against a
full-cover assembly on the new coarsest mesh. Do not remove transverse
components or infer a full-flow pass from energy or a few homogeneous modes.

BW05 is unchanged: four finite errors; BOTH finest refinement intervals
have observed order >=0.5; final relative error <0.05; quadrature/solve error
estimates at least ten times smaller than spatial error. Use actual h.
CG retains rtol=1e-12, atol=1e-14 and its recomputed-residual refusal.
No fitted time factor, normalization, budget or tolerance.

## Focused strict TDD gates

| Gate | Acceptance |
|---|---|
| SR00 | Freeze spec, baseline evidence, native binary and mathematical source hashes before construction; edits refuse |
| SR01 | Behavioral RED then GREEN: reference n^3 subdivision, exact volumes, conforming facet restriction, six fixed shapes; removed child/diagonal fails |
| SR02 | All four closed oriented covers, vertex links, exact integer node identity, free 120-fold deck orbits, measured h and shape; uniform-bound argument checked independently |
| SR03 | Old adapter byte/value agreement; new native cover versus quotient assembly, reversed-edge and metric-scale checks; independent cell oracle unchanged |
| SR04 | Berger: eight pinned native results, independent continuum oracle, unchanged four-level FULL-flow criterion; no successful-looking substitute |
| SR05 | Only after SR04 passes: same complete five-metric test matrix; otherwise explicitly NOT RUN |
| SR06 | Mechanism removals and adversarial verifier: source drift, lost cells, wrong orbits, omitted levels, fake zeros, changed thresholds or stale PASS must not survive |
| SR07 | Full default and Halcyon regressions deferred at user request until a numerically passing candidate exists |

Store raw requests, responses, hashes, red/green logs and measured work.
Use a separate output directory; never overwrite RF2B. Native jobs may run
concurrently with bounded workers, since they read immutable input and write
distinct files. Exact identical input/results may be reused by content hash;
never skip a changed mesh, metric, order, binary or mathematical contract.

A verified no-go is a valid experimental outcome, not a completed
correspondence. A numerical pass establishes only the declared tested
criteria, not an all-level analytical gradient-flow convergence theorem.
