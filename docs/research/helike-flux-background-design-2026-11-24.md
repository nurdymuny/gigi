# Design: a flux background for Helike's quantum probe (the sourced condensate)

24 November 2026. **DESIGN ON REVIEW HOLD. The pre-registration items are done (below). Ready for registration review. Not run.** It follows
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

## The observable (corrected)

The earlier draft said the sources shift every transverse vibration equally, so gap differences are source-free. That
is only partly true. Helike's mode lives in the transverse rows alone, so its shift is exactly −b. The low competitor
mixes the long and transverse rows, so its shift is not uniform. The registered predictions therefore use the sourced
vibration spectrum directly, which is exact at harmonic order (below), not edge differences.

Helike's vibration is the one the probe H2^(±2) couples to linearly: around q_bg,
δH2^(±2) ∝ δq₁₁ − δq₂₂ ± i(δq₁₂ + δq₂₁), the twist component.

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

## Pre-registration results (done; light)

**Closed forms (derived; independent of g; tests in `test_helike_berger_model.py`).** At q = diag(X, Y, Y), ∇W is
diagonal, with entries P = ω₀X − gY² and Q = Y(ω⊥ − gX). On the Maurer–Cartan line:

  a = (1 − s)(t − 2s),  b = (1 − s)[(1 − s)/t − s],  s_max = (t + 2)/(t + 1),
  V_src(q_bg) = −s³(s − 1)(½ + 1/t)/g²,  ω_H² = 2s(1 + 2/t − s)  (Helike's vibration).

The boundedness condition is ω⊥² − b = s[(2 − s)/t + 1 − s] > 0; the long-row one, s(2 + t − 2s) > 0, never binds. At
s = 1 (the flat second vacuum), ω_H = 2/√t, which is Step 1's protected gap. Moving off it, the flux lowers her
frequency for every t > 2.

**No rotor tower.** Colour is gauge, and q_bg's colour orbit is a full SO(3), so colour-singlet wavefunctions are
constant on it. The low spectrum consists of the 6 gauge-orthogonal vibrations alone. Classified by the locked U(1)
(frame rotation with colour rotation about axis 0) that preserves q_bg:
- Helike's |Q| = 2 doublet;
- the low **Q = 0 competitor**, which mixes the long and transverse rows;
- a |Q| = 1 doublet and a second Q = 0 mode, both high (frequency² about 6.5–10).

**The sourced crossing, the registered harmonic prediction.** Helike becomes the lowest vibration at s*_src(t):

| t | s*_src (sourced vibrations) | s* (unsourced edge sector 0) | s_max |
| --- | --- | --- | --- |
| 3 | 1.1436 | 1.1565 | 1.2500 |
| 4 | 1.0981 | 1.1116 | 1.2000 |
| 5 | 1.0718 | 1.0849 | 1.1667 |
| 6 | 1.0551 | 1.0673 | 1.1429 |
| 8 | 1.0355 | 1.0459 | 1.1111 |

**The reachable quantum regime.** At g² ≈ 0.01 the background sits at |q| ≈ 10, and the Haar series would need K in
the hundreds. That is infeasible, the same breakdown as the weak-coupling dome. The registered regime uses larger
depth instead. c(s, t) = s³(s − 1)(½ + 1/t) is largest near s_max at small t; at t = 3, s ≈ 1.15–1.24,
c ≈ 0.25–0.38. With g² = 0.1–0.2, c/g² ≈ 1.3–4 and the background sits at |q| ≈ 2–3. The orbit envelope represents
that with moderate K, frozen by the outside-origin truncation ratio. The table above is a harmonic, leading-order
prediction. At c/g² of order 1–4 the quantum corrections are not small; that is what the experiment measures. The
O(g²) corrections are not computed yet.

**The solver: built and validated (`scripts/helike_berger_condensate.py`).** It handles a mixed-width basis (each
function with its own per-row widths, full Π γ_k^(−3/2 − deg_k/2) normalisation), the sources in H, and orbit-envelope
functions m(G, d)·F_K(Λq)·e^(−½Σβ|q|²), with Λ = diag(β₀X, β⊥Y, β⊥Y) and F_K written exactly in (G, d) via
tr XXᵀ = Σλ²G_kk, tr(XXᵀ)² = Σλ_k²λ_l²G_kl² and det X = λ₀λ⊥²d. Tests (`test_helike_berger_condensate.py`, 5
passing):
- single-width items reproduce the sector solver (to 10⁻¹⁵);
- the mixed-width overlap closed form;
- the orbit envelope equals the colour-Haar average of the shifted Gaussian (Monte Carlo);
- the sources enter exactly (single-Gaussian shift −¾(a/α₀ + 2b/α⊥));
- orbit-envelope functions never raise a Ritz value, and lower the ground level in a sourced case (t = 3, s = 1.2,
  g² = 0.2).

Mutation check (`check_helike_condensate_mutations.py`): 3 of 3 caught by assertion (moment normalisation, dome scale,
dropped sources).

## Draft registration (for your review)

- **Couplings and backgrounds:** t ∈ {3, 4}; s on a grid across s*_src(t) inside the window (t = 3: 1.10, 1.13, 1.14,
  1.15, 1.18, 1.22; t = 4: 1.06, 1.09, 1.10, 1.12, 1.16); g² ∈ {0.1, 0.15, 0.2}.
- **Basis:** origin sector basis (m-resolved) plus orbit-envelope items. β is fixed from the sourced Hessian at q_bg
  (the contact frame). K is frozen by the outside-origin ratio below 10⁻⁶ at the top cutoff. Cutoffs fixed in advance.
- **Readouts:**
  - the lowest excitation in each |Q| channel;
  - its H2^(±2) weight;
  - the ordering switch s*_q(t, g) against the harmonic s*_src(t);
  - the two-edge (helicity², flux, quartic, source) shares.
- **Primary hypothesis:** the lowest excitation becomes the H2^(±2)-dominated |Q| = 2 state for s above a switch point
  that tends to s*_src(t) as g → 0, within registered tolerances.
- **Controls:**
  - s = 1, where the background is flat (B = 0): the protected 2/√t;
  - s < 1 (B < 0);
  - sources varied at fixed background.
- **Gates:** residual, convergence and replay, as in the sector review.
- **Scope:** a controlled quantum test of the helicity² − flux mechanism around a held background. It is not a claim
  about the unsourced vacuum.
