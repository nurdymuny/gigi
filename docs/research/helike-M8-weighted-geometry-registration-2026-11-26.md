# M8, the weighted geometry of the carrier: registration (before any code or value)

26 November 2026. **REGISTRATION of an experiment.** No code exists for it yet, and no quantity below has been computed.
It is milestone M8 of my note *One note in every room* (`helike-one-note-in-every-room-2026-11-26.md`, §4.7). It tests
hypothesis H1, carrier persistence, inside the homogeneous reduction and only there.

**Reading the current contract.** The earlier versions remain below as registration history. Amendment 3 governs
carrier matching, crossing evidence, handover and the coordinate frame; amendment 2 supplies the positive shares and
the six-level refinement budget. Earlier rules that conflict with these are superseded, not alternative tests. The
model, grid thresholds and own-block threshold are unchanged. The tests below are requirements, not passing receipts.

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

---

## Amendment 2 (26 November; before any code and before any value): a third review

A third review found two defects in amendment 1's gate. Both are real. Nothing has been coded or computed yet.

### A2.1 The fingerprint was not a probability vector

Amendment 1 treated the energy shares σ_X = D_XX/D, plus the aggregated cross term, as a vector in a simplex. It is
not one. Cross terms can be negative. A positive-definite metric on two coordinates can give normalised contributions
(5, 5, −9). The reviewer computed a half-L1 "jump" of 9.47 in an explicit example, where amendment 1 assumed a number
in [0, 1].

**Correction: a nonnegative split that is canonical.** At each node, let G = (g^{ij}) be the positive-definite kinetic
form, and let G^{1/2} be its symmetric positive square root. Then v = G^{1/2} ∂f satisfies

  ½ g^{ij} ∂ᵢf̄ ∂ⱼf = ½ Σₖ |vₖ|².

Each term ½|vₖ|² is nonnegative, and it is assigned to the block of coordinate k. The **positive shares**

  π_X = ½ ∫ Σ_{k∈X} |vₖ|² ψ₀² dμ / D

are nonnegative, and they sum to 1 (checked to 10⁻¹⁰).
- The symmetric square root needs no ordering choice, unlike a Cholesky factor.
- Where G is block-diagonal, π_X equals D_XX/D.
- The raw block matrix D_XY, negative cross terms included, is still reported. It is never used as a distribution.

The jump between two points is the total variation J = ½‖Δπ‖₁, which lies in [0, 1].

### A2.2 One bisection cannot tell a discontinuity from a narrow smooth transition

An avoided crossing can be perfectly continuous while concentrated inside one half-step. Amendment 1's single
bisection could have called it a jump, and so DISSOLVES. There is also a deeper point. For an isolated level,
perturbation theory makes the eigenvector analytic in the parameters. A genuine jump of an isolated carrier cannot
happen. A jump only happens at a level crossing, or reflects a step that has not been resolved. **So continuity
cannot be the failure criterion, and I stop using it as one.**

**Correction, in two parts.**

**(a) The decided quantity is a measurable property of the carrier.** For each channel, the decided quantity is its
**own-block share**: π_H for the lowest charge-2 state, which is Helike's channel, and π_C for the competitor. This is
the property that makes it *her* carrier: most of its excitation energy is spent in her own coordinates.
- **PERSISTS** if the own-block share is ≥ 0.5 at every point of the case.
- **HANDS OVER** if the own-block share falls below 0.5 only at points beyond a recorded switch. A recorded switch is
  the registered switch bracket, or a change of the identified level index, between that point and the previous one.
- **DISSOLVES** if the own-block share falls below 0.5 at a point where the state is still the channel's identified
  level, and no switch is recorded since the previous point. **M8 fails** for that case. The level went on, but its
  defining geometry did not.
- **OPEN** if any point is OPEN under Q0, or Q2's cross-check fails, or the grid case is unresolved, or a step stays
  unresolved under (b).

The threshold of one half is the stated amount of structural change that contradicts the prediction. Any deformation
that keeps more than half of the energy in the carrier's own coordinates is tracking.

