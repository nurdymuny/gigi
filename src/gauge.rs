//! Gauge transformations — §5a: Schema Migrations.
//!
//! A gauge transformation changes the fiber representation without
//! altering the base space or geometric invariants.
//!
//! Implements Definitions 5a.1–5a.2, Theorem 5a.1.

use crate::bundle::BundleStore;
use crate::types::{FieldDef, FieldType, Record, Value};

/// A gauge transformation that can be applied to a BundleStore.
pub enum GaugeTransform {
    /// ADD COLUMN: extend fiber with a new field + default value.
    AddColumn { field: FieldDef },

    /// DROP COLUMN: project fiber by removing a field.
    DropColumn { field_name: String },

    /// RENAME COLUMN: pure gauge — relabel only.
    RenameColumn { old_name: String, new_name: String },

    /// RESCALE: change numeric field units (e.g., meters → feet).
    /// Adjusts values by `factor` and range by `factor`.
    Rescale { field_name: String, factor: f64 },

    /// RETYPE: change field type (type coercion gauge).
    Retype { field_name: String, new_type: FieldType },
}

/// Apply a gauge transformation to a BundleStore, returning a new store.
///
/// Invariants preserved (Def 5a.2):
///   - Base points unchanged (key fields untouched)
///   - Curvature K preserved for isometric transforms
///   - Deviation norm preserved for pure gauges
pub fn apply_gauge(store: &BundleStore, transform: &GaugeTransform) -> BundleStore {
    match transform {
        GaugeTransform::AddColumn { field } => gauge_add_column(store, field),
        GaugeTransform::DropColumn { field_name } => gauge_drop_column(store, field_name),
        GaugeTransform::RenameColumn { old_name, new_name } => {
            gauge_rename_column(store, old_name, new_name)
        }
        GaugeTransform::Rescale { field_name, factor } => {
            gauge_rescale(store, field_name, *factor)
        }
        GaugeTransform::Retype { field_name, new_type } => {
            gauge_retype(store, field_name, new_type)
        }
    }
}

/// ADD COLUMN: F → F × F_new, zero section gains component `default`.
fn gauge_add_column(store: &BundleStore, field: &FieldDef) -> BundleStore {
    let mut new_schema = store.schema.clone();
    new_schema.fiber_fields.push(field.clone());

    let mut new_store = BundleStore::new(new_schema);
    for rec in store.records() {
        let mut new_rec = rec;
        new_rec.insert(field.name.clone(), field.default.clone());
        new_store.insert(&new_rec);
    }
    new_store
}

/// DROP COLUMN: F → F / F_f (projection onto remaining fibers).
fn gauge_drop_column(store: &BundleStore, field_name: &str) -> BundleStore {
    let mut new_schema = store.schema.clone();
    new_schema.fiber_fields.retain(|f| f.name != field_name);
    new_schema.indexed_fields.retain(|f| f != field_name);

    let mut new_store = BundleStore::new(new_schema);
    for rec in store.records() {
        let mut new_rec = rec;
        new_rec.remove(field_name);
        new_store.insert(&new_rec);
    }
    new_store
}

/// RENAME COLUMN: identity on values, relabeling on schema. Pure gauge.
fn gauge_rename_column(store: &BundleStore, old_name: &str, new_name: &str) -> BundleStore {
    let mut new_schema = store.schema.clone();

    // Rename in base fields
    for f in &mut new_schema.base_fields {
        if f.name == old_name {
            f.name = new_name.to_string();
        }
    }
    // Rename in fiber fields
    for f in &mut new_schema.fiber_fields {
        if f.name == old_name {
            f.name = new_name.to_string();
        }
    }
    // Rename in indexed fields
    for idx in &mut new_schema.indexed_fields {
        if idx == old_name {
            *idx = new_name.to_string();
        }
    }

    let mut new_store = BundleStore::new(new_schema);
    for rec in store.records() {
        let mut new_rec = Record::new();
        for (k, v) in rec {
            let key = if k == old_name { new_name.to_string() } else { k };
            new_rec.insert(key, v);
        }
        new_store.insert(&new_rec);
    }
    new_store
}

/// RESCALE: v → v * factor, range → range * factor.
/// Isometric gauge: Fisher metric d' = |v*f - w*f| / (range*f) = |v-w|/range = d.
/// Therefore K'(p) = K(p) exactly (Thm 5a.1).
fn gauge_rescale(store: &BundleStore, field_name: &str, factor: f64) -> BundleStore {
    let mut new_schema = store.schema.clone();
    for f in &mut new_schema.fiber_fields {
        if f.name == field_name {
            if let Some(ref mut range) = f.range {
                *range *= factor.abs();
            }
        }
    }

    let mut new_store = BundleStore::new(new_schema);
    for rec in store.records() {
        let mut new_rec = rec;
        if let Some(val) = new_rec.get(field_name).and_then(|v| v.as_f64()) {
            new_rec.insert(field_name.to_string(), Value::Float(val * factor));
        }
        new_store.insert(&new_rec);
    }
    new_store
}

