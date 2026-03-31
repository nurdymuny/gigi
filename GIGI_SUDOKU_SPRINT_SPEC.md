# GIGI Sudoku Sprint — Feature Gap Analysis & Implementation Spec

> **Date**: 2026-03-31  
> **Status**: Planning  
> **Revised**: 2026-03-31 (MIRADOR review incorporated)  
> **Principle**: Treat the space of mathematical features as a finite grid. Rows = math domains, columns = scope levels. Every occupied cell is a shipped feature. Every empty cell is a feature whose math we know and whose plumbing already exists. Fill the grid.

---

## 1. The Grid

| Domain | Pointwise (1 record) | Pairwise (2 records) | Global (whole bundle) | Cross-Bundle (2 bundles) |
|--------|:---:|:---:|:---:|:---:|
| **Differential Geometry** | ✅ `base_point`, `partition_fn` | ❌ **geodesic_distance** | ✅ `scalar_curvature`, `holonomy` | ❌ **pullback_curvature** |
| **Topology** | ✅ `point_query`, `exists` | ❌ **ricci_curvature** (per-edge) | ❌ **betti_numbers** β₀ β₁ | ✅ H¹ edge sync |
| **Information Geometry** | ✅ `field_stats` | ✅ `FiberMetric::distance` | ❌ **metric_tensor** | ❌ **kl_divergence** |
| **Statistical Mechanics** | ✅ `partition_fn`, `confidence` | — | ❌ **free_energy**, **entropy** | ❌ **phase_comparison** |

**Score**: 12 filled / 8 empty. All 8 empty cells have GQL keywords already reserved in the spec — they need Rust implementations.

---

## 2. The 8 Features

### 2.1 Geodesic Distance

> **Cell**: Differential Geometry × Pairwise  
> **Module**: `src/curvature.rs`  
> **GQL**: `GEODESIC ON bundle FROM {x:1} TO {x:5}`

**Math**:

Shortest path on the data manifold. Build the weighted adjacency graph from `field_index_graph()` (already in `spectral.rs`) with edge weights from `FiberMetric::distance()` (already in `metric.rs`). Solve with Dijkstra.

$$d_g(p, q) = \min_{\gamma: p \to q} \sum_{(i,j) \in \gamma} g_F(r_i, r_j)$$

where $g_F$ is the product fiber metric (Def 1.7).

**Signature**:
```rust
pub fn geodesic_distance(
    store: &BundleRef<'_>,
    key_a: &Record,
    key_b: &Record,
) -> Option<f64>
```

Returns `None` if no path exists (disconnected components).

**Dependencies**: `field_index_graph()`, `FiberMetric::distance()` — both exist.

**Scalability** (11M+ records): The field index graph has one node per record. Full Dijkstra on 11M nodes is not viable. Strategy:
- **A\* with fiber metric heuristic**: Use `FiberMetric::distance(key_a, key_b)` as the admissible heuristic. This is the straight-line lower bound on the manifold.
- **Locality bound**: Accept a `max_hops: Option<usize>` parameter (default: 1000). If the shortest path exceeds `max_hops` nodes, return `None` with a flag indicating truncation.
- **Lazy graph construction**: Don't materialize the full adjacency matrix. Expand neighbors on-demand from `field_index` lookups.
- **Sampled approximation**: For record counts > 100K, build a landmark-based distance oracle (sample ~sqrt(N) landmarks, precompute distances from each, interpolate).

**Unlocks**: ricci_curvature (denominator), kl_divergence (histogram binning), KNN queries, clustering.

---

### 2.2 Ollivier–Ricci Curvature

> **Cell**: Topology × Pairwise  
> **Module**: `src/curvature.rs`  
> **GQL**: `RICCI ON bundle FROM {id:1} TO {id:2}`

**Math**:

Per-edge curvature via optimal transport. For adjacent nodes $x, y$ with neighbor measures $\mu_x, \mu_y$:

$$\kappa(x, y) = 1 - \frac{W_1(\mu_x, \mu_y)}{d(x, y)}$$

where $W_1$ is the earth-mover's distance (1-Wasserstein) and $d$ is the geodesic distance.

- $\kappa > 0$ → cluster interior (neighbors overlap)
- $\kappa < 0$ → inter-cluster bridge (neighbors disjoint)
- $\kappa = 0$ → flat (tree-like)

