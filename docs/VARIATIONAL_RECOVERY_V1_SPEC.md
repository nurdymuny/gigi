# V1: a variational force from a constrained two-ring recovery — feasibility at n = 1, 2 (spec)

2026-11-19. Written before any measurement in this file. Design: `docs/research/variational-force-design-2026-11-19.md`
(Bee approved "constrained recovery is fine, go ahead with V1"). Research script
`scripts/ricci_variational_v1.py`, **not an engine run**. Abelian control of *Reading the Curve*
(round S³/2I, A = θ¹, target velocity −4a), the same frozen inputs, mass, clock and BW05 measure.

## In plain English

The force must be the gradient of an energy. The obvious energy, ½‖recovered curvature‖², gives a
force that diverges because the recovery responds to rough test variations. V1 changes the
recovery as little as possible so that its transpose stops responding to smooth fields, while
keeping everything that made the two-ring recovery accurate, and then measures the resulting
energy gradient against the unchanged gate. It is a feasibility test: does any recovery of this
form give an accurate variational force at all?

## The construction

Per cell K, the two-ring fit is a linear map M_K (11 × faces of N(K)) from fluxes to the 11
coefficients of a closed linear field, with T_K = M_K D_K its map from edge values. The new map is

    M'_K = M_K + Z_K C_K N_Kᵀ,

with Z_K an orthonormal basis of the null space of K's own-face flux rows (so K's own fluxes stay
reproduced exactly) and N_K an orthonormal basis of the left null space of K's flux map X_K (so
M'_K X_K = I: closed linear fields stay reproduced exactly). C_K is the only unknown (about 8 × 30
per cell).

The energy is E(a) = ½ Σ_K w_K (T'_K a)ᵀ G_K (T'_K a), G_K the metric Gram of the trial basis, and
the force is its gradient K' = Σ_K w_K T'_Kᵀ G_K T'_K: symmetric by construction.

**Column condition.** For every edge e and every field G in 𝒮,

    Σ_K w_K (T'_K[:, e])ᵀ v_K(G) = Σ_K w_K (T^W_K[:, e])ᵀ u_K(G),

where v_K(G) pairs K's trial basis with G and u_K(G) pairs K's Whitney face functions with G, all in
the metric of K, and every cell in the sum is first carried onto the sheet of one fixed reference
cell containing e (the same deck transforms the two-ring fit uses). This says the variational
force's test-variation term vanishes for every field in 𝒮.

**C_K** is the minimum-norm solution (Σ_K ‖C_K‖²) of these linear conditions, solved through the
normal equations per level; redundant generators are removed per edge by an SVD of their Gram on
the reference cell (tolerance 10⁻¹⁰ relative).

## The smooth fields 𝒮, and the hold-out

Smooth closed 2-forms of ℝ⁴ restricted to S³: the six constant forms dx_i∧dx_j and the twenty
linear closed forms d(x_k x_l dx_m). The control field dθ¹ is, restricted to S³, *exactly* one of the
three left-invariant constant forms. To keep the test honest, **the three left-invariant constant
forms are excluded from 𝒮**; 𝒮 is the three right-invariant constant forms plus the twenty linear
ones (23 generators). At any point the right-invariant constants span the same local directions, so
𝒮 is still a first-order basis of smooth fields, but it does not contain the scored field.

## Controls (thresholds fixed now)

- **C-span.** On every cell, the control's exact proxy (−2 θ₂ × θ₃) lies in the span of the three
  held-out left-invariant forms to 10⁻¹² relative, and its distance from the span of the 23 enforced
  generators is at least 10⁻³ relative (proves the proxy pipeline and that the hold-out is real).
- **C-repro.** With C_K = 0, the assembled variational force reproduces `ricci_pg_curvature.build(...,
  force="variational")` with two rings: relative error equal to 10⁻¹⁰ (45% at n = 1, 276% at n = 2).
- **C-exact.** After the change, every cell still reproduces its own fluxes and every closed linear
  field to 10⁻¹⁰.
- **C-solve.** The column condition holds after the solve: residual ≤ 10⁻⁸ of its initial size.

## Gates

- **V1-acc.** The new variational force's BW05 relative error is below the current variational
  force's (45%, 276%) at n = 1 and n = 2, **and** below Whitney's (29.8% at n = 1, 26.0% at n = 2).
- **V1-stable.** S3 at n = 1, 2: the symmetric operator K' has no negative eigenvalue against the mass
  on the physical space, and its small-eigenvalue count equals the gauge dimension (21, 163).
- **Reported, not gated:** the per-cell change ‖C_K‖, the amplification ‖M'_K‖₂ against ‖M_K‖₂, and the
  reconstruction and test-variation terms of Proposition 1 separately.

If V1-acc fails with C-solve satisfied, the column condition on 𝒮 is not enough for accuracy and the
route needs rethinking; that is the finding. If C-solve cannot be satisfied, the constraints are
inconsistent at this locality.

## Not measured

n = 3, 5 (V2), SU(2) (V3), the flow.
