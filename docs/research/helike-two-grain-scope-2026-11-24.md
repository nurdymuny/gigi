# Two grains: scoping the left-and-right homogeneous model

24 November 2026. **SCOPE, NOT A REGISTRATION. Nothing here has been computed beyond the tests named. Statements
marked *expected* are derivation sketches to be verified in Phase 1.**

## Why

The razor's last lesson was the sweep direction. In the model the sweep is the grain of S³. There are two: the
left-invariant frame e and the right-invariant frame ē, with opposite-handed Hopf fibrations and Maurer–Cartan
equations of opposite sign. Every result so far uses nine modes on one grain:

  A = q_{ai} eⁱ T_a.

That truncation has no parity, because parity exchanges the grains. It also makes the second vacuum inequivalent to
the first: the Hessians are all 1 at the origin and −1, 0, +2 at the second vacuum, although the two are gauge
copies in the full theory. The two-grain model keeps eighteen modes:

  A = q_{ai} eⁱ T_a + p_{ai} ēⁱ T_a.

It is the direct test of whether the scalar–tensor crossing, the second-vacuum scalar and the 9/2 floor survive
with both bevels on the blade.

## What is already exact

1. **Why one grain gives V = ½|∇W|² exactly.** The magnetic energy on a three-manifold is ½|grad CS|², with the
   gradient of the Chern–Simons functional being *F. The left-invariant connections are exactly the fixed set of
   the left translations. By the principle of symmetric criticality the full gradient at a fixed point is itself
   invariant, so it lies inside the nine-mode space, and restriction loses nothing. That is the origin of the
   identity used all week, with W = CS restricted.
2. **The opposite grain alone is the same model with g reversed.** Its Maurer–Cartan sign flips the cubic term,
   giving W_R(p) = ½tr(ppᵀ) + g det p, and V(−p; g) = V(p; −g). The spectrum is identical (ψ(p) → ψ(−p)); its
   second vacuum lies on the det p < 0 orbit. Test: `SweepDirection.test_opposite_grain_is_g_reversed`.
3. **The two grains carry opposite winding.** The left Maurer–Cartan form g⁻¹dg is the pure gauge of the
   identity map. −dg g⁻¹ = (g⁻¹)⁻¹d(g⁻¹) is the pure gauge of the inverse map, of opposite winding. So the left
   grain's second vacuum and the right grain's are the ±1 winding vacua (the overall sign depends on convention).
   With the grain and against it are the two instanton directions. Parity maps one to the other.

## What two grains should change (expected; Phase 1 verifies or refutes)

The frames are related by ē = D(x) e, with D(x) the adjoint rotation of the point x ∈ S³ = SU(2). Integrating the
energy over S³ is a Haar average over SU(2), the same quaternion machinery as the dome series.

- **E1. No quadratic or cubic mixing.** Every quadratic or cubic cross term in ∫|F|² should reduce to a single
  D(x) after the ε-contractions, and the Haar average of a single adjoint D is zero. Quartic cross terms carry two
  D's and survive.
- **E2. The superpotential splits.** By the same count, the Chern–Simons cross terms vanish, so
  W₁₈ = W(q; g) + W(p; −g).
- **E3. The floor survives as an inequality.** The eighteen-mode space is not the fixed set of any symmetry, so
  symmetric criticality no longer applies:

    V₁₈ = ½|∇W₁₈|² + C(q,p),  C = ½|(1 − P)*F|² ≥ 0,

  where P projects onto the eighteen-mode space. Then H₁₈ = ½A†A + 9 + C ≥ 9 (ΔW₁₈ = 9 + 9). The protection holds
  with both bevels, but the exact factorization becomes an inequality, and the gap between them is exactly the
  part of the curvature that leaves the truncation.
- **E4. Vacua.** Zero energy needs ∇W₁₈ = 0 and C = 0. Candidates: (0,0), (R/g, 0), (0, −R/g). Whether
  (R/g, −R′/g) has C = 0 is open.
- **E5. Symmetry.** SO(4) = SU(2)_L × SU(2)_R / Z₂ acts with q carrying one factor and p the other. States carry
  (j_L, j_R). Today's J = 2 tensor is a one-sided multiplet, and in the full model it gets a mirror partner.
  Parity exchanges the two; the physical 2⁺ and 2⁻ are their sum and difference, and the splitting comes only
  from C.

## Phases

**Phase 0: literature.** Van Baal and collaborators analysed exactly this setting: the left and right modes on
S³, the sphaleron, and θ-dependence of the low-lying glueballs (for example van Baal and Hari Dass on θ-dependence
beyond steepest descent, and van den Heuvel's three-sphere glueball work). The references must be verified and
conventions matched before any comparison. Questions for the literature:
- Is the eighteen-mode space closed under the winding-one gauge transformation?
- What did they find for the 0⁺ and 2⁺ levels as the volume grows?
- Do they report a scalar–tensor crossing?

**Phase 1: the potential.** Derive V₁₈ and W₁₈ exactly, with the cross term C written in invariants, then test:
- a Monte Carlo or quadrature integral of |F|² over S³ at random (q,p) against the formula;
- p = 0 reproduces V(q; g), and q = 0 reproduces V(p; −g);
- invariance under colour and SO(4), and the parity swap;
- E1–E4 confirmed or refuted, each as its own test;
- van Baal's potential reproduced after matching conventions.

**Phase 2: geometry without spectra.** Find the vacua and their Hessians, and whether a symmetry of V₁₈ exchanges
them. Locate the sphaleron and its barrier, and repeat the local Witten count with C included. These are cheap and
decisive. If the two-grain vacua become equivalent, the weak-coupling prediction of a scalar gap → 1 changes, and
the note on it must say so.

**Phase 3: sector solvers.** The (0,0) sector consists of colour invariants of the pair (q,p) with separate
left and right rotations. Its invariants include those of q alone (T_q, P_q, det q), those of p alone, and the mixed
invariants of M = qᵀp with the Gram matrices qᵀq and pᵀp. That ring is not free, so the basis must be built degree
by degree (exact rank at random rational points), and its size counted from the Molien series. Gaussian moments
need new integration-by-parts tables. The tensor sectors (2,0) and (0,2) give the parity pair. Budget the size
before building; Phase 3 starts only after Phases 1 and 2 are reviewed.

**Phase 4: registration.** The crossing with both grains, at the controls and at held-out couplings, written only
after Phases 1–3. It compares the one-grain and two-grain results at matched cutoffs, with the cross term removed
in a control.

## What this could show, and what it cannot

If the crossing and the second-vacuum scalar survive with both grains, they are not artefacts of shaving one way.
If they move or vanish, the one-grain model was cutting too close, and that is worth knowing. Either way it remains
a finite-mode model on a fixed sphere: not the continuum, and not a mass-gap result.
