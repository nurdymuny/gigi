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
2. **Other symmetric families.** <del>Lens spaces, other squashings, other groups (SU(3) on its homogeneous spaces). If
   an extremal mode again gives the floor by a closed-form law, the pattern is general.</del><ins>Lens spaces and other
   quotients do not count: the local operator is the same. In any circle-invariant family the extremal mode is
   automatically exact, gauge-orthogonal and closed-form (counting argument, pre-registration §1), so only *being the
   floor* is evidence. Exact criteria (classes E / P / N, verdict rule, families fixed in advance):
   [`skeleton-tests-preregistration-2026-11-20.md`](skeleton-tests-preregistration-2026-11-20.md) §4.</ins>
3. **Weak symmetry breaking.** <del>Perturb the space so the sectors no longer separate, and follow the floor mode. If it
   persists continuously and stays lowest, the skeleton is structure, not an artefact of exact symmetry.</del><ins>As
   first written this cannot fail: an isolated floor persists and stays lowest under any small perturbation, skeleton
   or not (pre-registration §0). <del>Restated so that it can fail: break the circle with (t, 1 + ε, 1). The skeleton
   predicts that the floor pair splits as ε^((j+4)/2), later than every other pair in its sector, which split as
   ε^(q′).</del><ins>Breaking the circle with (t, 1 + ε, 1) makes the floor pair split as ε^((j+4)/2), later than
   every other pair (which split as ε^(q′)). That follows from the charge selection rule, the antiunitary pairing and
   degenerate perturbation theory for any operator with this symmetry, so it verifies the selection rule, not the
   skeleton. The controls C1 (abelianised) and C2 (random circle-invariant) make that explicit.</ins> Gates A-code to
   <del>A-rank</del><ins>A-ctrl</ins>: pre-registration §2. Breaking left-invariance (sectors mixing) is §3, not yet
   designed.</ins>

**What would count against it.**

- the Landau-level test failing to account for λ_t(j);
- <del>other symmetric families whose floors are not extremal modes;</del><ins>any independent circle-invariant family
  in which the extremal mode is the floor nowhere in its pre-set range (class N; pre-registration §4). The Berger
  family itself is class P: the floor only for t ≥ t*;</ins>
- <del>the floor mode disappearing under arbitrarily small symmetry breaking.</del><ins><del>Test 3A: the floor pair
  splitting earlier than ε^((j+4)/2), or not being the last pair in its sector to split (gates A-exp and A-rank (i)).</del>
  Nothing in Test 3A: its outcomes verify the charge selection rule or reveal a code error (pre-registration §2,
  review). Disappearance under small breaking is not on this list because it cannot happen. The falsifiable content
  is Test 2.</ins>

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

## 6. Test 1 run: the twist is a lowest-Landau-level state (20 November 2026)

**Script:** `scripts/ricci_su2_landau_check.py`. **Receipt:** `research/su2-floor-family-20261119/landau_check.json`.
90 cases: t ∈ {1.5, 2, 2.5, 3, 3.5, 4, 5, 6, 8}, j = 1–10.

**Background.** The Weitzenböck form H + GG* = Σ X_i*X_i + R₀ separates the derivative X₀ along the stretched
fibre from X₁, X₂ on the base sphere of the Hopf fibration.

**Results** (Numerical, to round-off, in all 90 cases):

- **Lowest Landau level.** X₊b = (X₁ + iX₂)b = 0, to 1.7 × 10⁻¹². The holomorphic covariant derivative annihilates
  the twist. That is the defining property of a lowest-Landau-level state.
- **Definite fibre <del>charge</del><ins>momentum</ins>.** X₀b = iκb with κ = (2t − 4 − j)/√t. The twist carries
  the definite fibre <del>charge q = j − 2(t − 2)</del><ins>momentum √t·κ = −2(q − t), where q = j/2 + 2 is its
  conserved fibre charge (§7). The charge q is quantized. The −2t is the connection's own component along the fibre:
  a continuous shift of the momentum, not of the charge (redline, review of 20 November)</ins>.
- **The energy is exactly fibre + Landau + curvature**, each term matching to 4 × 10⁻¹⁴:

  **λ_t(j) = (j − 2(t−2))²/t  +  2(j + t)  +  (12 − 8t)**

  - **(j − 2(t−2))²/t** is the fibre kinetic energy of <del>the charge q</del><ins>the fibre momentum, 4(q − t)²/t</ins>.
  - **2(j + t)** is the Landau energy on the base sphere. It is linear in the charge, the signature of a lowest
    Landau level.
  - **12 − 8t** is the lowest eigenvalue of R₀, a constant curvature shift.
