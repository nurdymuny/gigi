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
///
/// When `agg_field == "*"` the function counts every record in each
/// group regardless of any field's nullness, skipping sum/min/max
/// updates (they have no meaning without a value field). This is the
/// path COUNT(*) uses when no other measure picks a real agg field.
pub fn group_by(
    store: &BundleStore,
    group_field: &str,
    agg_field: &str,
) -> HashMap<Value, AggResult> {
    let mut groups: HashMap<Value, AggResult> = HashMap::new();
    let count_only = agg_field == "*";

    for rec in store.records() {
        let group_val = match rec.get(group_field) {
            Some(v) => v.clone(),
            None => continue,
        };
        let agg_val = if count_only {
            0.0
        } else {
            match rec.get(agg_field).and_then(|v| v.as_f64()) {
                Some(v) => v,
                None => continue,
            }
        };

        let entry = groups.entry(group_val).or_insert(AggResult {
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        });
        entry.count += 1;
        if !count_only {
            entry.sum += agg_val;
            entry.sum_sq += agg_val * agg_val;
            entry.min = entry.min.min(agg_val);
            entry.max = entry.max.max(agg_val);
        }
    }
    groups
}

/// Per-measure GROUP BY — one `AggResult` per requested field, computed
/// in a single pass over the records.
///
/// This is the multi-measure form `INTEGRATE ... MEASURE f(a), g(b)`
/// needs: the single-field `group_by` forces every measure in a
/// statement to read the same accumulator, so a second `min()` over a
/// different field silently returns the first field's value.
///
/// Field semantics per accumulator:
/// - `"*"` — count-only: `count` is the number of records in the group.
/// - named field — `count` is the number of records where the field is
///   present and non-null (SQL `COUNT(field)`), whatever its type;
///   sum/min/max accumulate over the values that are numeric. For a
///   non-numeric field min/max stay at their empty sentinels
///   (`INFINITY` / `NEG_INFINITY`) — callers should surface those as
///   null rather than serialize them.
pub fn group_by_measures<I>(
    records: I,
    group_field: &str,
    fields: &[&str],
) -> HashMap<Value, Vec<AggResult>>
where
    I: IntoIterator<Item = crate::types::Record>,
{
    let mut groups: HashMap<Value, Vec<AggResult>> = HashMap::new();
    for rec in records {
        let group_val = match rec.get(group_field) {
            Some(v) => v.clone(),
            None => continue,
        };
        let entry = groups
            .entry(group_val)
            .or_insert_with(|| empty_measures(fields.len()));
        accumulate_measures(entry, &rec, fields);
    }
    groups
}

/// Global (no GROUP BY) form of [`group_by_measures`]: one `AggResult`
/// per requested field over every record. Same field semantics.
pub fn integrate_measures<I>(records: I, fields: &[&str]) -> Vec<AggResult>
where
    I: IntoIterator<Item = crate::types::Record>,
{
    let mut aggs = empty_measures(fields.len());
    for rec in records {
        accumulate_measures(&mut aggs, &rec, fields);
    }
    aggs
}

fn empty_measures(n: usize) -> Vec<AggResult> {
    (0..n)
        .map(|_| AggResult {
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            min: f64::INFINITY,
            max: f64::NEG_INFINITY,
        })
        .collect()
}

fn accumulate_measures(aggs: &mut [AggResult], rec: &crate::types::Record, fields: &[&str]) {
    for (agg, field) in aggs.iter_mut().zip(fields) {
        if *field == "*" {
            agg.count += 1;
            continue;
        }
        let Some(v) = rec.get(*field) else { continue };
        if matches!(v, Value::Null) {
            continue;
        }
        agg.count += 1;
        if let Some(x) = v.as_f64() {
            agg.sum += x;
            agg.sum_sq += x * x;
            agg.min = agg.min.min(x);
            agg.max = agg.max.max(x);
        }
    }
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

    /// Multi-measure GROUP BY: each measure aggregates its OWN field.
    /// Regression for GQL `INTEGRATE ... MEASURE min(a), min(b)` returning
    /// min(a) for both columns.
    #[test]
    fn test_group_by_measures_distinct_fields() {
        let store = make_store();
        let groups = group_by_measures(store.records(), "dept", &["salary", "id"]);
        assert_eq!(groups.len(), 5);
        let eng = groups.get(&Value::Text("Eng".into())).unwrap();
        assert_eq!(eng.len(), 2);
        // salary accumulator: min 40000; id accumulator: min 0, max 95
        assert!((eng[0].min - 40000.0).abs() < 0.01);
        assert!((eng[1].min - 0.0).abs() < 0.01);
        assert!((eng[1].max - 95.0).abs() < 0.01);
        // the two accumulators must NOT alias each other
        assert!((eng[0].min - eng[1].min).abs() > 1.0);
    }

    /// COUNT(*) counts every record in the group; COUNT(text_field)
    /// counts presence, not numeric-ness. Regression for GQL INTEGRATE
    /// silently returning an empty result set for both.
    #[test]
    fn test_group_by_measures_count_star_and_text() {
        let store = make_store();
        let groups = group_by_measures(store.records(), "dept", &["*", "dept", "salary"]);
        assert_eq!(groups.len(), 5);
        for aggs in groups.values() {
            assert_eq!(aggs[0].count, 20, "count(*) counts all group records");
            assert_eq!(aggs[1].count, 20, "count(text field) counts presence");
            // text field never accumulates numerics — sentinels intact
            assert!(aggs[1].min.is_infinite() && aggs[1].max.is_infinite());
            assert_eq!(aggs[2].count, 20);
        }
    }

    /// Global (no OVER) multi-measure aggregation over every record.
    #[test]
    fn test_integrate_measures_global() {
        let store = make_store();
        let aggs = integrate_measures(store.records(), &["*", "salary", "id"]);
        assert_eq!(aggs[0].count, 100);
        assert!((aggs[1].min - 40000.0).abs() < 0.01);
        assert!((aggs[2].max - 99.0).abs() < 0.01);
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
