# M8, the weighted geometry of the carrier: registration (before any code or value)

26 November 2026. **REGISTRATION of an experiment.** No code exists for it yet, and no quantity below has been computed.
It is milestone M8 of my note *One note in every room* (`helike-one-note-in-every-room-2026-11-26.md`, §4.7). It tests
hypothesis H1, carrier persistence, inside the homogeneous reduction and only there.

**The question.** The grid tells me *where* the competitor's excitation becomes cheaper than Helike's (the switch).
This experiment asks *why* one shape is cheapest, and *how* the other takes over. It reads each excitation as a slow
function on the field space weighted by the vacuum, and it measures where that function spends its energy.

## 1. The geometry, exactly

The reduction's Hamiltonian is H = T + V. Its kinetic part is a Dirichlet form over the chart:

  ⟨ψ, Tψ⟩ = ½ ∫ g^{ij}(z) ∂ᵢψ ∂ⱼψ dμ(z),

with the same metric g^{ij}, the same measure dμ and the same chart (z ∈ ℝ⁶, the normal frame at the background) that
build the solver's T matrix (`helike_positive_cone.ConeRitz`, Rust `helike cone`).

Let ψ₀ > 0 be the ground state and E₀ its energy. Write an excited state as ψ = f ψ₀. For real f, with the residual
ρ₀ = (H − E₀)ψ₀,

  ⟨ψ, (H − E₀)ψ⟩ = ½ ∫ g^{ij} ∂ᵢf ∂ⱼf ψ₀² dμ + ⟨f² ψ₀, ρ₀⟩.            (★)

For the exact ground state ρ₀ = 0, and the excitation energy is the Dirichlet energy of f on the geometry weighted by
ψ₀². In the Ritz calculation ρ₀ is small but not zero. The last term is the seam, and it is measured (Q0).

For a complex state of charge ±2 (Helike), (★) holds for its two real partners separately. All quantities below are
summed over the two partners, so they do not depend on the choice of partners in the degenerate pair.

## 2. The states and the chart

- **Points.** First the four controls (s = 1.15, t = 3; g² = 0.002 and 0.004; Mq = 8 and 10). Then every converged
  point of the registered flux grid (rerun under §5.7 and §5.8), at the final rung, Mq = 10, and only after that grid
  completes.
- **States.** From Rust `cone-export` (the export the domain gate already uses): the ground state, Helike's two real
  partners (the level-eigenvector columns before the chiral mixing), and the competitor. These are the states the
  grid identified.
- **Coordinate blocks.** The chart coordinates z are the background's normal modes. They are grouped into four
  blocks, fixed by the solver's harmonic frequencies:
  - **H**, Helike's soft doublet (charge 2);
  - **C**, the competitor's mode;
  - **S**, the stiff doublet (charge 1);
  - **R**, the remaining mode.
  The assignment is the one `helike_and_competitor` makes, so it is not a new choice.
- **Quadrature.** Gauss–Hermite in each chart direction. The fine order is 12, and the coarse order 10 is the
  quadrature seam (Q0b). Gradients of the basis functions are taken exactly, through the solver's derivative
  matrices.
- **Positivity.** Every node carrying weight above 10⁻¹² of the total must have ψ₀ > 0. Otherwise the point is OPEN:
  f = ψ/ψ₀ is not defined there, and nothing is decided.

## 3. The quantities

For each excited state ψ (Helike, competitor), with f = ψ/ψ₀:

**Q0, the identity seam (gating).**
- **Q0a.** The quadrature Dirichlet energy D = ½∫g^{ij}∂ᵢf∂ⱼf ψ₀² dμ plus the residual term ⟨f²ψ₀, ρ₀⟩ must
  reproduce the Ritz excitation energy ⟨ψ,(H − E₀)ψ⟩, to a relative 10⁻⁴. If it fails, the point is OPEN. This is
  the check that the implementation reads the same geometry as the solver.
- **Q0b.** Every share below must change by at most 10⁻³ from quadrature order 10 to order 12. If not, the point is
  OPEN.
- The residual term's share of the energy is reported. If it exceeds 10⁻², the point is flagged as *ground-state
  limited*, but it is not refused.

**Q1, where the energy lives (the decided quantity).** Split the Dirichlet energy by coordinate block:

  D_{XY} = ½ ∫ Σ_{i∈X, j∈Y} g^{ij} ∂ᵢf ∂ⱼf ψ₀² dμ,   for X, Y ∈ {H, C, S, R}.

The **share** of block X is σ_X = D_{XX}/D. The cross terms D_{XY}, X ≠ Y, are reported. Their sum with the diagonal
shares is 1 by construction, and that is checked to 10⁻¹⁰. The **direction** of the state is the block with the
largest share.

**Q2, the alignment (a cross-check, not decided on).** Let y_X be the centred block coordinates. The weighted
alignment of f with its own block is

  a = Σ_{y∈X} ⟨f, y⟩²_{ψ₀²} / (‖f‖²_{ψ₀²} ‖y‖²_{ψ₀²}).

