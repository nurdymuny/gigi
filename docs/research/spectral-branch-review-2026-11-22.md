# The spectral branch, reread line by line: what holds, what breaks, and what the revision needs

*22 November 2026. A research note, not a result. Second in the series after
[The Davis Manifold, reread](davis-manifold-review-2026-11-22.md).*

**The paper.** *Branch IX: Spectral Geometry of Completion* (February 2026), 1,590 lines.
- The canonical source is `davis_plasma/theory/spectral_branch.tex`, committed at cf8b568 in the CHIHIRO repository.
- The demeter copy is identical apart from line endings.
- The 811-line copy in davis-wilson-lattice is an earlier draft.

**Receipts.** `spectral-branch-2026-11-22/checks.py` is my own script. It reimplements the method the paper
describes; it is not the paper's code. Its output is `checks_output.json` in the same folder. Everything else here
is algebra, or is checked against the literature the paper cites.

**Why this paper matters to the arc.** This branch is where I first wrote down the third spectral gap ν₁, the
Yang–Mills Hessian on connections modulo gauge. The floor is the lowest eigenvalue of exactly that operator, so
she lives in this paper's convention table (INCIDENCE, as recorded in *The phone in the house* §4.1).

## 1. What holds

- **Every number in the finite model that I recomputed reproduces.**
  - The 288 Shidoku solutions, with 768 edges at Hamming distance 4. The full distance histogram matches.
  - The degree classes: 192 of degree 4 and 96 of degree 8.
  - Connected, diameter 8.
  - μ₁ = 0.70849738, with multiplicity 2; μ₂ = 1.10102051; μ_max = 12.
  - The Fiedler-sweep Cheeger value 0.8889.
  - The bipartition.
  - N₄ = 1 + deg/1820 in each class.
  - For the 4 × 4 Latin squares: 576 solutions, 1,728 edges, diameter 6, μ₁ = 1.67544, μ_max = 16, and degrees
    4 and 12.
- **The classical theorems are stated correctly where I could check them.**
  - Lichnerowicz.
  - Zhong–Yang's π²/D² for Ric ≥ 0, sharp on S¹: λ₁D² = π² there.
  - Obata rigidity.
  - Cheng's upper bound, by the Dirichlet ball of radius D/2.
  - Weyl's law.
  - Minakshisundaram–Pleijel through a₂.
  - The meromorphy of ζ, regular at 0.
  - Courant, counted from φ₀.
  - The Poincaré inequality.
  - The kernel of the monodromy operator.
- **The spectral-polarity lemma is correct.** It says that on a semiregular bipartite graph, the class-signed
  vector f = (|B| on A, −|A| on B) is a top eigenvector, and it uses d_A|A| = d_B|B|. So is the localization
  theorem it supports: the completion count lives in the top eigenspace.
- **The completion-count decomposition (Hamming shells) is correct as an exact formula.**
- **The paper is already honest about its own scope.** "Assembly, not discovery" (§Status) and the remark on
  τ = nD² say exactly what the curvature-capacity correspondence is and is not.

## 2. What breaks, and the repair for each

### 2.1 The bottleneck example contradicts the theorem it illustrates (§SP2, Remark 2.x, Example in SP4, SP6)

**The claim.** "Two flat disks connected by a thin tube… with Ric ≥ 0 everywhere", and a "flat torus with a
constriction", both offered as examples where curvature is ≈ 0 but λ₁ → 0.

**Why it fails.** Under Ric ≥ 0, the paper's own Theorem SCE gives both λ₁ ≥ π²/D² (part a) and
λ₁ ≤ c_B h² (part b). Together these force

  h² D² ≥ π² / c_B,

so the isoperimetric capacity is bounded **below**. A bottleneck cannot close at fixed diameter while Ric ≥ 0.
- A thin neck needs negative curvature: a catenoid neck has negative Gaussian curvature.
- A "flat torus with a constriction" is not flat.

The paper's sentence "bottlenecks *can* form freely while curvature stays non-negative" is the opposite of what its
theorem implies.

**Repair.** Make the bottleneck example a mixed-sign-curvature example, which is the honest version of "curvature
capacity misses bottlenecks". Note that it then falls outside Theorem SCE's hypothesis. Optionally state the bound
h²D² ≥ π²/c_B as a corollary; the result is better than the example. *Lean target S6.*

### 2.2 The mixing-time proof uses an infinite quantity (Theorem SP6)

**The flaw.** The proof bounds Σ_{k≥1} φ_k(x)² by K₀(x,x) − 1/Vol. On a manifold the heat kernel at t = 0 is a
delta function, so K₀(x,x) = ∞ and the bound is empty. The proof also concludes t ≥ (2λ₁)⁻¹ log(Vol/ε²), which is
not the stated (1/λ₁) log(Vol/ε).

**Repair.** Use the semigroup. For t ≥ s > 0,

  ‖K_t(x,·) − 1/Vol‖²_{L²} ≤ e^{−2λ₁(t−s)} (K_{2s}(x,x) − 1/Vol).