**(b) Continuity is described, and refined until it is resolved.** For each adjacent pair with J > 0.2:
- The step is refined adaptively. Bisect it, then bisect again, recursively, every sub-step whose jump is still above
  0.2.
- The limit is six levels: a sub-step of 1/64 of the grid step.
- Midpoints are solved at the same settings (Mq = 10). They are M8 diagnostic points only, and never enter the grid.
- **Resolved** if every final sub-step has J ≤ 0.2. The width of the transition, meaning the span of the sub-steps
  that carried the change, is reported.
- **Unresolved** if any sub-step at the limit still has J > 0.2. That step is OPEN for the continuity description. If
  the own-block rule (a) needs that step, the case is OPEN. It never counts as DISSOLVES.
- An identified level index that changes inside a refined step is reported as a level crossing there. It counts as a
  recorded switch.

The continuity description sits beside the verdict. It says how the carrier changed (smoothly, through a narrow
avoided crossing of stated width, or unresolved), and it decides nothing.

### A2.3 Tests and mutants, amended

**Replaced:**
- **"A smooth rotation"** is now a family whose own-block share drifts from 0.95 to 0.6 with no level crossing. It
  must give PERSISTS, with a resolved continuity description.
- **"A planted jump"** becomes two tests:
  - A narrow avoided crossing of width 1/40 of the grid step. Refinement must resolve it, report its width, and leave
    the verdict to rule (a).
  - A true level crossing, a symmetry-protected exchange. It must be recorded as a switch, giving HANDS OVER when the
    own-block share drops beyond it.

**Added:**
- **The reviewer's counterexample.** A two-coordinate positive-definite metric whose raw normalised contributions are
  (5, 5, −9). The positive shares must be nonnegative and sum to 1, and J must lie in [0, 1].
- **Planted dissolution.** The own-block share falls below 0.5 at an identified level with no switch. It must give
  DISSOLVES.

**Added mutants, which must be caught:**
- raw D_XX/D used as the fingerprint;
- a Cholesky factor in place of the symmetric square root, which must change π on a non-diagonal G while the test
  expects invariance under reordering the coordinates;
- the refinement depth cut to one level;
- an unresolved step read as DISSOLVES;
- continuity used as the failure criterion.

**Withdrawn:** amendment 1's mutants "the bisection skipped" and "the bisection threshold loosened to 1.0", which
belonged to the withdrawn rule.

### A2.4 Predictions, restated

- At the controls, π_H ≥ 0.9 for Helike and π_C ≥ 0.9 for the competitor (replacing σ ≥ 0.9).
- On the grid, Helike PERSISTS up to the switch, with π_H staying above 0.5. Any transition at the switch is resolved
  under refinement.

---

## Amendment 3 (26 November; before any code and before any value): a fourth review

A fourth review confirmed that amendment 2's shares are nonnegative and normalised. The reviewer checked that the
(5, 5, −9) counterexample gives positive shares (½, ½). It found that the switch and handover contract was still loose,
in two places, and that the frame of the split needed stating. All three points are right.

**Tightened before implementation.** A confident endpoint match is not yet a measured path. I now require the
matching to survive subdivision before it can supply evidence for a crossing. This changes neither the Hamiltonian
nor the six-level refinement limit. It closes a way for the reading method to invent the event it is meant to find.

### A3.1 A changed selection is not a crossing

Amendment 2 counted "a change of the identified level index" as a recorded switch. But an overlap-based selector can
pick a different eigenvector while the energies stay apart. The reviewer's counterexample: two levels stay exactly at
1 and 2 while their eigenvectors rotate. Their overlaps with a fixed reference trade places, so the selected index
changes, and nothing crosses.

**Correction.** Two things are kept apart.
- **selection_changed.** The grid's overlap selector picks a different level index than at the previous point. It is
  reported, and it is never a switch.
