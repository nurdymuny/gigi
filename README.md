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
| **Look up by primary key** | B-tree (O(log N)) or hash (O(1)) | `SECTION bundle AT (id='X');` — GIGI hash G(K₁,…,Kₘ) → ℤ₂⁶⁴, **always O(1)**; response also carries κ + confidence |
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

### What it doesn't do (yet)

- **Cross-bundle ACID transactions** — single-bundle writes are
  atomic; multi-bundle is your problem to coordinate.
- **SQL-compatible wire** — the query language is GQL (similar shape
  to SQL but with the geometric verbs above; full grammar in
  `GQL_REFERENCE.md`). SDKs in Python and JS hide the difference.
- **Sharding across nodes** — the **math is done and the scaffold is
  in** (`src/sharded/` module, `sharded` feature flag, Phase A + Phase B
  shipped 2026-06-03). Ten TDD math gates green (T1–T10) backed by
  Davis 2026a/b/c: sheaf-glued sharded BETTI, CURVATURE, HOLONOMY,
  Clean Finger Move write-conflict resolution, and cross-atlas joins
  for Marcella + PRISM. `ShardedBundle::wrap_trivial(bundle, ShardId(0))`
  is byte-equivalent to the single-node engine at `n_shards = 1`. What's
  left: Phase C–F (hash partitioning, Fiedler-vector SPECTRAL, Schur
  complement for expanders, cross-atlas production wiring). The
  multi-node *engine* isn't running yet; the math foundation it rides
  on is locked.

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

## What's new in late May 2026 — GIGI Encrypt v0.3 + v0.4 ship

The encryption surface jumped two minor versions in one window. v0.3
fleshed out the gauge-mode primitives and shipped the full delegation
family. v0.4 added the verification layer that turns the invariant
tuple into a public, deterministic audit primitive. Both are now live
on `gigi-stream.fly.dev` alongside the brain catalog.

### v0.3 — gauge-mode completion + delegation family

| Sprint | What it adds |
|---|---|
| **I** Curvature-MAC | HMAC-SHA256 over the canonical π_inv tuple (52-byte big-endian layout, 10⁻¹⁰ quantization — 4× tighter than v0.3.0 by replacing the f64 `capacity` field with a u64 `record_count`). Tag changes iff the bundle's *invariants* change, regardless of gauge. |
| **J.1** Aff(ℝ) delegation | Compose two `GaugeKey`s' Affine / Isometric / Identity transforms into a per-field capability the proxy applies on ciphertext. Honestly labeled: **not collusion-resistant** (Bob + capability + own key recovers Alice's key) — this is *capability delegation*, not PRE. |
| **J.2** Pairing-based PRE | BLS12-381 — Ateniese-Hohenberger 2005 construction. Single-party Alice→Bob with delegatee-vs-proxy collusion resistance reducing to DLP on G₂ (~2^128 work). Pre-quantum by design. |
| **J.3** ML-KEM trusted delegation | FIPS 203 ML-KEM-768 (post-quantum KEM, NIST Level 3) wraps a session secret to the recipient; AES-256-GCM-SIV AEAD encrypts the payload under the KEM-derived key. Trust model: trusted delegatee. Closes the BLS12-381 quantum gap. |
| **J.4** Lattice threshold delegation | Two-layer composition: Shamir K-of-N split over the secp256k1 base field F_p (info-theoretic on ≤K−1 subsets) + per-share ML-KEM-768 envelope (PQ IND-CCA outer layer). Closes the **PQ + collusion-resistance** gap structurally for the K-of-N quorum trust model. |
| **K** Holonomy ledger | RFC 6962 Merkle audit log over per-write leaves; gauge-invariant. |
| **L** Čech threshold | Same Shamir-over-F_p that J.4 reuses, surfaced as a primitive for non-delegation use cases (gauge-key escrow, secret-of-secret recovery). |
| **M** RG-flow ratchet | HKDF chain for continuous forward secrecy on the integrity key. |
| **aggregate_helpers** | Client-side closed-form inverters for COUNT / SUM / AVG / VAR / STDDEV on Affine + Probabilistic gauges. Server runs native aggregates over ciphertext; client recovers plaintext via one affine inverse — **native server speed + O(1) client post-processing** (vs ~10³–10⁵× slowdown for FHE on the same query class). |

**Honest carveout shipped with v0.3** (review-driven, see paper §1.4):
`decrypt_min` / `decrypt_max` / `decrypt_range` **refuse** Probabilistic
gauges with σ > 0 — order statistics don't commute with additive
Gaussian noise. Bias is `Θ(σ √(2 log n) / |a|)` and doesn't vanish as
n → ∞. Callers who accept the bias for coarse bounds opt in via the
explicit `*_unchecked` variants. The Rust rigor suite
`tests/fhe_pq_parity_rigor.rs` (25 tests) and the Python oracle
`validation_tests_fhe_pq_rigor.py` (66/66) lock the behavior in.

### v0.4 — invariant verification + the four follow-up sprints