In the weighted L², ⟨f, y⟩_{ψ₀²} = ⟨ψ, y ψ₀⟩, which is the dressed overlap of §5.3. So a must equal the grid's dressed
weight to 10⁻⁶. **This is a gluing check: two routes to one number.** If it fails, the point is OPEN.

**Q3, the price of the bare shape (the "why").** The Rayleigh quotient of the block coordinate alone,

  P_X = D[y_X] / ‖y_X‖²_{ψ₀²}   (with the same residual correction as (★)),

is the excitation energy the bare shape would cost on this geometry. By the variational principle, P_X ≥ Δ_X, the
level the grid reports, up to the residual term. P_X is reported for X = H and X = C, together with:
- the **dressing discount** P_X − Δ_X;
- the **ordering at the level of shapes**: sign(P_C − P_H), set beside the grid's sign(Δ_C − Δ_H).

**Q4, the curvature (exploratory, floating, reported only).** At each node carrying weight, take the smallest
eigenvalue of the Bakry–Émery tensor Ric_g − 2 Hess log ψ₀, with the Ricci tensor of the chart metric computed by
finite differences of g. Report:
- its weighted 5% quantile;
- its minimum over the nodes that carry 99% of the weight;
- the bound κ/2 beside the computed gap.

This is not a bound and not a proof. If it is not implemented by the time the grid points run, it is reported as *not
computed*. It decides nothing.

## 4. The verdicts

**Per point** (Helike and the competitor separately):
- **ALIGNED** if σ of the state's own block is ≥ 0.5;
- **DRESSED** if the largest share is still its own block but below 0.5;
- **TURNED** if another block carries the largest share.

**Per case** (a (t, g²) row of the grid, along increasing s), for each channel:
- **PERSISTS** if its direction never changes.
- **HANDS OVER** if its direction changes only between adjacent points where the grid records a switch: either the
  registered switch bracket (the adjacent pair whose gap difference changes sign), or a change of the identified level
  index.
- **DISSOLVES** if its direction changes between adjacent points where no switch is recorded. **M8 fails** for that
  case.
- **OPEN** if any point of the case is OPEN, or the grid case is unresolved.

**The explanation**, reported for each case and decided on nothing:
- **Decided by shape**, if sign(P_C − P_H) matches sign(Δ_C − Δ_H) on both sides of the switch;
- **decided by dressing**, if the bare shapes order one way and the levels the other.

This is the answer to "why she stays cheapest, or how the other takes over".

## 5. Predictions, stated now

At the four controls (weak coupling, nearly harmonic):
- Q0a holds. The residual share is ≤ 10⁻³.
- Q2 reproduces the dressed weights the grid reports there (about 0.97 for Helike).
- Both channels are **ALIGNED**, with σ_H ≥ 0.9 for Helike and σ_C ≥ 0.9 for the competitor.
- P_X − Δ_X is small and positive, of order g².
- The shape ordering matches the level ordering.

On the grid I predict:
- Helike **PERSISTS** up to the switch.
- At or across the switch, the competitor's channel takes the lowest place without either direction turning.
- The ordering is **decided by shape** in the weak-coupling cases.

If the ordering is decided by dressing anywhere, that is the more interesting finding. It would mean the cheapest shape
is not the cheapest bare shape, and the vacuum's weight has rearranged them.

## 6. Tests and mutants (before any control value)

**Synthetic tests:**
- A harmonic oscillator with an exact Gaussian ψ₀. Here f for the first excitation is exactly its coordinate, so:
  - σ = 1 on that block;
  - a = 1;
  - P = Δ;
  - the residual term is 0.
- A shifted-and-sheared Gaussian, which tests the chart.
- A planted mixture on the harmonic oscillator, f = cos θ·ŷ_H + sin θ·ŷ_S, in unit-normalised block coordinates with
  frequencies w_H and w_S. Then σ_H = cos²θ·w_H / (cos²θ·w_H + sin²θ·w_S), the cross term is 0, and a = cos²θ.
- The per-point and per-case verdict boundaries, including OPEN propagation.

**Mutants that must be caught:**
- the weight ψ₀² dropped from the measure;
- the residual term dropped from Q0a;
- shares normalised by the trace of a different state;
- the direction taken from the alignment a instead of the share σ;
- HANDS OVER granted without a recorded switch;
- OPEN read as PERSISTS.

## 7. Scope

Everything here is about the sourced homogeneous reduction. A DISSOLVES verdict would refute H1 in this model only.
PERSISTS everywhere would support it in this model only, and would say nothing yet about the unsourced theory (§5 of
the note, the source bridge).

---

## Amendment 1 (26 November; before any code and before any value): a second review

An outside review of the note's §4.7 and of this registration found four points. All four are right. Nothing has been
coded or computed, so the design is corrected here, in the open, before anything can be seen.

### A1.1 The measure, and complex states

- **The reduced measure is not the metric's volume.** Write dμ = w dvol_g. In the chart, dμ = e^{log_scale + ℓ·z} dz,
  and dvol_g = (det g_{ij})^{1/2} dz, where g_{ij} is the inverse of the kinetic g^{ij}. So

    w(z) = e^{log_scale + ℓ·z} / (det g_{ij}(z))^{1/2},

  computed at each node from the solver's own chart (`ConeChart.log_measure`, `ConeChart.metric`).
