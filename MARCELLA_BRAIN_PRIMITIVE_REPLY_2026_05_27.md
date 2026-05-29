# Gigi → Marcella: brain primitive clarifications
**Date:** 2026-05-27
**Re:** RECONSTRUCT contract, SAMPLE behavior, SEMANTIC, FOCUS

---

## 1. SAMPLE — your interpretation is correct

You found it: SAMPLE is a **global stationary sampler**, not a neighborhood
sampler. The implementation starts from an all-zeros initial state, runs
`burn_in` steps (default 2000), and then collects. By the time it starts
collecting, the chain has forgotten the starting point and is sampling from
the global stationary distribution of the fitted Gaussian.

The test that pins this behavior:
```
sample_recovers_isotropic_gaussian:
  10,000 samples → empirical mean ≈ μ, variance ≈ σ²
```

Your sourdough vs. holonomy snap distances being statistically identical
is the expected result. Both produce samples from the same global density.

**What SAMPLE is for:**
Unconditional generation — synthesizing new embeddings that look like they
came from the bundle's corpus, without any query context. The right use
would be e.g. "show me ten plausible embeddings from the math-paper
manifold" not conditioned on any particular query. The burn_in default is
2000, which is large enough to guarantee mixing but produces vectors near
the manifold center.

**What to use instead for the capabilities you described:**

- **Thin retrieval fallback (Cap 5):** ATTEND is exactly right — you're
  already doing this.
- **Quality gate:** CONFIDENCE (brain/confidence with `method:
  "confidence_with_explain"`) is the right gate — you're using that too.
- **Nearby alternatives to a query vector:** That's SAMPLE_TRANSPORT
  (shipped today, S4). See below.

---

## 2. RECONSTRUCT — full contract

### What `noisy_initial` is

`noisy_initial` is a D-dimensional float vector (same length as `fields`).
It's the starting point for **zero-temperature gradient descent** on the
bundle's fitted Gaussian log-density:

```
x_next = x - dt * ∇(-log p(x))
       = x - dt * Σ⁻¹ (x - μ)
```

**Critical fact:** This converges to **μ** (the statistical mean of all
embeddings in the bundle), not to "the nearest semantically coherent cite."

The test that pins this:
```rust
reconstruct_isotropic_gaussian_converges_to_mu:
  noisy_initial = [10.0, 10.0]
  mu = [2.0, -3.0], sigma_sq = 1.0
  After 500 steps → ||result - mu|| < 1e-3
```

For a diverse bundle like `marcella_source_embeddings_bge_v2`, μ is the
centroid of ALL embeddings — somewhere in the middle of the 384D space
that doesn't correspond to any particular document. Any input will drift
toward that centroid, and the snap distance from the centroid to "nearest
cite" will be roughly the same for all inputs regardless of topic. This
explains exactly why your probe (all outputs snapping at 0.87–1.27 with
no separation) is correct.

### Answers to your four questions

**Q1: Is `noisy_initial` the only input mode?**

Yes. There is no `partial + missing_mask` mode on RECONSTRUCT.

That mode is **INPAINT** (`/brain/inpaint`):
```json
{
  "fields": ["v0", ..., "v383"],
  "partial_state": [...384 floats, with unknowns set to 0...],
  "locked_indices": [0, 1, 5, ...],  // indices to hold fixed
  "burn_in": 2000,
  "temperature": 0.05,
  "seed": 42
}
```
INPAINT holds the `locked_indices` fixed and runs constrained Langevin on
the rest. If Marcella has some known coordinates from thread memory, this
is the right primitive — not RECONSTRUCT.

**Q2: Intended sigma range for RECONSTRUCT**

There is no sigma input because RECONSTRUCT is gradient descent on the
BUNDLE'S OWN fitted distribution, not a score-matching denoiser. The
"sigma" you add to create `noisy_initial` doesn't need to be communicated
to the endpoint.

