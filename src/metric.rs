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
}
