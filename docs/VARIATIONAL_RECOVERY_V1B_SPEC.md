# V1b: the constrained recovery that minimises stiffness, not change (spec)

2026-11-19. Written before any V1b measurement, and before V1's n = 2 result was read. Bee: "do V1b
if n = 2 fails". Same script as V1 (`scripts/ricci_variational_v1.py --objective stiffness`), same
constraints, controls, gates and hold-out as `docs/VARIATIONAL_RECOVERY_V1_SPEC.md`; only the
objective of the solve changes. Run only if V1-acc fails at n = 2.

## Why

At n = 1, V1's minimum-norm change satisfied the column condition but multiplied the recovery's
amplification by 6.5, and the reconstruction term carried that amplification (4.7% → 432%). The
size of the change was the wrong thing to minimise: what matters is how strongly the new recovery
responds to data.

## The objective

Over all C_K satisfying V1's constraints (own fluxes and closed linear fields kept exactly; the
column condition on every edge for the 23 enforced smooth fields), minimise

    S(C) = Σ_K w_K ‖G_K^{1/2} M'_K D_K‖²_F = trace of the variational operator K'.

It is the energy of the recovered field summed over a complete set of unit edge data: the total
stiffness of the energy E(a) = ½ aᵀK'a. It uses no knowledge of the control field. The problem is a
convex quadratic with linear constraints; with S1_K = Z_KᵀG_KZ_K and S2_K = (N_KᵀD_K)(N_KᵀD_K)ᵀ it
reduces to V1's minimum-norm solve in the variables C̃_K = √w_K S1_K^{1/2} C_K S2_K^{1/2}, shifted by
the unconstrained minimiser. Directions of C_K that do not change T'_K (the null space of S2_K) are
set to zero.

## Gates (unchanged from V1)

- **V1-acc.** BW05 relative error below the two-ring variational force (45%, 276% at n = 1, 2) and
  below Whitney (29.8%, 26.0%).
- **V1-stable.** S3 at n = 1, 2: no negative eigenvalue of K' on the physical space, gauge count
  21, 163.
- **Reported:** trace(K') against V1 and against the two-ring variational force; amplification;
  the reconstruction and test-variation terms.

## Controls

V1's, plus **C-opt**: the solution satisfies the constraints to 10⁻⁸ relative, and trace(K') is not
larger than V1's (it minimises it over the same feasible set).
