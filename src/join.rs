//! Pullback join — §4: Bundle Morphisms.
//!
//! Implements Theorem 4.1: O(|left|) join via pullback.

use crate::bundle::BundleStore;
use crate::types::{Record, Value};

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
}