$\mu_x$ = uniform measure on the geometric neighbors of $x$ in the field index graph.

$W_1$ is solved by the standard transportation LP on the neighbor sets. For small neighborhoods (typical: 5–50 nodes), this is fast.

**Signature**:
```rust
pub fn ricci_curvature(
    store: &BundleRef<'_>,
    key_a: &Record,
    key_b: &Record,
) -> f64
```

**Dependencies**: `geodesic_distance` (feature 2.1), `field_index_graph()`.

**Scalability**: Ricci requires geodesic distances between all neighbor pairs of $x$ and $y$. With local neighborhoods of ~50 nodes, that's ~2500 geodesic calls per edge. Mitigations:
- **Local Dijkstra**: Don't compute full shortest paths. Run Dijkstra from $x$ with a hop limit (e.g., 3-hop neighborhood). All distances needed for $W_1$ are within this local subgraph.
- **Cached neighborhoods**: Precompute and cache the k-hop neighborhood for each query node.
- **Batch Ricci**: If computing Ricci for all edges, share the local distance computations across overlapping neighborhoods.

**Unlocks**: Community detection (cut edges with κ < 0), anomaly explanation, curvature flow.

---

### 2.3 Betti Numbers

> **Cell**: Topology × Global  
> **Module**: `src/spectral.rs` (or new `src/topology.rs`)  
> **GQL**: `BETTI ON bundle` → `{beta_0: 3, beta_1: 2}`

**Math**:

Topological invariants of the field index graph $G = (V, E)$:

$$\beta_0 = \text{number of connected components}$$

$$\beta_1 = |E| - |V| + \beta_0 \quad \text{(independent cycles)}$$

$\beta_0$ is **already computed** inside `spectral_gap()` (union-find for components) but not exposed. $\beta_1$ follows directly from the Euler characteristic $\chi = \beta_0 - \beta_1 = |V| - |E|$.

**Signature**:
```rust
pub fn betti_numbers(store: &BundleRef<'_>) -> (usize, usize)
// Returns (β₀, β₁)
```

**Dependencies**: `field_index_graph()` — exists. β₀ extraction from union-find — already implemented internally.

**Unlocks**: Topological fingerprinting, data shape summary, consistency validation.

**Note**: This is a freebie — the math is already computed, just not returned.

**Caveat (TDA scope)**: This computes Betti numbers of the **1-dimensional simplicial complex** (the field index graph). It gives β₀ (components) and β₁ (independent cycles) only. Higher Betti numbers (β₂, β₃, ...) require constructing a Vietoris-Rips or Čech complex and running proper persistent homology — that's a separate feature (full TDA), not part of this sprint. This graph-theoretic shortcut is adequate for data shape fingerprinting but should not be marketed as "persistent homology."

---

### 2.4 Metric Tensor (Empirical Correlation Geometry)

> **Cell**: Information Geometry × Global  
> **Module**: `src/metric.rs`  
> **GQL**: `METRIC ON bundle` → `{matrix: [[...]], condition_number: 42.7}`

**Math**:

The Riemannian metric tensor on the statistical manifold of the bundle. For $n$ numeric fiber fields $f_1, \ldots, f_n$, compute the **empirical correlation matrix** of standardized fields:

$$G_{ij} = \frac{1}{N} \sum_{k=1}^{N} \tilde{f}_i(r_k) \cdot \tilde{f}_j(r_k)$$

where $\tilde{f}_i = \frac{f_i - \bar{f}_i}{\sigma_i}$ are standardized (zero mean, unit variance).

> **Naming note**: This is the empirical correlation matrix, NOT the Fisher information matrix. True FIM requires a parametric statistical model: $G_{ij}^{\text{Fisher}} = \mathbb{E}\left[\frac{\partial \log p}{\partial \theta_i} \frac{\partial \log p}{\partial \theta_j}\right]$. A future extension could implement FIM via kernel density estimation or mixture models. For now, the correlation matrix serves as the natural metric tensor on the empirical data manifold and is the correct geometric object for field dependency detection.

