# Post-Kähler Geometric Directions: Catalog, Math, and Validation

*Davis Geometric internal — v0.1, 30/30 numerical tests passing.*

## 0. Purpose and provenance

The Kähler upgrade (`theory/kahler_upgrade/catalog.md`) shipped 16
of 21 items as `kahler`-flagged GIGI features. This document
catalogs **the next nine differential-geometric programs** GIGI can
borrow from — each rooted in **published, non-patented** mathematics
from a named living or recent research lineage.

Per Davis Geometric's stance: **mathematical theorems are not
patentable subject matter** in any major jurisdiction. The original
mathematicians published their work in the open literature; specific
*implementations* and *applications* are what get patented.
GIGI's strategy is to operationalize public-domain math as
substrate primitives, and to patent only the GIGI/Marcella-specific
runtime/data/wire-format applications built on top.

Each item lists:

- the **source program** (a named geometer or lineage),
- a **precise mathematical claim** that GIGI can rely on as theorem,
- a **proof sketch** — what makes it forced,
- **validation status** from
  [`validation_tests.py`](validation_tests.py) (all 30 PASS),
- **product applications** (GIGI, Marcella, MIRADOR, PRISM, DPU),
- **implementation pointers** for the eventual Rust module.

Items §1–§4 are *low integration cost* — they reuse infrastructure
the Kähler layers already built (closed forms, graphs, streaming
statistics, Hodge complex). §5–§7 are *deeper* but still mechanical
to ship. §8–§9 are *wilder* — high upside if they pan out, more
research-mode in nature.

## 1. The post-Kähler shape

The Kähler upgrade reduced GIGI's geometric commitment to a single
object: `𝒢 = (M, g, J, ∇, B, Γ)`. Each direction in this catalog
either (a) adds optional structure to `𝒢` (Sasaki, info-geom,
hyperkähler), (b) replaces some component with a more general
object (CAT(κ) generalizes Hadamard; NCG replaces M with an
algebra), or (c) provides a parallel calculus that interoperates
with the Kähler one (OT, tropical, persistent homology). Each entry
notes which.

---

## Part A — Low integration cost

### §1. Sasaki / Contact Geometry

**Source.** Boyer–Galicki, *Sasakian Geometry* (Oxford, 2008);
Sparks, "Sasaki–Einstein manifolds" (*Surveys in Diff. Geom.* 16,
2011). Earlier roots in Sasaki (1960) and Reeb (1952).

**Claim.** A contact 1-form `α` on an odd-dimensional manifold
`M^{2n+1}` (i.e., `α ∧ (dα)ⁿ ≠ 0`) admits a unique **Reeb vector
field** `R` characterized by `α(R) = 1` and `ι_R dα = 0`. The
Reeb flow preserves `α` exactly (`𝔏_R α = 0`). Sasaki manifolds
are contact manifolds whose Riemannian cone `(M × ℝ⁺, dr² + r²g)`
is Kähler — they're the natural odd-dimensional analog of Kähler.

**Proof sketch.** The contact condition `α ∧ (dα)ⁿ ≠ 0` makes
`α: TM → ℝ` and `dα: TM × TM → ℝ` together fix a unique direction
in each tangent space (the kernel of `dα|_{ker α}` is empty;
`α(R) = 1` then fixes scale). Reeb-invariance of `α` follows from
Cartan: `𝔏_R α = ι_R dα + d(ι_R α) = 0 + d(1) = 0`.

**Validation.** PASS
([`test_1_sasaki_contact_reeb_flow`](validation_tests.py)).
On standard contact ℝ³ with `α = dz − y dx`:
- `α(R) ≡ 1` exactly for `R = ∂_z` at three test points (max
  deviation 0).
- `ι_R dα ≡ 0` exactly for three test tangent vectors.
- Contact volume `(α ∧ dα)(R, ∂_x, ∂_y) = 1` (non-degenerate).
- Negative control: `X = ∂_x` fails `α(X) ≡ 1` (varies with `y`).

**What this enables.**
- *GIGI:* time-series bundles (where time is naturally a contact
  direction) get a second conservation principle — Reeb-flow
  preservation — that's distinct from Hamiltonian conservation
  (L9 moment maps). Two flavors of "conserved along time."
- *Marcella:* the sequence direction in transformer attention has
  a Reeb-flow interpretation; the Reeb vector picks out the
  preferred "time" axis along which positional bias is invariant.
