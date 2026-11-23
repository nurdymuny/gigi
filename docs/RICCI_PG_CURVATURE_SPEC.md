# S1–S3: Petrov-Galerkin curvature test, Abelian control

2026-11-17. Spec before RED tests and implementation. Route chosen by
Bee ("go with your intuition") after `RICCI_INDEPENDENT_ASSESSMENT_2026_11_17.md`.
Static only: one instantaneous force per mesh, Abelian control, no time
stepping, no Berger, no native change, no full suite, no commit.

## Frozen contract

Same four meshes, levels 0,1,2,3 = n 1,2,3,5 (120 n-cubed cells, verified),
q4 and q6. Original signed edge data `abelian_edge_integrals`. Original round
metric [1,1,1], mass, clock, and the BW05 error exactly as in
`ricci_consistency_diagnostic.scalar_control`: velocity is mass-inverse times
minus the force, compared with −4a in the mass norm divided by volume. Mass and
stiffness come from that module's `assemble`, called read-only. Tolerances
unchanged: absolute 1e-9, rank cutoff relative 1e-12, condition cap 1e12.

## Measured preconditions (scratch probes, to become RED tests)

- All cell weights are 1.0. Stored signed data equal `base_log` of each cell's
  own vertices to 2.7e-16. Every face is shared by exactly two cells.
- Neighbouring cells' vertices are related either identically (1728 face pairs
  at level 1) or by a **left** quaternion multiplication (192), residual
  3.3e-16; right multiplication does not fit (residual ≥ 0.054). Left
  multiplication preserves `base_log`, so the data are consistent across sheets.
- Without transporting neighbours onto one sheet, the fit's condition number
  reaches 2.4e14. With transport it is at most 134 at every level.

## Reconstruction R

For each cell K, independently:

1. **Neighbourhood**: K and its four face-neighbours, matched by shared triples
   of global edge ids. Each neighbour's vertices are carried onto K's sheet by
   h (aitch) = p times conjugate q, where p and q are one shared vertex as
   stored in K and in the neighbour. After transport, all three shared
   vertices must coincide with K's copies to 1e-12, else refuse.
2. **Chart**: central projection onto the affine 3-plane H through K's four
   vertices, in an orthonormal frame of H, centred at K's projected centroid,
   divided by K's largest edge length. Projection must have positive
   denominator for every neighbourhood vertex, else refuse. K's own points lie
   on H, so for them the chart is affine in K's reference coordinates.
3. **Unknown**: a closed linear two-form proxy P(x) = c + B x with trace B = 0,
   eleven coefficients, in that frame. Closed means divergence-free proxy.
4. **Observations**: for every distinct face of the neighbourhood, its flux,
   the signed sum of the original stored edge data around its boundary, local
   order i to j to k. Predicted flux of P through the straight chart triangle
   is P at the centroid dotted with half the cross product of its edges, which
   is exact for linear P.
5. **Fit**: K's own four face fluxes are equality constraints; all other
   neighbourhood faces are fitted in unweighted least squares. Solved by the
   null-space method. Refuse if the full flux map has rank below 11 or condition
   above 1e12.
6. **Back to reference coordinates**: with J (jay) the constant Jacobian from
   K's reference coordinates to the frame, P_ref = det(J) times J-inverse times P.

**Amendment, 2026-11-17, after GREEN and the first mutation run.** Step 5 is
wrong as written. Every face-star at every level has exactly 5 cells and 16
faces; each cell contributes one closure relation, so there are 16 − 5 = 11
independent fluxes, equal to the 11 unknowns. The system is exactly determined,
not over-determined, and the fit reproduces all 16 fluxes to 1.3e-16. The
equality constraints therefore did nothing: replacing them with plain least
squares left every test green (recorded in `MUTATIONS.json`). R is instead
defined as **the unique closed linear curvature whose flux through every face of
the star equals the data**, solved directly, and it must refuse if the residual
on any star face exceeds 1e-12 relative to the largest flux. That refusal also
catches data violating closure on any cell. Test T9 covers it.

R is linear in the edge data, maps pure gauge data to zero (all fluxes vanish),
and reproduces all sixteen star fluxes exactly, including K's own four. It is **not** conforming between
cells; Petrov-Galerkin does not require it to be.

## Exact curvature, for S1 only

With the Maurer–Cartan frame used by `cell_integrals`, the data are integrals of
theta-zero, so the curvature is d theta-zero, minus twice theta-one wedge
theta-two. Its proxy in reference coordinates is minus twice the cross product
of rows one and two of the frame matrix. **This convention is not trusted until
a test shows its flux through faces equals the Stokes sums of the stored data.**

