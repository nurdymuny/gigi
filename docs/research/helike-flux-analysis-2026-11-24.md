# Helicity and flux: the analysis before anything goes in the report

24 November 2026. **ANALYSIS. Not for the report until the open items below are closed.** Follows the
[curl-spectrum check](helike-curl-spectrum-2026-11-24.md). Only light checks were run here (9 × 9 matrices and single
sectors). Code: `scripts/helike_curl_spectrum.py`. Tests: `scripts/test_helike_curl_spectrum.py` (18 passing).
Mutation check: 6 of 6 caught by assertion.

Setting: the edge paper's Hessian at the circle-symmetric family M₀ = xτ₀, M₁ = yτ₁, M₂ = yτ₂ on (t,1,1), sector j
(l = j/2). In three dimensions H = curl_A² + Φ exactly, with curl_A = ∗D_A, the Hessian of Chern–Simons, and Φ the
field-strength coupling 2⟨F, [b, b]⟩. Write B = y² − 2√t·x for the field along the long circle and
κ = y(x − 2/√t) for the field across it.

## Theorem 1 (the form of the flux; proved)

Φ = φ ⊗ I_{V_j}: it does not act on V_j, so its spectrum is the same in every sector. The 9 × 9 form φ has spectrum

  { −B (×2), +B, +κ (×2), −κ (×2), (B + √(B² + 8κ²))/2, (B − √(B² + 8κ²))/2 }.

**Proof.** Write F_ij = ε_ijk 𝓕_k. For the family, 𝓕 = diag(B, κ, κ) in (frame, Lie) indices, from the paper's
field-strength equation. Then Σ_{i<j} F_ij·(b_i × b_j) = tr(𝓕ᵀ cof b), where cof b, the cofactor matrix of
b = (b_{ia}), is quadratic in b. With 𝓕 diagonal, the form is 2[B cof₀₀ + κ cof₁₁ + κ cof₂₂]. It splits into three
off-diagonal pairs, (b₁₂, b₂₁) with −2B b₁₂b₂₁ and two pairs with −2κ, giving eigenvalues ±B, ±κ, ±κ. The remaining
part is a diagonal block on (b₀₀, b₁₁, b₂₂) with matrix [[0, κ, κ], [κ, 0, B], [κ, B, 0]]. Its eigenvalues are −B (on
(0,1,−1)) and the roots of λ² − Bλ − 2κ² = 0 on span{(1,0,0), (0,1,1)}. ∎

Checked against the assembled operator at random family members and in sectors j = 0 and 3 (error ≤ 10⁻¹¹, with
multiplicities j + 1).

**Corollary 1 (when she is the flux minimiser).** The minimum of Φ is min{−B, (B − √(B² + 8κ²))/2}. It equals −B if and
only if B ≥ |κ|. Then, in every sector,

  H ≥ curl_A² − B.

The −B eigenspace contains E₋ = V_j ⊗ span{w₊, w₋}, where w± = (θ̂¹ ± iθ̂²) ⊗ (τ₁ ± iτ₂); the twist is the top-weight
vector of w₊. For B < 0, as at x > 0, the minimum is (B − √(B² + 8κ²))/2 < −|B|, and the twist is not the flux
minimiser. This is the side the paper proves she loses.

**Corollary 2 (the Berger path).** There B = 3t − 4 and κ = −t, so B ≥ |κ| ⟺ t ≥ 2. So H ≥ curl_A² − B holds in every
sector exactly for t ≥ 2. The threshold t = 2 is also where the Berger x changes sign, and where the Floor paper's
Casimir remark has the lift's coupling to the stretch vanish.

## Proposition 2 (the flux-aligned subspace; proved below)

Inside E₋, organised by the long-axis weight m = −l, …, l:
- **Helicity.** The compression P·curl_A·P has eigenvalues μ_m = −(x + (2m + 2)/√t), each twice. The twist is m = l.
- **Leak = gauge component (Cauchy–Riemann).** For each flux-aligned state ψ_m,

    ‖(1 − P)·curl_A ψ_m‖² = ‖G*ψ_m‖² = 2[l(l+1) − m(m+1)],

  independent of x, y and t (checked at six points, including x > 0 and t < 2, to 10⁻¹⁴).

