# H8 charge tail: frozen contract

**Status: frozen before the proof note and the Lean file were written. Step 2 of
[the rest of the route](h8-rest-of-the-route-2026-11-27.md).**

## Object

The H8 operator as frozen in [the H8 note](closed-set-20261127/H8-s1-carrier.md) §1:
- **Operator:** H_g = −½Δ + V₀ on colour-invariant L²(ℝ^{3×3}), natural realization from C_c^∞, where
  - V₀ = ½|∇W|²;
  - W = ½Σ_k ω_k‖X_k‖² − g det q;
  - ω = (√t, u, u), with u = t^(−1/2) and S = √t + 2u.
- **Charge:** the circle rotates rows 2 and 3, and charge m is its weight.
- **Gap convention:** energies are absolute. H8 compares E_{2,0} with E_{0,m} for m ≠ 0; the common E₀ cancels.

The statements below are made for every compact simple colour algebra with an invariant metric. Rows are
X_k ∈ 𝔤 and 𝒞 = ⟨X₁,[X₂,X₃]⟩. They are also made for every t > 0 and every real g. SU(2) with the cross
product is the H8 case, with n = dim 𝔤 = 3.

**Horizontal norm.** |X_hor(q)|² is the squared distance from the circle generator (0, −X₃, X₂) to the
colour-orbit tangent {([Y, X_k])_k : Y ∈ 𝔤}. It is defined at every q, with no rank condition.

## Claims

**T1 (horizontal barrier).** For every q:
- |∇W(q)|² ≥ |X_hor(q)|²/t;
- equality holds at q = (0, s e, 0) for every g.

At t = 3 this is the inequality (1) of the charge-tail note, on every rank stratum and both determinant
sheets. Consequently, b_m(g) = inf_q [V₀ + m²/(2|X_hor|²)] = |m|/√t exactly, for every g.

**T2 (zero-point charge floor).** Let σ = nS/2. For every m ≠ 0, inf spec H_g restricted to charge m is at
least F_m:
- F_m = σ + m²/(4tσ) when |m| ≤ 2√t σ;
- F_m = |m|/√t otherwise.

At t = 3 and SU(2), F_m = (225 + m²)/(30√3) for |m| ≤ 15.

**Corollary (charge cut).** If U is a certified upper bound on E_{2,0} at some coupling, every charge with
F_m > U is excluded at that coupling. At t = 3:

| Charges excluded | Condition on U |
| --- | --- |
| \|m\| ≥ 9 | U < 306/(30√3) ≈ 5.8890 |
| \|m\| ≥ 8 | U < 289/(30√3) ≈ 5.5618 |
| \|m\| ≥ 7 | U < 274/(30√3) ≈ 5.2731 |

The harmonic value 3S/2 + ω_C ≈ 5.1224 is a comparison number, not a certified U.

## Not claimed

- Any upper bound on E_{2,0}, which is route Step 3.
- Any lower bound for the charges that remain below the cut, which is Step 4.
- H8_reg, H8_∞, or the physical H8.
- Anything at s ≠ 1, or for the dressed operator (H2b).

## Gates

1. **Step-1 object checks (SU(2), nine coordinates, exact):**
   - ∇W = ω∘q − g·cof q;
   - circle and colour invariance of V₀ under exact rational rotations;
   - the Gram-form potential with its determinant sign;
   - ΔW = 3S;
   - the charge-tail note's Gram horizontal formula, compared with the direct projection at rational
     full-rank points.
2. **T1 identity, exact and symbolic for SU(2) and SU(3).** The identity is
   t|∇W|² = ‖circle − gauge(−g√t X₁)‖² + t²‖X₁ − (g/√t)[X₂,X₃]‖². It must also be cross-checked against the
   direct projection, and against the equality configuration.
3. **T2 ingredients, exact:**
   - W is invariant under the circle and under colour, so X_hor·∇W = 0;
   - ΔW = nS for SU(2) and SU(3);
   - the ground-state identity ½Σa*a = H − ΔW/2 on a generic function;
   - the closed form of F_m.
4. **Mechanism removals.** Each must break its claim:
   - ω₂ ≠ ω₃;
   - a non-invariant trilinear cubic;
   - the unprojected circle vector, which fails at q_MC;
   - the zero-point term removed, which leaves |m| = 7 uncut;
   - an inflated constant, which fails at the equality configuration.
5. **Lean, under the pinned toolchain, with standard axioms only:**
   - the gauge-row identity;
   - the AM–GM ruler;
   - both cases of the constrained floor;
   - the t = 3 closed form.

   Broken variants must fail to compile for mathematical reasons.
6. **Regressions.** The existing origin-splitting and vacuum-floor gates still pass.
