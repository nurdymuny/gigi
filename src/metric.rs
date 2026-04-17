//! Fiber metric g_F — Definition 1.7.
//!
//! Type-based product metric with per-field weights.

use crate::types::{FieldDef, FieldType, Value};

/// Fiber metric (Def 1.7).
#[derive(Debug, Clone)]
pub struct FiberMetric;

impl FiberMetric {
    /// Per-component distance g_i(a, b) from Def 1.7 type table.
    pub fn component_distance(field: &FieldDef, a: &Value, b: &Value) -> f64 {
        match (&field.field_type, a, b) {
            // Numeric: |a-b| / range
            (FieldType::Numeric, _, _) => {
                let va = a.as_f64().unwrap_or(0.0);
                let vb = b.as_f64().unwrap_or(0.0);
                let range = field.range.unwrap_or(1.0).max(f64::EPSILON);
                (va - vb).abs() / range
            }
            // Categorical: 0 if equal, 1 otherwise
            (FieldType::Categorical, _, _) => {
                if a == b {
                    0.0
                } else {
                    1.0
                }
            }
            // Ordered categorical: |rank(a) - rank(b)| / (|order| - 1)
            (FieldType::OrderedCat { order }, Value::Text(sa), Value::Text(sb)) => {
                let ia = order.iter().position(|x| x == sa);
                let ib = order.iter().position(|x| x == sb);
                match (ia, ib) {
                    (Some(a), Some(b)) => {
                        let n = (order.len() - 1).max(1) as f64;
                        (a as f64 - b as f64).abs() / n
                    }
                    _ => {
                        if a == b {
                            0.0
                        } else {
                            1.0
                        }
                    }
                }
            }
            // Timestamp: |a-b| / time_scale
            (FieldType::Timestamp, _, _) => {
                let va = a.as_f64().unwrap_or(0.0);
                let vb = b.as_f64().unwrap_or(0.0);
                let scale = field.range.unwrap_or(1.0).max(f64::EPSILON);
                (va - vb).abs() / scale
            }
            // Binary / fallback: discrete
            _ => {
                if a == b {
                    0.0
                } else {
                    1.0
                }
            }
        }
    }

    /// Product metric g_F(v, w) = sqrt(Σ ω_i · g_i²) from Def 1.7.
    pub fn distance(fields: &[FieldDef], a: &[Value], b: &[Value]) -> f64 {
        let mut total = 0.0;
        for (i, field) in fields.iter().enumerate() {
            let va = a.get(i).unwrap_or(&Value::Null);
            let vb = b.get(i).unwrap_or(&Value::Null);
            let d = Self::component_distance(field, va, vb);
            total += field.weight * d * d;
        }
        total.sqrt()
    }
}

// ── Metric Tensor (§2.4) ──────────────────────────────────────────────────

use crate::bundle::BundleStore;

/// Result of metric tensor computation on the data manifold.
#[derive(Debug, Clone)]
pub struct MetricTensorInfo {
    /// Empirical correlation matrix G_{ij} (n×n for n numeric fiber fields).
    pub matrix: Vec<Vec<f64>>,
    /// Eigenvalues in descending order.
    pub eigenvalues: Vec<f64>,
    /// Condition number κ(G) = λ_max / λ_min.
    pub condition_number: f64,
    /// Effective dimension d_eff = (Σλ)² / Σλ².
    pub effective_dimension: f64,
    /// Names of the numeric fiber fields (column/row labels).
    pub field_names: Vec<String>,
}

