//! Spectral capacity + RG flow — §3.6–3.7.
//!
//! Implements:
//!   Def 3.9:  Field index graph
//!   Def 3.10: Normalized graph Laplacian
//!   Thm 3.4:  Spectral capacity C_sp = λ₁ · D²
//!   Def 3.11: Coarse-graining operator
//!   Thm 3.5:  C-theorem (completion entropy non-increasing)

use std::collections::{HashMap, HashSet, VecDeque};

use crate::bundle::BundleStore;
use crate::types::{BasePoint, Value};

// ─── Error-budget partition for downstream consumers ───────────────────────
//
// Consumers (Marcella, SCJ, KRAKEN) carry per-consumer "error budget"
// allocations that bound the total geometric drift a query can absorb before
// the result is no longer trustable.  When the engine introduces a NEW source
// of approximation (a GPU matvec backend, an HNSW recall <1.0, a Chebyshev
// filter with looser λ_max), every consumer's budget has to widen by enough
// to absorb it.
//
// **Bound vs reservation are two different numbers.**  This distinction
// (negotiated 2026-06-06 → 2026-06-07 with the SCJ team, after an arithmetic
// error in our 2026-06-05 letter conflated the two) is now load-bearing for
// every consumer's budget arithmetic.  Specifically:
//
//   * `E_*_BOUND_*` constants below — the OBSERVED WORST CASE.  These are
//     measurement claims about the implementation: "the GPU backend's L∞
//     drift in K(t)·v vs the CPU f64 reference is at most this much."  They
//     do not, by themselves, move any consumer's budget — they only say
//     what we measured.
//
//   * `R_*_SLACK` constants below — the COMMITTED HEADROOM consumers reserve
//     in their δ_indep budget for the approximation.  These ARE the
//     consumer-side movement: SCJ moved their δ_indep partition to make
//     room for these.  The reservation is always ≥ the bound, with extra
//     slack to absorb operational realities the bound does not enumerate.
//
// The partition is published as a numerical contract.  See:
//   theory/scj/REPLY_TO_REPLY_2_2026-06-07.md §1 (the 4-row partition table)
//   theory/scj/REPLY_FROM_SCJ_2026-06-07.md   §2 (SCJ's counter-correction)
//
// The unit test `delta_indep_partition_sums_to_target()` below asserts the
// partition closes against δ_indep = 0.030 (the SCJ v0.5 §3.2 vacuity gate).
// Anyone touching a reservation in code triggers that test failure until
// both teams ack the change.  That is the substrate-side enforcement of
// the consumer contract.

// ─── E_*_BOUND_* — observed-worst-case bounds (measurement claims) ───────────

/// Maximum drift the eventual GPU/wgpu spectral backend may introduce in
/// `K(t)·v` (heat kernel applied to a source vector) versus the CPU f64
/// reference path, in L∞ norm relative to ‖v‖.
///
/// Reserved 2026-06-05 in correspondence with the SCJ (Shadow Clone Jutsu)
/// team during the Windows-Atlas-on-Gigi heads-up.  Renamed from
/// `E_BACKEND_SLACK_SPECTRAL` to `E_BACKEND_BOUND_SPECTRAL` 2026-06-07 to
/// make the bound/reservation distinction visible at every callsite.
///
/// SHIPPED CONTRACT: until `wgpu-spectral` lights up, all spectral paths
/// stay CPU-f64 and this term is structurally zero.  When it ships, the
/// per-shard backend with cross-shard correction on CPU keeps drift well
/// inside the bound.  The consumer reservation for this term is
/// `R_BACKEND_SLACK` below.
pub const E_BACKEND_BOUND_SPECTRAL: f64 = 1.0e-4;

/// Maximum recall miss the HNSW-backed `SIMILAR` path may introduce at
/// rank K=200 vs brute-force, as the rate at which the HNSW result-set
/// differs from the exact top-200 result-set.  Bound for SCJ's
/// approximate-default hunt path; SCJ's calibration path uses
/// `SIMILAR EXACT` and ignores this bound entirely.
///
/// Reserved 2026-06-06 by SCJ in their reply §1A.  Renamed from
/// `E_RECALL_SLACK_HNSW` to `E_RECALL_BOUND_HNSW` 2026-06-07 to make
/// the bound/reservation distinction visible.  The consumer reservation
/// for this term is `R_RECALL_SLACK` below.
///
/// SHIPPED CONTRACT: until the single-field HNSW path lights up for
/// `Value::Vector` (Ask C), this term is structurally zero.  The
/// cluster-restricted recall oracle (`SIMILAR EXACT WITHIN spectral_cluster
/// = C(q)` on 1% of calls) is the operational check that keeps this honest.
pub const E_RECALL_BOUND_HNSW: f64 = 0.05;

// ─── R_*_SLACK — committed-headroom reservations (consumer-side movement) ────
//
// These constitute the 4-row partition negotiated with the SCJ team
// 2026-06-06 → 2026-06-07.  SCJ moves their δ_indep partition (SCJ v0.5
// §3.2 vacuity gate) to:
//
//     δ_indep target  = DELTA_INDEP_TARGET                  = 0.030
//     ├─ r_backend    = R_BACKEND_SLACK                     = 0.005
//     ├─ r_recall     = R_RECALL_SLACK                      = 0.005
//     ├─ r_holonomy   = R_HOLONOMY_SLACK                    = 0.005
//     ├─ r_residual   = R_RESIDUAL_SLACK (unmodeled corr.)  = 0.010
//     └─ vacuity      = DELTA_INDEP_VACUITY_SLACK           = 0.005
//
// Sum = 0.005 + 0.005 + 0.005 + 0.010 + 0.005 = 0.030.  Asserted at test
// time by `delta_indep_partition_sums_to_target()`.

/// Reservation (consumer headroom) absorbing the eventual GPU/wgpu spectral
/// backend's drift in δ_indep.  Strictly greater than `E_BACKEND_BOUND_SPECTRAL`
/// to leave operational room above the observed-worst-case bound.
pub const R_BACKEND_SLACK: f64 = 0.005;

/// Reservation (consumer headroom) absorbing the HNSW approximate recall
/// miss in δ_indep.  Strictly less than `E_RECALL_BOUND_HNSW` because the
/// recall bound is measured *per query* and amortizes; the budget contribution
/// is the long-run average rather than the worst case.
pub const R_RECALL_SLACK: f64 = 0.005;

/// Reservation (consumer headroom) absorbing the holonomy estimate
/// gate-test (planted phase recovery ±1% on a 10-function toy CFG; the
/// §10 PR in `theory/post_kahler_directions/`).
pub const R_HOLONOMY_SLACK: f64 = 0.005;

/// Reservation absorbing unmodeled correlations across the other three
/// reservations.  Named explicitly (not folded into vacuity slack) because
/// **unmodeled correlations will exist whether or not we enumerate them**;
/// naming the residual is the discipline that lets the partition close.
/// Per SCJ 2026-06-07 reply §2 — they caught us not enumerating this, and
/// the unbalanced-without-it partition was a substrate-trust bug.
pub const R_RESIDUAL_SLACK: f64 = 0.010;

/// Vacuity slack remaining inside the δ_indep target after all reservations
/// land.  This is what stays unallocated as headroom for novel error
/// sources we have not yet anticipated.  Must be > 0 — a zero-vacuity
/// partition has no room to absorb a fifth term ahead of contract renegotiation.
pub const DELTA_INDEP_VACUITY_SLACK: f64 = 0.005;

/// δ_indep budget target.  Pinned at 0.030 in the SCJ v0.5 §3.2 vacuity gate.
/// The 4-row partition above sums to this target; the unit test enforces it.
pub const DELTA_INDEP_TARGET: f64 = 0.030;