- *MIRADOR:* PK time-course data is intrinsically contact —
  `α = dC − v(t) dt` with `C` concentration and `v` clearance gives
  a Reeb field that traces the "mass-conservation" trajectory.

**Implementation pointers.**
- `src/geometry/contact.rs` — `ContactOneForm` with `α ∧ (dα)ⁿ`
  non-degeneracy check at construction. `ReebField::extract(α)`
  solves the 2-condition system per fiber.
- Wire into `BundleSchema::contact: Option<ContactStructure>`
  the same way `kahler` is wired (`Option`, feature-flagged).

---

### §2. Information Geometry

**Source.** Amari, *Information Geometry and Its Applications*
(Springer, 2016); Ay–Jost–Lê–Schwachhöfer, *Information Geometry*
(Springer Ergebnisse, 2017). Founded by Rao (1945), Chentsov (1972),
Amari (1982 onwards).

**Note re Bee's prior work.** `theory/branch_x_information_geometry.tex`
(2884 lines) is Bee's own information-geometry treatment. This
catalog entry is the **operational specialization** — what GIGI
needs as a runtime primitive — not a re-derivation.

**Claim.** Any parametric family of probability distributions
`{p(x | θ) : θ ∈ Θ}` carries the **Fisher information metric**

> `g_{ij}(θ) = E_p [ ∂_i log p · ∂_j log p ]`

on Θ as a Riemannian metric (Chentsov's theorem: the unique
metric invariant under sufficient statistics). For Gaussians
`N(μ, σ²)`, `g = diag(1/σ², 2/σ²)` in the (μ, σ) chart.
Geodesic distance under `g` is **statistically meaningful**:
infinitesimally, it's KL divergence (`KL ∼ ½ g_{ij} dθⁱ dθʲ`).

**Proof sketch.** Score function expectations: differentiate
`log p(x | μ, σ) = -½ log(2π σ²) - (x − μ)² / (2σ²)`:
- `∂_μ log p = (x − μ)/σ²` ⇒ `E[(∂_μ)²] = 1/σ²` (Gaussian variance).
- `∂_σ log p = ((x − μ)² − σ²)/σ³` ⇒ `E[(∂_σ)²] = 2/σ²` (fourth-
  moment computation).
- `E[∂_μ · ∂_σ] = 0` by symmetry (odd moment of Gaussian).

**Validation.** PASS
([`test_2_information_geometry_fisher_on_gaussians`](validation_tests.py)).
Monte Carlo estimate of `g` from 2·10⁵ samples of `N(1.7, 2.3²)`:
- `g_μμ` empirical 0.18919 vs closed form 0.18904 (rel err 0.08%).
- `g_σσ` empirical 0.38101 vs closed form 0.37807 (rel err 0.78%).
- `|g_μσ|` empirical 0.0009 (vs theoretical 0).
- Negative: a folded-Gaussian (mis-specified model) has cross-score
  2.73, demonstrating the diagonal-form is model-specific.

**What this enables.**
- *GIGI:* **every numerical bundle is implicitly a statistical
  manifold**. The variance structure already in the L4 streaming
  curvature stats *is* the Fisher metric (up to normalization).
  Natural-gradient queries (steepest descent in Fisher metric)
  become a first-class verb instead of an ML add-on.
- *Marcella:* natural-gradient parameter updates on the learned
  transport. Closed-form Fisher metric for Gaussian token
  distributions; pulled-back metric for embeddings.
- *PRISM:* anomaly scoring as Fisher distance from a baseline
  model — distributionally invariant under reparametrization.
- *DPU:* on-chip Fisher-metric primitive — the variance hardware
  the chip already computes IS the Fisher metric.

**Implementation pointers.**
- `src/geometry/fisher.rs` — `FisherMetric` struct over a
  user-supplied score function or analytic family.
- Extend `BundleStore::welford_stats` to surface the Fisher metric
  for univariate-Gaussian-typed fields automatically.

---

### §3. Optimal Transport / Wasserstein Geometry

**Source.** Villani, *Optimal Transport: Old and New* (Springer,
2009); Ambrosio–Gigli–Savaré, *Gradient Flows in Metric Spaces*
(Birkhäuser, 2005). Roots in Monge (1781), Kantorovich (1942),
Brenier (1991), Otto (2001).