Key derived quantities:
- **Condition number**: $\kappa(G) = \frac{\lambda_{\max}}{\lambda_{\min}}$ — how "stretched" the data is. High κ → fields are near-colinear.
- **Effective dimension**: $d_{\text{eff}} = \frac{(\sum \lambda_i)^2}{\sum \lambda_i^2}$ (participation ratio).

**Signature**:
```rust
pub struct MetricTensorInfo {
    pub matrix: Vec<Vec<f64>>,
    pub eigenvalues: Vec<f64>,
    pub condition_number: f64,
    pub effective_dimension: f64,
    pub field_names: Vec<String>,
}

pub fn metric_tensor(store: &BundleRef<'_>) -> MetricTensorInfo
```

**Scalability**: Iterates all $N$ records to build the $n \times n$ matrix where $n$ = number of numeric fiber fields (typically 10–50). Cost is $O(N \cdot n^2)$ for construction + $O(n^2 \cdot \text{iters})$ for eigenvalues via power iteration. This is fine — $n$ is small even when $N$ is 11M.

**Dependencies**: `field_stats()` for means/stddevs — exists. Eigenvalues via power iteration — pattern exists in `spectral_gap()`.

**Unlocks**: Field dependency detection, dimensionality assessment, condition monitoring.

---

### 2.5 KL Divergence

> **Cell**: Information Geometry × Cross-Bundle  
> **Module**: `src/metric.rs`  
> **GQL**: `DIVERGENCE FROM bundle_a TO bundle_b` → `{kl: 0.34, js: 0.12}`

**Math**:

Measures information lost when using bundle $Q$ to represent bundle $P$. Per-field histogram approach:

$$D_{KL}(P \| Q) = \sum_{\text{field } f} \sum_{\text{bin } b} P_f(b) \ln \frac{P_f(b)}{Q_f(b)}$$

Bins are determined from the union of value ranges. Laplace smoothing prevents division by zero.

**Binning strategy**: Use **Freedman–Diaconis rule** per field: bin width $h = 2 \cdot \text{IQR} \cdot N^{-1/3}$, clamped to $[10, 200]$ bins. This adapts to data spread and is robust to outliers. For categorical fields, each distinct value is its own bin (no discretization needed). Apply additive Laplace smoothing: $\hat{p}(b) = \frac{n_b + \alpha}{N + \alpha K}$ where $\alpha = 1$ and $K$ = bin count.

> **Independence assumption**: The per-field sum approach assumes field independence: $D_{KL}(P \| Q) \approx \sum_f D_{KL}(P_f \| Q_f)$. This is a strong assumption — it misses cross-field correlations. A future extension could use the metric tensor (2.4) to compute a multivariate divergence, but the per-field decomposition is more interpretable and serves the primary use case (schema drift detection).

Also compute the symmetric Jensen–Shannon divergence:

$$D_{JS}(P, Q) = \frac{1}{2} D_{KL}(P \| M) + \frac{1}{2} D_{KL}(Q \| M), \quad M = \frac{P + Q}{2}$$

$D_{JS} \in [0, \ln 2]$. It's a proper metric (square root is).

**Signature**:
```rust
pub struct DivergenceReport {
    pub kl_forward: f64,       // D_KL(P || Q)
    pub kl_reverse: f64,       // D_KL(Q || P)
    pub jensen_shannon: f64,   // D_JS(P, Q)
    pub per_field: Vec<(String, f64)>,  // field-level KL contributions
}

pub fn kl_divergence(
    store_a: &BundleRef<'_>,
    store_b: &BundleRef<'_>,
) -> DivergenceReport
```

**Dependencies**: `field_stats()` for range bounds, `records()` for histogram construction.

**Unlocks**: Schema drift detection, replication quality, A/B testing, bundle similarity ranking.

---

### 2.6 Free Energy & Entropy

> **Cell**: Statistical Mechanics × Global  
> **Module**: `src/curvature.rs`  
> **GQL**: `FREEENERGY ON bundle TEMPERATURE 0.5` and `ENTROPY ON bundle`

**Math**:

**Free energy** wraps the existing `partition_function()`:

$$F(\tau) = -\tau \ln Z(\beta, p)$$

where $\beta = 1/\tau$ and $Z$ is already implemented (Def 3.7). Averaged over a sample of base points.

**Shannon entropy** of the data distribution (already computed inside `coarse_grain()` but not exposed):

