# A spectral skeleton for fields: what the evidence shows and what I think it means (20 November 2026)

**Research note.** It records what the floor work of 19–20 November uncovered, states my theory as I hold it, and
separates every claim by status:

- **Proved:** exact algebra or symbolic identity;
- **Certified:** a computer-assisted proof with rigorous error bounds;
- **Numerical:** double precision, not certified;
- **Observed:** an experiment with a stated setup;
- **Theory:** my interpretation and conjecture, not established.

The detailed results are in [`floor-family-squashing-2026-11-19.md`](floor-family-squashing-2026-11-19.md) and in
*The Floor Theorem for the Yang–Mills Hessian at the Berger Field*, version 2 (concept
doi:10.5281/zenodo.24386613).

## 1. The object

- **The space.** Take the three-sphere S³ = SU(2). Stretch it along one direction by a factor t; that is the
  Berger metric g = (t, 1, 1). t = 1 is round, and t = 3 is the stretch of the original Floor Theorem.
- **The field.** The space defines its own gauge field, its Levi-Civita connection read as an SU(2) field. I call
  it the Berger field.
- **The question.** Disturb the field and ask what each disturbance costs: the second variation of the Yang–Mills
  energy, restricted to the gauge-orthogonal slice. These costs are the field's tones.
- **Sectors.** Symmetry sorts the tones into sectors by spin j/2, j = 0, 1, 2, … (Peter–Weyl). Each sector is
  finite. The lowest tone in a sector is its floor.

## 2. What the evidence shows

### 2.1 One disturbance is special in every sector — Proved

In every sector j and at every stretch t > 0, one disturbance is an exact vibration of the field, the twist:

  b = f_top (θ¹ + iθ²) ⊗ (τ₁ + iτ₂).

- **What it is.** It sits at the top weight, the most rotation along the stretched axis that the sector allows. It
  turns circularly in two places at once: in the space directions (θ¹ + iθ²) and in the field's internal
  directions (τ₁ + iτ₂).
- **Gauge.** It is orthogonal to every gauge direction, so it is a physical disturbance, not a relabelling.
- **Its cost:**

  **λ_t(j) = [ j(j+2) − 2(t−3)·j − 2(t−2)(t+4) ] / t.**

- **Proof.** A symbolic identity in both j and t (`scripts/ricci_su2_sector_formula_squashed.py`).
- At t = 3 this is the original floor formula, (j(j+2) − 14)/3.

### 2.2 It is the floor — Certified, over a range

The twist is the cheapest disturbance in its sector, with multiplicity exactly two:

- at t = 3, for the sectors j = 0, 12, 20, 24 that survive on the Poincaré homology sphere, together with a bound
  for every higher sector (the published Floor Theorem);
- at t = 4, for j = 0, 12, 20, 24, 30, 32, with the complete Poincaré-sphere spectrum below 136;
- **for every t in [2.75, 10] at once**, in the sectors j = 0–12, 20 and 24. Here the stretch itself was treated as an
  interval: 1,091 slices tile the range, each certified for every t inside it, and none failed.

### 2.3 Below a tipping point it is not — Numerical

- The twist is the floor of every sector j ≤ 40 for every t in [2.7, 10] tested.
- Below t* ≈ 2.698 a different disturbance becomes cheaper, first in the sector j = 4.
- At t ≤ 2 the twist is the floor of no sector.
- This is not certified.

### 2.4 Where each twist goes free — Proved (algebra on 2.1)

- **The crossing.** The twist's cost reaches zero at the stretch
  t_j = ½(√((j+2)² + 2(j+4)²) − (j+2)), and is negative beyond it. Some values: t₀ = 2, t₂ = 2.69, t₃ = 3.045,
  t₄ = 3.403.
- **How it crosses.** Linearly in (t − t_j): a stiffness passing through zero at a critical value of a control
  parameter.
- **How many give way.** At stretch t, every sector with j < (t − 4) + √(3t² − 4t) has negative cost.
- **The floor gap** on the Poincaré sphere is (240 − 24t)/t. At t = 3 it is 168/3, one third of the sphere's first
  scalar eigenvalue. That relation is specific to t = 3.
- **The softest sector moves** with the stretch, to j ≈ t − 4.
- Where the twist is the floor, the certified range of 2.2 applies to these statements.

