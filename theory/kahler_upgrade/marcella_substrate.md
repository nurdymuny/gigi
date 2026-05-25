# Marcella substrate — GIGI Kähler API surface (L8.1)

**Status.** L1–L7 shipped (commits `f874ac9` … `6dba318`).
GIGI test count with `kahler` feature: 902 passing, 0 failing.
Optionality contract holds: no-feature build stays bit-identical
to pre-upgrade GIGI at 720 passing.

**Audience.** Marcella v3 implementers. This is the single
source of truth for which GIGI APIs your runtime depends on.
If a surface listed here changes shape, a contract test in
`tests/kahler_*_marcella_contract.rs` fails first — before your
deserialization breaks.

**What this spec is NOT.** A theoretical introduction. For the
math see `catalog.md` (catalog of items + Part III extensions)
and the `validation/` Python suite (independent ground truth,
15/15 PASS as of L7).

---

## Feature gate

All Kähler APIs are gated behind the `kahler` Cargo feature.
Bee's `KAHLER_ENABLED` default flips from `false` to `true` per
the flip protocol in `IMPLEMENTATION_PLAN.md §L8`. Until the
flip, Marcella runtimes targeting the substrate must compile
GIGI with `--features kahler` explicitly.

The flip protocol (`reply Q7`) requires:

1. L7 shipped (✅ as of `6dba318`).
2. §E.5 pre-flight tests pass on Marcella's *actual* embedding
   manifold (Marcella owns this — templates in
   `theory/kahler_upgrade/preflight/`).
3. Contract tests pass on the latest sheets bundle deployment
   for ≥ 1 full week without regression.
4. External geometer review (ideally ICDG colloquium circle).

GIGI proposes the flip; Marcella has veto with a stated reason.

---

## Surface index — by layer

