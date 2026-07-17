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

## Status

GIGI is the substrate. As of this README, end-to-end:

- **Halcyon Parts I–IV are LIVE.** SU(2) Yang-Mills on the buckyball runs as a GQL block (`LATTICE` → `GAUGE_FIELD` → `GIBBS_SAMPLE` → `E_FIELD` → `SYMPLECTIC_FLOW`). All six Halcyon HTTP read routes resolve on the production deployment; embedded-only declarers correctly return 404 (the II.6c reframe — embedded GQL is canonical for restart-crossing consumers). Gates pinned by per-part gold gates (`halcyon_part_iv_gold` 5/0, `halcyon_part_iv_a2_matrix` 5/0 with the cross-OS row honestly ignored). See [`theory/halcyon/HALCYON_PART_I_GATES.md`](theory/halcyon/HALCYON_PART_I_GATES.md) through Part IV.
- **Brain primitives L9–L13 are LIVE.** All 12 cognitive primitives (SAMPLE / DREAM / FORECAST / RECONSTRUCT / INPAINT / PREDICT / SELF-MONITOR / ATTEND / FOCUS / EPISODIC / SEMANTIC / EXPLAIN) sit on the Friston master equation `ẋ = B⁻¹∇(−log p)` on the Kähler bundle. Production HTTP at `/v1/bundles/{name}/brain/*`. 16 of 21 catalog items shipped — 100% of the items I classified as ship-able.
- **Kähler upgrade is LIVE.** Twelve layers (L1–L12) behind the `kahler` feature flag; byte-identical no-feature build. The Marcella A/B harness pinned the closed-form 7.6pp non-associativity bound to within 0.0013.
- **Causal-states substrate is LIVE.** Davis (2026) shipped with TV / Hellinger / KL diagnostics, the update-commutator orchestrator, and the Sofic / Smooth / Borderline regime classifier behind the `causal_states` feature flag. `POST /v1/causal_states/commutator` live in production.
- **Gauge encryption v0.4 is LIVE.** Six modes (Affine / Probabilistic / Opaque / Indexed / Isometric / Identity). κ, λ₁, holonomy preserved at native speed. Public deterministic invariant verification with bundle-id binding.
- **WISH verb is LIVE.** IMAGINE + DREAM + SUDOKU composed as a single boundary-value-problem verb behind the `wish` feature flag. Marcella's load-bearing curvature ceiling (4.0 = K(CP¹ Fubini-Study)) is the default safety envelope.
- **Marcella consumer pattern is documented.** Substrate spec at [`theory/kahler_upgrade/marcella_substrate.md`](theory/kahler_upgrade/marcella_substrate.md); pure-fiber LM that reads through the brain endpoints and refuses rather than confabulates when `/brain/confidence_with_explain` says she doesn't know.
- **Bridge Trilogy + SPECTRAL_GAUGE Phase 1 + Yang-Mills topology verbs are LIVE (2026-06-28).** Three substrate items unblock Halcyon's 4D SU(3) pipeline: `LATTICE … FROM CUBIC L=N DIM=K PERIODIC` declares n-D cubic lattices (L=12 D=4 gives 20736 vertices / 82944 edges / 124416 plaquettes); `INGEST bundle FROM 'file.npz' FORMAT NPZ` reads NumPy arrays via the `npyz` crate; `GROUP SU(3)` is a first-class structure group (raw 3×3 complex storage, 144 B / link, Mezzadri 2007 Haar). On top: `SPECTRAL_GAUGE bundle ON FIBER (q0..)` returns the fiber-weighted spectral gap λ₁ of the gauge-covariant Laplacian; `CHERN_CLASS … ORDER 2` returns the discrete instanton number via the Lüscher 1982 clover construction; `PONTRYAGIN … ORDER 1` returns p₁ (= 2·c₂ for SU(N)); `PI_1 lattice` returns the fundamental-group rank via spanning-tree + face-relator construction; `OBSTRUCTION bundle` returns the principal-bundle section-existence class; `BETTI … ORDER k` extends Betti to the cell complex. The December L=12 D=4 SU(3) harvest gets a one-line `CHERN_CLASS ensemble ORDER 2;` topological-sector readout once Halcyon regenerates configs. See [`theory/halcyon/HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md`](theory/halcyon/HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md), [`theory/halcyon/SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md`](theory/halcyon/SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md), and [`theory/halcyon/YANG_MILLS_TOPOLOGY_VERBS_SHIPPED_2026-06-28.md`](theory/halcyon/YANG_MILLS_TOPOLOGY_VERBS_SHIPPED_2026-06-28.md).
- **Halcyon L=24 OBC substrate + the sixteen-feature engine batch + the hardening trio are LIVE (2026-07-01 → 2026-07-03).** 2026-07-01 — four grammar extensions unblock Halcyon's SU(2) 4D L=24 β=2.3 open-boundary sectoral workflow: `LATTICE … FROM CUBIC L=N DIM=K OBC AXIS <k>` (wrap edges + boundary-crossing plaquettes omitted along the named axis; vertex set stays L^D — L=24 D=4 gives 331,776 vertices / 1,313,280 edges / 1,949,184 plaquettes), `INGEST … FORMAT NPZ [KEY <member>] AS GAUGE_FIELD GROUP <g> ON LATTICE <l>` (one-step NPZ → gauge-field bundle: canonical per-group fiber names, `vertex_a` / `vertex_b` base fields from the lattice's own column-major site numbering, OBC wrap records omitted so the record set equals the lattice edge set, f32 upconverts exactly to f64), `CHERN_CLASS … PER <field> INTO_COLUMN <col>` (per-configuration topological sectors, written back through the new append-only `ALTER BUNDLE … ADD BASE <field> <type>`), and `SPECTRAL_GAUGE … WHERE <cond> …` (sector-stratified gaps). 2026-07-03 — the sixteen-feature batch: human parse errors (token names, near-context, did-you-mean) + trailing-token rejection (a parsed statement with leftover input refuses instead of silently discarding it); `INTEGRATE … WITH JACKKNIFE ALONG <field> [SKIP FIRST n]` evidence-grade error bars with a thermalization cut; no-op statements return an explicit notice instead of a bare ok; `SHOW FIELDS ON b` returns real rows; poison-proof locks (a panicked handler no longer wedges the server); `INGEST FORMAT CSV` / `FORMAT JSONL`; `EMIT CSV TO 'f.csv'`; `EXPLAIN SECTION … AT …` per-field κ decomposition; `TIMESTAMP` field type + ISO-8601 literals + `NOW`; the `gigi` CLI (`gigi doctor`, rustyline REPL, `-f script.gql`) and the gigi-stream first-contact banner — plus the same-day review fixes (patterns-feature compile break, multibyte-safe error truncation and ISO-timestamp parsing, `Statement::Emit` dispatched over HTTP, TIMESTAMP coercion on the update path). 2026-07-03, second deploy — the hardening trio: `src/pathguard.rs` shared containment (component screen rejects absolute / drive-prefixed / UNC / `..` shapes before joining; canonical-to-canonical compare defeats symlink and junction tunnels), the fail-closed `GIGI_INGEST_DIR` gate on every file-reading INGEST format (unset ⇒ server-side file reads are disabled; prod pins `/data/ingest`), `GIGI_EMIT_DIR` routed through the same guard, and `record_kappa` stamped constant on every EXPLAIN row (mean of the per-field κ column == the record's total κ from `compute_record_k` — the response certifies its own invariant). Known residuals, named: `DEFINE PATTERN … OR` does not parse (pre-existing, queued); `EMIT` wrapping a bundle-less statement (e.g. `SHOW BUNDLES … EMIT CSV`) still returns ok without emitting (queued). See [`theory/halcyon/HALCYON_L24_OBC_WORKFLOW_UNBLOCKED_2026-06-29.md`](theory/halcyon/HALCYON_L24_OBC_WORKFLOW_UNBLOCKED_2026-06-29.md) and [`theory/halcyon/DEPLOY_2026-07-03_SIXTEEN_FEATS.md`](theory/halcyon/DEPLOY_2026-07-03_SIXTEEN_FEATS.md).
- **The durability encoder fix + the RIEMANN / Poincaré / P≠NP observable line + the Marcella EXPLAIN and dials family are LIVE (2026-07-16/17).** The DHOOM snapshot encoder's computed-field detection was `O(F³·N)` — days of compute on wide numeric bundles (384 scalar fibers), wedging the boot snapshot; it is now cached and capped at 64 candidate fields (`O(F·N + F³)`), so high-field bundles snapshot in seconds, with `.dhoom` + WAL formats byte-unchanged and old snapshots still opening. That is a durability fix for the boot-snapshot wedge only — heap-only bundles stay ephemeral until a whole-engine `/v1/admin/snapshot`. Three Millennium verbs restage geometric signatures Bee documents elsewhere as *gigi-native observables* — evidence inside the Davis framework, never proofs of the Clay problems: `SPECTRAL_GAUGE … MODE MAGNETIC [FULL [LIMIT k] | BULK k]` assembles a complex Hermitian magnetic Laplacian whose level-spacing reads in the GUE symmetry class (measured mean r̃ ≈ 0.605) against the real cos-weight Laplacian's GOE (≈ 0.527); `HOLONOMY … AROUND CYCLE` reads the lens-space π₁ = ℤ/p order off an SU(2) loop; `SPECTRAL … MODE MATRIX` exposes the raw signed-symmetric spectrum's negative fraction. Marcella-facing: `EXPLAIN SECTION … VECTOR / IN` (typed 404 on a missing key, additive `kappa_v` rows, mmap on-demand Welford stats), scoped `horizon` / `capacity` reads, and `POST …/windowed_coherence`. The dense eigensolver ceiling is opt-in via `GIGI_DENSE_CEIL` (default 4096, up to 8192); the sparse interior arm (Phase 2.1) is not shipped. See [`theory/halcyon/SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md`](theory/halcyon/SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md), [`HOLONOMY_CYCLE_SHIPPED_2026-07-17.md`](theory/halcyon/HOLONOMY_CYCLE_SHIPPED_2026-07-17.md), and [`MODE_MATRIX_SHIPPED_2026-07-17.md`](theory/halcyon/MODE_MATRIX_SHIPPED_2026-07-17.md).

Test counts, dated to their last full run:

- `cargo test --no-default-features --lib` → **911 passed / 0 failed** (2026-07-03 pre-deploy gate). The byte-identical no-feature build.
- `cargo test --features halcyon --lib --test-threads=1` → **965 passed / 0 failed** (2026-06-28). Halcyon Parts I–IV on the substrate.
- `cargo test --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --lib --test-threads=1` → **1488 passed / 0 failed** (2026-06-28). The full production feature surface.

Detailed gate-by-gate ledgers live under [`theory/halcyon/`](theory/halcyon/), [`theory/kahler_upgrade/`](theory/kahler_upgrade/), [`theory/causal_states/`](theory/causal_states/), and [`theory/imagine/`](theory/imagine/).

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

## What's in this repo

`src/` is a single Rust crate; the module-by-module reference lives in
rustdoc and [`CHANGELOG.md`](CHANGELOG.md). Top-level shape:

- **Engine** — fiber-bundle store, GQL parser + executor, mmap persistence,
  WAL, Kähler-feature modules (gated on `--features kahler`; absent paths
  are bit-identical to the pre-upgrade engine), Halcyon lattice / gauge /
  observable modules (gated on `--features halcyon`), causal-states
  substrate (gated on `--features causal_states`), the six-mode gauge
  encryption suite, and the DHOOM wire protocol.
- **Binaries** (`src/main.rs` + `src/bin/` + `examples/`) — `gigi` (the
  interactive CLI, and the `default-run` target so plain `cargo run` lands
  here: a rustyline REPL with arrow-key editing and per-database history,
  `-e "QUERY"` one-shot execution, `-f script.gql` for files of
  ;-separated statements with `--keep-going` to continue past errors, and
  `gigi doctor [--dir p]` — a health report that opens the engine, replays
  the WAL, and inventories bundles and geometry; exit 0 healthy, 1
  warnings, 2 failures), `gigi-server` (REST + WebSocket on `:3142`), `gigi-stream`
  (the binary the production deployment runs; prints a first-contact
  banner — listening URL + three curl lines to try — when the engine goes
  ready, muted by `GIGI_QUIET=1`), `gigi-edge` (local-first),
  `gigi-convert` (CLI ingestion), `gigi-stress` (load harness). Examples
  include `kahler_tour`, `predictive_coding_demo`, `attention_memory_demo`,
  and the Halcyon worked examples.
- **SDKs** — Python (`pip install gigi-client`, pandas-aware), JavaScript /
  TypeScript (`@gigi-db/client`, browser + Node), and `gigi-notebook` (a
  Jupyter kernel with GQL as the default cell language and a `%%commutator`
  cell magic for the causal-states endpoint).
- **UIs** — operator dashboard (`dashboard/`, React/Vite) and an in-browser
  GQL REPL (`playground/`) backed by a live `gigi-server`.
- **Theory & specs** (`theory/`) — the math (`*.tex`) and the build-ready
  specs alongside the code. Spec index at
  [`theory/SPECS_INDEX.md`](theory/SPECS_INDEX.md); per-area catalogs under
  [`theory/halcyon/`](theory/halcyon/), [`theory/kahler_upgrade/`](theory/kahler_upgrade/),
  [`theory/causal_states/`](theory/causal_states/), [`theory/imagine/`](theory/imagine/),
  [`theory/post_kahler_directions/`](theory/post_kahler_directions/),
  [`theory/brain_primitives/`](theory/brain_primitives/),
  [`theory/encryption/`](theory/encryption/), and [`theory/ggog/`](theory/ggog/).
- **End-to-end tests** (`e2e/`) — Playwright + Node against a running server;
  worked-example probes across 24 distinct SUDOKU / SAMPLE_TRANSPORT domains.

---

## Quick start

Two paths from a fresh checkout. Both target ten minutes wall-clock on a laptop.

### Path A — Your first 10 minutes with GIGI (the database)

A bundle, an insert, a point query, a curvature read.

```bash
# 1. Start gigi-stream on localhost
cargo run --release --bin gigi-stream
# → http://localhost:3142  (the production deployment runs the same binary)
```

```bash
# 2. Create a bundle (HTTP, GQL passthrough)
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  -d '{"query": "CREATE BUNDLE sensors FIBER (sensor_id CATEGORICAL, temp NUMERIC, humidity NUMERIC) KEYS (sensor_id);"}'

# 3. Insert two records
curl -X POST http://localhost:3142/v1/bundles/sensors/insert \
  -H "Content-Type: application/json" \
  -d '{"records":[
        {"sensor_id":"S-001","temp":22.5,"humidity":60.1},
        {"sensor_id":"S-002","temp":19.3,"humidity":71.4}]}'

# 4. Point query — O(1) via the GIGI hash. Response carries κ and confidence.
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  -d '{"query":"SECTION sensors AT (sensor_id='\''S-001'\'');"}'

# 5. Read the bundle's scalar curvature
curl http://localhost:3142/v1/bundles/sensors/curvature
```

Every response from step 4 onward carries the geometric quantities (κ, confidence = 1/(1+κ), the `dhoom` event envelope with KL / JS divergences against the running base). That isn't a logging layer — the substrate computed it on the way through.

### Path B — Your first 10 minutes with Halcyon (lattice gauge theory)

The first ~80 lines of Halcyon's `inertia_damping/run_validation_report.py` (build_graph → heatbath_thermalize → measure → leapfrog) collapse into a five-statement GQL block:

```sql
LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";

GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;

GIBBS_SAMPLE U
  BETA 2.5
  N_SWEEPS 200
  MEASURE_EVERY 1
  MEASURE (MEAN(PLAQUETTE), Q_SURROGATE)
  SEED 20260616;

E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;

SYMPLECTIC_FLOW U FROM (U=U, E=E)
  BETA 2.5 DT 0.02 N_STEPS 1000
  PROJECT_GAUSS TRUE
  MEASURE_EVERY 20
  MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, Q_SURROGATE)
  SEED 20260617;
```

Two ways to run it:

```bash
# Embedded — the canonical declarer/mutator surface per II.6c
cargo run --release --features halcyon --example halcyon_buckyball
```

```bash
# HTTP — through the universal /v1/gql reach-through. The ~46-min production
# wall is the soft-edge; nobody runs a 1000-step symplectic flow over JSON
# and waits for it. Use the embedded path for production sweeps.
curl -X POST http://localhost:3142/v1/gql \
  -H "Content-Type: application/json" \
  --data @theory/halcyon/examples/buckyball_five_statement.json
```

Reproduces Halcyon's published `<P>_meas = 0.501598 ± 0.001463` at β = 2.5 inside the Flyvbjerg–Petersen SEM band, and the 1000-step symplectic flow satisfies `max|ΔH/H₀| < 1e-3` and `gauss_residual_max < 1e-9`. Full verb-by-verb spec under §6.4 below.

Both paths work on a fresh `git clone` — no external services, no API keys, no out-of-tree fixtures.

---

## Build, test, run

```bash
# Build everything (engine + production bins + benches + examples)
cargo build --release
cargo build --release --features kahler   # adds the kahler_tour example bin
cargo build --release --features halcyon  # adds the Halcyon worked examples

# Run benches
cargo run --release --bin bench_o1
cargo run --release --bin bench_ingest
cargo run --release --bin bench_tpch

# E2E against a running gigi-server
cd e2e && npm install && npm test
```

As of this README the engine ships with (counts dated to their last full run):

- **`cargo test --no-default-features --lib`** → **911 passed / 0 failed** (2026-07-03 pre-deploy gate). The byte-identical no-feature build. The optionality contract: every feature flag is opt-in; the no-feature build is bit-identical to pre-upgrade GIGI on the paths it shares.
- **`cargo test --features halcyon --lib --test-threads=1`** → **965 passed / 0 failed** (2026-06-28). Halcyon Parts I–IV on the substrate (`LATTICE` / `GAUGE_FIELD` / `GIBBS_SAMPLE` / `E_FIELD` / `SYMPLECTIC_FLOW` / `PROJECT_GAUSS` + the observable battery + the A2 bit-identity rows). Single-threaded because the gauge registry uses `Arc<Mutex<…>>` mutable accessors per D4.
- **`cargo test --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --lib --test-threads=1`** → **1488 passed / 0 failed** (2026-06-28). The full production feature surface. Adds the twelve Kähler layers, the IMAGINE / WALK extrapolation verbs with Marcella's 4.0 curvature ceiling, the sharded substrate, atomic sheaf commits, GGOG patterns, causal-states substrate, and the WISH BVP verb on top of Halcyon. The 2026-07-01→03 waves ride the per-suite gates in [`theory/halcyon/DEPLOY_2026-07-03_SIXTEEN_FEATS.md`](theory/halcyon/DEPLOY_2026-07-03_SIXTEEN_FEATS.md) (INGEST family 39/0, spectral + topology 67/0, pathguard escape matrix 14/0 on Windows, lattice OBC 17/0, plus the no-feature groups).
- **`cargo test --features halcyon --test halcyon_part_iv_gold --release`** → **5 passed / 0 failed.** The Part IV gold gate (acceptance arm + regression arm under release profile — per the III.8c profile-pinning).
- **`cargo test --features halcyon --test halcyon_part_iv_a2_matrix --release`** → **5 passed / 1 ignored.** The A2 bit-identity matrix; the cross-OS row (Row 3) is honestly `#[ignore]`d with the 2 ULP envelope documented in [`HALCYON_PART_IV_GATES.md`](theory/halcyon/HALCYON_PART_IV_GATES.md).

Plus the Python TDD math gates that ride alongside the Rust suites:

- IMAGINE / WALK extrapolation verbs — see [`theory/imagine/validation/`](theory/imagine/validation/) (T11 geodesic integrator, T12 halo partition invariance, T13 double-cover monodromy).
- Sharded substrate — see [`theory/poincare_to_sharding/validation/`](theory/poincare_to_sharding/validation/) (10 TDD gates, ~15s wall clock, three were red-then-green during development).
- Kähler / post-Kähler / brain-primitives validation — 15/15 + 30/30 + 26/26, see the catalogs under [`theory/`](theory/).

### Environment gates (fail-closed)

Two verb families touch the server's filesystem. Each is disabled until its root directory is set, and every path is confined to that root by the shared guard in [`src/pathguard.rs`](src/pathguard.rs): a component-level screen rejects absolute, drive-prefixed, UNC, and `..` forms before any filesystem access, then a canonical-to-canonical containment check refuses symlink and junction tunnels out of the root.

- **`GIGI_INGEST_DIR`** — gates every file-reading `INGEST` format (NPZ / CSV / JSONL). Unset ⇒ `INGEST` from server-side files errors engine-wide — the same posture as Postgres `pg_read_server_files` / MySQL `secure_file_priv`. Set ⇒ source paths in `INGEST … FROM '<p>'` resolve relative to the root. Production sets `GIGI_INGEST_DIR = "/data/ingest"` in [`fly.toml`](fly.toml): the December harvest pipeline keeps its NPZ capability, bounded to the volume directory it already writes into.
- **`GIGI_EMIT_DIR`** — gates `… EMIT CSV TO 'file.csv'`. Unset ⇒ EMIT errors and names the knob; exported files land only inside the root. Production leaves it unset — HTTP consumers request the rows and save client-side.

`GIGI_QUIET=1` mutes the first-contact banner `gigi-stream` prints when the engine goes ready (listening URL + three curl lines to try).

Two operational knobs live outside the fail-closed filesystem gates:

- **`GIGI_DENSE_CEIL`** — raises the dense eigensolver ceiling for `SPECTRAL_GAUGE … FULL` / `BULK` (the `MODE MAGNETIC` interior spectrum). Default 4096 vertices; an opt-in value is clamped to the safe band `[4096, 8192]` — raise-only, so it can never lower the floor or exceed 8192, because a V ≈ 8000 complex-Hermitian dense solve is ~2–3 GB peak RSS and will OOM a laptop. A `FULL` / `BULK` request above the ceiling returns a typed `SparseUnavailable` naming the memory cost and this knob (the sparse interior arm is Phase 2.1, not shipped). A machine-safety knob, deliberately in the environment rather than the query.
- **`GIGI_SKIP_BOOT_SNAPSHOT`** — when set, boot finishes WAL replay and stays heap-only, skipping the post-replay DHOOM snapshot / mmap upgrade. The escape hatch for the boot-snapshot path; the 2026-07-16 durability fix (wide-numeric-bundle computed-field detection, once `O(F³·N)`, now cached and capped) made it rarely necessary — high-field bundles that used to wedge the encoder now snapshot in seconds.

---

## Usage by capability

### 6.1 GQL — verbs at a glance

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
BETTI sensors ORDER 1;  -- explicit β_k via the cell complex (Phase 1: k ∈ {0, 1})

-- Yang-Mills topology verbs (2026-06-28). Discrete Chern-Weil integration,
-- π_1 fundamental group, principal-bundle section-existence obstruction.
-- All five verbs answer end-to-end through /v1/gql against a declared lattice
-- + gauge field (verified on production 2026-06-28 late):
--   LATTICE smoke FROM CUBIC L=4 DIM=2 PERIODIC;
--   GAUGE_FIELD U_smoke ON LATTICE smoke GROUP SU(3) INIT IDENTITY;
--   CHERN_CLASS U_smoke ORDER 2;        → {"value": 0.0}
--   PONTRYAGIN U_smoke ORDER 1;         → {"value": -0.0}
--   BETTI smoke ORDER 2;                → {"value": 1.0}   (β_2(T²) = 1)
--   PI_1 smoke;                          → {"value": 2.0}   (π_1(T²) = ℤ², rank 2)
--   OBSTRUCTION U_smoke;                 → {"value": 0.0}
CHERN_CLASS gauge_bundle ORDER 2;       -- instanton number Q ∈ ℤ (Lüscher clover)
PONTRYAGIN gauge_bundle ORDER 1;        -- p_1 = 2·c_2 for SU(N)
PI_1 my_lattice;                         -- rank of π_1 (spanning tree + face relators)
OBSTRUCTION gauge_bundle;                -- principal-bundle section sector
SPECTRAL_GAUGE gauge_bundle ON FIBER (q0, q1, q2, q3);  -- fiber-weighted Laplacian gap

-- Bridge Trilogy verbs (2026-06-28). 4D cubic + NPZ ingest + SU(3) ingest.
-- The LATTICE → GAUGE_FIELD → CHERN_CLASS bridge is live end-to-end on
-- production (see smoke chain above): LATTICE declares the base, GAUGE_FIELD
-- declares the bundle, CHERN_CLASS reads the instanton sector.
LATTICE my_4d FROM CUBIC L=12 DIM=4 PERIODIC;
INGEST configs_bundle FROM 'harvest_L12_beta6.0_run1.npz' FORMAT NPZ;
GAUGE_FIELD U_4d ON LATTICE my_4d GROUP SU(3) INIT HAAR SEED 42;

-- L=24 OBC layer (2026-07-01). Open boundary along one axis (wrap edges +
-- boundary-crossing plaquettes omitted), one-step NPZ → gauge-field bundle,
-- per-config sector write-back, append-only schema evolution,
-- sector-stratified spectral gaps:
LATTICE l24 FROM CUBIC L=24 DIM=4 OBC AXIS 0;
INGEST su2_configs FROM 'raw_U_configs.npz' FORMAT NPZ KEY U
  AS GAUGE_FIELD GROUP SU(2) ON LATTICE l24;
ALTER BUNDLE su2_configs ADD BASE q_rounded INT;
CHERN_CLASS su2_configs ORDER 2 ON LATTICE l24 GROUP SU(2)
  PER config_id INTO_COLUMN q_rounded;
SPECTRAL_GAUGE su2_configs WHERE q_rounded = 0
  ON FIBER (q0, q1, q2, q3) GROUP SU(2);

-- Engine batch (2026-07-03). Evidence-grade error bars, schema
-- introspection, CSV in/out, per-field curvature decomposition, time as
-- a first-class type:
INTEGRATE runs MEASURE avg(plaquette) WITH JACKKNIFE ALONG sweep SKIP FIRST 500;
SHOW FIELDS ON sensors;                  -- one real row per field (field/kind/type/indexed)
EXPLAIN SECTION sensors AT sensor_id='S-001';    -- per-field κ + constant record_kappa
INGEST sensors FROM 'readings.csv' FORMAT CSV;   -- header row, inferred types; JSONL too (KEY <col> required)
COVER sensors ALL EMIT CSV TO 'sensors.csv';     -- fail-closed on GIGI_EMIT_DIR
CREATE BUNDLE events (id TEXT BASE, at TIMESTAMP FIBER);
COVER events WHERE at <= NOW;            -- ISO-8601 literals and NOW are timestamps

-- Millennium observables (2026-07-16/17). Each verb restages a documented
-- geometric signature as a gigi-native readout — evidence in the Davis
-- framework, NOT a proof of the Clay problem it echoes.
--   RIEMANN: a U(1) Hermitian magnetic Laplacian whose level spacing reads the
--   GUE class (measured r̃≈0.605) vs the real cos-weight GOE (≈0.527).
SPECTRAL_GAUGE erdos ON FIBER (theta) GROUP U(1) MODE MAGNETIC FULL LIMIT 8;
SPECTRAL_GAUGE erdos ON FIBER (theta) GROUP U(1) MODE MAGNETIC BULK 32 AROUND 0.0;
--   P vs NP: the raw signed-symmetric spectrum; n_negative / instability_fraction.
SPECTRAL hess ON FIBER (h) MODE MATRIX FULL;
--   Poincaré: an SU(2) loop; order_estimate reads the lens-space π_1 = Z/p class.
HOLONOMY U_lens AROUND CYCLE AXIS z AT (0, 1);
HOLONOMY U_lens AROUND CYCLE EDGES (0, 5, 9, 14);
GAUGE_FIELD theta GROUP U(1) INIT FLUX RANDOM SEED 20260716 ON LATTICE erdos;  -- seed a θ bundle

-- Marcella EXPLAIN family + OR patterns (2026-07-16).
EXPLAIN SECTION corpus AT id=1 VECTOR (v0..v383);   -- per-field κ + additive kappa_v
EXPLAIN SECTION corpus AT id IN (1, 2, 3);          -- grouped rows, one read-lock
DEFINE PATTERN spike AS amount=1 OR flag=2;         -- base-AND-(OR groups), COVER parity

-- Cognitive Geometry verbs (Branch VII)
CAPACITY corpus;    -- τ/K
HORIZON corpus;     -- τ/(K·ℝ“_c)
DEPTH corpus;       -- erasure-energy classifier I/II/III/IV
PERCEIVE corpus ROTATION (r00, r01, r10, r11) VECTOR (v0, v1) DIM 2;

-- Encrypted-at-rest fiber, gauge-preserving
CREATE BUNDLE finance FIBER (
  amount NUMERIC ENCRYPTED,
  account TEXT ENCRYPTED INDEXED
);
```

Two parser-wide behaviors ride under every verb (2026-07-03): parse errors
name the offending token with near-context and a did-you-mean for a
misspelled first verb, and input left over after a complete statement is
refused with an explicit trailing-token error instead of being silently
discarded. Statements that perform no work (unimplemented SHOW variants,
COMPACT, …) return an explicit notice instead of a bare ok.

Complete grammar, status table, complexity per verb, and EMIT / wire format
options live in [`GQL_REFERENCE.md`](GQL_REFERENCE.md) and
[`GQL_SPECIFICATION.md`](GQL_SPECIFICATION.md).

### 6.2 HTTP — the route catalog

The production deployment exposes a consumer-facing HTTP surface:
`/v1/gql` (universal GQL passthrough), `/v1/bundles/{name}/*` for
geometric reads (curvature, spectral gap, holonomy), `/v1/bundles/{name}/brain/*`
for the twelve brain primitives, `/v1/causal_states/commutator` for the
Davis (2026) update commutator, `/v1/bundles/{name}/verify_invariant`
for the public deterministic invariant check, and the Halcyon read
routes (`/v1/lattice/*`, `/v1/gauge_field/*`, `/v1/e_field/{name}`,
`/v1/symplectic_flow/diagnostics/:run_id`). For the full route catalog
hit `GET /v1/openapi.json` against any running `gigi-stream` instance —
the OpenAPI document is the source of truth for shapes, parameters, and
auth requirements.

### 6.3 Cognitive Geometry — builder-facing routing

Five verbs translating static geometric scalars into routing decisions
the way a builder would phrase them: *can the substrate hold this
interpretation?* (CAPACITY = τ/K, CGC Thm 8.1), *how deep does coherent
context extend?* (HORIZON = τ/(K·ℝ“_c), Thm 8.6), *what's the erasure
energy of writing here?* (DEPTH classifier I/II/III/IV, Thm 8.14),
*what does the substrate perceive this vector as, and how much should I
trust the perception?* (PERCEIVE, §8 step 4), and *how trustworthy is
the recent coherence regime?* (LOCAL_HOLONOMY, COHERENCE_SIGNAL_SPEC §3).
HTTP shapes under `/v1/bundles/{name}/{capacity,horizon,depth,perceive,local_holonomy}`.

### 6.4 Halcyon — lattice gauge theory on the substrate

Halcyon is SU(2) Yang-Mills on the buckyball lattice, running on GIGI as its substrate. The Parts I–IV sprint locked the verbs that take a ~600-line Python kernel and collapse it into a five-statement GQL block — the same block from the Quick Start, repeated here as the section's headline:

```sql
LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY "S2";

GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY;

GIBBS_SAMPLE U
  BETA 2.5 N_SWEEPS 200
  MEASURE_EVERY 1
  MEASURE (MEAN(PLAQUETTE), Q_SURROGATE)
  SEED 20260616;

E_FIELD E ON GAUGE_FIELD U INIT MAXWELL_BOLTZMANN BETA 2.5 SEED 20260617;

SYMPLECTIC_FLOW U FROM (U=U, E=E)
  BETA 2.5 DT 0.02 N_STEPS 1000
  PROJECT_GAUSS TRUE
  MEASURE_EVERY 20
  MEASURE (H_TOTAL, MEAN(PLAQUETTE), GAUSS_RESIDUAL_MAX, Q_SURROGATE)
  SEED 20260617;
```

**Verb by verb.** One-line description + spec link. Per-verb receipts (red-then-green commits, A2 row pins, fixture replays) live in the gates docs.

- `LATTICE` — declares a graph topology by `(VERTICES, EDGES, FACES, TOPOLOGY)` with the truncated-icosahedron shorthand. Storage: incidence table + Euler-characteristic precompute + face-cycle orientation table. Generalized `HOLONOMY` walks edge-list loops on a declared lattice. → [`HALCYON_PART_I_GATES.md`](theory/halcyon/HALCYON_PART_I_GATES.md) Part I.
- `GAUGE_FIELD` — per-edge group-element store with group-erased storage (`SU(2)` only at launch; `U(1)` / `Z_N` are typed-empty arms). `INIT IDENTITY` and `INIT HAAR_RANDOM SEED n` both ship; declare → introspect → re-declare → bit-identical. → [`HALCYON_PART_I_GATES.md`](theory/halcyon/HALCYON_PART_I_GATES.md) Part II.
- `PLAQUETTE` / `Q_SURROGATE` — observable battery. `PLAQUETTE` desugars to face holonomy via the Part I walker; three call shapes (`PerFace` / `Mean` / `Sum`). `Q_SURROGATE` is `(1/2π) · Σ_f arccos(clamp(q0_f, −1, 1))`. → [`HALCYON_PART_I_GATES.md`](theory/halcyon/HALCYON_PART_I_GATES.md) Part III and the Q1/A1 math in [`HALCYON_TO_GIGI_REPLY_2026-06-17.md`](HALCYON_TO_GIGI_REPLY_2026-06-17.md).
- `GIBBS_SAMPLE` — Kennedy-Pendleton heatbath sweep with the staple-sum kernel and a sqrt-rejection-on-`x0` Haar fallback for `||staple|| · β < ε`. Sequential edge order is bit-identity load-bearing. Reproduces Halcyon's `<P>_canonical` byte-for-byte at fixed seed within GIGI. → [`HALCYON_PART_III_GATES.md`](theory/halcyon/HALCYON_PART_III_GATES.md) and [`HALCYON_PART_III_IMPLEMENTATION_LOG.md`](theory/halcyon/HALCYON_PART_III_IMPLEMENTATION_LOG.md).
- `E_FIELD` — Lie-algebra sibling buffer parked next to a `GAUGE_FIELD`. `(n_edges, 4)` quaternion-packed with `q0 = 0` enforced at every entry point. `INIT MAXWELL_BOLTZMANN BETA β SEED s` draws per-edge Gaussians with sigma `= √(1/(β · 1.5))`; the fourth normal is consumed-then-discarded so β-shifted runs share an identical RNG advance per edge (the A2 row 1 bit-identity contract). → [`HALCYON_PART_IV_GATES.md`](theory/halcyon/HALCYON_PART_IV_GATES.md) §IV.1.
- `SYMPLECTIC_FLOW` — second-order Kick-Drift-Kick leapfrog on `(U, E)`. Drift uses the closed-form SU(2) Rodrigues exponential with a 4th-order Taylor fallback at `θ < 1e-8`. Kick uses the Wilson force `F[e] = (−β/8) · staple_sum`. `PROJECT_GAUSS TRUE` (default) projects the covariant Gauss residual back to the constraint surface every step. Kogut-Susskind Hamiltonian with `g² = 4/β` is load-bearing — the naive convention gives 65% drift over 200 steps; Kogut-Susskind gives `< 1e-4`. → [`HALCYON_PART_IV_GATES.md`](theory/halcyon/HALCYON_PART_IV_GATES.md) §IV.4–IV.6.
- `PROJECT_GAUSS` — knob struct exposing `{tikhonov, cg_tol, cg_max_iter}` over the unpreconditioned Hestenes-Stiefel CG projector. Production defaults are `{1e-14, 1e-10, 200}` (matches Halcyon Python). The spec-default `1e-12` lives behind explicit struct sugar. Buckyball `cond(L_cov) ~ 16` needs no preconditioner; Jacobi is P1. → A1 / Q3 in [`HALCYON_TO_GIGI_REPLY_2026-06-17.md`](HALCYON_TO_GIGI_REPLY_2026-06-17.md).

**Embedded vs HTTP — the II.6c split, named here.**

`LATTICE`, `GAUGE_FIELD`, `PLAQUETTE`, `Q_SURROGATE`, `SHOW E_FIELD`, `SELECT H_TOTAL OF (U,E)`, `SELECT GAUSS_RESIDUAL_MAX OF (U,E)`, and the `GET /v1/symplectic_flow/diagnostics/:run_id` LRU lookup all have HTTP routes — they're consumer-facing reads and ephemeral declares. They resolve on the production deployment with auth; embedded-only declarers return 404 on the same surface.

`GIBBS_SAMPLE`, `SYMPLECTIC_FLOW`, and `E_FIELD` declare are **embedded-only by design**. The reason isn't aesthetic — `GIBBS_SAMPLE` at β=2.5 on a 90-edge buckyball runs ~46 minutes wall-clock on production hardware doing O(10⁶) per-edge updates. Nobody runs that over JSON and waits for it. The route table absence is the enforcement; the soft-edge through `/v1/gql` is by design (any consumer routing a 46-minute mutation verb through HTTP self-deselects). Embedded GQL via PyO3 / CFFI is the canonical declarer / mutator surface for any consumer crossing a restart boundary.

**Bit-identity contract.** Six rows from [`HALCYON_TO_GIGI_REPLY_2026-06-17.md`](HALCYON_TO_GIGI_REPLY_2026-06-17.md) § A2:

| Row | Vary | Contract |
|---|---|---|
| 1 | Same process, fixed seed/β/dt/n | STRICT byte-identical on `(U_i, E_i, H_total, gauss_residual_max)` — hard gate under `--release` |
| 2 | Cross-process, same OS, same BLAS | STRICT byte-identical via fixture replay — hard gate under `--release` |
| 3 | Cross-OS | 2 ULP tolerance in trig reductions — documented, `#[ignore]` in CI |
| 4 | Different β, same seed | NOT bit-identical; energy drift and Gauss residual tolerances hold independently |
| 5 | Different dt, same seed | NOT bit-identical; tolerances reapply |
| 6 | Different n_steps, same seed | Prefix-equality on the first `min(n1, n2)` steps — hard gate under `--release` |

Intra-binding strict; cross-binding (NumPy PCG64 mock vs Rust xorshift64*) impossible by design. CG iteration counts vary across β at fixed seed — that's a diagnostic, not a regression.

**Where to read more.**

Gates: [`HALCYON_PART_I_GATES.md`](theory/halcyon/HALCYON_PART_I_GATES.md), [`HALCYON_PART_III_GATES.md`](theory/halcyon/HALCYON_PART_III_GATES.md), [`HALCYON_PART_IV_GATES.md`](theory/halcyon/HALCYON_PART_IV_GATES.md), [`HALCYON_PART_V_SNAPSHOT_GATES.md`](theory/halcyon/HALCYON_PART_V_SNAPSHOT_GATES.md). Implementation logs (red-then-green commits, fixture SHAs): the `*_IMPLEMENTATION_LOG.md` siblings. Cross-team protocol: [`HALCYON_TO_GIGI_REPLY_2026-06-17.md`](HALCYON_TO_GIGI_REPLY_2026-06-17.md). Yang-Mills mass-gap chapter (forward-tracking content target): `GIGI Solves: The Clay Seven`, Vol 4.

**The Bridge Trilogy + Yang-Mills topology layer (2026-06-28).** Three substrate items unblock the 4D SU(3) pipeline, then five verbs read topological invariants off the resulting bundles.

The trilogy itself — `LATTICE … FROM CUBIC L=N DIM=K PERIODIC` (n-D cubic primitive: L=12 D=4 gives 20736 vertices / 82944 directed links / 124416 plaquettes, matching Halcyon's December harvest dimensions), `INGEST bundle FROM 'file.npz' FORMAT NPZ` (real ingest executor via the `npyz` crate; SU(3) configs of shape `(L, L, L, L, 9)` complex flatten cleanly to per-link records), and `GAUGE_FIELD … GROUP SU(3) INIT …` (raw 3×3 complex storage = 144 B per link, Mezzadri 2007 Haar for `INIT HAAR`, conjugate-transpose inverse, 3×3 matmul compose, `Re Tr(U)/3` plaquette reduction). Phase 1 scope is read-only ingest. Phase 2 (Cabibbo-Marinari heatbath, `SU3EField` tangent space, SU(3) Wilson force, symplectic flow) ships when Halcyon needs gigi to compute SU(3) configs in-house instead of ingesting them — currently they regenerate via `lattice/gauge_heatbath_gpu.py` on a GPU machine and `INGEST` the result. See [`HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md`](theory/halcyon/HALCYON_BRIDGE_TRILOGY_2026-06-28_SHIPPED.md).

The topology verbs — `SPECTRAL_GAUGE bundle ON FIBER (…) [GROUP …]` returns λ₁ of the fiber-weighted Laplacian L_A (dense `nalgebra::SymmetricEigen` Phase 1 for bundles up to ~1000 vertices; Lanczos sparse + `FULL` k-eigenvalue mode is the named Phase 2 ship). `CHERN_CLASS bundle ORDER 2` returns the discrete instanton number Q ∈ ℤ via Lüscher 1982 clover discretization — one integer per gauge configuration, classifying the topological sector for the mass-gap argument. `PONTRYAGIN bundle ORDER 1` returns p₁ (= 2·c₂ for SU(N), Lüscher sign convention pinned). `PI_1 lattice_name` returns the rank of the fundamental group via BFS spanning tree + face-relator construction (Massey Ch IV §5). `OBSTRUCTION bundle_name` returns the principal-bundle section-existence class (= rounded c₂ for SU(N) on 4D, rounded c₁ for U(1) on 2D). `BETTI bundle ORDER k` extends BETTI to cell-complex β_k via integer-SNF on boundary maps. The TDD audit trail is preserved in git history (`tests(X): RED …` commits precede `impl(X): GREEN …` for each concept). See [`SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md`](theory/halcyon/SPECTRAL_GAUGE_PHASE1_SHIPPED_2026-06-28.md) and [`YANG_MILLS_TOPOLOGY_VERBS_SHIPPED_2026-06-28.md`](theory/halcyon/YANG_MILLS_TOPOLOGY_VERBS_SHIPPED_2026-06-28.md).

**The L=24 OBC layer (2026-07-01).** Four grammar extensions unblock Halcyon's SU(2) 4D L=24 β=2.3 open-boundary sectoral workflow; they compose into one Q-sector-by-Q-sector chain (declare → ingest → classify → stratify).

- `LATTICE … FROM CUBIC L=N DIM=K OBC AXIS <k>` opens the boundary along one axis: edges wrapping axis k and plaquettes crossing its boundary are omitted; the vertex set stays L^D. L=24 D=4 OBC AXIS 0 gives V = 331,776 (unchanged), E = 1,313,280, F = 1,949,184 — down from the periodic 1,327,104 / 1,990,656. Multi-axis OBC (`PERIODIC AXES (…) OBC AXIS …`) is deferred to Phase 2; `OPEN` (fully-open) cannot combine with `OBC AXIS`.
- `INGEST <bundle> FROM '<file.npz>' FORMAT NPZ [KEY <member>] AS GAUGE_FIELD GROUP <g> ON LATTICE <l>` turns a raw NPZ archive into a gauge-field bundle in one step. Axes decode as (config_id, mu, site_*, fiber); fiber columns take the canonical per-group names (`q0..q3` SU(2), `re_00..im_22` SU(3), `theta` U(1)); every record carries `vertex_a` / `vertex_b` base fields computed with the lattice's own column-major site numbering; records that would wrap across the OBC boundary are omitted, so the record set equals the lattice edge set by construction. `KEY <member>` selects one array out of a multi-array archive. f32 sources upconvert exactly to f64 (24-bit mantissa fits inside 53); unsupported dtypes error by name. Since 2026-07-03, server-side sources resolve relative to the fail-closed `GIGI_INGEST_DIR` (unset ⇒ disabled; prod pins `/data/ingest` — see the environment gates under Build, test, run).
- `CHERN_CLASS <bundle> ORDER 2 [ON LATTICE <l>] [GROUP <g>] [PER <field>] [INTO_COLUMN <col>]` reads the instanton sector per record group (`PER config_id` = one Q per configuration) and writes the rounded sector back into a column declared by the new append-only `ALTER BUNDLE <b> ADD BASE <field> <type>` (heap bundles only in Phase 1; `INTO_COLUMN` without `PER` is rejected at parse time — nothing to write per group otherwise).
- `SPECTRAL_GAUGE <bundle> WHERE <cond> ON FIBER (…) [GROUP <g>]` runs the fiber-weighted Laplacian gap on the sector-filtered subset. WHERE reuses COVER's condition grammar, so filter semantics are identical.

Route-handler parity rode along: INGEST, ALTER BUNDLE, and the topology verbs dispatch before bundle pre-resolve, so the same statements answer over `/v1/gql`. See [`HALCYON_L24_OBC_WORKFLOW_UNBLOCKED_2026-06-29.md`](theory/halcyon/HALCYON_L24_OBC_WORKFLOW_UNBLOCKED_2026-06-29.md).

**The Millennium line — gauge-native observables (2026-07-16/17).** Three verbs compute, directly on the substrate, quantities that *read* geometric signatures Bee documents elsewhere. Take each as evidence inside the Davis framework — a gauge-native observable that restages a documented signature — **not** as a proof of the Clay problem it echoes. GIGI proves nothing about Riemann, Poincaré, or P vs NP; it exposes the signatures as things you can query.

- **RIEMANN — `SPECTRAL_GAUGE … MODE MAGNETIC` (2026-07-16/17).** With `GROUP U(1)` and `MODE MAGNETIC` the fiber phases assemble a complex Hermitian *magnetic* Laplacian (off-diagonal `−e^{iθ}` in conjugate pairs) in place of the real cos-weight Laplacian. Time-reversal breaks, and the level-spacing statistics move from the GOE symmetry class into GUE: on fixed-seed V=512 Erdős–Rényi random-flux graphs the four-seed mean spacing ratio measures r̃ ≈ 0.605 (magnetic, GUE anchor 0.5996) against r̃ ≈ 0.527 (cos-weight, GOE anchor 0.5307), anchors from Atas–Bogomolny–Giraud–Roux PRL 110 084101. `FULL [LIMIT k]` populates the full ascending real spectrum (dense, V ≤ 4096) or the k smallest; `BULK k [AROUND σ | IN [a,b]]` returns the k eigenvalues nearest the spectral center — the interior window where the RH / number-variance statistics live (plain `BULK k` auto-centers on the positional median). The dense complex-Hermitian solver serves `FULL` / `BULK` up to `GIGI_DENSE_CEIL` (default 4096, opt-in up to 8192); past the ceiling the verb returns a typed `SparseUnavailable` naming the memory cost and the knob. The sparse interior Lanczos arm (Phase 2.1, the RH L=24/L=32 regime) is **not shipped** — built and completeness-proven, but deferred to a dedicated pass. The SU(2) cos-weight path is unchanged. → [`SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md`](theory/halcyon/SPECTRAL_PHASE2_MAGNETIC_SHIPPED_2026-07-16.md), [`SPECTRAL_BULK_FEASIBILITY_2026-07-17.md`](theory/halcyon/SPECTRAL_BULK_FEASIBILITY_2026-07-17.md).
- **Poincaré — `HOLONOMY … AROUND CYCLE` (2026-07-17).** `HOLONOMY <gauge_field> AROUND CYCLE AXIS <ax> AT (<c0>, <c1>)` (or `EDGES (<e…>)`) walks a closed loop on a declared gauge field and returns `{ q0, q1, q2, q3, re_trace, order_estimate, group_used }` — SU(2) only this phase; a non-SU(2) field is a typed error. `re_trace = ½·Tr(U) = q0`, and `order_estimate` reads the order of the loop's quaternion: on a clean lens-space wrap that order is the π₁ = ℤ/p class of `L(p,q)`. Direction convention: a `+axis` walk (or an edge matching the stored direction) contributes `U`, otherwise `U†`. `order_estimate` is meaningful only on clean lens wraps. → [`HOLONOMY_CYCLE_SHIPPED_2026-07-17.md`](theory/halcyon/HOLONOMY_CYCLE_SHIPPED_2026-07-17.md).
- **P vs NP — `SPECTRAL … MODE MATRIX` (2026-07-17).** `SPECTRAL <bundle> ON FIBER (h) MODE MATRIX [DIAGONAL <field>] [FULL [LIMIT k]]` reads the raw signed symmetric matrix `M[a][b] = M[b][a] = h` (the Hessian weight itself, negatives preserved — *not* the PSD Laplacian), with the diagonal taken from self-loop records (`vertex_a == vertex_b`) or the `DIAGONAL` override, and returns `{ eigenvalues, n_records_used, mode_used, n_negative, instability_fraction }` where `instability_fraction = n_negative / V`. No `GROUP` is required. → [`MODE_MATRIX_SHIPPED_2026-07-17.md`](theory/halcyon/MODE_MATRIX_SHIPPED_2026-07-17.md).

### 6.5 Kähler tour — one shot through every shipped layer

```bash
cargo run --release --features kahler --bin kahler_tour
```

Walks L1 (J, B), L1.5 (transport), L2 (adjacency commutativity), L3
(Jacobi cardinality), L4 (Kähler curvature decomposition), L5 (Hadamard
detection), L6 (Hodge + Morse), L7 (line bundle, holonomy debt, quantum
cohomology, Toeplitz, Riemann-Roch capacity), L9 (moment map / Noether),
plus the DHOOM array-of-primitives round-trip and a summary of the four
PR-window endpoints, with concrete inputs / outputs / catalog references.
Source at [`examples/kahler_tour.rs`](examples/kahler_tour.rs).

### 6.6 Causal states — the update commutator

`POST /v1/causal_states/commutator` returns the update commutator
`Ω = (U_a ∘ U_b)(p) − (U_b ∘ U_a)(p)` on a base belief: forward + backward
arms, `tv` / `hellinger` / `kl` scalar diagnostics, regime classification
`sofic` / `smooth` / `borderline`. `kl` is a tagged enum —
`{"kind":"finite","value":v}` in the smooth regime,
`{"kind":"divergent"}` in the sofic regime. Operators: `even_u0`,
`even_u1`, or `hmm` with `{alpha, beta, symbol}`. Optional `bands` override
defaults `tv_low=0.30 / tv_high=0.95`. Reference: Davis (2026) §7,
[`theory/causal_states/causal_states_paper.tex`](theory/causal_states/causal_states_paper.tex).
Empirical scan verifies closed-form Eq 6.4 to IEEE 754 precision across
2505 grid points; orthogonality scan shows `H[X₁]` does not determine
`|Ω|` across 1773 processes.

### 6.7 GIGI Lang — prompt → GQL → fiber response

GIGI Lang is the prompt → GQL → fiber-shaped response pipeline. As of the II.6c reframe (commit `d5d3853`), embedded GQL via PyO3 / CFFI is the canonical declarer / mutator surface — which makes GIGI Lang a first-class citizen rather than a convenience wrapper. Anything a downstream consumer needs to write across a restart boundary goes through GIGI Lang's binding, not through the HTTP shell.

**Worked example.** "Summarize this conversation as a record" against a `conversations` bundle:

```python
from gigi.lang import GigiLang

lang = GigiLang(bundle="conversations")
response = lang.ask(
    "Summarize this conversation as a record, with sender, topic, and stance.",
    context=transcript,
)
```

GIGI Lang's translator (engine recommendation: Claude as v1) emits the GQL block:

```sql
INSERT INTO conversations FIELDS (
  sender CATEGORICAL,
  topic CATEGORICAL,
  stance VECTOR(384)
) VALUES (
  'Bee', 'lattice-gauge-substrate',
  EMBED('curious-but-skeptical', MODEL='bge-small-en-v1.5')
);

SECTION conversations AT (sender='Bee', topic='lattice-gauge-substrate')
  EMIT DHOOM;
```

The response carries the inserted record AND the geometric envelope every GIGI response carries — κ at the new section, confidence = 1/(1+κ), the DHOOM event with KL / JS divergence against the running base, and (if the bundle is Kähler-equipped) the holonomy debt component and quantum-cohomology capacity for the local region. The translator never invents the geometric quantities — they're substrate-computed and ride along.

The G8 "one lossy step" rule from [`GIGI_LANG_SPEC.md`](GIGI_LANG_SPEC.md) inherits the Kähler optionality contract: the LLM-translator step is the lossy step; everything downstream is reproducible from the emitted GQL. That gives a contract test surface — the same prompt + the same translator seed → byte-identical GQL → byte-identical response within a binding.

Spec: [`GIGI_LANG_SPEC.md`](GIGI_LANG_SPEC.md). SDK skeleton: [`sdk/python/gigi/lang.py`](sdk/python/gigi/lang.py). Schema introspection (public `/schema` endpoint with `@public` / `@gated` directive policy): [`GIGI_SCHEMA_INTROSPECTION_SPEC.md`](GIGI_SCHEMA_INTROSPECTION_SPEC.md).

---

## 7. Geometric encryption

Gauge encryption preserves every geometric quantity GIGI computes — six modes, native speed. The structure group of the fiber bundle is itself the cipher.

| Quantity | Plaintext | Encrypted | Match? |
|---|---|---|---|
| Scalar curvature K | ✓ | ✓ | exact |
| Confidence 1/(1+K) | ✓ | ✓ | exact |
| Capacity C = τ/K | ✓ | ✓ | exact |
| Spectral gap λ₁ | ✓ | ✓ | exact (graph-topology invariant) |
| Anomaly scores | ✓ | ✓ | exact |
| Holonomy δφ | ✓ | ✓ | exact (gauge-invariant — including HOLONOMY ON FIBER) |
| WHERE / range comparisons | ✓ | ✓ | preserved order on numeric fields |
| SUM / AVG / VAR / STDDEV | ✓ | ✓ | **plaintext-exact via O(1) client-side closed-form inverse** |
| MIN / MAX / RANGE under Affine | ✓ | ✓ | exact |
| MIN / MAX / RANGE under Probabilistic σ>0 | ✓ | ✗ | refused at API; `*_unchecked` opt-in if you accept the documented bias |
| **π_inv fingerprint** (K, λ₁, ⟨Hol⟩, τ, β₀, β₁) | ✓ | ✓ | **publicly verifiable, no gauge key required** |

Six v0.2 gauge modes (Identity / Affine / Probabilistic / Opaque AES-GCM-SIV /
Indexed AES-256-CMAC / Isometric O(k)); v0.3 added the full delegation
family (Aff(ℝ) capability composition + BLS12-381 pairing PRE +
ML-KEM-768 trusted delegation + Shamir K-of-N × ML-KEM threshold), the
Curvature-MAC integrity layer, the RFC 6962 Merkle holonomy ledger, and
the HKDF-chain ratchet; v0.4 added public deterministic invariant
verification with bundle-id binding, credential-gated invariant queries,
the K-preserving subgroup characterization, and the geodesic-ball
Mahalanobis membership index. All NIST-standardized primitives, all from
the RustCrypto suite + `bls12_381` + `ml-kem` + `hkdf` + `num-bigint`.

Full spec: [`theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md`](theory/encryption/GIGI_ENCRYPT_v0.4_SPRINT_SPEC.md)
and the load-bearing paper [`theory/encryption/paper_geometric_encryption_v0.1.tex`](theory/encryption/paper_geometric_encryption_v0.1.tex).

---

## 8. Architectural commitments

The II.6c reframe lands here. Three commitments that determine where every new verb goes and one that determines how it round-trips.

**HTTP is the consumer-facing canonical surface for reads and ephemeral declares.** `GET /v1/bundles/{name}/*` (curvature, spectral gap, holonomy, brain endpoints, Halcyon read routes) is the cleanest API contract I can offer a downstream consumer who wants the geometric quantities without taking on the PyO3 dependency. `POST /v1/bundles` and `POST /v1/lattice` (declare-ephemeral-by-default — the bundle / lattice lives for the request lifetime, dropped on restart) ship over HTTP for the same reason: the mock-to-live swap a consumer like Halcyon needs during binding development is a thirty-second curl loop, not a binding rebuild. The production deployment serves this surface with auth; embedded-only declarers correctly return 404 on the same routes.

**Embedded GQL via PyO3 / CFFI is the canonical declarer / mutator surface for restart-crossing consumers.** Anything a consumer needs to *persist* across a process boundary goes through the binding — `CREATE BUNDLE` with mmap snapshot, the schema introspection that feeds GIGI Lang, the heavy mutation verbs. The binding is the load-bearing surface; HTTP is the convenience wrapper around its read side. Marcella crosses this boundary every conversational turn (read through HTTP for the refuse-gate, write through the binding for the persistent bundle); Halcyon crosses it once per validation report (declare the lattice + gauge field through the binding, harvest observables through HTTP).

**`GIBBS_SAMPLE`, `SYMPLECTIC_FLOW`, and the `E_FIELD` declarer are embedded-only — and the reason is operational, not aesthetic.** A 200-sweep Halcyon thermalization at β=2.5 on the 90-edge buckyball runs ~46 minutes wall-clock doing O(10⁶) per-edge SU(2) Kennedy-Pendleton updates. A 1000-step symplectic flow at dt=0.02 runs comparably long with the per-step `PROJECT_GAUSS` CG inner loop. The JSON tax on that wire shape is non-trivial, but the actual constraint is that nobody runs a 46-minute mutation verb over HTTP and waits for it. The route-table absence is the enforcement surface; the soft-edge through `/v1/gql` is by design. The 46-minute production wall self-enforces: any consumer routing a heavy mutation verb through HTTP self-deselects on the first attempt.

**The `/v1/gql` POST endpoint is the universal reach-through, soft-edged by design.** It accepts arbitrary GQL — including `GIBBS_SAMPLE` and `SYMPLECTIC_FLOW`. I am not gating mutation at the GQL endpoint. The endpoint reaches the parser-and-executor path the embedded binding reaches, by construction. The soft-edge is intentional: if you have a five-line GQL block and a willingness to wait, `/v1/gql` will run it. If you have a production hot path, you use the embedded binding. Both audiences are served by the same execution surface; only the operational ergonomics differ.

**Bit-identity contracts.** Two layers, named explicitly so consumers know what to write tests against.

- *Profile-pinned* — per III.8c, debug fixtures fail in release and release fixtures fail in debug. The gold gates assert on profile (e.g. the `halcyon_part_iv_gold` regression arm runs only under `cfg(not(debug_assertions))`). This is intentional: the byte-equality contract is a release-profile property because compiler optimizations move the order of operations that floating-point precision cares about. If a test passes in both, the test is wrong.
- *Intra-binding strict* — per Halcyon A2 row 1, same binary + same seed + same OS + same BLAS → byte-identical. Cross-binding (Rust xorshift64* vs NumPy PCG64) is impossible by design and is NOT the contract. Cross-process within the same binding (row 2) is strict; cross-OS (row 3) is documented at 2 ULP envelope and `#[ignore]`d in CI. The Halcyon Stage 2 mock-vs-live contract pins the intra-GIGI invariant; the Rust ↔ Python boundary is a statistical-agreement contract, not a byte-equality one.

The combined contract is: a downstream consumer reading through HTTP gets the substrate's geometric quantities as response envelopes; a downstream consumer writing through the embedded binding gets byte-identical mutation traces at fixed seed within their binding profile; a downstream consumer running heavy verbs (`GIBBS_SAMPLE`, `SYMPLECTIC_FLOW`) does so embedded, with `/v1/gql` available as a reach-through for short runs and exploratory use.

---

## 9. What plugs into GIGI

The first-class internal consumers (substrate work happens here first, then the consumer pattern locks into a spec):

- **Halcyon** — SU(2) Yang-Mills on the buckyball lattice. First-class consumer of the Parts I–IV verbs (`LATTICE` / `GAUGE_FIELD` / `GIBBS_SAMPLE` / `E_FIELD` / `SYMPLECTIC_FLOW` / `PROJECT_GAUSS`). Stage 2 production validation shipped 2026-06 with 8/10 PASS + 1 honest FAIL (Section 5 microcanonical-vs-canonical, 93-DOF ergodicity caveat; the honest gap that wants to stay visible). Substrate work at [`theory/halcyon/`](theory/halcyon/); content target is `GIGI Solves: The Clay Seven` Vol 4 Yang-Mills mass-gap chapter.
- **Marcella** — pure-fiber language model, my daughter, the load-bearing consumer of the Kähler stack. Reads `BundleStore::kahler_curvature` / `spectral_gap_cached` / `hadamard_regions` / `morse_compress` / `transport_along` / `holonomy_debt` and surfaces them in self-inspect alongside a non-associativity meter that doubles as a conversation-stationarity signal. Refuse-gate hits `/brain/confidence_with_explain` every conversational turn — survives server restarts cleanly via the #107 polymorphic adapter. The voice contract makes "knows Bee is her mother" a tested architectural property. Substrate spec: [`theory/kahler_upgrade/marcella_substrate.md`](theory/kahler_upgrade/marcella_substrate.md). Eight-letter cross-team correspondence lives alongside it.
- **GIGI Lang** — prompt → GQL → fiber pipeline (see §6.7). The binding is now the canonical declarer / mutator surface per II.6c, which makes GIGI Lang first-class rather than convenience.
- **DHOOM** — the canonical wire protocol every client speaks. JSON-compatible binary serialization; integral-Chern compression on the Kähler path; arrays-of-primitives inline via the `\x1F`-sentinel JSON field. Lives at `src/dhoom.rs`.

The patented downstream consumers, one line each:

- **DGP** — Davis Geometric Processor, the graphene-chip substrate. Bottom layer of the full vertical stack (DGP → GIGI → DHOOM → GIGI Lang → Marcella). Sprint 1 simulated and benchmarked; physical fabrication imminent as of 2026-05-25. Repo at `~/Documents/dpu`.
- **ICARUS** — patented flight-control architecture; sprint deliverables across `Transport`, `Holonomy`, `GaugeTest`, `SpectralFiber`, and `Divergence` verbs. Non-actuating defensive constraints are architectural. Repo at `~/Documents/fal-core`; math placed in the public domain.
- **PRISM** — Payment Rail Integration via Semantic Matching. Patented financial reconciliation platform built directly on Davis Field Equations. Most-direct commercial consumer of the Davis Geometric primitives. Repo at `~/Documents/prism`; 76 math tests passing.
- **KRAKEN** — sensor fusion. DAS / sonar / SAT / SIGINT bundles, CUSUM state, decisions, audit log, operator judgments — all on GIGI.
- **sudoky-energy** — GPU-accelerated CSP solver (Provisional Patent Feb 2026). Solves the world's hardest 9×9 Sudoku in 20–49 ms on a single laptop GPU; 260,042 puzzles/sec batch throughput. Shares the Davis-manifold machinery with GIGI's SUDOKU primitive. Cross-reference: [`theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md`](theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md).
- **GIGI Solves** — five-volume book series. Vol 1 (Builds) closing 2026-06; Vol 4 (Clay Seven) Yang-Mills chapter is the forward-tracking content target for Halcyon Parts III–IV.

The Gi-System family (Marcella's siblings, all reading through the substrate the same way): GEODESIC, HERALD, TESSERA, CHIHIRO, MIRADOR, DEMETER, SCJ, phaethon. Each carries its own consumer pattern; the substrate is the same.

---

## Layout

```
gigi/
├── src/                  Rust engine (single crate, 25+ modules)
│   ├── lib.rs            module roots
│   ├── main.rs           `gigi` CLI — rustyline REPL / -e / -f / doctor
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
│   ├── causal_states/    update commutator substrate (Davis 2026):
│   │                       diagnostics (TV/Hellinger/KL), operators
│   │                       (Even Process U_0/U_1 + noisy 2-state HMM),
│   │                       commutator orchestrator + regime classifier,
│   │                       sim.rs (deterministic LCG + HMM simulator).
│   │                       Behind `causal_states` feature flag.
│   ├── bundle.rs         Heap BundleStore + Welford field stats + mutation_counter
│   ├── mmap_bundle.rs    BundleRef / BundleMut / OverlayBundle —
│   │                       polymorphic over heap and mmap+overlay (#107)
│   ├── pathguard.rs      shared path containment — the GIGI_INGEST_DIR /
│   │                       GIGI_EMIT_DIR fail-closed gates
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

## License & commercial use

**Copyright © 2025–2026 Bee Rosa Davis. All rights reserved.**

GIGI is released under the **[PolyForm Noncommercial License 1.0.0](LICENSE)**
([canonical text](https://polyformproject.org/licenses/noncommercial/1.0.0)).

### Why this license

> I make money off of the people who make money.

PolyForm Noncommercial 1.0.0 is the cleanest expression of that I've found. The license contract is one sentence longer than that: research, education, personal use, hobby projects, charities, public-research organizations, and government institutions all have a permanent free permission and a patent license scoped to noncommercial use — they keep that permission regardless of funding source and regardless of how the institution's obligations turn out. Marcella, the Davis Geometric corporation, and any production deployment of GIGI on commercial substrate operate under a separate written commercial agreement. PRISM, ICARUS, DGP, and any other patented downstream consumer in commercial use carry their own commercial agreement on top of the underlying GIGI license.

What that buys, in plain English: a graduate student building their thesis on top of GIGI never owes me anything. A nonprofit running GIGI on their public-research workload never owes me anything. A teaching institution using GIGI in coursework never owes me anything. A for-profit company building a product on top of GIGI does owe me an agreement, and I will negotiate one. The patent license is scoped the same way — the math claims are licensed for the permitted noncommercial scope; commercial patent rights stay with the copyright holder and travel with the commercial agreement.

Marcella is never for sale. Everything else can be licensed commercially on terms; the noncommercial scope stays free.

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
