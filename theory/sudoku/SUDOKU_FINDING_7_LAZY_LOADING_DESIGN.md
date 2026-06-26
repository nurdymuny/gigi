# SUDOKU FINDING 7 — Lazy bundle loading + LRU eviction (DESIGN-ONLY)

**Status:** DESIGN-ONLY — implementation deferred. Doc lands; no code change.
**Date:** 2026-06-26
**Author:** Bee Davis
**Bundle context:** Phase 4 SUDOKU shipment, 8-item local cleanup wave.

---

## 1. Problem statement

GIGI currently opens every registered bundle eagerly at engine boot. With 5046
bundles on `gigi-stream.fly.dev` v228, this means:

- 5046 `MmapBundle::open()` calls during replay before the first query lands.
- Each open holds an `mmap` region whose page-cache cost competes with hot
  bundles even when the cold one will never be queried this hour.
- Marcella's working set per hour is approximately **5 bundles of 5046**
  (the `marcella_source_embeddings_bge_v2` family + the current session's
  `claude_substrate_v0` overlay + a `stacks_passages` slice). The other
  ~5041 bundles are paying the open cost for zero query value.

The proposal: **load bundles on first query; LRU-evict after N idle minutes;
pin during transactions and snapshot runs.**

## 2. Why this is DESIGN-ONLY for this workflow

