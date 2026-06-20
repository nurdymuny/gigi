//! Generic single-flight cache with mutation-counter invalidation.
//!
//! ## History
//!
//! Per the 2026-06-20 hygiene audit (workflow `w2n0fgqkk`), three
//! independent reimplementations of the same correctness-critical
//! single-flight cache pattern lived in `gigi`:
//!
//! - `BundleFlowCache` (~130 LOC) in `src/bin/gigi_stream.rs`
//! - `VectorMatrixCache` (~809 LOC, incl. matrix math) in
//!   `src/vector_cache.rs`
//! - `MorseCache` (~375 LOC) in `src/morse_cache.rs`
//!
//! Each independently reimplemented:
//!
//! 1. `RwLock<HashMap>` backing store with lock-free read on hit.
//! 2. Per-key `Mutex<()>` for single-flight deduplication on cache miss
//!    (prevents thundering-herd recompute when N threads miss the
//!    same key concurrently).
//! 3. Mutation-counter invalidation: each stored entry carries the
//!    bundle's mutation counter at compute time; a lookup whose
//!    `current_counter` argument does not match is treated as a miss.
//! 4. Random eviction (`map.keys().next().cloned()`) at capacity.
//!
//! Marcella's 2026-05-27 design review flagged the duplication as
//! correctness-critical: three copies = three places to make the bug.
//! A fix to one does not propagate.
//!
//! ## Contract preserved bit-for-bit
//!
//! - **Invalidation model.** Per-entry monotonic counter snapshot
//!   supplied by the caller, not owned by the cache. `get(key,
//!   current_counter)` returns `Some(v)` only when the stored counter
//!   equals `current_counter`. The cache holds no clock and no TTL.
//!   This matches the existing `BundleStore::mutation_counter()` use
//!   in all three callers.
//! - **Eviction model.** Random FIFO-on-iteration at capacity: when
//!   `len() == max_entries` at insert time, drop
//!   `map.keys().next().cloned()`. Documented as "v1 deferred LRU";
//!   the discovery flagged any deviation as a hit-rate regression
//!   risk.
//! - **Lock strategy.** `inner: RwLock<HashMap<K, (V, u64)>>` for the
//!   hot path (lock-free reads on hit, brief write lock on
//!   insert/evict). `compute_locks: Mutex<HashMap<K, Arc<Mutex<()>>>>`
//!   for per-key single-flight. Lock acquisition order is fixed:
//!   `compute_locks` (brief) → per-key `Mutex` (held during compute) →
//!   `inner` `RwLock` (brief, on insert). Never hold `inner` across
//!   compute. Never hold `compute_locks` across per-key acquisition.
//!
//! ## Specialization by composition
//!
//! `V` is parametric — no `CacheValue` trait, no compute-method
//! dispatch inside the cache. Per-cache specialization happens at the
//! wrapper layer:
//!
//! - `BundleFlowCache` wraps `SingleFlightCache<CacheKey, CachedFit>`.
//! - `VectorMatrixCache` wraps `SingleFlightCache<VectorCacheKey,
//!   CachedMatrix>` and owns the secondary per-bandwidth max-density
//!   cache as a sibling on its own value type, not inside the generic.
//! - `MorseCache` wraps `SingleFlightCache<MorseCacheKey, CachedMorse>`
//!   where `CachedMorse: Copy`; `V: Clone` is auto-satisfied for
//!   `Copy` types at zero atomic-refcount cost.
//!
//! The biggest correctness win is [`SingleFlightCache::get_or_compute`]:
//! every pre-consolidation caller open-coded the acquire / double-check
//! / compute / insert / release dance. Centralizing it eliminates the
//! per-call-site opportunity to skip the double-check or leak the
//! per-key lock on the error path.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

