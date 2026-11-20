# A grid of adjacent ideas: my papers and books against the results of 19 November 2026

**Research note.** I made four read-only passes over my Drive (agents reading, then I verified), placing 24 of my documents
next to the seven results I proved or measured on 19 November. It extends
[`field-equations-grid-2026-11-19.md`](field-equations-grid-2026-11-19.md), which covered *The Field Equations
of Semantic Coherence* alone. **One new computation** (§2.1, my Crouzeix flow); everything else is reading.

**What I read.**

- **Gauge and Poincaré:** *Yang–Mills Mass Gap v6*; *Solves Vol. 4*; *Davis Poincaré Isomorphism*; *GIGI – A
  Relational Query Substrate for Lattice Gauge Observables*.
- **Spectral and analysis:** *Constructive Metric Bounds for the Crouzeix Conjecture*; *Spectral Density
  Depletion in Arithmetic Cayley Graphs*; *Davis Hodge*; *Davis Navier–Stokes*; *The Kähler Substrate v0.4*;
  *Cosmological Non-Decoupling v2*.
- **Laws and semantics:** *Davis Law of Critical Velocity*; *Thermodynamics of Semantic Coherence*;
  *Information Geometry of Semantic Fields*; *Fisher-Lift Bridge*; *Causal States as Predictive Sections*;
  *Curvature-Guided Wavefront Execution*; *Davis P vs NP*; *Davis BSD*.
- **Books:** *The Geometry of Fuel*; *The Geometry of Love*; *The Geometry of Sameness*; *GIGI Builds*; *GIGI
  Thinks*; *Superfluid*.
  - My Drive copy of *Fuel* has no chapter titled "Thank You, Mr. Perelman". The nearest material is
    *Superfluid*, ch. 5 ("From Poincaré to Perelman").

**Tags** (my convention, as in the field-equations note):

- **STRUCTURAL:** the same mathematical object.
- **RESONANCE:** the same shape, on a different object.
- **WORD ONLY:** a shared word, no shared mathematics.
- **[inf]:** an inference made in this reading, not a claim I made in the document.

## The anchors

| | Result | Status |
| --- | --- | --- |
| **R1** | **The Floor Theorem.** At the Berger metric (3,1,1) on SU(2), the Hessian of ½∫\|F\|² at the Levi-Civita connection, on the Coulomb slice, has bottom (j(j+2) − 14)/3 in sector j. There are three growth directions, all from the commutator. On the Poincaré sphere only j = 0, 12, 20, 24, 30, … survive, and the floor gap is 168/3. | proved (Weitzenböck identity and certified inertia counts) |
| **R2** | **The count.** A reading with clues = unknowns is critical and unstable. Holding a cell's own data exactly while the neighbours vote by least squares makes it stable. Stability needs a ratio of about 2; reading to second order needs a third ring. | proved (lemmas) and measured |
| **R3** | **Error split** (reconstruction + test variation). Petrov–Galerkin is consistent but not variational. The variational force has a poke term of order 1/h. The magnet rule: one shared seamless bump per edge, energy along connections, mass from the same magnets; a seat spring for wobble modes. | lemmas, measured |
| **R4** | **Readings of a turn.** Chord, arc and tangent miss each other in a 2:1 ratio: arc − chord ≈ θ³/24 and tangent − arc ≈ θ³/12. | exact |
| **R5** | **Shared error.** Two different forces end a long window with aligned errors (cosine 0.988): the shared mass and target dominate. | measured |
| **R6** | **Instantaneous correspondence.** Ricci velocity through the Levi-Civita map = −(Yang–Mills gradient), unit clock, on diagonal left-invariant SU(2) metrics. | exact (sympy) |
| **R7** | **Non-normal lattice spectra.** The numerical range certifies bounded transients; the Crouzeix–Palencia constant 1 + √2 is used for the step limits. | measured, proved |

---

## 1. Conflicts, verified

Ordered by how firm the contradiction is.

