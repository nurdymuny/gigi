# The magnet rule, reread: the physics in plain terms, the holes, and one small test (19 November 2026)

A review of `magnet-rule-variational-2026-11-19.md`, written on the Windows machine. No new measurement.
The same status labels: **[measured]**, **[derived]**, **[conjecture]**, **[decision]**.

## 1. The physics, rebuilt without symbols

The lattice is a frame of rods. Each rod (edge) carries one number: how far the field twists you as you walk
along it. Each triangular wall (face) carries one number: the twist around its rim, i.e. the curvature passing
through it. The energy is the total squared curvature. The force on a rod is how much the energy changes when
that one rod is nudged.

A force is variational when it comes from one energy landscape. Physically, three things follow:

- you cannot gain energy by nudging around a closed cycle;
- push and push-back are equal;
- the frame cannot run away, because a sum of squares has no downhill direction that goes on for ever.

To get the energy right we do not trust the walls one by one. We **read** a smooth field from the rods around
each cell, the two-ring or hybrid fit, and take the energy of that reading. The trouble starts when we ask for the
force, because the force is a *poke* question: nudge one rod, and how does the energy change?

- **The per-cell reading today.** Every cell that can see the poked rod refits on its own. A dozen patches each
  explain the poke their own way, and they disagree where they meet. The energy counts the disagreement at those
  seams as if it were real curvature. A poke is a sharp kink, so its curvature scales as 1/h. The error therefore
  *grows* as the mesh refines: 45%, then 276%, then 330% **[measured]**.
- **Bee's rule.** Before you poke, know what the piece is connected to, and let the poke's energy spread along
  those connections. In physical terms, a poke must produce **one shared bump**: a single smooth disturbance that
  every connected cell agrees on, with no seams. The **target** is set by the same magnets: what a poke "weighs"
  (the mass) is measured with the same bumps.
- **Why that cures it (Lemma A, [derived], and I checked the algebra).** With no seams, the poke's derivative can
  be moved onto the true field by integration by parts, and the true field is smooth. What is left compares the
  bump with the Whitney tent it replaces, without differentiating either. The 1/h disappears. This is the core
  physical claim of the note, and I think it is right.
- **Why V1 had to fail (Lemma C).** V1 kept the seams and tried to fix the force by forcing it to agree with
  Whitney on 20 *tilted* (linear) fields. Whitney preserves each wall's total but not where the flux sits on it,
  so agreeing on tilts and reading exactly at the same time is impossible to first order. The only place left to
  pay was the field's own curvature, which is the count's finding. Two different routes give the same diagnosis,
  which is good evidence.

## 2. The holes

**Hole 1, the main one: a voting reading has blind spots, and blind spots rattle.** The two-ring fit is accurate
*because* it votes: rough data is outvoted and comes back as almost nothing. Now take the energy of such a reading.
A jagged pattern of rod values that the vote throws away costs almost no energy, so the energy cannot push it
back. That is a zero-energy wobble, the hourglass mode of reduced-integration finite elements.

- **Today's readings avoid it by the seat.** They hold the cell's own walls exactly, so every jag is still felt
  through those walls. Whitney avoids it the same way.
- **A smooth, shared magnet reading built only to reproduce polynomials has no seat.** It will very likely have
  more near-zero modes than the gauge dimension. With the Whitney mass (option A), those modes are neutral: they
  neither grow nor decay, and S3's small-eigenvalue count would fail. With the magnet mass (option B), B is
  singular on exactly those modes, so the flow cannot even be solved. **[derived, not checked]**