$$S = -\sum_{i} \frac{n_i}{N} \ln \frac{n_i}{N}$$

where $n_i$ are group sizes from the finest-grained partition.

**Heat capacity** (second derivative signals phase transitions):

$$C_V = -\tau^2 \frac{\partial^2 F}{\partial \tau^2} \approx \tau^2 \frac{F(\tau + \delta) - 2F(\tau) + F(\tau - \delta)}{\delta^2}$$

**Signature**:
```rust
pub fn free_energy(store: &BundleRef<'_>, tau: f64) -> f64

pub fn entropy(store: &BundleRef<'_>) -> f64

pub struct ThermodynamicProfile {
    pub free_energy: f64,
    pub entropy: f64,
    pub heat_capacity: f64,
    pub temperature: f64,
    pub curvature: f64,
}

pub fn thermodynamic_profile(store: &BundleRef<'_>, tau: f64) -> ThermodynamicProfile
```

**Dependencies**: `partition_function()` — exists. `coarse_grain()` — exists (entropy extraction).

**Note**: This is mostly a freebie — wrapping existing functions and exposing already-computed values.

---

### 2.7 Pullback Curvature

> **Cell**: Differential Geometry × Cross-Bundle  
> **Module**: `src/join.rs`  
> **GQL**: `PULLBACK CURVATURE FROM left_bundle TO right_bundle ON fk_field`

**Math**:

Given a pullback join $f: L \to R$ along a foreign key field, the pullback curvature measures geometric distortion:

$$K_{f^*R} = \frac{1}{|L|} \sum_{l \in L} K_R(f(l))$$

where $K_R(f(l))$ is the local curvature contribution at the right-side record that $l$ maps to.

Compare with the native curvature $K_R$ of the right bundle:

$$\Delta K = K_{f^*R} - K_R$$

- $\Delta K \approx 0$ → join is geometrically faithful
- $\Delta K \gg 0$ → FK skews toward high-curvature regions (fan-out)
- $\Delta K \ll 0$ → FK skews toward low-curvature regions (sparse linkage)

**Signature**:
```rust
pub struct PullbackReport {
    pub pullback_curvature: f64,
    pub native_curvature: f64,
    pub delta: f64,
    pub coverage: f64,          // fraction of right records reached by FK
    pub fan_out_mean: f64,      // avg records per FK target
    pub fan_out_max: usize,
}

pub fn pullback_curvature(
    left: &BundleRef<'_>,
    right: &BundleRef<'_>,
    left_field: &str,
    right_field: &str,
) -> PullbackReport
```

**Dependencies**: `pullback_join()` — exists. `scalar_curvature()` — exists.

**Unlocks**: Join quality metrics, referential integrity health, cross-bundle consistency.

---

### 2.8 Phase Comparison

> **Cell**: Statistical Mechanics × Cross-Bundle  
> **Module**: `src/curvature.rs`  
> **GQL**: `PHASE FROM bundle_a TO bundle_b`

**Math**:

Two bundles are in the "same phase" if their thermodynamic signatures are close. The signature vector for a bundle $B$ is:

$$\vec{\Phi}(B) = \left( K_B,\ \lambda_{1,B},\ S_B,\ F_B(\tau_0),\ \beta_0,\ \beta_1 \right)$$

Phase distance:

$$d_{\Phi}(A, B) = \left\| \frac{\vec{\Phi}(A) - \vec{\Phi}(B)}{\vec{\sigma}} \right\|_2$$

where $\vec{\sigma}$ is a normalization vector (e.g., typical scale of each component across known bundles, or just std of the two values).

**Signature**:
```rust
pub struct PhaseSignature {
    pub curvature: f64,
    pub spectral_gap: f64,
    pub entropy: f64,
    pub free_energy: f64,
    pub beta_0: usize,
    pub beta_1: usize,
}

pub struct PhaseReport {
    pub signature_a: PhaseSignature,
    pub signature_b: PhaseSignature,
    pub phase_distance: f64,
    pub same_phase: bool,        // distance < threshold
}

pub fn phase_comparison(
    store_a: &BundleRef<'_>,
    store_b: &BundleRef<'_>,
    tau: f64,
) -> PhaseReport
```

**Dependencies**: `scalar_curvature()`, `spectral_gap()`, `entropy()` (feature 2.6), `free_energy()` (feature 2.6), `betti_numbers()` (feature 2.3).

