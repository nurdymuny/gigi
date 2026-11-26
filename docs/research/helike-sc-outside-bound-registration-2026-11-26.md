# S_C: the outside of the tube, by rays from the origin (registration before code)

26 November 2026. **REGISTRATION, before implementation.** This is item 1 of R1.9.4.

**Claim to prove at each control.** inf V over the complement of the gauge-swept product tube
T = {(x₀ + N(u, v))R : |u| < 5, |v| < 2, R ∈ SO(3)} is at least ρ_C. The Neumann operator H_out^N ≥ inf V there,
because its kinetic part is ≥ 0. With the inside chain of R1.8.5, λ₅(H) ≥ min(ℓ₅, ρ_C) and Lemma R then give S_C.

## The picture

The valley floor is a ring (the SO(3) orbit of q_b), and T is a fat ring around it. Every point is reached by a ray
from the origin, and along a ray the potential is an explicit quartic. So the bound is proved ray by ray, exactly as
in the fold check of R1.8.4. A radius R_n = 8 splits the work between the two pictures that describe each part
best.

## O1. Near the ring (slice coordinates)

Every q has a nearest orbit point q_b R. Then q − q_b R is normal to the orbit there, so q = (x₀ + Nz)R with
|z| = dist(q, orbit). If q ∉ T, then this z (like every slice representative of q) lies outside B_u(5) × B_v(2). So
it suffices to prove

    P(z) ≥ τ = ρ_C − V_bg   for all z with |z| ≤ R_n and z ∉ B_u × B_v.

No injectivity of the slice map is needed for this step.

**Method.** Rays z = s·x, with x on the symmetry-reduced max-norm faces of R1.8.1; P and the box are invariant. The
segment to check is s ∈ [s_exit(x), R_n/|x|], with s_exit = min(L_u/|x_u|, L_v/|x_v|). P(s·x) = s²Q₂ + s³P₃ + s⁴P₄.
Per x-box:
- enclosures of Q₂, P₃ and P₄;
- an s-interval covering every x in the box;
- the exact minimum of s²q_lo + s³p3_lo + s⁴p4_lo over that interval (endpoints and the roots of the derivative's
  quadratic factor), compared with τ;
- split on failure.

## O2. Far from the ring (symmetric matrices)

Every q = S R′ with S symmetric (polar decomposition, with a sign moved into R′ ∈ SO(3)). V(q) = V(S), and
dist(q, orbit) = dist(S, orbit). So it suffices to prove

    V(S) ≥ ρ_C   for all symmetric S with dist(S, orbit) ≥ R_n.

**Coordinates.** Orthonormal Frobenius coordinates are used. The charge conjugation S ↦ LSLᵀ, L = diag(1, R_θ),
preserves V, the orbit and Sym. It rotates the traceless 2–3 block by 2θ and (s₁₂, s₁₃) by θ. Together with the
reflection diag(1, 1, −1), it reduces to s₂₃ = 0, s₂₂ ≥ s₃₃, s₁₂ ≥ 0, s₁₃ ≥ 0. That gives 5 coordinates
(s₁₁, d, m, a, b) on a half-space cone, covered by the 7 max-norm faces s₁₁ = ±1, m = ±1, d = +1, a = +1, b = +1.

**Rays.** V(r·x) = r²A(x) + r³B(x) + r⁴C(x), with A = ½Σ(ω_k² − src_k)|row_k|² > 0, B = −g(Σω_k) det and
C = (g²/2)|cof|² ≥ 0. V(0) = 0 > ρ_C. Per (x-box, r-interval) cell, one of two things must be shown:
- **(clear)** the exact minimum over the r-interval of r²A_lo + r³B_lo + r⁴C_lo is ≥ ρ_C; or
- **(near)** sup over the cell of |r·S(x) − q_b R*|_F < R_n. Here R* is an exactly orthogonal Cayley rotation chosen
  per cell by a floating Procrustes fit; any R ∈ SO(3) gives an upper bound on the distance.

Initial r-intervals are [0, r_b], where r_b is the larger root of A_lo + B_lo r + C_lo r² (beyond it the lower quartic
is ≥ 0 > ρ_C). If B_lo ≥ 0, or the discriminant is ≤ 0, the box is clear at once. If C_lo ≤ 0 with B_lo < 0, split.

## Decision and checks

- **OUTSIDE PROVED** at a control if O1 and O2 both close within budget (O1: 400,000 boxes; O2: 1,000,000 cells).
- **UNRESOLVED** otherwise, naming the open cell.

**Checks:**
- the sampled minimum of P on the O1 region, from the §8.3 shell minima, must exceed the proved bound only through
  slack;
- the O2 cover must contain the floating global-minimum search of Route 1 (nothing below V_bg + 20 off the ring).

**Mutants** (each must be caught):
- R_n in O2 larger than in O1 (a gap between the regions);
- R* not orthogonal (a float matrix);
- B taken with the wrong sign;
- a dropped face;
- s_exit replaced by the larger wall.

ρ_C is the recomputed value of R1.9.2 until the frozen receipt value is substituted. The threshold is inclusive of
the slack only through ρ_C itself.
