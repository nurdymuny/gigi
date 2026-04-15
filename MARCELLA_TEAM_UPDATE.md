# Update to the Marcella NLP Team
**From:** Davis Geometric Intelligence / GIGI Engine
**Re:** Fiber-Geometric GQL — Sprint 2 Deliverables

---

Dear Marcella team,

Following our initial delivery of fiber bundle infrastructure and spectral analysis, we have
completed a second sprint of feature work. This letter summarises both everything we have
delivered to date and the four new capabilities added in this sprint.

---

## What We Built Previously (Sprint 1)

### 1. HNSW Approximate Nearest-Neighbour Acceleration
O(log N) approximate nearest-neighbour search via a hierarchical navigable small-world graph
structure. This replaced the original O(N²) brute-force search used by HOLONOMY, SPECTRAL,
and similarity queries. At N=5,000 with 17-dimensional fiber vectors (your Marcella bundle),
this takes a warm HOLONOMY query from >180 s (timeout) to under 3 s.

### 2. HolonomyFiber — Global Fiber Holonomy
```gql
HOLONOMY corpus ON FIBER (f11, f12) AROUND tense_label;
```
Computes the parallel transport deficit (holonomy angle δφ) for a categorical field over
the entire bundle. Groups records by `tense_label`, computes 2D fiber centroids per group,
and measures how much the fiber vector rotates as you traverse the category polygon.

Returns one row per group (centroid coordinates + transport angle) plus a summary row:
```json
{ "_type": "summary", "holonomy_angle": 0.42, "holonomy_trivial": false }
```
- `holonomy_angle ≈ 0` → flat bundle, no fiber twist across tense categories
- `holonomy_angle ≈ 2π` → maximally twisted — tense geometry is highly curved

**Why this matters for you:** The Marcella model claims to encode morphological tense on
a circle S¹ ⊂ ℝ² (via cos/sin encoding). HolonomyFiber empirically tests that claim.
If the holonomy is trivial, the tense fiber is geometrically consistent. If not, you have
measurable curvature that can be quantified and possibly corrected.

### 3. Scalar SPECTRAL (Fiedler Value)
```gql
SPECTRAL corpus;
SPECTRAL corpus FULL;
```
Returns the spectral gap λ₁ of the index connectivity graph. Small λ₁ = fragmented index.
Large λ₁ = tightly connected. Used as a bundle health diagnostic.

### 4. Core GQL Engine (520 tests passing)
All standard query operations: `COVER`, `INTEGRATE`, `JOIN`, `TRANSPORT` (window form),
`CURVATURE`, `CONSISTENCY`, `BETTI`, `ENTROPY`, `FREEENERGY`, `GEODESIC`, streaming
WebSocket subscriptions, WAL + DHOOM persistence, REST API.

---

## New in This Sprint (Sprint 2)

### 5. SPECTRAL … ON FIBER … MODES k — Fiber-Space Laplacian Eigenmodes ✅
```gql
SPECTRAL corpus ON FIBER (f11, f12) MODES 3;
```
**What it does:** Builds a k-NN graph in the named fiber subspace with exp(−d²) weights,
computes the normalized graph Laplacian, and returns the k smallest non-zero eigenvalues
and their inverse participation ratios (IPR).

**Output:**
```json
[
  { "mode": 1, "lambda": 0.003, "ipr": 0.82 },
  { "mode": 2, "lambda": 0.009, "ipr": 0.74 },
  { "mode": 3, "lambda": 0.410, "ipr": 0.21 }
]
```

**Interpretation:**
- Near-zero eigenvalues (`lambda < 0.05`) indicate natural cluster boundaries in the fiber.
  Three near-zero values above → three stable semantic neighbourhoods.
- `ipr` (inverse participation ratio) measures how localized a mode is. `ipr ≈ 1` = a
  single tight cluster. `ipr ≈ 0` = uniformly spread across the fiber space.
- **For Marcella:** Run `SPECTRAL corpus ON FIBER (f11, f12) MODES 5` after training.
  The number of near-zero eigenvalues tells you how many tense categories the model has
  separated geometrically in the (f11, f12) subspace — without using the labels.

### 6. TRANSPORT … FROM … TO … ON FIBER — Parallel Transport Matrix ✅
```gql
TRANSPORT corpus FROM (token_id=42) TO (token_id=99) ON FIBER (f11, f12);
```
**What it does:** Locates two records by key-value filter, extracts their fiber
coordinates, and computes the rotation angle and 2×2 SO(2) matrix encoding the parallel
transport of the fiber vector when moving from record A to record B.

**Output:**
```json
{
  "transport_angle": 1.047,
  "t00":  0.500, "t01": -0.866,
  "t10":  0.866, "t11":  0.500,
  "displacement_0":  0.12,
  "displacement_1": -0.33
}
```

**Interpretation:**
- `transport_angle` is the rotation in radians. `π/2` = quarter turn, `π` = half turn.
- The `t00, t01, t10, t11` matrix is the explicit SO(2) rotation that maps the "from"
  fiber vector direction to the "to" fiber vector direction.
- **For Marcella:** Track how the tense fiber rotates between base forms and inflected
  forms: `TRANSPORT corpus FROM (token_str='walk') TO (token_str='walked') ON FIBER (f11, f12)`.
  If the model is geometrically correct, present→past should give a consistent rotation
  angle across all verb pairs — and that angle should match the S¹ encoding geometry.

