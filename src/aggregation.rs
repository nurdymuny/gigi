//! Fiber integration / aggregation — §5: Theorems 5.1–5.2.

use std::collections::HashMap;

use crate::bundle::{BundleStore, QueryCondition};
use crate::types::Value;

/// Fiber integral result for a single aggregation.
#[derive(Debug)]
pub struct AggResult {
    pub count: usize,
    pub sum: f64,
    pub sum_sq: f64,
    pub min: f64,
    pub max: f64,
}

impl AggResult {
    pub fn avg(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.sum / self.count as f64
        }
    }
    pub fn variance(&self) -> f64 {
        if self.count < 2 {
            return 0.0;
        }
        let n = self.count as f64;
        (self.sum_sq / n) - (self.sum / n).powi(2)
    }
    pub fn stddev(&self) -> f64 {
        self.variance().sqrt()
    }
}

/// Compute COUNT, SUM, AVG, MIN, MAX over a fiber field for all records (Thm 5.1).
pub fn fiber_integral(store: &BundleStore, field: &str) -> AggResult {
    let mut result = AggResult {
        count: 0,
        sum: 0.0,
        sum_sq: 0.0,
        min: f64::INFINITY,
        max: f64::NEG_INFINITY,
    };
    for rec in store.records() {
        if let Some(v) = rec.get(field).and_then(|v| v.as_f64()) {
            result.count += 1;
            result.sum += v;
            result.sum_sq += v * v;
            result.min = result.min.min(v);
            result.max = result.max.max(v);
        }
    }
    result
}

/// GROUP BY via base space partition (Thm 5.2).
///
/// Returns map from group_value → AggResult for the aggregated field.
pub fn group_by(
    store: &BundleStore,
    group_field: &str,
    agg_field: &str,
) -> HashMap<Value, AggResult> {
    let mut groups: HashMap<Value, AggResult> = HashMap::new();

    for rec in store.records() {
        let group_val = match rec.get(group_field) {
            Some(v) => v.clone(),
            None => continue,
        };
        let agg_val = match rec.get(agg_field).and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => continue,
        };

        let entry = groups.entry(group_val).or_insert(AggResult {
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        });
        entry.count += 1;
        entry.sum += agg_val;
        entry.sum_sq += agg_val * agg_val;
        entry.min = entry.min.min(agg_val);
        entry.max = entry.max.max(agg_val);
    }
    groups
}