- **The carrier is followed through the physical Hilbert space, not by a selector.** The configuration space (the cone
  of 3×3 Gram matrices) and its measure dμ do not depend on s. Only the Hamiltonian does. So states at different
  points live in one Hilbert space, and their overlap ⟨ψ_a(s_k), ψ_b(s_{k+1})⟩ is well defined.
  - It is computed by Gauss–Hermite quadrature at the nodes of the later point's chart. The earlier state is evaluated
    there through the change of chart, using the same machinery as the domain gate's export.
  - The quadrature seam is order 10 against order 12, within 10⁻⁴.
  - For Helike's two-dimensional subspace, the overlap is the mean of the squared singular values of the 2×2 overlap
    matrix. For the competitor, it is the squared overlap.
  - The largest physical overlap proposes a match; it does not accept one. The sector is charge ±2 for Helike, and
    the Q = 0 excitations for the competitor. Branches participating in a possible crossing are matched one-to-one:
    two earlier branches cannot both acquire the same later state.
  - Every step is checked through its midpoint, even if its endpoint overlap is 1 and J is zero. Compare the direct
    endpoint match with the match composed through the midpoint. Compare eigen-subspaces, not signs, phases or the
    ordering of partners within a degenerate pair.
  - All constituent matches must be unambiguous: the largest overlap is at least 0.5, and it exceeds the second
    largest by more than 0.1. The direct and composed routes must select the same endpoint subspace. The quadrature
    seam above must pass on both routes.
  - If a route is ambiguous or the routes disagree, discard that coarse match and refine its two child steps. A
    resolved sequence of child matches replaces the discarded match; the coarse assignment never survives as
    crossing evidence. Midpoint checks consume the same six-level budget as A2.2(b), down to 1/64 of the original
    grid step. *Clarified before code:* the budget counts sample spacing. The finest sample spacing is 1/64 of the
    grid step, so the deepest step that can be accepted is 1/32 of it, checked through its midpoint at 1/64. A step
    of 1/64 cannot be accepted, because its own midpoint would fall outside the budget. There is no extra unreported refinement. If the required checks cannot resolve within that budget,
    the step is OPEN.
  - At a degeneracy, retain the whole degenerate subspace. An independently defined exact symmetry label may
    distinguish the branches; an arbitrary eigensolver basis may not. If branch identity remains ambiguous, report
    OPEN rather than manufacturing an exchange by relabelling vectors.
- **A verified crossing** here means a numerically resolved crossing of the computed Mq = 10 levels, not a theorem
  about the exact operator. Within a sector it requires all of the following:
  1. Both participating branches have passed the subdivision-consistency check above. Their labels are fixed by that
     check, never reassigned by sorting energies at each endpoint.
  2. Their signed energy separation has opposite, resolved signs on the two sides of a bracket. Each sign must clear
     both 10⁻⁴ in the solver's energy units and the sum of the two branches' reported numerical energy uncertainties.
     The method producing those uncertainties must be recorded; a missing estimate is not zero. Missing or
     unresolved numerical-error information makes the event OPEN. The truncation diagnostics are reported alongside
     this finite-model event; numerical stability is not renamed a rigorous spectral enclosure.
  3. The followed carrier changes from the sector's lowest excitation to a higher one, and the other matched branch
     becomes the lowest. The candidate-level set must resolve those identities; missing levels or ambiguous ties
     make the event OPEN.
  Coming within 10⁻⁴ alone is a reason to inspect a possible crossing, never evidence that one happened. A reversal
  produced by the discarded coarse matching is not an independent spectral observation. A resolved avoided crossing
  is not a crossing; an unresolved possible crossing is OPEN and cannot authorise HANDS OVER.
- **The inter-sector switch** is the registered switch bracket, where Δ_C − Δ_H changes sign with both channels
  identified. It is verified by the spectrum by definition. It is reported as **the gap handing over** from one channel
  to the other. It changes neither channel's own carrier, and it grants no HANDS OVER to either.

The receipt retains the sampled positions, overlap matrices, direct and composed assignments, rejected coarse
matches, signed energy differences with their uncertainties, bracket and refinement depth. This is resolution at a
declared sampling scale. No finite midpoint test rules out every arbitrarily narrow unsampled excursion, and I do not
call it a proof of continuity or of an exact degeneracy.

### A3.2 A handover is remembered

