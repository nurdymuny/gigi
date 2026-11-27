# H8 charge tail: the horizontal barrier holds, and the zero-point floor sharpens it

**Status:**
- **Proved:** Step 2 of [the rest of the route](h8-rest-of-the-route-2026-11-27.md), the horizontal barrier, on
  every rank stratum and both determinant sheets, for every t > 0 and every g. Also proved is a stronger
  charge floor that uses the zero-point energy.
- **Not proved:** no eigenvalue is computed or enclosed. Step 3, the certified upper bound on E_{2,0}, is still
  needed before any charge is actually removed. H8 remains open.

Contract: [h8-charge-tail-contract-2026-11-27.md](h8-charge-tail-contract-2026-11-27.md), frozen before this
note.

## 1. Object

The operator is H_g = −½Δ + V₀ on colour-invariant L²(ℝ^{3×3}), from [the H8 note](closed-set-20261127/H8-s1-carrier.md) §1:
- V₀ = ½|∇W|²;
- W = ½Σ_k ω_k‖X_k‖² − g𝒞;
- ω = (√t, u, u), with u = t^(−1/2) and S = √t + 2u.

I work with a general compact simple colour algebra 𝔤 with invariant metric ⟨·,·⟩ and dimension n. Rows are
X_k ∈ 𝔤, and 𝒞 = ⟨X₁,[X₂,X₃]⟩. For SU(2) with the cross product, 𝒞 = det q and n = 3; that is H8.

**Gradient.** By invariance, ∇W has rows:
- ω₁X₁ − g[X₂,X₃];
- ω₂X₂ − g[X₃,X₁];
- ω₃X₃ − g[X₁,X₂].

**Circle generator.** X(q) = (0, −X₃, X₂). It is a symmetry because ω₂ = ω₃ and
[cX₂ + sX₃, −sX₂ + cX₃] = [X₂,X₃].

**Colour-orbit tangent.** Γ_Y(q) = ([Y,X₁], [Y,X₂], [Y,X₃]) for Y ∈ 𝔤.

**Horizontal norm.** The projection definition of the charge-tail note is equivalent to

  |X_hor(q)|² = min_{Y∈𝔤} ‖X(q) − Γ_Y(q)‖².

This is defined at every q, with no rank condition and no Gram eigenvalues.

## 2. The horizontal barrier

**Theorem T1.** For every q ∈ 𝔤³, every t > 0 and every real g:

  |∇W|² ≥ |X_hor|²/t.

Equality holds at q = (0, s e, 0), for any unit e ∈ 𝔤, any s and any g.

*Proof.* Rotate the colour frame about X₁ by the amount the cubic asks for: take Y* = −g√t X₁. Using
[X₁,X₁] = 0 and antisymmetry, the rows of X − Γ_{Y*} are:
- 0;
- −(X₃ − g√t[X₁,X₂]);
- X₂ − g√t[X₃,X₁].

Since g√t = g/u, comparing with the gradient rows gives the exact identity

  t|∇W|² = ‖X − Γ_{Y*}‖² + t²‖X₁ − (g/√t)[X₂,X₃]‖².

Hence |X_hor|² ≤ ‖X − Γ_{Y*}‖² ≤ t|∇W|².

At q = (0, s e, 0), all three brackets vanish and |∇W|² = u²s². The circle vector is (0, 0, s e). Every
colour tangent is (0, s[Y,e], 0), which is orthogonal to it, so |X_hor|² = s². ∎

At t = 3 and SU(2), this is inequality (1) of the charge-tail note,
‖diag(√3, 1/√3, 1/√3)q − g·cof q‖ ≥ |X_hor|/√3. It holds **on every rank stratum and both determinant sheets**,
because the proof never divides by λ_i + λ_j, never uses det q ≠ 0, and never chooses a sheet. That closes
obligations 1 and 2 of that note.

**What the identity is.** The identity is a one-line completion of squares, not an estimate found by search.
The colour rotation about X₁ by g√t X₁ cancels the cubic's contribution in rows 2 and 3 exactly. The only
thing discarded is the row-1 residual t²‖X₁ − (g/√t)[X₂,X₃]‖².

