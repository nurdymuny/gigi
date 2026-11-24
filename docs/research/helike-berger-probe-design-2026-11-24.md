# Design: Helike as a probe-blade on the Berger-deformed matrix model

24 November 2026. **DESIGN ON REVIEW HOLD. Step 1, the mappings and the solver are done. Ready for registration review. Not run.** Model identities are checked
(`scripts/helike_berger_model.py`, `scripts/test_helike_berger_model.py`, 9 passing). Spectra are for the other machine
after registration. The literature check found no glueball or low-lying-spectrum study on a Berger or squashed S³
(see [the literature check](helike-literature-check-2026-11-24.md)).

## Why this experiment

The flux analysis showed that Helike is a double-edged blade. She is paid −B by the flux along her own circle and
pays μ² in helicity. On the round sphere's vacua there is no such flux, so the round quantum probe could only see
helicity. The Berger deformation supplies the missing ingredient. This experiment asks whether her flux edge shows
up in a quantum spectrum.

## The model (exact, checked)

Constant left-invariant fields on the Berger sphere (t, 1, 1), in the edge paper's conventions, satisfy
∗F_k = (cof M)_k − c_k M_k with c = (2√t, 2/√t, 2/√t). Hence V_t = ½|∇W_t|² exactly at every stretch (symmetric
criticality: left translations are still isometries). In model units (rows of q = frame, with row 0 along the long
circle; columns = colour), with ω = (√t, 1/√t, 1/√t):

  H_t = −½Δ_q + ½|ω∘q − g cof q|²
     = −½Δ_q + ½Σ_k ω_k²|q_k|²  (helicity² oscillator: ω_k are the homogeneous curl eigenvalues)
              − g(√t + 2/√t) det q  (the flux coupling: an isotropic cubic)
              + (g²/2)|cof q|²   (the isotropic quartic).

Checked: it equals the edge-paper curvature at t = 0.6, 1, 2.5 and 7, and is exactly the round model at t = 1. The
floor is H_t ≥ (3/2)(√t + 2/√t). That is 4.5 at t = 1 and at t = 4, and smallest (3√2) at t = 2.

**Scope.** One grain (left-invariant modes) only, as in the round model. On a Berger sphere the right-invariant modes
are not the fixed set of an isometry group, so a two-grain Berger model would also lose the exact superpotential.
The coupling g is the effective coupling at stretch t: the volume factor 2π²√t is absorbed into it. Two scan
conventions are therefore needed: fixed effective g, and fixed physical coupling with g² ∝ 1/√t.

## Symmetry, sectors and budget

The symmetry is colour SO(3) × SO(2) rotations about the long axis (frame rows 1, 2), together with a combined
reflection that gives m ↔ −m degeneracy. States carry long-axis charge m. Molien counts (exact, integral):