Amendment 2 asked only whether a switch happened "since the previous point". Shares of 0.8, 0.4, 0.3, with a genuine
switch between the first two points, would read HANDS OVER at 0.4, then DISSOLVES at 0.3.

**Correction: each channel is a small state machine along increasing s.**
- **It starts in HELD.** The followed carrier is the channel's identified level at the first point, and it is the
  sector's lowest level. If that starting identity is not resolved, the case is OPEN; it is not assumed.
  Apply the own-share rule at the first point too, with no preceding crossing.
- **In HELD, after all required point and step checks resolve, use this priority:**
  - At a verified crossing, the channel goes to **HANDED** (handed over), regardless of the former carrier's share.
    From then on, the channel's carrier is the sector's new lowest level, followed by physical overlap from the
    crossing on. The former carrier's shares are still reported, but they decide nothing more.
  - Otherwise, if the own-block share of the followed carrier is ≥ 0.5, the channel stays HELD.
  - Otherwise the share is < 0.5 with no verified crossing, and the channel goes to **DISSOLVED**. That is final for
    the case. A still-unresolved possible crossing gives OPEN before this rule is applied, not DISSOLVED.
- **In HANDED:** the state is kept for the rest of the case. A later fall in either carrier's own-block share cannot
  turn it into DISSOLVED. The new carrier's dominant block, and its shares, are reported at every later point.
- **OPEN** at any OPEN point or step stops the machine. The case verdict is then OPEN.
- **Case verdicts:**
  - **PERSISTS** if the channel ends HELD;
  - **HANDS OVER** if it ends HANDED;
  - **DISSOLVES** if it ever reaches DISSOLVED (**M8 fails** for that case);
  - **OPEN** if the machine stopped.

**selection_changed** points are listed beside the verdict. A selection_changed point where the followed carrier
stays HELD means the grid's selector and the physical carrier disagree there. That is a finding about the selector,
and it is reported as such.

HANDS OVER records that a resolved transfer occurred and was followed. It is not a claim that the successor keeps
the original carrier's defining geometry forever. Its later geometry is evidence I retain, not a prediction this
absorbing verdict tests. OPEN takes precedence over every case verdict if a required check remains unresolved.

### A3.3 The frame of the split

The symmetric-square-root split of A2.1 is canonical within a declared frame. It is invariant under permutations and
under orthogonal changes inside each block, but **not** under arbitrary rescaling of coordinates. The frame is
therefore fixed explicitly.
- At each point, take the solver's chart coordinates z, and the kinetic form G(0) at the background node z = 0.
- If G(0) is not block-diagonal across H, C, S, R to 10⁻⁸, relative to its norm, the point is OPEN.
- Otherwise, use the coordinates z̃ = G(0)^{-1/2} z. In these coordinates the kinetic form at the background is the
  identity. The map is block-diagonal, so the blocks are kept.
- The shares are computed in z̃. They are unique up to orthogonal changes inside each block, and they are invariant
  under those.

### A3.4 Tests and mutants, amended

**Added tests:**
- **The reviewer's constant-spectrum counterexample.** Two levels stay fixed at 1 and 2 while their eigenvectors
  rotate by 90° inside one original grid step: H(s) = R(πs/2) diag(1, 2) R(πs/2)ᵀ, 0 ≤ s ≤ 1. Start with only
  the two endpoints. Their overlap favours the wrong branch with weight 1, so the midpoint check must run even
  though the endpoint match looks certain. Refinement must recover the energy-1 branch. The selector's index changes,
  but no crossing is verified. For the verdict part of this fixture, supply the own-block share cos²(πs/2): it reaches
  zero, so the expected verdict is DISSOLVES, not HANDS OVER. This share is planted for the verdict test, not inferred
  as a Dirichlet decomposition of the 2×2 Hamiltonian. The energy-share kernel has its separate exact tests.
- **Unresolved matching.** Supply ambiguous overlaps at every permitted refinement level. The budget is exhausted
  at level six and the verdict is OPEN, even if a coarse energy ordering appears to reverse.
- **The three-point handover.** Shares of 0.8, 0.4 and 0.3, with a verified crossing between the first two points, must
  give HANDS OVER, never DISSOLVES.