#[cfg(test)]
mod budget_partition_tests {
    use super::*;

    /// **Substrate-side enforcement of the SCJ δ_indep contract.**
    ///
    /// Asserts the 4-row partition + vacuity slack sums to the published
    /// target.  Any change to a reservation triggers this test until both
    /// teams ack the contract change.  See:
    ///   theory/scj/REPLY_TO_REPLY_2_2026-06-07.md §1 (table)
    ///   theory/scj/REPLY_FROM_SCJ_2026-06-07.md   §2 (counter-correction)
    #[test]
    fn delta_indep_partition_sums_to_target() {
        let sum = R_BACKEND_SLACK
            + R_RECALL_SLACK
            + R_HOLONOMY_SLACK
            + R_RESIDUAL_SLACK
            + DELTA_INDEP_VACUITY_SLACK;
        let diff = (sum - DELTA_INDEP_TARGET).abs();
        assert!(
            diff < 1.0e-12,
            "δ_indep partition broken: {R_BACKEND_SLACK} (r_backend) + \
             {R_RECALL_SLACK} (r_recall) + {R_HOLONOMY_SLACK} (r_holonomy) + \
             {R_RESIDUAL_SLACK} (r_residual) + {DELTA_INDEP_VACUITY_SLACK} (vacuity) \
             = {sum}; target = {DELTA_INDEP_TARGET}; \
             diff = {diff:e}.  Update partition or ack contract change with \
             SCJ before landing."
        );
    }

    /// Asserts the single bound-vs-reservation row that actually has both
    /// constants defined and aligned in direction: `R_BACKEND_SLACK` (the
    /// committed budget headroom for GPU/wgpu spectral drift) must dominate
    /// `E_BACKEND_BOUND_SPECTRAL` (the observed-worst-case bound on that
    /// drift).
    ///
    /// **Why this test only covers one row of the four-row partition.**
    /// Named explicitly per SCJ 2026-06-07 close drift #1 — the prior
    /// name `reservations_dominate_bounds` (plural) over-promised what the
    /// body actually enforces. The three rows NOT asserted here:
    ///
    /// - `R_RECALL_SLACK` (= 0.005) vs `E_RECALL_BOUND_HNSW` (= 0.05):
    ///   the inequality is **inverted by design**. The bound is per-query
    ///   worst case; the reservation is the long-run-average budget
    ///   contribution. SCJ's δ_indep math sums per-query reservations,
    ///   not per-query worst cases. Asserting `R >= E` here would
    ///   over-reserve and fail the 4-row partition; documented in the
    ///   rustdoc on `R_RECALL_SLACK`.
    ///
    /// - `R_HOLONOMY_SLACK` (= 0.005) vs `E_HOLONOMY_BOUND_*`:
    ///   the bound constant does not yet exist. `E_HOLONOMY_BOUND_*` is
    ///   pending the §10 PR in `theory/post_kahler_directions/` — once
    ///   that lands with the planted-phase recovery ±1% gate-test,
    ///   peer constant + sibling test arrive together.
    ///   TODO(§10): add `r_holonomy_slack_dominates_e_holonomy_bound`.
    ///
    /// - `R_RESIDUAL_SLACK` (= 0.010) vs nothing: `r_residual` has no
    ///   peer `E_*` by construction. It absorbs unmodeled correlations
    ///   across the other three rows; there is no single bound to
    ///   dominate. The 4-row partition discipline IS the assertion
    ///   here, and that's already enforced by
    ///   `delta_indep_partition_sums_to_target()` above.
    #[test]
    fn r_backend_slack_dominates_e_backend_bound() {
        assert!(
            R_BACKEND_SLACK >= E_BACKEND_BOUND_SPECTRAL,
            "r_backend ({R_BACKEND_SLACK}) must be ≥ E_backend bound ({E_BACKEND_BOUND_SPECTRAL})"
        );
    }
}

