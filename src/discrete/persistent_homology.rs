//! PK-4 / Persistent homology — H₀ (connected components) via the
//! Euclidean minimum spanning tree and the elder rule.
//!
//! For a point cloud, the H₀ persistence of the Vietoris–Rips
//! filtration is computed exactly from the MST: sweeping the scale `ε`
//! upward, two components merge precisely when `ε` reaches the weight
//! of the MST edge bridging them (the elder rule keeps the older
//! component alive). So the `n − 1` MST edge weights **are** the finite
//! H₀ death times; every point is born at `ε = 0`; one component
//! survives forever (an infinite bar).
//!
//! The cluster-count signature: for `k` well-separated blobs the `k − 1`
//! longest MST edges (the inter-cluster bridges) stand well above the
//! intra-cluster edges — a clean "persistence gap". A single blob shows
//! no such gap.
//!
//! Ground truth: `theory/post_kahler_directions/validation_tests.py
//! ::test_4_persistent_homology_clusters`. Uses Prim's MST, matching the
//! python reference exactly.

#![cfg(feature = "post_kahler_phase1")]

use std::fmt;

/// Error for degenerate persistence inputs.
#[derive(Debug, Clone, PartialEq)]
pub enum PersistenceError {
    /// Fewer than two points — no merge events exist.
    TooFewPoints,
    /// Points have mismatched or zero dimension.
    BadDimension,
}

impl fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PersistenceError::TooFewPoints => {
                write!(f, "persistent homology needs at least 2 points")
            }
            PersistenceError::BadDimension => {
                write!(f, "all points must share the same non-zero dimension")
            }
        }
    }
}

impl std::error::Error for PersistenceError {}

/// An H₀ persistence bar `[birth, death)`. Births are always 0 (points
/// exist at scale 0); a finite `death` is an MST merge event; the single
/// surviving component has `death = ∞`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PersistenceInterval {
    pub birth: f64,
    pub death: f64,
}

impl PersistenceInterval {
    /// Bar length `death − birth` (∞ for the surviving component).
    pub fn persistence(&self) -> f64 {
        self.death - self.birth
    }
}

fn dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// The `n − 1` Euclidean MST edge weights via Prim's algorithm — the H₀
/// merge events. Returned **sorted descending** (longest bridge first),
/// the form the cluster-count signature reads.
pub fn mst_merge_edges(points: &[Vec<f64>]) -> Result<Vec<f64>, PersistenceError> {
    let n = points.len();
    if n < 2 {
        return Err(PersistenceError::TooFewPoints);
    }
    let d = points[0].len();
    if d == 0 || points.iter().any(|p| p.len() != d) {
        return Err(PersistenceError::BadDimension);
    }

    // Prim's MST: grow the tree from vertex 0, always adding the cheapest
    // edge crossing the frontier. `best[j]` = distance from j to the tree.
    let mut in_tree = vec![false; n];
    let mut best = vec![f64::INFINITY; n];
    in_tree[0] = true;
    for j in 1..n {
        best[j] = dist(&points[0], &points[j]);
    }
    let mut edges = Vec::with_capacity(n - 1);
    for _ in 1..n {
        // cheapest frontier vertex
        let mut u = usize::MAX;
        let mut w = f64::INFINITY;
        for j in 0..n {
            if !in_tree[j] && best[j] < w {
                w = best[j];
                u = j;
            }
        }
        // n≥2 and finite coords ⇒ a finite minimum always exists
        in_tree[u] = true;
        edges.push(w);
        for j in 0..n {
            if !in_tree[j] {
                let dj = dist(&points[u], &points[j]);
                if dj < best[j] {
                    best[j] = dj;
                }
            }
        }
    }
    edges.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    Ok(edges)
}

/// Full H₀ persistence diagram: `n − 1` finite bars `[0, w)` (one per MST
/// edge, longest-lived first) plus one infinite bar `[0, ∞)` for the
/// component that never merges.
pub fn h0_persistence(points: &[Vec<f64>]) -> Result<Vec<PersistenceInterval>, PersistenceError> {
    let edges = mst_merge_edges(points)?;
    let mut bars: Vec<PersistenceInterval> = edges
        .iter()
        .map(|&w| PersistenceInterval { birth: 0.0, death: w })
        .collect();
    bars.push(PersistenceInterval { birth: 0.0, death: f64::INFINITY });
    Ok(bars)
}