**Proof of Proposition 2 (the Cauchy–Riemann identity).** The conventions are those of
`scripts/ricci_su2_spin_sector.py`:
- the frame is ê₀ = e₀/√t, ê₁ = e₁, ê₂ = e₂, with c₁₂⁰ = 2√t and c₂₀¹ = c₀₁² = 2/√t;
- on V_j, êᵢ acts as ρᵢ/sᵢ with ρ_a = −2iJ_a, where J₀ is the long-axis generator;
- the Lie bracket is the cross product, with the field mᵢ = (xe₀, ye₁, ye₂);
- D_ij b = êᵢbⱼ − êⱼbᵢ + mᵢ×bⱼ − mⱼ×bᵢ − c_ij^k b_k, and (curl b)_k = D_ij b for (i,j,k) cyclic;
- G*b = −Σᵢ(êᵢ + mᵢ×)bᵢ, since ρᵢ is anti-Hermitian and mᵢ× is antisymmetric.

Put J± = J₁ ± iJ₂, so ê₁ ± iê₂ = −2iJ± and ê₀ = −2iJ₀/√t. Let u = τ₁ + iτ₂. Then
e₀×u = −iu, e₁×u = ie₀ and e₂×u = −e₀.

Take ψ = f ⊗ (θ̂¹ + iθ̂²) ⊗ u with J₀f = mf, that is b₀ = 0, b₁ = fu, b₂ = ifu.

- **Frame-0 slot.** (curl ψ)₀ = D₁₂ψ = i(ê₁f)u − (ê₂f)u + y[e₁×(ifu) − e₂×(fu)] = i(ê₁ + iê₂)f·u +
  yf(−e₀ + e₀) = 2(J₊f)⊗u.
- **Frame-1 slot.** (curl ψ)₁ = D₂₀ψ = −ê₀(if)u − x e₀×(ifu) − (2/√t)fu = (−2m/√t − x − 2/√t)fu = μ_m b₁.
- **Frame-2 slot.** (curl ψ)₂ = D₀₁ψ = (ê₀f)u + x e₀×(fu) − (2/√t)(ifu) = −i(x + (2m+2)/√t)fu = μ_m b₂.
- **Divergence.** G*ψ = −[(ê₁ + iê₂)f·u + yf(e₁×u + ie₂×u)] = −(−2iJ₊f)u + 0 = 2i(J₊f)⊗u.

Hence

  curl ψ = μ_m ψ + χ,  χ = (2J₊f ⊗ u, 0, 0) = (−i·G*ψ, 0, 0),

so the transverse curl is exactly −i times the divergence. This is the Cauchy–Riemann statement: along θ̂¹ + iθ̂², both
are the single operator ê₁ + iê₂ = −2iJ₊, and every field term (y, and x outside μ) cancels.

χ lies in the frame-0 slot. By the proof of Theorem 1, that slot carries only the ±κ (pair) and diagonal-block
eigenspaces of φ, which are orthogonal to the −B eigenspace whenever −B ∉ {κ, −κ}. In that case Pχ = 0, so
P·curl·ψ = μ_m ψ and (1 − P)·curl·ψ = χ, and

  ‖(1 − P)curl ψ‖² / ‖ψ‖² = ‖G*ψ‖² / ‖ψ‖² = 4‖J₊f‖²‖u‖² / (2‖f‖²‖u‖²) = 2[l(l+1) − m(m+1)].

The mirror branch ψ = f ⊗ (θ̂¹ − iθ̂²) ⊗ ū is identical with J₋ in place of J₊: e₀×ū = iū and e₁×ū = −ie₀ give
curl ψ = μ₋ₘψ + (−2J₋f ⊗ ū, 0, 0), G*ψ = 2i(J₋f)⊗ū, and leak ratio 2[l(l+1) − m(m−1)]. Finally ψ ∈ E₋: in Theorem 1's
coordinates b₁₁ = −b₂₂ and b₁₂ = b₂₁, which is the −B eigenspace. ∎