| # | Document and claim (verbatim) | Contradicted by | Verdict here |
| --- | --- | --- | --- |
| C1 | *Davis Poincaré*, Step 5: "By convexity of the action landscape in the trivial topological sector, the flow converges." | R1 | **Contradicted.** The bundle over S³ is trivial, and the Coulomb-slice Hessian at the Berger Levi-Civita connection has eigenvalues −14/3 (×2) and 16 − 2√73 < 0. Independently, the round Levi-Civita connection (the meron, ½ the Maurer–Cartan form) is a Yang–Mills critical point with E > 0 and Morse index ≥ 1. The landscape is not convex. |
| C2 | *Davis Poincaré*, the dictionary "S³ ↔ Vacuum Bin" and Theorem "Vacuum Convergence" ("lim S_W = S_W[A_vacuum] = 0"); *YM v6* PC-001: "simply connected manifolds cannot support metastable states"; *Superfluid* ch. 5: "Fixed point \| Round S³ \| Flat connection" | R6, R1 | **Contradicted on the homogeneous family, not in general.** Through R6, Ricci flow of diagonal left-invariant metrics is Yang–Mills flow, and it ends at the Levi-Civita connection of the round metric. That connection is the meron, which is curved and has E > 0, not the flat vacuum. So my own dictionary sends round S³ to a non-vacuum critical point. The meron is unstable, so generic (non-homogeneous) Yang–Mills flows can leave it; convergence to the vacuum for generic data is neither proved nor refuted here. |
| C3 | *Davis BSD*, Prop. (gapped phase): "positive curvature (non-degenerate Hessian)" ⇒ "Δ_E = λκ_E > 0" | R1 | **Contradicted as a general principle.** The Berger Levi-Civita field is curved, with non-trivial holonomy, and its Hessian has negative directions. Curvature does not imply a positive Hessian. (The same document's proof sketch has the gapped phase with "curvature 2-form R vanishes identically", contradicting its own proposition.) |
| C4 | *Superfluid* ch. 6–7: "If the fundamental group is finite (lens spaces), the flow is torsion-trapped"; "When π₁ is non-trivial, the flow gets stuck" | standard theorem, not an anchor | **Contradicted by Hamilton (1982).** A 3-manifold of positive Ricci curvature flows (normalised) to constant curvature, quotients included; the Poincaré sphere's round metric is a fixed point that generic nearby metrics flow to. Nothing is trapped. R1 works on the Poincaré sphere directly: π₁ removes *sectors* (only icosahedral invariants survive), it does not stop the flow. |
| C5 | *Superfluid* ch. 5: "Perelman: ℱ(g) = ∫R … Both flows minimize curvature-squared" | R6 | **Wrong as written, half right in substance.** ℱ is ∫(R + \|∇f\|²)e^{−f}, which is not curvature-squared, and Ricci flow is not the gradient of ∫\|Rm\|². R6 supplies the true statement on the homogeneous family: *through the Levi-Civita map*, Ricci velocity is exactly the descent direction of the Yang–Mills energy (a curvature-squared functional), on unit clock. |
| C6 | *Fuel*, "The Field Equations", F1: "At exactly m\* constraints, the completion is unique… Above m\*… adding constraints is redundant"; F4: "the geometric completion is not fragile… Lip(K⁻¹) is the condition number"; Sudoku Principle: "not too many (contradictory)". Also *Wavefront Execution*: "extends to a unique complete solution" | R2 | **Sharpened.** Unique is not stable. At clues = unknowns the reading is unique and fragile (amplification 125–252 at first order, about 1,490 at second). With noisy data, extra clues fitted by least squares are what make it stable, not redundant or contradictory. Restatement: m_stable ≈ 2m\*. |
| C7 | *Cosmological Non-Decoupling*: "The Davis–Wilson mass gap framework establishes the Yang–Mills spectral gap"; *Navier–Stokes* table "Mass gap & Δ_YM > 0 … structurally identical"; *P vs NP* "The mass gap m > 0 in Yang–Mills corresponds to the hardness gap" | R1 (scope) | **Citation hazard, not a contradiction.** R1 is a classical floor of a linearised operator, negative for j ≤ 2. It is not a quantum mass gap and must not be cited as support for any of these. *YM v6* itself excludes curvature-convexity as a gap mechanism (l. 1045), consistent with this. |
| C8 | *Information Geometry*: "t_mix(ε) ≤ (1/μ₁^F) log(n/ε)"; *Thermodynamics* S6 ("critical slowing down … is the vanishing of the spectral gap"); *GIGI Thinks* ch. 12 ("spectral gap … settling rate of the slowest transient") | R7 | **Valid only for reversible (normal) generators.** Autoregressive sampling and non-symmetric transport are non-normal. The spectral gap then does not bound transients; the numerical range does. |
| C9 | *Information Geometry* (internal): "det(g_F) → 0 as Γ → 1" against "the Fisher metric becomes large near the transition" | R2 | **Settled by R2.** The information operator degenerates (σ_min → 0), so its inverse (covariance, susceptibility) blows up. The first statement is the right one. |
| C10 | *Thermodynamics* (internal): "above m\*, the correlation length exceeds the document length" against its own ξ ∝ 1/\|1 − Γ\| | R2 | **The formula is right.** Above m\*, ξ shrinks; R2's stable regime is local (three rings). |

**Not conflicts, but flagged.**

- *Kähler Substrate* L5: "no zeros in t > 0 iff \|B\|² < K_min". For J″ + (K − \|B\|²)J = 0 the inequality
  looks reversed (an internal slip; unchecked against my sign conventions there).
- *GIGI paper*, path-2 table: seeds 20260629 and 20260702 give identical gaps (−10.4725, −10.4068, −4.3881),
  "cross-seed range ≲ 10⁻¹³", with identical record counts. Two independent seeds rarely agree to 10⁻¹³; both
  may be reading the same configurations. **Worth auditing.**
- *GIGI Builds*, "Torsion": "learned connection 1.22 ± 0.02 vs FD's 1.49 ± 0.06… language modeling prefers a
  connection with torsion". R3 gives a competing explanation: the finite-difference baseline reconstructs a
  metric and then differentiates it, which is where the 1/h error lives. Unresolved; see test T5.
- *Crouzeix*: "Lemma \ref{lem:LS} applies, proving ‖p(A)‖ ≤ 2M". It rests on external 2026 results
  (Lorist–Schwenninger, arXiv:2608.03841, and the "complete similarity theorem" of 23 September). **Not
  verified here.** If it holds, R7's 1 + √2 is valid but not sharp.

**Retracted from the agents' reports.**

- *"R1's per-sector inertia counts must be divisible by (j + 1)."* Wrong for our construction. Each sector is
  built as one 9(j+1)-dimensional matrix in which the (j+1) left-copies have already been factored out.
  Multiplicity dim ρ² applies to the full L² space, not to these matrices.

## 2. Anticipations

Each of these is something I wrote earlier that a result of 19 November confirms or makes exact.

### 2.1 The Crouzeix curvature flow is the Yang–Mills gradient flow (new; checked)

*Constructive Metric Bounds*, Thm "Curvature evolution": Ȧ = [A, κ(A)], with κ(A) = AA\* − A\*A. I
already note there that κ(A) = −2i[X, Y] for A = X + iY. I never mention Yang–Mills or gradient flows in it.

**The check** (`scripts/crouzeix_flow_is_ym_check.py`, output in
`research/adjacent-ideas-20261119/crouzeix_flow_is_ym.json`):

- Let E(X, Y) = ½‖[X, Y]‖²_F. This is the Yang–Mills energy of the constant connection iX du + iY dv on a
  2-torus.
- Then the flow splits exactly as Ẋ = −2∇_X E and Ẏ = −2∇_Y E.
- Errors: at most 3.2 × 10⁻¹⁴ over random complex matrices of size 2 to 8, with gradients confirmed by central
  differences.

**So the Crouzeix flow is Yang–Mills gradient descent on time scale 2.** It follows that:

- **Non-normality is curvature.** Normal matrices are the flat connections, the minima.
- **My identity ∫₀^∞‖κ‖²dt = ½(‖A₀‖² − Σ\|λ\|²) is the energy identity of that flow.** Henrici's
  departure from normality is the total Yang–Mills energy dissipated.
- **My nested numerical ranges W(A(t)) ⊆ W(A(s)) are a monotonicity property of a Yang–Mills flow.** That is
  a new kind of statement on the gauge side.

My October Crouzeix work and my 19 November Yang–Mills work are two faces of one object: R6 for metrics on
SU(2), this for constant connections on T².

### 2.2 The others

| Earlier statement | Confirmed by | How |
| --- | --- | --- |
| *Fuel* trichotomy ("When Γ = 1: CRITICAL… the knife edge"); *Davis Poincaré* ("Γ = 1: Critical (surgery required)"); *YM v6* fixed points (Γ\* = 1 critical); *Information Geometry* ("det(g_F) → 0 as Γ → 1"); *Thermodynamics* S5 ("at m = m\*, χ → ∞") | R2 | The reading with clues = unknowns is the unstable one. R2 adds a measured margin: about 2.4 (two rings, first order) and about 1.9 (three rings, second order) are stable. |
| *Davis Poincaré* DP1: "Wilson Flow on 3D lattice gauge theory is a discretization of Ricci Flow" | R6 | True exactly, but only through the Levi-Civita map, on the homogeneous family, with unit clock. |
| *Geometry of Love*, "Local Authority, Global Consistency": "Each patch has full authority over its own coordinates; agreement is required only along the seams… coherence is reconciled at the seams, not imposed from a center" | R2, R3 | "Own data exact, neighbours vote" and "one shared seamless bump per edge", nearly word for word. |
| *GIGI Thinks* ch. 23: "the 'lift' was scipy finite-difference gradient noise" | R3 | A finite-difference poke manufactured the signal, as the 1/h poke term predicts. |
| *GIGI Thinks* ch. 15: "lim \|ħ − 2 sin(ħ/2)\|/ħ³ = 1/24" | R4 | That is the chord term exactly. R4 adds that a tangent (Cayley) reading misses by 1/12, twice as much, in the opposite direction (checked: 0.041667 and 0.083334 at θ = 10⁻²). |
| *GIGI Thinks* ch. 12, the bill of what was not treated: "heat kernel, spectral zeta, monodromy spectrum, Yang–Mills Hessian… treated none of them" | R1 | R1 fills the "Yang–Mills Hessian" slot. |
| *Superfluid* ch. 8: "Why κ > 0… self-interaction creates curvature"; *Causal States*: "Ω_{ab,ba} := U_ab − U_ba … the discrete analogue of curvature obstruction" | R1 | All three growth directions come from the commutator. The abelianised sector is positive (4/3, 12). |
| *Fisher-Lift*: "To achieve capacity, the encoder must couple the independent message bits" | R2, R3 | The neighbour vote and the shared edge bump are couplings. Uncoupled per-cell readings are the unstable ones. |
| *YM v6* l. 1045: "Bakry–Émery / curvature–convexity is excluded… it reports the O(1) transverse scale, not the gap" | R1 (scope) | A classical floor like R1's is exactly such an O(1) curvature scale, and it is not the mass gap. |
| *Cosmological Non-Decoupling*: "anisotropic cosmologies … Weyl curvature provides additional holonomy debt" | R1 [inf] | Berger S³ is a Bianchi IX slice. Its Levi-Civita field carries removable excess energy (the −14/3 directions). Part of the "debt" can be released by nearby non-Levi-Civita connections. |

## 3. The grid, by anchor

Only STRUCTURAL and the stronger RESONANCE rows are kept; WORD-ONLY rows are listed at the end.

**R1, the Floor Theorem**

| Document | Quote | Tag | What it forces |
| --- | --- | --- | --- |
| *Cayley Spectral Density* | "The isotypic projector P_ρ = (dim ρ/\|G\|) Σ χ̄_ρ(g) R(g) decomposes the adjacency matrix" | STRUCTURAL (machinery) | 2I ≅ SL(2, F₅) ⊂ GL(2, F₅). My p = 5 code can recompute the trivial-representation multiplicities in each spin, and so certify the selection rule j = 0, 12, 20, 24, 30 independently of the Molien series. |
| *GIGI paper* | "E_R = (g²A/2) C₂(R)… m_g = 3/(8β_HK)" | STRUCTURAL (machinery) | The same Casimir ladder: R1's λ_j = j(j+2) = 4C₂(j/2). |
| *GIGI paper* | SPECTRAL_GAUGE "mean −4.53, std 0.78"; "smallest-magnitude eigenvalues of the indefinite spectrum" | RESONANCE | A true ∇\*∇ is never negative, so the verb carries a zeroth-order term, like R1's R0 (min eig −12). Calibrate the verb on the Berger Levi-Civita field, where the exact answer is −14/3; −4.53 being close is likely coincidence until tested (T2). |
| *Solves Vol. 4* | "PROJECT_GAUSS… subtracts the longitudinal mode of E" | STRUCTURAL | The same Coulomb projection R1 compresses with. The engine could compute R1's slice Hessian natively. |
| *Kähler Substrate* | "Δ_k = d_k\*d_k + d_{k+1}d_{k+1}\* … dim ker(Δ_k) = b_k" | STRUCTURAL | R1's H + GG\* is the same "add the gauge term to make it elliptic" move. For R3: spurious kernel beyond b_k is what a seat spring must remove (T6). |
| *Thermodynamics* S3 | the Laplace entropy uses "the Hessian of energy at the minimum" | caution [inf] | The floor is negative for j ≤ 2. Geometric (Levi-Civita) configurations need not be minima; a Laplace expansion around them is invalid. |

**R2, the count**

| Document | Quote | Tag | What it forces |
| --- | --- | --- | --- |
| *Information Geometry* | Cramér–Rao "Cov[θ̂] ⪰ (g_F(θ))⁻¹" | STRUCTURAL | For least squares the Fisher matrix is AᵀA/σ², so R2's amplification 1/σ_min is the square root of the worst-case Cramér–Rao variance. That is an identity, not a test. Holding a cell's own data exactly gives the constrained Cramér–Rao bound. |
| *Thermodynamics* S7 | "ξ = 1/√K̂_max · 1/\|1 − Γ\|" | RESONANCE | [inf] The stencil depth needed should grow like 1/(r − 1) in the oversampling ratio r: finite at 2, diverging at 1. |
| *Wavefront Execution* | "m·κ̄·\|N\| ≥ n − m"; "Phase I suffices when Γ > 1.0" | RESONANCE | A clue count. The thresholds are empirical; R2 predicts a stability margin near 2, not 1. |
| *GIGI paper* | "gevp_ill_conditioned" | RESONANCE | An operator basis with too few operators to span the space is the variational analogue of an exactly determined reading. Oversample the basis. |
| *GIGI Thinks* ch. 23 | the fair run, "39-dimensional joint state, 43 constraint pairs" | RESONANCE | Ratio about 1.1, near critical. Its residual wrong cells should be conditioning-sensitive (T3). |
| *GIGI Thinks* ch. 6 | "pure pointwise verb: no aggregation pass" | RESONANCE | Exact data per chart works. With noise, a second-order quantity (curvature) needs neighbours voting. |

**R3, the error split and the magnet rule**

| Document | Quote | Tag | What it forces |
| --- | --- | --- | --- |
| *Wavefront Execution* | Remark "Quasi-Static Geometry": "The gradient above treats K_loc(v) as constant during differentiation" | STRUCTURAL | The Phase II update is not ∇E_c. It is a Petrov–Galerkin, "one-way magnet" update. E_c can rise within an epoch, and the update's Jacobian is non-symmetric (T7). |
| *GIGI Builds* | the torsion chapter (above) | RESONANCE | Learning the connection directly is "energy along connections"; the finite-difference baseline is the poke (T5). |
| *Sameness* §6 | error-budget transfer with separate geometric, linkage, calibration and abstention terms | RESONANCE | The same additive error split as R3. |

**R4, R5, R6, R7**

| Document | Quote | Anchor, tag | What it forces |
| --- | --- | --- | --- |
| *GIGI Thinks* ch. 21 | "Cayley(A) := (I − A)(I + A)⁻¹… cheap orthogonal frame move" | R4, STRUCTURAL [inf] | A Cayley step turns by 2 arctan(θ/2) ≈ θ − θ³/12. Its error is twice that of a chord-based step, so in parallel-scan transport the leading error coefficient should be 1/12 (T4). |
| *Davis Poincaré* | "A_γ = Hol_γ − I"; "holonomy is additive to first order" | R4, RESONANCE | The chord reading. The "+O(τ²)" term in its surgery decomposition has a fixed coefficient set by the reading chosen. |
| *Solves Vol. 4* | "Shell ≠ ensemble… ⟨P⟩_time ≈ 0.66… heatbath ≈ 0.506", "NO_CLOSURE" | R5, RESONANCE [inf] | Swap the integrator or force and keep the mass and target. Aligned biases put the failure in the shared kinetic metric or measurement (T8). |
| *Critical Velocity* | "β depends only on obstacle geometry, not atomic properties" (CV 0.0% over a 12:1 mass ratio) | R5, RESONANCE | The shared part dominates. Residuals from different species should align. |
| *Crouzeix* | the curvature flow (§2.1) | R6, R7, STRUCTURAL (checked) | Non-normality = Yang–Mills energy. |
| *Crouzeix* | "X_θ = Re(e^{−iθ}A) … h(θ) = λ_max(X_θ)" | R7, STRUCTURAL | The same object as R7's numerical-range left edge, with directional gaps added. |
| *Crouzeix* | "K_A = max_{0≤k<n} ‖A^k‖/r^k … exact: some monomial attains it" | R7, RESONANCE | For shift-like lattice operators the exact constant is computable, and may be below 2. |
| *Crouzeix* | exact rational LDL\* certificates, outward-rounded intervals | R1, RESONANCE | The same certification style as R1's inertia counts. Shared tooling. |
| *Navier–Stokes* | "The vorticity peaks at 7.3× the initial value, then decays" | R7, RESONANCE [inf] | A linear 7.3× peak at t = 0.796 under ‖e^{tL}‖ ≤ 2e^{tω(L)} would need a numerical abscissa ω ≳ 1.6. Heuristic, since the flow is nonlinear. |

**WORD ONLY** (recorded so nobody chases them):

- The "168" hits in the books.
- "magnet" in the books (physical magnetism).
- The truncated icosahedron in *Solves 4* (a graph, not S³/2I).
- LQG √(j(j+1)).
- *Kähler* "L9 moment_map".
- *P vs NP* clause density (more clauses make SAT harder; more clues make R2 easier).
- *BSD* "flat directions".
- *Navier–Stokes* κ_g.
- *Critical Velocity* "2v∞".
- *Fisher-Lift* "Hessian of capacity = Fisher metric".

## 4. What the grid forces: tests, best first

| | Test | Prediction | Cost |
| --- | --- | --- | --- |
| **T1** | **Henrici departure along R7's lattice operators.** *Run 19 Nov; see [`henrici-t1-results-2026-11-19.md`](henrici-t1-results-2026-11-19.md).* | **Failed as predicted.** dep does not track max \|Im λ\|. The two-ring relative departure falls (0.224, 0.168, 0.149) while max \|Im λ\| rises (104, 150, 418). The native force tends to normal like h³. The flow identity holds on the real operators to 10⁻⁹. New use: dep/‖G‖_F = 0 exactly for a variational force, a direct score for M0. | done |
| **T2** | **Calibrate SPECTRAL_GAUGE on R1.** Feed GIGI the discretised Berger Levi-Civita connection, and the Poincaré quotient. | Bottom −14/3; spin-6 floor 154/3; floor gap 168/3, within R7's resolution. This explains the verb's indefinite spectrum. | an afternoon |
| **T3** | **Saturation sweep.** Sweep the clue ratio m/m\* on GIGI completion (or the Sudoku endpoint) with noisy data. | Error peaks at 1 and flattens near 2. The *GIGI Thinks* fair run (ratio about 1.1) should recover its wrong cells near 2. | a day |
| **T4** | **Cayley vs exp in parallel-scan transport** (*GIGI Thinks* ch. 21), or rerun Ilse's receipt with the tangent reading. | Leading error coefficient exactly 1/12, twice 1/24. Not seeing it would falsify R4's transfer. | hours |
| **T5** | **Builds torsion test.** Retrain with the connection forced torsion-free; compare with finite differences at several step sizes h. | If torsion-free still wins and σ_FD grows like 1/h, the gain is conditioning (R3), not torsion. If it loses, torsion is real. | a day |
| **T6** | **Spurious kernel and the seat spring** on the *Kähler* T² 6×6 grid. | Wobble modes = dim ker(discrete Δ) − b_k. The seat spring is needed exactly when this is positive. | hours |
| **T7** | **Wavefront Phase II.** Log E_c at every step; compute the numerical range of the update Jacobian. | E_c is non-monotone within an epoch; the Jacobian is non-normal. The cure is to differentiate K_loc, or to accept it as Petrov–Galerkin. | hours |
| **T8** | **Lattice flow endpoint.** Run Wilson flow from the discretised Berger Levi-Civita connection on an S³ lattice. | It leaves along the −14/3 modes and ends at a curved critical point with E > 0 (the meron on the homogeneous family), not S_W = 0. Settles C2 numerically. | a day |
| **T9** | **Unit-clock monotonicity.** Along Ricci flow from (3,1,1), d/dt YM(LC(g_t)) = −\|∇YM\|². | Holds exactly by R6. Also watch whether the j ≤ 2 growth directions disappear as the metric rounds; the round meron is still unstable, so at least one should survive. | minutes (sympy) |
| **T10** | **Cross-certify the selection rule** with the *Cayley* GL(2, F₅) code restricted to SL(2, F₅) ≅ 2I. | Zero trivial-representation multiplicity for j = 1–11; one each at 12, 20, 24, 30. | an hour |

## Scope

- The semantic, statistical and CSP documents are about different objects from gauge fields. Every STRUCTURAL
  tag above was checked against that.
- Nothing here is to be cited in the mathematics papers as support for those documents, or the reverse.
  - The one exception is §2.1: it is ordinary mathematics and can go into my Crouzeix paper.
- Conflicts C1, C3, C4 and C5 concern my statements as written. In several I meant something narrower; those are the sentences to fix.
- The reading agents' extracts are in `$CLAUDE_JOB_DIR/tmp/grid1`–`grid4` (job-local, not committed).
