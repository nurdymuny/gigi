# Two grains, Phase 3 start: Molien counts, a proven basis, and the budget

24 November 2026. **COMPUTED (exact counts and an exact-rank proof). No spectra.** Phase 3 of the
[two-grain scope](helike-two-grain-scope-2026-11-24.md). Code: `scripts/ym_two_grain_molien.py`. Tests:
`scripts/test_ym_two_grain_molien.py` (7 passing). Mutation check: `scripts/check_ym_two_grain_molien_mutations.py`
(4 of 4 caught by assertion).

## The count

The group is G = SO(3)_colour × SO(3)_L × SO(3)_R. q carries colour and the left spatial index; p carries colour
and the right. A sector (j_L, j_R) counts colour-singlet multiplets of spin j_L under SO(3)_L and j_R under SO(3)_R.
Its bigraded Hilbert series is the Molien–Weyl average of χ_{j_L}χ_{j_R}/(det(1 − sρ_q) det(1 − tρ_p)) over the three
maximal tori. The q-part depends only on the colour and left angles and the p-part only on the colour and right
angles, so the triple average factorises. A uniform grid with the Weyl weight (1 − cos θ) is exact for these
trigonometric polynomials, and the code refuses non-integral results.

Checks, all exact:
- The one-grain marginals reproduce the verified solvers: 23, 34, 47, 64, 84, 108 (J = 0) and 24, 38, 57, 81, 111, 148
  (J = 2) at cutoffs 10 to 20.
- An independent count, the null space of the nine Lie-algebra generators on monomials, agrees at bidegrees (1,1),
  (2,1), (3,0), (4,0), (2,2) and (3,1).

Sector sizes (number of multiplets with total degree ≤ N):

| N | One grain J = 0 | (0,0) | (1,1) | (2,0) = mirror (0,2) | (2,2) | (2,1) | (1,0) | One grain J = 2 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 14 | 47 | 666 | 1,834 | 1,033 | 2,300 | 1,573 | 332 | 57 |
| 20 | 108 | 3,872 | 14,722 | 8,224 | 24,344 | 16,058 | 3,370 | 148 |
| 24 | 169 | 10,598 | 46,720 | 26,027 | 86,252 | 55,937 | 11,671 | 244 |

## A proven basis for the (0,0) sector through cutoff 20

The colour-singlet invariants are built from the two colour Gram matrices A = qqᵀ and B = ppᵀ and the two
determinants. Candidate generators:

  T_q, P_q, d_q, T_p, P_p, d_p, X₁₁ = tr AB, X₂₁ = tr A²B, X₁₂ = tr AB², X₂₂ = tr A²B²

(and Y = tr A²B²AB). For every bidegree of total degree ≤ 20, all 231 of them, I evaluated every product of
generators at random integer points modulo the prime 2147483629, and computed the rank exactly. A rank mod p is at
most the rank over the rationals, which is at most the Molien count. At every bidegree the ten generators give

  number of monomials = exact rank mod p = Molien count.

So their monomials of total degree ≤ N form a basis of the (0,0) sector for every N ≤ 20. That is a proof, not a
sampling estimate. The rank from rational points alone is only a lower bound, as the revised scope says; the
Molien count supplies the upper bound. Y is not needed through degree 20: at (6,6) it lies in the span of the
others. A test checks each generator's homogeneity, after a mutant showed that a wrong-degree generator could
otherwise pass the rank test.

**A relation must appear above degree 20.** The group has dimension 9 and acts on 18 variables with finite generic
stabiliser, so there are only 9 algebraically independent invariants, and there are 10 generators. Beyond N = 20
the solver must handle Y and the relations; the same exact-rank tool will find them.

## What the count says about physics

- **The crossing question changes shape.** Physical spin about a point of S³ is the diagonal SU(2) inside
  SU(2)_L × SU(2)_R. With one grain every state was (j, 0), so physical spin was j. With two grains, physical
  scalars come from every (j, j) sector and physical spin 2 from (2,0), (0,2), (1,1), (2,2), (2,1) and others.
- **A new low competitor.** The (1,1) sector begins at degree 2 with qᵀp, a colour-contracted product of one mode
  from each grain. Under physical spin it contains J = 0, 1 and 2, at the oscillator level alongside the one-grain
  scalar and tensor. It did not exist with one grain.
- **Parity partners.** (2,0) and (0,2) are mirror sectors with identical counts, consistent with the review's point
  that an SO(4)-invariant Hamiltonian cannot split them.

## The build plan

1. **Basis.** The ten-generator monomials of total degree ≤ N (proven above for N ≤ 20), times
   e^(−(T_q + T_p)/2), in x = √α coordinates as before.
2. **Exact moments.** q and p are independent Gaussians. Write A = U_q diag(a) U_qᵀ and B = U_p diag(b) U_pᵀ with
   U_q, U_p ∈ SO(3) (the determinant signs carried by the signed singular values). The relative rotation
   R = U_qᵀU_p is Haar-distributed and independent of the eigenvalues, so
   X_{αβ} = Σᵢⱼ aᵢ^α bⱼ^β Rᵢⱼ². Averaging over R gives exact rationals: Rᵢⱼ is quadratic in a unit quaternion, and the
   quaternion moments are the Dirichlet(½,½,½,½) moments already used for the dome. What remains are symmetric
   functions of a and b, whose moments come from the exact one-grain recursion.
3. **Exact operator tables.** Gradient inner products and Laplacians of the ten generators, reduced back to the basis.
   For example |∇X₁₁|² = 4(X₁₂ + X₂₁). Products that leave the span of a bidegree are reduced by an exact linear solve
   against the proven basis at that bidegree.
4. **Linear algebra at scale.** At N = 20 the sector has 3,872 functions. Pure-Python mpmath, as in the one-grain
   solvers, will not reach that. Options: python-flint (arb matrices in C) on your machine, or an orthogonalised
   basis built exactly and solved in float64 on the GPU.
5. **Scope of the Hamiltonian.** Decide before running: either implement van Baal and Hari Dass's boundary
   identifications (eqs. (96)–(108)), or state explicitly that the spectra are for the truncated Hamiltonian on
   R¹⁸ without them.
6. **Sectors to solve.** (0,0) first, then (2,0) and (1,1). The physical-spin decomposition, the parity pairs and the
   matched comparison with the one-grain results are fixed in a registration before any spectrum is computed.
