# Helike's eigenvalue is helicity squared minus flux: the curl-spectrum check on Berger spheres

24 November 2026. **COMPUTED, with exact identities checked numerically.** This is the first intention of the
[reflection note](helike-reflections-and-intentions-2026-11-24.md). Code: `scripts/helike_curl_spectrum.py`,
reusing the sector conventions of `scripts/ricci_su2_spin_sector.py`. Tests: `scripts/test_helike_curl_spectrum.py`
(8 passing). Mutation check: `scripts/check_helike_curl_mutations.py` (4 of 4 caught by assertion).

## The hypothesis, and what it became

The reflection note guessed that Helike's classical floor is the smallest |curl eigenvalue|. That guess assumed the
Hessian at the trivial connection, (curl)². The operator in *By Enough, But Not Too Much* is the Yang–Mills Hessian at
the Berger field, which is not flat (B = 3t − 4). In three dimensions Σ_{i<j}‖D_{ij}b‖² = ‖∗D_A b‖², so the paper's
second variation is exactly

  Q(b) = ‖curl_A b‖² + 2⟨F, [b, b]⟩,  H = curl_A² + Φ,

where curl_A = ∗D_A is the Hessian of Chern–Simons at A, and Φ is the field-strength coupling. In Chern–Simons
language this is Hess(½|∇W|²) = (Hess W)² + ∇W·∇³W: the first term is helicity squared, the second is the gradient
of W contracted with its cubic term. Checked numerically: curl_A is Hermitian, and D_allᴴD_all = curl_A² and
H = curl_A² + Φ hold to 10⁻¹².

## Result 1: Helike's eigenvalue splits exactly (every family member tested)

The twist b = f(θ̂¹ + iθ̂²) ⊗ (τ₁ + iτ₂) is alone in its charge block, so it is an exact eigenvector of both curl_A and
Φ, not only of H. Numerically, at the Berger field and at family members:

  curl_A b = μ b,  μ = −(x + (j+2)/√t),
  Φ b = −B b,  B = y² − 2√t·x (the field along her own circle),
  λ = μ² − B.

This is the paper's closed form λ = (x + (j+2)/√t)² + 2√t·x − y², read as **helicity squared minus the flux along her
circle**. The twist and her mirror share the same μ; the curl leak off their span is at most 4×10⁻¹⁴. On the Berger
path μ = (t − j − 4)/√t (the sign set by orientation), which vanishes at t = j + 4: the j = 0 twist is a curl zero
mode at t = 4.

## Result 2: the floor is not the curl floor (the hypothesis is refuted)

Per sector, gauge-orthogonal, j ≤ 12, along the Berger path:

| Test | Result |
| --- | --- |
| Twist is the H floor of every sector | Holds from t = 2.698027 upward through t = 20 (bisection). This reproduces the published t* ≈ 2.6980, the validation. |
| Twist is the curl² floor of every sector | Fails at every tested t except near t = 4. Inside the window, competitors have smaller \|curl\|; for example the j = 0 margin is −0.2 at t = 5 and −3.6 at t = 10. |

So Helike is **not** the lowest-helicity mode. My reflection-note hypothesis, in its simple form, is wrong.

## Result 3: she is the flux minimiser, and the floor is a balance

On the Berger path the twist attains the lowest eigenvalue of Φ over all sectors, in both the full and the
gauge-orthogonal spectra, at every tested stretch (t = 2.75 to 15): min Φ = −B exactly. That gives an operator
inequality on the Berger path:

  H ≥ curl_A² − B.

The twist saturates the flux part and pays her helicity μ². She is the floor where no competitor's lower helicity can
undercut her flux advantage. The window is that balance, which is a precise sense of "by enough, but not too much":
the flux gain grows with the stretch (B = 3t − 4), and her helicity cost moves with t. The bound Φ ≥ −B holds at all
tested family members with x < 0. It fails for x > 0, where B < 0 (at (0.5, 1, 4), min Φ = −1.366 against −B = 1).
That is the side the paper proves the twist loses.

## What this changes in the thesis

The Chern–Simons thesis survives in refined form. Helike is not "the lowest mode of Hess W". She is the state that
saturates the ∇W·∇³W term, maximally aligned with the field along her circle, and her eigenvalue is (Hess W eigenvalue)²
minus that alignment. The floor operator is the Hessian of ½|∇W|², not of W. Both of its pieces are Chern–Simons
objects, and the window is their competition.

**Open.**
- An analytic proof of Φ ≥ −B on the Berger path, for all sectors (here j ≤ 10 numerically).
- The exact form of min Φ when x > 0.
- Whether the upper end of the window (proved to t = 10 in the Floor paper, observed here to t = 20 for j ≤ 12) has a
  helicity–flux explanation as well.
- What this decomposition says about the quantum probe, where the H2 channel is Helike's j = 0 member.