**Unlocks**: Drift detection (compare snapshots over time), environment comparison, replication health.

---

## 3. Dependency Graph

```
                    geodesic_distance (2.1)         [KEYSTONE]
                    /                  \
           ricci_curvature (2.2)    kl_divergence (2.5)
                                         \
betti_numbers (2.3) ─────────────── phase_comparison (2.8)
                                         /
free_energy + entropy (2.6) ────────────┘
fisher_matrix (2.4) ─────────── [INDEPENDENT]
pullback_curvature (2.7) ────── [INDEPENDENT, uses existing join]
```

**Independent (can build in any order)**:
- 2.3 Betti numbers (freebie — expose existing computation)
- 2.4 Metric tensor (new math, self-contained)
- 2.6 Free energy + entropy (freebie — wrap existing functions)
- 2.7 Pullback curvature (freebie — uses existing join + curvature; **moved to Sprint A** to prove cross-bundle BundleRef pattern early)

**Depends on geodesic_distance (2.1)**:
- 2.2 Ricci curvature
- 2.5 KL divergence (needs histogram binning, can use simpler approach without geodesics)

**Depends on multiple features**:
- 2.8 Phase comparison (needs 2.3 + 2.6)

---

## 4. Recommended Build Order

| Sprint | Features | Why |
|--------|----------|-----|
| **Sprint A** | 2.3 (Betti), 2.6 (free energy + entropy), 2.7 (pullback curvature) | Freebies: expose already-computed math. Pullback proves cross-bundle BundleRef pattern, de-risks later sprints. |
| **Sprint B** | 2.1 (geodesic), 2.4 (metric tensor) | Keystone + independent. Opens the dependency chain. |
| **Sprint C** | 2.2 (Ricci), 2.5 (KL divergence) | Now unlocked by Sprint B. Pairwise + cross-bundle. |
| **Sprint D** | 2.8 (phase comparison) | Capstone: depends on everything above. |

---

## 5. GQL Keyword Mapping

Each feature maps to a GQL statement. The parser already reserves most of these keywords.

| Feature | GQL Statement | Returns |
|---------|---------------|---------|
| geodesic_distance | `GEODESIC ON b FROM {k1} TO {k2}` | `ExecResult::Scalar(f64)` |
| ricci_curvature | `RICCI ON b FROM {k1} TO {k2}` | `ExecResult::Scalar(f64)` |
| betti_numbers | `BETTI ON b` | `ExecResult::Rows([{beta_0, beta_1}])` |
| metric_tensor | `METRIC ON b` | `ExecResult::Rows([{matrix, condition_number, effective_dim}])` |
| kl_divergence | `DIVERGENCE FROM b1 TO b2` | `ExecResult::Rows([{kl, js, per_field}])` |
| free_energy | `FREEENERGY ON b TEMPERATURE τ` | `ExecResult::Scalar(f64)` |
| entropy | `ENTROPY ON b` | `ExecResult::Scalar(f64)` |
| pullback_curvature | `PULLBACK CURVATURE FROM b1 TO b2 ON field` | `ExecResult::Rows([{pullback_k, native_k, delta, coverage}])` |
| phase_comparison | `PHASE FROM b1 TO b2` | `ExecResult::Rows([{sig_a, sig_b, distance, same_phase}])` |

---

## 6. REST API Endpoints

| Feature | Method | Path |
|---------|--------|------|
| geodesic_distance | `POST` | `/v1/bundles/{name}/geodesic` |
| ricci_curvature | `POST` | `/v1/bundles/{name}/ricci` |
| betti_numbers | `GET` | `/v1/bundles/{name}/betti` |
| metric_tensor | `GET` | `/v1/bundles/{name}/metric` |
| kl_divergence | `POST` | `/v1/divergence` (body: `{from, to}`) |
| free_energy | `GET` | `/v1/bundles/{name}/free-energy?tau=0.5` |
| entropy | `GET` | `/v1/bundles/{name}/entropy` |
| pullback_curvature | `POST` | `/v1/pullback-curvature` (body: `{left, right, left_field, right_field}`) |
| phase_comparison | `POST` | `/v1/phase` (body: `{bundle_a, bundle_b, tau}`) |

