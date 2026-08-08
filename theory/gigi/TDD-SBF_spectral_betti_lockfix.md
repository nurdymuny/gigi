# TDD-SBF — SPECTRAL/BETTI: implicit clique operator + lock release

**Status:** SCOPED — not implemented. Every claim below is cited to source as of
commit `8697355`; every gate is written to be falsifiable on the real substrate.

**The measured problem (2026-08-07, 43,472-record trades bundle, indexed on
`symbol` + `side`):** plain `SPECTRAL` and plain `BETTI` each exceeded 180 s,
and while either ran, ingest was hard-blocked and subsequent reads queued —
observed as a whole-engine stall. `CURVATURE` on the same bundle: 1.7 ms.

---

## 1 · Root cause, from source

Three independent defects compound:

**(a) Explicit clique materialization.** The Def 3.9 field-index graph is a
union of cliques — one complete graph per `(field, value)` index bucket —
and `field_index_graph` (`src/spectral.rs:280-304`) materializes every edge
of every clique into `HashMap<BasePoint, HashSet<BasePoint>>` via a double
loop over each bucket. Cost and resident memory are Θ(Σ g²) over bucket
sizes g. On the trades bundle: `side` gives 2 buckets of ~21 k and `symbol`
4 of ~11 k ⇒ ~1.4 × 10⁹ hash-set inserts and a multi-GB adjacency.
`BETTI` (`spectral.rs:538-554`) builds the same structure **just to count
edges** (`adj.values().map(len).sum() / 2`).

**(b) Fixed-cost iteration over the materialized edges.**
`sparse_spectral_gap` (`spectral.rs:343-409`) runs **exactly 300** power
iterations with no convergence exit (`:383`), each one a mat-vec walking
every directed edge with a per-edge `HashMap` lookup (`bp_to_idx.get`,
`:372`). ≈ 300 × Θ(Σ g²) hash-assisted flops.

**(c) Lock held for the whole computation.** The `/v1/gql` read path takes
the engine-wide `RwLock` read guard and computes inline under it
(`src/bin/gigi_stream.rs:13458` acquire → `:13472` execute → guard drops
after serialization; no `spawn_blocking` on this path). Writes take the
write lock (`:13384`), so ingest blocks for the full duration; and both the
Linux futex and Windows SRWLOCK implementations queue **new readers behind a
waiting writer**, so one queued insert converts "concurrent reads allowed"
into "everything waits." The dedicated GET endpoints (`/spectral` `:3884-3901`,
betti stats `:8967-8976`) have the same shape. The sharded endpoint already
demonstrates the cure in this same file: copy under a scoped guard, drop,
compute lock-free (`:3625-3654`).

**The math does not force any of this.** λ₁, β₀, β₁ of a union-of-cliques
are computable without touching a single edge, because the operator has
closed block structure. That is the fix.

---

## 2 · The algorithm — implicit operator by inclusion–exclusion

Let F = the schema's indexed fields, and for a nonempty subset S ⊆ F let the
**S-groups** be the partition of records by their value-*tuple* on S
(records missing any field of S belong to no S-group). For p ≠ q:

```
edge(p,q)  ⇔  ∃ f ∈ F sharing a bucket
1[∃ share] = Σ_{∅≠S⊆F} (−1)^{|S|+1} · 1[p,q in the same S-group]
```

so the 0/1 adjacency (deduplicated exactly as today's `HashSet` gives) is

```
W = Σ_{∅≠S⊆F} (−1)^{|S|+1} · C_S ,      C_S = ⊕_groups (1 1ᵀ − I)
```

Everything the two verbs need falls out in O(N · (2^|F|−1)):

**Degrees.**  `d(p) = Σ_S (−1)^{|S|+1} (|G_S(p)| − 1)`   (0 if p ∉ any S-group)

**Edge count (BETTI).**  `|E| = Σ_S (−1)^{|S|+1} Σ_{G ∈ S-groups} C(|G|,2)`
— no graph, no hashing per edge. β₀ stays `components_from_index` (already
cheap, bitmap union-find, unchanged). β₁ = |E| − |V| + β₀ unchanged.

**Mat-vec (SPECTRAL).** For M = D^(−1/2) W D^(−1/2), with z = D^(−1/2)x:
per subset S, one pass computes each S-group's sum Σ_G z, then
`(C_S z)_p = Σ_{G_S(p)} z − z_p`; accumulate with sign, multiply by
D^(−1/2). One mat-vec = O(N · (2^|F|−1)) adds, zero hash lookups in the
inner loop (group ids precomputed as dense `u32` arrays, one per S).

**Preserved shortcuts.** Disconnected ⇒ 0.0 (`spectral_gap` step 1,
unchanged). Clique check becomes `all d(p) == n−1` — same n/(n−1) branch,
no adjacency needed. Vertex set = records present in ≥ 1 bucket (today's
`adj` keyset — singleton-bucket records included at degree 0), pinned by
gate SBF-1 fixtures.

**Iteration.** Same deflation (u = D^(1/2)·1 normalized), same seed vector
`sin((i+1)·2.654)`, cap stays 300, plus a residual exit
`|μ₂⁽ᵏ⁾ − μ₂⁽ᵏ⁻¹⁾| < 1e−13·max(1,|μ₂|)`. Note: today's output is already
run-to-run nondeterministic in the last ulps (HashMap key order fixes `bps`
ordering), so parity is a tolerance gate, not a bitwise gate.

**One-time build cost:** per S, group-id assignment by tuple hashing —
O(N · 2^|F|) time, O(N · 2^|F|) u32s memory (trades bundle, |F| = 2: three
passes, ~0.6 MB; |F| = 5: 31 passes, ~6 MB at 50 k records). **Guard:**
|F| > 12 falls back to the explicit path with a logged warning (2^|F|
passes stop being a joke; no real schema is near this).

---

## 3 · The lock fix

For `Statement::Spectral`, `Statement::Betti(order=None)` on the GQL read
path and the two GET endpoints:

1. Under the read guard: clone the input snapshot only — `n`, indexed field
   names, and each bucket's `RoaringBitmap` membership (cheap, compressed;
   this is the entire input to §2). Stamp `mutation_counter`.