- **The same shares without the crossing** must give DISSOLVES at the second point.
- **A true intra-sector crossing:** H(s) = diag(2 + s, 2 − s), −1 ≤ s ≤ 1, with fixed, independently supplied
  symmetry labels distinguishing its two branches within the sector. The symmetry resolves the central degeneracy;
  the branch energies reverse order and the lowest state changes. It must be verified as a crossing. A fixture with
  the former carrier's share still above 0.5 must also enter HANDED, checking the state-machine priority.
- **A resolved avoided crossing with a gap of 10⁻²:** H(s) = [[2 + s, ε], [ε, 2 − s]], ε = 0.005, on the original
  step −0.1 ≤ s ≤ 0.1. The energies never meet. The registered refinement must resolve the lower branch without
  verifying a crossing. Its narrower parameter interval is part of this synthetic fixture, not a changed grid step.
- **A sub-tolerance avoided crossing:** H(s) = [[2 + s, ε], [ε, 2 − s]], ε = 5·10⁻⁶, on the original step
  −1 ≤ s ≤ 1. The exact minimum separation is 10⁻⁵, below the inspection tolerance but strictly positive. With the
  registered ambiguity rule and six-level budget, tracking through its central rotation remains unresolved: OPEN,
  never a verified crossing or HANDS OVER. This is a separate fixture from the resolved avoided crossing above.
- **Unresolved energy ordering.** Matched branches appear to exchange order, but the numerical uncertainty overlaps
  zero at a bracket endpoint. The crossing status and case verdict must be OPEN, not HANDS OVER.
- **Frame invariance:** the shares are unchanged under a random block-orthogonal change of z. A non-block-diagonal G(0)
  is refused.

**Added mutants, which must be caught:**
- selection_changed counted as a switch;
- the mandatory midpoint skipped when the endpoint overlap is high;
- a rejected coarse match retained when computing the energy-order reversal;
- two earlier branches assigned to the same later state;
- closeness within 10⁻⁴ used as sufficient crossing evidence;
- an unresolved matching, degeneracy or energy sign converted into a verified crossing;
- the crossing verified by the energy condition alone;
- the crossing verified by the subspace condition alone;
- the handover state forgotten, with the old "since the previous point" rule;
- the own-share rule applied before a resolved crossing, so a share above 0.5 hides a handover;
- the inter-sector switch granting HANDS OVER;
- the frame normalisation G(0)^{-1/2} skipped.

These fixtures must first fail against the relevant removed mechanism, then pass with it restored. No M8 control
value is released as a result until those tests and mutants have receipts. This amendment registers the checks; it
does not report them as already run.

---

## Implementation notes (26 November; written during coding, before any control value was seen)

These record how two registered quantities are realised in code. Neither changes a rule, a threshold or a verdict.

**N1. Q2 compares against the §5.3 dressed overlap, computed with the Gram inverse.**
- §3's Q2 says the alignment "must equal the grid's dressed weight". Two different quantities carry similar names:
  - the grid's **dressed weight**, w/Z, which is the bare reference weight divided by the vacuum weight (amendment 5.2);
  - the **§5.3 dressed overlap**, which is the projection weight of the state on span{y_m G − ⟨G|y_m G⟩G}.
- Only the second is mathematically the same object as M8's alignment, because ⟨f, ȳ_m⟩_{ψ₀²} = ⟨Ψ, ȳ_m Ψ₀⟩. So Q2's
  cross-check is against the §5.3 dressed overlap: the solver's `dressed_overlap_weight`, exported per level by
  `helike cone-levels`, at the resolved fine order. The tolerance is still 10⁻⁶.
- The alignment is computed as the projection weight vᵀB⁻¹v, with v_m = ⟨f, ȳ_m⟩ and B the Gram matrix of the centred
  ȳ_m in the ψ₀² inner product, per unit ‖f‖². The solver uses the same construction. When the ȳ_m are orthogonal it
  equals §3's sum Σ⟨f, ȳ⟩²/(‖f‖²‖ȳ‖²). For Helike's pair, the per-level values are compared level by level, and their
  mean is reported.