**Cross-bundle body schemas** (for OpenAPI):
```json
// POST /v1/divergence
{"from": "bundle_a", "to": "bundle_b"}

// POST /v1/pullback-curvature
{"left": "orders", "right": "customers", "left_field": "customer_id", "right_field": "id"}

// POST /v1/phase
{"bundle_a": "snapshot_jan", "bundle_b": "snapshot_feb", "tau": 0.5}
```

---

## 7. Test Strategy

Each feature gets:
1. **Unit test** (pure math on synthetic data with known answer)
2. **Property test** (invariants hold: e.g., β₁ ≥ 0, KL ≥ 0, geodesic satisfies triangle inequality)
3. **Integration test** (end-to-end through GQL and REST)

Known-answer tests:
- Complete graph K_n: β₀ = 1, β₁ = n(n-1)/2 - n + 1
- Two disconnected cliques: β₀ = 2
- Uniform data: entropy = ln(N), Fisher = identity, KL = 0
- Single cluster: Ricci > 0 internally
- Star graph: Ricci < 0 on spokes

---

## 8. Existing Infrastructure Used

| Existing Function | Used By |
|---|---|
| `spectral::field_index_graph()` | geodesic, ricci, betti |
| `metric::FiberMetric::distance()` | geodesic, ricci |
| `curvature::partition_function()` | free_energy |
| `curvature::scalar_curvature()` | pullback_curvature, phase |
| `spectral::spectral_gap()` | phase |
| `spectral::coarse_grain()` | entropy (extract from existing) |
| `join::pullback_join()` | pullback_curvature |
| `bundle::BundleStore::records()` | metric_tensor, kl_divergence |
| `bundle::BundleStore::field_stats()` | metric_tensor, kl_divergence |

**No new dependencies required.** Every feature builds on what's already there.

---

## 9. Cross-Bundle BundleRef Locking Strategy

Cross-bundle functions (`kl_divergence`, `pullback_curvature`, `phase_comparison`) need two `BundleRef`s simultaneously. The current `Engine` accessor returns one `BundleRef` at a time via `engine.bundle(name)` which borrows the `RwLockReadGuard`.

**Strategy**: Hold a single read lock on the `Engine`, then extract both `BundleRef`s from it:

```rust
// Safe: single read lock, two borrows from the same guard
let engine = state.engine.read().unwrap();
let store_a = engine.bundle("a").ok_or("not found")?;
let store_b = engine.bundle("b").ok_or("not found")?;
let report = kl_divergence(&store_a, &store_b);
```

This works because both `BundleRef`s borrow from the same `RwLockReadGuard<Engine>` lifetime. No deadlock risk — it's a single shared read lock. Write operations are blocked while both refs are alive, which is the correct semantic (snapshot consistency).

**For pullback with `as_heap()` fallback**: If the underlying function (e.g., `join::pullback_join`) still takes `&BundleStore`, extract via `store.as_heap()` inside the handler. Both heap refs borrow from the same engine guard.

---

## 10. Alignment Notes

> The user has done independent mathematical work on several of these concepts. This spec captures the computational signatures and API surface — the underlying mathematical theory may have deeper formulations in the user's own work. Before implementing, verify alignment:
>
> - **Ricci curvature**: Ollivier formulation chosen here. User may prefer Forman-Ricci or a custom formulation.
> - **Metric tensor**: Empirical correlation matrix approach (NOT Fisher information — see 2.4 naming note). User may have a parametric model-based formulation that warrants true FIM.
> - **Phase comparison**: Simple signature vector approach. User's statistical mechanics work may define phases differently.
> - **Free energy**: Direct wrapping of partition function. User's thermodynamic theory may include additional terms.
>
> **Action**: Before each Sprint, review user's independent work for the relevant features and reconcile definitions.

---

## 11. Review History

| Date | Reviewer | Changes |
|------|----------|---------|
| 2026-03-31 | Initial | Full spec written |
| 2026-03-31 | MIRADOR | BundleRef signatures (all 8), Fisher→MetricTensor rename, Betti TDA caveat, KL binning strategy (Freedman-Diaconis), geodesic/Ricci scalability (A*, locality bounds, lazy graph), pullback→Sprint A, cross-bundle locking strategy §9, REST body schemas |