- The three terms sum to the proved formula identically. The split itself is verified numerically, not
  symbolically.<ins> X₊b = 0 and X₀b = iκb are statements at the top weight, polynomial in j and t, so they can be
  proved symbolically in the same way as the floor law (`scripts/ricci_su2_sector_formula_squashed.py`); until then
  they stay Numerical.</ins><ins> **Now Proved** (20 November): `scripts/ricci_su2_landau_symbolic.py`, receipt
  `research/su2-floor-family-20261119/landau_symbolic.txt`. At the top weight, (ρ₁ + iρ₂)f = 0 identically, so both
  facts reduce to two j-independent 9 × 9 identities in t on frame ⊗ Lie: (Ω₁ + iΩ₂)β = 0 and
  Ω₀β = i(2t − 4)/√t·β, with β = (θ¹ + iθ²) ⊗ (τ₁ + iτ₂). Both hold symbolically for every t > 0. Hence X₊b = 0 and
  X₀b = iκb, κ = (2t − 4 − j)/√t = −2(q − t)/√t, for every j ≥ 0 and t > 0. The same script proves R₀(t)β = (12 − 8t)β (R₀(t) from
  `scripts/ricci_su2_weitzenbock_symbolic_t.py`). So the fibre term is κ², the curvature term is 12 − 8t, and the
  base term, λ_t(j) − κ² − (12 − 8t), is exactly 2(j + t): the three-term split is an identity for every j and t.</ins>

<ins>**Control (the removal arm)** (`scripts/ricci_su2_landau_control.py`, `landau_control.json`; the same 90
cases). The decomposition into fibre, base and curvature holds for any vector, so it is not by itself evidence. What
could fail is X₊b = 0. It singles out the twist:

- the twist: |X₊b| ≤ 1.4 × 10⁻¹²;
- its conjugate copy (bottom weight): annihilated by X₋ instead (≤ 2.4 × 10⁻¹²), with |X₊b′| ≥ 3.16;
- every other eigenvector of H + GG* in the sector: |X₊v| ≥ 0.52, and none is annihilated.

So the two copies of λ_t(j) are a chirality pair, and the lowest-Landau-level property belongs to the twist alone.</ins>

**What this does for the theory.**

- The skeleton has a mechanism. The floor mode is the lowest Landau level of a charge moving over the base sphere,
  with the stretch setting both the fibre <del>charge</del><ins>momentum</ins> and the field.
- It is not a coincidence of one formula. It is the same structure that organises a charged particle in a magnetic
  field on a sphere (monopole harmonics).<ins> That structure is classical: Wu and Yang's monopole harmonics
  (T. T. Wu, C. N. Yang, *Nucl. Phys. B* 107 (1976) 365–380) and Haldane's Landau levels on the sphere
  (F. D. M. Haldane, *Phys. Rev. Lett.* 51 (1983) 605–608). What is new here is that it survives inside the
  Yang–Mills Hessian, with the frame and gauge indices folded into the fibre charge, at a field that is not at
  rest.</ins>
- **Test 1 passes.**

**A bound that does not work (recorded as a lead).**

- Since |X₊v|² ≥ 0, H + GG* ≥ L := X₀*X₀ + i[X₁, X₂] + R₀ as operators.
- If L's floor equalled λ_t(j), the twist would be proved the floor in every sector, closing the open question of
  minimality for all j. It does not: L lies about 4j + 4t below λ_t(j).
- The cause is the opposite-chirality states (annihilated by X₋), on which i[X₁, X₂] is large and negative.
- A chirality-aware bound, using |X₊v|² or |X₋v|² charge by charge, is the natural next attempt at an all-j proof.

## 7. Toward minimality for every j: chirality and charge (20 November 2026)

**The chirality-aware Landau bound** (`scripts/ricci_su2_chiral_bound.py`, `chiral_bound.json`).

- Since |X₊v|² ≥ 0 and |X₋v|² ≥ 0, we have −X₁² − X₂² ≥ c·i[X₁, X₂] for every c ∈ [−1, 1].
- The fibre charge Q = J₀ + R_frame + R_Lie commutes with H + GG*, with X₀ and with i[X₁, X₂]. So the bound can be
  optimised charge block by charge block: max over c of λ_min(−X₀² + c·i[X₁, X₂] + R₀).
- **Result (Numerical).** The bound is tight at j = 0 for every t ≥ 2.5, and in some small sectors at large stretch
  (j ≤ 3 at t = 6 and t = 10).
- For j ≥ 1 it falls short by an amount growing like j²; at t = 3 by exactly j(j+2)/3. The commutator is linear in
  the weight. It discards the sector's Casimir energy on the base sphere, the "lift" that the twist pays.
- **So the chirality-aware bound alone does not prove minimality for all j.**

**The charge blocks** (`scripts/ricci_su2_charge_blocks.py`, `charge_blocks.json`; t ∈ {2.75, 3, 4, 6, 10},
j up to 128). All Numerical.

