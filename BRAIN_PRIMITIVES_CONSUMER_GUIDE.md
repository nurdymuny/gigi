# Brain primitives — consumer guide

**One doc. All 12 primitives. Read this before you wire anything.**

If you're Marcella / MIRADOR / PRISM / DPU and you want to consume
GIGI's brain primitives over HTTP or by linking the crate, this is
the single reference. Replaces what was previously scattered across:

- `theory/brain_primitives/catalog.md` (math)
- `src/geometry/{generative_flow, predictive_coding, attention, memory}.rs` (Rust API docs)
- `tests/kahler_brain_endpoints_contract.rs` (wire shapes)
- 4 demo binaries
- ~10 Marcella correspondence letters

---

## The one-screen mental model

Every brain primitive is one of three boundary conditions on the
same Friston master equation on a Kähler bundle:

```
ẋ = -∇H(x) dt + √(2T) dW       (dissipative; uses no B)
ẋ =  B⁻¹∇H(x)                  (conservative; Hamiltonian)
```

with `H(x) = -log p(x)` from the bundle's Welford-streaming fit.
The primitives differ only in **boundary conditions** (initial
state, temperature, locked coords, integration budget) and
**post-processing** (max-of-attention, change-point on sorted
values, etc.).

> Catalog: `theory/brain_primitives/catalog.md`.
> Validation: 26/26 Python tests in `theory/brain_primitives/validation_tests.py`.

---

## Quick reference — use case → primitive

| You want… | Use | Endpoint |
|---|---|---|
| Generate novel records from p | **SAMPLE** | `POST /brain/sample` |
| Predict the next state along a flow | **FORECAST** | `POST /brain/forecast` |
| Novel-but-plausible outputs (REM-sleep mode) | **DREAM** | `POST /brain/dream` |
| Denoise a noisy observation → MAP | **RECONSTRUCT** | `POST /brain/reconstruct` |
| Fill in missing fields of a partial record | **INPAINT** | `POST /brain/inpaint` |
| Take one predictive-coding step from current state | **PREDICT** | `POST /brain/predict` |
| Find what's relevant to a query (soft) | **ATTEND** | `POST /brain/attend` |
| Find top-k closest records (hard) | **FOCUS** | `POST /brain/attend` w/ `top_k` |
| Detect change-points in a value sequence | **EPISODIC** | `POST /brain/episodic` |
| Get the bundle's topological gist | **SEMANTIC** | `GET /brain/semantic` |
| Refuse-to-generate gate ("I don't know") | **SELF-MONITOR** | `POST /brain/confidence` |
| Show how to get from query to nearest known | **EXPLAIN** | `POST /brain/explain` |

**All endpoints under `/v1/bundles/{name}/brain/*`.** All require
`Authorization: Bearer $GIGI_API_KEY`. All require the bundle to
be heap-resident (the engine returns 404 if it's only on-disk
mmap).

---

## Per-primitive reference

Each section: what the math is, when to use it, the wire shape,
defaults that matter, common pitfalls.

### §2 SAMPLE — Langevin draws from the bundle's density

**What.** Stationary draws from `p(x) ∝ exp(-H(x))` via Langevin
SDE. At temperature `T = 1` you get faithful samples of the
bundle's empirical density (Roberts–Tweedie 1996 convergence).

**When.** Generative inference: "give me a record that looks
like one of yours."

**Request.**
```json
POST /v1/bundles/{name}/brain/sample
{
  "fields": ["x", "y"],
  "n_samples": 100,
  "temperature": 1.0,
  "burn_in": 2000,
  "seed": 42,
  "fit_mode": "isotropic",
  "sigma_floor_epsilon": 0.001
}
```

**Response (truncated; full shape in contract test).**
```json
{
  "samples": [[...], [...], ...],
  "fit_mean": [...],
  "fit_sigma_sq": 1.23,
  "fit_sigma_sq_per_field": [...],
  "fit_sigma_sq_per_field_raw": [...],
  "fit_sigma_floor_used": 0.03,
  "fit_floored_indices": [],
  "fit_mode_used": "isotropic"
}
```

**Tuning.** `temperature: 4.0+` reshapes SAMPLE into DREAM
(see §4). `fit_mode: "diagonal"` is recommended for anisotropic
manifolds (learned embeddings); see §"Fit modes" below.

---

### §3 FORECAST — Hamilton-flow extension from a seed

**What.** Deterministic conservative flow `ẋ = B⁻¹∇H`. Energy is
conserved; trajectories trace closed orbits for quadratic H.

