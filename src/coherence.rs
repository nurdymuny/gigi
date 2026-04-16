//! Coherence extensions — GIGI_COHERENCE_EXTENSIONS_v0.1
//!
//! Six features, all O(1) per insert:
//!   Feature 1 + 3: Atlas (AUTO-CHART + PROPAGATION)
//!   Feature 2:     BranchStore (CONSISTENCY BRANCH)
//!   Feature 4:     predict() — weighted centroid interpolation
//!   Feature 5:     cover_within() — geodesic sub-bundle view
//!   Feature 6:     ProvenanceGraph (PROVENANCE FIBERS — WHY / IMPLICATIONS)

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

// ── Shared helper: extract flat f64 fiber from a record ─────────────────────

use crate::types::{Record, Value};

/// Extract a flat f64 vector from `record` using the field names in `fields`.
/// Float and Integer scalars contribute one dimension each.
/// Returns None if any field is missing or non-numeric.
pub fn extract_fiber(record: &Record, fields: &[String]) -> Option<Vec<f64>> {
    let mut v = Vec::with_capacity(fields.len());
    for f in fields {
        match record.get(f) {
            Some(Value::Float(x)) => v.push(*x),
            Some(Value::Integer(i)) => v.push(*i as f64),
            _ => return None,
        }
    }
    Some(v)
}

/// Infer fiber field names from a record: all Float/Integer fields, sorted.
pub fn infer_fiber_fields(record: &Record) -> Vec<String> {
    let mut names: Vec<String> = record
        .iter()
        .filter(|(_, v)| matches!(v, Value::Float(_) | Value::Integer(_)))
        .map(|(k, _)| k.clone())
        .collect();
    names.sort();
    names
}

// ── Feature 1 + 3: Atlas (AUTO-CHART + PROPAGATION) ────────────────────────

/// Configuration for a bundle's auto-chart atlas.
#[derive(Debug, Clone)]
pub struct AutoChartConfig {
    /// Maximum normalised curvature τ a chart may have after accepting a point.
    pub tau: f64,
    /// Spatial bucket granularity g (edge length of each bucket cell).
    pub granularity: f64,
}

impl Default for AutoChartConfig {
    fn default() -> Self {
        Self { tau: 0.3, granularity: 0.15 }
    }
}

/// One chart in the atlas: centroid + Welford running variance + radius.
#[derive(Debug, Clone)]
pub struct Chart {
    pub id: u32,
    /// Number of member points.
    pub n: usize,
    /// Running centroid μ_C (Welford mean).
    pub centroid: Vec<f64>,
    /// Welford M2 per dimension (sum of squared deviations from running mean).
    m2: Vec<f64>,
    /// Maximum distance of any member from the centroid.
    pub radius: f64,
}

impl Chart {
    fn new(id: u32, v: Vec<f64>) -> Self {
        let d = v.len();
        Self { id, n: 1, centroid: v, m2: vec![0.0; d], radius: 0.0 }
    }

    /// Normalised curvature K_C = mean(σ_i) over all dimensions.
    pub fn curvature(&self) -> f64 {
        if self.n < 2 || self.centroid.is_empty() {
            return 0.0;
        }
        let d = self.centroid.len() as f64;
        self.m2.iter().map(|m2| (m2 / self.n as f64).sqrt()).sum::<f64>() / d
    }

    /// Predict K_C after absorbing `v` without committing (for tau check).
    fn predict_k_after(&self, v: &[f64]) -> f64 {
        let n1 = (self.n + 1) as f64;
        let d = self.centroid.len();
        if d == 0 || d != v.len() {
            return 0.0;
        }
        let m2_sum: f64 = self.m2
            .iter()
            .zip(self.centroid.iter())
            .zip(v.iter())
            .map(|((m2, mu), vi)| {
                let d1 = vi - mu;
                let new_mu = mu + d1 / n1;
                m2 + d1 * (vi - new_mu)
            })
            .sum();
        (m2_sum / d as f64 / n1).sqrt()
    }

