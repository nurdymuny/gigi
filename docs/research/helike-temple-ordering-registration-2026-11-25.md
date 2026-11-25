# Helike below the competitor: Rayleigh-quotient lemma, separation hypothesis, Kato–Temple rule

25 November 2026. **REGISTRATION, before any Rayleigh-quotient enclosure or Temple bound is computed.** Inputs: the
saved control states (e21087a1), the interval residual and charge enclosures and the Arb tail and leakage endpoints
(the corrected sector receipt, feat/helike-lab 34b8860), and the passed
[relative leakage gate](helike-leakage-gate-results-2026-11-25.md). The model, states, cutoff and all earlier
thresholds are unchanged. The full grid stays held.

## 0. The question in plain English

At each control, is Helike's level (the lowest state of charge ±2) below the competitor's (the second state of
charge 0)? That is the switch question of the flux-background registration, asked at the controls first.

The Helike side can be bounded from above with no hypothesis. The competitor side needs a lower bound, and every
lower bound needs some knowledge of what lies above. Here that knowledge enters as **one explicit counting
hypothesis**, frozen below. A positive answer is reported as "ordered, conditional on S_C", never as unconditional.

## 1. Notation and standing condition

- **Sectors.** H is the physical (Cartesian, colour-singlet, two-sheet) operator restricted to an exact SO(2) charge
  sector m; P = P_m.
- **Trial vectors.** ψ is a saved trial state, normalised (its norm is enclosed), η the registered cutoff, and E
  the saved centre (an exact dyadic). φ̃ = Pηψ and φ = φ̃/‖φ̃‖. φ is smooth and compactly supported in the interior
  of the positive sheet, so it lies in D(H).
- **Rigorous upper bounds**, all already enclosed:
  - r ≥ ‖(H − E)ψ‖;
  - ℓ ≥ ‖[H, η]ψ‖;
  - t ≥ ‖(1 − η)ψ‖;
  - c ≥ ‖(L − m)ψ‖;
  - q ≤ ‖φ̃‖, with q > 0;
  - R = (r + ℓ)/q ≥ ‖(H − E)φ‖.
- **The new quantity to enclose:** s = ⟨ψ, (H − E)ψ⟩, the linear moment.

**Condition (A), discrete spectrum.** In each sector, H has compact resolvent. Sketch: the sourced potential tends
to infinity in every direction at the controls.
- The quadratic part ½Σ(ω_k² − src_k)|q_k|² is positive definite (ω₀² − a = 3.105 and ω₁² − b = 0.153 at s = 1.15,
  t = 3).
- The quartic (g²/2)|cof q|² dominates the cubic −g(Σω)det q wherever cof q is not small. Where it is small, det q
  is small too.

This sketch must be written out as a short lemma before any result citing (A) is called proved. Until then, every
result below is "conditional on (A) and S_C".

## 2. Lemma 1 (the Rayleigh quotient of the trimmed, projected state)

Let A = H − E and μ = ⟨φ, Aφ⟩, so that the Rayleigh quotient of φ is E + μ. Then

    μ ∈ (s ± e)/[q², 1],   e = 2tr + ℓ + c(r + ℓ),

meaning μ lies in the hull of (s − e)/x and (s + e)/x for x ∈ [q², 1].

**Proof.**
- P commutes with A and is an orthogonal projector, so ⟨φ̃, Aφ̃⟩ = ⟨ηψ, PAηψ⟩ = ⟨ηψ, Aηψ⟩ − ⟨(I−P)ηψ, (I−P)Aηψ⟩.
- **The projection term.** η is invariant, so it commutes with L, and ‖(I−P)ηψ‖ ≤ ‖(L−m)ηψ‖ = ‖η(L−m)ψ‖ ≤ c (the
  charges are integers). Also ‖(I−P)Aηψ‖ ≤ ‖Aηψ‖ ≤ r + ℓ. So that term is at most c(r + ℓ).
- **The commutator term.** Pointwise, A(ηψ) = ηAψ + [H,η]ψ, so ⟨ηψ, Aηψ⟩ = ⟨η²ψ, Aψ⟩ + ⟨ηψ, [H,η]ψ⟩, and the
  second piece is at most ℓ.
- **The cutoff term.** ⟨η²ψ, Aψ⟩ = s − ⟨(1−η²)ψ, Aψ⟩. Since 0 ≤ 1 − η² ≤ 2(1 − η), the last piece is at most 2tr.
- **Normalisation.** Divide by ‖φ̃‖² ∈ [q², 1]. ∎

**Corollary.** σ² := ‖(H − (E+μ))φ‖² = ‖Aφ‖² − μ² ≤ R² − μ² ≤ R².

## 3. Lemma 2 (Kato–Temple for the largest eigenvalue below ρ)

Let ρ be real and λ* the largest eigenvalue of the sector below ρ. Under (A) it exists if any eigenvalue lies
below ρ. If E + μ < ρ, then

    λ* ≥ (E + μ) − σ²/(ρ − E − μ) ≥ E + μ_lo − R²/(ρ − E − μ_hi).

**Proof.** No spectrum lies in (λ*, ρ), so (H − λ*)(H − ρ) ≥ 0 on the sector. Write η = E + μ. Then
0 ≤ ⟨φ, (H−λ*)(H−ρ)φ⟩ = σ² + (η − λ*)(η − ρ). Divide by ρ − η > 0. The second inequality is monotone in μ over its
enclosure, and uses σ² ≤ R². ∎

If the enclosure cannot establish ρ − E − μ_hi > 0, refuse.

## 4. Lemma 3 (two charge-0 eigenvalues exist below ρ_C, unconditionally)

