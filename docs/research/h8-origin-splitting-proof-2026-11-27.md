# The charge-0 state goes down first

27 November 2026. **CLOSED BY DEDUCTION (reduced model, at q = 0):** the O(g²) splitting of the two-quantum colour
singlets at the origin, for every fixed compact simple metric Lie algebra and every t > 1. This is the comparison
H7 Step 6 left open. **H7 and H8 remain open**, and so does the physical room, which needs H2b.

Around q = 0, at s = 1, the lowest colour-singlet excitations are two quanta in rows 2 and 3. They come in charge 0
and charge ±2, and at harmonic order they tie at 2u, u = t^(-1/2). I expected the tie to be broken by an
order-g² perturbation computation with the cubic and quartic terms, carrying colour factors. It breaks more
cleanly than that. The superpotential sends the quartic term into the factorization, so only one first-order
operator is left. That operator kills the charge-±2 polynomials outright. The charge-0 polynomial goes down by an
amount fixed by the adjoint Casimir.

## 1. The operator

Same objects as the [vacuum-floor note](h8-vacuum-floor-proof-2026-11-27.md). Fix a compact simple real Lie algebra
𝔤 of dimension n with a positive Ad-invariant inner product, and write q = (X₁, X₂, X₃) ∈ 𝔤³. Let

- u = t^(-1/2), ω = (√t, u, u), S = √t + 2u;
- 𝒞(q) = ⟨X₁, [X₂, X₃]⟩, W = ½ Σ ω_k ‖X_k‖² − g𝒞;
- H = −½Δ + ½‖∇W‖², with its full-space Friedrichs realization.

Let κ be the adjoint Casimir in this metric: Σ_c ad(e_c)² = −κ on an orthonormal basis. Equivalently,
Σ_c ‖[X, e_c]‖² = κ‖X‖². In the physics normalization (Tr t_a t_b = ½δ_ab in the fundamental of SU(N)), κ = N = h^∨.
The SU(2) convention of the register, with the cross product as bracket, has κ = 2.

**Restriction t > 1.** For t > 1 the lowest singlet excitations at the origin are the rows-2,3 two-quantum states
(H7 Step 6). For t < 1 the state ‖X₁‖² at 2√t lies lower, and the comparison is a different one.

## 2. The ground-state picture

The vacuum-floor note proves H = ½ Σ a_i* a_i + nS/2 with a_i = ∂_i + ∂_iW, since ΔW = nS is constant. For any
smooth φ,

    ½ Σ a_i* a_i (e^(−W) φ) = e^(−W) L φ,   L = −½Δ + ∇W·∇ = L₀ + g L₁,
    L₀ = −½Δ + Σ_k ω_k X_k·∂_{X_k},   L₁ = −∇𝒞·∇.

The quartic term ½g²‖∇𝒞‖² of the potential does not appear in L. **The perturbation is exactly first order in g.**
L₀ is the Ornstein–Uhlenbeck operator, self-adjoint in L²(γ) with γ ∝ exp(−Σ ω_k‖X_k‖²). Its eigenfunctions are the
Hermite polynomials. A Hermite polynomial with row degrees (d₁, d₂, d₃) has eigenvalue d₁√t + (d₂ + d₃)u. Because
∇𝒞 is quadratic, L₁ raises polynomial degree by at most one. Everything below happens inside polynomials of
bounded degree.

The three partial gradients are ∂𝒞/∂X₁ = [X₂, X₃], ∂𝒞/∂X₂ = [X₃, X₁] and ∂𝒞/∂X₃ = [X₁, X₂]. They follow from
ad-invariance: ⟨X₁,[X₂,X₃]⟩ = ⟨X₂,[X₃,X₁]⟩ = ⟨X₃,[X₁,X₂]⟩.

## 3. Charge ±2: the tie does not break at any order

By Schur's lemma, the invariant symmetric bilinear forms on the compact simple 𝔤 are the multiples of the metric. So
the quadratic singlets in rows 2 and 3 are spanned by ‖X₂‖², ‖X₃‖² and ⟨X₂,X₃⟩. The charge-±2 pair is
P_± = ‖X₂‖² − ‖X₃‖² ± 2i⟨X₂,X₃⟩.

- ∇𝒞·∇(‖X₂‖² − ‖X₃‖²) = 2⟨X₂,[X₃,X₁]⟩ − 2⟨X₃,[X₁,X₂]⟩ = 2𝒞 − 2𝒞 = 0. For a row-trilinear cubic this is Euler
  homogeneity, and invariance is not needed.