/// Generic single-flight cache keyed on `K`, storing values of type
/// `V` together with a per-entry monotonic counter.
///
/// See module docs for the full contract and audit history.
///
/// ## Type bounds
///
/// - `K: Clone + Eq + Hash` — keys are cloned on insert into both the
///   main map and (when needed) the per-key lock map.
/// - `V: Clone` — returned values are cloned on hit. For values where
///   the cost matters, wrap in `Arc<T>` (cheap atomic-refcount clone)
///   or use a `Copy` value type (the generic does not introduce any
///   indirection over what the caller chose).
///
/// `Send` / `Sync` are NOT required on the type itself — the lock
/// types provide the synchronization. They will be required at the
/// usage site if the cache is shared across threads (which all three
/// real callers do, via `Arc<SingleFlightCache<…>>`).
pub struct SingleFlightCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// Hot-path store. Read-locked on cache hit, write-locked only
    /// for insert / evict. Stores `(value, counter_at_insert)` so the
    /// invalidation check is fully centralized.
    inner: RwLock<HashMap<K, (V, u64)>>,
    /// Per-key single-flight locks. The outer `Mutex` guards the
    /// lookup map; the inner `Arc<Mutex<()>>` is what concurrent
    /// missers on the same key block on. Held only briefly to look
    /// up or insert the per-key Arc.
    compute_locks: Mutex<HashMap<K, Arc<Mutex<()>>>>,
    /// Capacity cap. Insert past this evicts one arbitrary entry
    /// (FIFO-on-iteration random); see module docs for rationale.
    max_entries: usize,
}

