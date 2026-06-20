//! Cache for `semantic_gist()` / `morse_compress()` results.
//!
//! Defense-in-depth for the SEMANTIC HTTP endpoint. The 2026-06-02
//! `betti_rank` rewrite (commit `0ec9405`) dropped the per-call cost
//! from O(V³ + E³ + F³) eigendecomposition to sparse F₂ Gaussian
//! elimination — measured 0.54s on the production
//! `marcella_source_embeddings_bge_v2` bundle (down from 60s+
//! timeout). This cache layers on top: subsequent reads on the same
//! bundle skip even that 0.54s, returning the previously-computed
//! Betti tuple in O(1) hashmap-lookup time.
//!
//! Per the perf-letter sequencing call (Bee, 2026-06-02): "MorseCache
//! is still needed. Not abandoned by [the algorithm fix]. The
//! algorithm fix + mutation_counter cache together are the complete
//! solution."
//!
//! ### Pattern
//!
//! Lifted verbatim from
//! [`crate::vector_cache::VectorMatrixCache`] (which itself mirrors
//! `BundleFlowCache` in `gigi_stream.rs`):
//!
//! 1. `RwLock<HashMap<Key, CachedMorse>>` for hot-path lookup.
//! 2. `Mutex<HashMap<Key, Arc<Mutex<()>>>>` for per-key
//!    single-flight on cache miss (concurrent calls for the same
//!    bundle don't race-compute; the second-in-line waits for the
//!    first to publish).
//! 3. Mutation-counter invalidation: every cached entry carries
//!    the `BundleStore::mutation_counter()` value at compute time;
//!    on lookup, a counter mismatch returns `None` and the caller
//!    re-computes lazily.
//!
//! ### Invalidation semantics
//!
//! The right invalidation event for SEMANTIC is "a record was
//! inserted/deleted from this bundle." That's exactly what
//! `BundleStore::mutation_counter` tracks — bumped via
//! `fetch_add(1, Relaxed)` inside `BundleStore::mark_mutated()` on
//! every insert/update/delete. (Relaxed suffices: the counter is a
//! staleness token, needing monotonicity, not happens-before.)
//!
//! Wall-clock TTL would be wrong here: a read-only bundle never
//! changes (so we shouldn't recompute on a wall-clock schedule) and
//! an actively-written bundle could serve stale data between writes
//! if we cached on a TTL.

#![cfg(feature = "kahler")]

use crate::discrete::BettiNumbers;
use std::sync::{Arc, Mutex};

/// Cache key. A bundle is uniquely identified by name within an
/// engine; the per-entry `counter_at_build` (stored in `CachedMorse`)
/// is what handles the same-bundle-different-version case.
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct MorseCacheKey {
    bundle_name: String,
}

impl MorseCacheKey {
    /// Build the cache key for a bundle by name.
    pub fn build(bundle_name: &str) -> Self {
        MorseCacheKey {
            bundle_name: bundle_name.to_string(),
        }
    }
}

/// Cached SEMANTIC result for a bundle at a specific mutation
/// counter. Holds the full [`BrainSemanticResponse`] field-tuple
/// the HTTP endpoint serializes — the cache returns this directly,
/// no recomputation, no MorseComplex materialization.
///
/// [`BrainSemanticResponse`] is defined in `gigi_stream.rs`; the
/// fields here are kept as primitive types so this module stays
/// free of the binary's response struct (the wire layer copies
/// these into the response struct on the hot path).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CachedMorse {
    /// Mutation counter snapshot at compute time. Used for
    /// invalidation: on lookup, mismatch with the bundle's current
    /// counter → cache miss → recompute.
    pub counter_at_build: u64,
    /// Betti numbers — the topological invariants the wire layer
    /// serializes as `betti_b0` / `betti_b1` / `betti_b2`.
    pub betti: BettiNumbers,
    /// `MorseComplex::n_critical()` — count of Morse-compressed
    /// critical cells.
    pub n_critical: usize,
    /// `MorseComplex::n_original()` — pre-compression cell count.
    pub n_original: usize,
    /// `MorseComplex::compression_ratio()` — `n_critical / n_original`.
    pub compression_ratio: f64,
    /// `MorseComplex::cohomology_preserved()` — sanity invariant
    /// (always true by construction, but we cache it so the
    /// response is bit-identical to a freshly-computed one).
    pub cohomology_preserved: bool,
}