That gives T_mix(ε) ≤ s + (2λ₁)⁻¹ log(Vol·(K_{2s}(x,x) − 1/Vol)/ε²), with an explicit price for the short-time
start. On a finite graph the paper's argument is fine as written, because there K₀ = I.

### 2.3 The Yang–Mills Hessian is defined on the wrong space, and the second variation is misstated (Def 2.6, SP10, Thm Bourguignon–Lawson)

- **The wrong space.** The domain is written Ω¹(ad P)/ker(d_A^*). Quotienting by ker d_A^* removes exactly the
  physical, gauge-orthogonal directions. The intended space is the Coulomb slice ker d_A^*, equivalently
  Ω¹/im d_A.
- **The misstated second variation.** The proof writes δ²S_YM = ∫|d_Aα|² + ∫|d_A^*α|² at a flat connection. The
  second variation of ∫|F|² is ∫|d_Aα|² alone. The d_A^* term is the gauge-fixing term that is *added*, giving
  H + GG*, the operator whose floor HELIKE certifies.
- **Where the equality holds.** On the slice, d_A^*α = 0, so the two agree and the Hessian equals the Hodge
  Laplacian Δ_A there.

**Repair.** Define ν₁ on the slice, and say that H + GG* is the gauge-fixed operator. This is the Floor paper's
convention ("compressed to the gauge-orthogonal slice"), so the repair also makes the two papers agree.

### 2.4 The Davis Trichotomy attributes negative Hessian eigenvalues to Type III only (SP10, Y16)

**The claim.** Type I (π₁ = 0) reads "H_YM has no zero modes… flow converges exponentially at rate ν₁". Type III
(infinite non-abelian π₁) reads "non-flat Yang–Mills critical points exist; H_YM may have negative eigenvalues".