### 2.5 The landscape and the flow — Observed (exact sector flow, double precision)

- **Operator held at the Berger field.** A disturbance in sector j decays below t_j, grows above it, and turns into
  the twist from any start. The overlap is 1.000 in all six runs.
- **Under the actual Yang–Mills flow.** The Berger field is not at rest. It slides toward a flat field within flow
  time ≈ 0.1, faster than any twist grows, and the disturbance decays.
- **So the floor law describes the landscape around the Berger field,** not yet a transition in time.

### 2.6 The resting relative is stiff — Observed (multistart, double precision)

- **What exists.** Among left-invariant fields on (t₁, t₂, 1), across 36 shapes with axis ratios up to 20, there
  are exactly three resting fields up to gauge: two flat, and one non-flat relative of the meron. A multistart
  search like this is not a proof of completeness.
- **What it does.** The meron relative keeps exactly one unstable direction everywhere. No sector j ≤ 12 goes soft
  on it.

## 3. My theory, as I hold it

1. **Mathematics is an imprint of nature, not an invention.** Nature is what we measure. When numbers touch it
   through abstract ideas, they take the pattern that is already there. Anything that exists, and continues to
   exist, obeys the rules of the turn: chord, arc and tangent (*Three Ways Across a Turn*).
2. **Fields have a spectral skeleton.** A field's possible disturbances form layers upon layers of complex,
   multi-dimensional structure. The lowest few run through everything at once, with the same breath: one formula
   threads through every layer. What we hear when we compute the spectrum is that breath.
3. **The cheapest disturbance is the twist, and a twist becomes a squeeze.** When the shape squeezes far enough,
   the twist's cost crosses zero and the field gives way along it. That is the same law as combustion, buckling,
   or anything that triggers a phase transition: a stiffness crossing zero at a critical value.
4. **The floor law will eventually be recognised as a law of nature.**

## 4. The theory stated as a testable hypothesis

> **Spectral-skeleton hypothesis.** For fields on spaces with symmetry, the low spectrum is organised by a rigid
> skeleton of extremal (highest-weight) modes. Their energies follow exact Casimir-type laws in the quantum numbers
> and the geometry, and they persist through deformations of the space and away from equilibrium.

**Where it stands.**

- **Established:** one instance. SU(2) Yang–Mills on the Berger family:
  - the extremal mode is exact at every stretch (proved);
  - it is the floor across [2.75, 10] (certified);
  - it holds off equilibrium, because the Berger field is not a Yang–Mills field.
- **Not established:** that this is general.

**Three tests.**

1. **The Landau-level test.**
   - **Background.** The Berger sphere is a circle bundle over the round two-sphere (the Hopf fibration), and the
     stretch rescales the circle. A mode's weight along the circle acts like an electric charge moving over the
     base sphere in a magnetic field.
   - **Prediction.** If the twist is a *lowest-Landau-level* state, then λ_t(j) is a lowest-Landau-level energy
     plus a fixed curvature shift. The skeleton would then have a mechanism and a name.
2. **Other symmetric families.** Lens spaces, other squashings, other groups (SU(3) on its homogeneous spaces). If
   an extremal mode again gives the floor by a closed-form law, the pattern is general.
3. **Weak symmetry breaking.** Perturb the space so the sectors no longer separate, and follow the floor mode. If it
   persists continuously and stays lowest, the skeleton is structure, not an artefact of exact symmetry.

**What would count against it.**

- the Landau-level test failing to account for λ_t(j);
- other symmetric families whose floors are not extremal modes;
- the floor mode disappearing under arbitrarily small symmetry breaking.

**The phase-transition reading (point 3 of my theory).** It is established for the landscape, as a stiffness
crossing zero linearly at an exact t_j. It is not established as a dynamical transition: the Berger field moves
faster than the twist grows (2.5), and the stiff resting relative never lets a twist go free (2.6). A dynamical
version needs a resting field on which a twist goes soft. Candidates are resting fields that are not
left-invariant, or other bundles.

## 5. What is not claimed

- The results are about the classical second variation of the Yang–Mills energy. They say nothing about the quantum
  theory or the Yang–Mills mass gap.
- "Law of nature" in §3 is my conjecture and conviction. What is proved is a law of this mathematical object across
  a family of shapes.