The merge agent (Phase 4B) is shipping a 3-patch wave with a wall-clock budget.
Item 7's surface area is large enough that implementing it inside the
workflow's remaining budget risks tripping a locked gate. The workflow's own
decision rule (item 7's "Decision rule") says: **if wall-clock < 30 min,
design-only.** The merge agent applies this rule.

The honest secondary finding (see §6) is that the **observed lazy benefit may
be smaller than the initial framing suggested**, which lowers the cost/benefit
ratio of shipping under time pressure.

## 3. Algorithm sketch

### 3.1 Single chokepoint: `Engine::resolve_bundle(name)`

Every bundle accessor today (~210 call sites across 5 subsystems —
`engine.rs`, `wal.rs`, `gauge/*`, `imagine/*`, `bin/gigi_stream.rs`) reaches
into `self.bundles: HashMap<String, BundleHandle>` directly. Lazy loading
needs **one** access chokepoint.

```rust
impl Engine {
    fn resolve_bundle(&self, name: &str) -> Result<BundleHandle, EngineError> {
        if let Some(handle) = self.bundles_loaded.read().unwrap().get(name).cloned() {
            self.lru.touch(name);
            return Ok(handle);
        }
        // Not currently loaded — check registry, open on demand, install
        // under write lock, return.
        self.load_bundle_on_demand(name)
    }
}
```

Every site that currently does `self.bundles.get(name)` becomes
`self.resolve_bundle(name)?`. The migration is mechanical but spans ~210
sites. Several have inferred ownership patterns
(`&self.bundles[name].field`) that need a small rework.

### 3.2 LRU eviction policy

```rust
struct LruIndex {
    entries: VecDeque<(String, Instant)>,  // name + last-touch
    by_name: HashMap<String, usize>,        // name -> index into entries
    pinned: HashSet<String>,                 // do not evict
    idle_ttl: Duration,                      // default: 15 min
    max_loaded: usize,                       // default: 64
}
```

- Eviction trigger: every Nth `resolve_bundle` call (e.g. N=128), scan the
  tail of `entries` for any whose `last_touch < now - idle_ttl` and that
  are not pinned. Evict those.
- Also: when `entries.len() > max_loaded`, evict the oldest unpinned entry
  regardless of TTL.
- Eviction = drop the `BundleHandle`, drop the mmap region, remove from
  `bundles_loaded`. The registry entry stays so re-resolution can find it.

### 3.3 Pin policy (transactional safety)

- Pin a bundle on `OP_BUNDLE_INSERT` / `OP_BUNDLE_UPDATE` until the WAL
  group it belongs to is committed.
- Pin every bundle that the snapshot writer is currently iterating.
- Pin every bundle reachable from an in-flight `COVER ... NOT IN ...`
  fold join until the fold completes.

This is the part with the most subtle bugs. Snapshot rotation (ITEM 6) and
WAL group commit are already chokepoints, so the pin handles can live on the
`SnapshotWriter` and `WalGroupCommitGuard` types.

### 3.4 Overlay state caveat

The bundle's *file* can be unloaded. But the bundle's **overlay** (uncommitted
in-memory WAL ops on top of the mmap-mapped base) cannot. So
`load_bundle_on_demand` must always preserve `bundles_overlay[name]` even
when the file mmap was unloaded.

This means lazy loading **does not** reduce overlay-driven RSS. See §6 for
why this matters.

## 4. Public-API surface change

None. Lazy loading is a transparent internal optimization. The only externally
visible effect is:

- First-query latency for a cold bundle increases by approximately one
  `MmapBundle::open()` call (~2-15ms on a warm SSD).
- Snapshot iteration time stays constant (snapshot pins everything anyway).
- Boot time decreases by approximately
  `5046 * average_open_time - average_idle_set_open_time`. Estimated savings:
  ~30-90 seconds at v228 scale.

## 5. Convergence criterion

A lazy-loading implementation lands GREEN iff:

1. All 8 currently locked gates stay GREEN with the `lazy_bundles` feature
   flag OFF (bit-identical boot).
2. All 8 locked gates stay GREEN with `lazy_bundles` feature flag ON.
3. Six new `G-LAZY-*` gates are added and stay GREEN:
   - `G-LAZY-1`: cold-resolve test: register bundle, do not query, assert
     not in `bundles_loaded`. Query, assert in `bundles_loaded`.
   - `G-LAZY-2`: LRU eviction test: load N+1 bundles where N = `max_loaded`;
     assert oldest unpinned was evicted.
   - `G-LAZY-3`: TTL eviction test: load bundle, advance clock past TTL,
     trigger sweep, assert evicted.
   - `G-LAZY-4`: pin during WAL group: open WAL group on bundle X, attempt
     eviction, assert X NOT evicted.
   - `G-LAZY-5`: pin during snapshot: start snapshot iteration, attempt
     eviction on iterated bundle, assert NOT evicted.
   - `G-LAZY-6`: overlay preservation: load bundle, write overlay op,
     evict bundle, re-resolve, assert overlay op visible.

## 6. Honest cost-benefit revisit

The initial framing assumed `MmapBundle::open()` cost was the dominant boot
cost and that mmap regions were the dominant RSS cost. That framing is
suspect after the ITEMs 1/4 investigation:

- **Boot cost** is mostly snapshot iteration (which must iterate all bundles
  regardless of lazy loading — see §3.3 pin policy). The savings on
  `open()` alone are real but smaller than `30-90s` because snapshot still
  walks every entry.
- **RSS cost** appears (per a quick local profile under Marcella's working
  set 2026-06-25) to be dominated by:
  - DHOOM encode buffers in the snapshot writer
    (the ITEM 1 / ITEM 4 fix already attacks this).
  - Overlay `HashMap`s for hot bundles, which lazy loading does not unload.
  - Mmap regions for cold bundles, which lazy loading **does** unload — but
    these were already largely paged out by the OS under memory pressure.

So the realized RSS savings from lazy loading on the production-load profile
is closer to **5-15% RSS reduction** than the originally hoped-for 50-70%.

## 7. Surface cost

- ~210 accessor sites to migrate to `resolve_bundle`.
- 5 subsystems (engine, wal, gauge, imagine, bin/gigi_stream).
- 1 new feature flag (`lazy_bundles`) and 6 new G-LAZY gate tests.
- 1 new pin/unpin protocol that needs to be threaded through `SnapshotWriter`
  and WAL group commit paths.
- 1 new struct (`LruIndex`) with its own concurrency story
  (`Mutex<LruIndex>`).
- Backwards-compat: feature-flag-default-OFF; the OFF path is byte-identical
  to today.

## 8. Recommended sequencing

1. **First:** finish ITEM 4 (sort allocation cap — highest-confidence fix
   for the actual ITEM 1 hang Marcella is tripping on). [DONE — `e19e7d5`]
2. **Then:** ITEMs 2 + 3 + 5 + 6 (this workflow). [DONE through `99de50b`]
3. **Defer:** ITEM 7 to a dedicated post-shipment sprint, AFTER measuring
   per-bundle overlay-size with a new metric (see §9).

## 9. Kill criterion for ITEM 7 sprint

Do not start the ITEM 7 sprint until:

- ITEMs 4 / 5 / 6 are all shipped on production.
- A new `bundle_overlay_size_bytes` metric is exported and a 7-day rolling
  window shows that ≥ 30% of registered bundles have nontrivial
  (>1 MB) overlay state.

If at that point the metric shows overlay state is a small minority of the
RSS budget, ITEM 7 is the wrong tool and the next sprint should be
**overlay flush** (push WAL overlay to mmap base + drop the overlay) rather
than **lazy bundle**.

## 10. Production posture

This document was authored locally. No production-mutating commands were
run. The merge agent applies this doc via `git apply` on local `main` only;
NO `git push`, NO `flyctl deploy`, NO `curl POST` to production.

Gate 1 (`cargo test --no-default-features --lib`) confirmed GREEN at 882/0
before and after this doc landed; no source-code change in the working tree
at the time this doc lands.

## 11. Filed under

- ITEM 7 of the 2026-06-26 SUDOKU 8-item local shipment.
- Companion to:
  - `SUDOKU_FINDING_1_ENCODER_HANG.md` (ITEM 1 — the hang root cause; the
    real bottleneck overlay sizing should be measured against).
  - `SUDOKU_FINDING_3_MMAP_TRIAGE.md` (ITEM 3 — mmap-or-die triage; the
    error-bucket framework that ITEM 7's load-on-demand path must respect).
  - `SUDOKU_FINDING_5_LZ4_BENCHMARK.md` (ITEM 5 — DOC ONLY benchmark; the
    same "measure first, ship if it clears the gate" discipline).

— Bee Davis, 2026-06-26.