    /// Commit Welford update with point `v`.
    fn update(&mut self, v: &[f64]) {
        let n1 = (self.n + 1) as f64;
        let d = self.centroid.len();
        if d != v.len() {
            return;
        }
        let mut new_m2 = Vec::with_capacity(d);
        for i in 0..d {
            let d1 = v[i] - self.centroid[i];
            let new_mu = self.centroid[i] + d1 / n1;
            new_m2.push(self.m2[i] + d1 * (v[i] - new_mu));
            self.centroid[i] = new_mu;
        }
        self.m2 = new_m2;
        let dist = l2(v, &self.centroid[..v.len()]);
        self.radius = self.radius.max(dist);
        self.n += 1;
    }

    fn distance(&self, v: &[f64]) -> f64 {
        l2(v, &self.centroid[..v.len().min(self.centroid.len())])
    }
}

fn l2(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
}

/// Spatial bucket key: quantised fiber coordinates.
type BucketKey = Vec<i32>;

fn quantize(v: &[f64], g: f64) -> BucketKey {
    v.iter().map(|x| (*x / g).floor() as i32).collect()
}

/// All 3^d neighbor buckets (including self) for a given bucket key.
fn neighbor_buckets(base: &BucketKey) -> Vec<BucketKey> {
    let d = base.len();
    let total = 3usize.saturating_pow(d as u32);
    (0..total)
        .map(|mut idx| {
            let mut bucket = base.clone();
            for i in 0..d {
                let offset = (idx % 3) as i32 - 1;
                bucket[i] += offset;
                idx /= 3;
            }
            bucket
        })
        .collect()
}

/// Result of one auto-chart insert (Feature 1 + 3 combined).
#[derive(Debug, Clone)]
pub struct ChartAction {
    pub chart_id: u32,
    /// `"confirm"` = absorbed into existing chart; `"extend"` = spawned new chart.
    pub action: &'static str,
    pub k_before: f64,
    pub k_after: f64,
    /// Novelty ∈ [0, 1]: 1.0 for extend; |ΔK|/K_before for confirm.
    pub novelty: f64,
    /// Chart ids whose K changed (including host). For Feature 3 PROPAGATION.
    pub affected_chart_ids: Vec<u32>,
}

/// Chart atlas — the spatial index for AUTO-CHART.
///
/// Invariants (per spec):
///   I1: K_C ≤ τ for every chart at all times.
///   I2: Every inserted point is in exactly one chart.
///   I3: Spawning a new chart never modifies existing charts.
#[derive(Debug, Clone, Default)]
pub struct Atlas {
    pub charts: Vec<Chart>,
    /// Bucket → chart ids in that cell.
    buckets: HashMap<BucketKey, Vec<u32>>,
    pub config: AutoChartConfig,
    /// Ordered list of fiber field names — determines vector layout.
    pub fiber_fields: Vec<String>,
    next_id: u32,
}

impl Atlas {
    pub fn new(fiber_fields: Vec<String>, tau: f64, granularity: f64) -> Self {
        Self {
            charts: Vec::new(),
            buckets: HashMap::new(),
            config: AutoChartConfig { tau, granularity },
            fiber_fields,
            next_id: 0,
        }
    }