impl CachedMorse {
    /// Convenience constructor.
    pub fn new(
        counter_at_build: u64,
        betti: BettiNumbers,
        n_critical: usize,
        n_original: usize,
        compression_ratio: f64,
        cohomology_preserved: bool,
    ) -> Self {
        CachedMorse {
            counter_at_build,
            betti,
            n_critical,
            n_original,
            compression_ratio,
            cohomology_preserved,
        }
    }
}

/// The cache itself. One instance held in `StreamState`, shared via
/// `Arc` across all worker threads.
///
/// Thin wrapper around the generic
/// [`crate::caches::single_flight::SingleFlightCache`] (extracted
/// 2026-06-20, workflow `w2n0fgqkk`). Behavior is byte-identical to
/// the prior hand-rolled implementation. [`CachedMorse`] is `Copy`,
/// so `V: Clone` is auto-satisfied at zero atomic-refcount cost — the
/// hit path returns the value by value, no indirection.
///
/// The `counter_at_build` field on [`CachedMorse`] is retained for
/// backward compatibility but no longer consulted for invalidation;
/// the cache's `(V, u64)` tuple is the source of truth.
///
/// Capacity: bounded by `max_entries` with random eviction on full
/// (same policy as `VectorMatrixCache` and `BundleFlowCache`). Tune
/// via `GIGI_MORSE_CACHE_SIZE` env var (default 64, set in
/// `StreamState::new`).
pub struct MorseCache {
    inner: crate::caches::single_flight::SingleFlightCache<MorseCacheKey, CachedMorse>,
}

impl MorseCache {
    /// New cache holding up to `max_entries` results. Capacity below
    /// 1 is clamped to 1.
    pub fn new(max_entries: usize) -> Self {
        MorseCache {
            inner: crate::caches::single_flight::SingleFlightCache::new(max_entries),
        }
    }

    /// Acquire (or create) the per-key single-flight compute lock.
    pub fn acquire_compute_lock(&self, key: &MorseCacheKey) -> Arc<Mutex<()>> {
        self.inner.acquire_compute_lock(key)
    }

    /// Drop the per-key compute-lock entry.
    pub fn release_compute_lock(&self, key: &MorseCacheKey) {
        self.inner.release_compute_lock(key);
    }

    /// Hot-path lookup. Returns `Some(cached)` only if the entry's
    /// stored counter matches `current_counter`.
    pub fn get(&self, key: &MorseCacheKey, current_counter: u64) -> Option<CachedMorse> {
        self.inner.get(key, current_counter)
    }

    /// Insert a freshly-built CachedMorse. Returns `true` iff an
    /// eviction happened to make room.
    pub fn insert(&self, key: MorseCacheKey, cached: CachedMorse) -> bool {
        let counter = cached.counter_at_build;
        self.inner.insert(key, cached, counter)
    }

    /// Number of cached entries. Diagnostic.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty. Diagnostic.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Drop all cached entries.
    pub fn clear(&self) {
        self.inner.clear();
    }
}

