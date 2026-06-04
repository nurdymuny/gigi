//! Fiedler-vector partition strategy for sharded bundles.
//!
//! The math is locked by TFP1 (`theory/poincare_to_sharding/validation/
//! tfp1_fiedler_preserves_curvature.py`): hash partitioning shreds the
//! neighborhood graph that GIGI's K depends on (k_sum error 112.9% at
//! n=2, 9,440% at n=8). Fiedler partitioning preserves it (0.0% at
//! n=2, 39.5% at n=8). The honest-disclosure RED on hash sharding
//! becomes a GREEN with Fiedler — the partition respects the
//! neighborhood structure K is computed against.
//!
//! ## Algorithm
//!
//! 1. Build a k-NN adjacency graph from the records (symmetrized).
//! 2. Form the combinatorial Laplacian `L = D - A`.
//! 3. Find the Fiedler vector (eigenvector of L for λ_2 — the
//!    second-smallest eigenvalue) via shifted power iteration on
//!    `(c·I - L)` with deflation against the all-ones vector.
//! 4. Bisect by median of the Fiedler vector.
//! 5. For n_charts > 2, recursively bisect each half.
//!
//! `n_charts` must be a power of 2. (The recursive-bisection
//! requirement is inherent to spectral partitioning; for arbitrary
//! n_charts a different algorithm — k-means on the spectral
//! embedding — is the standard extension. Phase E-future.)

use crate::types::{BundleSchema, Record, Value};

/// Errors from Fiedler partitioning.
#[derive(Debug, Clone, PartialEq)]
pub enum FiedlerError {
    /// `n_charts` must be a power of 2 for recursive bisection.
    NotPowerOfTwo(u32),
    /// At least one record must have an extractable numeric or vector
    /// fiber for k-NN graph construction.
    NoNumericFields,
    /// At least 2 records are required for partitioning.
    TooFewRecords(usize),
    /// Schema disagrees with records on field types.
    SchemaMismatch(String),
}

/// Configuration for Fiedler partitioning.
#[derive(Clone, Debug)]
pub struct FiedlerConfig {
    /// Number of charts to partition into. Must be a power of 2.
    pub n_charts: u32,
    /// Number of nearest neighbors used to build the adjacency graph.
    pub k_neighbors: usize,
    /// Maximum power-iteration steps per bisection.
    pub max_iterations: usize,
}

impl Default for FiedlerConfig {
    fn default() -> Self {
        Self {
            n_charts: 4,
            k_neighbors: 6,
            max_iterations: 200,
        }
    }
}

/// Extract numeric coordinates from a record's fiber fields.
///
/// Walks the schema's fiber fields in declared order and pulls Float /
/// Integer values into a coordinate vector. Categorical and other
/// types are skipped. Returns None if no numeric coordinates are
/// available.
pub(crate) fn extract_coords(record: &Record, schema: &BundleSchema) -> Option<Vec<f64>> {
    let mut coords = Vec::new();
    for field in &schema.fiber_fields {
        match record.get(&field.name)? {
            Value::Float(f) => coords.push(*f),
            Value::Integer(i) => coords.push(*i as f64),
            Value::Vector(v) => coords.extend(v.iter().copied()),
            _ => {} // skip non-numeric
        }
    }
    if coords.is_empty() {
        None
    } else {
        Some(coords)
    }
}

