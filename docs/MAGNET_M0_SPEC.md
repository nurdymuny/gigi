# M0: does a seamless reading stop the poke term from growing, and does it wobble? (spec)

2026-11-19. Written before any M0 code exists. Research script `scripts/ricci_magnet_m0.py` (to be written),
**not an engine run**. Receipts go to `research/magnet-m0-20261119/`. Abelian control of *Reading the Curve*, as
in V1: round S³/2I, A = θ¹, target velocity −4a, q6, with the same frozen inputs, Whitney mass M and BW05
measure.

Sources:

- `docs/research/magnet-rule-variational-2026-11-19.md` (Lemmas A–C, Bee's magnet rule).
- `docs/research/magnet-rule-review-2026-11-19.md` (Holes 1–5, the seat spring (M5), this test).
- Bee's poke picture: where you poke a shard decides whether it slides or swings, so the push and the twist are
  measured separately.

## In plain English

Today's variational force gets *worse* as the mesh refines (45%, 276%, 330%). The magnet note says why: when
one rod is poked, every cell refits on its own, and the cells disagree at their shared walls. Remove the seams
and the growth should stop (Lemma A). The review adds a risk: a reading that votes is blind to jagged
patterns, so the frame may wobble freely (Hole 1). The cure for that is a spring that keeps each cell seated on
its own walls (M5).

M0 measures exactly those two things with the cheapest seamless reading there is. It builds nothing else: no
zig-zag, no exact gauge, no new verbs. It is a test of the mechanism, not of accuracy.

## The reading (fixed now)

A **moving least-squares 1-form**, refitted at every evaluation point x:

- **Fit.** At x in cell K, in K's orthonormal chart (the published fit's chart and transported neighbours), choose
  the polynomial 1-form p_c of degree k that minimises Σ_e w_e(x) (∫_e p_c − a_e)². The line integrals run along
  the chart segments, as the published fit treats them.
- **Reading.** R¹a(x) = p_{c(x)}(x).
- **Weights.** w_e(x) = φ(|x − m_e| / ρ), where m_e is the edge midpoint in the chart and φ is the Wendland C²
  function φ(r) = (1 − r)⁴₊(4r + 1). The weights vary smoothly with x, so the reading is smooth inside each chart.
- **Reach ρ.** One constant per level: ρ = κ · h_max. κ is the smallest value in {1.0, 1.25, 1.5, 2.0, 2.5, 3.0}
  that satisfies both of these at every quadrature point of every cell:
  - at least **twice** as many rods with w > 0 as coefficients (the vote of the count note);
  - a weighted Vandermonde condition number ≤ 10⁸.

  The chosen κ and the minimum rod count are recorded.
- **Degree.** k = 1 (12 coefficients) is the primary arm. k = 2 (30 coefficients) is **not** run in M0; it is
  decided after M0 (see the stop rules).
- **The curvature reading** is *defined* as the curl of the reading, with the **full** derivative of the moving
  fit, including the derivative of the weights:

  curl R¹a = curl p_c + Σ_j ∇c_j × (basis_j),

  with ∇c from c = G⁻¹b (G = VᵀWV, b = VᵀWa), differentiated exactly. This is what Lemma A needs: the face
  reading is the curl of the shared bump.

## The energies (two arms)