- ∇𝒞·∇⟨X₂,X₃⟩ = ⟨[X₃,X₁],X₃⟩ + ⟨X₂,[X₁,X₂]⟩ = 0. Each term is ⟨Y,[Z,Y]⟩ = 0 by ad-invariance and antisymmetry.
  This step does need invariance: a non-invariant trilinear cubic breaks it (exact mutation in the checker).
- Δ‖X₂‖² = Δ‖X₃‖² = 2n, so the Laplacian cancels in the difference, and Δ⟨X₂,X₃⟩ = 0.

So **L₁P_± = 0 and L₀P_± = 2uP_±**, hence L P_± = 2u P_± exactly. As a formal series the charge-±2 level is 2u,
with every correction zero.

## 4. Charge 0: a closed-form shift downward

Let r = ‖X₂‖² + ‖X₃‖² − n/u, the centred Hermite polynomial, so L₀r = 2ur. For t > 1 it is the only charge-0
singlet eigenpolynomial of L₀ at 2u. That is because:

- a Hermite energy d₁√t + d₂₃u equals 2u only for (d₁, d₂₃) = (0, 2), or for (1, 0) at t = 2;
- a polynomial linear in X₁ is never a singlet, since the adjoint has no invariant vector;
- among the rows-2,3 quadratics, the charge-0 singlet is unique by Schur.

So non-degenerate perturbation theory applies inside the charge-0 singlet sector. Solve
(L − E)(r + gφ₁ + g²φ₂) = O(g³) with E = 2u + ge₁ + g²e₂.

**Order g.** L₁r = −∇𝒞·∇(‖X₂‖² + ‖X₃‖²) = −4𝒞. It is odd, so e₁ = ⟨r, L₁r⟩_γ = 0. Next, 𝒞 is multilinear in
three distinct rows, so Δ𝒞 = 0 and L₀𝒞 = (√t + 2u)𝒞. Solving (L₀ − 2u)φ₁ = 4𝒞 gives

    φ₁ = 4𝒞/√t.

**Order g².** The equation is (L₀ − 2u)φ₂ = e₂r − L₁φ₁ = e₂r + (4/√t)‖∇𝒞‖². It is solvable exactly when the
right side has no component along r in L²(γ). That fixes e₂.

The coordinate variances under γ are σ_k² = 1/(2ω_k). For two distinct rows a ≠ b, averaging ‖[X_a, X_b]‖² over X_b
gives σ_b²κ‖X_a‖². So the second Hermite chaos of ‖∇𝒞‖² = ‖[X₂,X₃]‖² + ‖[X₃,X₁]‖² + ‖[X₁,X₂]‖² carries
coefficient κ(σ₃² + σ₁²) = κ(1/(2u) + 1/(2√t)) on each of the centred ‖X₂‖² and ‖X₃‖², and a separate coefficient on
the centred ‖X₁‖², which is orthogonal to r. Hence

    e₂ = −(4/√t) · κ (1/(2u) + 1/(2√t)) = −(2κ/√t)(√t + 1/√t),

    **e₂ = −2κ(1 + 1/t)**, which is negative for every κ > 0 and t > 0.

For the register's SU(2) convention this is −4(1 + 1/t), which is −16/3 at t = 3. For SU(N) in the physics
normalization it is −2N(1 + 1/t). Only the adjoint Casimir enters; dim 𝔤 does not. Once e₂ is set this way,
φ₂ is an explicit polynomial of degree ≤ 4. The checker constructs it for SU(2) and verifies all three
order-by-order equations exactly.

## 5. From formal series to spectrum: quasimodes

Choose a smooth radial cutoff χ = 1 on |q| ≤ δ/(2g) and 0 outside |q| ≤ δ/g. Pick δ so small that
g|𝒞| ≤ c_𝔤 δ |q|² ≤ (min ω/4)|q|² on the support, where c_𝔤 is the cubic's operator-norm constant. Then
W ≥ (min ω/4)|q|² there. Put

    ψ₀ = χ e^(−W)(r + gφ₁ + g²φ₂),   ψ₂ = χ e^(−W) P_±.

Both are compactly supported and smooth, so they lie in the operator domain. They are colour singlets with charges
0 and ±2, because χ, W and the polynomials are. By §2,

    (H − nS/2 − E)ψ = χ e^(−W)(L − E)φ + [H, χ] e^(−W)φ.