fn squared_dist(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

/// Build a symmetric k-NN adjacency matrix from records' fiber coords.
pub(crate) fn knn_adjacency(coords: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let n = coords.len();
    let mut adj = vec![vec![0.0; n]; n];
    let k_eff = k.min(n.saturating_sub(1));
    for i in 0..n {
        let mut dists: Vec<(f64, usize)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (squared_dist(&coords[i], &coords[j]), j))
            .collect();
        let len = dists.len();
        if len > 0 && k_eff > 0 {
            let pivot = k_eff.saturating_sub(1).min(len - 1);
            dists.select_nth_unstable_by(pivot, |a, b| {
                a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        for &(_, j) in dists.iter().take(k_eff) {
            adj[i][j] = 1.0;
            adj[j][i] = 1.0;
        }
    }
    adj
}

/// Combinatorial Laplacian L = D - A.
pub(crate) fn laplacian(adj: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = adj.len();
    let degrees: Vec<f64> = adj.iter().map(|row| row.iter().sum()).collect();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            l[i][j] = -adj[i][j];
        }
        l[i][i] += degrees[i];
    }
    l
}

/// Find the Fiedler vector via shifted power iteration.
///
/// Pub(crate) so the substrate-side Laplacian extractor can use it for
/// 2-way bisection partitioning of arbitrary bundles.
///
/// Strategy: iterate `M = c·I - L` for `c` larger than the max diagonal
/// of L. The largest eigenvalue of M corresponds to the smallest
/// eigenvalue of L. After each step, project orthogonal to the
/// all-ones vector (deflating the null space of L for connected
/// graphs).
pub(crate) fn fiedler_vector(l: &[Vec<f64>], max_iter: usize) -> Vec<f64> {
    let n = l.len();
    if n == 0 {
        return Vec::new();
    }

    // Deterministic, mean-≈-0 seed vector
    let mut v: Vec<f64> = (0..n).map(|i| (i % 3) as f64 - 1.0).collect();
    let mean = v.iter().sum::<f64>() / n as f64;
    for x in v.iter_mut() {
        *x -= mean;
    }
    let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm < 1e-12 {
        return vec![0.0; n];
    }
    for x in v.iter_mut() {
        *x /= norm;
    }

    let c = 2.0 * (0..n).map(|i| l[i][i]).fold(0.0_f64, f64::max) + 1.0;

    for _ in 0..max_iter {
        // w = (c·I - L) · v
        let mut w = vec![0.0; n];
        for i in 0..n {
            let mut sum = c * v[i];
            for j in 0..n {
                sum -= l[i][j] * v[j];
            }
            w[i] = sum;
        }
        // Orthogonalize against all-ones
        let mean_w = w.iter().sum::<f64>() / n as f64;
        for x in w.iter_mut() {
            *x -= mean_w;
        }
        // Normalize
        let norm = w.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm < 1e-12 {
            break;
        }
        for x in w.iter_mut() {
            *x /= norm;
        }
        // Convergence
        let diff: f64 = w.iter().zip(v.iter()).map(|(a, b)| (a - b).powi(2)).sum();
        v = w;
        if diff < 1e-12 {
            break;
        }
    }
    v
}

/// Bisect a set of indices by the median of the Fiedler vector
/// computed on the subgraph induced by those indices.
fn fiedler_bisect(
    indices: &[usize],
    coords: &[Vec<f64>],
    k_nn: usize,
    max_iter: usize,
) -> (Vec<usize>, Vec<usize>) {
    if indices.len() < 2 {
        return (indices.to_vec(), Vec::new());
    }
    let sub_coords: Vec<Vec<f64>> = indices.iter().map(|&i| coords[i].clone()).collect();
    let adj = knn_adjacency(&sub_coords, k_nn.min(indices.len().saturating_sub(1)));
    let l = laplacian(&adj);
    let fv = fiedler_vector(&l, max_iter);
    let mut sorted_fv = fv.clone();
    sorted_fv.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted_fv[fv.len() / 2];
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (local_idx, &global_idx) in indices.iter().enumerate() {
        if fv[local_idx] < median {
            left.push(global_idx);
        } else {
            right.push(global_idx);
        }
    }
    (left, right)
}

/// Partition records into `config.n_charts` charts using recursive
/// Fiedler bisection. Returns the assignment vector (record_index ->
/// chart_index).
///
/// Per TFP1: this partition strategy preserves the neighborhood graph
/// structure that GIGI's K computation depends on, making sharded
/// CURVATURE partition-invariant where hash sharding makes it
/// partition-dependent.
pub fn fiedler_partition(
    records: &[Record],
    schema: &BundleSchema,
    config: &FiedlerConfig,
) -> Result<Vec<u32>, FiedlerError> {
    if records.len() < 2 {
        return Err(FiedlerError::TooFewRecords(records.len()));
    }
    if !config.n_charts.is_power_of_two() {
        return Err(FiedlerError::NotPowerOfTwo(config.n_charts));
    }
    if config.n_charts == 1 {
        return Ok(vec![0; records.len()]);
    }

    // Extract coordinates per record
    let coords: Vec<Vec<f64>> = records
        .iter()
        .filter_map(|r| extract_coords(r, schema))
        .collect();
    if coords.len() != records.len() {
        return Err(FiedlerError::NoNumericFields);
    }

    // Recursive bisection
    let n = records.len();
    let mut assignment = vec![0u32; n];

    fn recurse(
        indices: &[usize],
        coords: &[Vec<f64>],
        k_nn: usize,
        max_iter: usize,
        current_chart: u32,
        target_charts: u32,
        assignment: &mut [u32],
    ) {
        if target_charts == 1 || indices.len() < 2 {
            for &i in indices {
                assignment[i] = current_chart;
            }
            return;
        }
        let (left, right) = fiedler_bisect(indices, coords, k_nn, max_iter);
        let half = target_charts / 2;
        recurse(
            &left,
            coords,
            k_nn,
            max_iter,
            current_chart,
            half,
            assignment,
        );
        recurse(
            &right,
            coords,
            k_nn,
            max_iter,
            current_chart + half,
            half,
            assignment,
        );
    }

    let all_indices: Vec<usize> = (0..n).collect();
    recurse(
        &all_indices,
        &coords,
        config.k_neighbors,
        config.max_iterations,
        0,
        config.n_charts,
        &mut assignment,
    );

    Ok(assignment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FieldDef, Value};

    fn make_schema() -> BundleSchema {
        BundleSchema::new("fiedler_test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(10.0))
            .fiber(FieldDef::numeric("y").with_range(10.0))
    }

    /// Reproduces TFP1's two-cluster dataset in Rust.
    fn make_two_cluster_records(n_per_cluster: usize) -> Vec<Record> {
        let mut records = Vec::new();
        let mut rng_state: u32 = 12345;
        let mut lcg = || -> f64 {
            rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
            (rng_state as f64) / (u32::MAX as f64) - 0.5
        };
        let mut pk: i64 = 0;
        for &(cx, cy) in &[(-1.0_f64, 0.0_f64), (1.0, 0.0)] {
            for _ in 0..n_per_cluster {
                let x = cx + 0.3 * lcg() * 2.0;
                let y = cy + 0.3 * lcg() * 2.0;
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(pk));
                r.insert("x".into(), Value::Float(x));
                r.insert("y".into(), Value::Float(y));
                records.push(r);
                pk += 1;
            }
        }
        records
    }

    #[test]
    fn fiedler_partition_into_one_chart_returns_all_zeros() {
        let records = make_two_cluster_records(20);
        let schema = make_schema();
        let cfg = FiedlerConfig {
            n_charts: 1,
            ..Default::default()
        };
        let assignment = fiedler_partition(&records, &schema, &cfg).unwrap();
        assert_eq!(assignment.len(), 40);
        assert!(assignment.iter().all(|&c| c == 0));
    }

    #[test]
    fn fiedler_partition_rejects_non_power_of_two() {
        let records = make_two_cluster_records(20);
        let schema = make_schema();
        let cfg = FiedlerConfig {
            n_charts: 3,
            ..Default::default()
        };
        let r = fiedler_partition(&records, &schema, &cfg);
        assert_eq!(r, Err(FiedlerError::NotPowerOfTwo(3)));
    }

    #[test]
    fn fiedler_partition_rejects_too_few_records() {
        let records = vec![{
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(0));
            r.insert("x".into(), Value::Float(0.0));
            r.insert("y".into(), Value::Float(0.0));
            r
        }];
        let schema = make_schema();
        let r = fiedler_partition(&records, &schema, &FiedlerConfig::default());
        assert_eq!(r, Err(FiedlerError::TooFewRecords(1)));
    }

    #[test]
    fn fiedler_partition_bisects_balanced_at_two_charts() {
        let records = make_two_cluster_records(40);
        let schema = make_schema();
        let cfg = FiedlerConfig {
            n_charts: 2,
            ..Default::default()
        };
        let assignment = fiedler_partition(&records, &schema, &cfg).unwrap();
        let n_0 = assignment.iter().filter(|&&c| c == 0).count();
        let n_1 = assignment.iter().filter(|&&c| c == 1).count();
        assert_eq!(n_0 + n_1, 80);
        // Balanced within ±5 of perfect 40/40 (TFP1 case)
        assert!(
            (n_0 as i32 - n_1 as i32).abs() <= 5,
            "Fiedler bisection unbalanced: {}/{}",
            n_0,
            n_1
        );
    }

    #[test]
    fn fiedler_partition_separates_well_separated_clusters() {
        // TFP1 case 1: two well-separated clusters should land in
        // different charts.
        let records = make_two_cluster_records(40);
        let schema = make_schema();
        let cfg = FiedlerConfig {
            n_charts: 2,
            ..Default::default()
        };
        let assignment = fiedler_partition(&records, &schema, &cfg).unwrap();
        // Cluster A is records [0..40), cluster B is [40..80)
        let a_in_chart_0 = assignment[0..40].iter().filter(|&&c| c == 0).count();
        let b_in_chart_0 = assignment[40..80].iter().filter(|&&c| c == 0).count();
        // Each cluster concentrated (>= 30 of 40 in one chart)
        let a_concentrated = a_in_chart_0.max(40 - a_in_chart_0) >= 30;
        let b_concentrated = b_in_chart_0.max(40 - b_in_chart_0) >= 30;
        assert!(
            a_concentrated,
            "cluster A not concentrated (a_in_chart_0={})",
            a_in_chart_0
        );
        assert!(
            b_concentrated,
            "cluster B not concentrated (b_in_chart_0={})",
            b_in_chart_0
        );
        // And the two clusters land in different charts
        let a_dominant = if a_in_chart_0 >= 20 { 0 } else { 1 };
        let b_dominant = if b_in_chart_0 >= 20 { 0 } else { 1 };
        assert_ne!(
            a_dominant, b_dominant,
            "clusters landed in same chart (a={}, b={})",
            a_dominant, b_dominant
        );
    }

    #[test]
    fn fiedler_partition_into_four_charts_balanced() {
        let records = make_two_cluster_records(40);
        let schema = make_schema();
        let cfg = FiedlerConfig {
            n_charts: 4,
            ..Default::default()
        };
        let assignment = fiedler_partition(&records, &schema, &cfg).unwrap();
        for chart in 0..4u32 {
            let count = assignment.iter().filter(|&&c| c == chart).count();
            // Each chart should be ~20 (80 / 4). Allow ±5 for boundary slack.
            assert!(
                (count as i32 - 20).abs() <= 7,
                "chart {} unbalanced: count={}",
                chart,
                count
            );
        }
    }

    #[test]
    fn fiedler_partition_is_deterministic() {
        let records = make_two_cluster_records(40);
        let schema = make_schema();
        let cfg = FiedlerConfig {
            n_charts: 4,
            ..Default::default()
        };
        let a1 = fiedler_partition(&records, &schema, &cfg).unwrap();
        let a2 = fiedler_partition(&records, &schema, &cfg).unwrap();
        let a3 = fiedler_partition(&records, &schema, &cfg).unwrap();
        assert_eq!(a1, a2);
        assert_eq!(a2, a3);
    }

    #[test]
    fn extract_coords_pulls_numeric_fiber_fields() {
        let schema = make_schema();
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("x".into(), Value::Float(3.5));
        r.insert("y".into(), Value::Float(-2.0));
        let coords = extract_coords(&r, &schema).unwrap();
        assert_eq!(coords, vec![3.5, -2.0]);
    }

    #[test]
    fn extract_coords_returns_none_when_no_numeric() {
        let schema = BundleSchema::new("no_numeric")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"));
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("name".into(), Value::Text("alice".into()));
        let coords = extract_coords(&r, &schema);
        assert!(coords.is_none());
    }

    #[test]
    fn fiedler_partition_refuses_no_numeric_fiber() {
        let schema = BundleSchema::new("no_numeric")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"));
        let mut records = Vec::new();
        for i in 0..10 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("u_{}", i)));
            records.push(r);
        }
        let cfg = FiedlerConfig {
            n_charts: 2,
            ..Default::default()
        };
        let result = fiedler_partition(&records, &schema, &cfg);
        assert_eq!(result, Err(FiedlerError::NoNumericFields));
    }
}
