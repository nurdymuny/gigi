//! Byte-identity contract for the consolidated `SingleFlightCache`.
//!
//! Per the 2026-06-20 hygiene audit (workflow w2n0fgqkk), three caches
//! converged independently on the same single-flight pattern:
//!
//!   - `BundleFlowCache` in `src/bin/gigi_stream.rs`
//!   - `VectorMatrixCache` in `src/vector_cache.rs`
//!   - `MorseCache` in `src/morse_cache.rs`
//!
//! All three are consolidated onto the generic `SingleFlightCache<K, V>`
//! in `src/caches/single_flight.rs`. This file pins the observable
//! contract: for a fixed sequence of `get` / `insert` / counter-advance
//! operations, the returned sequence of `(HitOrMiss, ValueHash)` must
//! match the pre-consolidation behavior bit-for-bit.
//!
//! Each test exercises one shape of the consolidated API that mirrors
//! one of the three migrated caches. They fail until the new module
//! lands.

use gigi::caches::single_flight::SingleFlightCache;
use std::sync::Arc;

// ── Shape 1: simulates BundleFlowCache ──────────────────────────────
//
// Multi-entry per "bundle" via composite key (the discovery flagged
// this as load-bearing — different fit modes produce mathematically
// different outputs, so they MUST be distinct cache entries).

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct FlowKey {
    bundle: String,
    mode: u8,
    fields_hash: u64,
    eps_bits: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct FlowVal {
    mu: Arc<Vec<f64>>,
    sigma_sq: f64,
}

fn flow_val_hash(v: &FlowVal) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for x in v.mu.iter() {
        x.to_bits().hash(&mut h);
    }
    v.sigma_sq.to_bits().hash(&mut h);
    h.finish()
}

#[test]
fn flow_cache_shape_miss_hit_invalidate_sequence() {
    let cache: SingleFlightCache<FlowKey, FlowVal> = SingleFlightCache::new(10);
    let key = FlowKey {
        bundle: "test".into(),
        mode: 1,
        fields_hash: 0xdead_beef,
        eps_bits: 1e-3_f64.to_bits(),
    };
    let v = FlowVal {
        mu: Arc::new(vec![1.0, 2.0]),
        sigma_sq: 0.5,
    };

    // Fresh cache: miss.
    assert!(cache.get(&key, 0).is_none(), "fresh cache must miss");

    // Insert at counter 5, look up at 5: hit, same value.
    cache.insert(key.clone(), v.clone(), 5);
    let hit = cache.get(&key, 5).expect("hit at same counter");
    assert_eq!(flow_val_hash(&hit), flow_val_hash(&v));
    assert_eq!(hit.sigma_sq, 0.5);

    // Counter mismatch: miss (stale).
    assert!(cache.get(&key, 6).is_none(), "newer counter must miss");
    assert!(cache.get(&key, 4).is_none(), "older counter must miss");

    // Re-insert at counter 6: now hit at 6.
    let v6 = FlowVal {
        mu: Arc::new(vec![3.0, 4.0]),
        sigma_sq: 0.7,
    };
    cache.insert(key.clone(), v6.clone(), 6);
    let hit6 = cache.get(&key, 6).expect("hit at new counter");
    assert_eq!(flow_val_hash(&hit6), flow_val_hash(&v6));
    // Old counter still stale (now-overwritten entry is at 6).
    assert!(cache.get(&key, 5).is_none(), "stale counter after overwrite");
}

#[test]
fn flow_cache_shape_key_disambiguates_mode_and_eps() {
    let cache: SingleFlightCache<FlowKey, FlowVal> = SingleFlightCache::new(10);
    let v = FlowVal {
        mu: Arc::new(vec![0.0]),
        sigma_sq: 1.0,
    };
    let k_iso = FlowKey {
        bundle: "b".into(),
        mode: 0,
        fields_hash: 1,
        eps_bits: 1e-3_f64.to_bits(),
    };
    let k_diag = FlowKey {
        bundle: "b".into(),
        mode: 1,
        fields_hash: 1,
        eps_bits: 1e-3_f64.to_bits(),
    };
    let k_full = FlowKey {
        bundle: "b".into(),
        mode: 2,
        fields_hash: 1,
        eps_bits: 1e-3_f64.to_bits(),
    };
    cache.insert(k_iso.clone(), v.clone(), 0);
    cache.insert(k_diag.clone(), v.clone(), 0);
    cache.insert(k_full.clone(), v.clone(), 0);
    assert_eq!(cache.len(), 3, "three modes → three entries");
    assert!(cache.get(&k_iso, 0).is_some());
    assert!(cache.get(&k_diag, 0).is_some());
    assert!(cache.get(&k_full, 0).is_some());
}

