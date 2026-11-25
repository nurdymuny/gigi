# Condition (A): the sourced condensate has discrete spectrum in every sector

25 November 2026. **PROOF.** It replaces the sketch of condition (A) in the
[Kato–Temple registration](helike-temple-ordering-registration-2026-11-25.md), §1. The
[ordering results](helike-temple-ordering-results-2026-11-25.md) are therefore conditional on S_C alone.

## The statement in plain English

Far from the origin, the sourced potential grows at least like a fixed positive multiple of |q|². This holds in
every direction, on both determinant sheets, at every coupling g > 0, and everywhere in the registered window
t > 1, 1 < s < (t + 2)/(t + 1). A quantum particle in such a well cannot escape to infinity. So in every symmetry sector the
spectrum is a list of isolated eigenvalues of finite multiplicity, increasing to infinity, with no continuum.

That is exactly what the Weinstein, min–max and Kato–Temple arguments need. The window condition is not an extra
assumption: the same inequality that defines the window makes the quadratic part of the potential positive.

## Setting

- **The operator.** q ∈ ℝ^{3×3} (rows are frame indices, columns colour) with the Euclidean (Frobenius) norm |q|.
  The registered operator is H = −½Δ + V on L²(ℝ⁹), in its regular Cartesian (Friedrichs) realisation, restricted
  to colour singlets. The potential is

      V(q) = ½|ω∘q − g cof q|² − ½ Σ_k src_k |q_k|²,

  with:
  - ω = (√t, 1/√t, 1/√t);
  - src = (a, b, b), a = (1 − s)(t − 2s), b = (1 − s)((1 − s)/t − s);
  - g > 0, t > 0;
  - cof q the cofactor matrix (rows q₁ × q₂, q₂ × q₀, q₀ × q₁).
- **Two symmetries act unitarily on L²(ℝ⁹):**
  - colour rotations q ↦ qR, R ∈ SO(3);
  - the spatial rotation q ↦ R_θ q, with R_θ rotating rows 1 and 2.

  A sector is the closed subspace of colour singlets with a fixed integer charge m of the second action.

## Lemma A1 (expanded potential)

    V(q) = ½ Σ_k (ω_k² − src_k)|q_k|² − g S det q + (g²/2)|cof q|²,   S := ω₀ + ω₁ + ω₂.

**Proof.** Expand the square. The cross term is −g Σ_k ω_k ⟨q_k, (cof q)_k⟩. Row k of the cofactor matrix is
orthogonal to the other two rows, and its inner product with row k is det q (Laplace expansion). ∎

## Lemma A2 (the quadratic part is positive exactly on the window)

    ω₀² − a = s(2 + t − 2s),      ω₁² − b = s[(2 + t) − s(1 + t)]/t.

For s > 0, the second is positive if and only if s < (t + 2)/(t + 1). The first is positive if and only if
s < (t + 2)/2. On the registered window, t > 1 and 1 < s < (t + 2)/(t + 1). Since t > 1 gives
(t + 2)/(t + 1) < (t + 2)/2, both conditions hold there.

So on the whole registered window, κ := min(ω₀² − a, ω₁² − b) > 0. For t ≤ 1 the first condition would have to be
checked separately; no registered point has t ≤ 1.

**Proof.** Direct algebra:
- ω₀² − a = t − (1 − s)(t − 2s) = 2s + st − 2s².
- ω₁² − b = [1 − (1 − s)² + ts(1 − s)]/t = [s(2 − s) + ts(1 − s)]/t.

At the controls (s = 1.15, t = 3): κ = min(3.105, 0.15333) = 0.15333. At the extreme registered grid points,
κ = 0.01653 (s = 1.24, t = 3) and 0.02950 (s = 1.18, t = 4). These are small but positive. ∎

## Lemma A3 (coercivity)

For every q ∈ ℝ^{3×3},

    V(q) ≥ (κ/2)|q|² − S⁴/(2g²).

**Proof.** Let σ₁ ≥ σ₂ ≥ σ₃ ≥ 0 be the singular values of q. Then:
- |q|² = σ₁² + σ₂² + σ₃²;
- |det q| = σ₁σ₂σ₃;
- |cof q|² = σ₁²σ₂² + σ₁²σ₃² + σ₂²σ₃². The cofactor matrix is det(q)·q^{−T} for invertible q, so its singular values
  are the pairwise products; by continuity this holds for all q.

By Lemma A1 and Lemma A2, ½ Σ_k (ω_k² − src_k)|q_k|² ≥ (κ/2)|q|². For the cubic term, AM–GM gives

    g S |det q| = (gσ₁σ₂)(Sσ₃) ≤ ½g²σ₁²σ₂² + ½S²σ₃².