impl<K, V> SingleFlightCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    /// New cache holding up to `max_entries` entries. A capacity of 0
    /// is clamped to 1 (matches the pre-consolidation behavior of all
    /// three migrated caches).
    pub fn new(max_entries: usize) -> Self {
        SingleFlightCache {
            inner: RwLock::new(HashMap::new()),
            compute_locks: Mutex::new(HashMap::new()),
            max_entries: max_entries.max(1),
        }
    }

    /// Hot-path lookup. Returns `Some(v)` only if there is an entry
    /// for `key` AND its stored counter equals `current_counter`.
    /// A counter mismatch is treated as a miss; the stale entry is
    /// left in place and overwritten lazily by the next `insert`.
    pub fn get(&self, key: &K, current_counter: u64) -> Option<V> {
        let map = match self.inner.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let (v, c) = map.get(key)?;
        if *c == current_counter {
            Some(v.clone())
        } else {
            None
        }
    }

    /// Insert a freshly-built value under `key` with the snapshot
    /// counter `counter`. Returns `true` iff an existing entry was
    /// evicted to make room (capacity was full and a different key
    /// was dropped). An overwrite of `key` itself returns `false`.
    pub fn insert(&self, key: K, value: V, counter: u64) -> bool {
        let mut map = match self.inner.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let key_present = map.contains_key(&key);
        let mut evicted = false;
        if !key_present && map.len() >= self.max_entries {
            // Random FIFO-on-iteration eviction. Identical to the
            // pre-consolidation behavior of all three caches. v1
            // deferred LRU per discovery.
            if let Some(k) = map.keys().next().cloned() {
                if k != key {
                    map.remove(&k);
                    evicted = true;
                }
            }
        }
        map.insert(key, (value, counter));
        evicted
    }

    /// Acquire (or create) the per-key single-flight compute lock.
    /// The caller is expected to hold the returned `Arc<Mutex<()>>`
    /// for the duration of the compute work. Other threads that miss
    /// on the same key will block on this lock; on release they
    /// re-check the main cache and (if a value was inserted) skip
    /// the recompute.
    ///
    /// Most callers should prefer [`Self::get_or_compute`], which
    /// centralizes the acquire / double-check / compute / insert /
    /// release dance and is guaranteed to release the lock on the
    /// panic / error paths via RAII.
    pub fn acquire_compute_lock(&self, key: &K) -> Arc<Mutex<()>> {
        let mut locks = match self.compute_locks.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        locks
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Remove the per-key compute lock entry. Idempotent: removing a
    /// key that has no entry is a no-op. Existing `Arc` clones held
    /// by other threads remain valid (this only drops the lookup-map
    /// entry; the lock itself lives as long as anyone holds an Arc).
    pub fn release_compute_lock(&self, key: &K) {
        let mut locks = match self.compute_locks.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        locks.remove(key);
    }

    /// Get the value for `key` at `current_counter`, computing it
    /// under single-flight if absent or stale. Returns the value (a
    /// fresh `clone` of either the cached entry or the newly-
    /// computed one), or propagates the compute closure's error.
    ///
    /// Semantics:
    ///
    /// 1. Lock-free hot read. Cache hit + counter match → return clone.
    /// 2. Cache miss / stale: acquire the per-key compute lock. Other
    ///    threads with the same key block here.
    /// 3. Re-check the main cache under the per-key lock (a thread
    ///    that held it before us may have just inserted). Hit →
    ///    release lock, return.
    /// 4. Run `compute`. On `Ok`, insert and release the per-key
    ///    lock entry. On `Err`, release the per-key lock entry WITHOUT
    ///    poisoning the cache (the next call retries cleanly).
    ///
    /// The per-key lock entry is released by an internal RAII guard,
    /// so a panic inside `compute` does not leak the lock (the panic
    /// propagates after the guard runs).
    pub fn get_or_compute<F, E>(
        &self,
        key: K,
        current_counter: u64,
        compute: F,
    ) -> Result<V, E>
    where
        F: FnOnce() -> Result<V, E>,
    {
        // 1. Hot read.
        if let Some(v) = self.get(&key, current_counter) {
            return Ok(v);
        }

        // 2. Acquire per-key compute lock under RAII so the lookup-map
        //    entry is dropped even on panic in `compute`.
        let lock_arc = self.acquire_compute_lock(&key);
        let _guard = match lock_arc.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let _release_guard = ReleaseLockOnDrop {
            cache: self,
            key: &key,
        };

        // 3. Double-check after acquiring the lock. Another thread
        //    that held this per-key lock before us may have just
        //    inserted while we were waiting.
        if let Some(v) = self.get(&key, current_counter) {
            return Ok(v);
        }

        // 4. True miss: run compute, then insert. On compute Err we
        //    let _release_guard drop, releasing the per-key lock; the
        //    cache stays empty for this key so the next caller retries.
        let value = compute()?;
        self.insert(key.clone(), value.clone(), current_counter);
        Ok(value)
    }

    /// Explicit single-entry invalidation. Returns the prior value if
    /// it existed (regardless of staleness). Useful when the caller
    /// knows a specific key is wrong (e.g. external schema change),
    /// without going through the counter mechanism.
    pub fn invalidate(&self, key: &K) -> Option<V> {
        let mut map = match self.inner.write() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        map.remove(key).map(|(v, _)| v)
    }

    /// Drop all cached entries AND all idle per-key compute lock
    /// entries. Callers actively waiting on per-key locks retain
    /// their `Arc` clones, so in-flight computes complete cleanly;
    /// the next request for those keys allocates fresh entries.
    pub fn clear(&self) {
        {
            let mut map = match self.inner.write() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            map.clear();
        }
        let mut locks = match self.compute_locks.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        locks.clear();
    }

    /// Number of cached entries (regardless of staleness).
    pub fn len(&self) -> usize {
        let map = match self.inner.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        map.len()
    }

    /// Whether the cache holds no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// RAII guard that drops the per-key compute lock entry from the
/// lookup map on scope exit. Ensures panic-safety: a panic inside the
/// `compute` closure of [`SingleFlightCache::get_or_compute`] does not
/// leave the per-key Arc<Mutex<()>> dangling in `compute_locks`
/// forever.
///
/// Note: this only drops the *lookup-map entry*, not the lock itself.
/// Other threads that called `acquire_compute_lock` before us hold
/// independent `Arc` clones; their `Arc<Mutex<()>>` is unaffected
/// (those threads continue to hold a live lock and will release it
/// normally when their own scope ends).
struct ReleaseLockOnDrop<'a, K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    cache: &'a SingleFlightCache<K, V>,
    key: &'a K,
}

