# The wave test: a tap on the stretched sphere (20 November 2026)

**The question I asked.** Can separate places communicate through the shared field, and what carries the
conversation? The protocol was fixed before any code: [`wave-test-preregistration-2026-11-20.md`](wave-test-preregistration-2026-11-20.md),
with Amendment 1, made after the script was written and before any run.

**Receipts.**
- The gated run: `research/su2-floor-family-20261119/wave_test.json` (`scripts/ricci_su2_wave_test.py`).
- Exploratory diagnostics, not pre-registered and run after the gated test: `wave_diagnostics.json`
  (`scripts/ricci_su2_wave_diagnostics.py`).

**Status labels:**
- **Gated:** a pre-registered pass/fail.
- **Reported:** pre-registered, no gate.
- **Exploratory:** after the fact.
- Everything is double precision; nothing here is certified.

## What was done

- **Background.** The Berger field of (t, 1, 1), frozen, at t = 3, 4.5, 6 and 8. It is a probe of the landscape,
  not the true dynamics: the real Yang–Mills flow takes the Berger field to a flat field.
- **Equation.** The linearised, gauge-fixed wave equation ä = −(H + GG*) a.
- **The tap.** At τ = 0 the field is zero and its velocity is a heat-smoothed delta at the identity e (σ = 0.01), in
  the component θ̂¹ ⊗ τ₁, which overlaps the twist's internal frame.
- **Exact synthesis.** Sector by sector through Peter–Weyl, j ≤ 60, with each sector evolved exactly from its
  eigen-decomposition (sin, sinh, or τ for the zero modes).
- **Listening points.**
  - Transverse, at q = exp(θ e₁), where the distance is exactly θ: θ = 0.5, 1.0, 1.5, 2.0, 2.5.
  - Along the fibre, at q = exp(θ e₀), where θ ≤ d ≤ √t θ: θ = 0.5, 1.0.
- **Time.** τ from 0 to 30.

The Berger field has growing directions throughout this range: the j = 0 floor is negative for every t > 2. So a
tap does not ring forever. It grows, and the fastest-growing global patterns take over.

## Gated results

| | t = 3 | t = 4.5 | t = 6 | t = 8 |
| --- | --- | --- | --- | --- |
| **W1** causality: pre-arrival signal ≤ 10⁻⁴ of the post-arrival signal | pass (≤ 1.7 × 10⁻⁷) | pass (≤ 3.7 × 10⁻⁸) | pass (≤ 2.4 × 10⁻⁸) | pass (≤ 6.4 × 10⁻⁹) |
| **W2** pointwise late growth rate = √(−min_j λ_t(j)) ± 2% | **pass** (all 7 points within 0.05%) | **fail** (range 3.052–3.153, predicted 3.073; worst +2.6%) | **fail** (3.670–3.889, predicted 3.742; worst +3.9%) | **fail** (4.346–4.863, predicted 4.472; worst +8.7%) |
| **W3** ≥ 99% of the late L² norm in twist pairs | pass (1.000000) | pass (1.000000) | pass (1.000000) | pass (1.000000) |

**W2 failed at three of the four stretches.** That is the pre-registered outcome, and it stands.

## Reported (pre-registered, no gate)

**Which sectors carry the field at τ = 30** (share of the L² norm):

| t | carriers | expected j* ≈ t − 4 |
| --- | --- | --- |
| 3 | j = 0: 100% | 0 |
| 4.5 | j = 1: 64.9%, j = 0: 33.9%, j = 2: 1.2% | 0 and 1 (tied rates) |
| 6 | j = 2: 65.4%, j = 3: 21.8%, j = 1: 12.2% | 2 |
| 8 | j = 4: 51.9%, j = 5: 25.4%, j = 3: 19.1% | 4 |

