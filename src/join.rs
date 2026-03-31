//! Pullback join — §4: Bundle Morphisms.
//!
//! Implements Theorem 4.1: O(|left|) join via pullback.

use std::collections::HashSet;

use crate::bundle::BundleStore;
use crate::types::{BundleSchema, FieldDef, Record, Value};

/// Pullback join: for each record in `left`, look up the matching record
/// in `right` via the foreign key relationship.
///
/// left_field: the FK field in the left bundle
/// right_field: the PK field in the right bundle
///
/// Complexity: O(|left|) — each left record does one O(1) lookup in right.
pub fn pullback_join(
    left: &BundleStore,
    right: &BundleStore,
    left_field: &str,
    right_field: &str,
) -> Vec<(Record, Option<Record>)> {
    let mut results = Vec::with_capacity(left.len());

    for left_rec in left.records() {
        let fk_value = left_rec.get(left_field).cloned().unwrap_or(Value::Null);

        // Build a key record for the right bundle lookup
        let mut right_key = Record::new();
        right_key.insert(right_field.to_string(), fk_value);

        let right_rec = right.point_query(&right_key);
        results.push((left_rec, right_rec));
    }
    results
}

/// Report from pullback curvature computation.
#[derive(Debug, Clone)]
pub struct PullbackReport {
    /// Curvature of the left (source) bundle.
    pub k_left: f64,
    /// Curvature of the right (target) bundle.
    pub k_right: f64,
    /// Curvature of the pullback (merged) data.
    pub k_pullback: f64,
    /// ΔK = K_pullback - K_left.
    pub delta_k: f64,
    /// Number of left records with a successful FK match.
    pub matched: usize,
    /// Number of left records with no right match (null fiber).
    pub unmatched: usize,
    /// Number of right records that no left record mapped to.
    pub right_unmatched: usize,
}

