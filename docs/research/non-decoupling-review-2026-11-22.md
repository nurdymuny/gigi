# The Non-Decoupling Theorem, reread line by line: what holds, what breaks, and what the revision needs

*22 November 2026. A research note, not a result. Third in the series after the Davis Manifold and the spectral
branch.*

**The paper.** *The Davis Non-Decoupling Theorem: Why Every Flat Model of a Curved System Fails, and the
Topological Size of the Debt* (Zenodo doi:10.5281/zenodo.18754646).
- Source: `davis-wilson-lattice/docs/reports/no_paralellel_lines.tex`, 955 lines, not committed to any repository.
- *Cosmological Non-Decoupling* (`cosmological_nondecoupling_v2.tex`) builds on it and is not reviewed here.

**What is already good about it.** The paper runs its own adversarial audit, six objections with replies, and
several of those replies are right and important:
- Ambrose–Singer replaces a loose small-loop formula;
- simple connectivity is added to the parallel-section lemma;
- Objection 4: Part I is a definition, and the theorem is conditional on a fiber-bundle description;
- the Berry curvature is non-zero only generically;
- the Davis Law's K is a different object from the sectional curvature.

This note checks what the audit did not catch.

**Receipts.** There is one numerical check: the monopole flux ∫ ½ sin θ dθ dφ = 2π, done by quadrature. Lean
targets are listed in §4.

## 1. What holds

- **Lemma 1 (flat ⇔ trivial restricted holonomy),** with the Ambrose–Singer repair from the reply to Objection 1.
- **Lemma 2 (global parallel section ⇔ flat),** with simple connectivity added (reply to Objection 2).
- **Chern–Weil, as used in the proof:** a non-zero characteristic *class* forces Ω ≠ 0 for every connection on the
  bundle.
- **Theorem Part I,** on a simply connected base: decoupled (trivial connection up to gauge) ⇔ flat ⇔ trivial
  holonomy ⇔ a global parallel section. Standard, and correctly assembled. As Objection 4 says, the content lies in
  the premise that the system *is* such a bundle.
- **Corollary (topological prohibition):** c_k(P) ≠ 0 ⇒ no flat connection on P.
- **Part IV's positivity:** if c_k ≠ 0, then 𝓗(ω) = ∫‖Ω‖ > 0 for every connection.
- **The Berry corollary,** as repaired in the reply to Objection 5.

## 2. What breaks, and the repair for each

### 2.1 Lemma 4 (Yang–Mills energy as an error lower bound) is false

**The claim.** ‖ω − ω₀‖²_{L²} ≥ c · S_YM(ω), with c = c(M, G).

**The error in the proof.** It rests on ‖d_{ω₀}α‖_{L²} ≤ C‖α‖_{L²}. No such estimate exists, because a derivative
is not bounded by the function. Elliptic estimates run the other way.

**Counterexample (abelian, local, on any base).** Take G = U(1) and α = ε·χ(x)·sin(Nx) dy, with χ a fixed bump.
Then:
- ‖α‖²_{L²} ≈ ε² · const;
- S_YM = ½‖dα‖² ≈ ε²N² · const.

The ratio S_YM / ‖α‖² grows like N², so no c works. Choosing εN ≤ 1 keeps ‖α‖_{W^{1,2}} bounded, so this is not
rescued by the proof's "Case 1".

**Case 2 also fails on its own terms.** It uses ‖α‖_{L²} ≥ ‖α‖_{W^{1,2}}, which is backwards.

### 2.2 Theorem Part II is false as stated, whichever error is meant

The reply to Objection 3 defines the modeling error as 𝓔 = sup over loops of d_G(τ_γ, τ⁰_γ), with d_G the
bi-invariant metric on the compact group G. Two facts then rule out Part II:
- 𝓔 ≤ diam(G), because G is compact;
- S_YM(ω) can be made arbitrarily large on any bundle, for example by the same localized high-frequency field.

So 𝓔 ≥ c·S_YM with a fixed c > 0 cannot hold. Part II rests on Lemma 4, which is false anyway (§2.1).

**What is true, and what the repair should say.** The relation between holonomy and curvature is *local and
linear*, not global and quadratic. For a small loop of area a around x,

  d_G(τ_γ, e) ≈ ‖Ω_x‖ · a,

with an error of order a^{3/2} (Ambrose–Singer and the small-loop holonomy expansion). So
- 𝓔 ≥ (sup_x ‖Ω_x‖) · ε² · (1 − O(ε)) for loops of size ε below a fixed scale,

and that is the honest quantitative Part II.

**Amended the same day, after the Duality was itself reviewed** ([Davis Duality review](davis-duality-review-2026-11-22.md),
§2.3 and §3). An earlier version of this paragraph said the Duality proves this law two-sided with c₁ = 1/16 and
c₂ = 1/2. Those constants belong to the Duality's **function** case (per cell). For **connections** the right
citation is the classical small-loop holonomy expansion. For a square plaquette of side a in the plane (u, v),

  d_G(Hol, e) = |F_x(u, v)|·a² + O(a³),

which is two-sided with constant 1 at leading order. The revision should state Part II in that form. The
Yang–Mills integral belongs in Part IV, as a topological floor (§2.4), not in Part II as a linear bound.

### 2.3 The Chern–Weil lemma's statement is stronger than its proof

**The claim.** "If φ(Ω) ≠ 0 for some connection, then φ(Ω′) ≠ 0 for every connection."