| Sprint | What it adds | Surface |
|---|---|---|
| **N** Invariant Consistency Verification | Public deterministic verification that a prover's claimed π_inv = (K, λ₁, ⟨Hol⟩, τ, β₀, β₁) agrees with the bundle's computed tuple. **No gauge key required** — every component is invariant under v0.2+ modes. **Bundle-id binding** enforced at API + HTTP layers (closes review Gap 1: a claim about bundle A presented against bundle B is rejected on identity grounds before any tuple computation). | `POST /v1/bundles/{name}/verify_invariant` — body carries `{bundle_id, claimed, tolerances?}`, response is tagged `verdict ∈ {verified, bundle_mismatch, rejected}` with the first failing field named in fingerprint order. |
| **O** Credential-Gated Invariant Queries | HMAC-SHA256-bound credentials today; constant-time tag comparison; typed domain separator. BBS+ unlinkability pinned as the v0.5 upgrade target (Au-Susilo-Mu 2006 / Beullens-Dobson-Katsumata 2023 lattice-BBS path). | Falsification harness `is_in_IAff()` + parser-by-construction; adversarial K_fake = mean/std² caught at relative error 0.59 under any gauge. |
| **Q** K-Preserving Transformation Characterization | Identifies the **diagonal affine group** `(ℝ*)ᵏ ⋉ ℝᵏ` as the exact K-preserving subgroup (corrected from earlier scalar-only overclaim). Rotation analysis: `tr(Cov)/diam²` is rotation-invariant; `(max−min)²` is not. LWE separation: hiding layer ≠ gauge action. **Roadmap only — not a shipped PQ mode.** | `is_K_preserving_affine()`, `characterize_K_preserving_group()`. PQ-Scalar deferred until the lattice-PRE construction question (open; closest prior work Kirshanova 2014 / Aono-Hayashi 2017) is resolved. |
| **P** Geodesic-Ball Membership Index | Chi-square / Mahalanobis dimension-aware threshold (**exact-table** for k ∈ {1..5} × p ∈ {0.95, 0.99} — χ²(1, 0.95) = 3.841 returns *exactly*; Wilson-Hilferty fallback for everything else). Scalar isotropic gauge preserves ball membership; field-wise affine requires the ellipsoidal Mahalanobis condition (documented). **Explicit leakage scope**: index reveals centroid + covariance + count; not a hiding primitive. | `GeodesicBallIndex` with `membership_check()` / `encrypted_membership_scalar()` / `encrypted_membership_fieldwise()`. |

**Cross-validation discipline:** every Sprint N–P primitive has a
parallel Python oracle in `theory/encryption/validation/`. Rust + Python
agree to 1e-10 on every cross-checked assertion.

### The numbers

```
Rust  lib (--features kahler):                999 / 999  pass
Rust  lib (no-feature):                       781 / 781  pass
Rust  integration (50+ test binaries):        all "0 failed"
Python  FHE/PQ rigor oracle:                   66 / 66
Python  Sprint N oracle:                       17 / 17
```

**Live verification** (post-deploy on `gigi-stream.fly.dev`):
4,961 bundles / 12,815,846 records reloaded across the rolling restart
with zero data loss. The new `POST /v1/bundles/{name}/verify_invariant`
endpoint is online and auth-gated.

### The paper