| Sector | Degree 2 content | Basis size at N = 10 / 12 / 14 |
| --- | --- | --- |
| m = 0 | G₀₀ and G₁₁ + G₂₂ | 84 / 150 / 252 |
| m = ±1 | G₀₁ ± iG₀₂ | 61 / 116 / 205 |
| m = ±2 | **G₁₁ − G₂₂ ± 2iG₁₂ (Helike's channel)** | 61 / 116 / 205 |

These are about a tenth of the 9-variable all-spin basis (2,178 at N = 14). Here G = qqᵀ is the frame Gram matrix
with colour contracted.

## The probes

- **Helike's channel:** H2^(±2) = G₁₁ − G₂₂ ± 2iG₁₂, the |m| = 2 member of the spin-2 shape.
- **The other members:** H2^(±1) = G₀₁ ± iG₀₂ and H2^(0) = G₀₀ − ½(G₁₁ + G₂₂).
- **Scalars:** T, G₀₀ and det q. G₀₀ becomes a scalar once SO(3) is broken to SO(2).

## What the structure already predicts (exact at harmonic order)

The ω_k are helicities. For t > 1 the two short rows are soft (1/√t) and the long row stiff (√t). For t < 1 it is
the other way round. The lowest colour-singlet excitations at weak coupling are pairs of soft quanta:
- **t > 1:** the pairs from rows 1 and 2 are exactly Helike's m = ±2 doublet and one m = 0 scalar (G₁₁ + G₂₂),
  degenerate at gap 2/√t. Everything with a long-row quantum is higher (1/√t + √t, then 2√t).
- **t < 1:** the lowest pair is G₀₀ (m = 0 only). Helike's channel starts at the next level.

So at weak coupling Helike's channel is among the lowest excitations exactly when the sphere is stretched (t > 1).
That is a helicity effect.

**The flux effect** is the splitting of the degenerate pair (m = ±2 against the m = 0 scalar) at order g². It comes
from the cubic (the flux coupling, at second order) and the quartic (first order).

## Step 1: done (exact perturbation theory, validated)

Code: `scripts/helike_berger_pert.py`. V₃ and V₄ are finite polynomials in ladder operators, so the second-order sums
are exact finite sums, not a spectrum run. It is validated against the verified round solver: the J = 0 two-quantum
scalar's gap coefficient is −12 from perturbation theory, and the solver slope tends to −11.98 as g → 0. It also
agrees with the new Berger sector solver at t = 2 (below).

**Results, for every t > 1 tested (1.5 to 10), in closed form:**

| State | Gap to order g² | Quartic share | Cubic (flux) share |
| --- | --- | --- | --- |
| Helike's channel, m = ±2 | 2/√t exactly (no g² term) | +(t + 2)/2 | −(t + 2)/2 |
| Pair scalar, m = 0 | 2/√t − g²(4 + 4/t) | | |
| Split Δ₂₀ = E(±2) − E(0) | **+g²(4 + 4/t)** | −t | 4 + 4/t + t |

**This is against my expectation, and I register it as found.** In the quantum vacuum the flux cubic does not favour
Helike's channel. It lowers the pair scalar, so at weak coupling the pair scalar is the lowest excitation and Helike's
channel is second. The reason is that the vacuum has no background flux along her circle. Her classical discount (−B)
needs one, and the cubic acts only through virtual fluctuations, which favour the transverse-size scalar.

**Theorem (Helike's quantum channel is protected).** Let f = Σ_a(q₁ₐ + iq₂ₐ)². Then f·e^(−W_t) is an exact formal
eigenfunction of H_t with eigenvalue floor + 2/√t, to all orders in perturbation theory.

*Proof.* Write u = q₁ + iq₂ (a colour vector), so f = u·u. Then:
- Δf = Σ_a(2 + 2i²) = 0;
- the oscillator part of ∇W·∇f gives (2/√t)f;
- the cubic part is −2g·u·(cof₁ + i·cof₂) = −2g·u·(i q₀ × u) = 0.

The same holds at t = 1 for f_n = Σ_a(n·q_a)² with any null vector n (n·n = 0), so the whole round J = 2 multiplet is
protected. ∎

Like e^(−W) itself (E₀ = 9/2 to all orders), these states are not normalisable. Every correction to the protected gap
is therefore non-perturbative. The source is the twist's null structure: τ₁ + iτ₂ has zero self-bracket. That is
what made her exact classically, and it is what protects her quantum channel. Test
`test_protected_channel_is_a_formal_eigenfunction` checks H(f e^(−W)) = (floor + 2/√t) f e^(−W) pointwise.

**What this means for the probe.** At weak coupling Helike's channel is a clean reference line, fixed by helicity
alone (2ω⊥ = 2/√t), with no perturbative correction. Anything the probe sees move it is non-perturbative. The flux
mechanism of the classical floor is not available in the vacuum. A quantum test of it needs a background with flux
along the circle, for example an excited or field-polarised sector, or a θ/Chern–Simons term. That is a new design
question, recorded and not assumed.

## Mapping to van den Heuvel's crossing (estimate; verify)

With the standard normalisation tr(T_aT_b) = −½δ_ab, our frame sphere has radius 2 (orthonormal τ under −2 tr), so
vol = 16π². Then g_model² = g_YM²/(8π²), and with f = g_YM²/(2π²) (unit radius), **f ≈ 4 g_model²**. Before use, this
must be checked against the normalisation in Balachandran–Vaidya–de Queiroz. If it holds, our one-grain crossing at
g² ≈ 1.76 sits at f ≈ 7, against their 0⁺/2⁺ crossing at f ≈ 0.3 (g² ≈ 0.075). That is a very different regime, where
a homogeneous truncation cannot claim to represent Yang–Mills. So it is not evidently the same phenomenon, and it
carries no physical claim either way.

## Curl spectrum

Our homogeneous curl eigenvalues at the trivial connection are −2√t (fibre, ×3) and −2/√t (horizontal, ×6), so
ω_k = |helicity|/2 in model units. Matching to Capoferri–Vassiliev Theorem D.1 still needs their appendix (the HTML
version stops before it). Hypothesis: their squashing parameter corresponds to √t.

## The sector solver: built and validated

Code: `scripts/helike_berger_sector.py`. Exact moments for the colour SO(3) × SO(2) ring R[G] ⊕ d·R[G] with
anisotropic widths, sectors (m, parity), and full-operator residuals through the shared `finish()`.
- **Dimensions** equal the Molien counts at N = 6, 8 and 10 (for m = 0 the two parities sum to it).
- **At t = 1** it reproduces the verified round J = 0 and J = 2 solvers exactly (10⁻¹⁴ in energy, identical
  residuals).
- **At t = 2, small g,** it reproduces Step 1: Helike's channel slope 0.0001 (zero up to O(g⁴)), pair scalar −6.085
  against −6.
- Tests: `scripts/test_helike_berger_sector.py` (8) and `test_helike_berger_model.py` (9), all passing. Mutation
  check `scripts/check_helike_berger_mutations.py`: 4 of 4 caught by assertion.

## The experiment (on the other machine, after registration)

- **Stretches:** t ∈ {0.5, 0.75, 1, 1.5, 2, 3, 4}. t = 1 is the control and must reproduce the verified round
  results.
- **Couplings:** g² ∈ {0.1, 0.25, 0.5, 1, 2} at fixed effective g, and a second scan at fixed physical coupling.
- **Method:** sector solvers for m = 0, ±1, ±2 (invariant and covariant bases for colour SO(3) × SO(2), exact moments
  with row-anisotropic widths, full-operator residuals). Alternatively, your 9-variable GPU solver with row-dependent
  widths and m-resolution. Widths are chosen per row by ground energy at a fixed cutoff. Cutoffs 10, 12, 14 and beyond
  while the sectors stay small.
- **Readouts per low level:**
  - energy and m;
  - the H2^(m) channel weights;
  - **the two-edge readout:** the excitation's expectation of each term of V (helicity² oscillator, flux cubic,
    quartic) minus the vacuum's, so each level says whether it was won on helicity or on flux.

## Predictions to register (after Step 1)

1. **(Harmonic, exact)** For t > 1 at small g, the lowest excitations are the m = ±2 doublet and the m = 0 pair scalar.
   For t < 1, the m = 0 scalar G₀₀ is alone at the bottom.
2. **(Step 1, fixed)** Δ₂₀ = +g²(4 + 4/t) + O(g⁴): the pair scalar is lowest at weak coupling for t > 1. Helike's
   channel gap is 2/√t with no perturbative correction. Its cubic share and the no-cubic counterfactual
   (split −t) are registered.
3. **(Crossing)** How the round model's scalar–tensor crossing moves with t. A direction is to be fixed from Step 1
   and the round results, after mapping van den Heuvel's crossing (hep-lat/9604017), which they read as a one-loop
   artefact, to this model.

## Controls and gates

- t = 1 reproduces the round sector results (J = 0 and J = 2) to solver precision.
- Exact m ↔ −m degeneracy.
- **A no-flux counterfactual:** the same model with the cubic coefficient set to zero (quartic kept). It must remove
  the cubic share of Δ₂₀. This attributes the mechanism; it is not a physical model.
- The two-edge readouts sum to the energy differences.
- Residual, convergence and replay gates as in the sector review, with mutation checks on the new operators.

## Before lifting the hold

1. Step 1 analytic prediction (here).
2. The mapping to van den Heuvel's crossing and to the Capoferri–Vassiliev curl spectrum (here).
3. Sector-solver build for colour SO(3) × SO(2), with tests (here; light).
4. Registration, then runs on the other machine.