/// Compute the empirical metric tensor (correlation geometry) of the bundle.
///
/// For n numeric fiber fields, computes the n×n correlation matrix of
/// standardized field values, then extracts eigenvalues via power iteration.
pub fn metric_tensor(store: &BundleStore) -> MetricTensorInfo {
    let stats = store.field_stats();
    let schema = &store.schema;

    // Identify numeric fiber fields with nonzero variance
    let fields: Vec<(usize, String, f64, f64)> = schema
        .fiber_fields
        .iter()
        .enumerate()
        .filter_map(|(idx, f)| {
            let fs = stats.get(&f.name)?;
            if fs.count < 2 {
                return None;
            }
            let var = fs.variance();
            if var < f64::EPSILON {
                return None;
            }
            let mean = fs.sum / fs.count as f64;
            let std = var.sqrt();
            Some((idx, f.name.clone(), mean, std))
        })
        .collect();

    let n = fields.len();
    if n == 0 {
        return MetricTensorInfo {
            matrix: vec![],
            eigenvalues: vec![],
            condition_number: 0.0,
            effective_dimension: 0.0,
            field_names: vec![],
        };
    }

    // Build correlation matrix: G_{ij} = (1/N) Σ z_i(k) z_j(k)
    // where z_i = (f_i - mean_i) / std_i
    let mut g = vec![vec![0.0_f64; n]; n];
    let mut count = 0usize;

    for (_bp, fiber) in store.sections() {
        let mut z = Vec::with_capacity(n);
        let mut valid = true;
        for &(idx, _, mean, std) in &fields {
            let val = fiber.get(idx).and_then(|v| v.as_f64());
            match val {
                Some(v) => z.push((v - mean) / std),
                None => { valid = false; break; }
            }
        }
        if !valid || z.len() != n {
            continue;
        }
        for i in 0..n {
            for j in i..n {
                let product = z[i] * z[j];
                g[i][j] += product;
                if i != j {
                    g[j][i] += product;
                }
            }
        }
        count += 1;
    }

    if count > 0 {
        let inv_n = 1.0 / count as f64;
        for row in &mut g {
            for val in row.iter_mut() {
                *val *= inv_n;
            }
        }
    }

    // Eigenvalues via power iteration (symmetric matrix)
    let eigenvalues = symmetric_eigenvalues(&g, n);

    let lambda_max = eigenvalues.first().copied().unwrap_or(0.0);
    let lambda_min = eigenvalues.last().copied().unwrap_or(0.0).max(f64::EPSILON);
    let condition_number = lambda_max / lambda_min;

    let sum_lambda: f64 = eigenvalues.iter().sum();
    let sum_sq: f64 = eigenvalues.iter().map(|&l| l * l).sum();
    let effective_dimension = if sum_sq > f64::EPSILON {
        (sum_lambda * sum_lambda) / sum_sq
    } else {
        0.0
    };

    MetricTensorInfo {
        matrix: g,
        eigenvalues,
        condition_number,
        effective_dimension,
        field_names: fields.iter().map(|(_, name, _, _)| name.clone()).collect(),
    }
}

/// Extract eigenvalues of a symmetric matrix via deflated power iteration.
fn symmetric_eigenvalues(matrix: &[Vec<f64>], n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }

    let mut a: Vec<Vec<f64>> = matrix.to_vec();
    let mut eigenvalues = Vec::with_capacity(n);
    let max_iter = 200;
    let tol = 1e-10;

    for k in 0..n {
        // Power iteration for dominant eigenvalue
        // Use a different starting vector for each iteration to avoid null-space issues
        let mut v = vec![0.0; n];
        for i in 0..n {
            v[i] = ((i + k + 1) as f64).sin().abs() + 0.1;
        }
        let norm: f64 = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for x in &mut v {
            *x /= norm;
        }
        let mut lambda = 0.0_f64;

        for _ in 0..max_iter {
            // w = A v
            let mut w = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    w[i] += a[i][j] * v[j];
                }
            }
            // Rayleigh quotient
            let new_lambda: f64 = v.iter().zip(&w).map(|(vi, wi)| vi * wi).sum();
            // Normalize
            let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < f64::EPSILON {
                break;
            }
            for x in &mut w {
                *x /= norm;
            }
            v = w;
            if (new_lambda - lambda).abs() < tol {
                lambda = new_lambda;
                break;
            }
            lambda = new_lambda;
        }

        eigenvalues.push(lambda.abs());

        // Deflate: A = A - lambda * v * v^T
        for i in 0..n {
            for j in 0..n {
                a[i][j] -= lambda * v[i] * v[j];
            }
        }
    }

    eigenvalues.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    eigenvalues
}

