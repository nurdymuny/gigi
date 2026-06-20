//! Cached materialized `(N, D)` matrices for vector-search endpoints.
//!
//! Per Marcella's 2026-05-29 bug report `GIGI_BUG_REPORT_onfields_latency.md`:
//! the brain endpoints (`intent_gate`, `confidence`, `confidence_with_explain`)
//! were calling [`extract_field_samples`] on every request, which allocates a
//! fresh `Vec<Vec<f64>>` of shape `(N, D)` and does `N * D` per-cell value
//! lookups + type validation. At `N = 9_964, D = 384` (bge_v2 substrate) this
//! ran in ~35 s per request on the production fly.io instance.
//!
//! ## Architecture
//!
//! - [`MaterializedMatrix`] holds a contiguous row-major `Vec<f64>` of length
//!   `n * d`, plus precomputed per-row squared L2 norms. Distance queries are
//!   one tight loop over contiguous memory; LLVM autovectorizes the inner
//!   dot-product.
//! - [`VectorMatrixCache`] mirrors the architecture of `BundleFlowCache` in
//!   `gigi_stream.rs`:
//!   - `RwLock<HashMap<Key, Cached>>` for hot-path lookup,
//!   - per-key `Mutex<()>` for single-flight compute-deduplication,
//!   - mutation-counter check on hit (stale entry returns `None`),
//!   - capacity bound with random eviction.
//!
//! The cache is keyed by `(bundle_name, fields_hash)`. A query that requests
//! the same set of fiber fields against the same bundle (and no mutations have
//! occurred since the matrix was built) gets the matrix in `O(1)` atomic
//! refcount + hash lookup. The actual distance computation is then `O(N * D)`
//! flops over a single contiguous slab.
//!
//! ## Expected speedup
//!
//! At `N = 9_964, D = 384`:
//!
//! - Per-call extract + per-call nearest-record loop (pre-fix): ~35 s.
//! - Cached matrix + one BLAS-shaped inner loop (post-fix): well under 200 ms
//!   for the full data plane (network round-trip dominates).
//!
//! No API contract change — response shapes are byte-identical.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};

// ── Key ─────────────────────────────────────────────────────────────

/// Cache key for the materialized matrix.
///
/// `fields_hash` is the hash of the field-name sequence in the order the
/// caller provided it. Different orderings would produce columns in a
/// different order, so they get distinct cache entries.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VectorCacheKey {
    pub bundle_name: String,
    pub fields_hash: u64,
}

impl VectorCacheKey {
    /// Build a cache key from a bundle name and the field list. Field order
    /// is hashed; reorderings get distinct entries.
    pub fn build(bundle_name: &str, fields: &[String]) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for f in fields {
            f.hash(&mut hasher);
        }
        VectorCacheKey {
            bundle_name: bundle_name.to_string(),
            fields_hash: hasher.finish(),
        }
    }
}

// ── Matrix ──────────────────────────────────────────────────────────

/// Materialized `(N, D)` matrix in row-major contiguous memory plus the
/// precomputed per-row squared L2 norm (used to fold the `‖r_i‖²` term out
/// of the per-call hot loop).
///
/// All distances are squared Euclidean computed via the cosine identity
///
/// ```text
/// ‖q − r_i‖² = ‖q‖² + ‖r_i‖² − 2 ⟨q, r_i⟩
/// ```
///
/// where `‖q‖²` is computed once per call and `‖r_i‖²` is precomputed at
/// build time.
#[derive(Debug)]
pub struct MaterializedMatrix {
    /// Flat row-major layout: row `i`, column `j` at index `i * d + j`.
    pub data: Vec<f64>,
    /// Row count (record count).
    pub n: usize,
    /// Column count (field count).
    pub d: usize,
    /// `‖row_i‖²` precomputed for each row, length `n`.
    pub row_norms_sq: Vec<f64>,
}

