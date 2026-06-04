# GIGI

**Geometric Intrinsic Global Index** — a fiber-bundle database engine.

> Records are sections of a fiber bundle. Keys live on the base space; values
> live on the fiber. Curvature, spectral connectivity, holonomy, and confidence
> are **properties of the bundle** — they update incrementally with every
> insert and ride along on every query response. Geometry is not a plugin.

[![License: PolyForm Noncommercial 1.0.0](https://img.shields.io/badge/license-PolyForm%20NC%201.0.0-blueviolet.svg)](LICENSE)
[![Rust 2021](https://img.shields.io/badge/rust-2021-orange.svg)](Cargo.toml)

> **Licensing:** GIGI is released under the **PolyForm Noncommercial License 1.0.0** —
> free for personal use, research, education, and nonprofit/government use.
> **Commercial use is not granted by this license** and is reserved by the
> copyright holder under a separate commercial agreement. See [License & commercial use](#license--commercial-use) below.

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
| **Look up by primary key** | B-tree (O(log N)) or hash (O(1)) | `SECTION bundle AT (id='X');` — GIGI hash G(K₁,…,Kₘ) → ℝ¤₂⁶⁴, **always O(1)**; response also carries κ + confidence |
| **`WHERE x = 5`** | Index scan or full table scan | Same O(1) GIGI hash on the addressed key; with geometric annotation per result |
| **`WHERE x BETWEEN 10 AND 20`** | Range scan over a sorted index | `filtered_query` over the bundle; returns hits + per-hit position in the fiber |
| **`GROUP BY region`** | Hash-grouped aggregation | `INTEGRATE field OVER bundle COVER ALL;` — aggregation = integration over a base-space cover (geometric, not relational) |
| **`JOIN orders ON customers`** | Hash join / merge join | `PULLBACK orders ALONG customers` — pullback of one bundle along a shared base map |
| **`COUNT(*)`, `AVG(x)`, `STDDEV(x)`** | Scan, or precomputed summary tables | Welford stats maintained incrementally on every insert — **O(1) read**, never stale |
| **Detect anomalies / outliers** | Add a streaming pipeline + outlier model | Curvature κ already updated per insert; outliers are points where κ spikes — no separate pipeline |
| **"How clustered is my data?"** | Run k-means or DBSCAN offline | `SPECTRAL bundle;` — Fiedler value λ₁ from the index Laplacian, cached and incrementally refreshed |
| **"Are A and B related geometrically?"** | Foreign key + join | `TRANSPORT bundle FROM (id=A) TO (id=B) ON FIBER (...);` — explicit parallel transport, returns the SO(n) rotation matrix |
| **Encrypt sensitive columns** | Column encryption — usually breaks indexes | **Gauge encryption v0.4** preserves κ, λ₁, anomaly scores, holonomy at **native speed**. Six modes (Affine / Probabilistic / Opaque / Indexed / Isometric / Identity); SQL analytics (SUM/AVG/VAR/STDDEV exact on ciphertext via closed-form inverses); PQ-safe trusted + threshold delegation (ML-KEM-768 + Shamir K-of-N); public deterministic invariant verification (`/v1/bundles/{name}/verify_invariant`) |
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

---

## Why GIGI

Conventional databases see rows. GIGI sees a section σ: B → E of a fiber
bundle (E, B, F, π, Φ): the base space B is the queryable keys, the fiber
F is the value schema, and every record is a point in the total space E.
This isn't decoration — it's how the engine indexes, queries, and reasons
about your data:

| You want… | Conventional DB | GIGI |
|---|---|---|
| O(1) point query by composite key | Multi-column hash index | GIGI hash G : K₁ × … × Kₘ → ℝ¤₂⁶⁴ — native |
| Anomaly detection | Add a streaming pipeline | Curvature κ updated per insert; outliers fall out |
| "How clustered is this?" | Run k-means offline | Spectral gap λ₁ from the index Laplacian |
| Compute on encrypted data | Homomorphic encryption (~10,000× slowdown) | Gauge encryption — **native speed**, geometry-preserving |
| Logging with semantic insight | Text logs + sampling | DHOOM events with κ, KL-div, JS-div per query |
| NLP fiber geometry (tense, morphology, …) | Vector DB + bespoke analysis | `HOLONOMY corpus ON FIBER (f11, f12) AROUND tense_label` |
| Long-context theorems on a substrate | "trust me" + benchmarks | Kähler-upgrade catalog with cross-team math validation matching observations to rounding precision |

---

## Recent changes

The detailed dated entries that used to live here have been moved to
[CHANGELOG.md](CHANGELOG.md). Quick map of the most recent ships
(newest first):

- **2026-06-04 — Sharding initiative complete + atomic sheaf commits Phase 1 + Marcella SwDA WIN** ([details](CHANGELOG.md#2026-06-04--sharding-complete--atomic-sheaf-commits-phase-1--marcella-swda-win)). 14/14 sharding math gates green; cross-atlas BETTI (T9) Rust port closes the last queued item; 2PC transactions ship with all 5 failure scenarios validated; Marcella's discourse-flow probe returns CI=[+0.0434, +0.0634] on structured moves — the IMAGINE substrate earns its seat exactly where the protocol predicted.
- **2026-06-03 (evening) — IMAGINE / WALK** ([details](CHANGELOG.md#2026-06-03-evening--imagine--walk-lands-extrapolation-verbs-with-marcellas-trust-envelope)). Extrapolation verbs with Marcella's load-bearing trust envelope: provenance compile-time enforced, `max_imagined_curvature = 4.0 = K(CP¹ FS)` default, FORECAST/IMAGINE routing anchored to Gate J's θ_density = 0.5.
- **2026-06-03 (afternoon) — Sharding lands as substrate** ([details](CHANGELOG.md#2026-06-03-afternoon--sharding-lands-as-substrate-not-as-compromise)). Ten TDD-gated math claims (T1–T10) + Phase A/B scaffolds; the framing flip from "sharding is a compromise" to "sharding is sheaf-glued by construction."
- **2026-06-03 — LOCAL_HOLONOMY + intent_gate perf fix + PolyForm NC license** ([details](CHANGELOG.md#2026-06-03--local_holonomy-5th-cognitive-geometry-verb--intent_gate-perf-fix--polyform-nc-license)). 5th Cognitive Geometry verb (Marcella's gain-gate signal); ~5 s → 0 ms on empty-constraints intent_gate; license transition to PolyForm Noncommercial 1.0.0.
- **2026-06-02 — SEMANTIC perf rewrite** ([details](CHANGELOG.md#2026-06-02--semantic-perf-rewrite-rank-based-betti)). Dense Laplacian eigendecomposition → sparse F₂ Gaussian elimination on boundary matrices. **2260× speedup** on T² 12×12; MorseCache layer adds O(1) second+ reads.
- **2026-05-30 — Cognitive Geometry verbs (Branch VII)** ([details](CHANGELOG.md#2026-05-30--cognitive-geometry-verbs-branch-vii)). CAPACITY · HORIZON · DEPTH · PERCEIVE — geometric scalars translated into builder-facing routing decisions.
- **Late May 2026 — GIGI Encrypt v0.3 + v0.4** ([details](CHANGELOG.md#late-may-2026--gigi-encrypt-v03--v04-ship)). Gauge-mode completion + the full delegation family (BLS12-381 pairing PRE, ML-KEM-768 PQ, lattice K-of-N threshold) + the invariant verification layer; Zenodo paper deposited.
- **Late May 2026 — SUDOKU + SAMPLE_TRANSPORT** ([details](CHANGELOG.md#late-may-2026--the-sudoku--sample_transport-sprint)). Constrained-inference meta-primitive verified across 24 domains + curvature-bounded neighborhood sampling.
- **2026-05-29 — Encryption paper + vector_cache** ([details](CHANGELOG.md#2026-05-29--encryption-paper-deposit--vector-search-cache)). Paper deposit; new caching primitive backing the brain endpoints.
- **2026 — the Kähler upgrade** ([details](CHANGELOG.md#2026--the-k%C3%A4hler-upgrade)). GIGI v3: twelve layers of Kähler machinery extending the fiber-bundle substrate; all 12 brain primitives operational; three companion catalogs; cross-team validation matching observation to rounding precision.

---

## The Kähler upgrade in one paragraph

GIGI v3 shipped the **Kähler upgrade**: twelve layers of geometric
machinery extending the fiber-bundle substrate with a complex structure J,
a closed 2-form B, and everything
that follows from the pair — Hadamard substructure detection,
holomorphic curvature decomposition, Morse compression, line-bundle
integrality checks, quantum cohomology on toy manifolds,
Berezin-Toeplitz operators, Riemann-Roch representational capacity,
moment-map / Noether conservation along Hamiltonian B-flows,
the Friston-FEP keystone (generative flow on the Kähler bundle that
parametrizes its boundary conditions to deliver SAMPLE / FORECAST /
DREAM / RECONSTRUCT as one piece of infrastructure), predictive-coding
primitives (INPAINT, PREDICT, SELF-MONITOR), and the attention +
memory pillar (ATTEND, FOCUS, EPISODIC, SEMANTIC). All 12 brain
primitives operational. The catalog closes at **16 of 21 items
shipped** — 100% of the items classified as ship-able.

GIGI ships three companion catalogs:

- [`theory/kahler_upgrade/`](theory/kahler_upgrade/) — Kähler
  catalog, 16/21 shipped; 15/15 Python validation tests pass.
- [`theory/post_kahler_directions/`](theory/post_kahler_directions/) —
  nine post-Kähler geometric programs (Sasaki, information geometry,
  OT/Wasserstein, persistent homology, Gromov δ-hyperbolicity,
  tropical, synthetic DG, NCG, CAT(κ)); 30/30 numerical checks pass.
- [`theory/brain_primitives/`](theory/brain_primitives/) — twelve
  brain-like operations forced by one master equation
  `ẋ = B⁻¹∇(−log p)` on the Kähler bundle — the same equation Friston
  writes down for variational free-energy minimization. 26/26 checks.

The first downstream consumer (Marcella) ran a 30-prompt A/B harness
on her real `marcella_source_embeddings_bge` substrate. Perfect
monotonicity: 21/21 reply-different when residue moved, 9/9 byte-
identical when it did not. Peak per-turn Δ-residue = **0.0747**,
matching the closed-form non-associativity bound of **7.6pp** to
within rounding (0.0013). Deep-trace held coherence through 86°
accumulated rotation across 10 turns — exactly 10 × 8.6° per turn,
linear. The full audit trail is in
[`theory/kahler_upgrade/`](theory/kahler_upgrade/).

---
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
| `crypto` | **GIGI Encrypt v0.4** — six gauge modes (Identity / Affine / Probabilistic / Opaque AES-GCM-SIV / Indexed AES-256-CMAC / Isometric O(k)), per-field encryption pipeline (`GaugeKey::encrypt_fiber`) |
| `aggregate_helpers` | v0.3 — client-side closed-form aggregate inverters (SUM/AVG/VAR/STDDEV exact under Affine + Probabilistic; MIN/MAX/RANGE exact under Affine, refused under Probabilistic σ>0 with explicit `*_unchecked` opt-in) |
| `integrity` | v0.3 — Curvature-MAC HMAC-SHA256 over the canonical π_inv tuple; `InvariantTuple` + `sign_bundle` / `verify_bundle` |
| `invariant_verify` | v0.4 Sprint N — public deterministic verification with bundle-id binding; `verify_invariant_statement` returns `Verified` / `BundleMismatch` / `Rejected{field}` |
| `credentials` | v0.4 Sprint O — HMAC-bound credentials today; BBS+ unlinkability pinned as v0.5 |
| `invariant_ring` | v0.4 Sprint O — falsification harness for I_Aff membership; parser-by-construction proof |
| `membership_index` | v0.4 Sprint P — geodesic-ball Mahalanobis index with dimension-aware χ² threshold (table-exact for {1..5}×{0.95, 0.99}, Wilson-Hilferty otherwise) |
| `delegation`, `pairing_delegation`, `mlkem_delegation`, `lattice_delegation` | v0.3 Sprint J family — Aff(ℝ) capability composition (J.1), BLS12-381 pairing PRE (J.2), ML-KEM-768 trusted delegation (J.3), Shamir K-of-N × ML-KEM threshold lattice delegation (J.4) |
| `threshold` | v0.3 Sprint L — Shamir secret sharing over secp256k1 F_p; info-theoretic on ≤K−1 subsets |
| `ledger` | v0.3 Sprint K — RFC 6962 Merkle holonomy ledger (gauge-invariant audit log) |
| `ratchet` | v0.3 Sprint M — HKDF chain for continuous forward secrecy on the integrity key |
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
| `vector_cache` | Cached `(N, D)` materialized matrices for vector-search brain endpoints (`intent_gate`, `confidence`, `confidence_with_explain`). Architecture mirrors `BundleFlowCache`: `RwLock<HashMap>` hot read, per-key `Arc<Mutex<()>>` single-flight on miss, `mutation_counter` invalidation, capacity bound with random eviction. `MaterializedMatrix` holds contiguous row-major data + precomputed per-row `–·–²`; distance queries use the cosine identity in one autovectorizable loop. Public helpers: `kde_raw_from_matrix`, `max_density_cached` (lazy per-bandwidth), `kde_normalized_cached`, `nearest`. Env var `GIGI_VECTOR_CACHE_SIZE` (default 64). |
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
| `geometry::toeplitz` | Berezin-Toeplitz operators with `ℝ ≥ 4 / embedding_dim` safety gate | L7.6 |
| `geometry::moment_map` | `MomentMap` + `InfinitesimalAction`; B-symplecticity validated; `measure_conservation` integrates Hamilton's equations and reports drift of `μ_ξ` along H-flow plus the pointwise invariance residual — Noether's "if and only if" both halves | L9 |
| `geometry::generative_flow` | `GenerativeFlow` keystone for the brain-primitives catalog: the SDE `ẋ = -∇H dt + √(2T) dW` (gradient half) and `ẋ = B⁻¹∇H` (Hamiltonian half) parametrized to deliver SAMPLE / FORECAST / DREAM / RECONSTRUCT as four boundary conditions on one generator. Convenience constructor `from_isotropic_gaussian()` plugs into L4's Welford stats so any bundle becomes a Friston-style generative model | L10 |
| `geometry::predictive_coding` | Three more brain primitives stacked on L10: `inpaint()` (constrained Langevin — lock some fields, sample the rest from the conditional density), `predict_one_step()` + `predict_one_step_natural()` (single Fisher-natural-gradient forward step — the brain's online predictive-coding update), `kernel_density_confidence()` + `confidence_normalized()` (kernel-density-estimate "I don't know" signal — separates known patients from out-of-cohort queries by 184 orders of magnitude in the demo) | L11 |
| `geometry::attention` + `geometry::memory` | Closes the brain-primitives catalog with the attention + memory pillar. `attend()` (softmax over `-–q-x–²/2σ²` — identical to a normalized Gaussian kernel), `focus()` (top-k attended → sub-bundle), `episodic_events()` (persistent-H₀ change-point detection via elder-rule on the sorted-values MST), `semantic_gist()` (wraps `BundleStore::morse_compress` under the brain-API name) | L12 |
| `geometry::bundle_stats` | One-pass Welford per-field empirical statistics — mean, std, min/max for numeric (with Bessel-corrected length-scale fallbacks for degenerate fields), value→count for categorical, component-wise mean + mean pairwise L2 length scale for Vector fields. Single source of truth for "what's a typical distance in this bundle" — feeds every Kähler-natural normalization downstream. Domain-agnostic by construction. | SUDOKU foundation |
| `geometry::sudoku` | The SUDOKU meta-primitive — `solve_constraints()` with the honest-coverage `Sat/Unsat/Unknown` tristate. Per-violation Kähler-natural `relaxation_cost`, per-constraint `K_c` curvature + selectivity, Pareto frontier of multi-violation near-misses, data-driven `RelaxationOption` menu sorted by gain/cost, ÄŒech-style `check_constraint_holonomy()` pre-flight contradiction detection (O(C²), zero false positives), S3.5 `attempt_expansion()` for UNSAT puzzle relaxation. 41 unit tests + 6 HTTP wire-gate tests + 8 worked-example demos across 24 domains. | Waves 3–6.2, S3.5 |
| `geometry::sample_transport` | Curvature-bounded neighborhood sampling: `sample_transport_neighborhood()` with `d² = (1-cos θ)/2 ∈ [0,1]` half-angle formula, Efraimidis-Spirakis weighted-sampling-without-replacement (`r^(1/w)` priorities, top-k), exp(-β·d²) kernel, per-candidate `curvature_k = 2·√d²`, bundle-wide `confidence = 1/(1+κ)`. 13 geometry tests + 3 HTTP wire-gate + 4-domain worked example. | S4 |
| `graph::adjacency` | Dual principal/auxiliary adjacency operators | L2 |
| `graph::commutativity` | Group-algebra-centrality commutativity classifier | L2 |
| `cost::jacobi_estimator` | Jacobi-field cardinality bounds via Bishop / Günther | L3 |
| `discrete::hodge_complex` | `d_0` / `d_1` operators built from cell incidence; `d² = 0` enforced | L6 |
| `discrete::hodge_laplacian` | Δ_k = d d + dd , Betti via eigendecomposition | L6 |
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
- `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` + `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` + `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` + [`theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md`](theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md) — gauge encryption v0.2 → v0.3 (full delegation family + aggregate inversion) → v0.4 (invariant verification + credentials + K-preserving characterization + geodesic-ball membership)
- [`theory/encryption/paper_geometric_encryption_v0.1.tex`](theory/encryption/paper_geometric_encryption_v0.1.tex) — the load-bearing encryption paper (Aff(ℝ) trusted-delegatee model + pairing-PRE BDH-hard delegation + threshold lattice delegation; honest carveouts for Probabilistic MIN/MAX/RANGE bias and threshold-vs-PRE trust-model dependence)
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

-- Cognitive Geometry verbs (Branch VII)
-- "Can this substrate hold this interpretation?"   — CAPACITY = τ/K
CAPACITY corpus;
-- "How deep does coherent context extend?"        — HORIZON = τ/(K·ℝ“_c)
HORIZON corpus;
-- "What's the erasure energy of writing here?"    — DEPTH classifier I/II/III/IV
DEPTH corpus;
-- "What does the substrate perceive this vector as?"
PERCEIVE corpus ROTATION (r00, r01, r10, r11) VECTOR (v0, v1) DIM 2;

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

Plus the **Cognitive Geometry verbs** (Branch VII — builder-facing routing decisions derived from the static geometric scalars; the five verbs together cover *can I hold this?* / *how deep does coherence run?* / *what's the erasure cost of writing here?* / *what does the substrate see this vector as?* / *how trustworthy is the recent coherence regime?*):

| Endpoint | What it returns | Reference |
|---|---|---|
| `GET  /v1/bundles/{name}/capacity` | `τ / K` — can the substrate hold this interpretation? | CGC Thm 8.1 |
| `GET  /v1/bundles/{name}/horizon` | `τ / (K · ℝ“_c)` — coherent-context length before frame rotation becomes irrecoverable | CGC Thm 8.6 |
| `GET  /v1/bundles/{name}/depth` | Erasure-energy classifier I/II/III/IV — what's the cost of writing here? | CGC Thm 8.14 |
| `POST /v1/bundles/{name}/perceive` | `(R_acc · v,  –R_acc − I–_F)` — what does the substrate perceive this vector as, and how much should we trust the perception? | CGC §8 step 4 |
| `POST /v1/bundles/{name}/local_holonomy` | `(R_window, defect, coherence, interpretation)` — windowed-rotation coherence signal for gain gating | COHERENCE_SIGNAL_SPEC §3 |

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

- **847 tests passing, 0 failed** on the default build (no `kahler` feature) — byte-equal to pre-Kähler-upgrade GIGI by the optionality contract (LOCAL_HOLONOMY and PERCEIVE are feature-independent and run here too; was 680 pre-CG-verbs, +6 LOCAL_HOLONOMY, +others from Branch VII).
- **1124 tests passing, 0 failed** with `cargo test --lib --features kahler` (+ 64 in `cargo test --bin gigi-stream --features kahler`) — adds the twelve-layer Kähler stack (L1–L12, all 12 brain primitives operational), the **five Cognitive Geometry verbs** (CAPACITY / HORIZON / DEPTH / PERCEIVE / LOCAL_HOLONOMY) end-to-end (math + HTTP + GQL parser + real-data smokes + cross-team contract pins), the SUDOKU meta-primitive (waves 3–6.2 + S3.5), SAMPLE_TRANSPORT (S4), the #107 polymorphic brain-endpoint fix, the rank-based Betti rewrite + MorseCache + column-indexed F₂ rank (`/brain/semantic` went from 10–30 s to sub-second on production-shape complexes), per-layer real-data smokes against the 20-record sensor dataset, and the six HTTP wire-gate tests verifying that every wave-3/4/5/6 field reaches the response and the ÄŒech pre-flight + Pareto + expansion paths return correctly.
- **1153 tests passing, 0 failed** with `cargo test --lib --features "kahler sharded"` — adds the [`src/sharded/`](src/sharded/) module (Phase A scaffold + Phase B `ShardedBundle` wrapper) behind the `sharded` feature flag. 29 new sharded tests cover `Atlas` / `ChartId` / `Transition` / `SpectralRegime` types + the `non_vacuity_check` and `cocycle_budget_check` gates + `sharded_write_resolve` Clean Finger Move resolver + `ShardedBundle::wrap_trivial` runtime wrapper with atlas serde round-trip. Feature OFF by default → zero regression for callers who haven't opted in.
- **1187 tests passing, 0 failed** with `cargo test --lib --features "kahler sharded imagine"` — adds the [`src/imagine/`](src/imagine/) module (Phase 1 scaffold: `ImaginedRecord` with required provenance, `imagine_geodesic` RK4 integrator, `imagine_halo` gauge-equivariant k-NN halos, `walk` with Marcella's load-bearing curvature safety envelope at default 4.0 = K(CP¹)). 23 new imagine tests include: RK4 matches embedded-picture S² closed form to machine precision; halo records carry correct provenance prefix; `walk` refuses paths exceeding the 4.0 curvature ceiling with `OverCurvatureRefused`; cite-render contract produces the `[imagined: ...]` / `[imagined-halo: ...]` / `[imagined-bridge: ...]` prefixes. Feature OFF by default; zero impact on existing consumers.

Plus **three Python TDD math gates for the IMAGINE / WALK extrapolation verbs** (see [`theory/imagine/validation/`](theory/imagine/validation/)):

```bash
python theory/imagine/validation/run_all.py
# -> ALL 3 IMAGINE GATES GREEN, ~3s wall clock
```

T11 (geodesic integrator on S²/T²/CP¹), T12 (halo-as-IMAGINE makes sharded CURVATURE partition-invariant — zero residual across {2, 4, 8} partitions), T13 (double cover monodromy resolution + discourse-state seam at `act_history=("qy",)`).

Plus a **separate Python TDD suite of 10 math gates** for the sharded substrate (see [`theory/poincare_to_sharding/validation/`](theory/poincare_to_sharding/validation/)):

```bash
python theory/poincare_to_sharding/validation/run_all.py
# -> ALL 10 TDD GATES GREEN, ~15s wall clock
```

Each gate cites its claim, references its source paper (Davis 2026a / b / c, Hatcher, Horn-Johnson, Nakahara, Saad), implements ground truth + claim-under-test through INDEPENDENT paths, and documents its circular-logic guards. Three gates (T2, T5, T6) were red-then-green during development and caught real math errors before they made it into the spec.

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

## Geometric encryption (Encrypt v0.2 → v0.3 → v0.4)

`src/crypto.rs` + the v0.3/v0.4 modules (`integrity`, `aggregate_helpers`,
`delegation` family, `ratchet`, `ledger`, `threshold`, `invariant_verify`,
`credentials`, `invariant_ring`, `membership_index`) ship **gauge
encryption** — the structure group of the fiber bundle is itself the
cipher. The result is encryption that preserves every geometric
quantity GIGI computes:

| Quantity | Plaintext | Encrypted | Match? |
|---|---|---|---|
| Scalar curvature K | ✓ | ✓ | exact |
| Confidence 1/(1+K) | ✓ | ✓ | exact |
| Capacity C = τ/K | ✓ | ✓ | exact |
| Spectral gap λ₁ | ✓ | ✓ | exact (graph-topology invariant) |
| Anomaly scores | ✓ | ✓ | exact |
| Holonomy δφ | ✓ | ✓ | exact (gauge-invariant — including HOLONOMY ON FIBER) |
| WHERE / range comparisons | ✓ | ✓ | preserved order on numeric fields |
| SUM / AVG / VAR / STDDEV | ✓ | ✓ | **plaintext-exact via O(1) client-side closed-form inverse** (v0.3 `aggregate_helpers`) |
| MIN / MAX / RANGE under Affine | ✓ | ✓ | exact |
| MIN / MAX / RANGE under Probabilistic σ>0 | ✓ | ✗ | refused at API (`BiasedUnderProbabilisticNoise`); `*_unchecked` opt-in if you accept the `Θ(σ √(2 log n) / \|a\|)` bias |
| **π_inv fingerprint** (K, λ₁, ⟨Hol⟩, τ, β₀, β₁) | ✓ | ✓ | **publicly verifiable, no gauge key required** (v0.4 Sprint N) |

**Six v0.2 gauge modes** + **v0.3 delegation family** + **v0.4 verification**:

| Layer | Primitives |
|---|---|
| **v0.2 gauge** | IDENTITY, AFFINE (numeric `v ↦ a·v + b`), PROBABILISTIC (Affine + i.i.d. Gaussian noise), OPAQUE (AES-256-GCM-SIV — random-access ciphertext, no equality leakage), INDEXED (AES-256-CMAC PRF — deterministic for indexed lookups, equality leaks by design), ISOMETRIC (O(k) rotation on Vector fields) |
| **v0.3 integrity** | Curvature-MAC (HMAC-SHA256 over canonical π_inv tuple; 10⁻¹⁰ quantization; 4× tighter than v0.3.0) |
| **v0.3 aggregate inversion** | Client-side closed-form decoders for SUM / AVG / VAR / STDDEV exact on Affine + Probabilistic; MIN / MAX / RANGE exact on Affine; honest refusal on Probabilistic σ>0 |
| **v0.3 audit log** | RFC 6962 Merkle holonomy ledger (gauge-invariant) |
| **v0.3 forward secrecy** | HKDF-chain RG-flow ratchet on the integrity key |
| **v0.3 delegation family** | J.1 Aff(ℝ) capability composition · J.2 BLS12-381 pairing PRE (DLP_G₂-hard, pre-quantum) · J.3 ML-KEM-768 trusted-delegatee (FIPS 203, NIST Level 3 PQ) · J.4 lattice threshold = Shamir K-of-N over F_p × per-share ML-KEM (info-theoretic on ≤K-1 subsets + PQ outer layer) |
| **v0.3 secret sharing** | Shamir over secp256k1 base field F_p (Sprint L); the primitive J.4 composes |
| **v0.4 Sprint N** | Public deterministic invariant-tuple verification; `POST /v1/bundles/{name}/verify_invariant`; bundle-id binding; `Verified` / `BundleMismatch` / `Rejected{field}` verdicts |
| **v0.4 Sprint O** | Credential-gated invariant queries (HMAC-bound today; BBS+ pinned as v0.5 unlinkability upgrade) |
| **v0.4 Sprint Q** | K-preserving subgroup characterized as the diagonal affine group `(ℝ*)áµ ⋉ ℝáµ`; rotation-invariant `tr(Cov)/diam²` (corrects earlier `(max−min)²` overclaim); LWE separation as hiding-vs-gauge layers. **Roadmap only** — not a shipped PQ mode |
| **v0.4 Sprint P** | Geodesic-ball Mahalanobis membership index with dimension-aware χ² threshold (table-exact for k ∈ {1..5} × p ∈ {0.95, 0.99}; Wilson-Hilferty fallback elsewhere). Explicit leakage scope: not a hiding primitive |

**Rigor** (cross-team review-driven, locked in by tests):

- 25 Rust integration tests in `tests/fhe_pq_parity_rigor.rs`
- 12 Rust integration tests in `tests/invariant_verify_v0_4.rs`
  (including end-to-end through real `EncryptionMode::Affine` and
  `EncryptionMode::Indexed` write paths under multiple gauge seeds)
- 6 + 6 + 5 Rust integration tests for Sprints O / P / Q
- Python oracle `validation_tests_fhe_pq_rigor.py` (66/66 assertions)
- Python oracle `validation_tests_v0_4_sprint_n.py` (17/17 assertions)
- Paper [`theory/encryption/paper_geometric_encryption_v0.1.tex`](theory/encryption/paper_geometric_encryption_v0.1.tex)
  with two review-driven honest carveouts (§1.4 FHE parity scope; §1.4
  threshold-vs-PRE trust-model dependence)

All NIST-standardized primitives, all from the RustCrypto suite +
`bls12_381` + `ml-kem` + `hkdf` + `num-bigint`. Specs:
`GIGI_GEOMETRIC_ENCRYPTION_SPEC.md`,
`GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md`,
`GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md`,
[`theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md`](theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md).

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
  same ÄŒech `HÌ†¹` holonomy obstruction for pruning. sudoky-energy solves
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

## License & commercial use

**Copyright © 2025–2026 Bee Rosa Davis. All rights reserved.**

GIGI is released under the **[PolyForm Noncommercial License 1.0.0](LICENSE)**
([canonical text](https://polyformproject.org/licenses/noncommercial/1.0.0)).

### What's covered for free

Per the PolyForm Noncommercial license, **any noncommercial purpose is a
permitted purpose**. That explicitly includes:

- **Personal use** — research, experimentation, testing for the benefit of
  public knowledge, personal study, private entertainment, hobby projects,
  amateur pursuits, and religious observance, *without any anticipated
  commercial application*.
- **Noncommercial organizations** — charitable organizations, educational
  institutions, public research organizations, public safety or health
  organizations, environmental protection organizations, and government
  institutions. This applies *regardless of the source of funding* or
  obligations resulting from the funding.

The license includes a **patent license scoped to noncommercial use** —
i.e., the patent claims listed below are licensed for use *within* the
permitted noncommercial scope.

### What's NOT covered (commercial use is reserved)

Commercial use of GIGI is **not granted by the PolyForm license** and is
reserved to the copyright holder. "Commercial" includes — but is not limited
to — building a paid product on top of GIGI, embedding GIGI in a SaaS or
hosted service offered to paying customers, redistributing GIGI as part of
a commercial offering, and using GIGI inside any organization that does not
qualify as a "noncommercial organization" under the license terms (§
"Noncommercial Organizations" in [LICENSE](LICENSE)).

If you want to use GIGI commercially, **you need a separate commercial
license from the copyright holder.** See *Commercial licensing* below.

### Patents

The mathematical constructions underlying GIGI — the fiber-bundle
representation of relational data, the curvature / spectral-gap / holonomy
machinery, the gauge encryption suite, the DHOOM wire protocol, the
discrete Hodge / Betti / Morse pipeline, the Cognitive Geometry verbs
(CAPACITY / HORIZON / DEPTH / PERCEIVE), the LOCAL_HOLONOMY coherence
signal, the SUDOKU constraint-satisfaction primitive — are the subject of
provisional and granted patents held by the copyright holder. The PolyForm
license grants patent rights *only for permitted (noncommercial) use*; all
commercial patent rights are reserved.

### Commercial licensing

For commercial use — including product use, hosted/SaaS use, paid
redistribution, or any use inside a for-profit organization that isn't
covered by the "Noncommercial Organizations" definition — contact the
copyright holder to negotiate an exclusive or non-exclusive commercial
license.

The standard commercial structure is a separate written agreement
covering: copyright license to the relevant code, patent license to the
relevant claims, scope (field of use, geography, exclusivity), term, and
fees. Marcella, the Davis Geometric corporation, and any production
deployment of GIGI on commercial substrate operate under such an
agreement.

### Compatibility note (no copyleft contamination)

The PolyForm Noncommercial License is **not** a copyleft license. It does
not contaminate derivative works in the GPL/AGPL sense. You may build on
top of GIGI for any noncommercial purpose without any obligation to
release your additions under PolyForm. The only requirements are the
standard ones: include the LICENSE text (or a link to the canonical URL),
preserve the `Required Notice:` copyright line, and stay within the
permitted-purpose scope.

### A note on prior releases

Earlier iterations of this README claimed an "MIT" license. **No LICENSE
file was ever published in this repository under MIT terms.** As of this
commit, the licensing is explicitly PolyForm Noncommercial 1.0.0 going
forward; any prior informal use is fact-of-the-matter whatever it was, and
the going-forward terms are what's in LICENSE today.