## Measurements

**S1, accuracy.** Relative error of P_ref against the exact curvature in the
metric two-form norm, the same weighting as `face_mass`, at every q6 point of
every cell; observed order m between levels. Stop if the finest m is below 1.5.

**S2, force.** In each cell, test P_ref against the four Whitney face functions
with the `face_mass` weighting, map faces to edges by INCIDENCE with the edge
signs and weight exactly as `assemble` maps stiffness, and sum into the global
force. Velocity, error and relative error exactly as `scalar_control`. Report
all four levels at q4 and q6, observed rates, and the q4/q6 difference.

**S3, stability.** Assemble the Petrov-Galerkin operator explicitly. At levels
0, 1 and 2, compute every generalized eigenvalue against the mass matrix.
Eigenvalues of magnitude below 1 must number exactly the gauge dimension found
for the original Whitney stiffness at that level; every other eigenvalue must
have strictly positive real part. Report the smallest real part and the largest
imaginary part. Level 3 is not decomposed; that is stated, not hidden.

## Tests and removals

RED before implementation, GREEN after:

- T1: a planted closed linear field's fluxes on a real level-0 neighbourhood
  are recovered to 1e-12.
- T2: K's own four face fluxes are preserved to 1e-12 on real data.
- T3: linearity in the data, to 1e-12.
- T4: pure gauge data reconstruct to zero, to 1e-12.
- T5: the exact-curvature proxy's flux through every level-0 face equals the
  Stokes sum of the stored data, to 1e-9.
- T6: with the trial replaced by Whitney, the assembled force equals
  `stiffness @ a` to 1e-12, so the S2 pipeline measures the BW05 quantity.
- T7: a neighbourhood without its face-neighbours refuses on rank.
- T8: a neighbour left on its original sheet is detected and refused.

Mechanism removals on the real run, each required to change the outcome:

- M1: trial set to Whitney reproduces RF2D's published errors at all four
  levels (26.1324% at n=5, q6). If not, S2 is not measuring BW05.
- M2: S1 with the Whitney trial must show order about 1, distinguishing it
  from the reconstruction's order.
- M3: deck transport disabled must refuse at every level, because shared
  vertices no longer coincide. Conditioning alone is NOT an adequate detector:
  without transport the scratch probe found condition numbers only up to 3.5e10
  at levels 0 and 1, under the 1e12 cap, so a conditioning-based refusal would
  pass silently on wrong geometry. The refusal is the coincidence check in
  step 1, applied after transport (identity when transport is off).

## S4 addendum: the variational energy with the same reconstruction

2026-11-17, written after S3 failed (Petrov-Galerkin has negative-real-part
eigenvalues on rough modes at levels 0 and 1) and before any S4 code.

Same R, same data, same mass, metric and BW05 measurement. Only the force changes:

    E(a) = one half, sum over cells K, of weight_K times the integral over K of
           P_K(a)-transpose g P_K(a) over root det g,

the same two-form weighting as `face_mass`, with P_K the reconstructed
curvature of section R. The force is its gradient, the sum over K of weight
times T_K-transpose W_K T_K a, where T_K maps the star's edge data to the
eleven coefficients and W_K is the eleven-by-eleven Gram matrix of the
reconstructed basis in that weighting. This is symmetric and positive
semidefinite by construction, so its spectrum against the mass is real and
non-negative; S3 is still run, not assumed.

Tests, RED before implementation:

- T10: the level-0 variational operator is symmetric to 1e-12 relative; its
  eigenvalues against the mass below 1 number exactly the gauge dimension (21),
  and every other eigenvalue is positive.
- T11: the force equals the central finite difference of E along random
  directions, with E computed by an **independent** path (fit, proxy and
  quadrature directly, not the assembled operator), to 1e-8 relative.
- T12: with the Whitney trial, the variational force equals `stiffness @ a`.
- T13: pure gauge data give zero force.

Each must fail under a mutation removing its mechanism (symmetric assembly,
transpose map, metric weight, gauge annihilation). Measurements: BW05 static
error at q4 and q6 on all four levels with rates, and S3 at levels 0 and 1.

## Acceptance and what it means

BW05 for this Abelian control: both finest observed rates at least 0.5, finest
relative error below 5%, q4/q6 difference under one tenth of the error, stable
spectrum at levels 0 to 2. Passing would show that Petrov-Galerkin with this
reconstruction beats the RF2D floor **for the Abelian control**. It would not
establish a Berger result, a variational method, or Ricci correspondence.
Failing S1 or S3 stops the route. Each worker is capped at 180 seconds;
evidence in `output/ricci-pg-curvature-20261117/`, which refuses to overwrite.