/// Pullback curvature: measures how joining two bundles distorts geometry.
///
/// Builds a merged bundle with fiber fields from both sides, then computes
/// scalar curvature on the merged data. ΔK = K_merged - K_left.
///
/// A faithful join (ΔK ≈ 0) preserves geometric structure.
pub fn pullback_curvature(
    left: &BundleStore,
    right: &BundleStore,
    left_field: &str,
    right_field: &str,
) -> PullbackReport {
    let k_left = crate::curvature::scalar_curvature(left);
    let k_right = crate::curvature::scalar_curvature(right);

    let joined = pullback_join(left, right, left_field, right_field);

    // Build merged schema: left base + fibers from both sides
    let mut merged_schema = BundleSchema::new("pullback");
    for bf in &left.schema.base_fields {
        merged_schema = merged_schema.base(bf.clone());
    }
    let mut seen_fields: HashSet<String> = HashSet::new();
    for ff in &left.schema.fiber_fields {
        merged_schema = merged_schema.fiber(ff.clone());
        seen_fields.insert(ff.name.clone());
    }
    for ff in &right.schema.fiber_fields {
        if !seen_fields.contains(&ff.name)
            && !left.schema.base_fields.iter().any(|b| b.name == ff.name)
        {
            merged_schema = merged_schema.fiber(ff.clone());
        }
    }

    let mut merged_store = BundleStore::new(merged_schema);
    let mut matched = 0usize;
    let mut unmatched = 0usize;
    let mut matched_right_keys: HashSet<String> = HashSet::new();

    for (left_rec, right_rec_opt) in &joined {
        let mut merged = left_rec.clone();
        if let Some(right_rec) = right_rec_opt {
            matched += 1;
            // Track which right keys were hit
            if let Some(v) = right_rec.get(right_field) {
                matched_right_keys.insert(format!("{:?}", v));
            }
            for (k, v) in right_rec.iter() {
                if !merged.contains_key(k) {
                    merged.insert(k.clone(), v.clone());
                }
            }
        } else {
            unmatched += 1;
        }
        merged_store.insert(&merged);
    }

    // Count right records that nothing mapped to
    let total_right = right.len();
    let right_unmatched = total_right.saturating_sub(matched_right_keys.len());

    let k_pullback = crate::curvature::scalar_curvature(&merged_store);

    PullbackReport {
        k_left,
        k_right,
        k_pullback,
        delta_k: k_pullback - k_left,
        matched,
        unmatched,
        right_unmatched,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn make_orders() -> BundleStore {
        let schema = BundleSchema::new("orders")
            .base(FieldDef::numeric("order_id"))
            .fiber(FieldDef::numeric("customer_id"))
            .fiber(FieldDef::numeric("amount").with_range(1000.0));
        let mut store = BundleStore::new(schema);
        for i in 0..100 {
            let mut r = Record::new();
            r.insert("order_id".into(), Value::Integer(i));
            r.insert("customer_id".into(), Value::Integer(i % 10));
            r.insert("amount".into(), Value::Float(100.0 + i as f64));
            store.insert(&r);
        }
        store
    }

    fn make_customers() -> BundleStore {
        let schema = BundleSchema::new("customers")
            .base(FieldDef::numeric("customer_id"))
            .fiber(FieldDef::categorical("name"));
        let mut store = BundleStore::new(schema);
        for i in 0..10 {
            let mut r = Record::new();
            r.insert("customer_id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("Customer_{i}")));
            store.insert(&r);
        }
        store
    }

    /// TDD-4.1: Pullback join correctness.
    #[test]
    fn tdd_4_1_correctness() {
        let orders = make_orders();
        let customers = make_customers();
        let results = pullback_join(&orders, &customers, "customer_id", "customer_id");

        assert_eq!(results.len(), 100);
        for (order, customer) in &results {
            let cust_id = order.get("customer_id").unwrap();
            if let Some(cust_rec) = customer {
                assert_eq!(cust_rec.get("customer_id").unwrap(), cust_id);
            }
        }
    }

    /// TDD-4.2: |result| = |left|.
    #[test]
    fn tdd_4_2_cardinality() {
        let orders = make_orders();
        let customers = make_customers();
        let results = pullback_join(&orders, &customers, "customer_id", "customer_id");
        assert_eq!(results.len(), orders.len());
    }

    /// TDD-4.3: Missing FK → null fiber.
    #[test]
    fn tdd_4_3_null_fiber() {
        let schema = BundleSchema::new("orders")
            .base(FieldDef::numeric("order_id"))
            .fiber(FieldDef::numeric("customer_id"));
        let mut orders = BundleStore::new(schema);
        let mut r = Record::new();
        r.insert("order_id".into(), Value::Integer(1));
        r.insert("customer_id".into(), Value::Integer(9999)); // doesn't exist
        orders.insert(&r);

        let customers = make_customers();
        let results = pullback_join(&orders, &customers, "customer_id", "customer_id");
        assert_eq!(results.len(), 1);
        assert!(results[0].1.is_none());
    }

    // ── Pullback curvature ─────────────────────────────────────────

    /// TDD-4.4: Faithful join → small ΔK and all matched.
    #[test]
    fn tdd_4_4_faithful_join_curvature() {
        let orders = make_orders();
        let customers = make_customers();
        let report = pullback_curvature(&orders, &customers, "customer_id", "customer_id");
        assert_eq!(report.matched, 100);
        assert_eq!(report.unmatched, 0);
        // All 10 customers are referenced by orders
        assert_eq!(report.right_unmatched, 0);
        assert!(
            report.delta_k.abs() < 1.0,
            "ΔK = {}, expected small for faithful join",
            report.delta_k
        );
    }

    /// TDD-4.5: No-match join → all unmatched.
    #[test]
    fn tdd_4_5_no_match_curvature() {
        let schema = BundleSchema::new("left")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("fk"));
        let mut left = BundleStore::new(schema);
        for i in 100..110 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("fk".into(), Value::Integer(i));
            left.insert(&r);
        }
        let customers = make_customers();
        let report = pullback_curvature(&left, &customers, "fk", "customer_id");
        assert_eq!(report.unmatched, 10);
        assert_eq!(report.matched, 0);
        // All 10 customers are unmatched since left FKs don't hit them
        assert_eq!(report.right_unmatched, 10);
    }

    /// TDD-4.6: Report fields are finite and consistent.
    #[test]
    fn tdd_4_6_report_consistency() {
        let orders = make_orders();
        let customers = make_customers();
        let report = pullback_curvature(&orders, &customers, "customer_id", "customer_id");
        assert!(report.k_left.is_finite());
        assert!(report.k_right.is_finite());
        assert!(report.k_pullback.is_finite());
        assert!(
            (report.delta_k - (report.k_pullback - report.k_left)).abs() < 1e-10,
            "ΔK should equal K_pullback - K_left"
        );
        assert_eq!(report.matched + report.unmatched, 100);
        // right_unmatched should be finite and logical
        assert!(report.right_unmatched <= 10, "at most 10 right records");
    }
}