**Corollary (sharp ruler).** For every g,

  b_m(g) = inf_q [V₀ + m²/(2|X_hor|²)] = |m|/√t,

and the infimum is attained at s² = |m|√t on the equality ray. The lower bound is T1 together with AM–GM. This
explains why the multi-start minima in the charge-tail note sat on m/√3 at both couplings: the ruler is exactly
|m|/√3. It also means **the pointwise ruler cannot be improved**. Any better cut has to use information that is
not pointwise in q. That is §3.

The Gram formula |X_hor|² = ½Σ|Ġ_ij|²/(λ_i + λ_j) is not needed here. I checked it against the direct projection
at four rational full-rank points, using its Lyapunov form ½tr(ĠY) with GY + YG = Ġ. The ratio is exactly 1 at
all four. At rank ≤ 1 that form is singular, while the projection definition used above is not.

## 3. A stronger floor from the zero-point energy

**Theorem T2.** Let σ = nS/2, which is 3S/2 for H8. For every m ≠ 0,

  inf spec(H_g|charge m) ≥ F_m,

with F_m = σ + m²/(4tσ) when |m| ≤ 2√t σ, and F_m = |m|/√t otherwise.

*Proof.* Let 𝒟 be the colour-averaged, charge-m projections of C_c^∞(ℝ^{9}). This is a form core for H_g on
charge m: H_g is the closure from C_c^∞, and both averages preserve C_c^∞ and commute with H_g. Take ψ ∈ 𝒟 with
‖ψ‖ = 1, and write:
- K = ½‖∇ψ‖² and P = ∫V₀|ψ|², so E := ⟨ψ, H_gψ⟩ = K + P;
- aψ = ∇ψ + ψ∇W, Witten's twisted gradient.

*The charge identity.* Charge m gives X·∇ψ = imψ, up to a sign that plays no role. Colour invariance gives
Γ_Y·∇ψ = 0 for every Y. With Y* from §2, (X − Γ_{Y*})·∇ψ = imψ pointwise. W is invariant under the circle and
under colour, so (X − Γ_{Y*})·∇W = 0, and therefore also (X − Γ_{Y*})·aψ = imψ.

The identity in §2 gives |X − Γ_{Y*}|² ≤ t|∇W|² = 2tV₀. So pointwise:
- |m||ψ| ≤ √(2tV₀)|∇ψ|;
- |m||ψ| ≤ √(2tV₀)|aψ|.

(T2 needs only the explicit Y*, not the minimizing projection.)

*Two inequalities.* Multiply each pointwise bound by |ψ|, integrate, and apply Cauchy–Schwarz:
- (A) m² ≤ 2t·P·2K, that is, c ≤ P(E − P);
- (B) m² ≤ 2t·P·‖aψ‖², and ½‖aψ‖² = E − σ, so c ≤ P(E − σ).

Here c = m²/(4t). The identity ½‖aψ‖² = E − σ is integration by parts on C_c^∞, using ΔW = nS, since 𝒞 is
linear in each row. It also gives E ≥ σ, which is the H8 note's Step 1.

*Minimizing.* E ≥ max(P, σ) + c/P, from (A) when P ≥ σ and from (B) when P ≤ σ. The minimum over P > 0 is:
- σ + c/σ, at P = σ, when c ≤ σ²;
- 2√c = |m|/√t otherwise, which is (A) alone.

Every ψ ∈ 𝒟 therefore has E ≥ F_m, and the infimum over the core is inf spec. ∎

**At t = 3, SU(2):** σ = 5√3/2, and F_m = **(225 + m²)/(30√3)** for |m| ≤ 15. In general the small-charge branch
is F_m = **σ + m²/(4tσ)**.

**Sanity check at g = 0.** The operator is the oscillator. The lowest charge-m singlet for even m is P^{m/2},
with P = ‖X₂‖² − ‖X₃‖² + 2i⟨X₂,X₃⟩, at energy σ + |m|u. F_m ≤ σ + |m|u throughout the small-charge range, so
the floor is consistent there. It is not sharp for small charges.

