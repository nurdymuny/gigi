# Design: a flux background for Helike's quantum probe (the sourced condensate)

24 November 2026. **DESIGN ON REVIEW HOLD. Not registered, not run.** It follows
[the Berger probe design](helike-berger-probe-design-2026-11-24.md). Verified facts are tested in
`scripts/test_helike_berger_model.py` (13 passing); only light checks were run here.

## Why a background is needed

Classically, Helike's eigenvalue is helicity² minus the flux along her circle, λ = μ² − B. The quantum vacuum of the
Berger matrix model is flat (∇W ≈ 0), so it supplies no B. Step 1 confirmed this: at weak coupling her channel's gap
is protected at 2/√t, and the flux cubic favours the pair scalar instead. To see her flux edge quantum-mechanically,
the state must live on a background with B ≠ 0 along her circle.

## The construction

**The edge family is a set of matrix-model points.** The constant field M = (xτ₀, yτ₁, yτ₂) is q_bg = diag(x, y, y)/(2g)
in model units. Verified at three family members (to 2×10⁻⁴, finite differences):

  Hess_q V(q_bg) = ¼ · H_edge(sector j = 0).

So the vibrational spectrum around a family background is the edge paper's sector-0 Hessian divided by 4, including
Helike's closed form λ = (x + 2/√t)² + 2√t·x − y².

**Holding the background: gauge-invariant sources.**

  H_src = H_t − ½a|q₀|² − ½b(|q₁|² + |q₂|²),

with (a, b) fixed so that q_bg is stationary. The sources are Lagrange multipliers for the long-row size ⟨G₀₀⟩ and the
transverse size ⟨G₁₁ + G₂₂⟩. They are colour singlets and long-axis SO(2) invariant. ∇V at q_bg is diagonal, so two
sources suffice. This is the background-field method, with the external source chosen gauge-invariant.

**Boundedness decides which backgrounds are allowed.** Yang–Mills has flat (commuting, rank-one) directions on which
the cubic and quartic both vanish. H_src is bounded below only if ω₀² − a > 0 and ω⊥² − b > 0. The Berger field and
other strong-flux members need sources that break this, so they **cannot** be held. Backgrounds near a flat connection
can: the Maurer–Cartan line A = sθ, (x, y) = (2s/√t, 2s), near the second vacuum s = 1. There:
- B = 4s(s − 1) and |κ| = 4s(s − 1)/√t, so for s > 1 and t > 1, B > |κ| > 0: Helike's flux-aligned regime;
- the window is bounded for s < s_max = 1 + 1/(t + 1) (found numerically at t = 3, 4, 5, 6, 8; derivation owed);
- inside it, q_bg is the global minimum of V_src (30 random starts), stable, with exactly 3 zero modes (the colour orbit).

## The classical prediction to test quantum-mechanically

Along the window, the flux discount overtakes the competitor, and Helike becomes the lowest vibration, the
sector-0 floor:

| t | Helike becomes the floor at s* | boundedness limit s_max = 1 + 1/(t+1) |
| --- | --- | --- |
| 3 | 1.1565 | 1.2500 |
| 4 | 1.1116 | 1.2000 |
| 5 | 1.0849 | 1.1667 |
| 6 | 1.0673 | 1.1429 |
| 8 | 1.0459 | 1.1111 |

For s < s* another mode is lower: at t = 4 its eigenvalue is 3.608 against hers 3.975, in edge units. Its long-axis
charge is still to be identified.

## The clean observable

The sources shift every transverse vibration by the same −b, and the long-row ones by −a. Within the transverse
family, gap differences are therefore source-independent and equal the edge paper's sector-0 differences divided by
4. The registered observable is the ordering and spacing of the low vibrational levels, not their absolute values.
Helike's vibration is the one the probe H2^(±2) couples to linearly: around q_bg,
δH2^(±2) ∝ δq₁₁ − δq₂₂ ± i(δq₁₂ + δq₂₁), which is exactly the twist component.

## The quantum regime

Every term of V scales as 1/g² at q_bg ∝ 1/g, so V_src(q_bg) = −c(s, t)/g². For example, c ≈ 0.10 at t = 4, s = 1.1,
from V_src(q_bg) = −0.277 at g² = 0.36. The ground state sits on the background's colour orbit when c/g² ≫ 1, which
means g² of order 0.01–0.02 near s*. The spectrum is then a rotor tower (colour orbit, spacing ∝ g²) times
vibrations with frequencies √(eigenvalues of Hess V_src). Corrections are O(g²).

## Method (to build here, light; to run on the other machine)

1. **Semiclassical predictions (here).** Exact harmonic vibrations from the edge sector-0 block, plus the rotor
   moment of inertia from q_bg. Then the first O(g²) corrections by the same exact finite perturbation sums as Step 1,
   in coordinates (R ∈ SO(3), normal vibrations).
2. **A basis that sits on the background.** At g² ≈ 0.01 the background is at |q| ≈ 10, out of reach of origin-centred
   polynomials. The right envelope is the dome's: a Haar average over the colour orbit of a Gaussian centred at
   q_bg R. Its widths come from Hess V_src at q_bg (the contact frame: Hess W's eigenspaces give the frame, and its
   eigenvalues the widths). The exact Dirichlet series for ∫dR exp(tr RᵀX) holds for any matrix argument X, so the
   anisotropic centre diag(x, y, y) is covered. The series order K must be frozen by the outside-origin truncation
   ratio from the dome review, before any comparison.
3. **Sectors and readouts.** m-resolved spectra; the gap ordering within the transverse vibrations; H2^(±2) weight in
   the lowest vibration; the two-edge readout (helicity², flux and quartic shares) per level.

## Predictions to register (after step 1)

1. **(Harmonic, exact)** Vibrational frequencies² = ¼·(edge sector-0 eigenvalues) − (a or b), and gap differences
   within the transverse family are source-free.
2. **(The flux edge)** The lowest vibrational excitation switches to Helike's channel (H2^(±2)-dominated) at s*(t) + O(g²),
   inside the bounded window.
3. **(Controls)**
   - At s = 1 (B = 0, the flat second vacuum) she is not the floor.
   - For s < 1 (B < 0) she moves further from it.
   - At t = 1 the round sourced model.
   - Changing the sources without moving the background shifts all transverse levels together, leaving the
     differences unchanged.

## What this can and cannot show

**Can show:** that the classical mechanism (helicity squared against flux along her circle) governs the ordering of
quantum vibrations around a held background, and that the probe channel follows it.

**Cannot show:** anything about the unsourced Yang–Mills vacuum. The sources are external. It is a controlled quantum
test of the mechanism, not a claim about the physical spectrum.

**Open before registration:**
- derive s_max and the sources in closed form;
- identify the competitor's charge;
- write the step-1 semiclassical predictions;
- build and test the orbit-envelope basis.
