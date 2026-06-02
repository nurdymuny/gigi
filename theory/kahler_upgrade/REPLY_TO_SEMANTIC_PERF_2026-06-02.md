# Reply to Marcella's SEMANTIC perf letter

**From.** Bee Davis + GIGI engine team (Claude pair).
**To.** Marcella team.
**Date.** 2026-06-02.
**Re.** Your "SEMANTIC performance and the shelf-depth badge" letter
— 10–30s on first call against `marcella_source_embeddings_bge_v2`
(9,964 records, 384-dim), Stacks UI needs < 2s, three proposed
options.

---

## TL;DR

You're right that this is broken. We're shipping the fix today
(3–5 hours, one branch, one merge). Two corrections to your
diagnosis below, then the plan.

**Corrections:**

1. **fly auto-stop is NOT on.** `fly.toml` line 39 says
   `auto_stop_machines = "off"`. Cold-start is not your culprit —
   the 10–30s pays on **every uncached call**, even on a warm
   machine. Your visitors aren't paying cold-start tax; they're
   paying eigendecomposition tax.

2. **The hot path isn't O(n²·D); it's worse — O(V³ + E³ + F³).**
   Per audit: `brain_semantic_endpoint` → `semantic_gist` →
   `morse_compress` → `betti()` (in `src/discrete/hodge_laplacian.rs`
   line 61). `betti()` builds three dense Laplacians (Δ₀, Δ₁, Δ₂)
   and calls `nalgebra::SymmetricEigen::new()` on each — cubic in
   the cell count. On a 10k-vertex bundle, the V×V dense eigendecomp
   alone is several seconds. Your 10–30s is the honest cost.

**Options 1 and 2 collapse into one** when you do them right. The
correct architecture is a lazy cache keyed by `mutation_counter`:
- First read after an insert: pays the eigendecomp (the "precompute"
  you wanted in Option 1 — except triggered by the first reader,
  not by the writer).
- Every subsequent read: O(1) hash lookup.
- Insert: O(1) — just bumps `mutation_counter`. **Writes don't
  block on Morse recompute.**

This is strictly better than precompute-on-insert because writes
stay fast, and bundles that nobody asks `semantic` of never pay.
For `stacks_works` (60 records inserted once, then read on every
page load), it means the GGOG ingest doesn't pay; the *first
visitor* pays once; every visitor after is instant.

This is also the pattern we already use three times in this
codebase. Lifting it is mechanical:

- **`vector_cache.rs`** (`VectorMatrixCache`) — used by intent_gate
  / confidence / confidence_with_explain. RwLock<HashMap> +
  per-key Arc<Mutex<()>> single-flight + `counter_at_build` check
  on hit. Battle-tested since 2026-05-29.
- **`spectral_gap_cached`** on BundleStore — mutex around an
  Option<SpectralGapSnapshot>, cleared on insert.
- **`BundleFlowCache`** in gigi_stream.rs — same pattern as
  vector_cache, used by SUDOKU / generative-flow fits.

`BundleStore::mutation_counter()` is the existing invalidation
key (file `src/bundle.rs` line 4373, incremented atomically on
every insert / update / delete via `fetch_add(1, Release)`).

---

## Plan

### Today (3–5 hours): SEMANTIC cache

Lift the `vector_cache` pattern to SEMANTIC. Concretely:

1. Add `MorseCache` struct mirroring `VectorMatrixCache` — keyed
   on bundle name, value is the cached `(BettiNumbers, n_critical,
   n_original, compression_ratio, cohomology_preserved)` tuple plus
   the mutation_counter at compute time.
2. Cache the result one layer DEEPER than the HTTP endpoint — at
   the `semantic_gist` or `morse_compress` call site — so the two
   demo binaries (`brain_tour_demo`, `attention_memory_demo`)
   benefit too, not just the HTTP layer.
3. Per-key single-flight (`Arc<Mutex<()>>`) so two concurrent
   first-callers don't both eigendecompose. The second one waits;
   the first publishes; both get the result.
4. Add `MorseCache` capacity tuning via `GIGI_MORSE_CACHE_SIZE`
   env (default 64), matching `GIGI_VECTOR_CACHE_SIZE`'s pattern.

**Test gate (real data):** smoke test runs `GET /brain/semantic`
on `marcella_source_embeddings_bge_v2` twice in succession,
asserts (a) second call returns identical struct to the first,
(b) second call is < 50ms (it'll actually be sub-millisecond — a
hashmap lookup), (c) inserting a record between the two calls
invalidates and the next read recomputes. This is a fail-on-
regression contract, not just a wall-clock benchmark.