- **The cure has a formal home.** Keep the seat as a spring, not a clamp:

  E(a) = ½‖d R¹a‖² + ½ σ Σ_cells ‖(own wall fluxes) − (the reading's own wall fluxes)‖².

  This is the consistency-plus-stabilization split of the virtual element method, and it is also hourglass
  control. On smooth data the spring term is O(h^{k+1}), because the reading reproduces the walls up to its
  approximation error. Its contribution to the force is then relatively O(h^{k−1}): first order at k = 2. And the
  energy now feels every wall, so no wobble survives. **[derived, sketch]** The note's (M1)–(M4) should get a
  fifth condition, **(M5): seated**. Every poke costs energy in proportion to its Whitney energy, up to gauge.

**Hole 2: the "hard part" (§7) may not be needed to test the idea.** Lemma A needs only conformity (M1), with the
curvature reading *defined* as the curl of the shared bump. It does not need exact gauge annihilation. A reading
that is smooth by construction is conforming automatically. Moving-least-squares readings are smooth: the fit is
recomputed at each point with smoothly varying weights. That takes the Čech–de Rham zig-zag off the critical path.

What is lost is that a pure gauge poke costs O(h^k) energy instead of zero. That is not instability, because the
energy is a sum of squares and stays nonnegative. It does make the gauge directions slightly stiff and adds a small
gauge component to the force. Whether that matters is a measurement, not a prerequisite. The zig-zag becomes worth
building only if that measurement says so.

**Hole 3: the mass decision is smaller than it looks.** BW05 compares the computed velocity with the exact edge
velocity −4a, measured in the Whitney norm. With the magnet mass (B), the *method* changes: it solves B ȧ = −K a.
The *gate* does not: the same target vector and the same norm. So (B) does not require restating the gate, only
reporting both masses. But (B) is ruled out unless Hole 1 is closed, since B must be invertible on rough modes.

**Hole 4: the coarsest mesh is too small for wide magnets.** A quadratic 1-form has 30 coefficients. By the count
method a vote needs about 60 or more rods inside each magnet's reach. At n = 1 there are only 142 rods on the
whole manifold, so each reach would cover close to half the space. Scaling evidence has to come from n = 2 and
n = 3, with n = 1 reported only.

**Hole 5: the chart.** A smooth reading built cell by cell in each cell's own chart is conforming only up to the
chart change between neighbours. That jump is O(h^{k+1}), not the O(1) seam of today. It is small, but it has to
be measured, not assumed.

**On "No Free Lunch".** The four properties we want (variational, local, accurate, stable) are the ones Wardetzky
et al. show cannot all hold for scalar Laplacians on general meshes. The seated-spring version bends locality,
since its magnets reach further than Whitney's. That is consistent with the note's own reading of the paper.

## 3. What I would change in the plan

1. **Keep V1c** (the Mac's spec of §5, `docs/VARIATIONAL_RECOVERY_V1C_SPEC.md`). It is cheap and tests Lemma C.
   It does not test Lemma A, because the reading stays seamed.
2. **Before building any zig-zag, test Lemma A and Hole 1 directly**, with the cheapest conforming reading there
   is (§4).
3. **Add (M5) seated** to the design. Build the spring in from the start, rather than discovering the wobble
   after a full construction.
4. **Decide the mass after §4.** If the seated reading is coercive, (B) becomes available. If it is not, (B) is off
   the table regardless.

## 4. A small test that cannot become a rabbit hole: M0

One script, one reading, the Abelian control, n = 2 and n = 3 (n = 1 reported only), the Whitney mass (A). No
zig-zag, no gauge exactness, no new verbs.

- **Reading.** A moving-least-squares 1-form, rebuilt at each quadrature point:
  - a weighted least-squares fit of a polynomial 1-form of degree k to the integrals along nearby rods;
  - smooth compactly supported weights (Wendland C²) in the cell's chart;
  - reach chosen so that every fit has at least twice as many rods as coefficients (the vote), with the count
    recorded.

  k = 1 (12 coefficients) is the primary arm. It is enough to test the *poke* term, which is about seams, not
  order, and it fits the meshes we have. k = 2 (30 coefficients) runs at n = 3 only if k = 1 behaves.
- **Energy.** E = ½∫|d R¹a|², with the full derivative of the moving fit, including the weights' derivative, so
  that the face reading is exactly the curl of the reading. A second arm adds the seat spring, with σ = 1 in
  Whitney units.
- **Measured.** The two terms of Lemma A (reconstruction and moment), separately, at each level; the BW05 error;
  the near-zero eigenvalue count of (K, M) against the gauge dimension; the tangential jump across walls (Hole 5).
- **Predictions, to be fixed in a spec before the code runs:**
  - **P-A (Lemma A).** For the unseated reading, the moment term at n = 3 is no larger than at n = 2. The seamed
    variational force went from 276% to 330% over the same step **[measured]**, so this is a sharp contrast.
  - **P-wobble (Hole 1).** Without the spring, the near-zero count exceeds the gauge dimension at n = 2. With the
    spring, it equals it.
  - **P-seam (Hole 5).** The tangential jump across walls is at most 1% of the reading's size at n = 2 and
    shrinks at n = 3.
- **Stop rules.**
  - If P-A fails, the seams are not what drives the poke term. Lemma A's mechanism is wrong, and the magnet
    construction is not built.
  - If P-A passes but the reconstruction term does not converge at k = 1, that is expected and not a failure:
    k = 1 tests the mechanism, not the accuracy.
  - Accuracy is k = 2's question, asked only after P-A and P-wobble pass.

Cost: the reading reuses the chart and transport code. The fits are small: 12 or 30 unknowns from about 25 to 70
rods per point. K is dense at n = 2 (3,372) and n = 3 (11,358), as in B6. This is not a construction project: it is
one measurement of the note's central claim and of its biggest risk.

## 5. Questions for Bee

- **[decision]** Add (M5) seated, as a spring rather than a clamp? It is the published "seat", made soft.
- **[decision]** Run M0 before any zig-zag construction, alongside V1c? I would write its spec next if yes.