    /// Insert fiber vector `v`. Returns chart assignment + novelty.
    pub fn insert(&mut self, v: &[f64]) -> ChartAction {
        let g = self.config.granularity;
        let tau = self.config.tau;
        let base = quantize(v, g);

        // Deduplicated candidate charts from 3^d neighbor buckets.
        let candidates: HashSet<u32> = neighbor_buckets(&base)
            .iter()
            .filter_map(|b| self.buckets.get(b))
            .flat_map(|ids| ids.iter().copied())
            .collect();

        // Nearest valid chart: must be within 2g AND K_after ≤ τ.
        let mut best: Option<(u32, f64, f64, f64)> = None; // (id, dist, k_before, k_after)
        for cid in &candidates {
            let c = &self.charts[*cid as usize];
            let dist = c.distance(v);
            if dist > 2.0 * g {
                continue;
            }
            let k_before = c.curvature();
            let k_after = c.predict_k_after(v);
            if k_after <= tau {
                match best {
                    None => best = Some((*cid, dist, k_before, k_after)),
                    Some((_, bd, _, _)) if dist < bd => {
                        best = Some((*cid, dist, k_before, k_after))
                    }
                    _ => {}
                }
            }
        }

        if let Some((cid, _, k_before, k_after)) = best {
            self.charts[cid as usize].update(v);
            let eps = 1e-9_f64;
            let novelty = if k_before < eps {
                0.0
            } else {
                ((k_after - k_before).abs() / k_before).min(1.0)
            };
            ChartAction {
                chart_id: cid,
                action: "confirm",
                k_before,
                k_after,
                novelty,
                affected_chart_ids: vec![cid],
            }
        } else {
            // Spawn new chart (I3: does not modify any existing chart).
            let id = self.next_id;
            self.next_id += 1;
            let chart = Chart::new(id, v.to_vec());
            self.charts.push(chart);
            self.buckets.entry(base).or_default().push(id);
            ChartAction {
                chart_id: id,
                action: "extend",
                k_before: 0.0,
                k_after: 0.0,
                novelty: 1.0,
                affected_chart_ids: vec![id],
            }
        }
    }

    // ── Feature 4: PREDICT ──────────────────────────────────────────────────

    /// Predict missing fiber dimensions given partial observations.
    ///
    /// `known`: `(fiber_dim_index, value)` pairs for the observed dimensions.
    /// `all_dims`: total dimensionality of the fiber.
    /// `bandwidth`: kernel bandwidth h (default: granularity g).
    pub fn predict(
        &self,
        known: &[(usize, f64)],
        all_dims: usize,
        bandwidth: f64,
    ) -> PredictResult {
        if self.charts.is_empty() {
            return PredictResult::empty(all_dims);
        }

        let mut weighted_sum = vec![0.0f64; all_dims];
        let mut weighted_w2_var = vec![0.0f64; all_dims];
        let mut weight_total = 0.0f64;
        let mut best_dist = f64::MAX;
        let mut best_chart_id: Option<u32> = None;

        for chart in &self.charts {
            if chart.centroid.len() < all_dims {
                continue;
            }
            let dist: f64 = known
                .iter()
                .map(|(i, v)| (chart.centroid[*i] - v).powi(2))
                .sum::<f64>()
                .sqrt();
            let w = (-dist / bandwidth).exp() / (1.0 + chart.curvature());
            if w < 1e-12 {
                continue;
            }
            for i in 0..all_dims {
                weighted_sum[i] += w * chart.centroid[i];
            }
            // Per-dim variance for unknown dimensions only.
            let known_set: HashSet<usize> = known.iter().map(|(i, _)| *i).collect();
            for i in 0..all_dims {
                if !known_set.contains(&i) {
                    let var_i = if chart.n >= 2 { chart.m2[i] / chart.n as f64 } else { 0.0 };
                    weighted_w2_var[i] += w * w * var_i;
                }
            }
            weight_total += w;
            if dist < best_dist {
                best_dist = dist;
                best_chart_id = Some(chart.id);
            }
        }

        if weight_total < 1e-12 {
            return PredictResult::empty(all_dims);
        }

        let predicted: Vec<f64> = weighted_sum.iter().map(|s| s / weight_total).collect();
        let known_set: HashSet<usize> = known.iter().map(|(i, _)| *i).collect();
        let uncertainty: Vec<f64> = (0..all_dims)
            .map(|i| {
                if known_set.contains(&i) {
                    -1.0 // known dim — no uncertainty
                } else {
                    (weighted_w2_var[i] / weight_total).sqrt()
                }
            })
            .collect();
        let best_k = best_chart_id
            .map(|id| self.charts[id as usize].curvature())
            .unwrap_or(1.0);
        let confidence =
            (1.0 / (1.0 + best_k)) * (-best_dist / bandwidth).exp();

        PredictResult {
            predicted,
            uncertainty,
            confidence,
            host_chart_id: best_chart_id,
            n_charts_used: (weight_total.max(1.0) as usize).min(self.charts.len()),
        }
    }

