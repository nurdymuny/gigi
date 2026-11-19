# Toward a variational two-ring force: design note (for Bee's review, before any code)

2026-11-19. Prompted by Bee's decision that the Davis–Wilson lattice force must be the gradient
of an energy. *Reading the Curve* (doi 10.5281/zenodo.24328085) gives a force that converges and is
stable but is not reciprocal; Section 8 shows why the obvious symmetrizations fail and names the
condition a reciprocal force must meet. This note turns that condition into a concrete, testable
construction and proposes staged experiments with gates. Nothing here is a result.

## The problem in one identity

Every force law in the paper has the form g(a)[η] = ⟨R(Da), S(Dη)⟩, and its error splits exactly
(Proposition 1) into

    reconstruction  ⟨RΦ_F − F, S Dη⟩   +   test variation  ⟨F, (S − j) Dη⟩.

A variational force is the gradient of E(a) = ½⟨R Da, H R Da⟩, so S = R. The reconstruction term is
small when R is accurate (the two-ring fit is second order). The test-variation term is
⟨(R − j)ᵀ H F, Dη⟩: the recovery's *transpose* applied to the smooth field. Nothing in the two-ring
fit controls it, which is why the variational two-ring force diverges (45%, 276% at n = 1, 2).

So a variational force needs one recovery R satisfying two linear conditions at once:

- **Rows (accuracy):** R X_K = I on each cell's model space (closed linear fields reproduced). The
  two-ring least-squares fit already satisfies this.
- **Columns (transpose blind to smooth fields):** (R − j)ᵀ H G ≈ 0 for every smooth field G, i.e. for
  each face f, Σ_{K : f ∈ N(K)} w_K ⟨(R − j)_{K,f}, G⟩_{H_K} ≈ 0.

## Why a local fix cannot work, and why a sparse global one might

Per cell, the correction R − j must carry F − jF (the part of a smooth field Whitney misses). That
has a first-order linear part, so a correction made orthogonal to linear fields *cell by cell* loses
accuracy; one that keeps the linear part fails the column condition at first order. The two
conditions conflict cell by cell. They need not conflict *across* cells: the column condition is a
sum over the cells whose neighbourhoods contain a face (15–44 of them), so corrections can cancel
between neighbours. That is the "solvable but dense" coupling the paper mentions — but the coupling
is sparse (each face touches a bounded neighbourhood), so the right formulation is a sparse
constrained least-change problem:

    minimise  Σ_K ‖R_K − R_K^{two-ring}‖²
    subject to  row exactness on every cell, and the column condition for every face
                and every G in a chosen smooth subspace 𝒮.

Unknowns: about 11 × 40 = 440 per cell (≈ 53,000 at n = 1, ≈ 420,000 at n = 2); constraints: 11 row
conditions per cell plus dim 𝒮 per face. A sparse KKT system; feasible with sparse direct or
iterative solvers.

## The honesty trap: which smooth subspace

The Abelian control's own field (dθ₁) is smooth. If 𝒮 contains it, the test-variation term vanishes
for the scored field by construction, and a good error proves nothing. Options, in order of
preference:

1. **Local linear 𝒮.** For each face, 𝒮 = closed linear fields in that face's chart (11-dimensional),
   carried into each neighbouring cell's chart with the existing transport. Generic (every smooth
   field is locally linear to first order), not tuned to the control. Requires chart bookkeeping per
   face; this is the principled choice.
2. **Hold-out global 𝒮.** Enforce on low Whitney eigenmodes excluding the band that contains the
   control, score on the control. Simpler, but the separation is imperfect.

## Staged plan (each stage: spec first, gates fixed before measurement)

- **V1 — feasibility at n = 1, 2 (Abelian).** Solve the sparse KKT problem with option 1. Gates:
  (a) the constraints are satisfiable (residual ≤ 10⁻¹⁰); (b) the variational force of the new R
  passes the **unchanged BW05** numbers at n = 1, 2 better than the current variational force
  (45%, 276%) and at least as well as Whitney (26%); (c) S3 stability. Failure of (a) or (b) means
  the column-condition route is dead as posed, and that is the finding.
- **V2 — convergence to n = 3, 5 and the full BW05 gate**, with the amplification of the new R
  recorded (the KKT change could re-introduce the conditioning problem two rings solved).
- **V3 — SU(2):** the covariant version, then the time flow, as for the two-ring force.

## What I need from Bee before V1

- Agreement that a **variational force built on a different (constrained) recovery** is acceptable,
  as opposed to requiring that the energy be exactly ½‖two-ring recovered curvature‖² (which is the
  one already shown to diverge).
- Whether option 1 (local linear 𝒮) is the right notion of "smooth" for the programme, or whether the
  physics (e.g. a particular symplectic/Hamiltonian structure) dictates a different energy.