/// Estimate the number of clusters from the H₀ persistence gap. Returns
/// the largest `k` such that the `k − 1` longest MST edges each exceed
/// `gap_factor ×` the `k`-th longest (default signature `gap_factor = 2`).
/// A single blob (no gap) returns `1`.
pub fn cluster_count(points: &[Vec<f64>], gap_factor: f64) -> Result<usize, PersistenceError> {
    let edges = mst_merge_edges(points)?;
    // edges sorted descending: [bridge₁, …, bridge_{k−1}, intra_max, …].
    // The persistence gap is the first `gap_factor×` drop scanning down
    // that separates the inter-cluster bridges (large — and only similar
    // to *each other*, so consecutive bridge ratios are ≈1) from the
    // largest intra-cluster edge below. The `i+1` edges above the drop
    // are the bridges ⇒ `i+2` clusters.
    //
    // The drop must be off a genuine *bridge*: `edges[i]` must exceed the
    // mean edge length. This rejects the noisy `>gap_factor×` ratios that
    // occur among the tiny tail edges of a single unclustered blob (whose
    // top ratio is < gap_factor by construction), which would otherwise
    // fabricate clusters from intra-blob edge noise.
    let mean = edges.iter().sum::<f64>() / edges.len() as f64;
    for i in 0..edges.len().saturating_sub(1) {
        if edges[i] >= mean && edges[i + 1] > 0.0 && edges[i] > gap_factor * edges[i + 1] {
            return Ok(i + 2);
        }
    }
    Ok(1)
}

// ── tests ─────────────────────────────────────────────────────────
// Mirror validation_tests.py::test_4_persistent_homology_clusters.

#[cfg(test)]
mod tests {
    use super::*;

    // deterministic gaussian-ish jitter via a small LCG + Box–Muller
    fn jittered_blobs(centers: &[[f64; 2]], per: usize, scale: f64, seed: u64) -> Vec<Vec<f64>> {
        let mut s = seed;
        let mut u = || {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (((s >> 11) as f64) + 0.5) / (1u64 << 53) as f64
        };
        let mut pts = Vec::new();
        for c in centers {
            let mut made = 0;
            while made < per {
                let (u1, u2) = (u(), u());
                let r = (-2.0 * u1.ln()).sqrt();
                let z0 = r * (std::f64::consts::TAU * u2).cos();
                let z1 = r * (std::f64::consts::TAU * u2).sin();
                pts.push(vec![c[0] + scale * z0, c[1] + scale * z1]);
                made += 1;
            }
        }
        pts
    }

    #[test]
    fn three_separated_blobs_show_persistence_gap() {
        // k=3 well-separated clusters (matches the python fixture).
        let centers = [[0.0, 0.0], [10.0, 0.0], [5.0, 8.0]];
        let pts = jittered_blobs(&centers, 30, 0.3, 20260525);
        let edges = mst_merge_edges(&pts).unwrap();
        let k = 3;
        let top = &edges[..k - 1];
        let next = edges[k - 1];
        assert!(
            top.iter().all(|&e| e > 2.0 * next),
            "top {} MST edges {top:?} must each exceed 2× the {k}-th ({next})",
            k - 1
        );
        // and the estimator agrees
        assert_eq!(cluster_count(&pts, 2.0).unwrap(), 3);
    }

    #[test]
    fn single_blob_has_no_gap() {
        // Negative control: one Gaussian blob — no 2× gap at the top.
        let pts = jittered_blobs(&[[0.0, 0.0]], 90, 1.0, 12345);
        let edges = mst_merge_edges(&pts).unwrap();
        let ratio = edges[0] / edges[1];
        assert!(ratio < 2.0, "single blob largest/2nd ratio {ratio} should be < 2");
        assert_eq!(cluster_count(&pts, 2.0).unwrap(), 1);
    }

    #[test]
    fn h0_diagram_has_one_infinite_bar() {
        let pts = jittered_blobs(&[[0.0, 0.0], [10.0, 0.0]], 10, 0.2, 7);
        let bars = h0_persistence(&pts).unwrap();
        assert_eq!(bars.len(), pts.len(), "n points → n bars (n−1 finite + 1 infinite)");
        assert_eq!(bars.iter().filter(|b| b.death.is_infinite()).count(), 1);
        assert!(bars.iter().all(|b| b.birth == 0.0));
    }

    #[test]
    fn too_few_points_errors() {
        assert_eq!(mst_merge_edges(&[]), Err(PersistenceError::TooFewPoints));
        assert_eq!(mst_merge_edges(&[vec![1.0, 2.0]]), Err(PersistenceError::TooFewPoints));
    }
}