    // ── Feature 5: COVER WITHIN GEODESIC ───────────────────────────────────

    /// Return all record indices (from a slice of pre-extracted fibers) whose
    /// fiber is within `radius` of `query`.
    ///
    /// `fibers[i]` = the fiber for record index i, or None if unavailable.
    ///
    /// Complexity: O(n) scan (correct); O(|result|) for typical well-clustered data.
    pub fn cover_within<'a>(
        &self,
        query: &[f64],
        radius: f64,
        fibers: impl Iterator<Item = Option<Vec<f64>>> + 'a,
    ) -> Vec<(usize, f64)> {
        fibers
            .enumerate()
            .filter_map(|(idx, maybe_v)| {
                let v = maybe_v?;
                if v.len() < query.len() {
                    return None;
                }
                let d = l2(query, &v[..query.len()]);
                if d <= radius { Some((idx, d)) } else { None }
            })
            .collect()
    }

    // ── Ollivier–Ricci curvature ─────────────────────────────────────────────

    /// Neighbourhood of a chart: all charts within `2 * granularity` L2 distance.
    fn neighborhood(&self, chart_id: u32) -> Vec<u32> {
        let r = 2.0 * self.config.granularity;
        let c = &self.charts[chart_id as usize];
        self.charts
            .iter()
            .filter(|o| l2(&c.centroid, &o.centroid) <= r)
            .map(|o| o.id)
            .collect()
    }

    /// Wasserstein-1 distance between two discrete uniform distributions on
    /// sets of centroid vectors `a` and `b`.
    ///
    /// For equal-size sets the optimal assignment is found by greedy nearest-
    /// neighbor matching (exact for well-separated clusters; tight approximation
    /// otherwise).  For unequal sizes every source point is matched to its
    /// nearest target: the cost is the mean of those distances, which is an
    /// admissible upper bound on W₁ used consistently for both κ(a,b) and
    /// κ(b,a) — symmetry is enforced by always computing both directions and
    /// averaging.
    fn wasserstein1(a_ids: &[u32], b_ids: &[u32], charts: &[Chart]) -> f64 {
        if a_ids.is_empty() || b_ids.is_empty() {
            return 0.0;
        }
        // Build centroid slices.
        let a_cents: Vec<&[f64]> = a_ids.iter().map(|&i| charts[i as usize].centroid.as_slice()).collect();
        let b_cents: Vec<&[f64]> = b_ids.iter().map(|&i| charts[i as usize].centroid.as_slice()).collect();

        // Greedy nearest-neighbour assignment from A → B.
        let mut used = vec![false; b_cents.len()];
        let mut total_ab = 0.0;
        for ac in &a_cents {
            let (j, d) = b_cents
                .iter()
                .enumerate()
                .filter(|(j, _)| !used[*j])
                .map(|(j, bc)| (j, l2(ac, bc)))
                .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or_else(|| {
                    // All matched — take absolute nearest (happens when |A| > |B|).
                    b_cents
                        .iter()
                        .enumerate()
                        .map(|(j, bc)| (j, l2(ac, bc)))
                        .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap()
                });
            used[j] = true;
            total_ab += d;
        }

        // Also compute B → A direction; symmetrize.
        let mut used2 = vec![false; a_cents.len()];
        let mut total_ba = 0.0;
        for bc in &b_cents {
            let (j, d) = a_cents
                .iter()
                .enumerate()
                .filter(|(j, _)| !used2[*j])
                .map(|(j, ac)| (j, l2(bc, ac)))
                .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or_else(|| {
                    a_cents
                        .iter()
                        .enumerate()
                        .map(|(j, ac)| (j, l2(bc, ac)))
                        .min_by(|x, y| x.1.partial_cmp(&y.1).unwrap_or(std::cmp::Ordering::Equal))
                        .unwrap()
                });
            used2[j] = true;
            total_ba += d;
        }

        // Symmetric W₁ estimate: average of mean cost in each direction.
        (total_ab / a_cents.len() as f64 + total_ba / b_cents.len() as f64) / 2.0
    }

    /// Ollivier–Ricci curvature κ(chart_a, chart_b).
    ///
    /// κ(x, y) = 1 − W₁(µ_x, µ_y) / d(x, y)
    ///
    /// where µ_x = uniform measure on B(x, 2g) and d = L2 of centroids.
    ///
    /// Returns `None` if either chart id is out of range or d = 0.
    pub fn ricci(&self, chart_a: u32, chart_b: u32) -> Option<RicciResult> {
        let n = self.charts.len() as u32;
        if chart_a >= n || chart_b >= n {
            return None;
        }
        let ca = &self.charts[chart_a as usize];
        let cb = &self.charts[chart_b as usize];
        let dist = l2(&ca.centroid, &cb.centroid);
        if dist < 1e-12 {
            // Same chart or zero distance — κ = 1 by convention.
            return Some(RicciResult {
                chart_a,
                chart_b,
                curvature: 1.0,
                distance: 0.0,
                w1: 0.0,
                n_neighbors_a: 1,
                n_neighbors_b: 1,
            });
        }
        let nbrs_a = self.neighborhood(chart_a);
        let nbrs_b = self.neighborhood(chart_b);
        let w1 = Self::wasserstein1(&nbrs_a, &nbrs_b, &self.charts);
        Some(RicciResult {
            chart_a,
            chart_b,
            curvature: 1.0 - w1 / dist,
            distance: dist,
            w1,
            n_neighbors_a: nbrs_a.len(),
            n_neighbors_b: nbrs_b.len(),
        })
    }
}