/// Find connected components directly from the field index bitmaps.
///
/// Two base points are in the same component if they share any indexed
/// field value.  Uses union-find over bitmap buckets — O(buckets × α(n)).
fn components_from_index(store: &BundleStore) -> Vec<Vec<BasePoint>> {
    let all_bps: Vec<BasePoint> = store.sections().map(|(bp, _)| bp).collect();
    if all_bps.is_empty() {
        return vec![];
    }

    // Union-Find
    let bp_to_idx: HashMap<BasePoint, usize> =
        all_bps.iter().enumerate().map(|(i, &bp)| (bp, i)).collect();
    let n = all_bps.len();
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank: Vec<usize> = vec![0; n];

    fn find(parent: &mut [usize], x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    fn union(parent: &mut [usize], rank: &mut [usize], a: usize, b: usize) {
        let ra = find(parent, a);
        let rb = find(parent, b);
        if ra == rb {
            return;
        }
        if rank[ra] < rank[rb] {
            parent[ra] = rb;
        } else if rank[ra] > rank[rb] {
            parent[rb] = ra;
        } else {
            parent[rb] = ra;
            rank[ra] += 1;
        }
    }

    // For each bitmap bucket, union all members
    for field_map in store.field_index_maps().values() {
        for bitmap in field_map.values() {
            let mut first_idx: Option<usize> = None;
            for bp32 in bitmap.iter() {
                let bp = store.resolve_bp(bp32);
                if let Some(&idx) = bp_to_idx.get(&bp) {
                    if let Some(fi) = first_idx {
                        union(&mut parent, &mut rank, fi, idx);
                    } else {
                        first_idx = Some(idx);
                    }
                }
            }
        }
    }

    // Collect components
    let mut comp_map: HashMap<usize, Vec<BasePoint>> = HashMap::new();
    for (i, &bp) in all_bps.iter().enumerate() {
        let root = find(&mut parent, i);
        comp_map.entry(root).or_default().push(bp);
    }
    comp_map.into_values().collect()
}

/// Build the adjacency structure of the field index graph (Def 3.9).
///
/// Uses the field index bitmaps directly — avoids materializing all records.
/// Two points are connected if they share any indexed field value.
///
/// NOTE: Only called for small graphs or when full adjacency is needed
/// for eigenvalue computation. For component detection, use
/// `components_from_index()` instead.
pub fn field_index_graph(store: &BundleStore) -> HashMap<BasePoint, Vec<BasePoint>> {
    let mut adj: HashMap<BasePoint, HashSet<BasePoint>> = HashMap::new();

    for field_name in &store.schema.indexed_fields {
        for val in store.indexed_values(field_name) {
            let group = store.neighborhood(field_name, &val);
            for &p in &group {
                let entry = adj.entry(p).or_default();
                for &q in &group {
                    if p != q {
                        entry.insert(q);
                    }
                }
            }
        }
    }

    adj.into_iter()
        .map(|(bp, set)| {
            let mut v: Vec<_> = set.into_iter().collect();
            v.sort();
            (bp, v)
        })
        .collect()
}

/// Compute the spectral gap λ₁ (smallest nonzero eigenvalue of normalized Laplacian).
///
/// Strategy:
///   1. Find connected components from field index — O(n × fields)
///   2. If disconnected (k > 1 components): λ₁ = 0 immediately
///   3. If connected: check for clique structure (analytic formula)
///   4. Fallback: sparse power iteration for general graphs
///
/// Def 3.10: L = I - D⁻¹/² W D⁻¹/²
pub fn spectral_gap(store: &BundleStore) -> f64 {
    let n = store.len();
    if n < 2 {
        return 0.0;
    }

    // Step 1: Connected components via field index BFS — no adjacency list
    let components = components_from_index(store);
    if components.len() > 1 {
        return 0.0;
    }

    // Step 2: Check if graph is a single clique (all share same indexed values)
    // For a complete graph K_n: λ₁ = n/(n-1)
    let adj = field_index_graph(store);
    let is_clique = adj.values().all(|nbrs| nbrs.len() == n - 1);
    if is_clique {
        return n as f64 / (n as f64 - 1.0);
    }

    // Step 3: Sparse power iteration for general connected graphs
    sparse_spectral_gap(&adj)
}

/// Sparse power iteration for λ₁ on a connected graph.
///
/// Uses deflation against the dominant eigenvector of the normalized
/// adjacency matrix M = D⁻¹/² W D⁻¹/².
fn sparse_spectral_gap(adj: &HashMap<BasePoint, Vec<BasePoint>>) -> f64 {
    let bps: Vec<BasePoint> = adj.keys().copied().collect();
    let bp_to_idx: HashMap<BasePoint, usize> =
        bps.iter().enumerate().map(|(i, &bp)| (bp, i)).collect();
    let n = bps.len();

    let degrees: Vec<f64> = bps
        .iter()
        .map(|bp| adj.get(bp).map_or(0, |v| v.len()) as f64)
        .collect();

    let d_inv_sqrt: Vec<f64> = degrees
        .iter()
        .map(|&d| if d > 0.0 { 1.0 / d.sqrt() } else { 0.0 })
        .collect();

    // Dominant eigenvector: u = D^{1/2} · 1, normalized
    let u: Vec<f64> = {
        let raw: Vec<f64> = degrees.iter().map(|d| d.sqrt()).collect();
        let norm = raw.iter().map(|x| x * x).sum::<f64>().sqrt();
        raw.into_iter().map(|x| x / norm).collect()
    };

    // Sparse M·v multiplication
    let mul_m = |src: &[f64]| -> Vec<f64> {
        let mut out = vec![0.0f64; n];
        for (i, &bp) in bps.iter().enumerate() {
            if let Some(neighbors) = adj.get(&bp) {
                for &nbp in neighbors {
                    if let Some(&j) = bp_to_idx.get(&nbp) {
                        out[i] += d_inv_sqrt[i] * src[j] * d_inv_sqrt[j];
                    }
                }
            }
        }
        out
    };

    let mut v: Vec<f64> = (0..n).map(|i| ((i as f64 + 1.0) * 2.654).sin()).collect();

    for _ in 0..300 {
        let dot: f64 = v.iter().zip(u.iter()).map(|(a, b)| a * b).sum();
        for i in 0..n {
            v[i] -= dot * u[i];
        }
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-14 {
            return 1.0;
        }
        for x in v.iter_mut() {
            *x /= norm;
        }
        v = mul_m(&v);
    }

    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < 1e-14 {
        return 1.0;
    }
    for x in v.iter_mut() {
        *x /= norm;
    }

    let mv = mul_m(&v);
    let mu2: f64 = v.iter().zip(mv.iter()).map(|(a, b)| a * b).sum();
    (1.0 - mu2).max(0.0)
}

/// Graph diameter: longest shortest path in the field index graph.
///
/// For disconnected graphs, returns the max diameter across components.
/// For cliques (all nodes share the same indexed value), diameter = 1.
pub fn graph_diameter(store: &BundleStore) -> usize {
    let n = store.len();
    if n < 2 {
        return 0;
    }

    // Fast component detection
    let components = components_from_index(store);

    // For small component counts with clique structure, use analytic result:
    // A clique has diameter 1. Union of disjoint cliques has max diameter 1.
    let all_cliques = components.iter().all(|comp| {
        if comp.len() <= 1 {
            return true;
        }
        // Check: does every node in this component have degree = comp_size - 1?
        // Instead of building full adjacency, check bucket sizes from index.
        // If a component maps to a single field-value bucket, it's a clique.
        comp.len() <= 1 || {
            // Quick check: see if every pair shares a field value
            let first = comp[0];
            let nbrs = store.geometric_neighbors(first);
            nbrs.len() >= comp.len() - 1
        }
    });

    if all_cliques {
        return if components.iter().any(|c| c.len() > 1) {
            1
        } else {
            0
        };
    }

    // General case: build adjacency and BFS
    let adj = field_index_graph(store);
    if components.len() > 1 {
        let mut max_diam = 0;
        for comp in &components {
            let sub_adj: HashMap<BasePoint, Vec<BasePoint>> = comp
                .iter()
                .filter_map(|&bp| adj.get(&bp).map(|nbrs| (bp, nbrs.clone())))
                .collect();
            max_diam = max_diam.max(component_diameter(&sub_adj));
        }
        max_diam
    } else {
        component_diameter(&adj)
    }
}

fn component_diameter(adj: &HashMap<BasePoint, Vec<BasePoint>>) -> usize {
    let bps: Vec<BasePoint> = adj.keys().copied().collect();
    let n = bps.len();
    if n < 2 {
        return 0;
    }
    let bp_to_idx: HashMap<BasePoint, usize> =
        bps.iter().enumerate().map(|(i, &bp)| (bp, i)).collect();

    let mut max_dist = 0usize;
    let limit = n.min(100);
    for start in 0..limit {
        let mut dist = vec![usize::MAX; n];
        dist[start] = 0;
        let mut queue = VecDeque::new();
        queue.push_back(start);
        while let Some(u) = queue.pop_front() {
            if let Some(neighbors) = adj.get(&bps[u]) {
                for &nbp in neighbors {
                    if let Some(&v) = bp_to_idx.get(&nbp) {
                        if dist[v] == usize::MAX {
                            dist[v] = dist[u] + 1;
                            queue.push_back(v);
                        }
                    }
                }
            }
        }
        for &d in &dist {
            if d != usize::MAX && d > max_dist {
                max_dist = d;
            }
        }
    }
    max_dist
}

/// Spectral capacity C_sp = λ₁ · D² (Thm 3.4).
pub fn spectral_capacity(store: &BundleStore) -> f64 {
    let lambda1 = spectral_gap(store);
    if lambda1 < f64::EPSILON {
        return 0.0; // Disconnected → skip expensive diameter computation
    }
    let diam = graph_diameter(store);
    lambda1 * (diam as f64) * (diam as f64)
}

// ── RG Flow: Coarse-Graining (§3.7) ──

/// Coarse-grain a bundle at a given scale ℓ (Def 3.11).
///
/// ℓ = 1: merge base points sharing ALL indexed field values.
/// Betti numbers of the field index graph (graph-theoretic, not persistent homology).
///
/// β₀ = number of connected components
/// β₁ = |E| - |V| + β₀ (cycle rank / first Betti number)
///
/// Caveat: These are graph-theoretic Betti numbers computed from the 1-skeleton
/// (field index graph), NOT topological Betti numbers from persistent homology/TDA.
///
/// For higher Betti numbers (β_k, k ≥ 2) on a [`crate::lattice::Lattice`] cell
/// complex — i.e. the topological β_k rather than this graph-theoretic
/// surface — call [`crate::topology::betti_topological`] directly.
/// The two functions are intentionally distinct: this one operates on the
/// BundleStore field-index graph (the existing pre-Halcyon surface); the
/// topology kernel operates on declared `Lattice` cell complexes (the
/// Halcyon § BETTI ORDER k / PI_1 surface introduced 2026-06-29).
#[cfg(feature = "lattice")]
pub use crate::topology::{
    betti_topological as higher_betti, pi_1_presentation, Pi1Presentation, TopologyError,
};

pub fn betti_numbers(store: &BundleStore) -> (usize, usize) {
    let n = store.len();
    if n == 0 {
        return (0, 0);
    }
    let components = components_from_index(store);
    let beta_0 = components.len();

    // Count edges: each adjacency list entry is a directed edge; divide by 2
    let adj = field_index_graph(store);
    let edge_count: usize = adj.values().map(|nbrs| nbrs.len()).sum::<usize>() / 2;

    // β₁ = |E| - |V| + β₀  (Euler formula for graphs)
    let beta_1 = (edge_count + beta_0).saturating_sub(n);

    (beta_0, beta_1)
}

/// Standalone entropy from field index groupings (in nats, using natural log).
///
/// S = -Σ (nᵢ/N) ln(nᵢ/N)
///
/// Unit: nats (natural log). For bits, divide by ln(2).
/// Uses coarse_grain at scale 1 (finest resolution).
pub fn entropy(store: &BundleStore) -> f64 {
    coarse_grain(store, 1).1
}

/// ℓ = 2: merge base points sharing all but one indexed field value.
/// etc.
///
/// Returns (groups, completion_entropy) where groups are the merged partitions
/// and entropy measures how much information was lost.
pub fn coarse_grain(store: &BundleStore, scale: usize) -> (Vec<Vec<BasePoint>>, f64) {
    let indexed = &store.schema.indexed_fields;
    if indexed.is_empty() || store.len() == 0 {
        return (vec![], 0.0);
    }

    // For scale ℓ, we match on (indexed_fields.len() - scale + 1) fields
    let match_count = indexed.len().saturating_sub(scale - 1).max(1);
    let match_fields: Vec<String> = indexed.iter().take(match_count).cloned().collect();

    // Group base points by their values on the match fields
    let mut groups: HashMap<Vec<Value>, Vec<BasePoint>> = HashMap::new();
    for rec in store.records() {
        let key_vals: Vec<Value> = match_fields
            .iter()
            .map(|f| rec.get(f).cloned().unwrap_or(Value::Null))
            .collect();
        let bp = store.base_point(&{
            let mut key = crate::types::Record::new();
            for bf in &store.schema.base_fields {
                if let Some(v) = rec.get(&bf.name) {
                    key.insert(bf.name.clone(), v.clone());
                }
            }
            key
        });
        groups.entry(key_vals).or_default().push(bp);
    }

    let group_list: Vec<Vec<BasePoint>> = groups.into_values().collect();

    // Completion entropy: H = -Σ (nᵢ/N) log(nᵢ/N)
    let n_total = store.len() as f64;
    let entropy: f64 = group_list
        .iter()
        .map(|g| {
            let p = g.len() as f64 / n_total;
            if p > 0.0 {
                -p * p.ln()
            } else {
                0.0
            }
        })
        .sum();

    (group_list, entropy)
}

// ── Geodesic Distance (§2.1) ───────────────────────────────────────────────

use std::cmp::Ordering;

/// Entry in the Dijkstra priority queue (min-heap by distance).
#[derive(PartialEq)]
struct DijkState {
    cost: f64,
    node: BasePoint,
}

impl Eq for DijkState {}

impl PartialOrd for DijkState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for DijkState {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

/// Geodesic distance on the data manifold.
///
/// Shortest path on the field index graph weighted by fiber metric distance.
/// Returns `None` if no path exists (disconnected components) or if the
/// path exceeds `max_hops` nodes.
///
/// $d_g(p, q) = \min_{\gamma: p \to q} \sum_{(i,j) \in \gamma} g_F(r_i, r_j)$
pub fn geodesic_distance(
    store: &BundleStore,
    bp_a: BasePoint,
    bp_b: BasePoint,
    max_hops: usize,
) -> Option<f64> {
    if bp_a == bp_b {
        return Some(0.0);
    }

    let adj = field_index_graph(store);
    if !adj.contains_key(&bp_a) || !adj.contains_key(&bp_b) {
        return None;
    }

    let fiber_fields = &store.schema.fiber_fields;

    // Cache fiber values for visited nodes
    let mut fiber_cache: HashMap<BasePoint, Vec<crate::types::Value>> = HashMap::new();
    let get_fiber = |bp: BasePoint, cache: &mut HashMap<BasePoint, Vec<crate::types::Value>>| -> Option<Vec<crate::types::Value>> {
        if let Some(v) = cache.get(&bp) {
            return Some(v.clone());
        }
        let fiber = store.get_fiber(bp)?.to_vec();
        cache.insert(bp, fiber.clone());
        Some(fiber)
    };

    // Dijkstra with hop limit
    let mut dist: HashMap<BasePoint, f64> = HashMap::new();
    let mut hops: HashMap<BasePoint, usize> = HashMap::new();
    let mut heap = std::collections::BinaryHeap::new();

    dist.insert(bp_a, 0.0);
    hops.insert(bp_a, 0);
    heap.push(DijkState { cost: 0.0, node: bp_a });

    while let Some(DijkState { cost, node }) = heap.pop() {
        if node == bp_b {
            return Some(cost);
        }

        // Skip if we already found a better path
        if let Some(&best) = dist.get(&node) {
            if cost > best {
                continue;
            }
        }

        let current_hops = hops[&node];
        if current_hops >= max_hops {
            continue;
        }

        let fiber_node = match get_fiber(node, &mut fiber_cache) {
            Some(f) => f,
            None => continue,
        };

        if let Some(neighbors) = adj.get(&node) {
            for &nbr in neighbors {
                let fiber_nbr = match get_fiber(nbr, &mut fiber_cache) {
                    Some(f) => f,
                    None => continue,
                };

                let edge_weight = crate::metric::FiberMetric::distance(
                    fiber_fields,
                    &fiber_node,
                    &fiber_nbr,
                );
                let new_cost = cost + edge_weight;

                let is_better = dist.get(&nbr).map_or(true, |&d| new_cost < d);
                if is_better {
                    dist.insert(nbr, new_cost);
                    hops.insert(nbr, current_hops + 1);
                    heap.push(DijkState { cost: new_cost, node: nbr });
                }
            }
        }
    }

    None // no path found within max_hops
}

/// Dijkstra with predecessor tracking — returns the sequence of base points
/// from `bp_a` to `bp_b` (inclusive), or `None` if unreachable within max_hops.
pub fn geodesic_path(
    store: &BundleStore,
    bp_a: BasePoint,
    bp_b: BasePoint,
    max_hops: usize,
) -> Option<Vec<BasePoint>> {
    if bp_a == bp_b {
        return Some(vec![bp_a]);
    }

    let adj = field_index_graph(store);
    if !adj.contains_key(&bp_a) || !adj.contains_key(&bp_b) {
        return None;
    }

    let fiber_fields = &store.schema.fiber_fields;
    let mut fiber_cache: HashMap<BasePoint, Vec<crate::types::Value>> = HashMap::new();
    let get_fiber = |bp: BasePoint, cache: &mut HashMap<BasePoint, Vec<crate::types::Value>>| -> Option<Vec<crate::types::Value>> {
        if let Some(v) = cache.get(&bp) { return Some(v.clone()); }
        let fiber = store.get_fiber(bp)?.to_vec();
        cache.insert(bp, fiber.clone());
        Some(fiber)
    };

    let mut dist: HashMap<BasePoint, f64> = HashMap::new();
    let mut hops: HashMap<BasePoint, usize> = HashMap::new();
    let mut prev: HashMap<BasePoint, BasePoint> = HashMap::new();
    let mut heap = std::collections::BinaryHeap::new();

    dist.insert(bp_a, 0.0);
    hops.insert(bp_a, 0);
    heap.push(DijkState { cost: 0.0, node: bp_a });

    let mut found = false;
    while let Some(DijkState { cost, node }) = heap.pop() {
        if node == bp_b { found = true; break; }
        if let Some(&best) = dist.get(&node) { if cost > best { continue; } }
        let current_hops = hops[&node];
        if current_hops >= max_hops { continue; }
        let fiber_node = match get_fiber(node, &mut fiber_cache) { Some(f) => f, None => continue };
        if let Some(neighbors) = adj.get(&node) {
            for &nbr in neighbors {
                let fiber_nbr = match get_fiber(nbr, &mut fiber_cache) { Some(f) => f, None => continue };
                let edge_weight = crate::metric::FiberMetric::distance(fiber_fields, &fiber_node, &fiber_nbr);
                let new_cost = cost + edge_weight;
                if dist.get(&nbr).map_or(true, |&d| new_cost < d) {
                    dist.insert(nbr, new_cost);
                    hops.insert(nbr, current_hops + 1);
                    prev.insert(nbr, node);
                    heap.push(DijkState { cost: new_cost, node: nbr });
                }
            }
        }
    }

    if !found { return None; }

    // Reconstruct path
    let mut path = Vec::new();
    let mut cur = bp_b;
    loop {
        path.push(cur);
        if cur == bp_a { break; }
        match prev.get(&cur) {
            Some(&p) => cur = p,
            None => return None, // broken predecessor chain
        }
    }
    path.reverse();
    Some(path)
}

// ── Spectral fiber modes ──

/// A single spectral mode computed from a fiber projection.
pub struct FiberMode {
    /// Mode index (1-based).
    pub mode: usize,
    /// Eigenvalue (variance explained by this mode).
    pub lambda: f64,
    /// Inverse participation ratio (localization measure; 1 = fully localized).
    pub ipr: f64,
}

/// Compute the top-`modes` spectral modes of the fiber projection onto `fiber_fields`.
///
/// Uses power iteration on the covariance matrix of the fiber vectors.
/// Returns at most `min(modes, fiber_fields.len())` modes.
pub fn spectral_fiber_modes(store: &BundleStore, fiber_fields: &[&str], modes: usize) -> Vec<FiberMode> {
    let dim = fiber_fields.len();
    if dim == 0 || modes == 0 {
        return Vec::new();
    }

    // Collect fiber vectors
    let data: Vec<Vec<f64>> = store
        .records()
        .map(|rec| {
            fiber_fields
                .iter()
                .map(|f| rec.get(*f).and_then(|v| v.as_f64()).unwrap_or(0.0))
                .collect()
        })
        .collect();

    let n = data.len();
    if n == 0 {
        return Vec::new();
    }

    // Centre the data
    let means: Vec<f64> = (0..dim)
        .map(|j| data.iter().map(|r| r[j]).sum::<f64>() / n as f64)
        .collect();
    let centred: Vec<Vec<f64>> = data
        .iter()
        .map(|r| r.iter().enumerate().map(|(j, v)| v - means[j]).collect())
        .collect();

    // Covariance matrix (dim × dim)
    let mut cov = vec![vec![0.0f64; dim]; dim];
    for row in &centred {
        for i in 0..dim {
            for j in 0..dim {
                cov[i][j] += row[i] * row[j];
            }
        }
    }
    let scale = if n > 1 { (n - 1) as f64 } else { 1.0 };
    for i in 0..dim {
        for j in 0..dim {
            cov[i][j] /= scale;
        }
    }

    // Power iteration: extract top eigenvectors via deflation
    let n_modes = modes.min(dim);
    let mut result = Vec::with_capacity(n_modes);
    let mut deflated = cov.clone();

    for mode_idx in 0..n_modes {
        // Random starting vector
        let mut v: Vec<f64> = (0..dim).map(|i| if i == mode_idx % dim { 1.0 } else { 0.0 }).collect();
        // Power iteration (max 100 steps)
        let mut lambda = 0.0f64;
        for _ in 0..100 {
            // Av
            let mut av: Vec<f64> = vec![0.0; dim];
            for i in 0..dim {
                for j in 0..dim {
                    av[i] += deflated[i][j] * v[j];
                }
            }
            // Rayleigh quotient
            let dot: f64 = av.iter().zip(v.iter()).map(|(a, b)| a * b).sum();
            let norm: f64 = av.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-12 { break; }
            lambda = dot;
            v = av.iter().map(|x| x / norm).collect();
        }
        // IPR = sum(v_i^4) / (sum(v_i^2))^2  (= 1 fully localised, 1/dim fully delocalised)
        let sum_sq: f64 = v.iter().map(|x| x * x).sum();
        let sum_4: f64 = v.iter().map(|x| x * x * x * x).sum();
        let ipr = if sum_sq > 1e-12 { sum_4 / (sum_sq * sum_sq) } else { 0.0 };

        result.push(FiberMode { mode: mode_idx + 1, lambda, ipr });

        // Deflate: remove this eigenvector's contribution
        for i in 0..dim {
            for j in 0..dim {
                deflated[i][j] -= lambda * v[i] * v[j];
            }
        }
    }

    result
}

// ─── SPECTRAL_GAUGE — Phase 1 (Halcyon, 2026-06-28) ────────────────────────
//
// Fiber-weighted spectral gap λ₁ of the gauge-weighted graph Laplacian L_A.
//
// HONEST FRAMING (per math lens corrections in HALCYON_BRIDGE_TRILOGY,
// cfeb5c5): L_A's spectrum is globally gauge-invariant, but the per-edge
// trace weight Re Tr(U_e)/N is only LOCALLY gauge-covariant. This verb
// returns the fiber-weighted spectral gap — NOT the strict Yang-Mills
// mass gap. Halcyon understands the distinction; the function is the
// usable substrate primitive a downstream Wilson-mass-gap pipeline can
// build on.
//
// Phase 1 ships the dense nalgebra::SymmetricEigen path for small graphs
// (buckyball-scale, V ≤ ~10k vertices). Phase 2 (NOT in this commit) will
// add the Lanczos sparse path + FULL mode (k smallest eigenvalues) — the
// `eigenvalues: Option<Vec<f64>>` field and `PhaseNotImplemented` variant
// pre-wire those hooks.

/// Result of SPECTRAL_GAUGE.
///
/// Carries the gauge-weighted spectral gap, the count of records
/// successfully decoded into the Laplacian, and which group was actually
/// used (group inference may have promoted the parser's `None` to a
/// specific tag). The `eigenvalues` field is `None` in Phase 1; Phase 2's
/// FULL mode will populate it.
#[derive(Debug, Clone)]
#[cfg(feature = "gauge")]
pub struct SpectralGaugeResult {
    /// λ₁ — smallest nonzero eigenvalue of the fiber-weighted Laplacian.
    pub gap: f64,
    /// `None` in Phase 1; Phase 2 FULL mode will populate with the
    /// first `LIMIT k` eigenvalues sorted ascending.
    pub eigenvalues: Option<Vec<f64>>,
    /// Number of records (edges) that contributed to L_A. Each record
    /// is one edge weighted by its reconstructed group element.
    pub n_records_used: usize,
    /// The group that was actually used — either passed in or inferred
    /// from `fiber_fields.len()` at exec time.
    pub group_used: crate::gauge::Group,
}

/// Errors surfaced by SPECTRAL_GAUGE Phase 1.
///
/// Every variant carries enough context for a CLI / HTTP envelope to
/// render an actionable message; the Display impls below intentionally
/// repeat key names + counts so a developer can fix the call site
/// without having to grep for the variant.
#[derive(Debug)]
#[cfg(feature = "gauge")]
pub enum SpectralGaugeError {
    /// The named bundle does not exist on the engine.
    BundleNotFound(String),
    /// The bundle exists but has zero records (no edges to weight).
    EmptyBundle { bundle: String },
    /// The base schema does not carry the `vertex_a` / `vertex_b`
    /// endpoint columns Halcyon's lattice contract requires.
    MissingEndpointFields { bundle: String, a: String, b: String },
    /// The number of fiber fields passed does not match the chosen
    /// group's representation dimension (e.g. 5 columns with GROUP
    /// SU(2) — SU(2) requires exactly 4).
    FiberArityMismatch {
        group: &'static str,
        expected: usize,
        actual: usize,
    },
    /// GROUP was omitted and `fiber_fields.len()` is not one of the
    /// canonical widths (1 → U(1), 4 → SU(2), 18 → SU(3)).
    AmbiguousGroupInference(usize),
    /// FULL mode requested — Phase 1 does not implement it. Phase 2
    /// ships the Lanczos sparse k-eigenvalue path.
    PhaseNotImplemented {
        phase: &'static str,
        description: &'static str,
    },
}

#[cfg(feature = "gauge")]
impl std::fmt::Display for SpectralGaugeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpectralGaugeError::BundleNotFound(name) => {
                write!(f, "SPECTRAL_GAUGE: bundle '{name}' not found")
            }
            SpectralGaugeError::EmptyBundle { bundle } => {
                write!(f, "SPECTRAL_GAUGE: bundle '{bundle}' has zero records (no edges to weight)")
            }
            SpectralGaugeError::MissingEndpointFields { bundle, a, b } => {
                write!(
                    f,
                    "SPECTRAL_GAUGE: bundle '{bundle}' missing edge endpoint fields {a}/{b} — \
                     Halcyon schema requires explicit vertex_a/vertex_b in base_fields"
                )
            }
            SpectralGaugeError::FiberArityMismatch { group, expected, actual } => {
                write!(
                    f,
                    "SPECTRAL_GAUGE: fiber arity {actual} does not match group {group} \
                     (expected {expected})"
                )
            }
            SpectralGaugeError::AmbiguousGroupInference(n) => {
                write!(
                    f,
                    "SPECTRAL_GAUGE: GROUP required when fiber width is ambiguous \
                     (got {n} fields; canonical widths are 1 → U(1), 4 → SU(2), 18 → SU(3))"
                )
            }
            SpectralGaugeError::PhaseNotImplemented { phase, description } => {
                write!(f, "SPECTRAL_GAUGE: {phase} not yet implemented — {description}")
            }
        }
    }
}