- Every sector splits into blocks of fixed fibre charge, each at most 9 × 9.
- **The twist is alone in its block.** Its charge q = ±(j/2 + 2) is the maximum possible: the top weight j/2, plus
  one unit of frame rotation, plus one unit of internal rotation. Exactly one state carries it.
  - So λ_t(j) is a single diagonal entry. **The floor mode is the unique state of maximal fibre charge.**
- **Every other block lies above λ_t(j)**, and its margin grows linearly in j:

| t | j = 4 | j = 32 | j = 128 |
| --- | --- | --- | --- |
| 2.75 | 0.16 | 17.1 | 69.5 |
| 3 | 2.1 | 21.3 | 85.3 (= 2j/3) |
| 4 | 5 | 33 | 129 (= j + 1) |
| 6 | 7.3 | 44.7 | 172.7 |
| 10 | 9.2 | 54 | 207.6 |

- The nearest competitor is the block one charge below the top (q = ±(j/2 + 1)), or q = 0 at the smallest j.

**What this means for the skeleton.** The floor mode is the unique highest-charge state of the symmetry that
survives the stretch, the circle about the long axis. That is the precise sense in which it is extremal. The skeleton
is the highest-weight structure of that symmetry, and the Landau picture of §6 is its energetics.

**Proof plan for minimality in every sector** (open):

1. Write the ≤ 9 × 9 charge blocks in closed form in (j, q, t). Their entries are polynomials and square roots from
   the spin ladder.
2. For large j, prove that every block other than the top lies above λ_t(j) by a margin linear in j.
3. Certify the finitely many small j with the existing interval machinery, over the same t-range as Theorem 6.3.

Together these would give the floor law for **every** sector and every t ∈ [2.75, 10].

**Status (later on 20 November): carried out, and the floor law now holds in every sector for every t ∈ [2.75, 10]
(Proved).** Steps 1–3 are done; details in Result 9 of
[`floor-family-squashing-2026-11-19.md`](floor-family-squashing-2026-11-19.md) and Theorem 6.6 of the Floor paper (v2).

- The block splitting is now exact rather than numerical (`charge_selection_symbolic.txt`).
- Step 2 used a Gershgorin bound. It proves that every other block lies strictly above λ_t(j), not that the margin is
  linear in j.
- The linear growth of the margin in the table above remains Numerical.

## 8. Follow-up to Test 2: why the floor on the Berger field and not on the squashed meron (exploratory, 20 November)

Bee's question after Test 2 ([`skeleton-test2-family1-results-2026-11-20.md`](skeleton-test2-family1-results-2026-11-20.md)):
what about the Berger field puts the symmetric state at the bottom?

**Script:** `scripts/ricci_su2_floor_vs_ceiling.py`. Numerical, not pre-registered.

**The common ground.** The Weitzenböck form H + GG* = Σ X_i*X_i + R₀ holds at both backgrounds (≤ 1.1 × 10⁻¹⁴,
j = 1, 2, 4). In both, (Ω₁ + iΩ₂)β = 0 exactly, so the extremal state is a lowest-Landau-level state on **both**.
The Landau structure comes from the symmetry and is shared.

**What differs is the curvature term.**

| t | Berger field: ⟨β, R₀β⟩ (rank in R₀) | R₀ range | squashed meron: ⟨β, R₀β⟩ (rank) | R₀ range |
| --- | --- | --- | --- | --- |
| 1 | +4.000 (5 of 9) | −2 to +4 | +4.000 (5 of 9) | −2 to +4 |
| 2 | −4.000 (1 of 9) | −4 to +9.66 | +1.261 (5 of 9) | −2.18 to +4.92 |
| 3 | −12.000 (1 of 9) | −12 to +15.54 | −1.073 (5 of 9) | −3.39 to +6.47 |
| 4 | −20.000 (1 of 9) | −20 to +21.49 | −3.267 (5 of 9) | −5.01 to +8.27 |
| 6 | −36.000 (1 of 9) | −36 to +33.44 | −7.482 (5 of 9) | −8.64 to +12.13 |

- On the Berger field, for t ≠ 1, β lies in the **lowest** eigenspace of R₀, at 12 − 8t: a curvature bonus that
  deepens linearly with the stretch.
- On the squashed meron, β sits in the middle of a much narrower R₀ spectrum.
- At t = 1 the two fields coincide, and β is mid-spectrum (+4). That is consistent with the twist not being the floor
  on the round sphere.

**Reading.**

- Symmetry makes the extremal state a lowest-Landau-level state on any circle-invariant background (§6, §1 of the
  pre-registration).
- Whether it is the floor is decided by the curvature term R₀: whether the background's curvature points the
  extremal state's internal frame β at its deepest negative direction.
- The Berger field does this, increasingly with the stretch. The resting squashed meron does not.
- This locates the floor property in the background's curvature, not in its symmetry. It is consistent with Test 2's
  verdict. It is an observation; no test was pre-registered for it.