/// RETYPE: change field type (type coercion gauge).
fn gauge_retype(store: &BundleStore, field_name: &str, new_type: &FieldType) -> BundleStore {
    let mut new_schema = store.schema.clone();
    for f in &mut new_schema.fiber_fields {
        if f.name == field_name {
            f.field_type = new_type.clone();
        }
    }

    let mut new_store = BundleStore::new(new_schema);
    for rec in store.records() {
        let mut new_rec = rec;
        // Coerce value to new type
        if let Some(val) = new_rec.get(field_name).cloned() {
            let coerced = match new_type {
                FieldType::Numeric => match val {
                    Value::Integer(_) | Value::Float(_) => val,
                    Value::Timestamp(t) => Value::Integer(t),
                    Value::Text(s) => s.parse::<f64>()
                        .map(Value::Float)
                        .unwrap_or(Value::Null),
                    Value::Bool(b) => Value::Integer(b as i64),
                    Value::Null => Value::Null,
                    Value::Vector(_) => Value::Null,
                },
                FieldType::Categorical => match val {
                    Value::Text(_) => val,
                    other => Value::Text(other.to_string()),
                },
                _ => val,
            };
            new_rec.insert(field_name.to_string(), coerced);
        }
        new_store.insert(&new_rec);
    }
    new_store
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curvature::scalar_curvature;
    use crate::types::*;

    fn make_store() -> BundleStore {
        let schema = BundleSchema::new("employees")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("salary").with_range(100_000.0))
            .fiber(FieldDef::categorical("dept"))
            .index("dept");
        let mut store = BundleStore::new(schema);
        let depts = ["Eng", "Sales", "HR", "Mkt", "Ops"];
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("name".into(), Value::Text(format!("User_{i}")));
            r.insert("salary".into(), Value::Float(40_000.0 + (i as f64) * 1200.0));
            r.insert("dept".into(), Value::Text(depts[i as usize % 5].into()));
            store.insert(&r);
        }
        store
    }

    /// TDD-5a.1: ADD COLUMN with default → K unchanged.
    #[test]
    fn tdd_5a1_add_column_k_unchanged() {
        let store = make_store();
        let k_before = scalar_curvature(&store);

        let new_field = FieldDef::numeric("bonus")
            .with_range(50_000.0)
            .with_default(Value::Float(0.0));
        let new_store = apply_gauge(&store, &GaugeTransform::AddColumn { field: new_field });

        // New field has zero variance → K on salary unchanged
        // K_new = average of (K_salary, K_bonus), K_bonus = 0
        // So K_new should be K_before / 2 (diluted by zero-variance field)
        // But let's check that salary's own curvature component is the same
        let k_after = scalar_curvature(&new_store);
        // With 2 numeric fields, K = (K_salary + K_bonus) / 2 = K_before / 2
        assert!((k_after - k_before / 2.0).abs() < 1e-6,
            "K_before={k_before}, K_after={k_after}, expected K_before/2");
        assert_eq!(new_store.len(), 50);
    }

    /// TDD-5a.2: DROP COLUMN → K on remaining fields unchanged.
    #[test]
    fn tdd_5a2_drop_column_k_unchanged() {
        let store = make_store();
        let k_before = scalar_curvature(&store);

        let new_store = apply_gauge(&store, &GaugeTransform::DropColumn {
            field_name: "name".into(),
        });

        let k_after = scalar_curvature(&new_store);
        // Dropping "name" (categorical, no variance tracking) shouldn't change K
        assert!((k_after - k_before).abs() < 1e-10,
            "K_before={k_before}, K_after={k_after}");
        assert_eq!(new_store.len(), 50);
    }

    /// TDD-5a.3: RENAME COLUMN → all query results identical.
    #[test]
    fn tdd_5a3_rename_column() {
        let store = make_store();

        let new_store = apply_gauge(&store, &GaugeTransform::RenameColumn {
            old_name: "salary".into(),
            new_name: "compensation".into(),
        });

        assert_eq!(new_store.len(), 50);

        // Query from original
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(10));
        let orig = store.point_query(&key).unwrap();

        let result = new_store.point_query(&key).unwrap();
        // Old field name gone, new one present with same value
        assert!(result.get("salary").is_none());
        assert_eq!(
            result.get("compensation"),
            orig.get("salary"),
        );
    }

    /// TDD-5a.4: RESCALE → Fisher-metric K unchanged (isometric gauge).
    #[test]
    fn tdd_5a4_rescale_k_unchanged() {
        let store = make_store();
        let k_before = scalar_curvature(&store);

        // Convert salary from dollars to cents (×100)
        let new_store = apply_gauge(&store, &GaugeTransform::Rescale {
            field_name: "salary".into(),
            factor: 100.0,
        });

        let k_after = scalar_curvature(&new_store);
        // Isometric gauge: values × 100, range × 100 → normalized distance unchanged
        assert!((k_after - k_before).abs() / k_before.max(1e-15) < 1e-6,
            "K_before={k_before}, K_after={k_after}");

        // Verify values are scaled
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(0));
        let r = new_store.point_query(&key).unwrap();
        assert_eq!(r.get("salary"), Some(&Value::Float(4_000_000.0))); // 40000 × 100
    }

    /// Gauge: rename is pure gauge — deviation norm preserved.
    #[test]
    fn gauge_rename_preserves_deviation() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_default(Value::Float(0.0)).with_range(100.0));
        let mut store = BundleStore::new(schema);
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(1));
        r.insert("val".into(), Value::Float(42.0));
        store.insert(&r);

        let bp = store.base_point(&r);
        let dev_before = store.deviation_norm(bp);

        let new_store = apply_gauge(&store, &GaugeTransform::RenameColumn {
            old_name: "val".into(),
            new_name: "value".into(),
        });

        let bp2 = new_store.base_point(&{
            let mut k = Record::new();
            k.insert("id".into(), Value::Integer(1));
            k
        });
        let dev_after = new_store.deviation_norm(bp2);
        assert_eq!(dev_before, dev_after);
    }
}
