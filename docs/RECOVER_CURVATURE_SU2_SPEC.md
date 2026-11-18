# RECOVER_CURVATURE for SU(2): the covariant two-ring recovery (spec)

2026-11-17, for Bee. **Spec only. Nothing here is built or run.** It extends
`docs/RECOVER_CURVATURE_SPEC.md` (the Abelian verb, shipped) and the paper
*Reading the Curve, Not Counting the Points* (`docs/papers/two-ring-pg/`).

## 1. Direction, and the ladder it sits on

The two-ring reading gives a consistent, stable discrete force for one Abelian
control. Bee's direction is to carry it into the non-Abelian theory. Read by its
shape, that direction runs up a ladder, and each rung is gated before the next is
built:

1. **Covariant recovery (this spec, step S1).** Read SU(2) curvature from link
   data the way the paper reads Abelian curvature from edge integrals.
2. **A consistent non-Abelian force (S2, outlined).** The Yang–Mills force
   `−D_A* F` is the velocity of the Yang–Mills gradient flow, which lattice gauge
   theory calls the Wilson flow and uses to smooth fields and set scales before
   measuring anything (Lüscher, *Properties and uses of the Wilson flow in
   lattice QCD*, JHEP 08 (2010) 071). On irregular meshes the flow is only as
   good as the force. S2 asks for the paper's BW05 consistency, non-Abelian.
3. **Reciprocity (S3, open).** Physical spectra need a symmetric
   (self-adjoint) operator. The two-ring force is not symmetric; that is the open
   problem the paper ends on. No spectral or mass claim is made before S3 passes.

The mass gap is the **motivation**, tagged RESONANCE. It is never a claim
produced by this ladder. The quantum side (correlators, a transfer matrix) is
separate work beyond S3.

## 2. What exists (verified 2026-11-17 on `feat/reading-the-curve`)

- **The Abelian verb.** `src/gauge/curvature_recovery.rs`: one real number per
  edge, transport onto the cell's sheet, a closed linear fit with 11 unknowns,
  own faces exact and the rest in least squares, and a Jacobi SVD. Parity with
  the paper is about 1e-14.
- **SU(2) links on these meshes.** `gauge::ricci_link::levi_civita_link(metric,
  a, b)` returns a unit quaternion per edge: the Levi-Civita spin transport of a
  diagonal left-invariant metric. Stored-link convention: "Base-space products
  are Hamilton/plus; stored matrix links are minus-cross" (source comment).
- **Native SU(2) Whitney assembly.** `examples/ricci_operator_probe.rs` builds
  the cell action, gradient and tangent mass from those links. This is the
  non-Abelian **Whitney** reading, the baseline S2 must beat.
- **The exact continuum.** `e2e/probes/ricci_wilson_oracle.py`:
  `connection_matrices(g)`, `field_curvature(a)` and `gradient_matrix(g, a)`
  give the curvature and the Yang–Mills gradient of the left-invariant
  connection for metric `g`.
- **Frozen meshes.** n = 1, 2, 3, 5, with round and Berger metric inputs, in
  the Ricci checkpoint.

## 3. The idea in one paragraph

For SU(2) there is no edge cochain to add up. The data are links `U_e`, and
what a face carries is its **holonomy** `U_f`, the product of the links around
it. Its logarithm `X_f = log U_f ∈ su(2)` is the non-Abelian face flux, to
leading order the integral of `F` over the face. Each `X_f` is expressed in the
frame of the vertex where its loop starts. Before fluxes from different faces can
be compared, each must be **turned into register** in the cell's own frame by
conjugating it with the transport `W` from its base vertex to the cell's
reference vertex: `X_f ↦ W X_f W⁻¹`. In the Abelian case this step was only the
deck transformation between sheets. Here it also carries the gauge frame, and it
is what the covariant Bianchi identity `D_A F = 0` asks for. After register, the
fit is the paper's fit, applied to each of the three su(2) components.

### Why the same 11-unknown fit applies

Work in Fock–Schwinger gauge at the cell centroid, where `A(0) = 0` and
`A_i(z) = −½ F_ij(0) z^j + O(z²)`. Then
`dF = −[A ∧ F] = O(z)`: the commutator term vanishes at the centroid and enters
only at the next order. So, to the order of a linear model, each su(2) component
of `F` is a **closed** linear field, `c^a + B^a z` with `tr B^a = 0`. The
non-Abelian content enters through register and through the face fluxes, not
through the model space.

### A consequence that makes S1 cheap

The flux rows depend on geometry only, so the fit map `R_K` (11 × faces) is the
**same** map the Abelian verb already builds. It applies to the 3 × faces array
of registered fluxes:

```
coefficients (3 × 11) = X_registered (3 × faces) · R_Kᵀ
```

Amplification and condition are the Abelian values unchanged. The cost over the
Abelian verb is the face holonomies, their logarithms and the conjugations.

## 4. Algorithm (S1)

Inputs: the closed tetrahedral mesh (the cell bundle, as in v1) and a link bundle:
numeric `edge` = 0..E−1 and four numeric fields `q0..q3` (unit quaternion,
scalar first), stored for the edge's stored orientation.