impl Default for MorseCache {
    fn default() -> Self {
        MorseCache::new(64)
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cm(counter: u64, b0: usize, b1: usize, b2: usize) -> CachedMorse {
        CachedMorse::new(
            counter,
            BettiNumbers { b0, b1, b2 },
            100,
            200,
            0.5,
            true,
        )
    }

    #[test]
    fn new_cache_is_empty() {
        let c = MorseCache::new(8);
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn insert_then_get_hits_when_counter_matches() {
        let c = MorseCache::new(8);
        let key = MorseCacheKey::build("test_bundle");
        c.insert(key.clone(), cm(42, 1, 2, 1));
        let got = c.get(&key, 42).expect("hit");
        assert_eq!(got.betti.b0, 1);
        assert_eq!(got.betti.b1, 2);
        assert_eq!(got.betti.b2, 1);
        assert_eq!(got.counter_at_build, 42);
    }

    #[test]
    fn get_misses_when_counter_advances() {
        let c = MorseCache::new(8);
        let key = MorseCacheKey::build("test_bundle");
        c.insert(key.clone(), cm(42, 1, 2, 1));
        // Bundle has been mutated since cache build (counter incremented).
        assert!(c.get(&key, 43).is_none());
        // Cached entry is still THERE — just stale.
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn lookup_on_unknown_key_returns_none() {
        let c = MorseCache::new(8);
        let key = MorseCacheKey::build("nonexistent");
        assert!(c.get(&key, 0).is_none());
    }

    #[test]
    fn insert_at_capacity_evicts() {
        let c = MorseCache::new(2); // tiny cache
        c.insert(MorseCacheKey::build("a"), cm(1, 1, 0, 0));
        c.insert(MorseCacheKey::build("b"), cm(2, 1, 0, 0));
        let evicted = c.insert(MorseCacheKey::build("c"), cm(3, 1, 0, 0));
        assert!(evicted, "third insert at capacity 2 must evict");
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn insert_under_capacity_does_not_evict() {
        let c = MorseCache::new(8);
        let evicted = c.insert(MorseCacheKey::build("a"), cm(1, 1, 0, 0));
        assert!(!evicted);
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn clear_drops_all_entries() {
        let c = MorseCache::new(8);
        c.insert(MorseCacheKey::build("a"), cm(1, 1, 0, 0));
        c.insert(MorseCacheKey::build("b"), cm(2, 1, 0, 0));
        assert_eq!(c.len(), 2);
        c.clear();
        assert_eq!(c.len(), 0);
        assert!(c.is_empty());
    }

    #[test]
    fn insert_with_same_key_overwrites() {
        let c = MorseCache::new(8);
        let key = MorseCacheKey::build("test");
        c.insert(key.clone(), cm(1, 1, 0, 0));
        c.insert(key.clone(), cm(2, 5, 5, 5));
        let got = c.get(&key, 2).expect("hit on new entry");
        assert_eq!(got.betti.b0, 5);
        assert_eq!(got.counter_at_build, 2);
        assert_eq!(c.len(), 1, "overwrite, not extra insert");
    }

    #[test]
    fn compute_lock_returns_same_arc_for_same_key() {
        let c = MorseCache::new(8);
        let key = MorseCacheKey::build("test");
        let lock_a = c.acquire_compute_lock(&key);
        let lock_b = c.acquire_compute_lock(&key);
        // Same key → same Arc<Mutex<()>> (Arc::ptr_eq).
        assert!(
            Arc::ptr_eq(&lock_a, &lock_b),
            "concurrent miss on same key must share the same compute lock"
        );
    }

    #[test]
    fn compute_lock_returns_different_arc_for_different_keys() {
        let c = MorseCache::new(8);
        let lock_a = c.acquire_compute_lock(&MorseCacheKey::build("a"));
        let lock_b = c.acquire_compute_lock(&MorseCacheKey::build("b"));
        assert!(!Arc::ptr_eq(&lock_a, &lock_b));
    }

    #[test]
    fn release_compute_lock_removes_entry() {
        let c = MorseCache::new(8);
        let key = MorseCacheKey::build("test");
        let _lock = c.acquire_compute_lock(&key);
        // After release, the next acquire should produce a fresh Arc.
        c.release_compute_lock(&key);
        let lock_new = c.acquire_compute_lock(&key);
        // Pointer-distinct from any previously held Arc (which we
        // dropped). New Arc has refcount 1. (Hard to assert
        // directly because we can't recover the old Arc; just
        // confirm the lookup returns *something*.)
        assert_eq!(Arc::strong_count(&lock_new), 2 /* one in cache, one here */);
    }

    #[test]
    fn capacity_zero_is_clamped_to_one() {
        let c = MorseCache::new(0);
        c.insert(MorseCacheKey::build("a"), cm(1, 1, 0, 0));
        assert_eq!(c.len(), 1);
    }
}
