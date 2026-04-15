# GQL Performance Analysis — Marcella Fiber Fidelity Run
**Date:** April 15, 2026  
**Bundle:** `fiber_fidelity_1776268580`  
**Records:** 5,000 fiber vectors, 17 dimensions, 1 base key (`token_id`)

---

## What ran slow and why

### CURVATURE — fast ✅
**Time:** < 5s  
**Why fast:** Curvature is computed incrementally during each `INSERT` and stored in the
bundle metadata. The `CURVATURE` query just reads a cached scalar. This is effectively O(1).
No graph construction required.

---

### SPECTRAL — timeout at 30s, passed at 180s ✅
**Returned:** λ₁ = 0.8134  
**Why slow:** SPECTRAL computes the Fiedler value (smallest nonzero eigenvalue of the graph
Laplacian) of the *index graph*. The index graph is built from the indexed fields.

Our bundle only has `token_id` as a base key, with no `indexed` declarations on fiber fields.
Without indexed fields, GIGI builds the adjacency by proximity on the numeric fields — which
requires O(n²) pairwise comparisons for n=5,000 records (25M comparisons). Then Lanczos
iteration over the resulting sparse Laplacian.

Additionally: the first run hit Fly.io cold-start latency on top of the computation time.
The warm second run completed in ~120s.

**Contributing factor:** No fiber fields were declared with INDEX or RANGE modifiers.
Without RANGE, the Laplacian edge weights are unnormalized, making convergence slower.

---

### CONSISTENCY — timeout at 30s, passed at 180s ✅
**Returned:** h¹ = 0.0234 (same as K, which is suspicious — may be piggybacking curvature)  
**Why slow:** Čech cohomology H¹ requires building a Čech complex from pairwise overlaps.
The 1-skeleton (edges) is O(n²), the 2-skeleton (triangles) is O(n³) in the worst case,
though implementations prune aggressively.

Same cold-start issue as SPECTRAL. Warm run passed. The `h1` value returning the same
number as `K` suggests the REST endpoint may be reusing the curvature result as the
consistency metric under the hood — worth confirming with the GIGI team whether these are
actually distinct computations.

---

### BETTI — fast ✅
**Returned:** 1.21287e+07  
**Why fast:** β₀ (connected components) is computed via union-find, which is O(n·α(n)) —
effectively linear. The large numeric value is suspicious; BETTI normally returns small
integers. Possibly returning total base-space measure or record count rather than a true
topological invariant. This is worth investigating.

---

### ENTROPY — fast ✅
**Returned:** 8.517 bits  
**Why fast:** Shannon entropy is a single-pass scan over the fiber field distribution. O(n).

---

### HOLONOMY AROUND (f11, f12) — timeout at 30s, timeout at 180s ❌

**Why it fails — two compounding reasons:**

**Reason 1 — No index on f11/f12 (algorithmic):**  
HOLONOMY AROUND (f11, f12) needs to construct the adjacency graph in the (f11, f12)
2-plane, compute the connection 1-form along each edge (how much the fiber "rotates" as
you move between nearby sections), then integrate the 1-form around every 1-cycle. Without
`f11` and `f12` declared as `INDEX` fields, GIGI must brute-force the proximity graph with
O(n²) comparisons. For 5,000 records that is 25 million distance computations in 2D —
which is manageable in principle, but building the full simplicial complex on top of that
and then computing the cycle-basis holonomy is expensive.

**Reason 2 — The distribution has no loops (geometric):**  
Holonomy is a continuous operation on a smooth S¹. It measures how a fiber rotates as you
traverse a CLOSED LOOP in the base space. Our tense distribution is:

| tense_label | f11 value | count |
|-------------|-----------|-------|
| na (→ no morphological tense) | +1.000 | 4,286 |
| present | +1.000 | 307 |
| past | −1.000 | 407 |

87% of records are at f11=+1.0 and 8% at f11=−1.0. There are **no records at intermediate
tense angles** — the fiber is populated at only two antipodal poles of S¹. A discrete
two-point distribution has no 1-cycle. There is no loop to integrate around. GIGI cannot
find a closed path in the (f11, f12) graph that would return non-trivially rotated — so
it's either running an exhaustive search for cycles that don't exist, or constructing a
full Rips complex trying to find them, and timing out either way.

The geometrically correct diagnostic for a discrete bimodal tense distribution is
**SECTIONAL CURVATURE ON (f11, f12)** — which measures how the 2-plane curves even when
there are no loops. Or **RICCI ON f11, f12**, which measures the curvature between the two
field directions.

---

## Summary table

| Query | Time | Complexity | Bottleneck |
|-------|------|------------|------------|
| CURVATURE | ~3s | O(1) — cached | None |
| BETTI | ~5s | O(n·α) — union-find | None |
| ENTROPY | ~5s | O(n) — single pass | None |
| SPECTRAL | ~120s | O(n·k) — Lanczos + cold start | Cold start + no RANGE hints |
| CONSISTENCY | ~120s | O(n²) — Čech complex | Cold start + no RANGE hints |
| HOLONOMY | >180s (fail) | O(n²) + no loops | No INDEX + bimodal distribution |

---

## Schema improvements for the next bundle

To fix performance, declare range bounds and enable indexing on fiber fields:

```gql
BUNDLE fiber_fidelity_v2
  BASE (token_id NUMERIC)
  FIBER (
    -- base 8D semantic sphere: all [-1,1]
    f0  NUMERIC RANGE 2,   -- physical
    f1  NUMERIC RANGE 2,   -- valence
    f2  NUMERIC RANGE 2,   -- arousal
    f3  NUMERIC RANGE 2,   -- social
    f4  NUMERIC RANGE 2,   -- temporal
    f5  NUMERIC RANGE 2,   -- agency
    f6  NUMERIC RANGE 2,   -- animacy
    f7  NUMERIC RANGE 2,   -- specificity

    -- person Z/3Z encoded on S^1: [-1,1]
    f8  NUMERIC RANGE 2,   -- person_cos
    f9  NUMERIC RANGE 2,   -- person_sin

    -- animacy {-1, +1}
    f10 NUMERIC RANGE 2,

    -- tense S^1: [-1,1] — INDEX so HOLONOMY can use it
    f11 NUMERIC RANGE 2 INDEX,   -- tense_cos
    f12 NUMERIC RANGE 2 INDEX,   -- tense_sin

    -- modality Z/3Z on S^1: [-1,1]
    f13 NUMERIC RANGE 2,
    f14 NUMERIC RANGE 2,

    -- POS Z/6Z on S^1: [-1,1] — INDEX for clustering
    f15 NUMERIC RANGE 2 INDEX,
    f16 NUMERIC RANGE 2 INDEX,

    -- metadata: indexed for fast COVER queries
    token_str   CATEGORICAL INDEX,
    pos_label   CATEGORICAL INDEX,
    tense_label CATEGORICAL INDEX,
    is_analytic NUMERIC,
    freq_rank   NUMERIC,
    freq_count  NUMERIC
  );
```

With RANGE 2 on all fiber fields, curvature is normalized correctly:
K = variance / range² = variance / 4 (vs. unnormalized K = variance/variance = 1 always).

With INDEX on f11, f12, f15, f16: HOLONOMY and SPECTRAL can use the index topology instead
of brute-force pairwise.