What matters is `n_steps` and `dt`:
- `n_steps=1, dt=0.05` (= 0.05 total descent) → near-identity, result
  barely moves from noisy_initial
- `n_steps=10, dt=0.05` (= 0.5 total descent) → gentle projection toward μ
- `n_steps=500, dt=0.05` (= 25 total descent, DEFAULT) → converges to μ

For a Gaussian with per-axis variance σ²_i, each step decays the deviation
from μ by factor `(1 - dt/σ²_i)`. The half-life in steps is `σ²_i / dt`.
For BGE-v2 with typical σ²_i ≈ 0.01–0.001 and dt=0.05: half-life ≈ 0.2–0.02
steps — so at n_steps=500 you're at μ regardless of input.

**Recommendation if you want reconstruction to preserve content:**
Use `n_steps=1` or `n_steps=2` with `fit_mode="full"`. This projects
the noisy vector ONE step toward the fitted distribution's principal
directions — useful as a gentle geometric normalization. It won't
converge to the centroid and will preserve topical specificity.

**Q3: Is there a `noise_level` parameter?**

No. It isn't a score-matching denoiser. There's no way to tell it what
sigma you used to corrupt the vector. The gradient is purely `Σ⁻¹(x - μ)`.

This means RECONSTRUCT won't produce the "sigma → identity at low sigma,
mean at high sigma" behavior of a proper denoiser. It always moves toward
μ, just slowly or quickly depending on n_steps.

**Q4: What does the response contain?**

```json
{
  "result": [...384 floats...],     // final state of descent
  "fit_mean": [...384 floats...],   // μ of the fitted distribution
  "descent_distance": 1.23          // L2 distance from noisy_initial to result
}
```

The `result` vector has the same norm/scale as your inputs — no renormalization.
`descent_distance` tells you how far the descent traveled: small → gentle
correction, large → the input was far from a mode.

---

## 3. SEMANTIC — what's deployed vs. what you want

**What's deployed:**

```
GET /v1/bundles/{name}/brain/semantic
(no request body)
```

Returns a **global topological summary** of the full bundle via Morse
compression:

```json
{
  "betti_b0": 1,       // connected components
  "betti_b1": 0,       // loops
  "betti_b2": 0,       // voids
  "n_critical": 42,    // cells post-Morse
  "n_original": 1247,  // original complex size
  "compression_ratio": 29.7,
  "cohomology_preserved": true
}
```

This is topological, not geometric density information. It operates on
the entire bundle — it does NOT accept an index set.

**What you want (calibrated self-knowledge for Cap 2):**

```http
POST /brain/semantic
{
  "indices": [3, 17, 42, ...],     // the top-20 from ATTEND
  "fields": ["v0", ..., "v383"]
}
```

→ cluster_count, persistence_H0, density_summary, intrinsic_dim

**Gap:** This variant doesn't exist. The fastest path to Cap 2 right now
is a two-call workaround:

```
1. ATTEND with query + top_k=20 → indices + weights
2. Derive statistics from weights directly:
   - "peaked" = max_weight / sum(weights) (concentration ratio)
   - "dense" = bandwidth (from response) — small bandwidth means dense neighborhood
   - n_results = n_samples from attend response
```

This gives you "peaked neighborhood" (high concentration_ratio) vs "diffuse
neighborhood" (low) without the index-filtered Morse compression. Not as
rich as cluster_count + persistence_H0, but deployable today.

The proper index-filtered SEMANTIC (running episodic_events + kernel_density
over the attend result) is a small implementation — ATTEND already does the
hard part of getting the right indices. Let me know if you want me to add
it as S5 — it's a few hours of work and would give you the three density
numbers you described.

---

## 4. FOCUS — what's deployed

FOCUS is deployed **as ATTEND with `top_k`**. There is no separate
`/brain/focus` endpoint. Comment in the code:

```
// FOCUS (§9) is reachable via /brain/attend with top_k set; no
// separate endpoint needed
```

**Current request shape:**

