# Pre-registration: the wave test (20 November 2026)

Written before any code for it exists.

## The question

Can separate places "talk" through the shared field, and what carries the conversation? My answer in advance:

- through the field, no faster than light;
- the shape of space decides which global patterns carry it.

This test makes that concrete on the Berger spheres.

## Setup, fixed now

- **Background, frozen.** The Berger field of (t, 1, 1). It is not at rest, so this is a probe of the landscape,
  like the frozen-background flow test. It is **not** the true dynamics: the true Yang–Mills flow takes the Berger
  field to a flat field.
- **Equation.** The linearised, gauge-fixed wave equation ä = −(H + GG*) a. Its principal part is the rough
  Laplacian of the Weitzenböck form, so signals travel at speed at most 1 in the metric g.
- **The tap.** At τ = 0, a = 0 and ȧ is a heat-smoothed delta at the identity e. The smoothing is
  ȧ = e^{−σΔ}δ_e ⊗ (θ̂¹ ⊗ τ₁), with Δ = −Σ ρ_i²/g_i the scalar Laplacian of the Berger metric and σ = 0.02. The
  component θ̂¹ ⊗ τ₁ overlaps the twist's internal frame.
- **Peter–Weyl synthesis.** a(g, τ) = Σ_j d_j tr(F_j(τ) D^j(g)), with:
  - F_j(τ) = Φ_j(τ)(e^{−σΔ_j} ⊗ e_c);
  - Φ_j(τ) = sin(√L τ)/√L on the spectrum of L = H + GG* in sector j (sinh for negative eigenvalues, τ for zero);
  - D^j(exp X) = exp(ρ(X)).
  - Sectors j ≤ 40. The heat factor at j = 40 is below 10⁻¹².
- **Listening points.**
  - Transverse: q = exp(θ e₁) with θ ∈ {0.5, 1.0, 1.5}. There the distance is exactly θ (the Berger metric
    dominates the round one, and the arc length is θ).
  - Fibre: q = exp(θ e₀) with θ ∈ {0.5, 1.0}. There θ ≤ d(q) ≤ √t θ.
- **Stretches.** t ∈ {3.0, 4.5, 6.0, 8.0}. **Time window.** τ ∈ [0, 30].

## What is automatic (code checks, not evidence)

- **W1, causality.** Finite propagation speed is a theorem for this hyperbolic system. Required: at every
  listening point, |a(q, τ)| ≤ 10⁻⁶ · max_{τ' ≤ 30} |a(q, τ')| for τ ≤ d_low(q) − 0.5. Here d_low is the round
  distance θ, a lower bound for the Berger distance, and the 0.5 covers the smoothing width √(2σ) = 0.2 with room. A
  failure is a code or truncation error: stop.

## What follows from the floor law (consistency predictions; they can fail through the code or through a
competitor the theorem does not exclude)

- **W2, growth rate.** At every listening point, the late-time log-slope of |a(q, τ)| over τ ∈ [20, 30] equals
  √(−λ_min), with λ_min = min_j λ_t(j) over j ≤ 40, to within 2%. The values are 2.160, 3.073, 3.742 and 4.472 at
  t = 3, 4.5, 6 and 8.
- **W3, the carrier is the twist.** At τ = 30, at least 99% of the field's L² norm, Σ_j d_j ‖F_j‖²/d_j², lies in the
  twist pairs (the floor eigenspaces λ_t(j)) of all sectors together.
  - This is not automatic. A non-twist mode in some sector within about 1/30 of λ_min in decay rate would spoil it.
    The theorem bounds such modes above the floor of their own sector, not above λ_min.
- **Reported, no gate:**
  - the measured arrival time at each point (first τ where |a| exceeds 10⁻³ of its eventual value at τ = 3);
  - the time after which the twist share at the point exceeds 90%;
  - which sectors carry the late field, expected near j* ≈ t − 4 (0 for t < 4; tied pairs or triples of
    neighbouring j at t = 4.5, 6, 8).

## What this test cannot show

- Anything about the true time evolution, quantum effects, or signalling faster than light. W1 rules the last out
  by construction.
- Anything about the resonance near t_j ("louder near the crossing"). On [2.75, 10] the j = 0 floor is always
  negative, so exponential growth hides the 1/ω enhancement of a single soft sector. That needs a background with
  no growing direction, which the Berger field is not.

Script: `scripts/ricci_su2_wave_test.py` (to be written after this commit). Receipt:
`research/su2-floor-family-20261119/wave_test.json`.