**When.** Predictive extension: "starting from here, where does
the bundle's flow take this state next?"

**Request.**
```json
POST /v1/bundles/{name}/brain/forecast
{
  "fields": ["x", "y"],
  "initial": [3.0, 2.0],
  "n_steps": 1000,
  "dt": 0.01,
  "fit_mode": "isotropic"
}
```

**Response.** `trajectory: Vec<Vec<f64>>` of length `n_steps + 1`,
plus `fit_mean`, `fit_sigma_sq`.

**Tuning.** No temperature (deterministic). Lower `dt` → smaller
truncation error but slower. Energy drift over the trajectory is
the diagnostic — if it grows, your dt is too big.

---

### §4 DREAM — high-temperature Langevin trajectory

**What.** Same SDE as SAMPLE but with `T ≫ 1`. The drift still
pulls toward high-density regions, but the noise dominates →
the trajectory wanders into states the data never visited.

> Note: `POST /brain/sample` with high T gives stationary draws.
> `POST /brain/dream` gives the full trajectory (order matters;
> narrative structure preserved).

**When.** Creative generation. Counterfactual exploration.
Out-of-distribution synthetic data. "Show me what novel-but-
plausible looks like."

**Request.**
```json
POST /v1/bundles/{name}/brain/dream
{
  "fields": ["x", "y"],
  "initial": [0.0, 0.0],
  "n_steps": 1000,
  "temperature": 4.0,
  "dt": 0.01,
  "seed": 42,
  "fit_mode": "isotropic"
}
```

**Response.** `trajectory` + `mean_dist_from_mean`,
`max_dist_from_mean` (diagnostics for "how far did it wander?").

**Tuning.** Higher T → wider exploration but less coherent.
On Hadamard sub-bundles (catalog §1.4 ideal-boundary), DREAM
stays coherent at higher T than on positive-curvature regions.

---

### §5 RECONSTRUCT — zero-noise descent to MAP

**What.** Deterministic gradient descent on `H = -log p`.
Converges to the local mode nearest the starting point.

**When.** Denoising. MAP estimation. Refining a noisy observation
to the closest "what the bundle thinks this should be."

**Request.**
```json
POST /v1/bundles/{name}/brain/reconstruct
{
  "fields": ["x", "y"],
  "noisy_initial": [10.0, 10.0],
  "n_steps": 500,
  "dt": 0.05,
  "fit_mode": "isotropic"
}
```

**Response.** `result: Vec<f64>` (MAP), `descent_distance`
(how far it had to descend — large value flags noisy input).

**Tuning.** Default `dt = 0.05` is larger than other endpoints —
descent doesn't need fine resolution. Multimodal H means
RECONSTRUCT finds the *nearest* local mode, not the global one.

---

### §6 INPAINT — constrained Langevin (fill in missing fields)

**What.** Locks a subset of coordinates at supplied values; samples
the rest from the conditional density `p(x_unlocked | x_locked)`.

**When.** Partial records. "I have weight but not clearance —
fill in clearance from the cohort's conditional distribution."

**Request.**
```json
POST /v1/bundles/{name}/brain/inpaint
{
  "fields": ["weight", "clearance"],
  "partial_state": [90.0, 0.0],
  "locked_indices": [0],
  "burn_in": 2000,
  "dt": 0.05,
  "temperature": 1.0,
  "seed": 12345,
  "fit_mode": "isotropic"
}
```

**Response.** `result: Vec<f64>` (locked coords exact;
unlocked coords are MC draws), `locked_indices` (echo).

**Tuning.** Default `dt = 0.05`. For a tight conditional, raise
`burn_in`. `temperature: 0` collapses to "what's the most likely
unlocked value given the locked ones?" (similar to RECONSTRUCT
on a constrained domain).

---

### §7 PREDICT — single Fisher-natural-gradient step

**What.** One forward step `x_{t+1} = x_t - lr · ∇H(x_t)` (with
Fisher-natural preconditioning for the diagonal fit).

**When.** Online predictive-coding update. Friston's brain-tick
operation: "given current state, what does the model predict for
next?"

**Request.**
```json
POST /v1/bundles/{name}/brain/predict
{
  "fields": ["x", "y"],
  "state": [5.0, 5.0],
  "lr": 0.1,
  "fit_mode": "isotropic"
}
```

**Response.** `next_state: Vec<f64>`, `step_size: f64`
(Euclidean magnitude of the update).

**Tuning.** `lr` is the only real knob. For iterated calls (your
own outer loop), shrink `lr` over iterations for convergence.