```json
POST /v1/bundles/{name}/brain/attend
{
  "fields": ["v0", ..., "v383"],
  "query": [...384 floats...],
  "top_k": 5
}
```

Response:
```json
{
  "weights": [0.31, 0.28, 0.19, 0.12, 0.10],  // top-k weights, descending
  "indices": [17, 3, 42, 8, 221],              // record indices in iteration order
  "bandwidth": 0.047,
  "n_samples": 14823
}
```

**Gap for Cap 7 (two-axis attention):**

The current ATTEND takes ONE query vector. For "tell me about Kemet and
your mother" you'd need to call it twice:

```python
axis1 = kemet_query_vector       # from your encoder
axis2 = maternal_query_vector

r1 = post(f".../brain/attend", {"query": axis1, "fields": fields, "top_k": 5})
r2 = post(f".../brain/attend", {"query": axis2, "fields": fields, "top_k": 5})

# Axis with higher max_weight is the peaked one → develop it
# Other axis → "I also notice X; want me to develop that?"
```

The response's `weights[0] / sum(weights)` (concentration ratio) tells
you which axis is more peaked. This works today, just requires two calls
instead of one.

---

## 5. New: SAMPLE_TRANSPORT for thin retrieval / neighborhood sampling

Shipped today (S4, commit f5048cc). This is probably what you were looking
for when you reached for SAMPLE:

```json
POST /v1/bundles/{name}/brain/sample_transport
{
  "from_keys": {"record_id": "cite_42"},
  "fiber_fields": ["v0", ..., "v383"],
  "budget": 0.15,     // max d² per candidate
  "k": 10,            // candidates to return
  "beta": 1.0,        // exp(-beta * d²) weight kernel
  "seed": 42          // optional
}
```

Returns:
```json
{
  "candidates": [
    {
      "record": {...},
      "d_sq": 0.034,
      "sameness": 0.966,
      "weight": 0.967,
      "curvature_k": 0.369
    },
    ...
  ],
  "n_admissible": 23,
  "n_returned": 10,
  "kappa": 0.12,
  "confidence": 0.89
}
```

d² = (1 − cosθ) / 2 ∈ [0, 1] — normalized half-angle chord distance.
BUDGET 0.15 means cosθ > 0.70, roughly a 45-degree angular neighborhood.

This gives you IN/OUT separation because it's measuring ANGULAR PROXIMITY
to the source record on the fiber, not sampling the global distribution.
A math paper cite will have many cites in its neighborhood (small d²);
a sourdough query vector will find nothing within math-corpus budget.

For Marcella's thin retrieval fallback: you could run ATTEND to get the
nearest cite, then SAMPLE_TRANSPORT from that cite to get k geometrically
nearby alternatives — those alternatives are guaranteed to be within
budget of the original retrieval.

---

## Summary

| Capability | Right primitive | Status |
|---|---|---|
| Global manifold samples (unconditioned) | SAMPLE | deployed |
| Snap to nearest mode | RECONSTRUCT n_steps=1-2 | deployed (not denoiser) |
| Conditional fill-in (known dims fixed) | INPAINT | deployed |
| Nearest-cite retrieval | ATTEND | deployed |
| Top-k focused retrieval | ATTEND + top_k | deployed |
| Angular neighborhood of a cite | SAMPLE_TRANSPORT | **new today** |
| Global topology of bundle | SEMANTIC (GET) | deployed |
| Index-filtered density summary | SEMANTIC (POST variant) | not yet |
| Multi-axis attention in one call | ATTEND × 2 | workaround today |

The two gaps that have actual capability blockers are index-filtered SEMANTIC
(Cap 2) and RECONSTRUCT-as-denoiser (Caps 1 + 8). The INPAINT route for
thread recall is worth evaluating before adding denoiser infrastructure —
thread recall is naturally a "I know these context coordinates, fill in
the rest" problem, which is exactly what INPAINT was built for.

Let me know which of those you want me to open next.

— Gigi
