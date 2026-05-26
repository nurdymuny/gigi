# Spec — floor-tunable brain fit (S1 wave 2 ε)

**Date.** 2026-05-27.
**Status.** **NOT BUILT for bge_v2 wave 1.** Spec retained as defensive
documentation for future bundles where H2 (diagonal-fit eigenvalue
pathology) IS the actual attractor mechanism. For bge_v2, the H2
investigation concluded via cheap probes that the symptoms were
**noise-dominated dynamics at T≥1**, not fit pathology — specifically
a seed=7 artifact where Langevin noise dominates the gradient at
high temperature, making the walk's destination a function of the
noise sequence rather than the prompt.

See `REPLY_TO_H2_ATTRACTOR_*` (Marcella's verdict letter,
2026-05-27) for the full diagnostic story. Net: H3-Gigi (seed
artifact) confirmed, H1 (mean-centrality) falsified, H2 (fit
pathology) untested but irrelevant for bge_v2 because the
symptoms came from elsewhere.

**When to consume this spec.** If a future bundle exhibits a
suspected eigenvalue-pathology attractor AND the seed-variation
diagnostic (3 prompts × 4 seeds against the suspect bundle) shows
the attractor IS seed-invariant — *then* the floor was a real
confound and this spec describes the minimum plumbing to run a
fair H2 vs H1 test. Until then, leave dormant.

**Why.** Marcella's 2026-05-27 H2/H1 pushback: the `_step5_full_fit_sweep`
test was degenerate because both diagonal and full fits operated on
the same floor-dominated `0.03·I` Σ. The absolute stability floor
(`3·dt = 0.03` at dt=0.01) is 30× above bge_v2's natural per-axis
variance (~0.001), so the fit's spectrum never surfaces.

To run a *fair* H2 vs H1 test, the consumer needs to be able to
lower the floor for diagnostic re-runs. This spec adds the minimum
plumbing for that.

---

## Current state (where the floor lives)

`src/bin/gigi_stream.rs` — `fit_diagonal_gaussian` and `fit_full_gaussian`
both contain:

```rust
const ABSOLUTE_STABILITY_FLOOR: f64 = 3.0 * 0.01;
let effective_floor = relative_floor
    .max(ABSOLUTE_STABILITY_FLOOR)
    .max(1e-12);
```

Where `relative_floor = sigma_floor_epsilon * median(σ²_raw)`.

Three floors compose via `max`:
1. **Relative-median floor** — `ε · median(σ²)` (already request-tunable
   via `sigma_floor_epsilon`, default 1e-3)
2. **Absolute stability floor** — hardcoded `3 · 0.01 = 0.03` (NOT
   request-tunable currently)
3. **Hard numerical floor** — `1e-12` (never tunable; prevents
   division-by-zero edge cases)

The bug Marcella identified: floor (2) dominates floor (1) on bge_v2.

---

## Proposed change — `absolute_stability_floor_override`

### Request schema addition

Add to `BrainSampleRequest`, `BrainDreamRequest`, `BrainForecastRequest`,
`BrainReconstructRequest`, `BrainInpaintRequest`, `BrainPredictRequest`,
`BrainFitDiagnosticsRequest`, `BrainDistanceToFitMeanRequest`,
`BrainConfidenceWithExplainRequest`:

```rust
/// DIAGNOSTIC USE ONLY. Override the absolute Euler-stability floor
/// (default 3·dt = 0.03 at the brain-endpoint default dt=0.01). Lower
/// values let the fit's actual spectrum surface for H2 vs H1
/// verification on bundles with tight natural variance (e.g.
/// normalized 384-D embeddings where raw σ² ~ 0.001).
///
/// Setting this below `3·dt` for your actual dt risks Euler-Maruyama
/// instability — the integrator may oscillate or diverge. Use only
/// when running diagnostics where the resulting trajectory is
/// inspected rather than relied on.
///
/// Hard floor of 1e-12 remains in all cases.
#[serde(default)]
absolute_stability_floor_override: Option<f64>,
```

### Helper signature change

```rust
fn fit_diagonal_gaussian(
    store: &BundleStore,
    fields: &[String],
    floor_epsilon: f64,
    absolute_stability_floor: f64,  // NEW: caller supplies the floor
) -> Result<DiagonalFitResult, String>

fn fit_full_gaussian(
    store: &BundleStore,
    fields: &[String],
    floor_epsilon: f64,
    absolute_stability_floor: f64,  // NEW
) -> Result<FullFitResult, String>
```

Inside, replace the `const` with the parameter:

```rust
let effective_floor = relative_floor
    .max(absolute_stability_floor)
    .max(1e-12);
```