The late field is a twist pattern centred on the sector the floor law makes softest, j ≈ t − 4. For t < 4 it is the
uniform (j = 0) twist. For t > 4 it is a non-uniform global twist that spreads over a few neighbouring sectors whose
growth rates nearly tie.

**Arrival.** Here "arrival" is the first τ where the signal exceeds 10⁻³ of its level just after the light-travel
time.

- Transverse points arrive at d − 0.46 to d − 0.20, for d = 1.0 to 2.5. The early edge is the width of the smoothed
  tap (its tail reaches 10⁻³ about 0.5 ahead of the front).
- Fibre points arrive between the bounds θ and √t θ, consistent with the fibre arc length √t θ.

Nothing arrives ahead of the light cone (W1).

## Exploratory diagnostics (not pre-registered)

**Why W2 failed: interference between neighbouring twists.** At each point I split the signal into sectors along its
dominant component at τ = 30.

- Near the tap (θ ≤ 1.0, and along the fibre), the sector contributions all have the same sign and add up. The
  point's log-slope matches the prediction to within 1%.
- Far from the tap, at transverse θ = 2.0 and 2.5, consecutive sectors' contributions alternate in sign, and each
  is many times larger than their sum. At t = 8, θ = 2.0, the fractions run
  −0.33, +5.7, −28.8, **+54.8**, −42.0, +13.4, −1.8 for j = 0, …, 6.
  - The field there is a small remainder of near-total cancellation between twists whose growth rates differ by
    less than 1%.
  - The remainder's log-slope swings both above and below the true maximum rate (4.86 and 4.35 against 4.47).
- At t = 3 the winning twist (j = 0) grows clearly faster than its neighbours (2.160 against 1.915). It dominates
  everywhere, and W2 passes everywhere.

**The global growth rate matches.** The L² norm cannot cancel, because sectors are orthogonal. Its late log-slope is
2.1602, 3.0713, 3.7317 and 4.4622, against the predicted 2.1602, 3.0732, 3.7417 and 4.4721: within 0.3%.

**Twist takeover at a point.** This is the first τ after which the twist share of the local signal stays between
90% and 110%. The "takeover" number in `wave_test.json` is not usable before arrival, where both parts are tiny.

| region | takeover |
| --- | --- |
| near the tap, and along the fibre | τ ≈ 1–8 |
| the far interference zone | τ ≈ 14–27 |

## What I take from it

- **The shared field does carry the tap everywhere, and no faster than light** (W1).
- **What it carries, once it settles, is the twist** (W3).
  - The specific global pattern is chosen by the shape of space, through the floor law: j ≈ t − 4. Stretch the
    sphere and the pattern changes.
  - That is the precise sense of "the shape of space decides which global patterns carry the conversation."
- **The far side hears a beat, not a single note** (exploratory). Where several twists are nearly tied, they arrive
  at a distant point with alternating signs and nearly cancel. The far side then hears a small remainder whose
  apparent growth rate wanders. Near the tap, and along the long circle, they add up.
- **My prediction W2 was too simple.** I named the risk in the pre-registration (tied rates at t = 4.5, 6, 8), but
  I still gated a pointwise rate, and it failed. The global rate is the quantity that matches the floor law.

## What this does not show

- **Not the true dynamics.** The background is frozen, and the Berger field is not at rest.
- **Nothing quantum,** and no faster-than-light channel. W1 checks the light cone directly.
- **Not the "louder near the crossing" resonance.** Exponential growth hides the 1/ω enhancement of a single soft
  sector. Seeing it needs a background with no growing direction. The Berger field is not one, and neither is the
  squashed meron, which has index one.

## Possible next steps

- **The resonance.** Remove the growing directions by projecting the tap onto the spectral subspace above zero,
  then watch a single sector's twist as t approaches its own t_j from below. That would need its own
  pre-registration, because the projection is a choice.
- **Real time.** Evolve the full nonlinear Yang–Mills wave equation from the Berger field: does the twist pattern
  survive the background's own motion?