impl MaterializedMatrix {
    /// Construct from a flat row-major buffer. Precomputes per-row squared
    /// norms; this is the only `O(N * D)` work done at build time beyond the
    /// caller's own copy of values into `data`.
    ///
    /// Panics in debug builds if `data.len() != n * d`.
    pub fn new(data: Vec<f64>, n: usize, d: usize) -> Self {
        debug_assert_eq!(
            data.len(),
            n * d,
            "MaterializedMatrix: data.len()={} but n*d={}",
            data.len(),
            n * d
        );
        let mut row_norms_sq = Vec::with_capacity(n);
        for i in 0..n {
            let row = &data[i * d..(i + 1) * d];
            let s: f64 = row.iter().map(|v| v * v).sum();
            row_norms_sq.push(s);
        }
        MaterializedMatrix {
            data,
            n,
            d,
            row_norms_sq,
        }
    }

    /// Squared Euclidean distance from `query` to every row.
    ///
    /// Returns a `Vec<f64>` of length `n`. Tiny negative values from float
    /// roundoff (when the identity yields a near-zero d²) are clamped to 0.
    pub fn d_sq_to_all(&self, query: &[f64]) -> Vec<f64> {
        debug_assert_eq!(query.len(), self.d);
        let q_norm_sq: f64 = query.iter().map(|x| x * x).sum();
        let mut out = Vec::with_capacity(self.n);
        for i in 0..self.n {
            let row = &self.data[i * self.d..(i + 1) * self.d];
            let dot: f64 = row.iter().zip(query.iter()).map(|(a, b)| a * b).sum();
            let d_sq = (q_norm_sq + self.row_norms_sq[i] - 2.0 * dot).max(0.0);
            out.push(d_sq);
        }
        out
    }

    /// Index + squared-distance of the row nearest to `query`.
    ///
    /// Inlines the per-row dot loop so no intermediate `Vec` is allocated
    /// when the caller only wants the nearest record. Cheaper than
    /// `d_sq_to_all` followed by argmin for the common case.
    ///
    /// Returns `(0, 0.0)` for an empty matrix; callers should check `n > 0`.
    pub fn nearest(&self, query: &[f64]) -> (usize, f64) {
        debug_assert_eq!(query.len(), self.d);
        if self.n == 0 {
            return (0, 0.0);
        }
        let q_norm_sq: f64 = query.iter().map(|x| x * x).sum();
        let mut best_idx = 0_usize;
        let mut best_d_sq = f64::INFINITY;
        for i in 0..self.n {
            let row = &self.data[i * self.d..(i + 1) * self.d];
            let dot: f64 = row.iter().zip(query.iter()).map(|(a, b)| a * b).sum();
            let d_sq = (q_norm_sq + self.row_norms_sq[i] - 2.0 * dot).max(0.0);
            if d_sq < best_d_sq {
                best_d_sq = d_sq;
                best_idx = i;
            }
        }
        (best_idx, best_d_sq)
    }

    /// Approximate in-memory footprint in bytes (matrix data + per-row
    /// norms; ignores Vec overhead). Useful for capacity heuristics.
    pub fn bytes(&self) -> usize {
        (self.n * self.d + self.n) * std::mem::size_of::<f64>()
    }
}

// ── Cache entry ─────────────────────────────────────────────────────

/// A cached materialized matrix plus the bundle mutation counter at build
/// time. The counter is checked on every hit; if it has changed since the
/// build, the entry is treated as stale and dropped.
///
/// `max_density_by_bw` is a lazy per-bandwidth cache for the corpus's max
/// `kernel_density_confidence` value. That quantity is needed by
/// `confidence_normalized` and is `O(N² · D)` to compute — but it's a
/// property of `(matrix, bandwidth)` only, not of the per-call query, so
/// it can be computed once and reused for the entire lifetime of the cached
/// matrix. Without this, every `intent_gate` / `confidence_with_explain`
/// call eats the full `O(N² · D)` cost regardless of `n_fields` (the ~6 s
/// baseline in Marcella's 2026-05-29 latency report).
#[derive(Debug, Clone)]
pub struct CachedMatrix {
    pub counter_at_build: u64,
    pub matrix: Arc<MaterializedMatrix>,
    /// Lazy per-bandwidth `max_density` cache. Key is `bandwidth.to_bits()`
    /// for exact float comparison. Computed once per `(matrix, bandwidth)`
    /// on first request that needs normalization; reused thereafter.
    pub max_density_by_bw: Arc<RwLock<HashMap<u64, f64>>>,
}