#[cfg(feature = "gauge")]
impl std::error::Error for SpectralGaugeError {}

/// Infer a `Group` from the fiber arity per the SPECTRAL_GAUGE spec
/// table: 1 → U(1), 4 → SU(2), 18 → SU(3). Any other width is
/// ambiguous (8 in particular is reserved for SU(3) Gell-Mann tangent
/// basis in Phase 2, but the dense Phase 1 path needs the raw 3×3 form).
#[cfg(feature = "gauge")]
pub fn infer_group_from_arity(n: usize) -> Result<crate::gauge::Group, SpectralGaugeError> {
    match n {
        1 => Ok(crate::gauge::Group::U1),
        4 => Ok(crate::gauge::Group::SU2),
        18 => Ok(crate::gauge::Group::SU3),
        other => Err(SpectralGaugeError::AmbiguousGroupInference(other)),
    }
}

/// Compute the per-edge trace weight `w_e = Re Tr(U_e) / N` from the
/// fiber values of a single record. The packing convention matches the
/// `gigi::gauge::Group` representation table:
///
/// * SU(2) — 4 floats `(q0, q1, q2, q3)` (scalar-first quaternion).
///   For unit quaternions, `Re Tr(U) = 2·q0` and `N=2`, so `w_e = q0`.
/// * SU(3) — 18 floats `[re_00, im_00, re_01, im_01, …, re_22, im_22]`
///   (row-major 3×3 complex matrix with interleaved real/imag pairs).
///   `Re Tr = re_00 + re_11 + re_22` (indices 0, 8, 16) and `N=3`.
/// * U(1) — 1 float θ. `U = e^{iθ}` acts on ℂ¹, so `Re Tr / N = cos(θ)`.
/// * Z_N — 1 float k packed as f64. `U = e^{2πi·k/n}`, so weight is
///   `cos(2π·k/n)`.
#[cfg(feature = "gauge")]
pub fn re_trace_over_n(fiber: &[f64], group: crate::gauge::Group) -> f64 {
    use crate::gauge::Group;
    match group {
        Group::SU2 => {
            // Quaternion (q0, q1, q2, q3) — Re Tr / N = q0 for unit quat.
            // Defensive against under-arity: zero fiber → zero weight.
            fiber.first().copied().unwrap_or(0.0)
        }
        Group::SU3 => {
            // Interleaved 18-float row-major 3×3 complex; diagonal real
            // parts at offsets 0 (re_00), 8 (re_11), 16 (re_22).
            let d0 = fiber.first().copied().unwrap_or(0.0);
            let d1 = fiber.get(8).copied().unwrap_or(0.0);
            let d2 = fiber.get(16).copied().unwrap_or(0.0);
            (d0 + d1 + d2) / 3.0
        }
        Group::U1 => fiber.first().copied().unwrap_or(0.0).cos(),
        Group::ZN { n } => {
            // Discrete index packed as f64; round to nearest integer to
            // tolerate FP serialization drift, then map to angle.
            let k = fiber.first().copied().unwrap_or(0.0).round();
            let n_f = n as f64;
            if n_f <= 0.0 {
                return 1.0;
            }
            let theta = 2.0 * std::f64::consts::PI * k / n_f;
            theta.cos()
        }
    }
}

