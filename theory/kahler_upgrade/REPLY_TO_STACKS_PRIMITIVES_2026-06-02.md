# Reply to Marcella's Stacks primitives ask

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-06-02.
**Re.** Your "Four GIGI primitives Marcella needs for the Stacks
integration" letter — `ACT_TUTOR`, `ACT_WEIGH_READINGS`,
`ACT_LOCATE`, `ACT_RECOMMEND` handler dependencies.

---

## TL;DR

All four endpoints you asked for have been live on
`gigi-stream.fly.dev` since 2026-05-25 (commit `fecfa17`, confirmed
in `REPLY_TO_BRAIN_ENDPOINTS_PROBE_2026-05-25.md`):

- ✅ `POST /v1/bundles/{name}/brain/forecast`
- ✅ `GET  /v1/bundles/{name}/brain/semantic` *(note: GET, not POST)*
- ✅ `POST /v1/bundles/{name}/brain/episodic`
- ✅ `POST /v1/bundles/{name}/brain/reconstruct`

All four are polymorphic (heap or mmap, task #107), feature-gated
behind `kahler`, production-hardened on the v192 deploy that went
out on 2026-05-30.

**The "unwired" status in `BRAIN_LEVERAGE_PLAN.md` means "Marcella's
Python hasn't probed them yet"** — not "GIGI hasn't shipped them."
The doc literally says: *"None of the 7 unwired have been probed
from Marcella's code."* That's a wiring task on your side, not a
build task on ours.

**BUT — your assumed request/response shapes don't match what's
shipped.** Every one of the four endpoints is a *math primitive*
(Hamilton flow / Morse cohomology / persistent-H₀ / MAP denoising).
Your letter assumes *enrichment shapes* (`doc_id`, `content_head`,
`snap_distance`, `topic_label`, `cluster_id`) that live one layer
up. That's the real conversation we need to have.

Below: the actual shipped shapes, the gap with yours, and the
honest options for closing it.

---

## 1. FORECAST — already live, different math than you imagined

**Shipped today** (`gigi_stream.rs` line 7383):

```rust
struct BrainForecastRequest {
    fields: Vec<String>,              // fiber fields to project into
    fit_mode: Option<FitMode>,        // isotropic | diagonal | full
    sigma_floor_epsilon: Option<f64>, // L13.6 floor (default 1e-3)
    initial: Vec<f64>,                // starting fiber-space vector
    n_steps: usize,                   // Hamilton steps (default 1000)
    dt: f64,                          // step size
}

struct BrainForecastResponse {
    trajectory: Vec<Vec<f64>>,        // T = 0 Hamilton flow
    fit_mean: Vec<f64>,
    fit_sigma_sq: f64,
}
```

**Your assumed shape:**
- Input: `query_vector` (= our `initial`), `T` (we have no `T` param
  — Hamilton flow is deterministic by construction, you wanted
  `T=0.0` anyway, so this is already what you get), `bundle_id` (we
  take it from URL path).
- Output: an enriched path with `doc_id` / `content_head` /
  `snap_distance` per step + a `terminal_doc_id`.

**The gap.** The shipped endpoint returns a *trajectory in fiber
space* — a sequence of `Vec<f64>` vectors. It does NOT join back
to records. The `doc_id`/`content_head`/`snap_distance` enrichment
requires a nearest-neighbor lookup per step against `stacks_works`,
which is the kind of thing your Python wrapper already does for
other endpoints (it's the `marcella/fiber_bundle/lookup.py`
pattern).

**Recommended path:** thin Python wrapper on your side:

```python
def forecast_tutor(query_vector, n_steps=4):
    traj = gigi.post("/brain/forecast", {
        "fields": BGE_FIELDS, "initial": query_vector,
        "n_steps": n_steps, "dt": 0.01
    })
    enriched = []
    for step, v in enumerate(traj["trajectory"]):
        doc_id, snap_distance = nearest_in_stacks(v)  # existing lookup
        enriched.append({
            "step": step,
            "doc_id": doc_id,
            "content_head": stacks.passage(doc_id).head(180),
            "snap_distance": snap_distance,
        })
    return {
        "forecast_path": enriched,
        "terminal_doc_id": enriched[-1]["doc_id"],
        "snap_distance": enriched[-1]["snap_distance"],
    }
```

**If you still want a GIGI-side enrichment:** that's a wrapper
endpoint, not a new primitive. We could ship
`POST /v1/bundles/{name}/brain/forecast_with_ids` that does the
join server-side, but it'd only be worth it if you measure that the
round-trip cost of doing the lookup on the client side is the
bottleneck. Premature.

---

## 2. SEMANTIC — already live, totally different primitive

**Shipped today** (line 6645, **GET** not POST):

```rust
// No request body — path param only.
struct BrainSemanticResponse {
    betti_b0: usize, betti_b1: usize, betti_b2: usize,
    n_critical: usize,
    n_original: usize,
    compression_ratio: f64,
    cohomology_preserved: bool,
}
```

**Your assumed shape (POST with body):**
- Input: `indices`, `metrics: [cluster_count, persistence,
  intrinsic_dim, density]`
- Output: `cluster_count`, `cluster_labels`, `cluster_centroids`,
  `intrinsic_dim`, `density`

**The gap is fundamental.** Two different mathematical objects
share the name "semantic":

| Quantity | What it measures | What it's good for |
|---|---|---|
| **GIGI SEMANTIC** (shipped) | Morse-compressed cohomology — Betti numbers of the bundle's Hodge complex | "How topologically connected is this bundle? How much can we compress the cells without losing topology?" |
| **Your SEMANTIC** (requested) | HDBSCAN-style cluster analysis on a vector set | "How many distinct angles does this work split along?" |

You want **clustering**; we shipped **persistent topology**.
They're both legitimate "semantic" primitives on a bundle, but
they answer different questions.

**The honest options:**

1. **(Recommended for the Stacks UI):** ship
   `POST /v1/bundles/{name}/brain/cluster` as a *new* primitive
   that returns `cluster_count` + `cluster_labels` +
   `cluster_centroids`. We'd back it with one of: HDBSCAN if you
   want density-based, k-means with elbow auto-k if you want
   centroid-based, or use the spectral_gap-derived count for a
   parameter-free option. Estimated cost: ~1 day for HDBSCAN,
   ~2 hrs for k-means.

2. **Use the shipped SEMANTIC for the *depth-of-shelf* badge.**
   `compression_ratio` and `cohomology_preserved` already answer
   "how deep / how connected is this shelf?" which is roughly what
   your reading-circle "depth badge" wants. Cluster count is a
   different need than depth.

3. **Combine: ship cluster as #1, keep semantic as it is, use
   both.** Cluster for "N distinct angles," shipped semantic for
   "depth/connectedness." This is probably the right answer for
   the Stacks UI.

Pick one of these — I'd vote #3 — and we'll wire it.

---

## 3. EPISODIC — already live, also a different primitive

**Shipped today** (line 6369):

```rust
struct BrainEpisodicRequest {
    field: String,                          // ONE numeric fiber field
    min_persistence_ratio: f64,             // default 50
    where_field: Option<String>,            // optional filter pair
    where_value: Option<serde_json::Value>,
    gap_floor_epsilon: Option<f64>,         // L13.7 denominator floor
}

struct BrainEpisodicResponse {
    events: Vec<BrainEpisodicEventWire>,    // change-points
    n_records: usize,
    threshold_used: f64,
    filter_applied: Option<EpisodicFilterEcho>,
    gap_floor_epsilon_used: f64,
}
```

**Your assumed shape:**
- Input: `user_id`, `session_id`, `topic_filter`, `n_events`
- Output: events with `timestamp` + `doc_id` + `content_head` +
  `topic` + `turn_count`

**Same fundamental gap.** Two different "episodic":

| Quantity | What it measures | What it's good for |
|---|---|---|
| **GIGI EPISODIC** (shipped) | **Persistent-H₀ change points** on a numeric fiber field's value sequence | "When did the temperature regime change? Where in this session did the topic-distance vector jump?" |
| **Your EPISODIC** (requested) | **Conversational session memory** keyed by `(user_id, session_id, topic)` | "Pick up where I left off — we were mid-Chapter 5." |

The shipped version is a *change-point detector on a single
numeric column*. Yours is a *session-history-resume* over
conversational events. Both reasonable; not the same primitive.

**The honest options:**

1. **Build session-resume on the Marcella side**, where it
   belongs. You already have a `jg_kv` bundle (chat content +
   metadata, AEAD-encrypted) — session history is exactly what
   `jg_kv` is for. The "last topic" + "last doc_id" you want are
   `MAX(updated_at) WHERE user_id = …`; GIGI's existing aggregate
   / query primitives cover that. You don't need a brain
   primitive; you need a `SELECT * FROM jg_kv WHERE user_id = ?
   ORDER BY updated_at DESC LIMIT n` (which is one GQL query).
   **Recommended.**

2. **If you want change-point detection on a per-session
   topic-vector sequence** (so "the topic shifted significantly
   at turn 7"), then shipped EPISODIC is what you want — pass it
   a single numeric column derived from per-turn embeddings (e.g.,
   distance to running mean) and it'll find the jumps. Useful for
   "what was the *real* topic of the session" analysis.

3. **Both — use `jg_kv` for resume, shipped EPISODIC for
   shift-detection.** Probably the right answer.

---

## 4. RECONSTRUCT — already live, different primitive again

**Shipped today** (line 7481):

```rust
struct BrainReconstructRequest {
    fields: Vec<String>,
    fit_mode: Option<FitMode>,
    sigma_floor_epsilon: Option<f64>,
    noisy_initial: Vec<f64>,        // noisy fiber-space vector
    n_steps: usize,                 // descent budget (default 500)
    dt: f64,
}

struct BrainReconstructResponse {
    result: Vec<f64>,               // denoised fiber vector (MAP)
    fit_mean: Vec<f64>,
    descent_distance: f64,          // ‖noisy_initial - result‖
}
```

**Your assumed shape:**
- Input: `partial_record` (a record with some fields missing),
  `fields_to_fill`
- Output: `filled_record` (all fields populated) + `confidence`

**Same shape gap:** the shipped version is **vector-space MAP
denoising** (gradient descent on the substrate density). Yours is
**record-level field imputation** ("fill `content` and
`topic_label`").

**The gap.** The shipped one says "you give me a noisy point in
fiber space, I'll move it to the nearest local mode of p(x)."
Yours says "you give me a record with holes, I'll fill the holes."
These are related but distinct: imputation requires deciding
*which* mode and *how* to convert a fiber vector back into a
record's missing fields. That's the same enrichment problem as
forecast #1 — a nearest-neighbor join + a content fetch.

**Recommended path:** Python wrapper, same shape as FORECAST:

```python
def reconstruct_record(partial_record, bundle_id):
    # 1. Vectorize what you know
    noisy_initial = embed_partial(partial_record)
    # 2. Denoise via GIGI
    result = gigi.post(f"/v1/bundles/{bundle_id}/brain/reconstruct",
                       {"fields": EMBED_FIELDS,
                        "noisy_initial": noisy_initial,
                        "n_steps": 500, "dt": 0.01})
    # 3. Nearest-neighbor in the bundle → which record's the MAP?
    nearest_doc_id, snap_dist = nearest_in_bundle(
        result["result"], bundle_id)
    rec = bundle.fetch(nearest_doc_id)
    # 4. Fill missing fields from rec
    filled = {**partial_record,
              **{f: rec[f] for f in fields_to_fill}}
    # confidence ~ exp(-snap_dist) or 1 / (1 + descent_distance)
    return {"filled_record": filled,
            "confidence": math.exp(-snap_dist)}
```

The `confidence` you want maps cleanly to `descent_distance`
(smaller = more confident the input was already near a mode)
and/or `snap_dist` (smaller = closer to a real record).

---

## What we do NOT need to ship right now

- **No new GIGI primitive for FORECAST / RECONSTRUCT** — the math
  is shipped; you need a 30-line Python enrichment wrapper, same
  pattern as `marcella/fiber_bundle/lookup.py`.
- **No new GIGI primitive for EPISODIC session-resume** — that's
  a `jg_kv` query, not a brain primitive.
- **A new GIGI primitive for SEMANTIC clustering is justified** —
  the shipped SEMANTIC answers a different question. Estimated
  cost above (~1 day HDBSCAN, ~2 hrs k-means).

## What we WILL ship if you say yes

- `POST /v1/bundles/{name}/brain/cluster` returning
  `{cluster_count, cluster_labels, cluster_centroids, density,
  intrinsic_dim}` — the genuine "cluster analysis" primitive.
  We'll back it with HDBSCAN by default (parameter-free) with a
  `method: "kmeans" | "spectral"` opt-in. Backwards-compatible
  with the shipped SEMANTIC (different route, different math).

## Suggested next steps for you

1. **Probe the four shipped endpoints from your Python NOW.** No
   deploy needed on our side. Curl them, see the actual shapes,
   find out which ones genuinely don't fit. The hardest part of
   this letter to write was guessing what you'd find unworkable —
   much easier if you've actually called them.
2. **Reply with a confirm/deny on the SEMANTIC → cluster ship.**
   That's the only one we owe you.
3. **For FORECAST / RECONSTRUCT enrichment, ship the wrapper on
   your side and tell us when the round-trip cost matters.** It
   probably won't.
4. **For EPISODIC session-resume, write the `jg_kv` GQL query.**
   That gets you the "pick up where I left off" act today.

---

**Open-source note (Bee's policy):** we'd rather you call shipped
endpoints than wait on us to build endpoints that already exist.
The 5 primitives the plan calls "wired" plus the 4 in this letter
= all 9 you need for the Stacks integration. Probe first, talk
after. We're here for the genuinely-missing piece (cluster
analysis) — but the rest is plumbing on your side.

On the same geodesic,
— the GIGI / Brain team (Bee + Claude)