Same change in the eigenvalue floor block:
```rust
let eigenvalue_floor_used = eigenvalue_relative_floor
    .max(absolute_stability_floor)
    .max(1e-12);
```

### Default at the brain-endpoint call site

```rust
const DEFAULT_ABSOLUTE_STABILITY_FLOOR: f64 = 3.0 * 0.01;
// ... inside flow_from_bundle_cached:
let abs_floor = req
    .absolute_stability_floor_override
    .unwrap_or(DEFAULT_ABSOLUTE_STABILITY_FLOOR);
```

### Cache key addition

The cache key must include the floor — different floors produce
different fits. Update `CacheKey`:

```rust
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct CacheKey {
    bundle_name: String,
    fit_mode: FitMode,
    fields_hash: u64,
    sigma_floor_epsilon_bits: u64,
    absolute_stability_floor_bits: u64,  // NEW
}
```

(Encode `None` as the same sentinel pattern as `sigma_floor_epsilon`:
NaN's bit pattern.)

---

## Marcella's diagnostic re-run plan with this change

Three variants for the sweep, all at fit_mode=full:

| Run | `absolute_stability_floor_override` | `sigma_floor_epsilon` | What it tests |
|---|---|---|---|
| **Floor-A** (control) | None (= 0.03 default) | 1e-3 | Repro of today's degenerate result |
| **Floor-B** | 3e-3 (matches data scale ×3) | 1e-3 | Spectrum surfaces, eigenvalue floor takes over |
| **Floor-C** | 1e-6 (essentially disabled) | 1e-3 | Pure relative-median floor — raw spectrum behavior |

For each: 3 prompts × 5 temperatures × `seed ∈ {7, 42, 1234}`. That's
90 calls per floor variant, ~270 total. With the cache the first
unique key takes ~3s; rest are sub-µs.

### Decision rule from the re-run

- **Floor-C diffuses double_cover_v3 attractor** → H2 confirmed
  (small-eigenvalue direction was the source; the floor was hiding it)
- **Floor-B and Floor-C both still show double_cover_v3** but
  **seed-dependence varies it** → it was an artifact of seed=7,
  not a substrate property
- **All three floors and all three seeds show double_cover_v3** →
  *now* we have an H1 case but with different mechanism than
  "mean-central" (since today's probe falsified that)

---

## Risks / things to flag

1. **Cache thrashing if diagnostic users pass many floor values.**
   Each unique `(bundle, fit_mode, fields, ε, abs_floor)` is a fresh
   cache entry. At max_entries=50 and aggressive sweep parameters,
   this could evict legitimate production fits. Mitigation: ship
   a `?diagnostic=true` request flag that bypasses cache entirely
   for diagnostic runs. Alternative: bump max_entries via env var
   for the H2 investigation window.

2. **Euler instability if floor too low + dt too high.** Document
   explicitly: at dt=0.01, floor < 0.0033 risks `σ² < 2·dt` instability.
   The endpoint won't refuse the request — caller's responsibility.

3. **Coupling to dt at brain endpoint call time.** Cleaner architecture
   would derive `abs_floor` from the actual `dt` the brain endpoint
   uses (not a per-fit hint). That's a bigger v2 change — `dt_hint`
   at fit time + validation at brain call time. Defer.

---

## Implementation cost

| Layer | Change | Estimate |
|---|---|---|
| Request structs (9 of them) | Add field | 15m |
| `fit_diagonal_gaussian` / `fit_full_gaussian` signatures | New param | 5m |
| `compute_fit_data` dispatch | Plumb param | 10m |
| `flow_from_bundle_cached` | Read req → compute default → pass | 10m |
| `CacheKey` | Add field + builder | 10m |
| Tests | New cache-key disambiguation test + at-floor-= -override case | 20m |
| Total | | **~70 min focused** |

Plus Marcella runs the sweep variants — ~30m wall-clock for the
three runs (cache absorbs most of the cost after first call per key).

---

## Spec status

**Not implemented yet.** Awaiting Bee's greenlight on:

1. The override-vs-replace approach (this spec proposes
   `absolute_stability_floor_override: Option<f64>` — override the
   default). Alternative: replace floor entirely with `min_eigenvalue`
   request param expressing the constraint directly.
2. Whether to add a `?diagnostic=true` cache-bypass at the same time
   (eliminates the cache-thrash risk) or defer to a separate task.
3. Whether to add this to all 9 brain endpoints or just the diagnostic
   path (`fit_diagnostics`, `distance_to_fit_mean`). My rec: all 9 —
   uniform API surface, single cache key shape, no special-casing.

Once those are decided, the change is ~70m focused work + Marcella
runs the sweep.