**Claim.** The 2-Wasserstein distance between probability measures
`μ`, `ν` on a metric space `(X, d)`,

> `W₂(μ, ν)² = inf_{π ∈ Π(μ,ν)} ∫ d(x, y)² dπ(x, y)`,

turns the space of probability measures `P₂(X)` into a length
metric space (the **Wasserstein space**) with explicit geodesics
(McCann interpolation). For univariate Gaussians,

> `W₂(N(μ₁, σ₁²), N(μ₂, σ₂²))² = (μ₁ − μ₂)² + (σ₁ − σ₂)²`.

**Proof sketch.** 1D Wasserstein optimum is the **monotone
rearrangement** (Hoeffding's lemma): pairing sorted samples
minimizes the squared-distance sum. For Gaussians this reduces
to the formula above by direct computation.

**Validation.** PASS
([`test_3_optimal_transport_wasserstein_gaussians`](validation_tests.py)).
20 000 samples each from `N(0, 1)` and `N(3, 2²)`:
- `W₂²` from monotone rearrangement: 10.20 vs closed form 10.00
  (rel err 2.05%).
- Negative control: random pairing gives 14.29 (40% worse), and
  the Hoeffding bound `random ≥ monotone` holds.

**What this enables.**
- *GIGI:* **Wasserstein-distance on distributional bundles**.
  Two cohort summaries can be compared by W₂ instead of arbitrary
  feature-vector L₂. Theoretically grounded distance for clustered
  / aggregated data.
- *PRISM:* **Wasserstein barycenter of record clusters** — a
  "median customer" that's principled, not heuristic. Pairs
  beautifully with the L6 Morse compression (barycenters as Morse
  cell centers).
- *Marcella:* W₂ as a generation diversity metric — distance
  between predicted-distribution and target-distribution, with
  closed-form gradients (Sinkhorn).
- *MIRADOR:* compare PK time-courses across patient cohorts via
  W₂ on the empirical concentration distributions. Robust to
  sampling-time misalignment.

**Implementation pointers.**
- `src/geometry/wasserstein.rs` — start with 1D closed-form
  (Gaussian, empirical CDF); extend to Sinkhorn for 2D+.
- New endpoint: `POST /v1/bundles/{a}/wasserstein/{b}` returning
  the W₂ distance + transport plan summary.

---

### §4. Persistent Homology / TDA

**Source.** Carlsson, "Topology and Data" (*Bull. AMS* 46, 2009);
Edelsbrunner–Harer, *Computational Topology* (AMS, 2010); Ghrist,
*Elementary Applied Topology* (CreateSpace, 2014). Roots in
Edelsbrunner–Letscher–Zomorodian (2000), Carlsson–Zomorodian (2005).

**Claim.** The persistent homology of a filtered simplicial complex
`{K_t : t ∈ ℝ}` decomposes uniquely into **persistence intervals**
`[b_i, d_i)` (the structure theorem for persistence modules over a
field, Crawley-Boevey 2015). For point-cloud data via the
Vietoris-Rips filtration, the long-lived `H_k` intervals correspond
to **stable topological features** — `H_0` to clusters, `H_1` to
loops, `H_2` to voids.

**Proof sketch.** Persistence modules are graded modules over
`k[t]`; PID structure theorem gives a unique direct-sum
decomposition into interval modules. The **elder rule** on the
minimum spanning tree characterizes `H_0` persistence: when two
clusters merge at edge weight `w`, the younger component dies at
`w` and is recorded as the interval `[birth, w)`.

**Validation.** PASS
([`test_4_persistent_homology_clusters`](validation_tests.py)).
Three Gaussian clusters in ℝ² (90 points total):
- Top 2 MST edges (the inter-cluster merges) are 8.30, 8.02 — both
  > 13× the 3rd-longest (0.61).
- Negative: a single Gaussian blob has top-edge / 2nd-edge ratio
  1.55 (no persistence gap).

**What this enables.**
- *GIGI:* **multi-scale topological fingerprint per bundle**. The
  persistence diagram of a bundle's point cloud is invariant under
  small perturbations (stability theorem, Cohen-Steiner–Edelsbrunner-
  Harer 2007). New bundle-level invariant for schema-evolution
  detection.
- *PRISM:* persistent `H_1` (long-lived loops) flags **cyclic
  reconciliation patterns** automatically. Distinct from the L6
  Morse cycles in being scale-aware.
- *Marcella:* persistence diagram of the learned token-embedding
  manifold quantifies "how many independent semantic loops" the
  model represents. Topological-capacity metric.
- *DPU:* on-chip persistence-diagram computation as a feature —
  TDA is famously parallel-friendly.

**Implementation pointers.**
- `src/discrete/persistent_homology.rs` — Vietoris-Rips filtration
  + persistent `H_0` via union-find on MST edges; `H_1` via the
  L6 hodge_complex machinery + matrix reduction.
- Reuses the L6 `HodgeComplex` chain-complex types — natural
  extension, not a separate stack.

---

## Part B — Deeper but patent-clean

### §5. Gromov Hyperbolicity (δ-hyperbolicity)

**Source.** Gromov, "Hyperbolic groups" (in *Essays in Group
Theory*, MSRI 1987); Bridson–Haefliger, *Metric Spaces of
Non-Positive Curvature* (Springer Grundlehren 319, 1999).

**Claim.** A metric space `(X, d)` is **δ-hyperbolic** if every
4-point subset satisfies the "Gromov 4-point condition": the
sorted-descending sums `S₁ ≥ S₂ ≥ S₃` of opposite-edge pair-totals
satisfy `S₁ − S₂ ≤ 2δ`. Closed-form δ values:
- Trees: `δ = 0`.
- Cycles `C_n`: `δ = ⌊n/4⌋`.
- Complete graphs `K_n`: `δ = 0`.

δ-hyperbolicity generalizes Hadamard-Cartan to discrete metric
spaces — graphs and finite point clouds where there's no smooth
manifold underneath.

**Proof sketch.** Gromov's 4-point definition is equivalent to
"all triangles are δ-slim" via standard convex analysis. On a tree
`T`, any 4-tuple has at most one branch point so two of the three
pair-sums coincide → `δ = 0`. On a cycle `C_n`, the maximum-δ
configuration places 4 points equally spaced at distance `n/4`,
yielding the closed form.

**Validation.** PASS
([`test_5_gromov_hyperbolicity`](validation_tests.py)).
- Tree `T₆`: `δ = 0` exactly (machine zero).
- Cycle `C₈`: `δ = 2.0` exactly = `⌊8/4⌋`.
- Complete `K₅`: `δ = 0` exactly.
- Growth: `δ(C₁₂) = 3.0 > δ(C₈) = 2.0`.

**What this enables.**
- *GIGI:* the L5 Hadamard detector only fires for smooth
  Riemannian-Hadamard bundles. δ-hyperbolicity catches the much
  larger class of **bundles whose underlying graph is δ-hyperbolic
  even when the metric isn't Riemannian** — relation graphs, sparse
  networks, tree-like document hierarchies. Direct expansion of
  L5's reach.
- *Marcella:* token-graph δ-hyperbolicity controls how "tree-like"
  the model's induced semantic graph is — small δ ⇒ embeds into
  low-dim hyperbolic space efficiently (Sarkar 2011).
- *PRISM:* counterparty graphs are almost always δ-hyperbolic in
  practice (financial networks tend tree-like); δ measures
  "how much tree-structure" — directly useful for routing /
  reconciliation planning.

**Implementation pointers.**
- `src/graph/gromov.rs` — `compute_delta(adjacency, sample_size)`
  with sampled 4-tuples for large graphs (full enumeration is
  `O(n⁴)`; sampling gives a high-confidence upper bound).
- Surface as `BundleStore::delta_hyperbolicity()` cached per
  bundle. Hook into L5 as an additional Hadamard signal:
  "δ-hyperbolic ⇒ practically Hadamard".

---

### §6. Tropical Geometry

**Source.** Maclagan–Sturmfels, *Introduction to Tropical
Geometry* (AMS Graduate Studies 161, 2015); Mikhalkin,
"Enumerative tropical algebraic geometry in ℝ²" (*JAMS* 18, 2005).
Roots in Viro (1980s), Bergman (1971).

**Claim.** With the **tropical semiring** `(ℝ ∪ {∞}, min, +)`,
a tropical polynomial `p(x) = min_i (a_i + i · x)` of tropical
degree `d` has at most `d` **tropical roots** (corners — points
where the active monomial changes). When the coefficient sequence
`{a_i}` is **min-convex** (the lower convex hull touches every
point), `p` has exactly `d` roots — the tropical Fundamental
Theorem of Algebra (Maclagan–Sturmfels §1.1).

**Proof sketch.** Tropical addition `min` is idempotent; the graph
of `p` is a piecewise-linear lower-convex function. Each corner
is where two monomial lines intersect; convex position ⇒ each
adjacent pair of monomials contributes one corner.

**Validation.** PASS
([`test_6_tropical_fundamental_theorem`](validation_tests.py)).
- Degree-1 `min(5, 0 + x)`: 1 root.
- Degree-2 `min(10, 2 + x, 0 + 2x)`: 2 roots.
- Degree-3 convex `min(0, 1 + x, 4 + 2x, 9 + 3x)`: 3 roots.
- Negative: degenerate `min(0, 100 + x, 4 + 2x)` (middle monomial
  never active): 1 root (< degree).

**What this enables.**
- *GIGI:* a **second algebra over the same bundle data**, where
  `+ → min` and `· → +`. Query operators in tropical semiring
  give **scheduling / shortest-path / min-cost** queries with
  identical syntax to the standard algebra. Same SQL → either
  classical or tropical semantics.
- *PRISM:* tropical algebra naturally encodes **time-cost
  reconciliation** (Bellman's equation in disguise). Optimal-fee
  routing, latency-bounded matching.
- *Marcella:* tropical layers (Maragos–Charisopoulos–Theodosis
  2021) are min-plus neural nets — semantically natural for
  hard-attention-like operations. Could replace some softmax
  layers cleanly.

**Implementation pointers.**
- `src/algebra/tropical.rs` — `TropicalSemiring` trait with `oplus`
  (= min) and `otimes` (= +). Tropical polynomial type with corner
  finder.
- Query planner extension: if a query uses only `min`/`+`
  reductions, route through tropical fast paths (which are
  embarrassingly parallel and have no precision issues).

---

### §7. Synthetic Differential Geometry

**Source.** Kock, *Synthetic Differential Geometry* (Cambridge
Lecture Notes 51, 2nd ed. 2006); Lavendhomme, *Basic Concepts of
Synthetic Differential Geometry* (Kluwer, 1996). Roots in Lawvere
(1967), Dubuc (1979).

**Claim.** Over the **dual-number ring** `R[ε]/ε²`, every smooth
function `f: R → R` extends uniquely to `f: R[ε] → R[ε]` satisfying
`f(a + b ε) = f(a) + f'(a) b ε` (the Kock–Lawvere axiom). Forward-
mode automatic differentiation IS this extension — derivatives
are exact, not approximate.

**Proof sketch.** For polynomial `f`, expand `f(a + bε)` and use
`ε² = 0`: every quadratic or higher term in `ε` vanishes; the
linear term is `f'(a) · b ε` by the binomial theorem. For smooth
`f` in synthetic-DG topos, the axiom is *posited* — and it has
models (Dubuc's well-adapted topos) where it actually holds.

**Validation.** PASS
([`test_7_synthetic_dg_dual_numbers`](validation_tests.py)).
Dual-number arithmetic on `f(x) = x³ + 2x² − 5x + 1`:
- `f(3) = 31` (value match).
- `f'(3) = 34` (exact derivative, no finite-difference truncation).
- `ε² = 0` confirmed in the ring.
- Negative: central finite differences give err ≈ 1e-6 for
  `h = 1e-3` — exact differentiation strictly better.

**What this enables.**
- *GIGI:* **first-class derivative queries** in GQL. `SELECT
  d(metric) / d(time)` returns exact derivatives without
  numerical differentiation. Dual-number primitive at the engine
  layer.
- *Marcella:* forward-mode AD for arbitrary user-supplied
  Hamiltonians on transport flows — clean alternative to symbolic
  differentiation.
- *DPU:* dual-number arithmetic in hardware would be a natural
  extension of the existing FP unit. Two-word multiplication; same
  silicon area as a complex multiplier.
- *Foundation:* the categorical vocabulary (smooth topos, microlinear
  object) gives a *language* to declare bundle invariants that's
  more flexible than first-order Rust types. Useful for the GIGI
  Lang spec.

**Implementation pointers.**
- `src/algebra/dual.rs` — `Dual<T>` newtype carrying (value,
  derivative-coefficient); trait-bound arithmetic.
- Extend GQL to support `d(expr) / d(var)` reduction operator;
  query planner inserts dual-number computation.

---

## Part C — Wilder / research-mode

### §8. Noncommutative Geometry (Connes)

**Source.** Connes, *Noncommutative Geometry* (Academic Press,
1994); Connes–Marcolli, *Noncommutative Geometry, Quantum Fields
and Motives* (AMS Colloquium Pub. 55, 2008).

**Claim.** A **spectral triple** `(A, H, D)` — `A` a C*-algebra,
`H` a Hilbert space, `D` a self-adjoint Dirac operator — encodes
metric, symmetry, and bundle data in one object. **Connes' formula**

> `d_Connes(p, q) = sup { |φ(f) − ψ(f)| : ‖[D, f]‖_op ≤ 1 }`

(states `φ, ψ` evaluated at points `p, q` for commutative `A`)
recovers the geodesic distance from the data of the algebra and
the Dirac operator alone — without ever referencing a manifold.
For `(C(S¹), L²(S¹), -i d/dθ)`, Connes distance = arc length on S¹.

**Proof sketch.** `[D, f] = -i f'` (commutator with differential
operator), so `‖[D, f]‖_op = ‖f'‖_∞`. The sup over 1-Lipschitz
functions of `|f(p) − f(q)|` is the geodesic distance by Kantorovich-
Rubinstein duality.

**Validation.** PASS
([`test_8_noncommutative_geometry_connes_distance`](validation_tests.py)).
Discretized `S¹` on N=2000 grid points:
- Three test pairs (`π/2`, `π`, generic): max error 3.5e-4 vs
  grid spacing 3.1e-3 (well within discretization).
- Negative: chord distance (2.0) ≠ arc (π); Connes is intrinsic.

**What this enables.**
- *GIGI:* the data substrate `𝒢` can be reformulated as a spectral
  triple `(C(M), L²(M), D)` — purely algebraic, no manifold
  required. Operations on non-classical bundles (quantum, fractal,
  graph-only) inherit the same calculus.
- *Marcella:* token-embedding distance via Connes formula — works
  for non-Euclidean token manifolds where there's no smooth metric
  but there IS a natural Dirac operator (graph Laplacian as a
  proxy).
- *Foundation:* unifies L2 (graph adjacency), L4 (Kähler curvature),
  L6 (Hodge / Dirac) under one algebraic packaging. Long-term:
  GIGI's `BundleSchema` becomes a spectral triple, with current
  fields as the algebra `A` and L6's Hodge structure providing `D`.

**Implementation pointers.**
- `src/geometry/spectral_triple.rs` — research-mode module;
  re-export L2 adjacency + L6 Dirac as spectral-triple data.
- Marcella-side: Dirac eigenvalue spectrum as token-distance
  metric.

---

### §9. CAT(κ) Spaces

**Source.** Bridson–Haefliger, *Metric Spaces of Non-Positive
Curvature* (Springer Grundlehren 319, 1999); Ballmann, *Lectures
on Spaces of Nonpositive Curvature* (Birkhäuser DMV 25, 1995).

**Claim.** A geodesic metric space `X` is **CAT(κ)** if every
geodesic triangle is "no fatter" than a comparison triangle in
the model space of constant curvature κ (Euclidean for κ=0,
sphere of curvature κ for κ>0, hyperbolic for κ<0). Equivalent
**CN-inequality** (Reshetnyak, Bruhat-Tits):

> `d(x, m)² ≤ ½ d(x, y)² + ½ d(x, z)² − ¼ d(y, z)²`

where `m` is the midpoint of `yz`. CAT(0) generalizes Hadamard
to non-smooth metric spaces — graphs, trees, polyhedral complexes,
all the discrete analogs.

**Proof sketch.** The CN-inequality is the *defining* (Reshetnyak)
characterization in dimension ≥ 2. For ℝⁿ it's the parallelogram
law (equality). For spheres / positive curvature, the inequality
*fails* on triangles larger than a curvature-dependent radius —
the triangle is "too fat" because geodesics diverge less than in
Euclidean.

**Validation.** PASS
([`test_9_cat_kappa_comparison`](validation_tests.py)).
- ℝ²: CN saturates (residual −1.4e-14, machine zero — Euclidean
  parallelogram law).
- Small triangles on S² (width 0.05 around the pole): satisfy CN
  (near-flat regime).
- Large random triangles on S²: 426/499 violate CN — confirming
  S² is **not** CAT(0).

**What this enables.**
- *GIGI:* generalize L5 Hadamard to **CAT(0) bundles** — covers
  discrete graph metrics that aren't Riemannian-smooth but still
  have non-positive curvature in the metric-space sense.
  Strictly enlarges the class of bundles that get the L5/L1.4/§1.5
  guarantees.
- *Marcella:* embedding spaces that are CAT(0) admit unique
  geodesics between any two tokens (Cartan-Hadamard for CAT(0)).
  Reversible-reasoning guarantees apply even when the token
  manifold isn't smooth.
- *PRISM:* relation graphs are often CAT(0) but rarely Riemannian-
  Hadamard. δ-hyperbolicity (§5) catches one piece; CAT(0) catches
  another (median-graph / cube-complex structures).

**Implementation pointers.**
- `src/geometry/cat_kappa.rs` — `is_cat0(bundle, sample_size)` via
  sampled CN-inequality testing on random 4-point configurations.
- Combine with L5 detector: bundle is "practically Hadamard" if
  CAT(0) OR conjugate-free OR (`K_B ≤ threshold`).

---

## 2. Validation summary

All 30 numerical checks PASS (see [`validation_tests.py`](validation_tests.py)).

| § | Direction | # checks | Status |
|---|---|---|---|
| 1 | Sasaki / contact | 4 (3 positive + 1 negative control) | PASS |
| 2 | Information geometry | 4 (3 + 1) | PASS |
| 3 | Optimal transport | 3 (1 + 2) | PASS |
| 4 | Persistent homology | 2 (1 + 1) | PASS |
| 5 | Gromov hyperbolicity | 4 (3 + 1) | PASS |
| 6 | Tropical geometry | 4 (3 + 1) | PASS |
| 7 | Synthetic DG | 4 (3 + 1) | PASS |
| 8 | Noncommutative geometry | 2 (1 + 1) | PASS |
| 9 | CAT(κ) | 3 (2 + 1) | PASS |

**Discipline notes** (preserved from the Kähler catalog):
- Every closed-form ground truth comes from a **different formalism**
  than the numerical computation (analytic differentiation vs. Monte
  Carlo, MST topology vs. random sampling, etc.).
- Every direction has at least one **negative control** — a
  configuration where the property must fail. Without negatives,
  PASS is meaningless.
- Where a result depends on a hypothesis (e.g. tropical FTA needs
  min-convex coefficients), the negative control violates the
  hypothesis and confirms the result fails accordingly.

## 3. Suggested implementation order

Rough prioritization by integration cost × strategic value. None
of these are scheduled — this is a menu, not a roadmap.

```
Cheap & high-value (each reuses existing L1–L9 infrastructure):
  §3 Wasserstein   ─────► PRISM cohort barycenters
  §5 Gromov δ      ─────► expand L5 reach
  §2 Fisher metric ─────► free from L4 streaming stats

Medium cost, opens a new algebra:
  §1 Sasaki contact ────► time-series / sequence bundles
  §6 Tropical       ────► min-plus query algebra
  §4 Persistent homology ► multi-scale topo fingerprint

Higher effort, broader payoff:
  §7 Synthetic DG / dual numbers ► GQL `d(expr) / d(var)`
  §9 CAT(κ)                      ► non-smooth metric bundles

Research-mode:
  §8 Noncommutative geometry — long-term unification
```

## 4. License & provenance note

All math cited above is published, peer-reviewed, and not subject
to patent claims by its originators. The named geometers
(Boyer-Galicki, Amari, Villani, Carlsson, Gromov, Maclagan-Sturmfels,
Kock-Lawvere, Connes, Ballmann-Bridson-Haefliger) released their
work under standard academic norms. GIGI's strategy:

- Operationalize the math as substrate primitives (the Rust
  modules under `src/`).
- Patent only the GIGI/Marcella-specific applications, wire
  formats, and runtime architectures built on top.
- Cite originators in module-level docs and the academic
  follow-up papers.

This is the same posture used for the Kähler upgrade
(catalog §0–§4 cite Adachi, Hashimoto, Hristov; the
implementations are GIGI-original).
