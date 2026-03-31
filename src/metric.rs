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
}