- **Interior term.** (L − E)φ = g³(L₁φ₂ − e₂φ₁) − g⁴e₂φ₂ for charge 0, and exactly 0 for charge ±2. These are
  polynomials of fixed degree, and their norms against e^(−2W) on the support are bounded uniformly in g.
- **Cutoff term.** It is supported where |q| ≥ δ/(2g), where e^(−W) ≤ exp(−min ω δ²/(16 g²)).
- **Norm.** ‖ψ‖ → ‖e^(−W₀) r‖ > 0, or the same with P_±, as g → 0.

So ‖(H − nS/2 − E_q)ψ_q‖ ≤ ε_q‖ψ_q‖ with ε₀ = O(g³) and ε_{±2} = O(e^(−c/g²)). By the spectral theorem, applied in
each reducing symmetry sector:

> **Theorem (proved here).** For every fixed compact simple metric Lie algebra, t > 1, and g small enough, H
> restricted to colour singlets has spectrum
> - in charge ±2, within O(e^(−c/g²)) of nS/2 + 2/√t;
> - in charge 0, within O(g³) of nS/2 + 2/√t − 2κ(1 + 1/t) g².
>
> So whenever these two points are the levels compared in H7 Step 6, the charge-0 level is lower for
> 0 < g < g₀. The window comes from the explicit remainder: K g < κ(1 + 1/t).

The last sentence is the conditional ordering deduction that Lean checks.

## 6. What this does and does not close

- **Closed:** the O(g²) splitting that H7 Step 6 posed in the reduced model, with its sign, for every compact simple
  𝔤. It is negative in charge 0 and zero (to all orders) in charge ±2. In the reduced model, at the origin, this is
  the charge statement "the charge-0 state lies lower".
- **Not closed: counting.** The quasimode theorem shows spectrum exists at these energies. It does not show that
  these are the only levels nearby, or that they belong to the q = 0 well rather than the q_MC well. That needs the
  harmonic-approximation localization (Simon 1983 for the nondegenerate point well at 0; Helffer–Sjöstrand for the
  interaction with the q_MC orbit). It remains a cited boundary, the same one as H8 Step 5.
- **Not closed: the physical room.** The fast-mode O(g²) dressing (H2b) is not included. What H8(G) now needs at
  the origin is exactly this: that the dressing does not reverse a reduced splitting of −2κ(1 + 1/t)g².
- **Not touched:** the q_MC reading of H8 Step 7 (the competitor quantum against Helike, |c_H − c_C| < 5.2), any
  t ≤ 1, or uniformity in the group.

H7 and H8 remain open; the top-level count remains seven.

## 7. Verification

`scripts/check_h8_origin_splitting.py` (exact sympy, no spectral run):

- **SU(2), all nine coordinates:** L₁P = L₁⟨X₂,X₃⟩ = 0, L₀P = 2uP, L₁r = −4𝒞, L₀𝒞 = (√t + 2u)𝒞, e₂ = −4(1 + 1/t).
- **Quasimode equations:** φ₁ = 4𝒞/√t and φ₂ solve the order-g and order-g² equations exactly. With e₂ off by one,
  the solver refuses, because a component at 2u remains.
- **SU(3), Gell-Mann structure constants, all 24 coordinates:** charge-±2 annihilation, L₁r = −4𝒞 and
  e₂ = −6(1 + 1/t) (κ = 3).
- **Casimir values:** κ = 2 and κ = 3. A rescaled metric (scale 3) gives κ → κ/3 and e₂ rescales with it.
- **Non-degeneracy at t > 1, and Schur:** invariant symmetric forms on su(2) and su(3) are one-dimensional.
- **Mechanism removals:**
  - a non-invariant trilinear cubic keeps ‖X₂‖² − ‖X₃‖² annihilated (homogeneity) but breaks ⟨X₂,X₃⟩;
  - ω₂ ≠ ω₃ breaks L₀P = 2uP;
  - no cubic gives e₂ = 0;
  - a reversed resolvent flips the sign of e₂.

**Lean** (`formal/h7-cubic-quartic/H8OriginSplitting.lean`) checks the following, without added axioms or admitted
steps:

- the closed form and sign of e₂;
- the three ad-invariance identities behind §3 and §4, for any bracket and bilinear form satisfying invariance,
  symmetry and antisymmetry;
- the conditional ordering of two quasimode energies.

It is **not end-to-end Lean**. Polynomial differentiation, Gaussian moments, Schur's lemma, the quasimode estimate
and spectral localization are written mathematics or exact sympy checks, not formalized.
