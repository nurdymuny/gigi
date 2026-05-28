# GIGI

**Geometric Intrinsic Global Index** — a fiber-bundle database engine.

> Records are sections of a fiber bundle. Keys live on the base space; values
> live on the fiber. Curvature, spectral connectivity, holonomy, and confidence
> are **properties of the bundle** — they update incrementally with every
> insert and ride along on every query response. Geometry is not a plugin.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/rust-2021-orange.svg)](Cargo.toml)

```
Davis Geometric · 2026 · Bee Rosa Davis
```

---

## Plain-English primer

### What's a fiber bundle, really?

Imagine a phone book. Names are how you find an entry; the stuff about
each person — address, phone, age, occupation — is what you find when
you look them up. The names form an **index**. The stuff at each name
is the **fiber** (the values).

A fiber bundle is the same shape, taken seriously. The index is the
**base space** B. The stuff at each index point is the **fiber** F.
Every "row" in the bundle is a *section* — a way of assigning one fiber
to each base point. That's the same thing every relational DB does:
keys on one side, values on the other.

Where it changes is what GIGI does *with* that shape. Imagine the index
isn't a flat list — it's the surface of the moon. Every point on the
moon (a base point) has terrain underneath it (the fiber). The way the
terrain changes as you walk around the moon — that's **curvature**.
The presence of crater rings — that's **non-trivial topology**. If you
walk in a closed loop and end up facing a different direction than when
you started — that's **holonomy**. None of that is in the phone book.
All of it is in the moon.

GIGI treats your database like the moon, not like the phone book. Every
record is a point on a curved surface. The shape of that surface tells
you things the rows alone never could:

- **Curvature κ** at a record = *"how anomalous is this row compared to
  what's nearby?"*
- **Spectral gap λ₁** of the whole bundle = *"how clumpy or smooth is
  the data?"*
- **Holonomy** around a categorical loop = *"does this category
  implicitly twist the values somehow?"*
- **Parallel transport** between two records = *"if I had to walk from
  row A to row B along the data's natural geometry, what path would
  I take?"*
- **Confidence 1/(1+κ)** = *"how much should I trust this answer?"*

These aren't add-ons. They are **what GIGI computes during normal
operation**. Curvature updates on every insert via Welford's online
algorithm. Spectral gap is cached and incrementally refreshed.
Holonomy is a verb in the query language (`HOLONOMY corpus ...`).

The headline: **a conventional database stores rows. GIGI stores the
geometry that rows live on.** The rows are still there — every GIGI
query returns the same shape Postgres or Mongo would — but every
response also carries the geometric quantities, free, because the
substrate computed them on the way through.

### Day-one operations: GIGI vs Postgres / MySQL / Mongo

How the basics work, side by side. Every row here is a real operation
the engine implements, not an aspirational future.