---

### §8 ATTEND — softmax over geodesic distance

**What.** Returns weights `α_i ∝ exp(-‖q - x_i‖² / 2σ²)` over
bundle records. Identical to a normalized Gaussian kernel
(Bishop, *PRML* §6.2). Bandwidth `σ` defaults to `√(fit σ²)`.

**When.** Soft retrieval. Replacement for transformer attention
heads. Distribution of relevance over context records.

**Request.**
```json
POST /v1/bundles/{name}/brain/attend
{
  "fields": ["x", "y"],
  "query": [0.5, 0.5],
  "bandwidth": 0.3,
  "top_k": 10
}
```

`top_k` is optional. With it, response gives only the top-k
indices+weights (= FOCUS). Without it, full attention vector.

**Response.**
```json
{
  "weights": [0.4, 0.3, 0.2, 0.1],
  "indices": [3, 1, 7, 12],
  "bandwidth": 0.3,
  "n_samples": 100
}
```

**Tuning.** Smaller `bandwidth` → sharper attention (focused on
nearest neighbors). Larger → smoother distribution. Default
(auto from fit) is usually a good first pass.

---

### §9 FOCUS — top-k attended sub-bundle

**Reach via** `/brain/attend` with `top_k` set (no dedicated
endpoint). Returns the same shape but trimmed.

**When.** Hard attention. "Give me the 5 most relevant records
for this query."

---

### §10 EPISODIC — change-point detection on a value series

**What.** Sorts the values, finds the largest gaps, reports any
gap whose `persistence_ratio = gap / median(gap)` exceeds the
threshold. Persistent-H₀ on the 1-D Vietoris-Rips complex.

**When.** Topic shifts in conversations. Regime changes in
transaction streams. Phase transitions in PK time courses.
Anything where "what's an event vs noise?" matters.

**Request.**
```json
POST /v1/bundles/{name}/brain/episodic
{
  "field": "timestamp_or_value",
  "min_persistence_ratio": 50.0,
  "where_field": "user_id_fiber",
  "where_value": 42,
  "gap_floor_epsilon": 0.000001
}
```

`where_field`/`where_value` are optional — pre-filter records
before running event detection. Both must be supplied together
(only fiber fields supported; base fields require app-layer
filtering on your end).

**Response.**
```json
{
  "events": [
    {"boundary_idx": 27, "gap": 234000.0, "persistence_ratio": 999500.0}
  ],
  "n_records": 50,
  "threshold_used": 50.0,
  "filter_applied": {"field": "user_id_fiber", "value": 42},
  "gap_floor_epsilon_used": 1e-6
}
```

