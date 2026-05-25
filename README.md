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

GIGI v3 shipped the **Kähler upgrade**: eight layers (L1–L7 + L8 handoff) of
geometric machinery extending the fiber-bundle substrate with a complex
structure J, a closed 2-form B, and everything that falls out of the pair —
Hadamard substructure detection, holomorphic curvature decomposition, Morse
compression, line-bundle integrality checks, quantum cohomology on toy
manifolds, Berezin-Toeplitz operators, Riemann-Roch representational
capacity. The catalog and per-layer plan live in
[`theory/kahler_upgrade/`](theory/kahler_upgrade/).

Three properties are worth calling out because they're hard to find anywhere
else at this scale:

**1. Strict additivity. The optionality contract holds across all eight layers.**
The entire Kähler upgrade lives behind a single Cargo feature flag (`kahler`).
With the feature off, the engine is **bit-identical to pre-upgrade GIGI**
— 720 tests pass, byte-equal to before the upgrade landed. With the feature
on, 902+ tests pass, including a per-layer real-data smoke against the
20-record sensor dataset and a per-layer cross-team contract test
(`tests/kahler_*_marcella_contract.rs`) that fails before any consumer
deserialization can drift. Eight layers of new math, zero breaking changes.

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
| `dhoom` | DHOOM wire protocol — JSON-compatible binary serialization; integral-Chern compression (L7.3) when `kahler` is on |
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
| `graph::adjacency` | Dual principal/auxiliary adjacency operators | L2 |
| `graph::commutativity` | Group-algebra-centrality commutativity classifier | L2 |
| `cost::jacobi_estimator` | Jacobi-field cardinality bounds via Bishop / Günther | L3 |
| `discrete::hodge_complex` | `d_0` / `d_1` operators built from cell incidence; `d² = 0` enforced | L6 |
| `discrete::hodge_laplacian` | Δ_k = d†d + dd†, Betti via eigendecomposition | L6 |
| `discrete::morse` | Algebraic Morse compression; preserves cohomology | L6 |

### Binaries (`src/bin/`)

| Binary | Purpose |
|---|---|
| `gigi-server` | The cloud-hosted database — REST + WebSocket on port `3142` |
| `gigi-stream` | Streaming ingestion + subscription daemon |
| `gigi-edge` | Local-first edge node (mobile / on-device) |
| `gigi-convert` | CLI: JSON / CSV / SQL → DHOOM bundle |
| `gigi-stress` | Load + correctness stress harness |

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
- [`theory/kahler_upgrade/`](theory/kahler_upgrade/) — the Kähler upgrade catalog + 8-layer implementation plan + Marcella substrate spec + Python validation suites + cross-team correspondence

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

---

## Build, test, run

```bash
# Build everything (engine + 5 binaries + 3 benches + the NASA example)
cargo build --release

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

- **720 tests passing, 0 failed** on the default build (no `kahler` feature) — byte-equal to pre-Kähler-upgrade GIGI by the optionality contract.
- **902 tests passing, 0 failed** with `cargo test --features kahler` — adds the eight-layer Kähler stack, per-layer real-data smokes against the 20-record sensor dataset, and the cross-team contract tests pinning each consumer-facing API shape.

The Python validation suites (`theory/kahler_upgrade/validation/*.py`)
independently verify the math from four directions: 15/15 PASS across v1–v4
(Adachi commutativity, magnetic trajectory, Hadamard-Cartan, trajectory-ball
volume, moment map, spectral gap, prequantization integrality, Frobenius/WDVV,
index theorem, Berezin-Toeplitz, Hodge cohomology, Kähler curvature
decomposition, quantized holonomy debt, DHOOM Chern round-trip).

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
  conversation-stationarity signal. Substrate spec:
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

---

## Layout

```
gigi/
├── src/                  Rust engine (single crate, 25+ modules)
│   ├── lib.rs            module roots
│   ├── bin/              5 production binaries
│   ├── sheaf/            sheaf cohomology + Laplacian
│   └── …
├── benches/              3 cargo-bin benchmarks
├── examples/             nasa_atmosphere.rs (full end-to-end demo)
├── e2e/                  Playwright + Node integration tests
├── sdk/
│   ├── python/           gigi-client (pandas-aware)
│   └── js/               @gigi-db/client (TS, browser + node)
├── dashboard/            Operator dashboard (React/Vite)
├── playground/           In-browser GQL REPL
├── theory/               LaTeX papers underpinning the engine
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