| You want to… | Conventional DB | GIGI |
|---|---|---|
| **Insert one row** | Append to table, update indexes | Append + bump Welford field stats + mutation_counter + curvature; the bundle's geometry incrementally evolves |
| **Look up by primary key** | B-tree (O(log N)) or hash (O(1)) | `SECTION bundle AT (id='X');` — GIGI hash G(K₁,…,Kₘ) → ℤ₂⁶⁴, **always O(1)**; response also carries κ + confidence |
| **`WHERE x = 5`** | Index scan or full table scan | Same O(1) GIGI hash on the addressed key; with geometric annotation per result |
| **`WHERE x BETWEEN 10 AND 20`** | Range scan over a sorted index | `filtered_query` over the bundle; returns hits + per-hit position in the fiber |
| **`GROUP BY region`** | Hash-grouped aggregation | `INTEGRATE field OVER bundle COVER ALL;` — aggregation = integration over a base-space cover (geometric, not relational) |
| **`JOIN orders ON customers`** | Hash join / merge join | `PULLBACK orders ALONG customers` — pullback of one bundle along a shared base map |
| **`COUNT(*)`, `AVG(x)`, `STDDEV(x)`** | Scan, or precomputed summary tables | Welford stats maintained incrementally on every insert — **O(1) read**, never stale |
| **Detect anomalies / outliers** | Add a streaming pipeline + outlier model | Curvature κ already updated per insert; outliers are points where κ spikes — no separate pipeline |
| **"How clustered is my data?"** | Run k-means or DBSCAN offline | `SPECTRAL bundle;` — Fiedler value λ₁ from the index Laplacian, cached and incrementally refreshed |
| **"Are A and B related geometrically?"** | Foreign key + join | `TRANSPORT bundle FROM (id=A) TO (id=B) ON FIBER (...);` — explicit parallel transport, returns the SO(n) rotation matrix |
| **Encrypt sensitive columns** | Column encryption — usually breaks indexes | **Gauge encryption** preserves κ, λ₁, anomaly scores, holonomy at **native speed** (vs ~10,000× slowdown for homomorphic encryption) |
| **Add a new column** | `ALTER TABLE`, schema migration, downtime | Fiber type evolves; old records remain valid sections under the wider fiber |
| **Restart the server** | Reload from disk, indexes rebuild from scratch | Reload from mmap snapshot in seconds; brain endpoints work polymorphically on the reloaded bundle (#107) |
| **"Tell me how surprising this query was"** | Build a custom logging layer | Every response includes κ, KL-divergence, JS-divergence via the `dhoom` event protocol — free |
| **"Find me an option that's close to satisfying these 5 constraints"** | Multiple WHERE queries + manual relaxation | `POST /v1/bundles/{name}/brain/sudoku` — returns solutions, near-misses with quantified relaxation cost, Pareto frontier of multi-violation alternatives, and an honest `Sat`/`Unsat`/`Unknown` verdict |
| **"What other records are geometrically nearby?"** | Vector DB + ANN index | `POST /v1/bundles/{name}/brain/sample_transport` — curvature-bounded neighborhood (`d² ≤ τ`), Efraimidis-Spirakis weighted sample, returns k records with per-candidate `curvature_k` and bundle-wide confidence |

The compounding effect: because curvature updates on every insert, you
don't have to *decide later* to add anomaly detection — it's already
running. Because spectral gap is cached, *"is this data clumpy?"* is a
constant-time read instead of an offline batch job. Because confidence
ships in every response, *"should I trust this?"* doesn't need a
separate ML stack. **Geometry is not a plugin.** It's the substrate.

### What it costs

- **Insert latency**: ~the same as a relational DB doing the same
  number of column writes. The Welford updates are O(1) per numeric
  field; the mutation_counter is a single atomic increment.
- **Memory**: ~1.5× a row-store for the same data, because the geometric
  metadata (per-field stats, fiber index, mutation epoch) lives
  alongside the records.
- **Cold-start after restart**: bundles reload from mmap snapshots in
  seconds, not the minutes a B-tree rebuild can take. Brain endpoints
  (curvature reads, SUDOKU, SAMPLE_TRANSPORT) are available immediately,
  not after a warm-up.

### What it doesn't do (yet)

- **Cross-bundle ACID transactions** — single-bundle writes are
  atomic; multi-bundle is your problem to coordinate.
- **SQL-compatible wire** — the query language is GQL (similar shape
  to SQL but with the geometric verbs above; full grammar in
  `GQL_REFERENCE.md`). SDKs in Python and JS hide the difference.
- **Sharding across nodes** — single-node engine right now. The
  fiber-bundle structure should shard cleanly along base-space
  partitions, but that work is for a later sprint.

---

## Why GIGI

Conventional databases see rows. GIGI sees a section σ: B → E of a fiber
bundle (E, B, F, π, Φ): the base space B is the queryable keys, the fiber
F is the value schema, and every record is a point in the total space E.
This isn't decoration — it's how the engine indexes, queries, and reasons
about your data:

| You want… | Conventional DB | GIGI |
|---|---|---|
| O(1) point query by composite key | Multi-column hash index | GIGI hash G : K₁ × … × Kₘ → ℤ₂⁶⁴ — native |
| Anomaly detection | Add a streaming pipeline | Curvature κ updated per insert; outliers fall out |
| "How clustered is this?" | Run k-means offline | Spectral gap λ₁ from the index Laplacian |
| Compute on encrypted data | Homomorphic encryption (~10,000× slowdown) | Gauge encryption — **native speed**, geometry-preserving |
| Logging with semantic insight | Text logs + sampling | DHOOM events with κ, KL-div, JS-div per query |
| NLP fiber geometry (tense, morphology, …) | Vector DB + bespoke analysis | `HOLONOMY corpus ON FIBER (f11, f12) AROUND tense_label` |
| Long-context theorems on a substrate | "trust me" + benchmarks | Kähler-upgrade catalog with cross-team math validation matching observations to rounding precision |

---

## What's new in 2026 — the Kähler upgrade

GIGI v3 shipped the **Kähler upgrade**: twelve layers (L1–L7, L8 cross-team
handoff, L9 moment maps, L10 generative flow, L11 predictive coding,
L12 attention + memory) of geometric machinery extending the fiber-bundle
substrate with a complex structure J, a closed 2-form B, and everything
that falls out of the pair — Hadamard substructure detection, holomorphic
curvature decomposition, Morse compression, line-bundle integrality checks,
quantum cohomology on toy manifolds, Berezin-Toeplitz operators,
Riemann-Roch representational capacity, moment-map / Noether conservation
along Hamiltonian B-flows, **the Friston-FEP keystone — generative flow on
the Kähler bundle that parametrizes its boundary conditions to deliver
SAMPLE / FORECAST / DREAM / RECONSTRUCT as one piece of infrastructure**,
**predictive-coding primitives stacked on top: INPAINT (constrained
Langevin for filling in missing fields), PREDICT (single Fisher-natural-
gradient step), and SELF-MONITOR (kernel-density confidence — the brain's
"I don't know" signal)**, and **the attention + memory pillar that closes
the brain-primitives catalog: ATTEND (softmax over geodesic distance),
FOCUS (top-k sub-bundle retrieval), EPISODIC (persistent-H₀ change-point
detection on time-indexed value sequences), SEMANTIC (Morse-compressed
gist wrapping L6)**. All 12 brain primitives now operational. The Kähler catalog
([`theory/kahler_upgrade/`](theory/kahler_upgrade/)) closes at **16 of 21
items shipped** — 100% of items the catalog itself classified as ship-able;
the remaining 5 (§1.6 hypersurface, §2.4 K-theory, §2.6 Floer, §2.7 mirror
symmetry, §E.4 hyperkähler) are explicitly deferred in the catalog's own
classification.

GIGI now ships **three companion catalogs**, each in the same format:

- [`theory/kahler_upgrade/`](theory/kahler_upgrade/) — the 21-item Kähler
  catalog with 16 shipped; 15/15 Python validation tests pass across
  v1–v4 suites.
- [`theory/post_kahler_directions/`](theory/post_kahler_directions/) — nine
  **post-Kähler** geometric programs (Sasaki, information geometry, OT/
  Wasserstein, persistent homology, Gromov δ-hyperbolicity, tropical,
  synthetic DG, NCG, CAT(κ)) from outside the Adachi lineage; 30/30
  numerical checks pass.
- [`theory/brain_primitives/`](theory/brain_primitives/) — the Sudoku-10×
  reading. **Twelve brain-like operations forced by one master equation**
  `ẋ = B⁻¹∇(−log p)` on the Kähler bundle — the same equation Friston
  writes down for variational free-energy minimization. One generator,
  twelve product-level primitives (SAMPLE, FORECAST, DREAM, RECONSTRUCT,
  INPAINT, PREDICT, ATTEND, FOCUS, EPISODIC, SEMANTIC, SELF-MONITOR,
  EXPLAIN); 26/26 numerical checks pass. L10 ships the keystone (gradient-
  + Hamilton-flow infrastructure); L11/L12 follow the same pattern.

Three properties are worth calling out because they're hard to find anywhere
else at this scale:

**1. Strict additivity. The optionality contract holds across all twelve layers.**
The entire Kähler upgrade lives behind a single Cargo feature flag (`kahler`).
With the feature off, the engine is **bit-identical to pre-upgrade GIGI**
— 674 tests pass, byte-equal to before the upgrade landed. With the feature
on, 821 tests pass, including a per-layer real-data smoke against the
20-record sensor dataset and a per-layer cross-team contract test
(`tests/kahler_*_marcella_contract.rs`) that fails before any consumer
deserialization can drift. Twelve layers of new math, zero breaking changes.

**2. Math predictions validated by production observation to rounding precision.**
The first downstream consumer (Marcella) ran a 30-prompt A/B harness +
10-turn deep-trace on her actual embedding substrate
(`marcella_source_embeddings_bge`, 9910 × L2-normalized 384-D vectors on
S³⁸³). Perfect monotonicity: 21/21 reply-different when the residue moved,
9/9 byte-identical when it didn't. Peak per-turn Δ-residue measured at
**0.0747**, matching the closed-form non-associativity bound of **7.6pp**
to within rounding (0.0013). The deep-trace held coherence through 86°
accumulated rotation across 10 turns — exactly 10 × 8.6° per turn, linear.

**3. Geometric machinery doing real work in user-facing behavior.**
The non-associativity meter that started as a math sanity check turned out
to be a **conversation-stationarity signal**: 4-of-4 stationary sessions
show monotonic decay at ~2pp per turn toward the calibrated floor. Same
infrastructure, two readable surfaces. Geometric structure showing up as
useful product behavior is what the substrate is for.

The full audit trail — eight per-layer commits + ~15 cross-team
correspondence docs + four Python validation suites (15/15 PASS across
v1–v4) + the per-layer contract / real-data / e2e tests — is in
[`theory/kahler_upgrade/`](theory/kahler_upgrade/) and
[`tests/kahler_*`](tests/). The language layer on top is specified in
[`GIGI_LANG_SPEC.md`](GIGI_LANG_SPEC.md) +
[`GIGI_SCHEMA_INTROSPECTION_SPEC.md`](GIGI_SCHEMA_INTROSPECTION_SPEC.md).

---

## What's new in late May 2026 — the SUDOKU + SAMPLE_TRANSPORT sprint

Six waves of work landed on top of the brain catalog, taking the substrate
from "we have 12 brain primitives" to "we have a constrained-inference
meta-primitive that solves real problems across unrelated domains" plus a
neighborhood-sampling primitive that answers "what other points are
geometrically reachable from here?"

The work shares the same Davis-manifold machinery as the
**sudoky-energy** sister project (Bee Davis, U.S. Provisional Patent
Feb 2026 — a GPU-accelerated CSP solver using `K_loc` curvature scheduling
+ `V(c)` information value + Γ trichotomy routing + holonomy pruning).
sudoky-energy solves canonical CSPs (Sudoku, SAT, graph coloring); GIGI's
SUDOKU primitive applies the same Čech-cohomology pre-filter and curvature
diagnostics to bundle-record filtering.

### SUDOKU — constrained inference on a learned affordance manifold

The primitive: a consumer hands SUDOKU a constraint set; it returns
satisfying records, near-miss records (records that violate exactly one
constraint), a Pareto frontier of multi-violation alternatives, a
counterfactual relaxation menu, per-constraint diagnostics, and an
**honest tristate verdict** — `Sat` / `Unsat` / `Unknown` (the last meaning
"I didn't look enough to claim either", explicit by API design; most CSP
solvers conflate empty-result with no-such-thing).

Six waves of additive geometry, all behind the `kahler` feature flag, all
free for the diagonal-metric case and Mahalanobis-ready for FitMode::Full
bundles:

| Wave | What it adds |
|---|---|
| **W3** | Per-violation `relaxation_cost` (Kähler-natural z-score = `|actual − threshold| / field_std`). Per-constraint `SelectivityReport` (marginal filter count, binding flag). `RelaxationOption` menu — counterfactual "what if I bent this rule to value X" with data-derived thresholds, sorted by gain/cost. |
| **W4** | `Solution.quality_score` — depth into the satisfaction region (soft-constraint posterior under independent half-normal priors). `Eq(Vector)` violation cost upgraded from flat 1.0 to bundle-derived L2 distance — fixes the dishonest math where geometrically close embeddings were indistinguishable from geometrically far ones. |
| **W5** | `ParetoNearMiss` — Pareto frontier on (n_violations, total_cost). Generalizes single-violation near-misses; the k=1 slice equals the existing list. Cap scales with constraint count (was incorrectly hard-capped at 3). |
| **W6.1** | `SelectivityReport.raw_curvature` — `K_c` = fraction of records that fail this constraint regardless of others. High K_c + zero marginal = **redundant constraint** (covered by another). Maps to sudoky-energy's per-variable `K_loc` scheduling signal. |
| **W6.2** | Čech-cohomology **holonomy pre-flight** — O(C²) pairwise scan for *trivially* self-contradictory constraint pairs (`Eq(x,a)+Eq(x,b)`, `Le(x,c)+Ge(x,d)` with `d>c`, `Between` intervals disjoint, `IsIn` empty intersection, etc.). Fires before any record IO, returns `Unsat` with `pre_flight_unsat_reason` populated. Provably zero false positives by construction. |
| **S3.5** | **Puzzle expansion** — when the original constraint set is UNSAT (verdict or pre-flight), opt-in `expansion: { allowed: true }` walks the relaxation menu (best gain/cost first) and stops at the first relaxation that finds ≥1 solution. Sets advisory when no relaxation works. |

**Wire surface** (single endpoint, content-negotiated DHOOM ↔ JSON):

```
POST /v1/bundles/{name}/brain/sudoku
```

Returns `solutions[]`, `near_misses[]`, `verdict`, `coverage`,
`n_records_considered`, `selectivity[]` (with `K_c`),
`relaxations[]`, `pareto_near_misses[]`, optional
`pre_flight_unsat_reason`, optional `expansion_result` — every field
data-derived, no domain configuration. The same call works on a drug-
discovery bundle, an apartment bundle, a stock-screening bundle, a
sensor bundle — verified end-to-end across 24 distinct domains in the
demo set below.

### SAMPLE_TRANSPORT — curvature-bounded neighborhood sampling

When deterministic `TRANSPORT` returns one geodesic, `SAMPLE_TRANSPORT`
returns a neighborhood of `k` valid destinations within a curvature
budget τ:

```
N(p_src, τ) = { p ∈ E : d²(p_src, p) ≤ τ }
```

where `d² = (1 - cos θ) / 2 ∈ [0, 1]` (Double-Cover half-angle formula —
`S + d² = 1`). Candidates weighted by `exp(-β · d²)`, sampled without
replacement via the **Efraimidis-Spirakis priority algorithm** (r^(1/w)
keys, top-k). Per-candidate `curvature_k = 2 · √d²`; bundle-wide
`confidence = 1 / (1 + κ)`.

```
POST /v1/bundles/{name}/brain/sample_transport
GQL:  SAMPLE_TRANSPORT bundle FROM (k=v,...) ON FIBER (...) BUDGET τ N k [BETA β] [SEED s];
```

### #107 — brain endpoints work on reloaded (mmap+overlay) bundles

Pre-existing limitation closed: every brain endpoint had the guard
`as_heap().ok_or(404 "not heap-resident")`, so after any server restart
bundles reloaded from snapshot became inaccessible until manual
recreation — Marcella's refuse-gate broke on every deploy. Fix:
`OverlayBundle::to_temp_heap_store()` materializes the merged
(mmap base − tombstones + overlay) view into a fresh heap store in
~10ms per 10k records; new `heap_or_promote` adapter dispatches —
zero cost on heap, one-shot promote on overlay. **15 brain endpoints
updated; live verified on `gigi-stream.fly.dev` after deploy
(4,961 bundles / 12.8M records reloaded with zero loss).**

### The eight worked-example demos (under `e2e/probes/`)

Each demo is self-contained — no shared schema, no shared config, just
the wire endpoint and a synthetic-but-realistic bundle. Together they
exercise every wave's functionality across **24 distinct domains**.

| Demo | What it shows |
|---|---|
| [`sudoku_six_domains_demo.py`](e2e/probes/sudoku_six_domains_demo.py) | Wave 3 baseline — drug discovery, real estate, recipes, hiring, stock screening, music playlists. Headline: relaxation cost in σ-units, binding constraint, gain-per-bend menu. |
| [`sudoku_six_more_domains_demo.py`](e2e/probes/sudoku_six_more_domains_demo.py) | Wave 4 — used cars (multi-numeric Pareto), restaurants (many-SAT quality rank), flights (timestamp-as-numeric), books, sensors (Vector Eq geometric distance), HR. Surfaced + closed two GP gaps (vector cost, quality_score). |
| [`sudoku_geometry_diagnostics_demo.py`](e2e/probes/sudoku_geometry_diagnostics_demo.py) | Waves 5 + 6.1 + 6.2 — NYC apartments (K_c curvature table) + Clinical trial eligibility (Čech pre-flight catches `age<18 AND age≥65` without walking 300 patient records). The proof that "your constraints can't both hold" is mechanically distinct from "the world doesn't have what you want". |
| [`sudoku_expansion_demo.py`](e2e/probes/sudoku_expansion_demo.py) | S3.5 — drug discovery, real estate, clinical trials, double-UNSAT. Original UNSAT, expansion relaxes cheapest constraint, finds solutions; double-UNSAT case fires the advisory cleanly. |
| [`sudoku_at_scale_demo.py`](e2e/probes/sudoku_at_scale_demo.py) | 100–1000 record bundles — NYC apartments (500), drug discovery (1000 compounds), SP500-sized stock screen (500), restaurants city-wide (300), sensor fleet (200, 8D embeddings). Real server-side latency: 7–52 ms per call. |
| [`sudoku_32x32_grid_demo.py`](e2e/probes/sudoku_32x32_grid_demo.py) | The namesake at literal scale — solves a 32×32 sudoku grid (1024 cells, 30% empty) using the SUDOKU primitive as a per-cell oracle inside a constraint-propagation loop. 300/307 cells filled correctly in **795 ms** / **1.5 ms per call**. The 7 unresolved cells are "needs backtracking" candidates where SUDOKU correctly returned ≥2 valid digits. |
| [`sample_transport_demo.py`](e2e/probes/sample_transport_demo.py) | S4 — semantic analogy (2D unit-circle corpus; "walk" finds "walked", "walking", "run", "ran" within budget=0.3), music similarity, drug analog discovery, reproducibility. 18/18 checks. |
| [`preship_audit.py`](e2e/probes/preship_audit.py) | 25-check production gate — malformed-input fuzz, memory/payload bounds, persistence smoke (insert → query → restart → re-query → identical fingerprint). All 25 pass pre- and post-#107. |

---

## What's in this repo

### Engine (Rust, single crate — `Cargo.toml`)

| Module | What it does |
|---|---|
| `bundle` | Fiber bundle store, schema, query plans, vector metrics |
| `engine` | Query engine, mutation log, trigger manager, cache |
| `mmap_bundle` | Memory-mapped persistence (BundleRef / BundleMut / OverlayBundle) |
| `wal` | Write-ahead log — durability across restarts |
| `query` | GQL query execution + result shape |
| `parser` | GQL grammar — `CREATE BUNDLE`, `SECTION`, `COVER`, `INTEGRATE`, `CURVATURE`, `SPECTRAL`, `HOLONOMY`, `TRANSPORT`, `BETTI`, `ENTROPY`, `FREEENERGY`, `GEODESIC`, … |
| `crypto` | **GIGI Encrypt v0.2** — gauge encryption (OPAQUE / AES-GCM-SIV, INDEXED / AES-256-CMAC), affine numeric gauge |
| `coherence` | Field consistency / Davis field equations |
| `curvature` | Scalar curvature K, capacity C = τ/K, confidence 1/(1+K) |
| `gauge` | Structure-group transformations on the fiber |
| `hash` | The 64-bit GIGI hash for base-space addressing |
| `metric` | Fiber metrics (Euclidean, cosine, custom) |
| `invariant` | Project-invariant guards used by `WHERE` clauses |
| `aggregation`, `join` | `INTEGRATE`, `JOIN`, `PULLBACK` |
| `sheaf` | Sheaf cohomology — `BETTI`, `CONSISTENCY` |
| `spectral` | Graph Laplacian eigenvalue/eigenvector queries |
| `concurrent` | Lock-free reader / single-writer concurrency |
| `dhoom` | DHOOM wire protocol — JSON-compatible binary serialization; integral-Chern compression (L7.3) when `kahler` is on; arrays-of-primitives encoded inline via a `\x1F`-sentinel JSON field (round-trip safe for `{tokens: ["the","cat",...]}`-shaped records) |
| `observability` | Geometric logs (κ, KL, JS per query) |
| `convert` | JSON / CSV / SQL → DHOOM ingestion |
| `edge` | Local-first sync layer (mobile/IoT) |

Plus the Kähler-feature modules (gated on `--features kahler`; absent paths are bit-identical to the pre-upgrade engine):

| Module | What it does | Layer |
|---|---|---|
| `geometry::complex_structure` | `ComplexStructure` (J² = -I, enforced) | L1 |
| `geometry::forms` | `TwoForm` + `ClosedTwoForm` with discrete dB closedness check | L1 |
| `geometry::transport` | B-perturbed magnetic transport via RK4; cyclotron-conserving | L1.5 |
| `geometry::hadamard` | Hadamard substructure detection + `transport_along` / `transport_inverse` | L5 |
| `geometry::line_bundle` | `LineBundle` + Dirac integrality check (Wu-Yang) | L7.1 |
| `geometry::quantum_cohomology` | Frobenius/WDVV composition on toy manifolds (CPⁿ, Tⁿ, S²) + Riemann-Roch capacity | L7.5 / L7.7 |
| `geometry::toeplitz` | Berezin-Toeplitz operators with `ℏ ≥ 4 / embedding_dim` safety gate | L7.6 |
| `geometry::moment_map` | `MomentMap` + `InfinitesimalAction`; B-symplecticity validated; `measure_conservation` integrates Hamilton's equations and reports drift of `μ_ξ` along H-flow plus the pointwise invariance residual — Noether's "if and only if" both halves | L9 |
| `geometry::generative_flow` | `GenerativeFlow` keystone for the brain-primitives catalog: the SDE `ẋ = -∇H dt + √(2T) dW` (gradient half) and `ẋ = B⁻¹∇H` (Hamiltonian half) parametrized to deliver SAMPLE / FORECAST / DREAM / RECONSTRUCT as four boundary conditions on one generator. Convenience constructor `from_isotropic_gaussian()` plugs into L4's Welford stats so any bundle becomes a Friston-style generative model | L10 |
| `geometry::predictive_coding` | Three more brain primitives stacked on L10: `inpaint()` (constrained Langevin — lock some fields, sample the rest from the conditional density), `predict_one_step()` + `predict_one_step_natural()` (single Fisher-natural-gradient forward step — the brain's online predictive-coding update), `kernel_density_confidence()` + `confidence_normalized()` (kernel-density-estimate "I don't know" signal — separates known patients from out-of-cohort queries by 184 orders of magnitude in the demo) | L11 |
| `geometry::attention` + `geometry::memory` | Closes the brain-primitives catalog with the attention + memory pillar. `attend()` (softmax over `-‖q-x‖²/2σ²` — identical to a normalized Gaussian kernel), `focus()` (top-k attended → sub-bundle), `episodic_events()` (persistent-H₀ change-point detection via elder-rule on the sorted-values MST), `semantic_gist()` (wraps `BundleStore::morse_compress` under the brain-API name) | L12 |
| `geometry::bundle_stats` | One-pass Welford per-field empirical statistics — mean, std, min/max for numeric (with Bessel-corrected length-scale fallbacks for degenerate fields), value→count for categorical, component-wise mean + mean pairwise L2 length scale for Vector fields. Single source of truth for "what's a typical distance in this bundle" — feeds every Kähler-natural normalization downstream. Domain-agnostic by construction. | SUDOKU foundation |
| `geometry::sudoku` | The SUDOKU meta-primitive — `solve_constraints()` with the honest-coverage `Sat/Unsat/Unknown` tristate. Per-violation Kähler-natural `relaxation_cost`, per-constraint `K_c` curvature + selectivity, Pareto frontier of multi-violation near-misses, data-driven `RelaxationOption` menu sorted by gain/cost, Čech-style `check_constraint_holonomy()` pre-flight contradiction detection (O(C²), zero false positives), S3.5 `attempt_expansion()` for UNSAT puzzle relaxation. 41 unit tests + 6 HTTP wire-gate tests + 8 worked-example demos across 24 domains. | Waves 3–6.2, S3.5 |
| `geometry::sample_transport` | Curvature-bounded neighborhood sampling: `sample_transport_neighborhood()` with `d² = (1-cos θ)/2 ∈ [0,1]` half-angle formula, Efraimidis-Spirakis weighted-sampling-without-replacement (`r^(1/w)` priorities, top-k), exp(-β·d²) kernel, per-candidate `curvature_k = 2·√d²`, bundle-wide `confidence = 1/(1+κ)`. 13 geometry tests + 3 HTTP wire-gate + 4-domain worked example. | S4 |
| `graph::adjacency` | Dual principal/auxiliary adjacency operators | L2 |
| `graph::commutativity` | Group-algebra-centrality commutativity classifier | L2 |
| `cost::jacobi_estimator` | Jacobi-field cardinality bounds via Bishop / Günther | L3 |
| `discrete::hodge_complex` | `d_0` / `d_1` operators built from cell incidence; `d² = 0` enforced | L6 |
| `discrete::hodge_laplacian` | Δ_k = d†d + dd†, Betti via eigendecomposition | L6 |
| `discrete::morse` | Algebraic Morse compression; preserves cohomology | L6 |

### Binaries (`src/bin/` + `examples/`)

| Binary | Purpose |
|---|---|
| `gigi-server` | The cloud-hosted database — REST + WebSocket on port `3142` |
| `gigi-stream` | Streaming ingestion + subscription daemon (deployed at `gigi-stream.fly.dev`) |
| `gigi-edge` | Local-first edge node (mobile / on-device) |
| `gigi-convert` | CLI: JSON / CSV / SQL → DHOOM bundle |
| `gigi-stress` | Load + correctness stress harness |
| `nasa_atmo` | End-to-end NASA-atmosphere demo (`examples/nasa_atmosphere.rs`) |
| `kahler_tour` | One-run walk through every Kähler layer L1–L11 + DHOOM round-trip + PR-window endpoints, with concrete inputs / outputs / catalog refs. Requires `--features kahler`. (`examples/kahler_tour.rs`) |
| `predictive_coding_demo` | L11 INPAINT / PREDICT / SELF-MONITOR exercised on a real `BundleStore` holding 80 synthetic MIRADOR-style PK records. The SELF-MONITOR signal cleanly separates known patients from out-of-cohort queries by **184 orders of magnitude**. Requires `--features kahler`. (`examples/predictive_coding_demo.rs`) |
| `attention_memory_demo` | L12 ATTEND / FOCUS / EPISODIC / SEMANTIC on two real `BundleStore` scenarios: a 12-token semantic-embedding bundle (ATTEND correctly surfaces the 4 animals when queried with a cat-like embedding; FOCUS picks exactly the 3 vehicles for a vehicle-like query) and a 60-day PRISM-style transaction stream (EPISODIC detects a regime change at **1711× persistence ratio**). Requires `--features kahler`. (`examples/attention_memory_demo.rs`) |

### Benches (`benches/`)

- `o1_proof.rs` — empirically validates O(1) point-query bound
- `ingest_bench.rs` — bulk-insert throughput
- `tpch_bench.rs` — TPC-H comparison harness

### SDKs

- **Python** (`sdk/python/`) — `pip install gigi-client`. Pandas-aware.
- **JavaScript / TypeScript** (`sdk/js/`) — `@gigi-db/client`. Browser + Node.

### UIs

- **`dashboard/`** — operator dashboard (React/Vite)
- **`playground/`** — in-browser GQL REPL backed by a live `gigi-server`

### End-to-end & integration tests (`e2e/`)

Playwright + Node:

- `anomaly_test.mjs` — curvature-based anomaly detection through the live API
- `encrypt_v02_live_test.mjs` — Encrypt v0.2 round-trip against the running server
- `spike_test.mjs`, `spike_test2.mjs` — burst-load correctness
- `diagnose.mjs` — bundle-health diagnostics

### Theory & specs

The repo carries the math (`theory/*.tex`) and the build-ready specs alongside the
code so a reviewer can read the claim and the implementation in the same place:

- `GIGI_SPEC_v0.1.md` — the formal mathematical foundation (definitions 1.1 – 4.x)
- `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` + `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` — gauge encryption
- `GIGI_OBSERVABILITY_SPEC.md` — geometric logging / DHOOM event protocol
- `GIGI_AUTOMATIC_ANALYTICS_API.md` — "the analytics ARE the database response"
- `GIGI_PERSISTENCE_UPGRADE_SPEC.md` — WAL + mmap durability
- `GIGI_PRODUCT_SPECS.md` — the three-product surface (Convert · Stream · Edge)
- `GQL_SPECIFICATION.md` + `GQL_REFERENCE.md` + `GQL_ADDENDUM_v2.1.md` — the query language
- [`GIGI_LANG_SPEC.md`](GIGI_LANG_SPEC.md) — natural-language → GQL → fiber response (v0.1.1)
- [`GIGI_SCHEMA_INTROSPECTION_SPEC.md`](GIGI_SCHEMA_INTROSPECTION_SPEC.md) — public `/schema` endpoint with `@public` / `@gated` directive policy
- [`theory/kahler_upgrade/`](theory/kahler_upgrade/) — the Kähler upgrade catalog (16/21 items shipped through L1–L9) + per-layer implementation plan + Marcella substrate spec + Python validation suites + cross-team correspondence
- [`theory/post_kahler_directions/`](theory/post_kahler_directions/) — companion catalog: nine **post-Kähler** geometric programs from outside the Adachi lineage (Sasaki / contact, information geometry, optimal transport / Wasserstein, persistent homology, Gromov δ-hyperbolicity, tropical geometry, synthetic DG, noncommutative geometry, CAT(κ)). Same template — claim, proof sketch, validation status, applications, implementation pointers. 30/30 numerical checks pass.

---

## Quick start

### Run the server

```bash
cargo run --release --bin gigi-server
# → http://localhost:3142
```

### Create a bundle, insert, query (Python)

```python
from gigi import GigiClient
db = GigiClient("http://localhost:3142")

db.create_bundle("sensors",
    fields={"sensor_id": "categorical", "temp": "numeric", "humidity": "numeric"},
    keys=["sensor_id"])

db.insert("sensors", [
    {"sensor_id": "S-001", "temp": 22.5, "humidity": 60.1},
    {"sensor_id": "S-002", "temp": 19.3, "humidity": 71.4},
])

# Every read carries curvature + confidence
result = db.query("SECTION sensors AT (sensor_id='S-001');")
```

### GQL — a few of the geometric verbs

```gql
-- Point query — O(1) via the GIGI hash
SECTION sensors AT (sensor_id='S-001');

-- Aggregate over a base-space cover — O(|r|)
INTEGRATE temp OVER sensors COVER ALL;

-- Curvature of the bundle
CURVATURE sensors;

-- Spectral connectivity (Fiedler value)
SPECTRAL sensors;

-- Local Laplacian eigenmodes in a fiber subspace
SPECTRAL corpus ON FIBER (f11, f12) MODES 5;

-- Holonomy: how much does the fiber rotate around a categorical loop?
HOLONOMY corpus ON FIBER (f11, f12) AROUND tense_label;

-- Parallel transport between two records — explicit SO(2) rotation matrix
TRANSPORT corpus FROM (token_str='walk') TO (token_str='walked')
  ON FIBER (f11, f12);

-- Betti numbers — sheaf cohomology
BETTI sensors;

-- Encrypted-at-rest fiber, gauge-preserving
CREATE BUNDLE finance FIBER (
  amount NUMERIC ENCRYPTED,
  account TEXT ENCRYPTED INDEXED
);
-- κ, λ₁, anomaly detection still work — at native speed
```

See `GQL_REFERENCE.md` for the complete grammar (status table, complexity per verb,
EMIT / wire format options).

### Kähler-substrate HTTP endpoints (`gigi-stream`, deployed)

For downstream consumers that want the geometric primitives directly over
HTTP, `gigi-stream` exposes four endpoints under `/v1/` (added in the
PR-window sprint for Marcella's Hopf + Riemann-Roch wiring; wire shapes
pinned by [`tests/kahler_pr_window_marcella_contract.rs`](tests/)):

| Endpoint | What it does | Catalog |
|---|---|---|
| `POST /v1/quantum_cohomology/compose` | Frobenius / WDVV composition on toy manifolds (CPⁿ, Tⁿ, S²) | §2.10 |
| `POST /v1/quantum_cohomology/capacity` | Riemann-Roch capacity — `dim H⁰(L^k)` | §2.2 |
| `POST /v1/bundles/{name}/holonomy_debt` | Davis non-decoupling — `Quantized(n)` vs `Continuous(x)` | §E.1 |
| `POST /v1/bundles/{name}/flat_transport` | Classical / magnetic parallel transport with `BSource` selector | §1.5 |

Plus the brain-primitives surface (`POST /v1/bundles/{name}/brain/*`, content-negotiated DHOOM ↔ JSON, all polymorphic over heap and mmap+overlay bundles per #107):

| Endpoint | What it returns | Layer |
|---|---|---|
| `/brain/sample` | Friston-FEP Langevin samples from `p ∝ exp(-H)` | L10 |
| `/brain/dream`, `/forecast`, `/reconstruct` | SAMPLE variants under different boundary conditions | L10 |
| `/brain/inpaint` | Constrained Langevin — lock some fields, sample the rest | L11 |
| `/brain/predict` | Single Fisher-natural-gradient step | L11 |
| `/brain/confidence`, `/confidence_with_explain` | Kernel-density confidence + nearest-record explain path | L11 (Marcella refuse-gate) |
| `/brain/attend`, `/focus` | Softmax over geodesic distance + top-k sub-bundle | L12 |
| `/brain/episodic` | Persistent-H₀ change-point detection | L12 |
| `/brain/semantic` | Morse-compressed gist | L12 |
| `/brain/explain` | Interpolation path to nearest known record | L12 |
| `/brain/fit_diagnostics`, `/distance_to_fit_mean` | Σ eigenstructure + Mahalanobis distance to fit mean | wave 1 |
| `/brain/sudoku` | Constrained inference — see SUDOKU section above | waves 3–6.2, S3.5 |
| `/brain/sample_transport` | Curvature-bounded neighborhood sampling | S4 |

### One-shot tour of every shipped Kähler layer

```bash
cargo run --release --features kahler --bin kahler_tour
```

Walks L1 (J, B), L1.5 (transport), L2 (adjacency commutativity), L3 (Jacobi
cardinality), L4 (Kähler curvature decomposition), L5 (Hadamard detection),
L6 (Hodge + Morse), L7 (line bundle, holonomy debt, quantum cohomology,
Toeplitz, Riemann-Roch capacity), L9 (moment map / Noether), plus the
DHOOM array-of-primitives round-trip and a summary of the four PR-window
endpoints. Each section prints concrete inputs and outputs with catalog
references. Source at [`examples/kahler_tour.rs`](examples/kahler_tour.rs).

---

## Build, test, run

```bash
# Build everything (engine + 5 production bins + 3 benches + 2 examples)
cargo build --release
cargo build --release --features kahler   # adds the kahler_tour example bin

# Run the full test suite — unit + integration tests in src/ and tests/
cargo test --release

# Run benches
cargo run --release --bin bench_o1
cargo run --release --bin bench_ingest
cargo run --release --bin bench_tpch

# E2E against a running gigi-server
cd e2e && npm install && npm test
```

As of this README the engine ships with:

- **680 tests passing, 0 failed** on the default build (no `kahler` feature) — byte-equal to pre-Kähler-upgrade GIGI by the optionality contract (was 674 pre-SUDOKU; +6 from the wave-2 + #107 work that's structural enough to run feature-off).
- **909 tests passing, 0 failed** with `cargo test --lib --features kahler` (+ 64 in `cargo test --bin gigi-stream --features kahler`) — adds the twelve-layer Kähler stack (L1–L12, all 12 brain primitives operational), the SUDOKU meta-primitive (waves 3–6.2 + S3.5), SAMPLE_TRANSPORT (S4), the #107 polymorphic brain-endpoint fix, per-layer real-data smokes against the 20-record sensor dataset, the cross-team contract tests pinning each consumer-facing API shape, and six new HTTP wire-gate tests verifying that every wave-3/4/5/6 field reaches the response and the Čech pre-flight + Pareto + expansion paths return correctly.

The Python validation suites independently verify the math from three
independent angles:

- `theory/kahler_upgrade/validation/*.py` — 15/15 PASS across v1–v4
  (Adachi commutativity, magnetic trajectory, Hadamard-Cartan,
  trajectory-ball volume, moment map, spectral gap, prequantization
  integrality, Frobenius/WDVV, index theorem, Berezin-Toeplitz, Hodge
  cohomology, Kähler curvature decomposition, quantized holonomy debt,
  DHOOM Chern round-trip).
- `theory/post_kahler_directions/validation_tests.py` — 30/30 PASS
  across the nine post-Kähler directions (Sasaki Reeb characterization,
  Fisher metric on Gaussians, Wasserstein W₂, MST persistence,
  Gromov-δ closed forms, tropical fundamental theorem, dual-number
  derivatives, Connes distance on S¹, CAT(κ) comparison inequality).
- `theory/brain_primitives/validation_tests.py` — 26/26 PASS for the
  twelve brain-like primitives (SAMPLE Langevin convergence, FORECAST
  harmonic energy conservation, DREAM temperature scaling, RECONSTRUCT
  MAP recovery, INPAINT conditional sampling, PREDICT natural-gradient
  step, ATTEND Gaussian-kernel softmax identity, FOCUS top-k
  correctness, EPISODIC persistent H₀ on time slices, SEMANTIC Morse
  Betti preservation, SELF-MONITOR Fisher precision decay).

Every check pairs an independently-derived closed-form ground truth
with a negative control.

---

## Geometric encryption (Encrypt v0.2)

`src/crypto.rs` ships **gauge encryption** — the structure group of the fiber
bundle is itself the cipher. The result is encryption that preserves every
geometric quantity GIGI computes:

| Quantity | Plaintext | Encrypted | Match? |
|---|---|---|---|
| Scalar curvature K | ✓ | ✓ | exact |
| Confidence 1/(1+K) | ✓ | ✓ | exact |
| Capacity C = τ/K | ✓ | ✓ | exact |
| Spectral gap λ₁ | ✓ | ✓ | exact |
| Anomaly scores | ✓ | ✓ | exact |
| Holonomy δφ | ✓ | ✓ | exact (gauge-invariant) |
| WHERE / range comparisons | ✓ | ✓ | preserved order on numeric fields |

Three modes:

1. **OPAQUE** (`AES-GCM-SIV`) — random-access ciphertext, no equality leakage.
2. **INDEXED** (`AES-256-CMAC`) — deterministic for indexed lookups; equality leaks by design (it's what lets the index work).
3. **AFFINE** (numeric gauge) — `v ↦ a·v + b` per fiber field, preserves variance/range² ratios. The original v0.1 substrate.

All NIST-standardized primitives, all from the RustCrypto suite. Spec:
`GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` and `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md`.

---

## What plugs into GIGI

- **Marcella** (NLP) — first consumer of the Kähler substrate. Runtime reads
  `BundleStore::kahler_curvature` / `spectral_gap_cached` / `hadamard_regions`
  / `morse_compress` / `transport_along` / `holonomy_debt` and surfaces them
  in self-inspect alongside a non-associativity meter that doubles as a
  conversation-stationarity signal. Refuse-gate hits `/brain/confidence_with_explain`
  every conversational turn — now survives server restarts cleanly via the
  #107 polymorphic adapter. Substrate spec:
  [`theory/kahler_upgrade/marcella_substrate.md`](theory/kahler_upgrade/marcella_substrate.md).
  Cross-team correspondence (8 letters) lives alongside it.
- **KRAKEN** (sensor fusion) — DAS / sonar / SAT / SIGINT bundles, CUSUM state, decisions, audit log, operator judgments — all on GIGI.
- **ICARUS** — sprint deliverables across `Transport`, `Holonomy`, `GaugeTest`, `SpectralFiber`, and `Divergence` verbs.
- **DHOOM** (`src/dhoom.rs`) — the canonical wire protocol used by every client.
- **GIGI Lang** — natural-language → GQL → fiber-shaped response. Spec at
  [`GIGI_LANG_SPEC.md`](GIGI_LANG_SPEC.md); SDK skeleton at
  [`sdk/python/gigi/lang.py`](sdk/python/gigi/lang.py) with contract tests
  pinning the shape; schema introspection at
  [`GIGI_SCHEMA_INTROSPECTION_SPEC.md`](GIGI_SCHEMA_INTROSPECTION_SPEC.md).
- **sudoky-energy** (sibling project, not in this repo) — Bee Davis's
  GPU-accelerated CSP solver (U.S. Provisional Patent Feb 2026). Solves
  the world's hardest 9×9 Sudoku puzzles in 20–49 ms on a single laptop
  GPU; **260,042 puzzles/sec** batch throughput. Shares the Davis-manifold
  machinery with GIGI's SUDOKU primitive: same `K_loc` curvature
  scheduling signal, same `V(c) = ∫_{R_c} K_loc dV_g` information value
  for ordering, same Γ trichotomy parameter for difficulty classification,
  same Čech `H̆¹` holonomy obstruction for pruning. sudoky-energy solves
  canonical CSPs; GIGI's SUDOKU applies the same machinery to bundle-
  record filtering. The cross-reference is documented in
  [`theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md`](theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md).

---

## Layout

```
gigi/
├── src/                  Rust engine (single crate, 25+ modules)
│   ├── lib.rs            module roots
│   ├── bin/              5 production binaries
│   ├── geometry/         Kähler L1–L12 + the SUDOKU sprint:
│   │                       L1   complex_structure, forms (J, B)
│   │                       L1.5 transport (B-perturbed magnetic)
│   │                       L5   hadamard
│   │                       L7   line_bundle, quantum_cohomology,
│   │                              toeplitz
│   │                       L9   moment_map (Noether)
│   │                       L10  generative_flow (Friston-FEP keystone)
│   │                       L11  predictive_coding (INPAINT/PREDICT/SELF-MONITOR)
│   │                       L12  attention, memory (ATTEND/FOCUS/EPISODIC/SEMANTIC)
│   │                       —    bundle_stats (W3 foundation)
│   │                       —    sudoku (W3–6.2 + S3.5)
│   │                       —    sample_transport (S4)
│   ├── graph/            L2 adjacency + commutativity classifier
│   ├── cost/             L3 Jacobi-field cardinality estimator
│   ├── discrete/         L6 Hodge complex + Laplacian + Morse
│   ├── sheaf/            sheaf cohomology + Laplacian
│   ├── bundle.rs         Heap BundleStore + Welford field stats + mutation_counter
│   ├── mmap_bundle.rs    BundleRef / BundleMut / OverlayBundle —
│   │                       polymorphic over heap and mmap+overlay (#107)
│   └── …
├── benches/              3 cargo-bin benchmarks
├── examples/             nasa_atmosphere.rs; kahler_tour.rs (every
│                         Kähler layer); predictive_coding_demo.rs (L11
│                         INPAINT/PREDICT/SELF-MONITOR on 80 MIRADOR
│                         PK records); attention_memory_demo.rs (L12
│                         on a 12-token corpus + 60-day PRISM stream)
├── e2e/
│   ├── probes/           8 SUDOKU + SAMPLE_TRANSPORT worked examples
│   │                       across 24 distinct domains + preship audit
│   │                       (sudoku_six_domains_demo, sudoku_six_more_
│   │                       domains_demo, sudoku_geometry_diagnostics_
│   │                       demo, sudoku_expansion_demo, sudoku_at_
│   │                       scale_demo, sudoku_32x32_grid_demo,
│   │                       sample_transport_demo, postdeploy_smoke,
│   │                       preship_audit)
│   └── *.mjs             Playwright + Node integration tests
├── sdk/
│   ├── python/           gigi-client (pandas-aware)
│   └── js/               @gigi-db/client (TS, browser + node)
├── dashboard/            Operator dashboard (React/Vite)
├── playground/           In-browser GQL REPL
├── theory/
│   ├── kahler_upgrade/   Kähler catalog (16/21 shipped) + impl plan +
│   │                       Marcella substrate spec + 4 Python validation
│   │                       suites (15/15 PASS) + cross-team correspondence +
│   │                       SUDOKU_PRIMITIVE_SPEC.md (sudoky-energy cross-ref)
│   ├── post_kahler_directions/
│   │                     Companion catalog: 9 post-Kähler directions
│   │                       (Sasaki, info-geom, OT, persistent homology,
│   │                       Gromov δ, tropical, synthetic DG, NCG, CAT(κ))
│   │                       + validation_tests.py (30/30 PASS)
│   └── brain_primitives/ 12 brain-like operations + 26/26 numerical checks
├── docs/                 Site + landing pages
├── demos/                Self-contained Python demos
└── *_SPEC.md             Build-ready specs (encryption, observability, …)
```

---

## Project status

Active. The engine is the substrate for several Davis Geometric products
(KRAKEN, Marcella, ICARUS, the Just-Gigi creator stack). Sprints land in
the open with TDD: each spec carries a v0.x section that maps to a passing
test in `cargo test`, and each landing-page claim is tied to a spec
section.

**Not in this README** are runtime data, the operational deploy workflow, and
operator-only restore tooling — those live in private channels.

---

## License

MIT. © Davis Geometric.

The mathematical content (the fiber-bundle representation of relational
data, the gauge encryption construction, the geometric query language, the
DHOOM wire protocol) is the subject of provisional patents; the *code* in
this repository is MIT-licensed.