Test `CauchyRiemann.test_vector_identities_both_branches` checks every line as an exact vector identity (to 10⁻¹¹),
for both branches and all weights, at three fields including x > 0 and t < 2.

**Corollary (the twist's closed form, derived directly).** For the top weight m = l, J₊f = 0, so χ = 0 and G*ψ = 0:
the twist is an exact curl eigenvector and gauge-orthogonal. By Theorem 1, Φψ = −Bψ. So
Hψ = (curl² + Φ)ψ = (μ_l² − B)ψ, which gives

  λ = (x + (j+2)/√t)² − B = (x + (j+2)/√t)² + 2√t·x − y²,

the paper's closed form. This derivation does not use the charge-block counting. The mirror (m = −l, J₋f = 0) has
the same μ.

**Consequence.** The twist (m = l) is the only flux-aligned state that is both leak-free and gauge-orthogonal. That is
the structural reason she is an exact eigenvector and the natural floor candidate: she gets the full flux discount
−B, pays only her helicity μ_l², and loses nothing to leakage or gauge.

## Theorem 3 (a sufficient condition for the floor; proved)

Let B > |κ|, Δ = B − |κ| (the flux gap), l = j/2, μ_l = x + (j+2)/√t, and g = ‖G‖ in sector j. Suppose either
j = 0 and Δ ≥ μ₀², or j ≥ 1 and Δ ≥ μ_l²(1 + g²/4l). Then the twist's eigenvalue λ = μ_l² − B is the lowest
eigenvalue of H on ker G* in sector j.

**Proof.** Let K = ker G*.
1. **Reduce to the complement of the twist.** By the Corollary to Proposition 2, the twist ψ₊ and her mirror ψ₋ lie
   in K and Hψ± = λψ±. For ψ ∈ K, write ψ = aψ₊ + bψ₋ + ψ′ with ψ′ ⊥ ψ±. Then ψ′ ∈ K and, since H is Hermitian,
   ⟨ψ, Hψ⟩ = (|a|² + |b|²)λ + ⟨ψ′, Hψ′⟩. So it suffices to show ⟨ψ′, Hψ′⟩ ≥ λ‖ψ′‖².
2. **Split by flux.** Write ψ′ = ψ_E + ψ_⊥ along E = V_j ⊗ span{w₊, w₋} (the −B eigenspace) and its complement. Φ
   preserves both. By Theorem 1, Φ = −B on E and Φ ≥ −|κ| on E^⊥, because for B > |κ| the next eigenvalue of φ is
   −|κ|. Hence ⟨ψ′, Φψ′⟩ ≥ −B‖ψ′‖² + Δ‖ψ_⊥‖².
3. **Bound the gauge part.** ψ_E has no top-weight component, because the top weights of E are exactly ψ±. By
   Proposition 2 the G*-images of different weights and branches are orthogonal (different weights in V_j, and
   ⟨ū, u⟩ = 0). So ‖G*ψ_E‖² = Σ 2[l(l+1) − m(m+1)]|c_m|² ≥ 4l‖ψ_E‖², the minimum being at m = l − 1 and its mirror.
   Since G*ψ′ = 0, G*ψ_E = −G*ψ_⊥, so 4l‖ψ_E‖² ≤ g²‖ψ_⊥‖².
4. **Combine.** With ‖curl ψ′‖² ≥ 0,

     ⟨ψ′, Hψ′⟩ − λ‖ψ′‖² ≥ −μ_l²(‖ψ_E‖² + ‖ψ_⊥‖²) + Δ‖ψ_⊥‖² ≥ ‖ψ_⊥‖²[Δ − μ_l²(1 + g²/4l)] ≥ 0.

   For j = 0, E = span{ψ±}, so ψ_E = 0 and the bracket is Δ − μ₀². ∎

The bound g² ≤ (2l/√t + |x|)² + 2(2l + |y|)² comes from ‖Gξ‖ ≤ Σᵢ(‖êᵢ‖ + |mᵢ|)‖ξ‖, with ‖ê₀‖ = 2l/√t and
‖ê₁,₂‖ = 2l. Theorem 3 holds on the whole family, not only the Berger path.

**What it proves.**
- **Sector 0, for every t ≥ 2√5 − 2 ≈ 2.472, with no upper limit.** On the Berger path the condition is t² + 4t − 16 ≥ 0.
  Spot checks agree out to t = 80.
- **A moving band of sectors near j ≈ t − 4,** where her helicity is small: j ≤ 1 at t = 4; 0–3 at t = 6; 0 and 3–8 at
  t = 10; 0 and 11–20 at t = 20; 17 of the first 61 sectors at t = 40.
- **Never violated:** the condition held at 1,222 random (family member, sector) pairs, and the twist was the floor at
  every one.
- **The gauge factor is needed.** The weaker condition Δ ≥ μ_l² alone is false: at (x, y, t) = (0.0674, 2.6596, 3.5715)
  sector 1 has a state 0.69 below the twist. That point is a test.

**What it does not prove.** The floor in every sector across the window. The bound throws away ‖curl ψ′‖² and uses
the global ‖G‖, which grows like 2l. The sectors far from j ≈ t − 4, where the twist's own helicity is large, are not
covered. Sharpening routes: keep the curl term on E^⊥, and use the exact G*-coupling inside each charge block (≤ 9
states) instead of the global norm. The Floor paper's interval certificates remain the proof there for t ∈ [2.75, 10].

## The x-band check

Corollary 1's flux boundary x_flux(y, t), the largest x with B ≥ |κ|, is

  x_flux = min{ y(y + 2/√t)/(2√t + y), y(y − 2/√t)/(2√t − y) }  (0 < y < 2√t).

Against the paper's Table 2 upper edges:

| t | y | x_flux | paper x*_A / x*_S |
| --- | --- | --- | --- |
| 2 | 0.5 | −0.196 | −0.613 / −0.605 |
| 3 | 0.5 | −0.110 | −0.335 / −0.318 |
| 4 | 1.5 | +0.300 | −0.208 / −0.192 |
| 6 | 1.5 | +0.302 | −0.045 / −0.033 |
| 8 | √8 | +1.179 | +0.083 / +0.138 (cut-off artefacts) |

**The flux boundary is not the x-band edge.** It lies to the right of the edge everywhere, and it reaches x > 0, where
the paper proves she loses. So the twist loses the floor while she is still the flux minimiser. The upper x-edge is set
by a competitor that wins on helicity (B₄,₀ for y ≳ 1.5, per the paper), not by flux alignment failing. In the
helicity–flux picture, the x-band is a helicity competition inside the flux-favourable region. Corollary 1 marks the
outer boundary of the flux mechanism, not the edge.

## A route I tried and eliminated

A flux-aligned trial state of weight m has Rayleigh quotient μ_m² + 2[l(l+1) − m(m+1)] − B. For m = −l it drops
below the twist's value when t < 8/3. That looked like a rigorous
lower-edge bound close to the published 2.698. It is not one. Those trial states are not gauge-orthogonal (by
Proposition 2 their gauge component is exactly their leak), so they bound only the full spectrum. The full spectrum
is dominated by gauge directions at these non-critical fields. The paper's gauge-orthogonal floor at t = 2.6, j = 2
is still the twist. Recorded so I do not reuse it.

## What is still open (before the report)

1. **The floor itself.** Partly proved: Theorem 3 gives sector 0 for all t ≥ 2.472 and a band near j ≈ t − 4. The rest
   is open. Earlier wording: Theorem 1 and Proposition 2 explain the twist's eigenvalue and her privileged place in E₋
   but do not prove she is the gauge-orthogonal floor: competitors from outside E₋ pay the flux gap
   Δ = B − |κ| (2t − 4 on the Berger path) but can save helicity. The lower edge t* = 2.698 comes from such a
   competitor. A proof route: H ≥ curl_A² − B + Δ(1 − P), combined with gauge-orthogonality, to bound the
   complement's contribution sector by sector.
2. **The upper end.** On the Berger path the flux gap 2t − 4 grows and the twist's helicity cost (j + 4 − t)²/t can be
   made small in the sector j ≈ t − 4. Observed: the twist is the floor of every sector j ≤ 12 up to t = 20. My
   conjecture is that there is no upper edge in t on the Berger path, and that the Floor paper's t ≤ 10 is the
   certificates' domain, not a physical edge. Not proved.
3. ~~A written proof of Proposition 2~~ **Done** (above): the transverse curl is −i times the divergence along θ̂¹ + iθ̂².
4. ~~The x-band~~ **Checked:** the flux boundary is not the edge; the edge is a helicity competition (above). Still
   open: the B₄,₀ competitor written in helicity–flux terms.

## What it says about the quantum probe (exact, light checks)

The matrix model's potential is V = ½|∇W|², so Hess V = (Hess W)² + ∇W·∇³W: helicity squared plus flux.
- **At its vacua** ∇W = 0, so the flux term vanishes and the local spectrum is helicity alone: |μ| = 1 at the origin;
  −1, 0, +2 by total spin at the second vacuum.
- **At the sphaleron** (one grain, by total spin J):

  | J | Hess V | helicity² | flux |
  | --- | --- | --- | --- |
  | 0 (breathing) | −½ | 0 | −½ |
  | 1 (orbit) | 0 | ¼ | −¼ |
  | 2 (shape) | 5/2 | 9/4 | +¼ |

  The sphaleron's unstable direction is pure flux. The orbit zero modes are helicity² exactly cancelled by flux. The
  spin-2 shape channel, where the quantum probe H2 lives, pays a flux penalty there, where Helike on the Berger
  sphere gets a flux discount.

  This suggests a test: a Berger-deformed matrix model supplies a field along the long circle (B ≠ 0). There, the
  |m| = 2 spin-2 states should gain a flux discount proportional to B, and the quantum probe should feel Helike's
  mechanism directly. That needs its own registration.

## For the other machine (intense runs, not here)

- Per-sector gauge-orthogonal floor scan on the Berger path, t ∈ [10, 60], j ≤ 60, with bisection of any edge. This
  tests the no-upper-edge conjecture.
- The same scan along x at fixed t, for comparison with the paper's x-band and Corollary 1.
- Proposition 2 at large j (j ≤ 60) and random family members.

## In plain words (how I picture it)

- **The space** is a three-sphere stretched along one family of circles, the long circles, by the factor t. Picture
  a bundle of circles, all longer than they are wide.
- **The background field** has two strengths, one along the long circles (B) and one across them (κ).
- **A disturbance pays two prices.** It pays for how much it twists against its own grain (helicity squared, μ²), and
  it is paid for lining up with the flux along the circles (−B at best).
- **Helike** is the disturbance that turns in the two short directions and rides the long circles. She takes the full
  flux payment and pays only her own twist: λ = μ² − B.
- **The lane and the ladder.** All the flux-aligned disturbances form a lane, organised like a ladder by their turning
  number m. Every rung but the top leaks sideways, into the long direction, and that leak is exactly its spreading.
  Twist and spread are two faces of one complex derivative (Cauchy–Riemann). The top rung has nowhere higher to go,
  so it cannot leak. That is Helike: the top of the ladder, the one flux-aligned disturbance that neither leaks nor
  spreads.
- **The window.** She is the floor where the flux gap (how much better the lane pays than anything outside it) covers
  what her twist costs. Stretching the sphere grows the gap, so "by enough" is a stretch. Inside the flux-favourable
  region, a neighbouring sector can still undercut her by twisting less. That sets the edge in x: "not too much".