So, dropping (g²/2)σ₂²σ₃² ≥ 0,

    V(q) ≥ (κ/2)|q|² − ½S²σ₃² + (g²/2)σ₁²σ₃² = (κ/2)|q|² + ½σ₃²(g²σ₁² − S²).

- If σ₁ ≥ S/g, the last term is nonnegative.
- If σ₁ < S/g, then σ₃ ≤ σ₁ < S/g, so the last term is at least −½S²·S²/g².

In both cases V(q) ≥ (κ/2)|q|² − S⁴/(2g²). ∎

**Consequence.** V is a polynomial, bounded below by −S⁴/(2g²), and V(q) → ∞ as |q| → ∞. The constant is large
at weak coupling (about 8.7 × 10³ at g² = 0.004), but only its existence matters here.

A numerical sanity check, which is not part of the proof, is in `scripts/check_helike_discrete_spectrum.py`.
Adversarial BFGS from 200 starts, at the controls and the extreme grid points, never brought the slack of the
inequality below 98% of S⁴/(2g²) (the minimum found was 0.9865 of it). The closed forms of Lemma A2 and the singular-value identities agree to
rounding.

## Theorem A (discrete spectrum in every sector)

On the registered window t > 1, 1 < s < (t + 2)/(t + 1), and for every g > 0, consider H = −½Δ + V on L²(ℝ⁹) and its restriction to any
sector. The restriction is self-adjoint and has compact resolvent. Its spectrum consists of eigenvalues of finite
multiplicity, accumulating only at +∞.

**Proof.**
1. **Self-adjointness.** V is a polynomial, hence in L²_loc, and V + S⁴/(2g²) ≥ 0. By Kato's theorem (Reed–Simon II,
   Theorem X.28), −½Δ + V is essentially self-adjoint on C_c^∞(ℝ⁹). Its closure is therefore the regular Cartesian
   (Friedrichs) realisation, with no boundary choice at det q = 0: the rank-two set is an ordinary hypersurface of
   ℝ⁹ and imposes nothing.
2. **Compact resolvent.** V is bounded below and V → ∞ at infinity (Lemma A3), so H has compact resolvent
   (Reed–Simon IV, Theorem XIII.67). Equivalently, by the Rellich criterion, the form domain
   {u : ∫|∇u|² + (κ/2)|q|²|u|² < ∞} embeds compactly in L².
3. **Symmetries.** Both actions commute with H.
   - Δ is invariant under orthogonal changes of the nine coordinates. Right multiplication by R ∈ SO(3), and left
     multiplication by R_θ, are orthogonal on ℝ^{3×3} with the Frobenius inner product.
   - V is invariant under both. For colour, cof(qR) = cof(q)R and det(qR) = det q. For R_θ, cof(R_θq) = R_θ cof(q)
     (since cof R = R for a rotation), det(R_θq) = det q, ω∘(R_θq) = R_θ(ω∘q) because ω₁ = ω₂, and the sources
     satisfy src₁ = src₂.
4. **Restriction.** The sector projection is a spectral projection of these commuting unitary actions, so it commutes
   with the resolvent of H. A compact resolvent restricted to a reducing subspace is compact. The restricted
   operator therefore has compact resolvent: discrete spectrum, finite multiplicities, accumulation only at +∞. ∎

## What this settles for the registered arguments

- **Every Weinstein window is an eigenvalue.** Each spectral point found by a Weinstein window is an eigenvalue of
  the sector, not essential spectrum. Two disjoint windows give two distinct eigenvalues (Lemma 3 of the
  registration).
- **The Temple and min–max premises hold.** "The largest eigenvalue below ρ" exists whenever any eigenvalue lies
  below ρ, and the ordered eigenvalues λ₁ ≤ λ₂ ≤ … used by min–max exist.
- **The trial states belong to the operator.** They are smooth and compactly supported inside det q > 0, extended
  by zero, so they lie in C_c^∞(ℝ⁹) ⊂ D(H) for the realisation above.
- **The ordering results now rest on S_C alone.** The three ORDERED controls are conditional on S_C only. The gap
  values remain conditional on S_0, S_H and S_C.

Because the theorem holds on the whole window and for every g > 0, it also covers every registered grid point. (A)
will not need to be revisited there.

## What this does not settle

- S_C, the counting hypothesis above the competitor. That needs a rigorous lower bound on the third charge-0
  eigenvalue, by a Lehmann–Goerisch or homotopy argument.
- Anything about the location of the switch or the grid. The grid stays held.

References: M. Reed and B. Simon, *Methods of Modern Mathematical Physics*, Vol. II (Fourier Analysis,
Self-Adjointness), Theorem X.28; Vol. IV (Analysis of Operators), Theorem XIII.67.