#[test]
fn flow_cache_shape_evicts_at_capacity_fifo_on_iteration() {
    // Same FIFO-on-iteration random eviction as the pre-migration
    // BundleFlowCache: at capacity, drop `map.keys().next().cloned()`.
    let cache: SingleFlightCache<FlowKey, FlowVal> = SingleFlightCache::new(3);
    let v = FlowVal {
        mu: Arc::new(vec![0.0]),
        sigma_sq: 1.0,
    };
    for i in 0..3 {
        let key = FlowKey {
            bundle: format!("b{}", i),
            mode: 0,
            fields_hash: 0,
            eps_bits: 0,
        };
        let evicted = cache.insert(key, v.clone(), 0);
        assert!(!evicted, "under capacity, no eviction");
    }
    assert_eq!(cache.len(), 3);
    let key4 = FlowKey {
        bundle: "b4".into(),
        mode: 0,
        fields_hash: 0,
        eps_bits: 0,
    };
    let evicted = cache.insert(key4.clone(), v.clone(), 0);
    assert!(evicted, "at capacity, must evict");
    assert_eq!(cache.len(), 3, "capacity preserved after eviction");
    assert!(cache.get(&key4, 0).is_some(), "new entry present");
}

// ── Shape 2: simulates VectorMatrixCache ────────────────────────────
//
// Value is `Arc<Heavy>` (cheap atomic-refcount clone on every hit).
// Verifies that `V: Clone` doesn't force any deep copy when `V` is `Arc`.