**The mechanism.** (A) is the pointwise ruler. (B) carries the zero-point energy σ, which the ruler throws away.
(B) needs W itself, not only V₀, to be invariant along X − Γ_{Y*}. That is the same structure Witten's
deformation uses.

## 4. What this does to the route

Compare F_m with the harmonic value 3S/2 + ω_C ≈ 5.1224. That number is a comparison only; Step 3 has to
replace it with a certified upper bound U on E_{2,0} at each registered coupling.

| Cut | Old route (ruler only) | With T2 | Margin over harmonic |
| --- | --- | --- | --- |
| \|m\| ≥ 9 | U < 9/√3 ≈ 5.1962 | U < 306/(30√3) ≈ 5.8890 | 0.074 → 0.767 |
| \|m\| ≥ 8 | U < 8/√3 ≈ 4.6188, below harmonic, no cut | U < 289/(30√3) ≈ 5.5618 | 0.439 |
| \|m\| ≥ 7 | U < 7/√3 ≈ 4.0415, no cut | U < 274/(30√3) ≈ 5.2731 | 0.151 |

So the finite list can shrink from 1,…,8 to 1,…,6. The |m| ≥ 9 cut no longer rests on a 0.074 margin. Both
still need **Step 3**: no charge is removed until a certified U exists.

In gap form, Δ_m = E_{0,m} − E₀ ≥ m²/(4tσ) − (E₀ − σ). The H8 note's Step 4(a) puts E₀ − σ exponentially
small, but without an explicit constant at the registered couplings, so I do not use the gap form numerically.

## 5. Scope

**Proved here:**
- T1 and its sharpness, for every t > 0, every g, every compact simple metric colour algebra, every rank
  stratum and both sheets;
- the exact value b_m(g) = |m|/√t;
- T2.

**Not proved:**
- any upper bound on E_{2,0} (Step 3);
- lower bounds for |m| ≤ 6 or 8 (Step 4);
- H8_reg, H8_∞, or the physical H8, which still needs the dressed operator including H2b;
- anything at s ≠ 1.

H8 remains open; the seven top-level hypotheses are unchanged. This is not a Yang–Mills mass-gap claim.

## 6. Verification

`scripts/check_h8_charge_tail.py` runs exact sympy algebra, with no eigenvalues and no diagonalization.

**Route Step 1 gate (SU(2), all nine coordinates):**
- ∇W = ω∘q − g·cof q;
- V₀ is invariant under exact rational rotations, both circle and colour;
- the Gram-form potential, with its determinant sign;
- ΔW = 3S;
- the Gram horizontal formula agrees with the projection.

**T1:**
- the identity, symbolic for SU(2) (9 coordinates) and SU(3) (Gell-Mann, 24 coordinates);
- the bound against exact projections at rational points;
- the equality ray, and b_m = |m|/√t.

**T2:**
- W is invariant under the circle and under arbitrary colour tangents, for SU(2) and SU(3);
- ΔW = nS;
- ½Σa*a = H − ΔW/2 on a generic function of nine variables;
- the floor's equality point;
- the t = 3 closed form for |m| = 1…15 and the cut table.

**Mechanism removals.** Each must break its claim:
- ω₂ ≠ ω₃ breaks circle invariance and the identity;
- a non-invariant trilinear cubic breaks the identity and colour invariance of W;
- the unprojected circle vector violates the bound at q_MC, where ∇W = 0 but |X|² = 2/g² and |X_hor| = 0;
- with the zero-point term removed, |m| = 7 is not cut;
- a 1% inflated constant fails on the equality ray.

**Lean** (`formal/h7-cubic-quartic/H8ChargeTail.lean`) checks five declarations with standard axioms only:
- the gauge-row identity for any bracket that is linear in its second slot and antisymmetric;
- the AM–GM ruler;
- the constrained floor in both regimes;
- the t = 3 closed form.

Four broken variants must fail to compile. The projection, the integration by parts, the form core and the
polynomial identity are written mathematics, not end-to-end Lean. See
[H8ChargeTail-README.md](../../formal/h7-cubic-quartic/H8ChargeTail-README.md).