impl<K, V> Drop for ReleaseLockOnDrop<'_, K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn drop(&mut self) {
        self.cache.release_compute_lock(self.key);
    }
}

// ── Unit tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    #[test]
    fn new_cache_is_empty() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn insert_then_get_hits_when_counter_matches() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        c.insert("k".into(), 42, 5);
        assert_eq!(c.get(&"k".into(), 5), Some(42));
    }

    #[test]
    fn get_misses_when_counter_advances() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        c.insert("k".into(), 42, 5);
        assert_eq!(c.get(&"k".into(), 6), None);
        assert_eq!(c.get(&"k".into(), 4), None);
        // Stale entry is still present.
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn get_misses_on_unknown_key() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        assert_eq!(c.get(&"missing".into(), 0), None);
    }

    #[test]
    fn insert_at_capacity_evicts_one() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(2);
        assert!(!c.insert("a".into(), 1, 0));
        assert!(!c.insert("b".into(), 2, 0));
        let evicted = c.insert("c".into(), 3, 0);
        assert!(evicted, "third insert at cap 2 must evict");
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn insert_overwrite_same_key_does_not_evict() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(2);
        assert!(!c.insert("a".into(), 1, 0));
        assert!(!c.insert("b".into(), 2, 0));
        // Overwrite is not a new entry — must not evict.
        let evicted = c.insert("a".into(), 99, 1);
        assert!(!evicted, "overwrite is not new insertion");
        assert_eq!(c.len(), 2);
        assert_eq!(c.get(&"a".into(), 1), Some(99));
    }

    #[test]
    fn capacity_zero_is_clamped_to_one() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(0);
        c.insert("a".into(), 1, 0);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn invalidate_drops_one_entry() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        c.insert("a".into(), 1, 0);
        c.insert("b".into(), 2, 0);
        let prior = c.invalidate(&"a".into());
        assert_eq!(prior, Some(1));
        assert_eq!(c.len(), 1);
        assert!(c.get(&"a".into(), 0).is_none());
        assert_eq!(c.get(&"b".into(), 0), Some(2));
    }

    #[test]
    fn clear_drops_all() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        for i in 0..5 {
            c.insert(format!("k{}", i), i as u64, 0);
        }
        assert_eq!(c.len(), 5);
        c.clear();
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn compute_lock_same_key_shares_arc() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        let l1 = c.acquire_compute_lock(&"k".into());
        let l2 = c.acquire_compute_lock(&"k".into());
        assert!(Arc::ptr_eq(&l1, &l2));
    }

    #[test]
    fn compute_lock_distinct_keys_distinct_arcs() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        let l1 = c.acquire_compute_lock(&"a".into());
        let l2 = c.acquire_compute_lock(&"b".into());
        assert!(!Arc::ptr_eq(&l1, &l2));
    }

    #[test]
    fn release_compute_lock_removes_entry() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        let _ = c.acquire_compute_lock(&"k".into());
        c.release_compute_lock(&"k".into());
        let l_new = c.acquire_compute_lock(&"k".into());
        // After release, fresh acquire: one Arc in map, one in local.
        assert_eq!(Arc::strong_count(&l_new), 2);
    }

    #[test]
    fn get_or_compute_hit_skips_compute() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        c.insert("k".into(), 42, 0);
        let calls = AtomicUsize::new(0);
        let v: Result<u64, ()> = c.get_or_compute("k".into(), 0, || {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(99)
        });
        assert_eq!(v, Ok(42));
        assert_eq!(calls.load(Ordering::Relaxed), 0, "hit must skip compute");
    }

    #[test]
    fn get_or_compute_miss_runs_and_inserts() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        let v: Result<u64, ()> = c.get_or_compute("k".into(), 0, || Ok(42));
        assert_eq!(v, Ok(42));
        assert_eq!(c.get(&"k".into(), 0), Some(42));
    }

    #[test]
    fn get_or_compute_error_does_not_poison() {
        let c: SingleFlightCache<String, u64> = SingleFlightCache::new(8);
        let r: Result<u64, &'static str> =
            c.get_or_compute("k".into(), 0, || Err("nope"));
        assert_eq!(r, Err("nope"));
        assert_eq!(c.len(), 0, "failed compute leaves cache empty");
        // Per-key lock entry released too — next acquire is fresh.
        let l = c.acquire_compute_lock(&"k".into());
        assert_eq!(Arc::strong_count(&l), 2);
    }

    #[test]
    fn get_or_compute_single_flight_under_contention() {
        // 16 threads race on the same key. Exactly 1 compute must run.
        let cache: Arc<SingleFlightCache<String, u64>> =
            Arc::new(SingleFlightCache::new(8));
        let count = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..16 {
            let c = Arc::clone(&cache);
            let n = Arc::clone(&count);
            handles.push(thread::spawn(move || {
                c.get_or_compute("hot".to_string(), 1, || -> Result<u64, ()> {
                    n.fetch_add(1, Ordering::Relaxed);
                    thread::sleep(std::time::Duration::from_millis(5));
                    Ok(42)
                })
            }));
        }
        let results: Vec<Result<u64, ()>> =
            handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(
            count.load(Ordering::Relaxed),
            1,
            "single-flight: exactly 1 compute across 16 concurrent misses"
        );
        for r in results {
            assert_eq!(r, Ok(42));
        }
    }

    #[test]
    fn get_or_compute_panic_in_compute_releases_lock() {
        let cache: Arc<SingleFlightCache<String, u64>> =
            Arc::new(SingleFlightCache::new(8));
        let cache2 = Arc::clone(&cache);
        // Spawn a thread that panics in compute, so the panic does not
        // crash the test process. Verify the per-key lock entry is
        // released afterwards (so the next acquire allocates fresh).
        let h = thread::spawn(move || {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _: Result<u64, ()> =
                    cache2.get_or_compute("k".into(), 0, || -> Result<u64, ()> {
                        panic!("compute panicked");
                    });
            }));
        });
        h.join().unwrap();
        // After the panic, the per-key lock map must be clean for "k".
        let l = cache.acquire_compute_lock(&"k".into());
        assert_eq!(
            Arc::strong_count(&l),
            2,
            "after panic, fresh acquire — not a lingering Arc"
        );
    }

    #[test]
    fn concurrent_distinct_keys_do_not_serialize() {
        // Two threads compute distinct keys concurrently. Total wall
        // time must be substantially less than the strict-serial bound
        // (2x compute) — distinct keys must not serialize on each
        // other's per-key locks.
        //
        // Use long enough compute waits that test-host scheduling
        // noise can't bridge the parallel-vs-serial gap. 250ms parallel
        // budget for two 200ms computes leaves 50ms headroom on the
        // parallel branch and 150ms headroom on the serial branch.
        let cache: Arc<SingleFlightCache<String, u64>> =
            Arc::new(SingleFlightCache::new(8));
        let start = std::time::Instant::now();
        let c1 = Arc::clone(&cache);
        let c2 = Arc::clone(&cache);
        let h1 = thread::spawn(move || {
            c1.get_or_compute("a".to_string(), 0, || -> Result<u64, ()> {
                thread::sleep(std::time::Duration::from_millis(200));
                Ok(1)
            })
        });
        let h2 = thread::spawn(move || {
            c2.get_or_compute("b".to_string(), 0, || -> Result<u64, ()> {
                thread::sleep(std::time::Duration::from_millis(200));
                Ok(2)
            })
        });
        h1.join().unwrap().unwrap();
        h2.join().unwrap().unwrap();
        let elapsed = start.elapsed().as_millis();
        // Serial would be ≥400ms. Parallel ≈200ms. 350ms bound is well
        // clear of host noise and decisively excludes serial execution.
        assert!(
            elapsed < 350,
            "distinct keys must compute in parallel; elapsed={}ms",
            elapsed
        );
    }
}