Weinstein puts spectrum in [E₀ − R₀, E₀ + R₀] and in [E_C − R_C, E_C + R_C], both in charge 0. If these windows are
disjoint and E_C + R_C < ρ_C, charge 0 has at least two distinct eigenvalues below ρ_C (under (A)). This is checked
in the evaluation from the enclosed numbers. It is not assumed.

## 5. The separation hypothesis (frozen)

**S_C.** *In charge 0, at most two eigenvalues (with multiplicity) lie below*

    ρ_C := E₀ᶜ + (ω_H⁺ + ω_H⁻) − ω_min/4,

where:
- E₀ᶜ is the saved ground centre of the same control;
- ω_H⁺, ω_H⁻ are the two charge ±2 normal frequencies, and ω_min the smallest normal frequency, all taken from the
  saved state's frequency vector w (binary64, exact);
- ω_H⁺ + ω_H⁻ is the next harmonic charge-0 level above the competitor. The charge-0 ladder is
  0, ω_C, 2ω_H, 2ω_C, … = 0, 1.1067, 2.1802, 2.2134, … at the controls.

This is a hypothesis with explicit numbers. It is not derived from the spec's S1, which is phrased with the exact
ground energy. Substituting a Ritz value there would be invalid, and this registration does not do that.

**Evidence, not proof.** At every control the Ritz levels show a single cluster between the competitor and about
2.17 above the ground. The first level above the competitor sits at ≥ 2.1668 (relative to E₀ᶜ), compared with
ρ_C − E₀ᶜ = 1.9077. A proof would replace S_C by a Lehmann–Goerisch or homotopy lower bound for the third charge-0
eigenvalue; that is out of scope here.

With Lemmas 2 and 3 and S_C, λ* for ρ_C is exactly the second charge-0 eigenvalue, the competitor.

**For gap values only (reported, not decisive):**
- **S_0.** At most one charge-0 eigenvalue below ρ₀ := E₀ᶜ + ω_C − ω_min/4.
- **S_H.** At most one charge-+2 (and, separately, charge −2) eigenvalue below ρ_H := E₀ᶜ + ω_H + ω_C − ω_min/4.

The ordering decision uses S_C only.

## 6. Decision rule, per control

- Upper bound on Helike, unconditional by one-dimensional min–max in charge +2: U_H := E_H + μ_H,hi ≥ λ₁⁽⁺²⁾. The same
  holds in charge −2 for the partner.
- Lower bound on the competitor, Lemma 2 with ρ_C: L_C := E_C + μ_C,lo − R_C²/(ρ_C − E_C − μ_C,hi).
- **ORDERED** (Helike strictly below the competitor, conditional on (A) and S_C) if Lemma 3's windows check passes
  and U_H < L_C.
- **UNRESOLVED** otherwise. The reverse order is never claimed by this rule.

The ordering of absolute levels is the ordering of the gaps, since both gaps are measured from the same ground
energy. E₀ cancels.

## 7. Implementation contract (helike-lab, before evaluation)

1. **The linear moment s.** An interval enclosure of ⟨ψ, (H−E)ψ⟩ by the same centred moment machinery, linear
   rather than squared, for the same hulled trial vector.
   - Expected size: about 1e-10 or smaller (E is the Ritz value or the doublet midpoint).
   - The enclosure, not the expectation, is used.
2. **RED tests first**, then the code:
   - a planted state with known s;
   - Lemma 1's bound on a manufactured example where each of its three terms is active;
   - refusal when ρ − E − μ_hi ≤ 0;
   - refusal when S_C is not supplied;
   - Lemma 3's window check.
3. **Mutants**, each caught by an assertion:
   - drop 2tr;
   - drop ℓ;
   - drop c(r + ℓ);
   - use E in place of E + μ;
   - use μ_lo in the Temple denominator;
   - omit the q² division;
   - skip Lemma 3;
   - claim ORDERED without S_C;
   - claim the reverse order.
4. Commit and push the code and tests before evaluating any control. Outward-rounded interval arithmetic
   throughout; status `INTERVAL_ENCLOSURE` for the bounds, and a separate verdict field per control.

## 8. Predictions (stated before evaluation)

These are computed from the existing receipts with s set to 0, the only input not yet enclosed. They are the
expectation, not the result.

| g² | Mq | E_C − E_H | Temple shift R_C²/(ρ_C − E_C) | predicted margin L_C − U_H | prediction |
| --- | --- | --- | --- | --- | --- |
| 0.004 | 6 | 0.00804 | 7.9e-3 | −0.0011 | UNRESOLVED |
| 0.004 | 8 | 0.00761 | 3.9e-4 | +0.0072 | ORDERED |
| 0.002 | 6 | 0.01220 | 1.9e-3 | +0.0100 | ORDERED |
| 0.002 | 8 | 0.01210 | 3.8e-5 | +0.0121 | ORDERED |

The Mq = 6 failure at g² = 0.004 is the expected cost of a residual 0.080 against a separation of 0.008. It is
recorded now so a later pass at that point cannot be read as a surprise, and a failure there as a defect.

## 9. What a result here does and does not establish

**It establishes**, if ORDERED, conditional on (A) and S_C: at that control, the exact charge-±2 ground level of the
physical operator lies strictly below its exact second charge-0 level. In words, Helike's excitation is below the
competitor's at that weak coupling.

**It does not establish:**
- anything at the registered grid couplings (g² = 0.03–0.07);
- the location of the switch;
- S_C itself;
- (A) as a written proof;
- any gap value to registered accuracy;
- a continuum statement.