**Tuning.** `min_persistence_ratio: 50` is a sensible default
(below this, gaps look like noise; above, they're real events).
`gap_floor_epsilon: 1e-6` (default) caps reported ratios near 1e6
to prevent overflow on clustered-input data — see §"Defensive
floors" below.

---

### §11 SEMANTIC — Morse-compressed gist

**What.** Returns the bundle's Morse-compressed Hodge complex.
Critical-cell counts equal Betti numbers; topology is preserved
while redundant cells are stripped.

**When.** Long-term semantic memory. "What's the bundle's
invariant structure, with the noise compressed out?" Useful for
the L6 sleep-cycle pattern: periodically re-compute to see how
topology evolves.

**Request.** `GET /v1/bundles/{name}/brain/semantic` (no body).

**Response.**
```json
{
  "betti_b0": 1,
  "betti_b1": 14,
  "betti_b2": 119,
  "n_critical": 134,
  "n_original": 332,
  "compression_ratio": 2.48,
  "cohomology_preserved": true
}
```

**Tuning.** None — it's a function of the bundle's structure.
404 if the bundle is too small or degenerate for Morse compression.

---

### §12 SELF-MONITOR — confidence as Bayesian precision

**What.** Returns `Σᵢ exp(-‖q - xᵢ‖² / 2σ²)` (raw kernel sum) and
the same normalized by the densest sample. High value → query
near data; near-zero → query far from any training record.

**When.** "I don't know" gate before generation. Out-of-distribution
detection. Refuse-to-answer threshold. Catches the case where
DREAM or RECONSTRUCT would otherwise produce confident garbage.

**Request.**
```json
POST /v1/bundles/{name}/brain/confidence
{
  "fields": ["x", "y"],
  "query": [0.5, 0.5],
  "bandwidth": 1.0
}
```

**Response.**
```json
{
  "raw": 5.91,
  "normalized": 0.66,
  "bandwidth": 1.0,
  "n_samples": 100
}
```

**Tuning.** Most consumers gate on `normalized > 0.01` (refuse
when query is < 1% as well-supported as the densest sample).
The PK-cohort demo and Marcella's bge probe both show that
the in-distribution / out-of-distribution gap is typically
**100+ orders of magnitude** — so the threshold isn't sensitive.

---

### §13 EXPLAIN — interpolation path to nearest known record

**What.** Finds the nearest sample to the query, returns the
linear-interpolation path between them. The "bridge between
novelty and memory."

**When.** Visualizing out-of-distribution queries against
training data. Explaining why a model output is what it is by
showing the trace from query → closest training example. Useful
debugging tool for SELF-MONITOR refusals.

**Request.**
```json
POST /v1/bundles/{name}/brain/explain
{
  "fields": ["x", "y"],
  "query": [15.0, 15.0],
  "n_steps": 10
}
```

**Response.**
```json
{
  "query": [15.0, 15.0],
  "nearest_record": [0.31, -0.31],
  "nearest_index": 13,
  "nearest_distance": 21.5,
  "path": [[15.0, 15.0], [13.5, 13.5], ..., [0.31, -0.31]],
  "n_steps": 10,
  "n_samples": 50
}
```

`path` has `n_steps + 1` points: first is `query`, last is
`nearest_record`.

**Tuning.** None other than `n_steps`. Interpolation is Euclidean
linear (Mahalanobis-rescaled paths would be a future enhancement).

---

## Fit modes (when to use isotropic vs diagonal)

The flow-based primitives (SAMPLE, FORECAST, DREAM, RECONSTRUCT,
INPAINT, PREDICT) all accept `fit_mode: "isotropic" | "diagonal"`.

| | `isotropic` (default) | `diagonal` |
|---|---|---|
| Fit | single scalar `σ²` (mean of per-field variances) | per-axis `σ²_i` (each field's variance) |
| Best for | data where every axis has comparable scale | anisotropic data — token embeddings, sensor multi-modal, learned features |
| Cost | minimal | minimal |
| Pitfall | smears anisotropy; spread ratios understated on wide axes, overstated on narrow ones | numerical instability if some axes are rank-deficient (mitigated by L13.6 floor) |
| Default for | quick prototyping, well-conditioned data | Marcella token bundles, BGE-style embeddings, any learned manifold |

**Rule of thumb**: if you're not sure, try both. The response
echoes `fit_mode_used` and (for diagonal) `fit_sigma_sq_per_field_raw`
+ `fit_floored_indices` so you can introspect.

---

## Defensive floors (L13.6 + L13.7)

Two numerical-safety knobs prevent the same `divide by ~0`
pathology in different parts of the math:

### σ² floor (diagonal fit)

**Knob:** `sigma_floor_epsilon` (default `1e-3`)
**Where:** SAMPLE / FORECAST / DREAM / RECONSTRUCT / INPAINT /
PREDICT with `fit_mode: "diagonal"`.
**Formula:** `σ²_eff = max(σ², ε × median(σ²), 3 × 0.01, 1e-12)`
**Why:** rank-deficient axes (σ² ≈ 0) would make the
natural-gradient `(x − μ) / σ²` explode. The floor caps gradient
magnitude; stability is guaranteed at default `dt = 0.01`.
**Surfaces:** `fit_sigma_floor_used`, `fit_floored_indices` in
response. Inspect these to see if your data has rank-deficient
dims.
**Pass `0` to disable** the relative ε floor (absolute stability
floor remains).

### Gap-denominator floor (EPISODIC)

**Knob:** `gap_floor_epsilon` (default `1e-6`)
**Where:** EPISODIC.
**Formula:** `denom = max(median(gap), ε × max_gap, 1e-300)`
**Why:** clustered-input data (batch-ingested timestamps) makes
`median(gap) → 0`, and `persistence_ratio = gap / median` would
overflow. The floor caps reported ratios near `1/ε = 1e6` —
preserves event-vs-noise ordering but stays a finite number.
**Surfaces:** `gap_floor_epsilon_used` in response.
**Pass `0` to disable** — only safe on well-spaced data.

---

## Failure modes (what to expect when things go wrong)

| Symptom | Cause | Fix |
|---|---|---|
| `"field 'foo' is a base_field"` | You passed a base_field name to a brain endpoint | Brain endpoints operate on fiber dimensions; query base-keyed records via the regular endpoints |
| `"dimension must be ≥ 2 and even"` | Flow endpoints need even fiber dim for symplectic 2-form | Use 2, 4, 6… fiber fields (or pad with a constant field) |
| Sample values blow up to 1e96+ | Rank-deficient axis with diagonal fit and no floor | Default floor (`sigma_floor_epsilon: 1e-3`) prevents this; if you explicitly disabled it, re-enable |
| `persistence_ratio` overflows to 1e288+ | Clustered timestamps with `gap_floor_epsilon: 0` | Default `1e-6` prevents this; re-enable |
| `"Bundle '...' is not heap-resident"` | Bundle only on mmap, brain endpoints need heap | Brain primitives all require heap; touch a record to bring it into heap or wait for next checkpoint |
| Empty `events` from EPISODIC on data with obvious change-points | Threshold `min_persistence_ratio` too high | Default 50× is conservative; try 20× or 10× for shorter sequences |
| ATTEND with all-equal weights | Bandwidth too large relative to inter-sample distance | Try `bandwidth: 0.1` or use the auto-derived default (omit field) |

---

## Tuning quick-reference card

| Primitive | Most-tuned knob | Sensible default | When to change |
|---|---|---|---|
| SAMPLE | `temperature` | 1.0 | High T → DREAM mode |
| FORECAST | `dt` | 0.01 | Smaller for energy-drift-sensitive forecasts |
| DREAM | `temperature` | 4.0 | Higher → more divergence |
| RECONSTRUCT | `n_steps` | 500 | Larger budget for multimodal H |
| INPAINT | `burn_in` | 2000 | Larger for tight conditionals |
| PREDICT | `lr` | 0.1 | Smaller for iterated calls |
| ATTEND | `bandwidth` | auto | Smaller → sharper retrieval |
| EPISODIC | `min_persistence_ratio` | 50.0 | Smaller for short sequences |
| SELF-MONITOR | `bandwidth` | auto | Smaller → tighter gate |
| EXPLAIN | `n_steps` | 10 | More for finer interpolation |

---

## Worked end-to-end examples

- **`examples/brain_tour_demo.rs`** — walks all 12 primitives on a
  single 40-record synthetic bundle. One run, every primitive,
  real numbers, ~80 lines of output. Start here.
- **`examples/dream_demo.rs`** — DREAM vs SAMPLE at different
  temperatures, on an isotropic Gaussian, ASCII scatter
  side-by-side.
- **`examples/dream_anisotropic_demo.rs`** — DREAM with iso vs
  diag fit on the same anisotropic bundle. Visualizes the L13.6
  win.
- **`examples/predictive_coding_demo.rs`** — L11 primitives
  (INPAINT / PREDICT / SELF-MONITOR) on a synthetic MIRADOR PK
  cohort.
- **`examples/attention_memory_demo.rs`** — L12 primitives
  (ATTEND / FOCUS / EPISODIC / SEMANTIC) on token embeddings + a
  transaction stream.
- **`examples/brain_endpoints_smoke.py`** — HTTP smoke test
  against a running gigi-stream (production or local). Hits all
  10 brain endpoints end-to-end with realistic inputs.

---

## Where to file bug reports / feature requests

1. **Math validation** for a primitive that doesn't behave as
   catalog claims: write a test in
   `theory/brain_primitives/validation_tests.py` that fails on
   the claim.
2. **Wire shape** that doesn't deserialize: write a contract
   test in `tests/kahler_brain_endpoints_contract.rs`.
3. **Performance** issue: capture before/after numbers; we can
   profile the Langevin / softmax hot paths.
4. **New primitive** request: write up the math claim + closed-form
   ground truth, add to `theory/brain_primitives/catalog.md` (or
   propose for `theory/post_kahler_directions/`).

---

## Versioning

Brain primitives are versioned via the L-layer scheme:
- L10 = generative_flow (§2–§5)
- L11 = predictive_coding (§6, §7, §12)
- L12 = attention + memory (§8, §9, §10, §11, §13)
- L13 = HTTP endpoints
- L13.3, L13.5–L13.7 = production-bug fixes from cross-team probes

Layer-level no-feature contracts hold: with `cargo --features kahler`
off, the engine is bit-identical to pre-upgrade GIGI. With it on,
the brain primitives are additive — no existing API changed shape.

**Cross-team contract test** at
`tests/kahler_brain_endpoints_contract.rs` (17 tests) is the
authoritative wire-shape definition. Anything not in there is
subject to change without notice; anything in there will be
preserved across feature additions.

---

— Bee Davis + GIGI engine team (Claude pair)
   _Davis Geometric · 2026-05-26_