- **Unseated:** E(a) = ½ Σ_K w_K ∫_K |curl R¹a|²_g (quadrature order 6, the code's metric pairing and Piola map).
- **Seated (M5), σ = 1:** E_σ(a) = E(a) + ½ σ Σ_K w_K r_Kᵀ G^W_K r_K.
  - Here r_K is the misfit of K's own four wall fluxes: the reading's flux (by Stokes, the line integrals of R¹a
    around each wall) minus the measured flux (Da)_f.
  - G^W_K is the Whitney 2-form Gram of K, so σ = 1 makes the spring exactly as stiff as Whitney's own walls.
  - Wall fluxes vanish on pure gauge pokes, so the spring adds no gauge stiffness of its own.

The force is K a, where K = ∇²E. It is symmetric by construction, and the mass is the Whitney M (option A).

## Gates and predictions (fixed now)

Levels: n = 2 and n = 3 decide. n = 1 is reported only (Hole 4: 142 rods in the whole space).

- **P-A (Lemma A), unseated arm.** The *poke term*, ⟨F, curl R¹η⟩ − ⟨F, W²Dη⟩, is assembled and normalised
  exactly as V1's test-variation term. Its value at n = 3 is **no larger** than at n = 2. Over the same step the
  seamed two-ring variational force went from 276% to 330% **[measured]**.
- **P-wobble (Hole 1).** Count the eigenvalues below 1 of (K, M) restricted to the M-orthogonal complement of the
  gauge directions range(δ⁰). The smallest continuum eigenvalue here is 4.
  - **Unseated:** at least one at n = 2. The wobble exists.
  - **Seated:** none at n = 2 or n = 3. The spring removes it.
- **P-seam (Hole 5).** At three points on every interior wall, evaluate the reading from each side on the wall's
  two edge vectors (a chart-free quantity), using the same edge data. The RMS jump, relative to the RMS reading on
  walls, is **≤ 1%** at n = 2 and smaller at n = 3.

## Reported, not gated

- The BW05 relative error, for both arms. Against Whitney's 26.0% (n = 2); n = 3 is listed in the paper.
- The reconstruction term, for both arms. At k = 1 it is not expected to converge, because k = 1 tests the
  mechanism, not the accuracy.
- The seat-spring term's share of the force error (seated arm).
- **Push and twist** (Bee's poke picture). Per edge, the column defect
  ⟨G, curl R¹δ_e⟩ − ⟨G, W²Dδ_e⟩ is split into its push part (the 3 constant generators) and its twist part
  (the 20 linear generators). It uses V1's generators, lift and pairing, and is normalised by the Whitney
  pairing's size. It is reported for the seamless reading (both arms) and, for comparison, for the seamed
  two-ring reading with C = 0, which V1's machinery already computes. Lemmas B and C read this way:
  - **push:** the shared bump should match Whitney's push more closely as h falls;
  - **twist:** it is not required to match, and the seamed reading's failure should show up as cells
    disagreeing about the twist.
- **Gauge stiffness:** the largest Rayleigh quotient of (K, M) on unit gauge pokes δ⁰1_v, relative to the
  smallest physical eigenvalue.
- The neighbourhood rings needed to cover the reach, and the seconds per stage.

## Controls

- **C-poly.** For every basis 1-form p of degree ≤ k in a cell's chart, with edge data its exact chart line
  integrals over the reach, the reading reproduces p, and curl p, at the cell's quadrature points to 10⁻¹⁰
  relative.
- **C-deriv.** The analytic curl agrees with a central finite difference of R¹a (step 10⁻⁵ h) to 10⁻⁶ relative,
  at 20 random points per level, using the stored real data.
- **C-sym.** ‖K − Kᵀ‖/‖K‖ ≤ 10⁻¹².
- **C-copies.** No reach sees two copies of the same edge. If it fails at a level, that level is reported
  inadmissible.
- **C-gauge-count.** The gauge dimension used is V − 1 (H¹ = 0 on S³/2I): 21, 163 and 545 at n = 1, 2, 3, from
  Euler's formula on each mesh.

## Stop rules (fixed now)

- **P-A fails:** the poke term still grows without seams. Lemma A's mechanism is not what drives the poke term,
  so **no magnet construction is built**, and the variational route returns to V2's results.
- **P-A passes, P-wobble (seated) fails:** the spring is not enough. M5 must be rethought before anything is
  built on it.
- **Both pass:** the next spec is k = 2 (accuracy), with the reach fixed by the same vote rule. The mass decision
  (A or B) is taken from the seated arm's coercivity.
- **A k = 1 reconstruction term that does not converge is not a failure**, by design.

## Recorded for later: the force's own geometry (not part of M0)

Bee: *the force has an angle and a dimension of its own; the force is itself a bundle, and so is what it pokes.*

In the SU(2) case the force on a rod is a direction in the Lie algebra, a section of the adjoint bundle. Before
two cells' responses can be said to push "the same", the poke's direction has to be carried into each cell's
frame along the connection: the register step, applied to the force. So when this work moves to SU(2), Lemma B's
moment condition has to be stated **covariantly**, with transported directions. The Abelian control cannot see
this, because every direction commutes.

Altitude:

- **STRUCTURAL** for the moment link (the same objects as Lemmas B and C) and for the bundle-valued force (the
  standard Yang–Mills setting).
- **RESONANCE** for the link to ICARUS, not checked.

## Cost

- **Per cell:** about 24 quadrature points, each a weighted least-squares solve with 12 unknowns from about 25 to
  60 rods, plus its derivative.
- **K** is dense at n = 2 (3,372) and n = 3 (11,358).
- **The complement eigensolve** at n = 3 is the largest step, of the size B6 already ran.

There is no constraint system and no lsqr.

## Not done here

k = 2. The magnet mass (B). SU(2). The zig-zag. Exact gauge. The flow.