/// Filtered GROUP BY — aggregate only records matching conditions (Sprint 2).
pub fn filtered_group_by(
    store: &BundleStore,
    group_field: &str,
    agg_field: &str,
    conditions: &[QueryCondition],
) -> HashMap<Value, AggResult> {
    let mut groups: HashMap<Value, AggResult> = HashMap::new();

    for rec in store.records() {
        if !crate::bundle::matches_filter(&rec, conditions, None) {
            continue;
        }

        let group_val = match rec.get(group_field) {
            Some(v) => v.clone(),
            None => continue,
        };
        let agg_val = match rec.get(agg_field).and_then(|v| v.as_f64()) {
            Some(v) => v,
            None => continue,
        };

        let entry = groups.entry(group_val).or_insert(AggResult {
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        });
        entry.count += 1;
        entry.sum += agg_val;
        entry.sum_sq += agg_val * agg_val;
        entry.min = entry.min.min(agg_val);
        entry.max = entry.max.max(agg_val);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleStore;
    use crate::types::*;

    fn make_store() -> BundleStore {
        let schema = BundleSchema::new("employees")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("dept"))
            .fiber(FieldDef::numeric("salary").with_range(100000.0))
            .index("dept");
        let mut store = BundleStore::new(schema);
        let depts = ["Eng", "Sales", "HR", "Mkt", "Ops"];
        for i in 0..100 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("dept".into(), Value::Text(depts[i as usize % 5].into()));
            r.insert("salary".into(), Value::Float(40000.0 + (i as f64) * 500.0));
            store.insert(&r);
        }
        store
    }

    /// TDD-5.1: COUNT, SUM, AVG accuracy.
    #[test]
    fn tdd_5_1_fiber_integral() {
        let store = make_store();
        let agg = fiber_integral(&store, "salary");
        assert_eq!(agg.count, 100);

        let expected_sum: f64 = (0..100).map(|i| 40000.0 + i as f64 * 500.0).sum();
        assert!((agg.sum - expected_sum).abs() < 0.01);
        assert!((agg.avg() - expected_sum / 100.0).abs() < 0.01);
    }

    /// TDD-5.2: GROUP BY produces correct number of groups.
    #[test]
    fn tdd_5_2_group_by_partition() {
        let store = make_store();
        let groups = group_by(&store, "dept", "salary");
        assert_eq!(groups.len(), 5);
    }

    /// GAP-F.1 / GAP-F.2: MIN/MAX fiber integrals.
    #[test]
    fn gap_f_min_max() {
        let store = make_store();
        let agg = fiber_integral(&store, "salary");
        assert!((agg.min - 40000.0).abs() < 0.01);
        assert!((agg.max - (40000.0 + 99.0 * 500.0)).abs() < 0.01);
    }

    /// GAP-F.3: GROUP BY MIN/MAX matches expected.
    #[test]
    fn gap_f3_group_min_max() {
        let store = make_store();
        let groups = group_by(&store, "dept", "salary");
        // Each dept gets every 5th employee, so:
        // Eng: ids 0,5,10,...,95 → salaries 40000, 42500, ...
        let eng = groups.get(&Value::Text("Eng".into())).unwrap();
        assert!((eng.min - 40000.0).abs() < 0.01); // id=0
        assert!((eng.max - (40000.0 + 95.0 * 500.0)).abs() < 0.01); // id=95
    }

    /// HAVING: filtered_group_by + manual post-filter (same logic as REST handler).
    #[test]
    fn test_having_count_gt() {
        let store = make_store();
        // All depts have exactly 20 records each (100 / 5)
        let groups = group_by(&store, "dept", "salary");
        // HAVING count > 25 → no groups should survive (all have 20)
        let filtered: HashMap<_, _> = groups.iter().filter(|(_, agg)| agg.count > 25).collect();
        assert!(filtered.is_empty(), "all depts have 20 records, none > 25");

        // HAVING count >= 20 → all 5 depts
        let all_groups = group_by(&store, "dept", "salary");
        let filtered_all: HashMap<_, _> = all_groups
            .iter()
            .filter(|(_, agg)| agg.count >= 20)
            .collect();
        assert_eq!(
            filtered_all.len(),
            5,
            "all 5 depts have at least 20 records"
        );
    }

    #[test]
    fn test_having_avg_gt() {
        let store = make_store();
        // Eng dept: ids 0,5,10,...,95 → avg salary = 40000 + 47.5 * 500 = 63750
        // All depts should have avg > 50000 since min salary is 40000 and there are 100 records
        let groups = group_by(&store, "dept", "salary");
        let above_50k: HashMap<_, _> = groups
            .iter()
            .filter(|(_, agg)| agg.avg() > 50000.0)
            .collect();
        // With 100 records and salaries 40000–89500, avg per group will be ~64750
        assert_eq!(above_50k.len(), 5, "all dept avgs should exceed 50000");
    }

    #[test]
    fn test_filtered_group_by_with_condition() {
        let store = make_store();
        // Only include Eng and Sales departments
        let conditions = vec![crate::bundle::QueryCondition::In(
            "dept".into(),
            vec![Value::Text("Eng".into()), Value::Text("Sales".into())],
        )];
        let groups = filtered_group_by(&store, "dept", "salary", &conditions);
        assert_eq!(groups.len(), 2, "only Eng and Sales should be grouped");
        assert!(groups.contains_key(&Value::Text("Eng".into())));
        assert!(groups.contains_key(&Value::Text("Sales".into())));
    }
}