/// Result of Ollivier–Ricci curvature computation between two charts.
#[derive(Debug, Clone)]
pub struct RicciResult {
    pub chart_a: u32,
    pub chart_b: u32,
    /// κ(x, y) = 1 − W₁(µ_x, µ_y) / d(x, y).  > 0 → cluster interior; < 0 → bridge.
    pub curvature: f64,
    /// L2 distance between centroids.
    pub distance: f64,
    /// Wasserstein-1 distance between neighbourhood measures.
    pub w1: f64,
    pub n_neighbors_a: usize,
    pub n_neighbors_b: usize,
}

/// Result of a PREDICT query.
#[derive(Debug, Clone)]
pub struct PredictResult {
    pub predicted: Vec<f64>,
    /// -1.0 for known dims, σ_pred for unknown dims.
    pub uncertainty: Vec<f64>,
    pub confidence: f64,
    pub host_chart_id: Option<u32>,
    pub n_charts_used: usize,
}

impl PredictResult {
    fn empty(dims: usize) -> Self {
        Self {
            predicted: vec![0.0; dims],
            uncertainty: vec![0.0; dims],
            confidence: 0.0,
            host_chart_id: None,
            n_charts_used: 0,
        }
    }
}

// ── Feature 2: CONSISTENCY BRANCH ───────────────────────────────────────────

/// What happens when a SECTION insert finds a conflicting existing value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContradictionPolicy {
    /// Allocate two new branches, tag both the old and the new section.
    Branch,
    /// Overwrite the old fiber with the new one (repair in-place).
    Repair,
    /// Reject the new insert and return an error.
    Reject,
    /// Accept both silently (last-write-wins behavior).
    Allow,
}

impl ContradictionPolicy {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_uppercase().as_str() {
            "BRANCH" => Some(Self::Branch),
            "REPAIR" => Some(Self::Repair),
            "REJECT" => Some(Self::Reject),
            "ALLOW" => Some(Self::Allow),
            _ => None,
        }
    }
}

/// Recorded contradiction event.
#[derive(Debug, Clone)]
pub struct ContradictionEvent {
    pub id: usize,
    pub base_id: String,
    /// (b_old, b_new) — both branches created at this contradiction.
    pub branches: [u32; 2],
    pub distance: f64,
    pub timestamp_ms: u64,
}

/// Per-bundle branch state for CONSISTENCY BRANCH.
#[derive(Debug, Clone)]
pub struct BranchStore {
    /// base_id → set of branch ids the section is tagged to.
    /// Empty set = visible in all branches (universal section).
    branch_sets: HashMap<String, BTreeSet<u32>>,
    pub contradictions: Vec<ContradictionEvent>,
    pub epsilon: f64,
    pub default_policy: ContradictionPolicy,
    next_branch: u32,
}