**N2. Sectors come from the charge compression, not from energy clustering.**
- The trial space splits Helike's exact symmetric pair slightly. At g² = 0.004 and Mq = 8, levels 1 and 2 differ by
  2.6·10⁻⁶, which is more than an energy tolerance of 10⁻⁶. Clustering by energy would break her pair and label each
  half charge 0.
- So `helike cone-levels` exports the full charge compression Im⟨v_a, L v_b⟩ over all twelve levels. A pair is a
  mutual strongest coupling with |Im⟨v_a, L v_b⟩| > ½, and its |charge| is that coupling, rounded. A level coupled to
  no partner is charge 0. The diagonal ⟨v_a, L v_a⟩, which is zero for a real vector, is exported as a check.
- The sectors are then:
  - Helike's channel: the charge-2 pairs;
  - the competitor's channel: the charge-0 single levels above the ground state.
- **A pair's numerical splitting joins its uncertainty.** Each member's recorded uncertainty is its in-space residual
  ‖Hx − θx‖ plus half the pair's splitting. The splitting is a truncation artifact of an exact degeneracy, so a
  crossing test cannot be passed or failed through it. The method is recorded in the export
  (`uncertainty_method`), and the addition is made in `helike_m8_overlap.PhysicalFamily.sample`.

---

## Amendment 4 (26 November): the quadrature seam pair is set by the basis degree

**Disclosure first.** This amendment comes **after** a control value was seen. At g² = 0.002 and Mq = 10 the registered
seam Q0b failed:
- the shares moved by 2.5·10⁻³ (Helike) and 4.0·10⁻³ (the competitor) between orders 10 and 12, against 10⁻³;
- the point was OPEN, as registered;
- the order-12 shares were seen: 0.9973 for Helike and 0.9993 for the competitor.

The rule below is fixed by an argument that depends on no value. It is applied to every point alike, and the seen
point is rerun under it. Bee decided to register it.

**Why the registered pair fails at Mq = 10.** Gauss–Hermite quadrature of order n integrates a polynomial of degree up
to 2n − 1 exactly, along each axis. A trial state's polynomial part p has degree at most Mq along each axis, so p² has
degree up to 2Mq.
- At Mq = 8, order 10 is exact to degree 19 ≥ 16, and order 12 to degree 23.
- At Mq = 10, order 10 is exact only to degree 19, which is below the 20 needed. So order 10 cannot integrate even the
  norm of the basis exactly, and the seam measures the coarse order's failure, not the reading's sensitivity.

The registered pair was sized for Mq ≤ 8.

**The rule.**
- The coarse order n_c is the smallest even integer with n_c ≥ max(10, Mq + 1). This is the least order exact for
  degree 2Mq, and never below the registered 10.
- The fine order is n_f = n_c + 2.
- This gives:
  - (10, 12) at Mq = 6 and Mq = 8, unchanged;
  - (12, 14) at Mq = 10.
- The rule applies to every quadrature with a seam in M8:
  - Q0b, the shares;
  - Q0a and Q2, which are evaluated at the fine order;
  - the physical overlaps of A3.1, whose seam tolerance stays 10⁻⁴.
- Degree exactness is only a floor. The integrands that decide anything (the ratio f = p/p₀, the metric, and the
  Laurent potential) are not polynomials. So the seam still decides, with its tolerance unchanged at 10⁻³.
- **One pair, no ladder.** If the seam fails at the pair this rule gives, the point is OPEN. Orders are not raised
  further until a seam happens to pass, because that would be shopping for a resolution.

**Predictions, stated now.** At g² = 0.002 and Mq = 10, the (12, 14) seam is ≤ 10⁻³ for both channels, and the
order-14 shares agree with the seen order-12 values to that tolerance. At g² = 0.004 and Mq = 10, the same holds. If
either seam fails at (12, 14), that point is OPEN.

**What is rerun.** Both Mq = 10 controls, under the rule. The Mq = 8 controls already used (10, 12), which the rule
leaves unchanged, so they stand.