- **Q4 uses the weighted curvature with the measure carried along:**

    Ric_g − Hess_g log(w ψ₀²),

  with Hess_g the covariant Hessian, built from the Christoffel symbols of g by finite differences. It is still floating
  and reported only.
- **The form (★) and Q0a hold for any f in the form domain.** For a complex f, one derivative takes the conjugate,
  ½ g^{ij} ∂ᵢf̄ ∂ⱼf, and normalisation uses |f|². The residual term becomes ⟨|f|²ψ₀, ρ₀⟩. The registered computation
  still uses the real partners and sums them. The complex form is used in a synthetic test, to check that the two
  agree.

### A1.2 What the curvature decides

Q4 constrains how cheap **any** excitation can be. It decides nothing about **which** excitation is cheapest. That is
decided by the comparison of levels and of state geometry (Q1–Q3).
- No verdict, and no explanation, may cite Q4 as the reason for a switch, or for its absence.
- §4 "the explanation" uses Q3 only, as registered.

### A1.3 The carrier may deform: continuity replaces direction

The per-case rule of §4 is replaced. As registered, a channel whose largest share moved from one block to another
counted as DISSOLVES. An eigenvector can rotate smoothly while its level stays isolated, and that is successful
tracking. So the decision is now continuity of the identified subspace, together with its alignment.

- **The fingerprint.** At each point, the subspace's energy-share vector σ = (σ_H, σ_C, σ_S, σ_R, σ_cross), from Q1.
  - The cross shares are aggregated, so σ sums to 1.
  - Helike's two partners are one subspace, and their shares are summed (§1).
  - The blocks are fixed by frequency identity, which deforms continuously with s, so σ can be compared across points.
- **The jump** between adjacent grid points k, k+1 is J_k = ½‖σ(s_{k+1}) − σ(s_k)‖₁, a number in [0, 1].
- **Continuity rule.**
  - If J_k ≤ 0.2, the step is *continuous*.
  - If J_k > 0.2, the pair is bisected once. The midpoint s is solved at the same settings (Mq = 10). It is an M8
    diagnostic point only, and it never enters the grid.
  - If both half-jumps are ≤ 0.75·J_k, the change is spread out, and the step is *continuous* (a fast, smooth
    rotation).
  - Otherwise it is concentrated, and the step is a *jump*.
  - A midpoint that is OPEN under Q0 makes the step OPEN.
- **Alignment.** At each point, the alignment with Helike's registered structure (Q2, the dressed weight) is reported
  and cross-checked. A resolved grid case already requires it to be ≥ 0.9 at every point (the grid's weight_min).
  - A point below 0.9 makes the case OPEN here, as in the grid.
  - It is never counted as continuity.
- **Per-case verdicts, for each channel:**
  - **PERSISTS** if every step is continuous. If its largest block changes along the way, it is reported as
    *PERSISTS, deforming*, with where and how far.
  - **HANDS OVER** if jumps occur only at steps where the grid records a switch: the registered switch bracket, or a
    change of the identified level index.
  - **DISSOLVES** if a jump occurs at a step with no recorded switch. **M8 fails** for that case.
  - **OPEN** if any point or step is OPEN, or the grid case is unresolved.
- **How much change contradicts the prediction:** a concentrated jump, J > 0.2 that does not spread under bisection, at
  a step with no switch. A smooth rotation of any size does not contradict it.
- **The per-point labels** ALIGNED, DRESSED and TURNED are kept as descriptions. They decide nothing.
- **A second fingerprint, reported only:** the subspace's projector weights on the 22 registered heat-signal
  monomials. Its jump is reported beside J_k.

### A1.4 Tests and mutants, amended

Added tests:
- **Q4 against an exact case.** A flat metric with a nonconstant density w and a Gaussian ψ₀, where the weighted
  tensor is known in closed form. Q4 must reproduce it, and must differ from it when w is dropped.
- **Complex and real forms agree.** For a planted charge-2 pair, the complex form (conjugate, |f|²) and the
  summed-real-partner form must agree to 10⁻¹².
- **A smooth rotation.** A synthetic family with σ rotating from H to S over the sweep, with no level crossing. The
  steps have J > 0.2, but the change spreads under bisection, so the verdict is PERSISTS, deforming.
- **A planted jump.** An avoided crossing with σ swapping inside one step. Without a recorded switch the verdict is
  DISSOLVES; with one at that step it is HANDS OVER.

Added mutants, which must be caught:
- w dropped from Q4;
- the old direction rule restored as decisive;
- the bisection skipped, so every J > 0.2 is read as a jump;
- the bisection threshold loosened to 1.0;
- Q4 cited in the explanation.

The mutant "the direction taken from the alignment a instead of the share σ" is replaced by "the continuity taken
from the alignment instead of the fingerprint".

### A1.5 Predictions, restated

- At the controls: unchanged.
- On the grid: Helike PERSISTS up to the switch. Small deformation is allowed, and I predict her largest block stays
  H. The two channels exchange the lowest place at the switch without a concentrated jump in either fingerprint.