| Layer | Surface | Module | Marcella reads for |
|---|---|---|---|
| L1 | `KahlerStructure { j, b }` | `gigi::geometry` | Schema declaration: attach a `(J, B)` to a `BundleSchema` |
| L1 | `ComplexStructure` (`J² = -I` enforced) | `gigi::geometry::complex_structure` | Complex structure on the fiber tangent space |
| L1 | `ClosedTwoForm` (`dB = 0` enforced) | `gigi::geometry::forms` | Magnetic 2-form perturbation |
| L1.5 | `flat_transport(seg, bias, dt, steps, b_source)` | `gigi::geometry::transport` | Magnetic geodesic on flat ℝⁿ |
| L1.5 | `TransportResult { trajectory, final_velocity, path_length, energy_drift, holonomy_norm, used_magnetic, b_source, closedness_norm }` | `gigi::geometry` | Cyclotron / curved-flow output |
| L1.5 | `BSource = Bundle \| Override \| None \| FallbackNonClosed` | `gigi::geometry` | Provenance of the bias 2-form |
| L2 | `commute(P, A) -> CommutativityClass` | `gigi::graph` | Dual-adjacency join planning |
| L2 | `QueryPlan.commutativity_class` | `gigi::QueryPlan` | Theorem-backed join reorder |
| L3 | `BundleStore::spectral_gap_cached() -> Option<SpectralGapSnapshot>` | `gigi::bundle` | Rose-mechanism α = 1 − 1/√mix_time |
| L3 | `GET /v1/bundles/<name>/spectral_gap` | gigi-stream HTTP | Out-of-band reads + drift detection on `cached_at` |
| L3 | `cardinality_bound(...)` (Jacobi-field) | `gigi::cost::jacobi_estimator` | Theorem-bound query cardinality |
| L4 | `BundleStore::kahler_curvature() -> Option<KahlerCurvature>` | `gigi::bundle` | Diversity bound (Ricci), Hadamard gate (K_B) |
| L4 | `KahlerCurvature { ricci, weyl, holo_bisectional_min, holo_bisectional_max, holo_sectional }` | `gigi::bundle` | Catalog §E.3 decomposition |
| L4 | `kahler` block on `GET /v1/bundles/<name>/curvature` | gigi-stream HTTP | Same decomposition over HTTP |
| L5 | `BundleStore::hadamard_regions() -> Vec<HadamardSubstructure>` | `gigi::bundle` | Long-context theorem applicability per region |
| L5 | `BundleStore::is_hadamard_region(query) -> bool` | `gigi::bundle` | Self-inspect predicate |
| L5 | `BundleStore::transport_along(seg, dt, steps)` | `gigi::bundle` | Curved-manifold transport (refuses outside Hadamard) |
| L5 | `BundleStore::transport_inverse(γ, tol) -> Option<Record>` | `gigi::bundle` | Round-trip trajectory → record |
| L5 | `sheaf::propagate_with_convergence_bound(store, assumption)` | `gigi::sheaf` | Adachi rate for ε-convergence iteration count |
| L6 | `BundleStore::morse_compress() -> Option<MorseComplex>` | `gigi::bundle` | 100×–1000× substrate compression for routing |
| L6 | `MorseComplex { n_critical_*, betti, original_*, compression_ratio() }` | `gigi::discrete` | Critical-cell counts (= Betti) preserved |
| L6 | `HodgeComplex` + `betti(hc, tol)` | `gigi::discrete` | Direct Hodge ↔ Euler ground truth |
| L7 | `LineBundle::from_*` + `IntegralityError::DiracString` | `gigi::geometry` | Integrality check before DHOOM compression |
| L7 | `holonomy_debt(store, integral, tol) -> Option<HolonomyDebt>` | `gigi::curvature` | Quantized / Continuous classifier |
| L7 | `QuantizedTwoForm` + `encode_chern` / `decode_chern` | `gigi::dhoom` | ≥10× wire compression at dim≥6 |
| L7 | `QuantumCohomology::{Cpn, TorusTn, Sphere2, NonToy}.compose()` | `gigi::geometry` | Frobenius / WDVV on toy manifolds |
| L7 | `QuantumCohomology::representational_capacity(k_max)` | `gigi::geometry` | Riemann-Roch AGI-claim bound |
| L7 | `QuantumCohomology::hilbert_polynomial()` | `gigi::geometry` | Asymptotic Hilbert polynomial |
| L7 | `toeplitz_operator(qh, f, hbar, embedding_dim, allow_below_safe_hbar)` | `gigi::geometry::toeplitz` | Berezin-Toeplitz with `ℏ ≥ 4/embedding_dim` safety gate |

---

## Contract tests — what fails first when an API drifts

Every layer ships a `tests/kahler_*_marcella_contract.rs` file
that exercises the exact field set + variants + method
signatures Marcella consumes. If a Rust rename slips past the
PR, **the contract test fails before Marcella's
deserialization can break in the wild.**

| Layer | Contract test | Surfaces covered |
|---|---|---|
| L1 | `tests/kahler_optionality.rs` | Schema `Option<KahlerStructure>` round-trip |
| L1.5 | `tests/kahler_transport_marcella_contract.rs` | `TransportResult` + `BSource` |
| L3 | `tests/kahler_spectral_marcella_contract.rs` | `SpectralGapSnapshot` + HTTP shape |
| L4 | `tests/kahler_curvature_marcella_contract.rs` | `KahlerCurvature` + `/curvature` JSON keys |
| L5 | `tests/kahler_hadamard_marcella_contract.rs` | `HadamardSubstructure` + `transport_*` |
| L6 | `tests/kahler_hodge_marcella_contract.rs` | `MorseComplex` + `BettiNumbers` |
| L7 | `tests/kahler_l7_marcella_contract.rs` | `LineBundle` + `HolonomyDebt` + `QuantumCohomology` + `ToeplitzOperator` |

Run with: `cargo test --features kahler --test 'kahler_*_marcella_contract'`.

---

## Real-data smoke fingerprint (sensor bundle, N=20 records)