impl CachedMatrix {
    /// Construct a fresh cache entry around an `Arc<MaterializedMatrix>`,
    /// with an empty `max_density_by_bw` cache (populated lazily on first
    /// normalization request).
    pub fn new(counter_at_build: u64, matrix: Arc<MaterializedMatrix>) -> Self {
        CachedMatrix {
            counter_at_build,
            matrix,
            max_density_by_bw: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

// ── KDE helpers (cache-aware) ───────────────────────────────────────

/// Raw kernel density at `query`: `Σᵢ exp(−‖q − xᵢ‖² / 2σ²)`.
///
/// Inlines the per-row dot-product to avoid allocating an intermediate
/// `Vec<f64>` of distances. Pure function — no cache access. One O(N · D)
/// tight loop over contiguous memory.
pub fn kde_raw_from_matrix(matrix: &MaterializedMatrix, query: &[f64], bandwidth: f64) -> f64 {
    debug_assert_eq!(query.len(), matrix.d);
    if matrix.n == 0 || bandwidth <= 0.0 {
        return 0.0;
    }
    let two_bw_sq = 2.0 * bandwidth * bandwidth;
    let q_norm_sq: f64 = query.iter().map(|x| x * x).sum();
    let mut sum = 0.0_f64;
    for i in 0..matrix.n {
        let row = &matrix.data[i * matrix.d..(i + 1) * matrix.d];
        let dot: f64 = row.iter().zip(query.iter()).map(|(a, b)| a * b).sum();
        let d_sq = (q_norm_sq + matrix.row_norms_sq[i] - 2.0 * dot).max(0.0);
        sum += (-d_sq / two_bw_sq).exp();
    }
    sum
}

/// Max KDE value across all rows of `matrix` at the given bandwidth.
///
/// Used as the denominator of `confidence_normalized`. Pure recomputation
/// (no caching here); callers that want amortization should use
/// [`max_density_cached`] which reads/writes the per-bandwidth cache on
/// `CachedMatrix`.
///
/// Cost: `O(N² · D)` (one KDE call per row). At N=10 000 and D=384 this
/// is ~4 s once; cached thereafter, it's a hashmap lookup.
pub fn max_density_of_matrix(matrix: &MaterializedMatrix, bandwidth: f64) -> f64 {
    if matrix.n == 0 || bandwidth <= 0.0 {
        return 0.0;
    }
    let two_bw_sq = 2.0 * bandwidth * bandwidth;
    let mut max = 0.0_f64;
    // Outer: each row as the query. Inner: KDE against all rows.
    // Hoist the row pointer arithmetic so the inner loop is tight.
    for i in 0..matrix.n {
        let row_i = &matrix.data[i * matrix.d..(i + 1) * matrix.d];
        let row_i_norm_sq = matrix.row_norms_sq[i];
        let mut sum = 0.0_f64;
        for j in 0..matrix.n {
            let row_j = &matrix.data[j * matrix.d..(j + 1) * matrix.d];
            let dot: f64 = row_j.iter().zip(row_i.iter()).map(|(a, b)| a * b).sum();
            let d_sq = (row_i_norm_sq + matrix.row_norms_sq[j] - 2.0 * dot).max(0.0);
            sum += (-d_sq / two_bw_sq).exp();
        }
        if sum > max {
            max = sum;
        }
    }
    max
}

/// Cached variant of `max_density_of_matrix`. Reads the per-bandwidth
/// entry on `cached.max_density_by_bw`; computes + inserts on miss.
///
/// Hot path: read-lock + hashmap lookup. Cold path: write-lock + double-
/// check + O(N² · D) compute.
pub fn max_density_cached(cached: &CachedMatrix, bandwidth: f64) -> f64 {
    let bw_bits = bandwidth.to_bits();
    // Hot read.
    {
        let map = match cached.max_density_by_bw.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(&v) = map.get(&bw_bits) {
            return v;
        }
    }
    // Cold: write-lock, double-check, compute, insert.
    let mut map = match cached.max_density_by_bw.write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    if let Some(&v) = map.get(&bw_bits) {
        return v;
    }
    let v = max_density_of_matrix(&cached.matrix, bandwidth);
    map.insert(bw_bits, v);
    v
}

/// Normalized KDE: `raw_density(q) / max_density(corpus)`.
///
/// Matches the contract of `geometry::confidence_normalized` but uses the
/// cached matrix + cached `max_density` for amortized `O(N · D)` per call
/// after the first.
pub fn kde_normalized_cached(cached: &CachedMatrix, query: &[f64], bandwidth: f64) -> f64 {
    let raw = kde_raw_from_matrix(&cached.matrix, query, bandwidth);
    let max = max_density_cached(cached, bandwidth);
    if max <= 0.0 {
        0.0
    } else {
        raw / max
    }
}

// ── Cache ───────────────────────────────────────────────────────────

/// Per-StreamState cache mapping `(bundle, fields)` → materialized matrix.
///
/// Thin wrapper around the generic
/// [`crate::caches::single_flight::SingleFlightCache`] (extracted
/// 2026-06-20, workflow `w2n0fgqkk`). Behavior is byte-identical to the
/// prior hand-rolled implementation:
///
/// - `RwLock<HashMap>` hot path, lock-free read on hit (atomic-refcount
///   `CachedMatrix` clone).
/// - Per-key `Mutex<()>` single-flight on cache miss.
/// - Mutation-counter invalidation via the cache's `(V, u64)` tuple.
/// - Random FIFO-on-iteration eviction at capacity.
///
/// The secondary `max_density_by_bw` per-bandwidth cache stays on the
/// value type ([`CachedMatrix`]) — it's tied to the matrix lifetime,
/// not the single-flight pattern, and lives outside the generic.
///
/// The `counter_at_build` field on [`CachedMatrix`] is retained for
/// backward compatibility but no longer consulted for invalidation; the
/// cache's `(V, u64)` tuple is the source of truth.
pub struct VectorMatrixCache {
    inner: crate::caches::single_flight::SingleFlightCache<VectorCacheKey, CachedMatrix>,
}

impl VectorMatrixCache {
    /// New cache holding up to `max_entries` matrices (random eviction at
    /// capacity).
    pub fn new(max_entries: usize) -> Self {
        VectorMatrixCache {
            inner: crate::caches::single_flight::SingleFlightCache::new(max_entries),
        }
    }

    /// Acquire (or create) the per-key compute lock for single-flight
    /// materialization. Caller holds the returned `Arc<Mutex<()>>` for the
    /// duration of the matrix build.
    pub fn acquire_compute_lock(&self, key: &VectorCacheKey) -> Arc<Mutex<()>> {
        self.inner.acquire_compute_lock(key)
    }

    /// Drop the per-key compute-lock entry from the lookup map.
    pub fn release_compute_lock(&self, key: &VectorCacheKey) {
        self.inner.release_compute_lock(key);
    }

    /// Hot-path lookup. Returns `Some(cached)` only if the cached matrix's
    /// counter matches `current_counter`.
    pub fn get(&self, key: &VectorCacheKey, current_counter: u64) -> Option<CachedMatrix> {
        self.inner.get(key, current_counter)
    }

    /// Insert a freshly-built matrix. Returns `true` iff an eviction
    /// happened to make room.
    pub fn insert(&self, key: VectorCacheKey, cached: CachedMatrix) -> bool {
        let counter = cached.counter_at_build;
        self.inner.insert(key, cached, counter)
    }

    /// Number of matrices currently cached. Diagnostic.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty. Diagnostic / clippy-friendliness.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Drop all cached matrices.
    pub fn clear(&self) {
        self.inner.clear();
    }
}

impl Default for VectorMatrixCache {
    fn default() -> Self {
        // Default capacity matches BundleFlowCache's env-overridable default;
        // production sets `GIGI_VECTOR_CACHE_SIZE` env var explicitly.
        VectorMatrixCache::new(64)
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Naive reference: explicit per-cell subtract + square + sum.
    /// Used to verify the cosine-identity fast path matches bit-for-bit
    /// (within float tolerance) on a worked example.
    fn naive_d_sq(data: &[f64], n: usize, d: usize, query: &[f64]) -> Vec<f64> {
        (0..n)
            .map(|i| {
                (0..d)
                    .map(|j| {
                        let v = data[i * d + j] - query[j];
                        v * v
                    })
                    .sum::<f64>()
            })
            .collect()
    }

    #[test]
    fn matrix_d_sq_matches_naive_on_small_grid() {
        // 4 records, 3 dims. Hand-picked values; both methods must agree.
        let data = vec![
            1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, //
            0.0, 0.0, 1.0, //
            1.0, 1.0, 1.0, //
        ];
        let m = MaterializedMatrix::new(data.clone(), 4, 3);
        let q = vec![0.5, 0.5, 0.5];

        let fast = m.d_sq_to_all(&q);
        let naive = naive_d_sq(&data, 4, 3, &q);

        assert_eq!(fast.len(), 4);
        for (f, n) in fast.iter().zip(naive.iter()) {
            assert!((f - n).abs() < 1e-12, "fast={} naive={}", f, n);
        }
    }

    #[test]
    fn matrix_d_sq_matches_naive_on_random_64d() {
        // 50 records, 64 dims, pseudo-random. Tests the inner-loop
        // vectorization path doesn't drift from the naive form at scale.
        let n = 50;
        let d = 64;
        let mut data = Vec::with_capacity(n * d);
        let mut x = 0.123_f64;
        for _ in 0..n * d {
            x = (x * 1.61803).sin();
            data.push(x);
        }
        let mut q = Vec::with_capacity(d);
        for _ in 0..d {
            x = (x * 1.61803).sin();
            q.push(x);
        }

        let m = MaterializedMatrix::new(data.clone(), n, d);
        let fast = m.d_sq_to_all(&q);
        let naive = naive_d_sq(&data, n, d, &q);

        for (i, (f, n)) in fast.iter().zip(naive.iter()).enumerate() {
            assert!(
                (f - n).abs() < 1e-9,
                "row {}: fast={} naive={}",
                i,
                f,
                n
            );
        }
    }

    #[test]
    fn matrix_nearest_matches_argmin_of_d_sq() {
        let data = vec![
            10.0, 0.0, //
            0.0, 10.0, //
            5.0, 5.0, //
            1.0, 1.0, //
            -3.0, -3.0, //
        ];
        let m = MaterializedMatrix::new(data, 5, 2);
        let q = vec![0.0, 0.0];

        let (idx, d_sq) = m.nearest(&q);
        let all = m.d_sq_to_all(&q);
        let argmin = all
            .iter()
            .enumerate()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(i, _)| i)
            .unwrap();
        assert_eq!(idx, argmin);
        assert!((d_sq - all[argmin]).abs() < 1e-12);
        // Sanity: (1, 1) is closest to origin among the five.
        assert_eq!(idx, 3);
    }

    #[test]
    fn matrix_handles_zero_vec_query_without_nan() {
        // A degenerate (zero) query against a normalized corpus must still
        // produce well-defined squared distances (all equal to ‖r_i‖²).
        let data = vec![1.0, 0.0, 0.0, 1.0]; // 2 records in 2D
        let m = MaterializedMatrix::new(data, 2, 2);
        let q = vec![0.0, 0.0];
        let d_sq = m.d_sq_to_all(&q);
        assert!((d_sq[0] - 1.0).abs() < 1e-12);
        assert!((d_sq[1] - 1.0).abs() < 1e-12);
        for v in d_sq {
            assert!(v.is_finite() && v >= 0.0);
        }
    }

    #[test]
    fn matrix_clamps_tiny_negative_d_sq_to_zero() {
        // q == r_0 exactly. The cosine identity yields
        //   ‖q‖² + ‖r_0‖² − 2 ⟨q, r_0⟩ = 2‖q‖² − 2‖q‖² = 0
        // but float roundoff can drop this to -1e-15. The clamp keeps the
        // contract that d² >= 0 always (callers that take sqrt won't see
        // NaN).
        let data = vec![0.123_456_789, 0.987_654_321];
        let m = MaterializedMatrix::new(data.clone(), 1, 2);
        let d_sq = m.d_sq_to_all(&data);
        assert!(d_sq[0] >= 0.0);
        assert!(d_sq[0] < 1e-12);
    }

    #[test]
    fn matrix_nearest_returns_zero_zero_on_empty_matrix() {
        // 0 rows. Calling code is responsible for checking n > 0 before
        // interpreting the result, but the call itself must not panic.
        let m = MaterializedMatrix::new(Vec::new(), 0, 3);
        let (idx, d_sq) = m.nearest(&[0.0, 0.0, 0.0]);
        assert_eq!(idx, 0);
        assert_eq!(d_sq, 0.0);
    }

    #[test]
    fn cache_miss_then_hit_then_invalidate() {
        let cache = VectorMatrixCache::new(8);
        let key = VectorCacheKey::build("bundle_a", &["v0".into(), "v1".into()]);

        // Initial: miss.
        assert!(cache.get(&key, 1).is_none());

        // Insert at counter 1.
        let mat = Arc::new(MaterializedMatrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2));
        cache.insert(key.clone(), CachedMatrix::new(1, Arc::clone(&mat)));
        assert_eq!(cache.len(), 1);

        // Counter matches: hit.
        let hit = cache.get(&key, 1).expect("expected hit");
        assert!(Arc::ptr_eq(&hit.matrix, &mat));

        // Counter advanced: stale → None (entry still in map, but get
        // returns None).
        assert!(cache.get(&key, 2).is_none());
    }

    #[test]
    fn cache_evicts_at_capacity() {
        let cache = VectorMatrixCache::new(2);
        let dummy = |s: u64| {
            CachedMatrix::new(s, Arc::new(MaterializedMatrix::new(vec![s as f64], 1, 1)))
        };
        cache.insert(VectorCacheKey::build("a", &["x".into()]), dummy(1));
        cache.insert(VectorCacheKey::build("b", &["x".into()]), dummy(2));
        assert_eq!(cache.len(), 2);

        // Adding a third entry must evict to keep at capacity.
        let evicted = cache.insert(VectorCacheKey::build("c", &["x".into()]), dummy(3));
        assert!(evicted);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn cache_key_disambiguates_field_order() {
        // Different field orderings produce different columns, so they must
        // get different cache entries.
        let k1 = VectorCacheKey::build("b", &["v0".into(), "v1".into()]);
        let k2 = VectorCacheKey::build("b", &["v1".into(), "v0".into()]);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_key_disambiguates_bundle_name() {
        let k1 = VectorCacheKey::build("alpha", &["v0".into()]);
        let k2 = VectorCacheKey::build("beta", &["v0".into()]);
        assert_ne!(k1, k2);
    }

    #[test]
    fn cache_compute_lock_is_per_key() {
        let cache = VectorMatrixCache::new(8);
        let k1 = VectorCacheKey::build("a", &["x".into()]);
        let k2 = VectorCacheKey::build("b", &["x".into()]);

        let l1 = cache.acquire_compute_lock(&k1);
        let l2 = cache.acquire_compute_lock(&k2);
        // Different keys → different Arcs.
        assert!(!Arc::ptr_eq(&l1, &l2));

        // Same key → same Arc.
        let l1b = cache.acquire_compute_lock(&k1);
        assert!(Arc::ptr_eq(&l1, &l1b));
    }

    #[test]
    fn cache_clear_drops_all_entries() {
        let cache = VectorMatrixCache::new(8);
        for (i, name) in ["a", "b", "c"].iter().enumerate() {
            cache.insert(
                VectorCacheKey::build(name, &["x".into()]),
                CachedMatrix::new(
                    i as u64,
                    Arc::new(MaterializedMatrix::new(vec![i as f64], 1, 1)),
                ),
            );
        }
        assert_eq!(cache.len(), 3);
        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn matrix_bytes_reports_data_plus_norms() {
        let m = MaterializedMatrix::new(vec![0.0; 10 * 4], 10, 4);
        // 10*4 = 40 cells + 10 norms = 50 f64 = 400 bytes.
        assert_eq!(m.bytes(), 50 * 8);
    }

    // ── KDE helper tests ────────────────────────────────────────────

    /// Naive reference for kernel_density_confidence: explicit per-cell
    /// distance, no cosine identity. Used to verify the matrix-cache form
    /// matches the legacy `geometry::kernel_density_confidence` semantics.
    fn naive_kde(data: &[f64], n: usize, d: usize, query: &[f64], bandwidth: f64) -> f64 {
        let two_bw_sq = 2.0 * bandwidth * bandwidth;
        let mut sum = 0.0_f64;
        for i in 0..n {
            let mut d_sq = 0.0_f64;
            for j in 0..d {
                let v = data[i * d + j] - query[j];
                d_sq += v * v;
            }
            sum += (-d_sq / two_bw_sq).exp();
        }
        sum
    }

    #[test]
    fn kde_raw_matches_naive() {
        let data = vec![
            1.0, 0.0, //
            0.0, 1.0, //
            -1.0, 0.0, //
            0.0, -1.0, //
        ];
        let m = MaterializedMatrix::new(data.clone(), 4, 2);
        let q = vec![0.3, 0.4];
        let bw = 0.5;
        let fast = kde_raw_from_matrix(&m, &q, bw);
        let naive = naive_kde(&data, 4, 2, &q, bw);
        assert!(
            (fast - naive).abs() < 1e-10,
            "fast={} naive={}",
            fast,
            naive
        );
    }

    #[test]
    fn kde_raw_zero_bandwidth_returns_zero() {
        let m = MaterializedMatrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2);
        assert_eq!(kde_raw_from_matrix(&m, &[0.0, 0.0], 0.0), 0.0);
        assert_eq!(kde_raw_from_matrix(&m, &[0.0, 0.0], -1.0), 0.0);
    }

    #[test]
    fn kde_raw_empty_matrix_returns_zero() {
        let m = MaterializedMatrix::new(Vec::new(), 0, 2);
        assert_eq!(kde_raw_from_matrix(&m, &[0.0, 0.0], 0.5), 0.0);
    }

    #[test]
    fn max_density_matches_naive_over_corpus() {
        let data = vec![
            0.0, 0.0, //
            1.0, 1.0, //
            2.0, 2.0, //
            0.5, 0.5, //
        ];
        let m = MaterializedMatrix::new(data.clone(), 4, 2);
        let bw = 0.7;
        let fast = max_density_of_matrix(&m, bw);
        let naive: f64 = (0..4)
            .map(|i| naive_kde(&data, 4, 2, &data[i * 2..(i + 1) * 2], bw))
            .fold(0.0_f64, f64::max);
        assert!(
            (fast - naive).abs() < 1e-10,
            "fast={} naive={}",
            fast,
            naive
        );
    }

    #[test]
    fn max_density_cached_returns_same_on_second_call() {
        let m = MaterializedMatrix::new(vec![0.0, 0.0, 1.0, 1.0, 2.0, 2.0], 3, 2);
        let cached = CachedMatrix::new(0, Arc::new(m));
        let bw = 0.5;
        let v1 = max_density_cached(&cached, bw);
        let v2 = max_density_cached(&cached, bw);
        assert_eq!(v1, v2);
        // Cache should be populated.
        let map = cached.max_density_by_bw.read().unwrap();
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&bw.to_bits()));
    }

    #[test]
    fn max_density_cached_separates_bandwidths() {
        let m = MaterializedMatrix::new(vec![0.0, 0.0, 1.0, 1.0], 2, 2);
        let cached = CachedMatrix::new(0, Arc::new(m));
        let _ = max_density_cached(&cached, 0.3);
        let _ = max_density_cached(&cached, 0.7);
        let _ = max_density_cached(&cached, 1.5);
        let map = cached.max_density_by_bw.read().unwrap();
        assert_eq!(map.len(), 3, "three distinct bandwidths → three entries");
    }

    #[test]
    fn kde_normalized_at_corpus_point_is_one() {
        // KDE at any corpus point should equal max_density when that point
        // is the densest, so normalized = 1.0. Use a symmetric corpus where
        // every point has identical density.
        let data = vec![
            0.0, 0.0, //
            1.0, 0.0, //
            0.0, 1.0, //
            1.0, 1.0, //
        ];
        let cached = CachedMatrix::new(0, Arc::new(MaterializedMatrix::new(data, 4, 2)));
        let bw = 0.6;
        // Query at any corpus point — by symmetry, density is equal at
        // each, so all normalize to 1.0.
        let n = kde_normalized_cached(&cached, &[0.0, 0.0], bw);
        assert!((n - 1.0).abs() < 1e-10, "expected 1.0, got {}", n);
    }

    #[test]
    fn kde_normalized_far_from_corpus_is_small() {
        let data = vec![0.0, 0.0, 0.1, 0.1, -0.1, -0.1];
        let cached = CachedMatrix::new(0, Arc::new(MaterializedMatrix::new(data, 3, 2)));
        let bw = 0.2;
        let far = kde_normalized_cached(&cached, &[100.0, 100.0], bw);
        assert!(far < 1e-6, "far-out normalized density should be tiny: {}", far);
    }
}