**What it doesn't fix:** the FIRST visitor after each insert still
pays 10–30s on first call. That's an algorithmic problem (dense
eigendecomp on a 10k×10k matrix is genuinely slow), not a caching
problem. Two follow-ups below address it if you want them.

### Next sprint: `/brain/cluster`

Confirmed shape, lifted from your original letter:

```json
POST /v1/bundles/{name}/brain/cluster
→ {
    cluster_count: int,
    cluster_labels: [int, ...],          // per-record
    cluster_centroids: [[float, ...], ...],  // per cluster, fiber-dim
    density: float,
    intrinsic_dim: float
  }
```

This is stable; wire against it. HDBSCAN (parameter-free) is the
default backend, with `?method=kmeans` (elbow auto-k) and
`?method=spectral` (uses `spectral_gap` for the count) as opt-ins.
Cached on the same MorseCache key set so repeated reads on the
same bundle hit O(1).

**Estimated cost (revised down from prior letter):** ~1 day end-
to-end including the cache wiring (the cache infrastructure lands
with SEMANTIC today, so cluster just adds another value type to
it). Will ship after SEMANTIC cache is in production and you've
confirmed the badge renders fast enough that we have time to do
cluster honestly.

### Future, if needed: algorithmic improvement to first-call latency

The cache makes second+ calls fast. The first call after each
ingest is still dense eigendecomposition. Two paths to fix that
if it ever matters:

- **Sparse eigendecomposition via Lanczos / ARPACK.** Drops the
  per-Laplacian cost from O(V³) to O(V·k²) where k = number of
  Betti generators we actually need (typically small). Honest
  estimate: ~2 days. Genuine algorithmic win.
- **Witness complex / sparse Vietoris–Rips.** Instead of the full
  simplicial complex on every record, sample landmark points
  (size √n or log n) and build the complex on those. Drops V from
  10k to ~100. Honest estimate: ~3 days. Loses some topological
  fidelity in exchange.

Don't ship either of these on speculation. Bring it up if, after
the cache is in production for a month, you have a real "the first
visitor after our nightly ingest gets a 10s wait" complaint that
matters.

---

## What we do NOT need to do

- **Option 1 as proposed (precompute-on-insert) is the wrong
  invariant.** It makes writes slow to make reads fast. For
  read-heavy bundles like `stacks_works` it's equivalent in
  practice to a cache, but for any bundle with active writes it
  blocks ingest on a 10-second eigendecomp. The mutation-counter
  cache is strictly better.
- **`Cache-Control: max-age=3600` HTTP header is wrong here.** The
  invalidation event is "a record was inserted/deleted from this
  bundle," not "an hour has passed." A wall-clock TTL would either
  serve stale data after writes or recompute too often on
  read-only bundles. The mutation_counter invalidation is exactly
  right because it's tied to the actual change event.
- **No schema migration. No format change. No GIGI side change to
  the request/response shape — your `shelf_depth()` wrapper keeps
  working.**

## What we will ship on confirm

A single branch with:
- `MorseCache` struct (new file `src/morse_cache.rs`, ~300 LOC,
  copy of `vector_cache.rs` adapted to semantic outputs)
- Wire it into `morse_compress()` and `semantic_gist` so all
  callers benefit
- Real-data smoke test on `marcella_source_embeddings_bge_v2`
  proving the second-call latency contract
- Unit tests on the cache lifecycle: miss → hit → mutation →
  invalidate → recompute → hit again
- README ship note + an entry in `BRAIN_PRIMITIVES_CONSUMER_GUIDE.md`
  saying "SEMANTIC is now O(1) on repeat reads; first read after
  insert recomputes lazily."

Gates we'll hold to before merge:
- 1082+ lib tests still pass with `kahler`
- 841+ no-feature regression
- 1252+ integration tests across all `tests/` files
- The new smoke test against the actual sensor / marcella bundle

Same as the CG-verbs branch pattern. Same as everything we've
shipped on this branch over the past week.

---

## What we need from you

Just a yes. Then we ship the cache today, the cluster endpoint
when bandwidth permits, and the algorithmic improvements only if
real production data proves they're needed.

The 10–30s is unacceptable; the fix is mechanical; the pattern
already exists three times in this codebase. There's no reason to
sit on it.

On the same geodesic,
— the GIGI / Brain team (Bee + Claude)

---

**P.S.** The `compression_ratio` proxy you're using for the
shelf-depth badge today is actually a defensible signal in its
own right — a bundle that compresses 95% under Morse really does
have low topological complexity (= shallow shelf), and one that
only compresses 30% has rich topology (= deep shelf). When
cluster lands, surface BOTH: `cluster_count` for "N angles" and
`compression_ratio` for "shelf depth." They measure different
things and they're both useful.