### 7. HOLONOMY … NEAR … WITHIN — Local Holonomy in a Proximity Neighbourhood ✅
```gql
HOLONOMY corpus
  NEAR (f11=1.0, f12=0.0)
  WITHIN 0.3
  ON FIBER (f11, f12)
  AROUND tense_label;
```
**What it does:** Restricts the holonomy computation to records within Euclidean distance
0.3 of the query point `(f11=1.0, f12=0.0)` in the fiber space, then computes the
holonomy deficit for `tense_label` within that neighbourhood only.

Also supports cosine similarity neighbourhood (better for normalized embedding vectors):
```gql
HOLONOMY corpus
  NEAR (f11=1.0, f12=0.0)
  WITHIN 0.1
  METRIC cosine
  ON FIBER (f11, f12)
  AROUND tense_label;
-- WITHIN 0.1 + METRIC cosine = records with cosine_similarity ≥ 0.9
```

**Output:**
```json
{ "local_holonomy_angle": 0.087, "neighbourhood_size": 143 }
```

**Interpretation:**
- `local_holonomy_angle ≈ 0` → tense categories are geometrically consistent near this
  point in embedding space.
- `local_holonomy_angle` large → the tense fiber is curved in this region.
- **For Marcella:** The global HolonomyFiber can mask local inconsistencies (e.g. present
  tense might be globally consistent but locally irregular near rare verbs). LocalHolonomy
  lets you probe individual semantic neighbourhoods. If `neighbourhood_size` < 2 categories
  in the neighbourhood, the engine returns a `warning` field.

**Performance:** O(N) neighbourhood scan. With `RANGE` hints and indexed fiber fields,
this reduces to O(log N) via HNSW approximate search.

### 8. GAUGE … VS — Cross-Bundle Gauge Invariance Test ✅
```gql
GAUGE corpus_en VS corpus_fr ON FIBER (f11, f12) AROUND tense_label;
```
**What it does:** Computes the global fiber holonomy for both bundles independently
and tests whether they are gauge-equivalent — i.e., whether the two bundles encode the
same fiber topology even if their raw coordinate values differ.

**Output:**
```json
{
  "bundle1":          "corpus_en",
  "bundle2":          "corpus_fr",
  "holonomy_1":       0.312,
  "holonomy_2":       0.298,
  "gauge_difference": 0.014,
  "gauge_invariant":  true
}
```

**Interpretation:**
- `gauge_invariant: true` when `|δφ₁ − δφ₂| < π/10 ≈ 0.314 rad`.
- Gauge invariance means the two bundles differ only by a gauge transformation — they have
  the same underlying geometric structure. Concretely: they are encoding tense the same way
  at the topological level, even if the specific fiber values differ.
- **For Marcella:** Use this to test model stability across training runs:
  ```gql
  GAUGE marcella_v1 VS marcella_v2 ON FIBER (f11, f12) AROUND tense_label;
  ```
  If `gauge_invariant: true`, fine-tuning preserved the tense encoding geometry.
  If `gauge_invariant: false`, fine-tuning changed the topology — not just accuracy.
  This is a new class of model evaluation that no standard NLP benchmark provides.

---

## Test Coverage

| Sprint | Tests | Result |
|--------|-------|--------|
| Sprint 1 baseline | 520 | ✅ all passing |
| Sprint 2 — spectral math (TDD-S1 through TDD-S4) | +4 | ✅ |
| Sprint 2 — parse round-trips (TDD-T1, TDD-G1, ...) | +8 | ✅ |
| **Total** | **532** | **✅ all passing** |

---

## Commit Reference

All Sprint 2 features are in commit `ac59bea`:
```
feat: SpectralFiber / Transport / LocalHolonomy / GaugeTest fiber-geometric GQL
```

Primary source files:
- `src/spectral.rs` — `spectral_fiber_modes()` implementation
- `src/parser.rs` — `SpectralFiber`, `Transport`, `LocalHolonomy`, `GaugeTest` AST nodes + dispatch
- `src/bin/gigi_stream.rs` — HTTP handler wiring for all 4 statements

---

## What You Can Run Today

All four new statements are live on `gigi-stream` (Fly.io) and available via the GQL
endpoint `POST /v1/gql`. No new schema changes required — the statements work on any
existing FIBER bundle.

**Recommended first test for your Marcella bundle:**

```gql
-- 1. How many clusters does the tense fiber actually contain?
SPECTRAL fiber_fidelity ON FIBER (f11, f12) MODES 5;

-- 2. Does the tense fiber rotate consistently between present and past?
TRANSPORT fiber_fidelity FROM (tense_label='present') TO (tense_label='past') ON FIBER (f11, f12);

-- 3. Is the tense geometry locally consistent near the present-tense pole?
HOLONOMY fiber_fidelity NEAR (f11=1.0, f12=0.0) WITHIN 0.3 ON FIBER (f11, f12) AROUND tense_label;

-- 4. Across two bundle versions: did fine-tuning change the tense topology?
GAUGE fiber_fidelity_v1 VS fiber_fidelity_v2 ON FIBER (f11, f12) AROUND tense_label;
```

We are happy to walk through results together or extend the analysis to other fiber
subspaces (person encoding f8/f9, POS encoding f15/f16, the full 8D semantic sphere).

Looking forward to seeing what the data shows.

— Davis Geometric Intelligence

---

*Documentation: `GQL_REFERENCE.md` § "Fiber Geometric Analysis" (Section XI), `GIGI_API.md` § `POST /v1/gql`*