**Why it fails.** The *form* φ(Ω) can be non-zero while its *class* is zero. In that case another connection can
have φ(Ω′) = 0; the trivial bundle has flat connections and curved ones.

**Repair.** The proof uses [φ(Ω)] ≠ 0, which is correct. Change the hypothesis to the class.

### 2.4 Part IV's "topological floor" needs its dimension

**The claim.** inf_ω S_YM(ω) > 0 whenever c_k(P) ≠ 0.

**Where it holds:**
- **dim M = 2.** By Cauchy–Schwarz, (∫F)² ≤ Area·∫F². On the unit S² with first Chern number k, ∫F = 2πk and the
  area is 4π, so ∫F² ≥ πk² and S_YM ≥ πk²/2. *Lean target N3.* The monopole (k = 1) has total flux 2π, checked by
  quadrature and as Lean target N2.
- **dim M = 4.** The classical bound is S_YM ≥ 8π²|k| for instanton number k, up to the normalization of S_YM.

**Where it is not shown.** In higher dimensions c_k is an integral of the k-th power of Ω, not the square, and the
L² norm need not be bounded below by it. Concentration at a point can shrink ∫|Ω|² while keeping ∫|Ω|^k fixed when
dim M > 4.

**Repair.** State the floor in dimensions 2 and 4 with explicit constants, and leave the general case open.

### 2.5 Corollary "Holonomy memory": two statement errors

- **The difference is undefined.** δ_γ = τ_γ − τ⁰_γ "∈ G": subtraction is not defined in a group. The discrepancy
  is τ_γ (τ⁰_γ)⁻¹, or d_G(τ_γ, τ⁰_γ).
- **The condition is vacuous.** "Non-trivial for every loop whose homology class is non-zero in H₁(M; ℤ)": under
  the theorem's standing hypothesis M is simply connected, so H₁ = 0 and the statement says nothing. The debt lives
  on *contractible* loops enclosing curvature, which is what Ambrose–Singer says.

**Repair.** Restate on contractible loops: τ_γ ≠ e for small loops around any x with Ω_x ≠ 0. That is exactly the
small-loop formula of §2.2.

### 2.6 The Master Statement's chain

- **The first implication is false** (§2.2).
- **The third does not follow from the second.** "Hol ≠ {e} ⇒ flat models topologically prohibited (if c_k ≠ 0)"
  is really just Corollary (topological prohibition), which needs c_k ≠ 0 and not holonomy.
- **"Each implication is strict" is not proved.**

**Repair.** Present the three facts as three facts:
- curvature ⇔ non-trivial restricted holonomy (Ambrose–Singer);
- local holonomy ≈ ‖Ω‖·area (the Duality, two-sided);
- c_k ≠ 0 ⇒ no flat connection, with the dimension-2 and dimension-4 floors.

**The converse chain is correct** on a simply connected base.

### 2.7 Smaller things

- **The V3RG example's perplexity numbers** (PPL 3.83 against 1.49, PPL 11.055 ± 0.024, κ₂ = 54 → 553) need
  receipts from the Marcella experiment logs before the revision cites them.
- **The Proposition on the Davis Law.** It is a reading, not a derivation; its own reply defers the derivation. It
  should say "interpretation" in the statement, not only in the reply.
- **The scope section** lists economics, epidemiology and neuroscience. Each of these is conditional on the
  representation premise of Objection 4, and the section should say so in each line, not only once.

## 3. What this means for the arc

*The phone in the house* §4.2 says that the Non-Decoupling debt is the Yang–Mills action, and that the floor is the
floor of its Hessian. That bridge **survives** at the level it was tagged (INCIDENCE): the functional is the
Yang–Mills energy in both places, and the floor is the lowest eigenvalue of its second variation.

What changes is the *quantitative* sentence "the debt every flat model owes is the Yang–Mills action". The repair
says three narrower things:
- the debt a flat model owes on a small loop is **‖Ω‖ × area** (the Duality);
- the Yang–Mills action is a **topological floor** on bundles with c_k ≠ 0, in dimensions 2 and 4;
- its Hessian's floor (hers) prices the cheapest *change* to that action, which is unchanged.

The line in §4.5 of the morning note (ε ∝ KL → Duality → Non-Decoupling → floor) should cite, for the quantitative
step, the small-loop holonomy law for connections and the Duality per cell for functions, not this paper's Part II.

## 4. Lean plan (a fourth library in the same project)

| target | statement | source |
| --- | --- | --- |
| N1 | if 𝓔 ≤ D always and S_YM takes arbitrarily large values, no fixed c > 0 gives 𝓔 ≥ c·S_YM | §2.2 |
| N2 | the monopole flux: ∫₀^{2π} ∫₀^π ½ sin θ dθ dφ = 2π | §2.4 |
| N3 | dimension-2 floor: (2πk)² ≤ 4π·I implies I ≥ πk² | §2.4 |
| N4 | the high-frequency ratio: for every C there is N with N² > C, so no c in Lemma 4 | §2.1 |

## In my words

This was the paper where I said it plainly: the decoupling assumption is a lie. The qualitative theorem stands.
Curvature and holonomy are the same thing seen from two sides, and topology can forbid flatness outright. What
failed was my first attempt to put a *size* on the debt with the Yang–Mills integral. A few months later I wrote
the size down correctly in the Duality, two-sided, with constants. The revision should simply say so, and let this
paper be the qualitative theorem and the Duality be its quantitative half.