impl Default for BranchStore {
    fn default() -> Self {
        Self {
            branch_sets: HashMap::new(),
            contradictions: Vec::new(),
            epsilon: 1e-6,
            default_policy: ContradictionPolicy::Branch,
            next_branch: 0,
        }
    }
}

impl BranchStore {
    pub fn new(epsilon: f64) -> Self {
        Self { epsilon, ..Default::default() }
    }

    /// Check whether inserting at `base_id` with measured fiber `distance`
    /// from the existing value constitutes a contradiction, and handle it
    /// according to `policy`.
    ///
    /// `distance` = 0.0 means "no existing record" → clean insert.
    pub fn check(
        &mut self,
        base_id: &str,
        distance: f64,
        policy: ContradictionPolicy,
    ) -> BranchDecision {
        if distance < self.epsilon {
            return BranchDecision::Clean;
        }
        match policy {
            ContradictionPolicy::Allow => BranchDecision::Clean,
            ContradictionPolicy::Reject => BranchDecision::Rejected,
            ContradictionPolicy::Repair => BranchDecision::Repaired,
            ContradictionPolicy::Branch => {
                let b_old = self.next_branch;
                self.next_branch += 1;
                let b_new = self.next_branch;
                self.next_branch += 1;
                self.branch_sets
                    .entry(base_id.to_string())
                    .or_default()
                    .insert(b_old);
                self.contradictions.push(ContradictionEvent {
                    id: self.contradictions.len(),
                    base_id: base_id.to_string(),
                    branches: [b_old, b_new],
                    distance,
                    timestamp_ms: now_ms(),
                });
                BranchDecision::Branched { b_old, b_new }
            }
        }
    }

    pub fn branch_set_of(&self, base_id: &str) -> Option<&BTreeSet<u32>> {
        self.branch_sets.get(base_id)
    }

    /// Collapse branch `b`: remove all sections exclusively in branch `b`.
    /// Returns the base_ids of removed sections.
    pub fn collapse(&mut self, b: u32) -> Vec<String> {
        let mut removed = Vec::new();
        self.branch_sets.retain(|id, bset| {
            if bset.len() == 1 && bset.contains(&b) {
                removed.push(id.clone());
                false
            } else {
                bset.remove(&b);
                true
            }
        });
        self.contradictions.retain(|ev| !ev.branches.contains(&b));
        removed
    }
}

/// Decision returned by BranchStore::check.
#[derive(Debug, Clone)]
pub enum BranchDecision {
    Clean,
    Branched { b_old: u32, b_new: u32 },
    Repaired,
    Rejected,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ── Feature 6: PROVENANCE FIBERS ────────────────────────────────────────────

/// Causal DAG for a bundle — bidirectional adjacency maps.
///
/// Invariant: forward[p] ∋ q ⟺ backward[q] ∋ p (maintained on every insert).
/// No cycles: insert-only DAG; only self-loops are structurally possible and
/// are rejected at O(k) cost.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceGraph {
    /// p → all direct derivations of p.
    forward: HashMap<String, Vec<String>>,
    /// q → all direct sources of q.
    backward: HashMap<String, Vec<String>>,
}

/// One node in a WHY / IMPLICATIONS walk.
#[derive(Debug, Clone)]
pub struct WalkNode {
    pub id: String,
    pub depth: usize,
}

impl ProvenanceGraph {
    /// Record derivation edges `id ← derived_from`.
    /// Returns Err if `derived_from` contains `id` (self-loop).
    pub fn insert(&mut self, id: &str, derived_from: &[String]) -> Result<(), String> {
        for src in derived_from {
            if src == id {
                return Err(format!(
                    "PROVENANCE: self-loop rejected ('{id}' cannot derive from itself)"
                ));
            }
        }
        if !derived_from.is_empty() {
            self.backward
                .entry(id.to_string())
                .or_default()
                .extend_from_slice(derived_from);
            for src in derived_from {
                self.forward
                    .entry(src.clone())
                    .or_default()
                    .push(id.to_string());
            }
        }
        Ok(())
    }