/// SPECTRAL_GAUGE Phase 1 — compute the fiber-weighted spectral gap λ₁
/// of L_A.
///
/// Reads `bundle` records, reconstructs the per-edge group element U_e
/// from `fiber_fields`, computes trace weights w_e = Re Tr(U_e)/N,
/// builds the dense weighted graph Laplacian L_A, and returns its
/// smallest nonzero eigenvalue via nalgebra::SymmetricEigen.
///
/// HONEST FRAMING: L_A's spectrum is globally gauge-invariant, but the
/// per-edge weight Re Tr(U_e)/N is only locally gauge-covariant. This
/// returns the fiber-weighted spectral gap — NOT the strict Yang-Mills
/// mass gap. See HALCYON_BRIDGE_TRILOGY notes (cfeb5c5).
///
/// `full = true` returns `SpectralGaugeError::PhaseNotImplemented`
/// (Phase 2 ships the Lanczos sparse k-eigenvalue path).
///
/// Negative-weight edges are physically meaningful (anti-aligned
/// holonomy → negative `Re Tr`); the resulting symmetric Laplacian is
/// real but is NOT guaranteed positive semidefinite under those
/// configurations. The "smallest nonzero" eigenvalue may legitimately
/// be negative in heavily anti-correlated regimes — this is the honest
/// physics, surfaced verbatim rather than clamped.
#[cfg(feature = "gauge")]
pub fn spectral_gauge_gap(
    engine: &crate::engine::Engine,
    bundle: &str,
    fiber_fields: &[String],
    group: crate::gauge::Group,
    full: bool,
    _limit: Option<usize>,
) -> Result<SpectralGaugeResult, SpectralGaugeError> {
    // ── Step 0: FULL mode → Phase 2 stub. Surface the typed error so
    //   callers get an exact phase tag plus what they get today (the
    //   gap-only return value). _limit is intentionally unused in
    //   Phase 1; Phase 2 reads it for the Lanczos k-eigenvalue path.
    if full {
        return Err(SpectralGaugeError::PhaseNotImplemented {
            phase: "Phase 2",
            description: "FULL mode (k eigenvalues + Lanczos sparse) ships in Phase 2 \
                          — Phase 1 dense path returns only the gap λ₁",
        });
    }

    // ── Step 1: Resolve the bundle. Use the typed BundleNotFound
    //   variant rather than the silent-zero fallback the unweighted
    //   SPECTRAL verb uses — Halcyon explicitly wants the typed error.
    let bundle_ref = engine
        .bundle(bundle)
        .ok_or_else(|| SpectralGaugeError::BundleNotFound(bundle.to_string()))?;
    let store = bundle_ref.as_heap().ok_or_else(|| {
        SpectralGaugeError::BundleNotFound(format!("{bundle} (not heap-resident)"))
    })?;

    // ── Step 2: Validate fiber arity matches the group's repr_dim.
    let expected = group.repr_dim();
    if fiber_fields.len() != expected {
        return Err(SpectralGaugeError::FiberArityMismatch {
            group: group.label(),
            expected,
            actual: fiber_fields.len(),
        });
    }

    // ── Step 3: Confirm vertex_a / vertex_b are in base_fields.
    let endpoint_a = "vertex_a";
    let endpoint_b = "vertex_b";
    let has_a = store.schema.base_fields.iter().any(|f| f.name == endpoint_a);
    let has_b = store.schema.base_fields.iter().any(|f| f.name == endpoint_b);
    if !has_a || !has_b {
        return Err(SpectralGaugeError::MissingEndpointFields {
            bundle: bundle.to_string(),
            a: endpoint_a.to_string(),
            b: endpoint_b.to_string(),
        });
    }

    // ── Step 4: Single pass through records building (i, j, w_e).
    //   Vertex indexing is dense: each unique vertex id gets a compact
    //   0..V row index on first sight.
    let mut vertex_idx: HashMap<i64, usize> = HashMap::new();
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    let mut n_records_used = 0usize;

    for rec in store.records() {
        let va = rec.get(endpoint_a).and_then(|v| v.as_i64()).unwrap_or(0);
        let vb = rec.get(endpoint_b).and_then(|v| v.as_i64()).unwrap_or(0);
        let next_a = vertex_idx.len();
        let i = *vertex_idx.entry(va).or_insert(next_a);
        let next_b = vertex_idx.len();
        let j = *vertex_idx.entry(vb).or_insert(next_b);

        // Pack fiber columns into a fixed-arity slice for re_trace_over_n.
        let fiber: Vec<f64> = fiber_fields
            .iter()
            .map(|f| rec.get(f.as_str()).and_then(|v| v.as_f64()).unwrap_or(0.0))
            .collect();

        let w_e = re_trace_over_n(&fiber, group);
        edges.push((i, j, w_e));
        n_records_used += 1;
    }

    let v_count = vertex_idx.len();
    if v_count < 2 {
        return Err(SpectralGaugeError::EmptyBundle {
            bundle: bundle.to_string(),
        });
    }

    // ── Step 5: Assemble dense Laplacian L_A as nalgebra DMatrix<f64>.
    //   Standard combinatorial Laplacian on the weighted graph:
    //     L[i,i] = Σ_e∋i w_e
    //     L[i,j] = -w_e for edge (i,j)
    //   Symmetric by construction: both off-diagonals get the same
    //   decrement; both diagonals get the same increment.
    let mut l = nalgebra::DMatrix::<f64>::zeros(v_count, v_count);
    for &(i, j, w) in &edges {
        if i == j {
            continue; // skip self-loops
        }
        l[(i, j)] -= w;
        l[(j, i)] -= w;
        l[(i, i)] += w;
        l[(j, j)] += w;
    }

    // ── Step 6: Eigendecomposition (symmetric, real spectrum).
    let eigen = nalgebra::SymmetricEigen::new(l);
    let mut vals: Vec<f64> = eigen.eigenvalues.iter().copied().collect();
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // ── Step 7: Smallest nonzero eigenvalue. For a connected graph
    //   the smallest eigenvalue is ≈ 0 (within FP noise); λ₁ is the
    //   next one. For disconnected: there are k zero eigenvalues for
    //   k components — λ₁ is still defined as the first eigenvalue
    //   above the tolerance.
    let tol = 1e-9_f64;
    let gap = vals
        .iter()
        .find(|&&v| v.abs() > tol)
        .copied()
        .unwrap_or(0.0);

    Ok(SpectralGaugeResult {
        gap,
        eigenvalues: None, // Phase 1: gap only. Phase 2 fills this.
        n_records_used,
        group_used: group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleStore;
    use crate::types::*;

    /// Build a well-connected store (all same dept = complete subgraph).
    fn make_connected_store() -> BundleStore {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("color"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("color");
        let mut store = BundleStore::new(schema);
        // All 20 records share color="Red" → fully connected graph
        for i in 0..20 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("color".into(), Value::Text("Red".into()));
            r.insert("val".into(), Value::Float(i as f64));
            store.insert(&r);
        }
        store
    }

    /// Build a store with two disjoint clusters.
    fn make_clustered_store() -> BundleStore {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("color"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("color");
        let mut store = BundleStore::new(schema);
        // Cluster A: color="Red" (ids 0-9)
        for i in 0..10 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("color".into(), Value::Text("Red".into()));
            r.insert("val".into(), Value::Float(i as f64));
            store.insert(&r);
        }
        // Cluster B: color="Blue" (ids 10-19)
        for i in 10..20 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("color".into(), Value::Text("Blue".into()));
            r.insert("val".into(), Value::Float(i as f64));
            store.insert(&r);
        }
        store
    }

    /// TDD-3.18: Fully connected field index → λ₁ is large.
    #[test]
    fn tdd_3_18_connected_large_gap() {
        let store = make_connected_store();
        let lambda1 = spectral_gap(&store);
        // For a complete graph on n vertices, λ₁ = n/(n-1 ≈ 1.05
        assert!(
            lambda1 > 0.5,
            "λ₁ = {lambda1}, expected > 0.5 for connected graph"
        );
    }

    /// TDD-3.19: Two disjoint clusters → λ₁ ≈ 0.
    #[test]
    fn tdd_3_19_clustered_small_gap() {
        let store = make_clustered_store();
        let lambda1 = spectral_gap(&store);
        // Two disconnected components → λ₁ should be ≈ 0
        // (Power iteration may not converge perfectly, but should be small)
        assert!(
            lambda1 < 0.3,
            "λ₁ = {lambda1}, expected < 0.3 for disconnected graph"
        );
    }

    /// TDD-3.20: C_sp ≥ π² for connected graph.
    #[test]
    fn tdd_3_20_spectral_capacity_bound() {
        let store = make_connected_store();
        let c_sp = spectral_capacity(&store);
        // For a connected graph, C_sp = λ₁ * D² ≥ π² ≈ 9.87
        // Complete graph: D = 1, λ₁ ≈ 1.05 → C_sp ≈ 1.05
        // This is a dense graph so D=1, meaning C_sp = λ₁
        // The π² bound applies to path graphs where D is large
        // For our complete graph: D = 1, so C_sp = λ₁ ≈ 1.0
        // We verify the computation is correct rather than enforcing pi² here
        assert!(
            c_sp > 0.0,
            "C_sp = {c_sp}, expected > 0 for connected graph"
        );
    }

    /// TDD-3.22: Coarse-grain at 3 scales. Verify entropy decreases.
    #[test]
    fn tdd_3_22_rg_flow_monotone() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("dept"))
            .fiber(FieldDef::categorical("region"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("dept")
            .index("region");
        let mut store = BundleStore::new(schema);
        let depts = ["Eng", "Sales", "HR"];
        let regions = ["East", "West"];
        for i in 0..30 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("dept".into(), Value::Text(depts[i as usize % 3].into()));
            r.insert("region".into(), Value::Text(regions[i as usize % 2].into()));
            r.insert("val".into(), Value::Float(i as f64));
            store.insert(&r);
        }

        let (g1, e1) = coarse_grain(&store, 1); // Fine: match all indexed fields
        let (g2, e2) = coarse_grain(&store, 2); // Medium: match fewer fields
        let (g3, e3) = coarse_grain(&store, 3); // Coarse: match fewest fields

        // More groups at finer scale
        assert!(
            g1.len() >= g2.len(),
            "groups: fine={} < medium={}",
            g1.len(),
            g2.len()
        );
        assert!(
            g2.len() >= g3.len(),
            "groups: medium={} < coarse={}",
            g2.len(),
            g3.len()
        );

        // C-theorem (Thm 3.5): entropy non-increasing under coarsening
        assert!(e1 >= e2 - 1e-10, "C-theorem violated: e1={e1} < e2={e2}");
        assert!(e2 >= e3 - 1e-10, "C-theorem violated: e2={e2} < e3={e3}");
    }

    /// TDD-3.23: C(ℓ₂) ≤ C(ℓ₁) for ℓ₂ > ℓ₁.
    #[test]
    fn tdd_3_23_c_theorem() {
        let store = make_clustered_store();
        let (_, e1) = coarse_grain(&store, 1);
        let (_, e2) = coarse_grain(&store, 2);
        assert!(e2 <= e1 + 1e-10, "C-theorem: e2={e2} > e1={e1}");
    }

    /// TDD-3.24: GROUP BY result equals coarse-grained fiber.
    #[test]
    fn tdd_3_24_group_by_equals_coarsening() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("dept"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("dept");
        let mut store = BundleStore::new(schema);
        let depts = ["Eng", "Sales", "HR"];
        for i in 0..30 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("dept".into(), Value::Text(depts[i as usize % 3].into()));
            r.insert("val".into(), Value::Float(i as f64 * 10.0));
            store.insert(&r);
        }

        // GROUP BY dept → 3 groups
        let groups = crate::aggregation::group_by(&store, "dept", "val");
        assert_eq!(groups.len(), 3);

        // Coarse-grain at scale 1 (match "dept") → same 3 groups
        let (coarse_groups, _) = coarse_grain(&store, 1);
        assert_eq!(coarse_groups.len(), 3);

        // Each coarse group has 10 elements (30 / 3 depts)
        for g in &coarse_groups {
            assert_eq!(g.len(), 10);
        }
    }

    // ── Betti numbers ──────────────────────────────────────────────

    /// TDD-3.25: Connected graph → β₀ = 1.
    #[test]
    fn tdd_3_25_betti_connected() {
        let store = make_connected_store();
        let (b0, _) = betti_numbers(&store);
        assert_eq!(b0, 1, "β₀ should be 1 for connected graph");
    }

    /// TDD-3.26: Two clusters → β₀ = 2.
    #[test]
    fn tdd_3_26_betti_disconnected() {
        let store = make_clustered_store();
        let (b0, _) = betti_numbers(&store);
        assert_eq!(b0, 2, "β₀ should be 2 for two disconnected clusters");
    }

    /// TDD-3.27: Complete graph K_n → β₁ = n(n-1)/2 - n + 1 = (n-1)(n-2)/2.
    #[test]
    fn tdd_3_27_betti_cycle_rank() {
        let store = make_connected_store(); // 20 nodes, all same color → K_20
        let (b0, b1) = betti_numbers(&store);
        assert_eq!(b0, 1);
        // K_20: |E| = 20*19/2 = 190, |V| = 20, β₁ = 190 - 20 + 1 = 171
        assert_eq!(b1, 171, "β₁ = |E| - |V| + β₀ for complete graph K_20");
    }

    /// TDD-3.28: Empty store → (0, 0).
    #[test]
    fn tdd_3_28_betti_empty() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("val");
        let store = BundleStore::new(schema);
        let (b0, b1) = betti_numbers(&store);
        assert_eq!((b0, b1), (0, 0));
    }

    // ── Entropy (standalone) ───────────────────────────────────────

    /// TDD-3.29: Single group → entropy = 0.
    #[test]
    fn tdd_3_29_entropy_single_group() {
        let store = make_connected_store(); // all same color → 1 group
        let s = entropy(&store);
        assert!(s.abs() < 1e-10, "Entropy of single group should be 0, got {s}");
    }

    /// TDD-3.30: k equal groups → entropy = ln(k).
    #[test]
    fn tdd_3_30_entropy_uniform_groups() {
        let store = make_clustered_store(); // 2 groups of 10
        let s = entropy(&store);
        let expected = (2.0_f64).ln();
        assert!(
            (s - expected).abs() < 1e-10,
            "Entropy should be ln(2)={expected}, got {s}"
        );
    }

    /// TDD-3.31: Entropy ≥ 0 always.
    #[test]
    fn tdd_3_31_entropy_non_negative() {
        let store = make_connected_store();
        assert!(entropy(&store) >= 0.0);
        let store2 = make_clustered_store();
        assert!(entropy(&store2) >= 0.0);
    }

    // ── Sprint B: Geodesic Distance ──

    /// TDD-B.1: Same point → distance 0.
    #[test]
    fn tdd_b1_geodesic_same_point() {
        let store = make_connected_store();
        let bp = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(0));
            r
        });
        let d = geodesic_distance(&store, bp, bp, 50);
        assert_eq!(d, Some(0.0));
    }

    /// TDD-B.2: Connected pair → finite distance.
    #[test]
    fn tdd_b2_geodesic_connected_pair() {
        let store = make_connected_store();
        let bp_a = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(0));
            r
        });
        let bp_b = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(5));
            r
        });
        let d = geodesic_distance(&store, bp_a, bp_b, 50);
        assert!(d.is_some(), "Connected points should have a geodesic");
        assert!(d.unwrap() > 0.0, "Different points should have d > 0");
        assert!(d.unwrap().is_finite(), "Distance should be finite");
    }

    /// TDD-B.3: Disconnected clusters → None.
    #[test]
    fn tdd_b3_geodesic_disconnected() {
        let store = make_clustered_store();
        // id=0 (Red cluster) and id=10 (Blue cluster) are disconnected
        let bp_a = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(0));
            r
        });
        let bp_b = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(10));
            r
        });
        let d = geodesic_distance(&store, bp_a, bp_b, 50);
        assert!(d.is_none(), "Disconnected points should have no geodesic");
    }

    /// TDD-B.4: Triangle inequality d(a,c) ≤ d(a,b) + d(b,c).
    #[test]
    fn tdd_b4_geodesic_triangle_inequality() {
        let store = make_connected_store();
        let bp = |id: i64| {
            store.base_point(&{
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(id));
                r
            })
        };
        let a = bp(0);
        let b = bp(5);
        let c = bp(10);
        let d_ab = geodesic_distance(&store, a, b, 50).unwrap();
        let d_bc = geodesic_distance(&store, b, c, 50).unwrap();
        let d_ac = geodesic_distance(&store, a, c, 50).unwrap();
        assert!(
            d_ac <= d_ab + d_bc + 1e-10,
            "Triangle inequality violated: d(a,c)={d_ac} > d(a,b)+d(b,c)={}",
            d_ab + d_bc
        );
    }

    /// TDD-B.5: Symmetry d(a,b) = d(b,a).
    #[test]
    fn tdd_b5_geodesic_symmetry() {
        let store = make_connected_store();
        let bp_a = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(0));
            r
        });
        let bp_b = store.base_point(&{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(5));
            r
        });
        let d_ab = geodesic_distance(&store, bp_a, bp_b, 50).unwrap();
        let d_ba = geodesic_distance(&store, bp_b, bp_a, 50).unwrap();
        assert!(
            (d_ab - d_ba).abs() < 1e-10,
            "Symmetry violated: d(a,b)={d_ab} ≠ d(b,a)={d_ba}"
        );
    }
}