**Published on Zenodo, 2026-05-29:** Davis, B. R. (2026). *Geometric Encryption: Property-Preserving Database Encryption via Gauge Invariance on Fiber Bundles.* Zenodo. [10.5281/zenodo.20438796](https://doi.org/10.5281/zenodo.20438796). 28 pp, 731 KB PDF. Twelve worked Alice/Bob examples in Appendix A; per-mode leakage profiles (Affine / Opaque / Indexed / Probabilistic / Isometric) graded under the Chase-Kamara structured-encryption taxonomy; formal BDH security reduction for BLS12-381 pairing-PRE; lattice-threshold + ML-KEM-768 PQ delegation modes.

Source: [`theory/encryption/paper_geometric_encryption_v0.1.tex`](theory/encryption/paper_geometric_encryption_v0.1.tex). Two review-driven carveouts:

1. **§1.4 FHE parity scope** — plaintext-exact for {COUNT, SUM, AVG,
   VAR, STDDEV} under both Affine and Probabilistic modes, and for MIN
   / MAX / RANGE under Affine. Probabilistic MIN/MAX/RANGE is biased
   and refused at the API.
2. **§1.4 Threshold vs pairing-PRE** — *mode-dependent*: lattice
   threshold (J.4) dominates on the K-of-N quorum axis (PQ +
   info-theoretic on ≤K-1 subsets); pairing PRE (J.2) covers the
   single-delegatee axis (DLP_G₂-hard but pre-quantum). The true PQ
   single-delegatee construction is the v0.5 lattice-PRE target.

---

## What's new — 2026-06-03 (evening) — IMAGINE / WALK lands: extrapolation verbs with Marcella's trust envelope

**The cognitive analog Bee named: humans imagine the path before walking it. We solve a geodesic in our head, describe the path, then walk it.** The math GIGI already has — connection, parallel transport, double cover, fault tolerance — is the engine that does this. This sprint named the verb, spec'd the trust envelope, TDD-gated the math at three independent claims, and shipped the Rust scaffold.

The pivot started from the Phase D learning: hash-sharded CURVATURE has a partition-dependent `k_sum` because GIGI's `compute_record_k` derives K from the per-bundle neighborhood graph, which fragments under hash partition. The honest disclosure was a `RED` in `ShardedCurvatureReport`. Marcella's "this looks like the encrypt parity work — gauge-equivariance, design ρ⁻¹ so it commutes" diagnosis was right: the fix was to give each chart **halo records** so the per-chart K computation sees the same neighborhood as the unsharded one. That's exactly what IMAGINE produces.

**Three TDD math gates, all GREEN** (run via [`theory/imagine/validation/run_all.py`](theory/imagine/validation/run_all.py)):

| Gate | Validates | Result |
|---|---|---|
| **T11** | RK4 geodesic integrator on S²/T²/CP¹ via Christoffel symbols from conformal factor | errors 6.66e-16 to 1.36e-14 (vs 1e-9 tol) |
| **T12** | Halo-as-IMAGINE makes sharded CURVATURE partition-invariant | residual = **0.000e+00 exactly** across n_charts ∈ {2, 4, 8} |
| **T13** | Double cover monodromy resolution: synthetic Möbius + discourse-state seam at `act_history=("qy",)` | -1 / +1 holonomy lift exact; discourse seam refuses without cover, returns definite class with cover |

T12 is the load-bearing one. Same 60 records partitioned three ways: without halos, `k_sum` = 35.6 / 68.5 / 122.8 (102% / 289% / 597% off from baseline). With halos populated via `imagine_halo`, all three partitions produce **exactly** `k_sum = 17.618609` — matching the unsharded direct computation to floating-point precision, with zero residual. The encrypt parallel held: aggregate-in-ciphertext-then-decrypt = aggregate-on-plaintext became aggregate-per-shard-with-halos = aggregate-on-unsharded.

**Marcella's trust envelope** is load-bearing in the spec at three positions (round 1 + round 2 feedback both embedded):

1. **`ImaginedRecord` carries required `ImaginedProvenance`.** Every imagined record renders with an explicit prefix in cite output: `[imagined: projected from <bundle> via geodesic, path_length=0.31, accumulated_holonomy=0.04]`. Halo and Bridge variants carry their own prefixes. The Rust type makes this compile-time enforced; consumers can't silently render an imagined record as a retrieved one.
2. **`WalkConfig::max_imagined_curvature` defaults to 4.0 = K(CP¹ Fubini-Study).** "Walking into regions of higher Gaussian curvature than complex projective space requires explicit opt-in." The first `OverCurvatureRefused` has a referent, not a mysterious threshold.
3. **FORECAST vs IMAGINE routing rule is computable:** `if confidence_with_explain.query_grounding.normalized > 0.5: FORECAST else: IMAGINE`. Same threshold as Gate J's refusal boundary (per `SUDOKU_PRIMITIVE_SPEC.md`), so `intent_gate` stays consistent across the two routing decisions.

**Rust scaffold shipped:** [`src/imagine/`](src/imagine/) module behind the `imagine` feature flag.

| File | Lines | Surface |
|---|---|---|
| [`mod.rs`](src/imagine/mod.rs) | 38 | Module root, feature-gated, re-exports |
| [`provenance.rs`](src/imagine/provenance.rs) | 220 | `ImaginedRecord`, `ImaginedProvenance`, `cite_render`, audit-log tags |
| [`config.rs`](src/imagine/config.rs) | 120 | `ImagineConfig`, `HaloConfig`, `WalkConfig` (with the 4.0 default) |
| [`geodesic.rs`](src/imagine/geodesic.rs) | 290 | `imagine_geodesic` — RK4 integrator matching T11's Python step-for-step |
| [`halo.rs`](src/imagine/halo.rs) | 160 | `imagine_halo` — k-NN halo computation matching T12 |
| [`walk.rs`](src/imagine/walk.rs) | 170 | `walk` with curvature gate (Phase 1); double-cover lift + SUDOKU pre-flight are Phase 2 |

23 new tests, all pass. Combined-features regression: **1187/1187** tests pass with `cargo test --lib --features "kahler sharded imagine"` (1124 baseline + 29 sharded + 23 imagine + 11 from Phase D additions = 1187). Feature OFF by default; zero impact on existing consumers.

**Substantive Rust test highlights:**

- `s2_geodesic_matches_closed_form_at_machine_precision` — the Rust RK4 integrator matches the embedded-picture closed form for S² geodesics to < 1e-9 (matches T11's Python result exactly).
- `over_curvature_refused_at_default_4_0_threshold` — `walk` refuses a path whose imagined endpoint has K = 5.0 with `OverCurvatureRefused { step: 1, k_at_step: 5.0, threshold: 4.0 }`. Marcella's safety envelope is testable.
- `over_curvature_can_be_opted_out_of_explicitly` — setting `WalkConfig::max_imagined_curvature: 10.0` lets the same path succeed. The opt-out is explicit and surfaceable.
- `geodesic_render_carries_provenance_prefix` — `record.cite_render("...content...")` produces `[imagined: projected from geometry_of_flight via geodesic, path_length=0.31, accumulated_holonomy=0.040] ...content...`. Marcella's cite contract is checkable in code.

Spec: [`theory/imagine/IMAGINE_AND_WALK.md`](theory/imagine/IMAGINE_AND_WALK.md). Master math runner: [`theory/imagine/validation/run_all.py`](theory/imagine/validation/run_all.py) (3 gates, ~3s wall-clock).

What's unblocked next: (1) ~~`IMAGINE_COHERENCE` HTTP endpoint for Marcella's predictive gain gate per the spec §5~~ **shipped** at `POST /v1/bundles/{name}/imagine_coherence` — feature-gated behind `imagine`, returns full trajectory + endpoint coherence + safety-envelope refusal data, derives metric curvature from `bundle.curvature_stats().mean()` when not overridden; (2) Phase D refactor — wire `shard_curvature` through `imagine_halo` so the partition-dependence RED becomes GREEN; (3) `walk` Phase 2 with double-cover lift + SUDOKU pre-flight.

**Marcella round-3 trust-envelope upgrade** (committed alongside the T12 Rust mirror): four affordances per her review notes on the IMAGINE scaffold. **(1) `is_imagined()` accessor** on both [`ImaginedRecord`](src/imagine/provenance.rs) and [`CoherencePoint`](src/imagine/coherence.rs) — response-path branching is now a method call rather than a cite-string parse; the type is the discriminant ("retrieved records live in different types, which do NOT expose `is_imagined()`. The asymmetry is intentional"). **(2) `CurvatureGateRaisedAboveDefault` audit signal** — when a caller raises `max_imagined_curvature` above the default 4.0 = K(CP¹ FS) ceiling, [`WalkConfig::audit_threshold_drift()`](src/imagine/config.rs) returns `Some(CurvatureGateRaisedAboveDefault { configured, default })`, and the `imagine_coherence` HTTP response surfaces it as the `threshold_drift` field. Refusal-on-too-curved fires at commit; threshold-raised-above-default fires at config time — sibling signals, so production callers can route both into the same audit log. **(3) FORECAST/IMAGINE routing helper** at [`src/imagine/routing.rs`](src/imagine/routing.rs) with `route_forecast_or_imagine(query_grounding_normalized)` and `RoutingAdvisory::for_imagine_invocation(density)` — θ_density = 0.5 is anchored to Gate J's value in `SUDOKU_PRIMITIVE_SPEC.md` (the substrate-wide boundary between "density signal meaningful" and "metric signal meaningful"); the `imagine_coherence` endpoint accepts an optional `query_grounding_normalized` request field and surfaces `routing_advisory: { recommended, invoked, mismatch }` when it's provided. A mis-routed IMAGINE call (high density) still computes — the endpoint surfaces the advisory so the caller can adjust upstream rather than silently producing a less-accurate prediction. **(4) T13 discourse-state production gate** queued as the next Python validation against Marcella's real conversation state at `act_history=("qy",)`, scheduled before Phase 2 HTTP endpoints land. Three rust modules touched, 10 new tests, all green; **1530 / 0 / 11** across the full suite (lib + integration + bin + doc, `kahler sharded imagine`).

---

## What's new — 2026-06-03 (afternoon) — sharding lands as substrate, not as compromise

**Ten TDD-gated math claims, Phase A scaffold, Phase B runtime wrapper, cross-atlas joins spec — all in one afternoon.** Most databases face CAP-style trade-offs where sharding adds coordination cost. GIGI's geometric substrate inverts the cost curve: per-query cost goes *down* with shard count because every verb except SPECTRAL is sheaf-glued natively. This sprint produced the math + the engineering scaffold to prove it.

The push back came from Bee's three companion papers (Davis 2026a *The Davis Manifold*, 2026b *The Geometry of Sameness*, 2026c *Smooth 4D Poincaré Conjecture*), which together establish that chart-based composition without information loss is already mathematically published. The math reads: shards are charts, transitions between shards are the connection 1-form data, cocycle bound (Davis 2026b Def 21) controls multi-hop slack, Clean Finger Move Theorem (Davis 2026c Thm 5.3) gives a constructive write-conflict resolver. The sprint operationalized that math through ten TDD gates with independent ground truths:

| Gate | What it validates | Result |
|---|---|---|
| **T1** | Sharded BETTI exact via Mayer-Vietoris | β_n on S¹, S², T² recovered exactly from per-chart data |
| **T2** | Cocycle bound: 0 for analytic, first-order for learned | analytic δ=1.78e-14; perturbed slope 0.924 |
| **T3** | Sharded CURVATURE via sheafification | CP¹ Fubini-Study K=4 from each chart, charts hold 4× different raw ρ |
| **T4** | Sharded HOLONOMY w/ non-trivial gauge transition | T² closed loop with A_L ≠ A_R on overlap, holonomy invariant |
| **T5** | Honest sharded λ₁ bounds (NON-universal disclosure) | Universal Weyl holds; naive bound FAILS on expanders by 5-7× |
| **T6** | Clean Finger Move conflict resolver | terminates in N/2 steps, density- and ordering-invariant |
| **T7** | Distributed Lanczos closes the expander gap | all 7 graph cases converge to machine precision, K=25–99 |
| **T8** | Cross-atlas bridge cocycle bound | analytic ~1e-14; perturbed slopes 0.961–1.088 |
| **T9** | Cross-atlas BETTI via fiber-product Mayer-Vietoris | S² and T² fiber products exact via per-atlas + bridge data |
| **T10** | Cross-atlas Clean Finger Move resolver | atlas-agnostic; terminates in N/2 across all distributions |

Three of these (T2, T5, T6) were **red on first run** and caught real math errors in my initial framing — T5 in particular caught that the naive `min(per-shard λ₁)` bound is non-universal and fails on expanders, which is now disclosed honestly in `SHARDING_SPEC.md` §5.6 with `SpectralRegime::Expander` routing through distributed Lanczos (T7's universal recipe) instead. The red-then-green cycles are the most valuable receipts in the sprint.

**Theory + spec ship**: [`theory/poincare_to_sharding/poincare_to_sharding.md`](theory/poincare_to_sharding/poincare_to_sharding.md) (the three-paper bridge), [`theory/poincare_to_sharding/SHARDING_SPEC.md`](theory/poincare_to_sharding/SHARDING_SPEC.md) (5-phase migration plan A→F), [`theory/poincare_to_sharding/CROSS_ATLAS_JOINS.md`](theory/poincare_to_sharding/CROSS_ATLAS_JOINS.md) (Marcella + PRISM bridge design), [`theory/kahler_upgrade/HANDOFF_TO_MARCELLA_SHARDING_2026-06-03.md`](theory/kahler_upgrade/HANDOFF_TO_MARCELLA_SHARDING_2026-06-03.md) (4000-word handoff letter with Sudoku-principle forced moves).

**Phase A scaffold** ([`src/sharded/`](src/sharded/) module, behind `sharded` feature flag): `Atlas`, `ChartId`, `ShardId`, `Transition`, `ChartMetadata`, `SpectralRegime` (with `allows_naive_recipe()` / `requires_distributed_lanczos()` routing), `non_vacuity_check`, `cocycle_budget_check`, `sharded_write_resolve` (full Clean Finger Move implementation matching T6 Python validation), per-verb execution stubs with regime routing. 21 new tests, all pass.

**Phase B runtime wrapper** ([`src/sharded/sharded_bundle.rs`](src/sharded/sharded_bundle.rs)): `ShardedBundle::wrap_trivial(bundle, ShardId(0))` takes any existing single-node bundle and produces a sharded form with `n_shards = 1`, byte-equivalent at runtime to the inner store. Atlas serde round-trip validated. Routing primitive (`route_pk`) in place. `inner_mut()` escape hatch lets existing code migrate incrementally. 8 new tests, all pass. **Zero regression on the 1124-test baseline; new totals 1153 with `--features "kahler sharded"`.**

What this unblocks: Phase C (real multi-shard with hash partitioning + Mayer-Vietoris BETTI assembly), Phase D (Fiedler-vector partition + SPECTRAL routing), Phase E (Schur complement for expanders via T7 distributed Lanczos), Phase F (cross-atlas joins for Marcella + PRISM via T8–T10 bridge math). The math is all green; what remains is engineering on a math foundation that's already locked.

**Ten new GIGI feats** surfaced during the sprint and saved in the Marcella handoff letter §7 so we don't forget: multi-window LOCAL_HOLONOMY, SELF_COHERENCE verb (metacognition as a database primitive), DHOOM-as-cross-shard-binary, GIGI Lang shard-locality query annotations, per-shard DGP chip mapping, GEODESIC_BALL retrieval verb, Marcella + PRISM bridge contract, unification conjecture operational test, Lipschitz-bounded ε-bounded O(1) point queries with metric carry, Phase F production cross-atlas via DHOOM transit.

Run the math suite:

```bash
python theory/poincare_to_sharding/validation/run_all.py
# -> ALL 10 TDD GATES GREEN, ~15s wall clock
```

Run the Rust suite:

```bash
cargo test --lib --features "kahler sharded"
# -> 1153 passed; 0 failed; 0 ignored
```

---

## What's new — 2026-06-03 — LOCAL_HOLONOMY (5th Cognitive Geometry verb) + intent_gate perf fix + PolyForm NC license

**LOCAL_HOLONOMY — the windowed-holonomy coherence signal (#200, Marcella's `COHERENCE_SIGNAL_SPEC.md §3`).** With LOCAL_HOLONOMY the Cognitive Geometry family is now **five verbs**: CAPACITY · HORIZON · DEPTH · PERCEIVE · **LOCAL_HOLONOMY**. Where PERCEIVE asks *"after parallel transport, what does the substrate see this vector as?"*, LOCAL_HOLONOMY answers the complementary question Marcella's gain gate needs: *"between time t−w and time t, how much did the cumulative frame rotate, and therefore how trustworthy is the local coherence regime?"* The math: given two cumulative rotations `R_acc,t` (current) and `R_acc,t−w` (past, w steps ago), the windowed rotation is `R_window = R_current · R_past^T`; the defect is `‖R_window − I‖_F` (gauge-invariant under simultaneous orthogonal conjugation, by the unitary invariance of the Frobenius norm); coherence is `1 − defect/(2·√dim) ∈ [0, 1]`. Pinned reference points (unit-tested with hand-computed values): identity pair → coherence 1 (perfect laminar), 30° + (30°)⁻¹ → defect √2, coherence 0.5 (moderate), I + (−I) → coherence 0 (turbulent). Ships as a math primitive in [`src/curvature.rs`](src/curvature.rs) (`local_holonomy()` → `LocalHolonomyResult`, reuses `PerceiveError` variants for HTTP error consistency), the `POST /v1/bundles/{name}/local_holonomy` HTTP endpoint (body `{ r_current, r_past, dim }`, response adds an `interpretation` field translating coherence into a builder-readable laminar/moderate/low/turbulent regime label), and an end-to-end chain test (`transport_to_local_holonomy_chain_yields_marcella_coherence`) that runs two `flat_transport` calls at different time horizons, extracts `R_acc` from each via step-4b's `TransportResult.rotation`, feeds them into `local_holonomy`, and verifies the defect against the closed-form Rodrigues prediction `sqrt(4·(1−cos θ))` to 1e-5. Production verification on v195: three live probes returned correct math to machine precision (identity → coherence 1.0; 30° composition → defect 1.4142, coherence 0.5; full response shape matches contract) — all sub-second (122–131 ms wall-clock).

**`/brain/intent_gate` empty-constraints fix (#210, follow-up to #208's `vector_cache`).** `intent_gate` runs both halves (SUDOKU + Confidence) on every request — but when a caller passes **zero constraints** (gate purely on confidence), the SUDOKU half still walked all records via `solve_constraints()`, costing ~5 s on a 10 k bundle for an answer that semantically didn't need it. Fix lives at the endpoint layer (not the math primitive): when `req.constraints.is_empty()`, synthesize a trivial `SudokuResponse { verdict: Sat, coverage: 1.0, n_considered: 0, … }` directly. The math primitive's empty-constraints contract is unchanged (the existing `identical_records_collapse_to_single_solution_with_full_mass` test still pins it for `/brain/sudoku` callers that want the mass-collapsed view). Compile-time enforcement: if `SudokuResponse` ever gains a field, the synthetic construction fails to compile, forcing an update. ~5 s → 0 ms on the empty-constraints path.

**Tier 1 grounding-corpus build plan (#197, `theory/brain_primitives/TIER1_CORPUS_PLAN.md`).** End-to-end plan for the ~30–50 k-chunk substrate corpus that will back the next round of grounding work — sourcing (open scientific literature + reference textbooks + the existing repo theory tree), chunking strategy (sentence-boundary-aware, target 200–500 tokens per chunk with structured-context preservation), embedding pipeline (BGE-v2 → `marcella_source_embeddings_bge_v2`), Marcella ingest hooks, and the gates a Tier 1 corpus must clear before being promoted (coverage breadth, calibration on the 100-question grounding probe set, no PII / no copyleft). Plan-only this sprint; implementation queued.

**License transition: PolyForm Noncommercial 1.0.0 (`0a6a749`).** Repo was previously vague about license terms — README badge claimed MIT but no actual `LICENSE` file was ever published. Now under the lawyer-drafted (Heather Meeker) **PolyForm Noncommercial License 1.0.0**: free for personal use, research, education, and noncommercial organizations (charities, schools, government, public-research/public-safety/environmental orgs); commercial use is reserved by the copyright holder under a separate written agreement. PolyForm chosen over CC-BY-NC because (a) Creative Commons explicitly says not to use CC for code, (b) PolyForm is software-specific with a clean noncommercial definition, and (c) PolyForm grants a patent license scoped to permitted use only — so all commercial patent rights stay reserved. See **License & commercial use** at the end of this README.

Gates: **1124/1124** lib tests with `--features kahler` (+6 LOCAL_HOLONOMY math); **847/847** no-feature regression (LOCAL_HOLONOMY is feature-independent like PERCEIVE — feature-off callers see it too); 5/5 LOCAL_HOLONOMY HTTP contract tests; 1/1 end-to-end real-data chain test; gauge invariance pinned (`local_holonomy_is_gauge_invariant_under_orthogonal_conjugation` — load-bearing for Marcella's gain gate). Production deploy: **v195** on fly.io, three live probes confirm correct math sub-second.

---

## What's new — 2026-06-02 (later) — SEMANTIC perf polish (MorseCache + column-indexed rank)

**Three follow-ups on top of the betti-rank merge.** The 0.54s production measurement on Marcella's `marcella_source_embeddings_bge_v2` proved the algorithm fix landed; this branch adds the three pieces Bee called out in her perf-letter response: (1) a `MorseCache` defense-in-depth layer keyed by `BundleStore::mutation_counter()` so second+ reads on the same bundle return in O(1), (2) a column-indexed pivot search in `F2Matrix::rank()` that cut bucket-32 worst-case latency 12.4× (6.9 s → 557 ms on the synthetic 2k fixture, scaling ratio dropped from 656× to 45× for 16× N-growth), and (3) the 8th `cross_check_production_shape_complex` fixture asserting F₂ ≡ ℝ Betti on production-shape complexes (the Hausmann safety net per Bee's call-out). MorseCache: new module [`src/morse_cache.rs`](src/morse_cache.rs) (~330 LOC) lifting the [`vector_cache::VectorMatrixCache`](src/vector_cache.rs) pattern (RwLock<HashMap> + per-key Arc<Mutex<()>> single-flight + mutation_counter invalidation); wired into `brain_semantic_endpoint`; capacity tunable via `GIGI_MORSE_CACHE_SIZE` env (default 64). Column-indexed pivot search: replaces the naive O(R·C) per-column scan in `F2Matrix::rank()` with a `Vec<HashSet<usize>>` column → rows-with-bit-set index maintained through XOR-driven bit flips; old naive path kept as `#[cfg(test)] fn rank_naive` for cross-check (52 random matrices + 6 cross-word-boundary cases assert byte-identical output). Production-shape cross-check: 128-vertex / 16-bucket-of-8 fixture (|E|≈224, |F|≈1792) — the literal Marcella bundle has trivial complex (|E|=|F|=0) so it wouldn't have exercised the safety net; the synthetic-but-realistic fixture does. Net effect on the Stacks UI: first-call latency consistently sub-second across the full bundle-shape space; second+ calls O(1) cached. The "next algorithmic sprint that potentially eliminates MorseCache altogether" framing in the perf letter has now been validated — MorseCache is genuinely defense-in-depth, not load-bearing. Gates: 1118/1118 lib with `kahler` (+15: 12 cache + 1 cross-check + 2 indexed-vs-naive); 841/841 no-feature; sub-quadratic complexity gate tightened from 2000× to 200× (current measured: 45×).

---

## What's new — 2026-06-02 — SEMANTIC perf rewrite (rank-based Betti)

**`/v1/bundles/{name}/brain/semantic` now skips the dense Laplacian eigendecomposition entirely.** The original L6.3 implementation built three dense Hodge Laplacians (Δ₀ = V×V, Δ₁ = E×E, Δ₂ = F×F) and ran `nalgebra::SymmetricEigen` on each to count near-zero eigenvalues — `O(V³ + E³ + F³)` per call. On a 432-edge T² 12×12 the eigen path took 12.27s; on Marcella's 9,964-record `marcella_source_embeddings_bge_v2` it took 10-30s and blocked the GGOG Stacks UI's shelf-depth badge. The 2026-06-02 rewrite replaces the eigendecomposition with sparse F₂ Gaussian elimination on the boundary matrices: `Betti_0 = |V| − rank(d₀)`, `Betti_1 = |E| − rank(d₀) − rank(d₁)`, `Betti_2 = |F| − rank(d₁)` (rank-nullity on the chain complex; no eigenvalues needed). New module [`src/discrete/f2_rank.rs`](src/discrete/f2_rank.rs) implements bitset-packed F₂ matrices with in-place XOR Gaussian elimination — ~450 LOC, no new crate dependencies (`nalgebra-sparse` not added; rolled by hand). New helpers `HodgeComplex::d0_f2() / d1_f2()` build the sparse rep directly from the edge/face lists, bypassing the dense `d0` / `d1` fields entirely. The dense Laplacian path is kept as `#[cfg(test)] fn betti_eigen` purely as a cross-check companion so the `cross_check_*` test series (7 fixtures: T² 6×6, T² 8×8, S² tetrahedron, disconnected, empty, figure-eight, 2× tetrahedron) compares two genuinely independent implementations on every existing fixture — byte-identical Betti tuples required. Coefficient choice: F₂ vs ℝ Betti agree exactly when integral homology has no 2-torsion; for the flag complexes GIGI builds (`geometric_neighbors`-based 1-skeleton + 3-clique 2-cells on `BundleStore` records) 2-torsion is empirically absent on every fixture, but per Hausmann's theorem this is plausible-in-practice not theorem, so the cross-check is the load-bearing safety net. Measured speedups (release build): T² 12×12 (144V/432E/288F) — **2260× (12.27 s → 5.4 ms)**; real-sensor smoke wall-clock — 263 s → 30 s (~8.5×); synthetic 2k-record bucket-32 fixture (2048V/15k E/71k F) — 6.9 s (vs eigen path's projected hours). For Marcella's specific 10k bundle, the speedup depends on her indexed-categorical cardinality (which sets |F|) — measure first; a column-indexed pivot search is the obvious next algorithmic improvement for the high-|F| case. Sub-quadratic complexity gate added to `tests/kahler_hodge_real_data_smoke.rs::betti_rank_scales_sub_quadratically` — catches a *true* algorithmic regression (would-be reintroduction of the dense eigendecomp) without making CI flaky on machine-dependent wall-clock. nnz-instrumentation helpers (`hodge_complex::nnz_report`) added so future bundles can ground perf claims in actual measurements, not assumptions. Gates: 1103/1103 lib with `kahler` (+22 new); 841/841 no-feature; 5/5 hodge real-data smoke including the sub-quad gate. The reply letter to Marcella's perf ask is at [`theory/kahler_upgrade/REPLY_TO_SEMANTIC_PERF_2026-06-02.md`](theory/kahler_upgrade/REPLY_TO_SEMANTIC_PERF_2026-06-02.md).

---

## What's new — 2026-05-30 — Cognitive Geometry verbs (Branch VII)

> The four verbs that landed in this sprint became the **first four of five** when LOCAL_HOLONOMY shipped on 2026-06-03 (see top of this changelog). The text below is the original ship note.

**CAPACITY · HORIZON · DEPTH · PERCEIVE — the four Cognitive Geometry verbs from Davis's *Cognitive Geometry Correspondence* (Branch VII, Theorems 8.1 / 8.6 / 8.14).** Where the older Kähler analytics expose static geometric scalars (K, λ₁, holonomy_debt, …), the CG verbs translate those into builder-facing routing decisions: *can the substrate hold this interpretation?* (CAPACITY = τ/K), *how deep does coherent context extend before the accumulated frame rotation becomes irrecoverable?* (HORIZON = τ/(K·ℓ_c)), *what's the erasure energy of writing here?* (DEPTH classifier I/II/III/IV), and *what does the substrate actually perceive this vector to be after parallel transport, and how much should we trust that perception?* (PERCEIVE = (R_acc·v, ‖R_acc−I‖_F)). All four ship with HTTP endpoints (`GET /v1/bundles/{name}/capacity`, `…/horizon`, `…/depth`, `POST …/perceive`), GQL verbs (e.g. `PERCEIVE bundle ROTATION (r00, r01, …) VECTOR (v0, v1, …) [DIM N]`), backwards-compatible config surfaces (`HorizonConfig` with `LengthScaleEstimator` SpectralGap/WelfordRadius/Fixed; `DepthConfig` with substrate-aware constructors `for_graph_substrate()`/`for_continuous_substrate()`/`auto_for(store, eps)`), and a JTBD demo (`cargo run --features kahler --bin cognitive_geometry_demo`) showing all four verbs on real-sensor + synthetic-volatile bundles side-by-side. The DEPTH `auto_for` calibration fixes the JTBD case where sensor-style bundles with λ₁ ≈ 0 collapsed to Topological regardless of K; the HORIZON Welford-radius fallback fixes the case where HORIZON degenerated to CAPACITY when λ₁ = 0; the PERCEIVE chain reads `R_acc` from `flat_transport`'s new `TransportResult.rotation` field so consumers can call `perceive(&result.rotation.unwrap(), &v, dim)` directly. 35 new tests (8 PERCEIVE math, 3 R_acc on transport, 6 GQL parser, 5 HTTP contract, 13 real-data smoke / contract) plus the existing 30 integration test files all pass — 1082 lib tests with `kahler`, 841 no-feature, 0 regressions. Marcella reads CAPACITY/HORIZON in her retrieval router; DEPTH gates write strategy; PERCEIVE feeds the COHERENCE_SIGNAL_SPEC §3 windowed-holonomy δ_t signal as the GIGI-side analogue of the rotation accumulated by her prefix scan.

---

## What's new — 2026-05-29 — encryption paper deposit + vector-search cache

**Geometric Encryption paper deposited on Zenodo.** Davis, B. R. (2026). *Geometric Encryption: Property-Preserving Database Encryption via Gauge Invariance on Fiber Bundles.* Zenodo. [10.5281/zenodo.20438796](https://doi.org/10.5281/zenodo.20438796). The v1 PDF (28 pp, 731 KB) covers Theorem 3.3 (ρ-equivariant ciphertext-computability over general answer spaces `Y_f`), the five-mode taxonomy (Affine / Opaque / Indexed / Probabilistic / Isometric) with explicit per-mode leakage profiles graded under the Chase-Kamara structured-encryption taxonomy, the v0.3 cryptographic suite (Curvature-MAC, Aff(ℝ) capability delegation, Holonomy ledger, Čech threshold sharing, continuous RG-flow ratchet, BLS12-381 pairing PRE with formal BDH reduction, ML-KEM-768 trusted-delegatee, lattice K-of-N threshold delegation), and the v0.4 follow-ups (public deterministic verification of π_inv, credential-gated invariant queries, geodesic-ball membership index, K-preserving group characterization). Appendix A walks twelve worked Alice/Bob examples end to end. The marketing page at [davisgeometric.com/gigi/gigi-encrypt](https://davisgeometric.com/gigi/gigi-encrypt) carries an interactive in-browser demo: pick a dataset, set a secret gauge `(a, b)`, run `SUM`/`AVG`/`MIN`/`MAX`/`VAR`/`STDDEV`/`COUNT` on ciphertext, and watch the closed-form `ρ⁻¹` recover the plaintext aggregate.

**New `vector_cache` module** ([`src/vector_cache.rs`](src/vector_cache.rs)). General-purpose primitive backing the vector-search brain endpoints (`/brain/intent_gate`, `/brain/confidence`, `/brain/confidence_with_explain`). Cached `(N, D)` materialized matrices with mutation-counter invalidation and per-key single-flight compute on miss — same architecture as `BundleFlowCache`. `MaterializedMatrix` holds contiguous row-major `Vec<f64>` plus precomputed per-row squared L2 norms; distance queries use the cosine identity `‖q − r‖² = ‖q‖² + ‖r‖² − 2⟨q, r⟩` in one autovectorizable inner loop. `CachedMatrix` carries a lazy per-bandwidth `max_density` cache so `confidence_normalized`'s `O(N²·D)` denominator is computed once per (matrix, bandwidth) and reused. Public helpers: `kde_raw_from_matrix`, `max_density_cached`, `kde_normalized_cached`, `MaterializedMatrix::nearest`, `MaterializedMatrix::d_sq_to_all`. New operator-facing env var `GIGI_VECTOR_CACHE_SIZE` (default 64) for capacity tuning. 21 new unit tests gating matrix math against a naive reference + the cache lifecycle (miss → hit → invalidate, eviction at capacity, field-order disambiguation, per-key compute-lock isolation, per-bandwidth cache separation).

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
| `delegation`, `pairing_delegation`, `mlkem_delegation`, `lattice_delegation` | v0.3 Sprint J family — Aff(ℝ) capability composition (J.1), BLS12-381 pairing PRE (J.2), ML-KEM-768 trusted delegation (J.3), Shamir K-of-N × ML-KEM threshold lattice delegation (J.4) |
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
| `vector_cache` | Cached `(N, D)` materialized matrices for vector-search brain endpoints (`intent_gate`, `confidence`, `confidence_with_explain`). Architecture mirrors `BundleFlowCache`: `RwLock<HashMap>` hot read, per-key `Arc<Mutex<()>>` single-flight on miss, `mutation_counter` invalidation, capacity bound with random eviction. `MaterializedMatrix` holds contiguous row-major data + precomputed per-row `‖·‖²`; distance queries use the cosine identity in one autovectorizable loop. Public helpers: `kde_raw_from_matrix`, `max_density_cached` (lazy per-bandwidth), `kde_normalized_cached`, `nearest`. Env var `GIGI_VECTOR_CACHE_SIZE` (default 64). |
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
- `GIGI_GEOMETRIC_ENCRYPTION_SPEC.md` + `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` + `GIGI_ENCRYPT_v0.3_SPRINT_SPEC.md` + [`theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md`](theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md) — gauge encryption v0.2 → v0.3 (full delegation family + aggregate inversion) → v0.4 (invariant verification + credentials + K-preserving characterization + geodesic-ball membership)
- [`theory/encryption/paper_geometric_encryption_v0.1.tex`](theory/encryption/paper_geometric_encryption_v0.1.tex) — the load-bearing encryption paper (Aff(ℝ) trusted-delegatee model + pairing-PRE BDH-hard delegation + threshold lattice delegation; honest carveouts for Probabilistic MIN/MAX/RANGE bias and threshold-vs-PRE trust-model dependence)
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
-- "How deep does coherent context extend?"        — HORIZON = τ/(K·ℓ_c)
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
| `GET  /v1/bundles/{name}/horizon` | `τ / (K · ℓ_c)` — coherent-context length before frame rotation becomes irrecoverable | CGC Thm 8.6 |
| `GET  /v1/bundles/{name}/depth` | Erasure-energy classifier I/II/III/IV — what's the cost of writing here? | CGC Thm 8.14 |
| `POST /v1/bundles/{name}/perceive` | `(R_acc · v,  ‖R_acc − I‖_F)` — what does the substrate perceive this vector as, and how much should we trust the perception? | CGC §8 step 4 |
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
- **1124 tests passing, 0 failed** with `cargo test --lib --features kahler` (+ 64 in `cargo test --bin gigi-stream --features kahler`) — adds the twelve-layer Kähler stack (L1–L12, all 12 brain primitives operational), the **five Cognitive Geometry verbs** (CAPACITY / HORIZON / DEPTH / PERCEIVE / LOCAL_HOLONOMY) end-to-end (math + HTTP + GQL parser + real-data smokes + cross-team contract pins), the SUDOKU meta-primitive (waves 3–6.2 + S3.5), SAMPLE_TRANSPORT (S4), the #107 polymorphic brain-endpoint fix, the rank-based Betti rewrite + MorseCache + column-indexed F₂ rank (`/brain/semantic` went from 10–30 s to sub-second on production-shape complexes), per-layer real-data smokes against the 20-record sensor dataset, and the six HTTP wire-gate tests verifying that every wave-3/4/5/6 field reaches the response and the Čech pre-flight + Pareto + expansion paths return correctly.
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
| **v0.3 delegation family** | J.1 Aff(ℝ) capability composition · J.2 BLS12-381 pairing PRE (DLP_G₂-hard, pre-quantum) · J.3 ML-KEM-768 trusted-delegatee (FIPS 203, NIST Level 3 PQ) · J.4 lattice threshold = Shamir K-of-N over F_p × per-share ML-KEM (info-theoretic on ≤K-1 subsets + PQ outer layer) |
| **v0.3 secret sharing** | Shamir over secp256k1 base field F_p (Sprint L); the primitive J.4 composes |
| **v0.4 Sprint N** | Public deterministic invariant-tuple verification; `POST /v1/bundles/{name}/verify_invariant`; bundle-id binding; `Verified` / `BundleMismatch` / `Rejected{field}` verdicts |
| **v0.4 Sprint O** | Credential-gated invariant queries (HMAC-bound today; BBS+ pinned as v0.5 unlinkability upgrade) |
| **v0.4 Sprint Q** | K-preserving subgroup characterized as the diagonal affine group `(ℝ*)ᵏ ⋉ ℝᵏ`; rotation-invariant `tr(Cov)/diam²` (corrects earlier `(max−min)²` overclaim); LWE separation as hiding-vs-gauge layers. **Roadmap only** — not a shipped PQ mode |
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