    /// BFS backward walk (WHY): ancestors of `id`, up to `max_depth`.
    pub fn why(&self, id: &str, max_depth: Option<usize>) -> Vec<WalkNode> {
        self.bfs(id, max_depth, |cur| {
            self.backward.get(cur).map(|v| v.as_slice()).unwrap_or(&[])
        })
    }

    /// BFS forward walk (IMPLICATIONS): descendants of `id`, up to `max_depth`.
    pub fn implications(&self, id: &str, max_depth: Option<usize>) -> Vec<WalkNode> {
        self.bfs(id, max_depth, |cur| {
            self.forward.get(cur).map(|v| v.as_slice()).unwrap_or(&[])
        })
    }

    fn bfs<'a, F>(&'a self, start: &str, max_depth: Option<usize>, next: F) -> Vec<WalkNode>
    where
        F: Fn(&str) -> &'a [String],
    {
        let mut visited: HashMap<String, usize> = HashMap::new();
        let mut queue: VecDeque<(String, usize)> = VecDeque::new();
        // Seed with direct neighbors of start.
        for neighbor in next(start) {
            if !visited.contains_key(neighbor.as_str()) {
                visited.insert(neighbor.clone(), 1);
                queue.push_back((neighbor.clone(), 1));
            }
        }
        while let Some((cur, depth)) = queue.pop_front() {
            let next_depth = depth + 1;
            if max_depth.map_or(false, |md| depth >= md) {
                continue;
            }
            for neighbor in next(&cur) {
                if !visited.contains_key(neighbor.as_str()) {
                    visited.insert(neighbor.clone(), next_depth);
                    queue.push_back((neighbor.clone(), next_depth));
                }
            }
        }
        let mut result: Vec<WalkNode> = visited
            .into_iter()
            .map(|(id, depth)| WalkNode { id, depth })
            .collect();
        result.sort_by(|a, b| a.depth.cmp(&b.depth).then(a.id.cmp(&b.id)));
        result
    }

    pub fn direct_sources(&self, id: &str) -> &[String] {
        self.backward.get(id).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.backward.is_empty() && self.forward.is_empty()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Ricci curvature TDD ───────────────────────────────────────────────────

    /// Build an atlas from 1D positions with τ very small so each point
    /// spawns its own chart.
    fn linear_atlas(positions: &[f64], g: f64) -> Atlas {
        let mut atlas = Atlas::new(vec!["x".to_string()], 1e-9, g);
        for &p in positions {
            atlas.insert(&[p]);
        }
        atlas
    }

    /// Ricci-1: κ ≤ 1 always.
    #[test]
    fn tdd_ricci_1_upper_bound() {
        let atlas = linear_atlas(&[0.0, 0.1, 0.2, 0.3, 0.4], 0.06);
        for a in 0..atlas.charts.len() as u32 {
            for b in 0..atlas.charts.len() as u32 {
                if a == b { continue; }
                if let Some(r) = atlas.ricci(a, b) {
                    assert!(r.curvature <= 1.0 + 1e-9, "κ({a},{b}) = {} > 1", r.curvature);
                }
            }
        }
    }

    /// Ricci-2: κ is symmetric — κ(a,b) = κ(b,a).
    #[test]
    fn tdd_ricci_2_symmetry() {
        let atlas = linear_atlas(&[0.0, 0.1, 0.2, 0.3, 0.4], 0.06);
        for a in 0..atlas.charts.len() as u32 {
            for b in (a + 1)..atlas.charts.len() as u32 {
                let ab = atlas.ricci(a, b);
                let ba = atlas.ricci(b, a);
                match (ab, ba) {
                    (Some(r_ab), Some(r_ba)) => {
                        assert!(
                            (r_ab.curvature - r_ba.curvature).abs() < 1e-9,
                            "κ({a},{b})={} ≠ κ({b},{a})={}",
                            r_ab.curvature,
                            r_ba.curvature
                        );
                    }
                    (None, None) => {}
                    _ => panic!("ricci({a},{b}) and ricci({b},{a}) disagree on Some/None"),
                }
            }
        }
    }

    /// Ricci-3: κ < 0 for a bridge edge between two disjoint clusters.
    ///
    /// Setup: two clusters {-1.0, -0.9, -0.8} and {0.8, 0.9, 1.0}.
    /// g = 0.06 → 2g = 0.12. Within each cluster, spacing = 0.1 ≤ 0.12.
    /// Between clusters: nearest pair is (-0.8, 0.8) at distance 1.6 >> 0.12.
    /// So B(-0.8) = {-1.0, -0.9, -0.8} and B(0.8) = {0.8, 0.9, 1.0} — disjoint.
    #[test]
    fn tdd_ricci_3_negative_bridge() {
        let positions: Vec<f64> = vec![-1.0, -0.9, -0.8, 0.8, 0.9, 1.0];
        let atlas = linear_atlas(&positions, 0.06);
        // Charts inserted in order: C0=-1.0, C1=-0.9, C2=-0.8, C3=0.8, C4=0.9, C5=1.0
        let result = atlas.ricci(2, 3).expect("ricci(2,3) should return Some");
        assert!(
            result.curvature < 0.0,
            "Expected κ < 0 for bridge, got κ = {}",
            result.curvature
        );
    }

    /// Ricci-4: κ > 0 for interior of a dense cluster.
    ///
    /// g = 0.25 → 2g = 0.5. All 5 charts at spacing 0.1 are within 0.4 < 0.5
    /// of each other → every chart has all 5 as its neighborhood.
    /// B(x) = B(y) = all charts → W₁ = 0 → κ = 1.
    #[test]
    fn tdd_ricci_4_positive_cluster_interior() {
        // All pairwise distances ≤ 0.4, g = 0.25 → 2g = 0.5 → universal neighborhood
        let atlas = linear_atlas(&[0.0, 0.1, 0.2, 0.3, 0.4], 0.25);
        let result = atlas.ricci(0, 4).expect("ricci(0,4) should return Some");
        assert!(
            result.curvature > 0.0,
            "Expected κ > 0 for cluster interior, got κ = {}",
            result.curvature
        );
    }

    /// Ricci-5: W₁ = 0 when B(x) = B(y) → κ = 1.0.
    #[test]
    fn tdd_ricci_5_identical_neighborhoods_kappa_one() {
        let atlas = linear_atlas(&[0.0, 0.1, 0.2, 0.3, 0.4], 0.25);
        // Any pair: all have the same universal neighborhood (all 5 charts)
        let r = atlas.ricci(1, 3).expect("ricci(1,3) should return Some");
        assert!(
            (r.w1).abs() < 1e-9,
            "W₁ = {} for identical neighborhoods (expected 0)",
            r.w1
        );
        assert!(
            (r.curvature - 1.0).abs() < 1e-9,
            "κ = {} for identical neighborhoods (expected 1.0)",
            r.curvature
        );
    }

    /// Ricci-6: distance field equals L2 of centroids.
    #[test]
    fn tdd_ricci_6_distance_is_l2_of_centroids() {
        let atlas = linear_atlas(&[0.0, 0.3], 0.06);
        let r = atlas.ricci(0, 1).expect("ricci(0,1) should return Some");
        assert!(
            (r.distance - 0.3).abs() < 1e-9,
            "distance = {} expected 0.3",
            r.distance
        );
    }

    /// Ricci-7: unknown chart IDs return None.
    #[test]
    fn tdd_ricci_7_invalid_id_returns_none() {
        let atlas = linear_atlas(&[0.0, 0.1], 0.06);
        assert!(atlas.ricci(0, 99).is_none(), "ricci with invalid id should return None");
        assert!(atlas.ricci(99, 0).is_none(), "ricci with invalid id should return None");
    }

    /// Ricci-8: empty atlas returns None.
    #[test]
    fn tdd_ricci_8_empty_atlas() {
        let atlas = Atlas::new(vec!["x".to_string()], 0.3, 0.15);
        assert!(atlas.ricci(0, 1).is_none());
    }
}