#[derive(Debug, PartialEq)]
struct Heavy {
    data: Vec<f64>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct VecKey {
    bundle: String,
    fields_hash: u64,
}

#[test]
fn vector_cache_shape_arc_clone_is_refcount_only() {
    let cache: SingleFlightCache<VecKey, Arc<Heavy>> = SingleFlightCache::new(8);
    let key = VecKey {
        bundle: "b".into(),
        fields_hash: 42,
    };
    let heavy = Arc::new(Heavy {
        data: vec![1.0, 2.0, 3.0, 4.0],
    });
    cache.insert(key.clone(), Arc::clone(&heavy), 1);

    // Hit returns an Arc pointing at the SAME allocation.
    let hit = cache.get(&key, 1).expect("hit");
    assert!(
        Arc::ptr_eq(&hit, &heavy),
        "Arc<Heavy> clone must share allocation (no deep copy)"
    );
    assert_eq!(hit.data, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn vector_cache_shape_get_or_compute_single_flight() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    let cache: Arc<SingleFlightCache<VecKey, Arc<Heavy>>> =
        Arc::new(SingleFlightCache::new(8));
    let compute_count = Arc::new(AtomicUsize::new(0));
    let key = VecKey {
        bundle: "b".into(),
        fields_hash: 0,
    };

    // 8 threads race to compute the same key. Single-flight contract:
    // exactly ONE compute body runs; the other 7 block on the per-key
    // mutex and find the entry in cache on their re-check.
    let mut handles = Vec::new();
    for _ in 0..8 {
        let cache = Arc::clone(&cache);
        let count = Arc::clone(&compute_count);
        let key = key.clone();
        handles.push(thread::spawn(move || {
            cache
                .get_or_compute(key, 1, || -> Result<Arc<Heavy>, ()> {
                    count.fetch_add(1, Ordering::Relaxed);
                    // Simulate slow compute (~5ms) so other threads
                    // pile up on the per-key lock.
                    thread::sleep(std::time::Duration::from_millis(5));
                    Ok(Arc::new(Heavy {
                        data: vec![1.0, 2.0],
                    }))
                })
                .unwrap()
        }));
    }
    let results: Vec<Arc<Heavy>> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    // Single-flight invariant: at most 1 compute should have run.
    assert_eq!(
        compute_count.load(Ordering::Relaxed),
        1,
        "single-flight: exactly 1 compute across 8 concurrent misses"
    );
    // All 8 results point at the same Arc.
    for r in &results[1..] {
        assert!(
            Arc::ptr_eq(&results[0], r),
            "all single-flight results share the same Arc"
        );
    }
}

// ── Shape 3: simulates MorseCache ───────────────────────────────────
//
// Value is `Copy`. The discovery flagged this as load-bearing:
// `V: Clone` must auto-satisfy for `Copy` types with zero atomic
// overhead. We don't need to assert "no Arc was constructed" (the
// generic doesn't wrap in Arc internally), but we DO verify the
// `Copy` value round-trips equal.

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct MorseKey {
    bundle: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MorseVal {
    b0: usize,
    b1: usize,
    b2: usize,
    n_critical: usize,
    n_original: usize,
    compression_ratio_bits: u64, // f64 bits — exact compare
    cohomology_preserved: bool,
}

#[test]
fn morse_cache_shape_copy_value_round_trips() {
    let cache: SingleFlightCache<MorseKey, MorseVal> = SingleFlightCache::new(8);
    let key = MorseKey {
        bundle: "test".into(),
    };
    let v = MorseVal {
        b0: 1,
        b1: 2,
        b2: 1,
        n_critical: 100,
        n_original: 200,
        compression_ratio_bits: 0.5_f64.to_bits(),
        cohomology_preserved: true,
    };
    cache.insert(key.clone(), v, 42);

    // Hit returns the Copy value by value, fully equal.
    let hit = cache.get(&key, 42).expect("hit");
    assert_eq!(hit, v);

    // Counter mismatch → None (stale), entry remains until overwritten.
    assert!(cache.get(&key, 43).is_none(), "stale counter misses");
    // Successful overwrite at new counter.
    let v2 = MorseVal {
        b0: 9,
        b1: 9,
        b2: 9,
        n_critical: 10,
        n_original: 100,
        compression_ratio_bits: 0.1_f64.to_bits(),
        cohomology_preserved: true,
    };
    cache.insert(key.clone(), v2, 43);
    assert_eq!(cache.get(&key, 43), Some(v2));
}

// ── Cross-shape contract checks ─────────────────────────────────────

#[test]
fn clear_drops_all_entries_across_shapes() {
    let cache: SingleFlightCache<MorseKey, MorseVal> = SingleFlightCache::new(8);
    for i in 0..5 {
        cache.insert(
            MorseKey {
                bundle: format!("b{}", i),
            },
            MorseVal {
                b0: i,
                b1: 0,
                b2: 0,
                n_critical: 1,
                n_original: 2,
                compression_ratio_bits: 0u64,
                cohomology_preserved: true,
            },
            0,
        );
    }
    assert_eq!(cache.len(), 5);
    cache.clear();
    assert_eq!(cache.len(), 0);
    assert!(cache.is_empty());
}

#[test]
fn acquire_compute_lock_per_key_arcs_shared() {
    let cache: SingleFlightCache<MorseKey, MorseVal> = SingleFlightCache::new(8);
    let k1 = MorseKey {
        bundle: "a".into(),
    };
    let k2 = MorseKey {
        bundle: "b".into(),
    };
    let l1 = cache.acquire_compute_lock(&k1);
    let l1b = cache.acquire_compute_lock(&k1);
    let l2 = cache.acquire_compute_lock(&k2);
    assert!(
        Arc::ptr_eq(&l1, &l1b),
        "same key → same per-key compute Arc"
    );
    assert!(
        !Arc::ptr_eq(&l1, &l2),
        "different keys → different per-key compute Arcs"
    );
    cache.release_compute_lock(&k1);
    let l1c = cache.acquire_compute_lock(&k1);
    assert!(
        !Arc::ptr_eq(&l1, &l1c),
        "after release, next acquire allocates a fresh Arc"
    );
}

#[test]
fn capacity_clamped_to_one_when_zero_requested() {
    let cache: SingleFlightCache<MorseKey, MorseVal> = SingleFlightCache::new(0);
    let v = MorseVal {
        b0: 0,
        b1: 0,
        b2: 0,
        n_critical: 0,
        n_original: 1,
        compression_ratio_bits: 0,
        cohomology_preserved: true,
    };
    cache.insert(
        MorseKey {
            bundle: "a".into(),
        },
        v,
        0,
    );
    assert_eq!(cache.len(), 1, "capacity 0 clamps to 1");
}

#[test]
fn get_or_compute_propagates_compute_error() {
    let cache: SingleFlightCache<MorseKey, MorseVal> = SingleFlightCache::new(8);
    let key = MorseKey {
        bundle: "a".into(),
    };
    // Compute returns Err — cache stays empty, next call retries.
    let r: Result<MorseVal, &'static str> =
        cache.get_or_compute(key.clone(), 0, || Err("boom"));
    assert_eq!(r, Err("boom"));
    assert_eq!(cache.len(), 0, "failed compute does not poison the cache");

    // Retry succeeds.
    let v = MorseVal {
        b0: 1,
        b1: 0,
        b2: 0,
        n_critical: 1,
        n_original: 1,
        compression_ratio_bits: 0,
        cohomology_preserved: true,
    };
    let r2: Result<MorseVal, &'static str> = cache.get_or_compute(key.clone(), 0, || Ok(v));
    assert_eq!(r2, Ok(v));
    assert_eq!(cache.get(&key, 0), Some(v));
}