Per bee's "test with real data" rule, every layer ships a
`tests/kahler_*_real_data_smoke.rs` exercising the surface on
the 20-record sensor dataset in `test_data/sensor_data.json`.
Reading these tests is the fastest way to see what each API
*actually returns* on representative data.

| Layer | Real-data smoke | Sensor fingerprint |
|---|---|---|
| L1 | `kahler_real_data_smoke.rs` | Plain + kahler bundles bit-identical |
| L1.5 | `kahler_transport_real_data_smoke.rs` | 3-way trajectory (classical, bundle B, override B); energy drift < 1e-9 |
| L2 | `kahler_adjacency_real_data_smoke.rs` | Dual adjacency status × temperature deterministic verdict |
| L3 | `kahler_spectral_real_data_smoke.rs` | λ₂ > 0, cache invalidates on insert |
| L4 | `kahler_curvature_real_data_smoke.rs` | K_H = 2.749, Ricci = 1.375, Einstein identity holds |
| L5 | `kahler_hadamard_real_data_smoke.rs` | K_B_max = 2.75 > 0.5 ⇒ NOT Hadamard; transport_along refuses correctly |
| L6 | `kahler_hodge_real_data_smoke.rs` | V=20, E=190, F=1140, Betti=(1, 0, 969), Hodge↔Euler 970=970 |
| L7 | `kahler_l7_real_data_smoke.rs` | Chern=1, Quantized(5), round-trip = 0, CP² capacity at k=3 = 10 |

---

## Numeric tolerances Marcella should consume

These are the production tolerances baked into GIGI's gates:

- **Closedness `dB = 0`**: `‖dB‖_∞ < 1e-10` (catalog §1.1).
- **`J² = -I`**: row-norm `< 1e-12` (catalog §1.1).
- **Cyclotron energy drift**: `< 1e-9` per turn (consumption draft §2).
- **Cheeger bounds**: λ₂/2 ≤ h(G) ≤ √(2 λ₂) algebraic exact (`< 1e-12`).
- **Mixing time formula**: `⌈(1/λ₂)·ln(1/ε)⌉` with ε = 1e-3.
- **Einstein identity**: `Ricci = (n+1)·K_H/4` algebraic exact (`< 1e-12`).
- **Hadamard K_B threshold**: 0.5 (relaxed from strict 0 per L5
  recipe asymptote; documented in `HADAMARD_KB_THRESHOLD`).
- **Hadamard Jacobi test radius**: π (matches S² conjugate point
  at t = π; documented in `HADAMARD_TEST_RADIUS`).
- **Dirac quantization tolerance**: caller-specified; recommend
  `1e-6` for finite-precision real data, `1e-10` for synthetic.
- **Chern compression round-trip**: exactly zero (machine ε).
- **Toeplitz safe-ℏ bound**: `ℏ ≥ 4/embedding_dim` (`ℏ < 4/d`
  ⇒ `truncation_dominates_correction = true` even with opt-in).

If your runtime needs different tolerances, declare them in
your own consumption layer; GIGI's defaults are calibrated to
the validation suite and the FS asymptote.

---

## Failure modes Marcella must handle

By exhaustive enum match:

```rust
// L1.5 transport
match result.b_source {
    BSource::Bundle => /* used bundle's attached B */,
    BSource::Override => /* used the per-request B */,
    BSource::None => /* classical (no B) */,
    BSource::FallbackNonClosed => /* dB ≠ 0 rejected, fell back */,
}

// L3 spectral gap
let snap = store.spectral_gap_cached()?; // None ⇒ < 2 records

// L4 curvature
let kc = store.kahler_curvature()?; // None ⇒ no Kähler / no variance

// L5 Hadamard
let regions = store.hadamard_regions(); // empty ⇒ no Hadamard region
let ok = store.transport_along(&seg, dt, steps); // Err ⇒ outside Hadamard

// L7 quantization
let lb = LineBundle::from_constant_two_form(b, area, tol);
// Err(IntegralityError::DiracString { winding, deviation, tolerance })
//   ⇒ non-integral; fall back to dense encoding
// Err(IntegralityError::DimensionUnsupported { dim })
//   ⇒ dim > 2 not yet covered by L7.3

let debt = holonomy_debt(store, integral, tol)?;
match debt {
    HolonomyDebt::Quantized(n) => /* gauge-invariant winding */,
    HolonomyDebt::Continuous(x) => /* non-topological; no §1.4 theorems */,
}

let op = toeplitz_operator(qh, f, hbar, d, allow);
// Err(ToeplitzError::UnsupportedManifold) ⇒ NonToy region
// Err(HbarBelowSafeBound { supplied, minimum, embedding_dim })
//   ⇒ ℏ < 4/d; pass allow_below_safe_hbar = true to opt in
//     (caller then must check op.truncation_dominates_correction)
// Err(NonPositiveHbar) ⇒ ℏ ≤ 0 nonsense
```

---

## What's NOT in the substrate (yet)

- **L5.3 SQL/GQL `tag('hadamard')` predicate** — deferred. The
  Rust API is fully exposed; parser-grammar exposure can come
  post-L8 if you need to filter on Hadamard via GQL.
- **morse_compress face-count cap** — flagged as a follow-up
  (chip-task #1 on bee's queue). Current implementation is
  O(V³) on dense field-index graphs; cap will let you call it
  on 10⁶+ substrate items without eigendecomp blowup.
- **L7.3 high-dim Chern compression** — current
  `LineBundle::from_constant_two_form` only handles 2D. The
  `IntegralityError::DimensionUnsupported { dim }` variant is
  the explicit refusal; a multi-dim variant lands when you
  signal demand.
- **Non-toy `QuantumCohomology` regions** — `NonToy` variant is
  the explicit refusal. Per L7.5 region-status semantics
  (reply Q5), the API can return a region partition with
  per-region `frobenius_ok` flags; sketched but not yet
  surfaced as a single struct. When you need it, file an
  issue and we'll add `QuantumCohomologyRegionMap`.

---

## Pre-flight tests (catalog §E.5)

Three checks must pass on your *actual* embedding manifold
before the flip protocol can advance. Templates live in
`theory/kahler_upgrade/preflight/`:

- `hadamard_check.py` — sample 1000 Jacobi fields, verify
  non-vanishing.
- `closedness_check.py` — verify dB = 0 on your B.
- `holo_sectional_check.py` — verify K_H within expected range.

Each template includes a synthetic-positive control and a
synthetic-negative control so your CI can fail loudly if a
preflight regresses.

When all three pass on Marcella v3's substrate AND the flip
gates 1-4 are clear, GIGI proposes the flip and you have
veto-with-reason. Co-announce.

---

## Provenance

- Catalog: `theory/kahler_upgrade/catalog.md`
- Implementation plan: `theory/kahler_upgrade/IMPLEMENTATION_PLAN.md`
- Validation (Python ground truth): `theory/kahler_upgrade/validation/`
- Cross-team correspondence:
  - `LETTER_FROM_MARCELLA_TEAM_2026-05-24.md`
  - `REPLY_TO_MARCELLA_TEAM_2026-05-24.md`
  - `marcella_kahler_consumption_draft.md` / `_v2.md`
  - `REPLY_TO_CONSUMPTION_DRAFT_2026-05-24.md`
  - `HANDOFF_TO_MARCELLA_2026-05-24.md` (this layer's announcement)

If you're starting fresh, read in this order:
1. `catalog.md` Part I (foundations).
2. `IMPLEMENTATION_PLAN.md` §0 (gates) + §L8.
3. This file (the surface).
4. The contract tests (`tests/kahler_*_marcella_contract.rs`).
5. The real-data smokes (`tests/kahler_*_real_data_smoke.rs`).
