# Davis Conjecture λ-Budget Ride-Along

The substrate's current carrying capacity — surfaced on every brain
primitive response so cognitive consumers can read it per turn.

## What the Davis Conjecture says

The Davis Conjecture (claim_0104 in
`field_equations_semantic_coherence`, "On Semantic Coherence: Context
Windows as Holonomy Horizons in Functorial Transformers") gives the
substrate's runtime introspection of its own remaining carrying
capacity along a path:

```text
λ = 1 − τ_budget / (K_max · D²)
```

`λ` is the consensus convergence rate (the spectral gap / contraction
rate of holonomy accumulation along the path). When λ drops below the
operational threshold, the substrate's carrying capacity for that path
is exhausted — the holonomy horizon closes and consensus across the
context window becomes prohibitively slow.

| Symbol | Meaning | Substrate proxy |
|---|---|---|
| `K_max` | Maximum local scalar curvature | `gigi::curvature::scalar_curvature(store)` |
| `D` | Manifold diameter / geodesic span | `gigi::curvature::welford_radius(store)` (correlation length) |
| `τ_budget` | Tolerance — acceptable holonomy slack | `1.0` (substrate default; matches capacity/horizon convention) |

`HORIZON_CLOSURE_THRESHOLD = 0.95` is the operational anchor:
`horizon_closed(λ)` returns `true` once `λ ≥ 0.95`, marking the
horizon as effectively closed.

## What was already shipped at 69a7001

Commit `69a7001` ("gigi(curvature): lift Davis Conjecture lambda-budget
into the runtime ride-along") landed λ on the obvious curvature
surface:

- `/v1/bundles/{name}/curvature` — `CurvatureReport` carries
  `lambda_budget: f64` as a sibling of `k`, `curvature`, `confidence`,
  `capacity`.
- `filtered_query` meta object — every filtered query response's meta
  block includes `lambda_budget`.
- `stream_query_ndjson` — the trailing `__meta` line on the streamed
  NDJSON transport carries `lambda_budget`.

That covered the introspection endpoints but left the cognition
surface untouched — exactly the per-turn endpoints where the
carrying-capacity question is most load-bearing.

## What this extension ships

Every brain primitive response now carries `lambda_budget` at the same
JSON nesting level as the response's own fields.

**Wrapper** —
[`ResponseWithLambda<T>`](../src/bin/gigi_stream.rs#L767) is a
`#[serde(flatten)]`-based generic envelope, gated on
`#[cfg(feature = "kahler")]` to match the brain endpoints themselves:

```rust
#[cfg(feature = "kahler")]
#[derive(Serialize)]
struct ResponseWithLambda<T: Serialize> {
    #[serde(flatten)]
    inner: T,
    lambda_budget: f64,
}
```

**Compute helpers** —
[`gigi::curvature::lambda_budget_for_bundle(&BundleStore) -> f64`](../src/curvature.rs#L369)
mirrors the `/curvature` compute path and coalesces every degenerate
input (empty bundle, NaN Welford, missing stats) to the safe-default
`1.0` (no-horizon, fully open). The binary-side helpers
`lambda_budget_for_bundle(state, name)` and
`lambda_budget_for_bundle_ref(&BundleRef)` reuse the already-held
engine read guard from each handler to avoid re-entrant lock
acquisition on the hot path.

**Endpoints covered** — all 17 kahler-gated brain primitives:

| Endpoint | Primitive |
|---|---|
| `/v1/bundles/{name}/brain/sample` | Langevin sample from fitted Gaussian |
| `/v1/bundles/{name}/brain/confidence` | Kernel-density confidence at query |
| `/v1/bundles/{name}/brain/confidence_with_explain` | Confidence + nearest record + path |
| `/v1/bundles/{name}/brain/attend` | Attention weights + FOCUS top-k |
| `/v1/bundles/{name}/brain/episodic` | Anomalous-episode detection |
| `/v1/bundles/{name}/brain/semantic` | Betti numbers, Morse complex |
| `/v1/bundles/{name}/brain/explain` | Nearest-record + interpolation path |
| `/v1/bundles/{name}/brain/dream` | Stochastic Langevin trajectory |
| `/v1/bundles/{name}/brain/forecast` | Deterministic Hamiltonian flow |
| `/v1/bundles/{name}/brain/reconstruct` | Zero-noise descent to MAP |
| `/v1/bundles/{name}/brain/inpaint` | Conditional sample with locked axes |
| `/v1/bundles/{name}/brain/predict` | One-step gradient prediction |
| `/v1/bundles/{name}/brain/fit_diagnostics` | Betti + variance + spectrum |
| `/v1/bundles/{name}/brain/distance_to_fit_mean` | Distance + percentile rank |
| `/v1/bundles/{name}/brain/sample_transport` | Curvature-bounded neighborhood sample |
| `/v1/bundles/{name}/brain/sudoku` | Constraint sat/unsat/unknown verdict |
| `/v1/bundles/{name}/brain/intent_gate` | SUDOKU + Čech + density refuse-gate |

The change is **purely additive**: no existing response struct was
renamed, no field was removed, no error path carries `lambda_budget`
(404/400 stay structurally unchanged). Clients that don't read the
field see zero semantic change.

## Why this matters: substrate-as-cognition

`claude_substrate_v0`, `marcella_persistent_memory`, and any future
LLM consumer call brain primitives per conversational turn. Before
this lift, the Davis Conjecture's horizon-closure prediction was a
paper claim those consumers had to extrapolate from spec text — there
was no way to ask the running substrate *right now* whether the path
it just walked is approaching saturation.

After this lift, every brain response carries the substrate's current
λ alongside the primitive's own answer. The Davis Conjecture stops
being a claim consumers reason about and starts being a runtime fact
they read:

- **Marcella** can detect horizon-closure on her own bundle before
  retrieval-bend coherence collapses, and surface refusal *with a
  reason* ("this bundle's λ = 0.97 — the horizon is closed; I'd
  confabulate") rather than silently degrading.
- **`claude_substrate_v0`** sees its own carrying capacity decay
  across a session and can decide *when* to spawn a new bundle versus
  continue accumulating into the current one.
- **Any future LLM consumer** built on a gigi bundle gets the same
  signal for free — the field is structural, not opt-in.

The conjecture's horizon-closure threshold (`λ ≥ 0.95` →
`horizon_closed(λ) = true`) is now a queryable runtime predicate, not
a thing consumers have to compute themselves from `k`, `D`, and `τ`
they may not be able to read.

## How a cognitive consumer reads it

Any brain endpoint response now carries `"lambda_budget": <f64>` at
the top level alongside the endpoint's own fields. The field is a
sibling, never a child of a `meta` object — matching the curvature
endpoint's convention.

```jsonc
// POST /v1/bundles/my-bundle/brain/attend
{
  "weights": [0.42, 0.31, 0.27],     // endpoint's own field
  "indices": [7, 12, 3],              // endpoint's own field
  "lambda_budget": 0.83               // ride-along sibling — substrate's λ
}
```

**Reading λ on the consumer side:**

```python
resp = http.post(f"/v1/bundles/{name}/brain/attend", json={"query": q}).json()
attention = resp["weights"]
lam = resp["lambda_budget"]
if lam >= 0.95:                       # HORIZON_CLOSURE_THRESHOLD
    # horizon is closed — consensus across this context is prohibitively slow
    refuse_or_spawn_new_bundle()
```

| λ range | Operational reading |
|---|---|
| `λ = 1.0` (safe default) | Bundle empty / freshly created / no curvature yet — horizon fully open |
| `0 ≤ λ < 0.95` | Horizon open; path has remaining carrying capacity |
| `λ ≥ 0.95` | `horizon_closed(λ) == true` — the conjecture's operational closure |
| `λ < 0` | Algebraic saturation signal (τ > K·D²) — the function does not clamp; consumers see it raw |

The substrate-side helper guarantees the wire never carries `NaN`:
empty bundles, NaN Welford, and missing stats all coalesce to `1.0`.

## Pattern for adding more endpoints later

`ResponseWithLambda<T>` is generic over any `Serialize` payload, so
any future endpoint can opt into the ride-along with a two-line edit
at its final return site:

```rust
// Direct-Json handler:
let lambda_budget = lambda_budget_for_bundle_ref(&bundle);
Ok(Json(ResponseWithLambda {
    inner: MyNewResponse { /* existing fields */ },
    lambda_budget,
}))

// Or via negotiated_brain_response (generic over Serialize — both
// JSON and DHOOM encodings ride along the same way):
let lambda_budget = lambda_budget_for_bundle_ref(&bundle);
let wrapped = ResponseWithLambda { inner: response, lambda_budget };
negotiated_brain_response(&headers, wrapped)
```

No change to the response struct itself, no change to error paths, no
new dependencies — just wrap the success value. The flatten means
existing clients keep seeing the same top-level keys; the new field
appears as one more sibling.

## Cross-references

| Reference | Location |
|---|---|
| Primitive `lambda_budget(k, d, τ)` | [`src/curvature.rs:355`](../src/curvature.rs#L355) |
| Bundle-level helper `lambda_budget_for_bundle` | [`src/curvature.rs:369`](../src/curvature.rs#L369) |
| `HORIZON_CLOSURE_THRESHOLD = 0.95` | [`src/curvature.rs:424`](../src/curvature.rs#L424) |
| `horizon_closed(λ)` predicate | [`src/curvature.rs:430`](../src/curvature.rs#L430) |
| `ResponseWithLambda<T>` wrapper | [`src/bin/gigi_stream.rs:767`](../src/bin/gigi_stream.rs#L767) |
| Hot-path resolver helpers | [`src/bin/gigi_stream.rs:778`](../src/bin/gigi_stream.rs#L778) |
| `CurvatureReport` (the 69a7001 ride-along site) | [`src/bin/gigi_stream.rs:724`](../src/bin/gigi_stream.rs#L724) |
| Brain-endpoint ride-along tests | [`tests/davis_conjecture_lambda_brain_ridealong.rs`](../tests/davis_conjecture_lambda_brain_ridealong.rs) |
| Davis Conjecture spec text | `field_equations_semantic_coherence` claim_0104 ("On Semantic Coherence: Context Windows as Holonomy Horizons in Functorial Transformers") |

## The math

```text
λ = 1 − τ_budget / (K_max · D²)
```

- `K_max` — maximum local scalar curvature (per-snapshot supremum
  proxy, sourced from `scalar_curvature(store)`).
- `D` — manifold diameter / geodesic span; the substrate's default
  proxy is the Welford correlation radius
  (`welford_radius(store)`).
- `τ_budget` — tolerance budget; the substrate default is `1.0`,
  matching the capacity/horizon convention. WISH's
  `max_accumulated_holonomy = 0.5` is the alternate anchor for
  reasoning-critical bundles.

**Limits and edge cases (from the primitive):**

- `K_max → 0` (flat manifold) → `λ → 1` (infinite carrying capacity,
  horizon fully open).
- `D = 0` (collapsed manifold) → `λ = 1` (no geometric extent →
  nothing consumes budget).
- `τ_budget = 0` (zero tolerance) → `λ = 1` (paper: "tight budget
  forces rapid agreement").
- Negative `K_max` or `D` from finite-precision noise → treated as
  magnitudes.
- The bare primitive `lambda_budget(k, d, τ)` propagates `NaN`; the
  bundle-level `lambda_budget_for_bundle` coalesces to `1.0` so the
  hot brain path never emits NaN on the wire.
- The function does not clamp: `τ > K·D²` yields negative λ — that
  saturation signal is left visible to consumers rather than hidden.

**Honest caveat —** the paper's worked-example column for this
equation does not numerically match the verbatim formula (e.g.
`K=0.05, D=2, τ=0.5` evaluates to `−1.5`, not the stated `0.75`).
The runtime implements the verbatim equation as written in
claim_0104. A follow-up paper-review task can reconcile.