1. **Links per local edge.** Reverse the stored link when `r_e = 1` (inverse =
   quaternion conjugate). The convention is pinned by test G1 (section 5), never
   by assertion.
2. **Face holonomy** for face (i, j, k): `U_f = U_ij · U_jk · U_ki`, composed in
   the order `gauge/mod.rs` documents (left to right in cyclic face order).
   `X_f = log U_f` (`= θ n̂/2` form, `‖θ‖ < π`; refuse if a face holonomy is
   near −1, because the logarithm is ill-defined there).
3. **Neighbourhood** exactly as v1: the cells within `rings` face-steps, carried
   onto the cell's sheet by the deck transformation, with coincidence refusal.
4. **Register.** For each face read, compute `W`: the product of links along the
   deterministic shortest edge path (breadth-first over the neighbourhood's
   vertex graph, ties broken by vertex id) from the face's base vertex to the
   cell's vertex 0. Set `X_f ↦ W X_f W⁻¹`.
   - **Path dependence is itself curvature.** Report `register_spread`: the
     largest difference between the registered flux along the chosen path and
     along the next-shortest path, over all faces read. It is a diagnostic, not a
     refusal.
5. **Fit.** `coefficients = X_registered · R_Kᵀ` with v1's `R_K` (own faces
   exact, the rest least squares). The own-face residual check is per component.
6. **Return** per cell: `c` (3 × 3: component × direction), `b` (3 × 3 × 3),
   `coefficients` (3 × 11), `amplification`, `condition`, `faces`, `cells`,
   `residual`, `own_residual`, `register_spread`.

**Surfaces.** v1 stays backward-compatible; the field arity selects the mode:

```
RECOVER_CURVATURE cells FROM edges ON a RINGS 2;                 -- Abelian (v1)
RECOVER_CURVATURE cells FROM links ON (q0, q1, q2, q3) RINGS 2;  -- SU(2)
POST /v1/bundles/{cells}/recover_curvature
     {"edges": "links", "fields": ["q0","q1","q2","q3"], "rings": 2}
```

`field` and `fields` are mutually exclusive; sending both, or neither, is a 422.

## 5. Gates (RED before GREEN, each failing for its own reason)

| id | fixture | must hold |
| --- | --- | --- |
| G0 | **Abelian embedding.** The paper's cochain `a` turned into links `exp(a_e τ₃)` along one fixed axis, on all four meshes | the `τ₃` component of the coefficients equals the v1 verb's coefficients to 1e-12 relative; the other two components are ≤ 1e-12. This must reproduce the paper to the digit. |
| G1 | **Gauge covariance.** A random `g_v ∈ SU(2)` per vertex, `U_ab ↦ g_a U_ab g_b⁻¹` | recovered `F` transforms as `Ad_{g(vertex 0)}`, to 1e-10; `‖F‖` is unchanged. This pins the composition and storage conventions by measurement. |
| G2 | **Exact non-Abelian field.** Levi-Civita links from `levi_civita_link` for round `[1,1,1]` and Berger `[3,1,1]`, n = 1, 2, 3, 5 | recovered curvature against `ricci_wilson_oracle.field_curvature`: report the error and the observed orders. Target is second order (the paper's Abelian recovery reached 0.155% at n = 5). |
| G3 | **Refusals** | holonomy near −1; plus every v1 refusal, unchanged |

On G2, honestly: `log U_f` equals the face integral of `F` only up to
non-Abelian Stokes corrections. If those cost an order, G2 shows first-order
convergence. The remedy is then a midpoint (BCH) correction of the face flux,
specified as **S1b** and built only if G2 shows the order loss. The gate accepts
one outcome at a time: second order with S1, or second order with S1b.

## 6. Mechanism removals (each must turn a gate red)

| id | removal | must fail |
| --- | --- | --- |
| M1 | no register (fluxes used in their own base frames; deck transform kept) | G1 (not covariant) and G2 (accuracy) |
| M2 | abelianised flux (sum of edge logarithms instead of the log of the holonomy) | G2 on the Berger field (`[A, A] ≠ 0`); G0 still passes, which shows M2 is invisible to Abelian data |
| M3 | no equality constraint | the own-face residual, as in v1 |
| M4 | one ring | amplification jumps to the v1 one-ring values; G2's error under injected noise grows |

## 7. S2 outline (not specified for build)

Force `g(U)[η] = ⟨R(F), D_A-covariant Whitney curl of η⟩`, the Petrov–Galerkin
pairing in non-Abelian form, using the native tangent masses. The target is the
oracle's `gradient_matrix` mapped through the paper's velocity normalisation.
The gate is BW05, unchanged, run against the native SU(2) Whitney force on the
same meshes. Stability is the S3-style generalized spectrum of the linearised
operator. S2 gets its own spec once S1's G0–G2 are green.

## 8. Not claimed, not built

No code, no run. No non-Abelian force, no flow, no spectrum. Nothing about the
mass gap beyond its role as motivation. The Fock–Schwinger argument in section 3
is a leading-order statement; G2 is what tests it.