2. **Drop the guard.**
3. Compute inside `tokio::task::spawn_blocking` (the handler is async; the
   current code computes synchronously on the runtime worker, which is how a
   handful of concurrent slow verbs can starve the whole runtime even
   without the lock).
4. Result is snapshot-consistent: it describes the bundle at acquisition,
   exactly as today's guard-frozen result does — but nothing waits behind it.

## 4 · Cache

`spectral_gap_cache` exists (`src/bundle.rs:734-747`) but is `kahler`-gated,
clear-on-insert, and consulted **only** by GET `/spectral_gap` — the GQL
`SPECTRAL` verb bypasses it entirely (`parser.rs:11354-11356`,
`gigi_stream.rs:14149-14152`). Scope:

- De-gate the cache (it is a `Mutex<Option<snapshot>>`; nothing Kähler in it).
- Re-key it on `mutation_counter` (`bundle.rs:768`, the mechanism built for
  exactly this — `MorseCache` in `src/morse_cache.rs` is the working
  precedent), deleting the clear-on-insert invalidation sites
  (`bundle.rs:1337, 1451`).
- All four consumers read through it: GQL SPECTRAL, GET /spectral,
  GET /spectral_gap, and the HORIZON/DEPTH embedded path (which calls
  `spectral_gap` internally, `parser.rs:11850-11852` — today that is a
  hidden 180 s bomb inside two "cheap" verbs; it inherits both the fast
  operator and the cache automatically).
- BETTI gets a sibling `(β₀, β₁, counter)` memo.

## 5 · Honesty fix riding along

`BundleRef` mmap/Overlay arms silently return `(0,0)` for BETTI and `0.0`
for spectral (`src/mmap_bundle.rs:1696-1701`). A silent zero is a wrong
answer wearing a plausible one's clothes. Scope: explicit
`ExecError("SPECTRAL/BETTI unavailable on mmap-resident bundles")` until the
snapshot path is taught to read overlay indexes (out of scope here).

---

## 6 · Gates (all must pass; none may be weakened silently)

| gate | fixture | assertion |
|---|---|---|
| SBF-1 edge exactness | randomized bundles, N ≤ 400, \|F\| ∈ {1,2,3}, **correlated fields forced** so bucket overlaps exercise every inclusion–exclusion term; plus records missing fields and singleton buckets; 200 seeds | implicit \|E\|, degrees == brute-force O(N²) reference, exact |
| SBF-2 λ₁ parity | same fixtures, connected cases | \|λ₁_implicit − λ₁_explicit\| ≤ 1e−9 |
| SBF-3 shortcut parity | disconnected fixture; perfect-clique fixture | 0.0 and n/(n−1) branches byte-identical |
| SBF-4 β parity | all SBF-1 fixtures + every existing spectral/betti test | (β₀, β₁) exact; existing suite green unchanged |
| SBF-5 scale + memory | N = 50,000, indexed fields of cardinality 4 and 2 (the trades shape) | SPECTRAL < 1 s, BETTI < 100 ms, peak RSS delta < 100 MB; measured numbers recorded in the test output, not just thresholds |
| SBF-6 no-wedge | same bundle; SPECTRAL launched, then concurrently 100 point-queries + 1 batch insert | reads p99 < 50 ms and insert completes < 500 ms **while** SPECTRAL is in flight |
| SBF-7 cache | SPECTRAL ×2, then insert, then SPECTRAL | 2nd call served from counter-matched cache; 3rd recomputes (counter advanced) |
| SBF-8 overlay honesty | mmap-resident bundle | explicit error, never (0,0)/0.0 |

## 7 · Non-goals

- Def 3.9 semantics unchanged — same graph, same numbers, faster and unlocked.
- `SPECTRAL FULL` untouched (dense, hard-capped at V = 4096, errors above).
- The `sharded` k-NN λ₁ endpoint untouched — it is a different estimator on a
  different graph and must keep saying so.
- No opinion here on whether λ₁ of this graph is *informative* on any given
  dataset (on the trades bundle it is dominated by two giant near-cliques);
  this spec makes the verb safe to call, which is a different property.

## 8 · Risks, named

- **Inclusion–exclusion is exact only for the deduplicated 0/1 graph** —
  SBF-1's forced-overlap fixtures exist precisely because the multiplicity
  bug (counting an edge once per shared field) is the natural first
  implementation error.
- **Float-order drift** between explicit and implicit mat-vec bounded by
  SBF-2's tolerance; current HashMap-order nondeterminism means the old
  path never promised better.
- **Snapshot semantics** change "result reflects the world while holding the
  lock" to "result reflects the world at acquisition." Identical observable
  contract for any single caller; concurrent callers now see fresher
  neighbors. Consistent with the mutation-counter cache design
  (`bundle.rs:754-762`).