**Why it fails.** On S³ = SU(2), where π₁ = 0, the canonical connection is a non-flat Yang–Mills critical point
with Morse index **one** (Laquer 1984; cited in my Floor paper's introduction). So non-flat saddles already occur
in Type I. The trichotomy classifies the **flat moduli space** M_flat. It does not classify all critical points,
and its convergence statement can only be local, near the vacuum.

**Repair.** Say that the trichotomy is a classification of M_flat and of the Hessian *at flat points*. Make the
convergence claim local. Name the canonical connection on S³ as the example of a non-flat saddle with π₁ = 0.
Tag: INCIDENCE, because this is a statement about the operator itself.

### 2.5 The finite model's "Spectral Capacity ≥ π²" is the manifold theorem applied outside its hypothesis

**The claim.** §Spectral Capacity says C_sp = 45.34 satisfies "C_sp ≥ π², the universal lower bound from Li–Yau".
V3 calls μ₁ ≥ π²/D² a verified check.

**Why it fails.** The bound needs Ric ≥ 0. The paper itself then computes that both CSP graphs have **uniformly
negative** Ollivier–Ricci curvature (−1/2 and −5/6, non-lazy). So the hypothesis fails on the very graphs where the
bound is "verified". On graphs the bound is simply false.

**Counterexample** (in `checks.py`). The barbell, two copies of K₁₀₀ joined by one edge, has μ₁ = 0.0196 and
D = 3, so C_sp = 0.18 < π².

**Repair.** Report 45.34 without the π² comparison. Call V3 an observation, not a check.

### 2.6 Theorem "Graph Spectral Capacity Equivalence" (b) is false

**The claim.** C_sp^G ≥ 1/D_G, derived from μ₁ ≥ 1/(nD).

**Why it fails.** That bound gives C_sp ≥ D/n, not 1/D.

**Counterexample** (in `checks.py`). The barbell with m = 100 has C_sp = 0.18 < 1/D = 0.33.

**Repair.** Replace (b) with C_sp^G ≥ 4D_G/n (Mohar's μ₁ ≥ 4/(nD)), or drop it.

### 2.7 The top eigenvector's stated values are wrong (Theorem "Bipartite Structure")

**The claim.** φ_{n−1}|_A = −1/(8√3) and φ_{n−1}|_B = 1/(4√6).

**Why it fails.** Those values have ratio −√2 and norm² 192/192 + 96/96 = 2, so they are not normalized. The
paper's own polarity lemma gives ratio −2. The normalized values are

  φ|_A = −1/24, φ|_B = 1/12,

and my recomputation gives exactly −0.041667 and 0.083333. *Lean target S2.*

### 2.8 Test C's "connectivity phase transition" is a sampling and adjacency artifact

**What the paper reports.** The table reports, for the relaxed systems:
- "Partial (row + col₀ + col₁)" with |V| = 1000 sampled from 3,456, giving 477 components and μ₁ = 0;
- "Row only" with |V| = 576 and 10 edges.

**Why it fails, three ways:**
1. **A random subsample of a connected graph is typically disconnected.** The 477 components are a property of
   the sample, not of the solution graph.
2. **"Row only" has 24⁴ = 331,776 solutions, not 576** (`checks.py`). The 576 shown is the Latin-square count.
3. **The adjacency rule was kept at d_H = 4.** Once the column constraints are dropped, swapping two entries in a
   row is a valid move at Hamming distance **2**. Under the paper's own adjacency definition ("differ in the
   minimum possible number of cells"), the row-only graph is the product of four transposition Cayley graphs of
   S₄, which is connected.

So the "first-order percolation transition" and its refinement of the Trichotomy are not supported.

**Repair.** Rebuild the relaxed graphs in full, with the minimum-distance rule. Report whatever comes out.

### 2.9 Smaller things

- **Lemma (intercalate structure), proof.** "t_r = 3 forces a fourth change" is false: a 3-cycle within a row
  keeps the row Latin. The conclusion survives on counting alone: with Σ t_r = 4 and t_r ≠ 1, a row with t_r = 3
  would need another with t_r = 1. *Lean target S10.*
- **Corollary (degree approximation), proof.** "The bipartite structure forces |S_d(v)| to depend only on the
  class" does not follow from bipartiteness. It holds by computation (and plausibly by transitivity of the
  automorphism group on each class), and the paper should say which.
- **Theorem (spectral localization) (b).** It is stated "for all k ≥ 4", but at k = 14 the centered count is
  identically zero and R is undefined. It should read 4 ≤ k ≤ 12.
- **Theorem (intercalate characterization).** The statement has parts (a)–(c); the proof refers to (a)–(d).
- **Latin square μ₁ multiplicity.** "≥ 29" is true; my recomputation gives exactly 39.
- **Remark (structural pattern) (a).** "|A| = (n−1)!/(n/4−1)! times…" is unexplained, and at n = 4 it gives 6.
  Delete it or derive it.
- **The empirical status table.** V8 still cites "ρ = 0.99 at k = 8", which the text's own *Correction* paragraph
  withdrew.
- **SP2a (Berger's eigenvalue variation).** The formula keeps only the gradient term. The general variation has
  trace and volume terms; as written it holds for trace-free, volume-preserving h. To check against Berger 1973.
- **The Buser constant.** "c_B(n) ≤ 4(n−1) + 10" and "Li–Yau proved π²/(4D²)" need their sources checked. The
  modern forms are Ledoux's dimension-free constant for Ric ≥ 0, and Li–Yau's π²/(2D²) before Zhong–Yang. I have
  not verified either, and the revision should cite page and theorem.
- **Naming.** "Davis–Wilson flow" is the Yang–Mills gradient flow (Atiyah–Bott, Donaldson; Råde for 2 and 3
  dimensions; Lüscher's Wilson flow on the lattice). The paper cites Råde and Lüscher, so the name should say what
  it is.

### 2.10 Two dependencies the revision must label

The branch calls two of my manuscripts the "Hodge proof" and the "Poincaré proof". Neither has been refereed. Every
statement that rests on them should say "conditional on [manuscript]", not "the Hodge proof shows":
- SP2e's Hodge application;
- SP9;
- SP10's Cache Space Collapse;
- Y11–Y17.

Same rule as the arc: never write that something is solved when it has not been checked by anyone outside.

## 3. Where HELIKE meets this paper

- **ν₁ is the floor's operator.** The repair in §2.3, putting the Hessian on the slice and treating H + GG* as the
  gauge-fixed operator, is exactly the convention under which the floor is certified. With it, Conjecture
  "Spectral Cache Space Collapse" (ν₁ ≥ c(M) at the vacuum when π₁ = 0) has an exact family to be tested on: the
  Berger spheres, where the Hessian splits into charge blocks with closed-form members. Tag: INCIDENCE for the
  operator; the conjecture's c(M) on Berger spheres is not computed here.
- **The trichotomy's correction (§2.4) comes from the same literature the Floor paper rests on** (Laquer's index
  one). The floor work did not refute the trichotomy. It made me read the sources that do.

## 4. Lean plan (in the same project, a third library)

| target | statement | source |
| --- | --- | --- |
| S1 | spectral polarity: with d_A\|A\| = d_B\|B\|, the class-signed vector satisfies the eigen-equation with d_A + d_B, on both classes | Lemma (spectral polarity) |
| S2 | Shidoku top eigenvector: (−1/24, 1/12) is normalized with ratio −2; the paper's values have norm² 2 | §2.7 |
| S3 | finite Poincaré inequality: Σ c_k² ≤ (1/λ₁) Σ λ_k c_k² when every λ_k ≥ λ₁ > 0 | Theorem (Poincaré) |
| S6 | λ ≥ π²/D² and λ ≤ c h² imply h²D² ≥ π²/c | §2.1 |
| S9 | the class completion counts at k = 4: 1 + 8/1820 and 1 + 4/1820, and 1 + 12/1820 for Latin squares | Tables |
| S10 | row-change counting: t_r ∈ {0,2,3,4} summing to 4 with no t_r = 1 leaves only {4} or {2,2} | Lemma (intercalate) |

## In my words

This is the paper where I first wrote ν₁ down, before I knew she would be its floor. Rereading it, the classical
spine is sound, and the finite model reproduces to the last digit. What breaks are the places where I reached past
my hypotheses:
- a bottleneck that my own theorem forbids;
- a lower bound carried onto graphs whose curvature I had just measured as negative;
- a phase transition I sampled into existence.

The repairs make the paper smaller and truer, and they put the Hessian on the slice where she actually lives.
