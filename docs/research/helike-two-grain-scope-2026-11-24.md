# Two grains: scoping the left-and-right homogeneous model

24 November 2026. **SCOPE, NOT A REGISTRATION. Phase 1 done: [results](helike-two-grain-phase1-2026-11-24.md). E1, E3, E4 and E5 confirmed; E2 confirmed with opposite grain signs (W₁₈ = W_R − W_L), so H ≥ 9 + C needs per-grain sweeps. Nothing here has been computed beyond the tests named. Statements
marked *expected* are derivation sketches to be verified in Phase 1.**

## Why

The razor's last lesson was the sweep direction. The model offers two canonical Lie-group framings on S³:
the left-invariant frame e and the right-invariant frame ē, with opposite-handed Hopf families and Maurer–Cartan
equations of opposite sign. Every result so far uses nine modes on one grain:

  A = q_{ai} eⁱ T_a.

The nine-mode truncation is not closed under the parity exchanging these families. Its two local Hessians differ,
while large-gauge equivalence in the full theory requires boundary identifications not specified by this finite
Hamiltonian alone. The two-grain model keeps eighteen modes:

  A = q_{ai} eⁱ T_a + p_{ai} ēⁱ T_a.

It can test how the scalar-tensor crossing and the proposed second-vacuum scalar change when the missing family is
restored. Absolute zero-point offsets change with the number of modes; compare excitation gaps using a matched
normalization, not an unchanged 9/2 offset.

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
   The corresponding field-theory paths are candidates for opposite topological transitions. The two algebraic
   factorizations of one Hamiltonian alone do not construct those paths or prove tunnelling rates.

## What two grains should change (expected; Phase 1 verifies or refutes)

The frames are related by ē = D(x) e, with D(x) the adjoint rotation of the point x ∈ S³ = SU(2). Integrating the
energy over S³ is a Haar average over SU(2), the same quaternion machinery as the dome series.

- **E1. No quadratic or cubic mixing.** Every quadratic or cubic cross term in ∫|F|² should reduce to a single
  D(x) after the ε-contractions, and the Haar average of a single adjoint D is zero. Quartic cross terms carry two
  D's and survive.
- **E2. Distinguish the oriented Chern-Simons functional from a positive auxiliary function.** With left and right
  curl signs opposite, the proposed restriction is CS₁₈ = W(q; g) - W(p; -g), up to a common overall convention.
  Its Laplacian is 9 - 9 = 0. The earlier sum was not the oriented Chern-Simons functional. Independently define
  U₁₈ = W(q; g) + W(p; -g). The block-separated sum and difference have the same squared gradient norm, but
  different Laplacians. Phase 1 must match the frames, kinetic metric and normalization explicitly.
- **E3. A conditional floor, using the auxiliary function.** Closure of the magnetic gradient inside the sum of
  the two mode families must be tested; one-family symmetric criticality does not establish that closure:

    V₁₈ = ½|∇CS₁₈|² + C(q,p) = ½|∇U₁₈|² + C(q,p),
    C = ½|(1 - P)*F|² >= 0,

  where P is the orthogonal L² projection. If E2 and the flat orthonormal kinetic metric hold, factoring with
  U₁₈ gives H₁₈ = ½A†A + 9 + C >= 9 on a domain where integration by parts has no boundary contribution.
  This is a possible auxiliary-function bound, not a deduction from Lap CS₁₈. Large-gauge boundary conditions
  must be checked before using it for a physical quotient. The positive remainder measures omitted magnetic
  components, not an excitation gap.
- **E4. Vacua.** Zero classical potential needs ∇CS₁₈ = 0 and C = 0. Candidates: (0,0), (R/g, 0), (0, −R/g). Whether
  (R/g, −R′/g) has C = 0 is open.
- **E5. Symmetry.** SO(4) = SU(2)_L × SU(2)_R / Z₂ acts with q carrying one factor and p the other. States carry
  (j_L, j_R). Today's J = 2 tensor is a one-sided multiplet, and in the full model it gets a mirror partner.
  Parity exchanges (2,0) and (0,2). If exact SO(4) and parity are retained, these inequivalent SO(4) irreducible
  sectors are degenerate partners: an SO(4)-invariant C cannot mix them to split their sum and difference.
  Relating these labels to physical diagonal-spin glueball sectors requires the full representation and boundary
  analysis; opposite parity does not by itself predict a nonzero splitting.

## Phases

**Phase 0: literature.** Van Baal and collaborators analysed exactly this setting: the left and right modes on
S³, the sphaleron, and θ-dependence of the low-lying glueballs (for example van Baal and Hari Dass on θ-dependence
beyond steepest descent, and van den Heuvel's three-sphere glueball work). The references must be verified and
conventions matched before any comparison. Questions for the literature:
- Is the eighteen-mode space closed under the winding-one gauge transformation?
- What did they find for the 0⁺ and 2⁺ levels as the volume grows?
- Do they report a scalar–tensor crossing?

Verified starting reference: van Baal and Hari Dass, [Nuclear Physics B 385 (1992), 185-226](https://www.ilorentz.org/research/vanbaal/DECEASED/HOME/JOUR/NPB385_185.pdf).
Equation (92) gives the two-family potential; equation (101) has the Chern-Simons difference, not a sum.
The surrounding boundary analysis warns that generic large-gauge images leave the retained mode space.
These are checks to reproduce after translating conventions, not permission to transplant coefficients.

**Phase 1: the potential.** Derive V₁₈, CS₁₈ and U₁₈ exactly, with C written in invariants, then test:
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
contractions of M = qᵀp with qᵀq and pᵀp. These matrices are covariants under the separate spatial rotations;
only appropriately contracted expressions are scalar invariants. A rank found by evaluation at rational points
is a lower bound on generic rank, not a completeness proof. Use symbolic identities and independent dimension
counts, for example a verified Molien series, to establish the full basis and its relations. Gaussian moments
need new integration-by-parts tables. The tensor sectors (2,0) and (0,2) give the parity pair. Budget the size
before building; Phase 3 starts only after Phases 1 and 2 are reviewed.

**Phase 4: registration.** The crossing with both grains, at the controls and at held-out couplings, written only
after Phases 1–3. It compares the one-grain and two-grain results at matched cutoffs, with the cross term removed
in a control.

## What this could show, and what it cannot

If the crossing and the second-vacuum scalar survive with both grains, they are not artefacts of shaving one way.
If they move or vanish, the one-grain model was cutting too close, and that is worth knowing. Either way it remains
a finite-mode model on a fixed sphere: not the continuum, and not a mass-gap result.