// ── KL Divergence (§2.5 Information Geometry × Cross-Bundle) ─────────────────

/// Per-field KL divergence report between two bundles.
///
/// `kl_forward` = D_KL(P ‖ Q)   `kl_reverse` = D_KL(Q ‖ P)
/// `jensen_shannon` = D_JS(P, Q) ∈ [0, ln 2] — symmetric.
/// `per_field` = per-field forward KL contributions.
#[derive(Debug, Clone)]
pub struct DivergenceReport {
    pub kl_forward: f64,
    pub kl_reverse: f64,
    pub jensen_shannon: f64,
    pub per_field: Vec<(String, f64)>,
    pub fields_compared: usize,
}

/// Optimal number of histogram bins via the Freedman–Diaconis rule.
/// h = 2 · IQR · N^{-1/3}, bins = ⌈range / h⌉, clamped to [10, 200].
pub fn freedman_diaconis_bins(values: &[f64]) -> usize {
    if values.len() < 4 {
        return 10;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let q1 = sorted[n / 4];
    let q3 = sorted[3 * n / 4];
    let iqr = (q3 - q1).abs();
    if iqr < 1e-12 {
        return 10;
    }
    let range = sorted[n - 1] - sorted[0];
    let h = 2.0 * iqr * (n as f64).powf(-1.0 / 3.0);
    ((range / h).ceil() as usize).clamp(10, 200)
}

/// Build a raw count histogram over `n_bins` equal-width bins on [min_val, max_val].
fn build_histogram(values: &[f64], n_bins: usize, min_val: f64, max_val: f64) -> Vec<f64> {
    let range = (max_val - min_val).max(1e-12);
    let bin_width = range / n_bins as f64;
    let mut hist = vec![0.0f64; n_bins];
    for &v in values {
        let idx = ((v - min_val) / bin_width).floor() as usize;
        hist[idx.min(n_bins - 1)] += 1.0;
    }
    hist
}

/// Laplace-smoothed probability vector from raw histogram counts.
/// p̂(b) = (n_b + α) / (N + α K)  where α = 1 and K = n_bins.
fn smooth(hist: &[f64], n_total: usize, alpha: f64) -> Vec<f64> {
    let k = hist.len() as f64;
    let denom = n_total as f64 + alpha * k;
    hist.iter().map(|&c| (c + alpha) / denom).collect()
}

/// D_KL(p ‖ q) = Σ p_i · ln(p_i / q_i).  Both p and q must be smoothed (> 0).
fn kl_term(p: &[f64], q: &[f64]) -> f64 {
    p.iter()
        .zip(q)
        .map(|(&pi, &qi)| if pi < 1e-300 { 0.0 } else { pi * (pi / qi).ln() })
        .sum()
}

/// Compute KL and Jensen–Shannon divergence between two bundles (per-field histogram approach).
///
/// Only numeric fields common to both bundles are compared.
/// Binning: Freedman–Diaconis per field, clamped to [10, 200].
/// Smoothing: additive Laplace (α = 1).
pub fn kl_divergence(a: &BundleStore, b: &BundleStore) -> DivergenceReport {
    use crate::types::Value;

    let stats_a = a.field_stats();
    let stats_b = b.field_stats();

    // Common numeric fields present in both stores with at least 1 record.
    let mut common_fields: Vec<String> = stats_a
        .keys()
        .filter(|f| stats_b.contains_key(*f))
        .filter(|f| stats_a[*f].count > 0 && stats_b[*f].count > 0)
        .cloned()
        .collect();
    common_fields.sort(); // deterministic order

    if common_fields.is_empty() {
        return DivergenceReport {
            kl_forward: 0.0,
            kl_reverse: 0.0,
            jensen_shannon: 0.0,
            per_field: vec![],
            fields_compared: 0,
        };
    }

    // Single pass over each store: collect per-field values.
    let collect = |store: &BundleStore, fields: &[String]| -> std::collections::HashMap<String, Vec<f64>> {
        let mut map: std::collections::HashMap<String, Vec<f64>> =
            fields.iter().map(|f| (f.clone(), Vec::new())).collect();
        for rec in store.records() {
            for f in fields {
                if let Some(v) = rec.get(f) {
                    if let Some(x) = v.as_f64() {
                        map.get_mut(f).unwrap().push(x);
                    }
                }
            }
        }
        map
    };

    let vals_a = collect(a, &common_fields);
    let vals_b = collect(b, &common_fields);

    let mut kl_fwd = 0.0;
    let mut kl_rev = 0.0;
    let mut js_total = 0.0;
    let mut per_field = Vec::new();
    let mut fields_compared = 0;

    for field in &common_fields {
        let va = &vals_a[field];
        let vb = &vals_b[field];
        if va.is_empty() || vb.is_empty() {
            continue;
        }

        // Range from both stores combined.
        let min_val = va.iter().chain(vb).cloned().fold(f64::INFINITY, f64::min);
        let max_val = va.iter().chain(vb).cloned().fold(f64::NEG_INFINITY, f64::max);
        if (max_val - min_val).abs() < 1e-12 {
            continue; // all values identical → no divergence
        }

        let n_bins = freedman_diaconis_bins(va)
            .max(freedman_diaconis_bins(vb))
            .clamp(10, 200);

        let hist_a = build_histogram(va, n_bins, min_val, max_val);
        let hist_b = build_histogram(vb, n_bins, min_val, max_val);

        let p = smooth(&hist_a, va.len(), 1.0);
        let q = smooth(&hist_b, vb.len(), 1.0);

        let kl_pq = kl_term(&p, &q);
        let kl_qp = kl_term(&q, &p);

        // Jensen–Shannon via midpoint distribution M = (P + Q) / 2.
        let m: Vec<f64> = p.iter().zip(&q).map(|(pi, qi)| (pi + qi) / 2.0).collect();
        let js = 0.5 * (kl_term(&p, &m) + kl_term(&q, &m));

        kl_fwd += kl_pq;
        kl_rev += kl_qp;
        js_total += js;
        per_field.push((field.clone(), kl_pq));
        fields_compared += 1;
    }

    DivergenceReport {
        kl_forward: kl_fwd,
        kl_reverse: kl_rev,
        jensen_shannon: js_total,
        per_field,
        fields_compared,
    }
}

/// Same as [`kl_divergence`] but accepts the unified [`BundleRef`] type so it works
/// with both heap and mmap (overlay) bundles.
pub fn kl_divergence_ref(
    a: &crate::mmap_bundle::BundleRef<'_>,
    b: &crate::mmap_bundle::BundleRef<'_>,
) -> DivergenceReport {
    // Collect all numeric field values for a list of field names from a BundleRef.
    // Returns only fields where at least one numeric value was found.
    // Cap at 100K records per bundle to keep latency bounded on large datasets.
    const MAX_SAMPLE: usize = 100_000;
    let collect_numeric = |store: &crate::mmap_bundle::BundleRef<'_>,
                           candidates: &[String]|
     -> std::collections::HashMap<String, Vec<f64>> {
        let mut map: std::collections::HashMap<String, Vec<f64>> =
            candidates.iter().map(|f| (f.clone(), Vec::new())).collect();
        for rec in store.records().take(MAX_SAMPLE) {
            for f in candidates {
                if let Some(v) = rec.get(f) {
                    if let Some(x) = v.as_f64() {
                        map.get_mut(f).unwrap().push(x);
                    }
                }
            }
        }
        // Keep only fields with actual numeric data.
        map.retain(|_, vs| !vs.is_empty());
        map
    };

    // Determine candidate field names.
    // Prefer field_stats (heap bundles maintain this); fall back to schema field_names
    // for mmap/overlay bundles where field_stats is not populated.
    let candidate_fields_a: Vec<String> = {
        let stats = a.field_stats();
        if !stats.is_empty() {
            stats.into_keys().collect()
        } else {
            a.field_names()
        }
    };
    let candidate_fields_b: Vec<String> = {
        let stats = b.field_stats();
        if !stats.is_empty() {
            stats.into_keys().collect()
        } else {
            b.field_names()
        }
    };

    // Intersect candidate field names from both bundles.
    let b_set: std::collections::HashSet<_> = candidate_fields_b.iter().collect();
    let mut common_candidates: Vec<String> = candidate_fields_a
        .iter()
        .filter(|f| b_set.contains(f))
        .cloned()
        .collect();
    common_candidates.sort();

    if common_candidates.is_empty() {
        return DivergenceReport {
            kl_forward: 0.0,
            kl_reverse: 0.0,
            jensen_shannon: 0.0,
            per_field: vec![],
            fields_compared: 0,
        };
    }

    let vals_a = collect_numeric(a, &common_candidates);
    let vals_b = collect_numeric(b, &common_candidates);

    // Only compare fields with numeric data in BOTH bundles.
    let mut common_fields: Vec<String> = common_candidates
        .into_iter()
        .filter(|f| vals_a.contains_key(f) && vals_b.contains_key(f))
        .collect();
    common_fields.sort();

    if common_fields.is_empty() {
        return DivergenceReport {
            kl_forward: 0.0,
            kl_reverse: 0.0,
            jensen_shannon: 0.0,
            per_field: vec![],
            fields_compared: 0,
        };
    }

    let mut kl_fwd = 0.0;
    let mut kl_rev = 0.0;
    let mut js_total = 0.0;
    let mut per_field = Vec::new();
    let mut fields_compared = 0;

    for field in &common_fields {
        let va = &vals_a[field];
        let vb = &vals_b[field];
        let min_val = va.iter().chain(vb).cloned().fold(f64::INFINITY, f64::min);
        let max_val = va.iter().chain(vb).cloned().fold(f64::NEG_INFINITY, f64::max);
        if (max_val - min_val).abs() < 1e-12 {
            continue;
        }
        let n_bins = freedman_diaconis_bins(va)
            .max(freedman_diaconis_bins(vb))
            .clamp(10, 200);
        let hist_a = build_histogram(va, n_bins, min_val, max_val);
        let hist_b = build_histogram(vb, n_bins, min_val, max_val);
        let p = smooth(&hist_a, va.len(), 1.0);
        let q_dist = smooth(&hist_b, vb.len(), 1.0);
        let kl_pq = kl_term(&p, &q_dist);
        let kl_qp = kl_term(&q_dist, &p);
        let m: Vec<f64> = p.iter().zip(&q_dist).map(|(pi, qi)| (pi + qi) / 2.0).collect();
        let js = 0.5 * (kl_term(&p, &m) + kl_term(&q_dist, &m));
        kl_fwd += kl_pq;
        kl_rev += kl_qp;
        js_total += js;
        per_field.push((field.clone(), kl_pq));
        fields_compared += 1;
    }

    DivergenceReport {
        kl_forward: kl_fwd,
        kl_reverse: kl_rev,
        jensen_shannon: js_total,
        per_field,
        fields_compared,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-1.12: Fisher metric numeric — g_F matches normalized L2.
    #[test]
    fn tdd_1_12_numeric_metric() {
        let f = FieldDef::numeric("x").with_range(100.0);
        let d = FiberMetric::component_distance(&f, &Value::Float(20.0), &Value::Float(40.0));
        assert!((d - 0.2).abs() < 1e-10);
    }

    /// TDD-1.13: Fisher metric categorical — same=0, diff=1.
    #[test]
    fn tdd_1_13_categorical_metric() {
        let f = FieldDef::categorical("color");
        assert_eq!(
            FiberMetric::component_distance(
                &f,
                &Value::Text("red".into()),
                &Value::Text("red".into())
            ),
            0.0
        );
        assert_eq!(
            FiberMetric::component_distance(
                &f,
                &Value::Text("red".into()),
                &Value::Text("blue".into())
            ),
            1.0
        );
    }

    /// GAP-A.1: Identity g(a,a) = 0.
    #[test]
    fn gap_a1_identity() {
        let fields = vec![
            FieldDef::numeric("x").with_range(100.0),
            FieldDef::categorical("c"),
        ];
        let a = vec![Value::Float(42.0), Value::Text("hello".into())];
        assert!(FiberMetric::distance(&fields, &a, &a).abs() < 1e-15);
    }

    /// GAP-A.2: Symmetry g(a,b) = g(b,a).
    #[test]
    fn gap_a2_symmetry() {
        let fields = vec![
            FieldDef::numeric("x").with_range(100.0),
            FieldDef::categorical("c"),
        ];
        let a = vec![Value::Float(10.0), Value::Text("A".into())];
        let b = vec![Value::Float(50.0), Value::Text("B".into())];
        let d_ab = FiberMetric::distance(&fields, &a, &b);
        let d_ba = FiberMetric::distance(&fields, &b, &a);
        assert!((d_ab - d_ba).abs() < 1e-14);
    }

    /// GAP-A.4: Ordered categorical metric.
    #[test]
    fn gap_a4_ordered_cat() {
        let order = vec!["XS", "S", "M", "L", "XL"]
            .into_iter()
            .map(String::from)
            .collect();
        let f = FieldDef {
            name: "size".into(),
            field_type: FieldType::OrderedCat { order },
            default: Value::Null,
            range: None,
            weight: 1.0,
        };
        let d_full = FiberMetric::component_distance(
            &f,
            &Value::Text("XS".into()),
            &Value::Text("XL".into()),
        );
        let d_one = FiberMetric::component_distance(
            &f,
            &Value::Text("XS".into()),
            &Value::Text("S".into()),
        );
        assert!((d_full - 1.0).abs() < 1e-10);
        assert!((d_one - 0.25).abs() < 1e-10);
    }

    /// GAP-A.5: Timestamp metric.
    #[test]
    fn gap_a5_timestamp() {
        let f = FieldDef::timestamp("ts", 86400.0);
        let d = FiberMetric::component_distance(&f, &Value::Float(0.0), &Value::Float(43200.0));
        assert!((d - 0.5).abs() < 1e-10);
    }

    /// GAP-A.6: Weighted metric.
    #[test]
    fn gap_a6_weighted() {
        let fields = vec![
            FieldDef::numeric("x").with_range(100.0).with_weight(4.0),
            FieldDef::numeric("y").with_range(100.0).with_weight(1.0),
        ];
        let origin = vec![Value::Float(0.0), Value::Float(0.0)];
        let bx = vec![Value::Float(50.0), Value::Float(0.0)];
        let by = vec![Value::Float(0.0), Value::Float(50.0)];
        let d_x = FiberMetric::distance(&fields, &origin, &bx);
        let d_y = FiberMetric::distance(&fields, &origin, &by);
        assert!((d_x / d_y - 2.0).abs() < 1e-10);
    }

    // ── Sprint B: Metric Tensor ──

    use crate::bundle::BundleStore;
    use crate::types::BundleSchema;

    fn make_numeric_store() -> BundleStore {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(100.0))
            .fiber(FieldDef::numeric("y").with_range(100.0));
        let mut store = BundleStore::new(schema);
        for i in 0..20 {
            let mut r = crate::types::Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Float(i as f64));
            r.insert("y".into(), Value::Float(i as f64 * 2.0));
            store.insert(&r);
        }
        store
    }

    /// TDD-B.6: Metric tensor has correct dimensions.
    #[test]
    fn tdd_b6_metric_tensor_shape() {
        let store = make_numeric_store();
        let info = metric_tensor(&store);
        assert_eq!(info.field_names.len(), 2);
        assert_eq!(info.matrix.len(), 2);
        assert_eq!(info.matrix[0].len(), 2);
        assert_eq!(info.eigenvalues.len(), 2);
    }

    /// TDD-B.7: Condition number ≥ 1.
    #[test]
    fn tdd_b7_condition_number_bound() {
        let store = make_numeric_store();
        let info = metric_tensor(&store);
        assert!(
            info.condition_number >= 1.0,
            "κ = {} should be ≥ 1",
            info.condition_number
        );
    }

    /// TDD-B.8: Effective dimension in [1, n].
    #[test]
    fn tdd_b8_effective_dimension_bounds() {
        let store = make_numeric_store();
        let info = metric_tensor(&store);
        let n = info.field_names.len() as f64;
        assert!(
            info.effective_dimension >= 0.9 && info.effective_dimension <= n + 0.1,
            "d_eff = {} not in [1, {}]",
            info.effective_dimension,
            n
        );
    }

    /// TDD-B.9: Eigenvalues are non-negative.
    #[test]
    fn tdd_b9_eigenvalues_non_negative() {
        let store = make_numeric_store();
        let info = metric_tensor(&store);
        for (i, &ev) in info.eigenvalues.iter().enumerate() {
            assert!(
                ev >= -1e-10,
                "eigenvalue[{i}] = {ev} is negative"
            );
        }
    }

    /// TDD-B.10: Diagonal of correlation matrix ≈ 1.
    #[test]
    fn tdd_b10_diagonal_ones() {
        let store = make_numeric_store();
        let info = metric_tensor(&store);
        for i in 0..info.matrix.len() {
            assert!(
                (info.matrix[i][i] - 1.0).abs() < 1e-10,
                "G[{i},{i}] = {} should be ≈ 1",
                info.matrix[i][i]
            );
        }
    }

    /// TDD-B.11: Symmetry G[i][j] = G[j][i].
    #[test]
    fn tdd_b11_matrix_symmetry() {
        let store = make_numeric_store();
        let info = metric_tensor(&store);
        for i in 0..info.matrix.len() {
            for j in 0..info.matrix.len() {
                assert!(
                    (info.matrix[i][j] - info.matrix[j][i]).abs() < 1e-10,
                    "G[{i},{j}] = {} ≠ G[{j},{i}] = {}",
                    info.matrix[i][j],
                    info.matrix[j][i]
                );
            }
        }
    }

    /// TDD-B.12: Empty store produces trivial output.
    #[test]
    fn tdd_b12_empty_store() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(100.0));
        let store = BundleStore::new(schema);
        let info = metric_tensor(&store);
        assert!(info.matrix.is_empty());
        assert!(info.eigenvalues.is_empty());
    }

    /// TDD-B.13: Symmetric eigenvalues helper correctness.
    #[test]
    fn tdd_b13_eigenvalues_2x2() {
        // [[3, 1], [1, 3]] → eigenvalues 4 and 2
        let mat = vec![vec![3.0, 1.0], vec![1.0, 3.0]];
        let eigs = symmetric_eigenvalues(&mat, 2);
        assert!((eigs[0] - 4.0).abs() < 0.1, "λ₁ = {}, expected ≈ 4", eigs[0]);
        assert!((eigs[1] - 2.0).abs() < 0.1, "λ₂ = {}, expected ≈ 2", eigs[1]);
    }

    // ── KL Divergence TDD ─────────────────────────────────────────────────────

    fn make_concentrated_bundle(x_val: f64, n: usize) -> BundleStore {
        use crate::types::*;
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(1.0));
        let mut store = BundleStore::new(schema);
        for i in 0..n {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i as i64));
            r.insert("x".into(), Value::Float(x_val));
            store.insert(&r);
        }
        store
    }

    fn make_uniform_bundle(lo: f64, hi: f64, n: usize) -> BundleStore {
        use crate::types::*;
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(1.0));
        let mut store = BundleStore::new(schema);
        for i in 0..n {
            let x = lo + (hi - lo) * (i as f64 / (n - 1).max(1) as f64);
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i as i64));
            r.insert("x".into(), Value::Float(x));
            store.insert(&r);
        }
        store
    }

    /// KL-1: D_KL(P || P) = 0 for identical distributions.
    #[test]
    fn tdd_kl_1_same_distribution_is_zero() {
        let a = make_uniform_bundle(0.0, 1.0, 200);
        let b = make_uniform_bundle(0.0, 1.0, 200);
        let rep = kl_divergence(&a, &b);
        assert!(rep.kl_forward.abs() < 1e-9, "D_KL(P||P) = {}", rep.kl_forward);
        assert!(rep.jensen_shannon.abs() < 1e-9, "D_JS(P,P) = {}", rep.jensen_shannon);
    }

    /// KL-2: D_KL ≥ 0 (Gibbs inequality).
    #[test]
    fn tdd_kl_2_non_negative() {
        let a = make_concentrated_bundle(0.1, 200);
        let b = make_concentrated_bundle(0.9, 200);
        let rep = kl_divergence(&a, &b);
        assert!(rep.kl_forward >= 0.0, "D_KL(P||Q) = {} < 0", rep.kl_forward);
        assert!(rep.kl_reverse >= 0.0, "D_KL(Q||P) = {} < 0", rep.kl_reverse);
        assert!(rep.jensen_shannon >= 0.0, "D_JS = {} < 0", rep.jensen_shannon);
    }

    /// KL-3: D_JS ≤ ln(2) (upper bound on Jensen-Shannon divergence).
    #[test]
    fn tdd_kl_3_js_upper_bound() {
        let a = make_concentrated_bundle(0.0, 500);
        let b = make_concentrated_bundle(1.0, 500);
        let rep = kl_divergence(&a, &b);
        let ln2 = std::f64::consts::LN_2;
        assert!(
            rep.jensen_shannon <= ln2 + 1e-9,
            "D_JS = {} > ln(2) = {ln2}",
            rep.jensen_shannon
        );
    }

    /// KL-4: D_JS is symmetric.
    #[test]
    fn tdd_kl_4_js_symmetry() {
        let a = make_uniform_bundle(0.0, 0.3, 200);
        let b = make_uniform_bundle(0.7, 1.0, 200);
        let ab = kl_divergence(&a, &b);
        let ba = kl_divergence(&b, &a);
        assert!(
            (ab.jensen_shannon - ba.jensen_shannon).abs() < 1e-9,
            "D_JS(P,Q)={} ≠ D_JS(Q,P)={}",
            ab.jensen_shannon,
            ba.jensen_shannon
        );
    }

    /// KL-5: D_KL large when distributions are well-separated.
    #[test]
    fn tdd_kl_5_large_when_disjoint() {
        let a = make_concentrated_bundle(0.0, 500);
        let b = make_concentrated_bundle(1.0, 500);
        let rep = kl_divergence(&a, &b);
        assert!(rep.kl_forward > 0.5, "D_KL(P||Q) = {} expected > 0.5", rep.kl_forward);
    }

    /// KL-6: per_field contributes to total KL.
    #[test]
    fn tdd_kl_6_per_field_sum() {
        let a = make_uniform_bundle(0.0, 1.0, 100);
        let b = make_uniform_bundle(0.0, 1.0, 100);
        let rep = kl_divergence(&a, &b);
        let sum: f64 = rep.per_field.iter().map(|(_, v)| v).sum();
        // For identical distributions, all per-field contributions should be ≈ 0
        assert!(sum.abs() < 1e-9, "sum of per-field KL = {sum}");
        assert_eq!(rep.fields_compared, 1);
    }

    /// KL-7: Empty bundle returns zero divergence, no panic.
    #[test]
    fn tdd_kl_7_empty_bundle() {
        use crate::types::*;
        let schema = BundleSchema::new("empty")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(1.0));
        let empty = BundleStore::new(schema.clone());
        let other = make_uniform_bundle(0.0, 1.0, 50);
        let rep = kl_divergence(&empty, &other);
        // No common fields with data → fields_compared = 0
        assert_eq!(rep.kl_forward, 0.0);
    }

    /// KL-8: Freedman-Diaconis bins in [10, 200].
    #[test]
    fn tdd_kl_8_bins_clamped() {
        // 3 values → fallback to 10
        let tiny: Vec<f64> = vec![0.1, 0.5, 0.9];
        let b = freedman_diaconis_bins(&tiny);
        assert!(b >= 10 && b <= 200, "bins = {b}");

        // Large uniform → more bins but still ≤ 200
        let large: Vec<f64> = (0..10000).map(|i| i as f64).collect();
        let b2 = freedman_diaconis_bins(&large);
        assert!(b2 >= 10 && b2 <= 200, "bins = {b2}");
    }
}
