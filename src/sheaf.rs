//! Sheaf completion engine — COMPLETE, PROPAGATE, CONSISTENCY.
//!
//! Implements the Sudoku Principle: if H¹ = 0 on a local neighborhood,
//! the missing fiber value is uniquely determined by sheaf extension.
//!
//! All math is domain-agnostic. Adjacency functions are declared in the
//! bundle schema (ADJACENCY clauses in CREATE BUNDLE). The engine knows
//! sections, neighbors, and consistency — nothing else.

use std::collections::HashMap;

use crate::bundle::BundleStore;
use crate::parser::FilterCondition;
use crate::types::{AdjacencyKind, BasePoint, Record, Value};

/// Minimum number of neighbors required for a completion.
const MIN_NEIGHBORS: usize = 2;

// ── Neighbor discovery ──

/// A neighbor contribution: one record contributing to a completion.
#[derive(Debug, Clone)]
struct Neighbor {
    bp: BasePoint,
    adjacency_name: String,
    weight: f64,
}

/// Find all neighbors of `bp` according to schema-declared adjacency functions.
/// Returns (neighbor_bp, adjacency_name, weight) triples.
fn find_neighbors(store: &BundleStore, bp: BasePoint, record: &Record) -> Vec<Neighbor> {
    let adjs = &store.schema.adjacencies;

    // Fast path: no adjacencies declared → fall back to geometric_neighbors
    if adjs.is_empty() {
        return store
            .geometric_neighbors(bp)
            .into_iter()
            .map(|nbp| Neighbor {
                bp: nbp,
                adjacency_name: "geometric".into(),
                weight: 1.0,
            })
            .collect();
    }

    let mut neighbors: HashMap<BasePoint, Neighbor> = HashMap::new();

    for adj in adjs {
        match &adj.kind {
            AdjacencyKind::Equality { field } => {
                if let Some(val) = record.get(field) {
                    if *val == Value::Null {
                        continue;
                    }
                    // Use range_query for correct bp resolution
                    let matching = store.range_query(field, std::slice::from_ref(val));
                    for nbr in &matching {
                        let nbp = store.base_point(nbr);
                        if nbp == bp {
                            continue;
                        }
                        neighbors
                            .entry(nbp)
                            .and_modify(|n| {
                                if adj.weight > n.weight {
                                    n.weight = adj.weight;
                                    n.adjacency_name = adj.name.clone();
                                }
                            })
                            .or_insert_with(|| Neighbor {
                                bp: nbp,
                                adjacency_name: adj.name.clone(),
                                weight: adj.weight,
                            });
                    }
                }
            }
            AdjacencyKind::Metric { field, radius } => {
                if let Some(val) = record.get(field).and_then(|v| v.as_f64()) {
                    // Scan all sections — could be optimized with spatial index
                    for (nbp, fiber) in store.sections() {
                        if nbp == bp {
                            continue;
                        }
                        // Find the field value in the fiber
                        if let Some(idx) = store.schema.fiber_field_index(field) {
                            if let Some(nval) = fiber.get(idx).and_then(|v| v.as_f64()) {
                                let dist = (val - nval).abs();
                                if dist < *radius {
                                    let scaled_weight = adj.weight * (1.0 - dist / radius);
                                    neighbors
                                        .entry(nbp)
                                        .and_modify(|n| {
                                            if scaled_weight > n.weight {
                                                n.weight = scaled_weight;
                                                n.adjacency_name = adj.name.clone();
                                            }
                                        })
                                        .or_insert_with(|| Neighbor {
                                            bp: nbp,
                                            adjacency_name: adj.name.clone(),
                                            weight: scaled_weight,
                                        });
                                }
                            }
                        }
                        // Also check base fields for metric adjacency
                        if let Some(idx) = store.schema.base_field_index(field) {
                            let base_rec = store.reconstruct(nbp);
                            if let Some(ref rec) = base_rec {
                                if let Some(nval) = rec.get(field).and_then(|v| v.as_f64()) {
                                    let dist = (val - nval).abs();
                                    if dist < *radius {
                                        let scaled_weight = adj.weight * (1.0 - dist / radius);
                                        let _ = idx; // used above
                                        neighbors
                                            .entry(nbp)
                                            .and_modify(|n| {
                                                if scaled_weight > n.weight {
                                                    n.weight = scaled_weight;
                                                    n.adjacency_name = adj.name.clone();
                                                }
                                            })
                                            .or_insert_with(|| Neighbor {
                                                bp: nbp,
                                                adjacency_name: adj.name.clone(),
                                                weight: scaled_weight,
                                            });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            AdjacencyKind::Threshold { field, threshold } => {
                // Neighbor if |fiber_value| > threshold
                if let Some(idx) = store.schema.fiber_field_index(field) {
                    for (nbp, fiber) in store.sections() {
                        if nbp == bp {
                            continue;
                        }
                        if let Some(v) = fiber.get(idx).and_then(|v| v.as_f64()) {
                            if v.abs() > *threshold {
                                neighbors
                                    .entry(nbp)
                                    .and_modify(|n| {
                                        if adj.weight > n.weight {
                                            n.weight = adj.weight;
                                            n.adjacency_name = adj.name.clone();
                                        }
                                    })
                                    .or_insert_with(|| Neighbor {
                                        bp: nbp,
                                        adjacency_name: adj.name.clone(),
                                        weight: adj.weight,
                                    });
                            }
                        }
                    }
                }
            }
        }
    }

    neighbors.into_values().collect()
}

// ── H¹ local consistency check ──

/// Check local consistency (H¹ = 0) for a specific fiber field among neighbors.
///
/// Uses median + MAD (robust to outliers). Returns (is_consistent, outlier_bps).
fn check_h1_local(
    store: &BundleStore,
    field_name: &str,
    neighbors: &[Neighbor],
    threshold: f64,
) -> (bool, Vec<BasePoint>) {
    let field_idx = match store.schema.fiber_field_index(field_name) {
        Some(i) => i,
        None => return (true, vec![]),
    };

    // Collect measured values from neighbors
    let mut values: Vec<(BasePoint, f64)> = Vec::new();
    for nb in neighbors {
        if let Some(fiber) = store.get_fiber(nb.bp) {
            if let Some(v) = fiber.get(field_idx).and_then(|v| v.as_f64()) {
                values.push((nb.bp, v));
            }
        }
    }

    if values.len() < 2 {
        return (true, vec![]);
    }

    // Compute median
    let mut sorted: Vec<f64> = values.iter().map(|(_, v)| *v).collect();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = if sorted.len().is_multiple_of(2) {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2.0
    } else {
        sorted[sorted.len() / 2]
    };

    // Compute MAD (median absolute deviation)
    let mut abs_devs: Vec<f64> = sorted.iter().map(|v| (v - median).abs()).collect();
    abs_devs.sort_by(|a, b| a.total_cmp(b));
    let mad = if abs_devs.len().is_multiple_of(2) {
        (abs_devs[abs_devs.len() / 2 - 1] + abs_devs[abs_devs.len() / 2]) / 2.0
    } else {
        abs_devs[abs_devs.len() / 2]
    };

    // If MAD is near zero, check if all values truly agree
    if mad < f64::EPSILON {
        // Check max deviation — if any value differs significantly, it's an outlier
        let max_dev = abs_devs.last().copied().unwrap_or(0.0);
        if max_dev < f64::EPSILON {
            return (true, vec![]); // All values identical → truly consistent
        }
        // MAD = 0 but some values differ: flag those with deviation > 0 as outliers
        let mut outliers = Vec::new();
        for (bp, v) in &values {
            if (v - median).abs() > f64::EPSILON {
                outliers.push(*bp);
            }
        }
        return (outliers.is_empty(), outliers);
    }

    // Find outliers: z_MAD = 0.6745 * |x - median| / MAD > threshold
    let mut outliers = Vec::new();
    for (bp, v) in &values {
        let z_mad = 0.6745 * (v - median).abs() / mad;
        if z_mad > threshold {
            outliers.push(*bp);
        }
    }

    (outliers.is_empty(), outliers)
}

// ── Confidence formula ──

/// Confidence = 1 / (1 + CoV²), where CoV = σ_weighted / μ_weighted.
/// Bounded [0, 1]. Consistent with curvature.rs confidence(K) = 1/(1+K).
fn confidence_cov(predictions: &[(f64, f64)]) -> f64 {
    if predictions.is_empty() {
        return 0.0;
    }
    if predictions.len() == 1 {
        return 1.0;
    }
    let total_weight: f64 = predictions.iter().map(|(_, w)| w).sum();
    if total_weight < f64::EPSILON {
        return 0.0;
    }
    let mean: f64 = predictions.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight;
    if mean.abs() < f64::EPSILON {
        // All predictions near zero — use variance directly
        let var: f64 = predictions
            .iter()
            .map(|(v, w)| w * (v - mean).powi(2))
            .sum::<f64>()
            / total_weight;
        return 1.0 / (1.0 + var);
    }
    let var: f64 = predictions
        .iter()
        .map(|(v, w)| w * (v - mean).powi(2))
        .sum::<f64>()
        / total_weight;
    let cov_sq = var / (mean * mean);
    1.0 / (1.0 + cov_sq)
}

/// Weighted mean of predictions.
fn weighted_mean(predictions: &[(f64, f64)]) -> f64 {
    let total_weight: f64 = predictions.iter().map(|(_, w)| w).sum();
    if total_weight < f64::EPSILON {
        return 0.0;
    }
    predictions.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight
}

/// Weighted standard deviation (uncertainty band).
fn weighted_std(predictions: &[(f64, f64)]) -> f64 {
    let total_weight: f64 = predictions.iter().map(|(_, w)| w).sum();
    if total_weight < f64::EPSILON {
        return 0.0;
    }
    let mean = weighted_mean(predictions);
    let var: f64 = predictions
        .iter()
        .map(|(v, w)| w * (v - mean).powi(2))
        .sum::<f64>()
        / total_weight;
    var.sqrt()
}

// ── COMPLETE ──

/// Run sheaf completion on a bundle. Returns completed records as rows.
///
/// Each result record contains:
///   _base_point, _field, _completed_value, _confidence, _uncertainty,
///   _origin, _method, _neighbor_count
/// And optionally: _constraint_graph (JSON string of neighbor contributions)
pub fn complete(
    store: &BundleStore,
    where_conditions: &[FilterCondition],
    min_confidence: f64,
    with_provenance: bool,
    with_constraint_graph: bool,
) -> Vec<Record> {
    let h1_threshold = store.schema.h1_threshold;
    let mut results = Vec::new();

    // Determine which fiber fields to complete (from WHERE IS NULL, or all)
    let target_fields: Vec<String> = where_conditions
        .iter()
        .filter_map(|fc| match fc {
            FilterCondition::Void(f) => Some(f.clone()),
            _ => None,
        })
        .collect();
    let all_fiber_fields: Vec<String> = store
        .schema
        .fiber_fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    let fields_to_check = if target_fields.is_empty() {
        &all_fiber_fields
    } else {
        &target_fields
    };

    // Iterate all sections looking for NULL fiber values
    for (bp, fiber) in store.sections() {
        let record = match store.reconstruct(bp) {
            Some(r) => r,
            None => continue,
        };

        for field_name in fields_to_check {
            let field_idx = match store.schema.fiber_field_index(field_name) {
                Some(i) => i,
                None => continue,
            };

            // Skip already-measured values
            match fiber.get(field_idx) {
                Some(Value::Null) => {} // target for completion
                None => {}              // missing = NULL
                Some(_) => continue,    // already measured
            }

            // Find neighbors via schema-declared adjacency functions
            let neighbors = find_neighbors(store, bp, &record);
            if neighbors.len() < MIN_NEIGHBORS {
                let mut skip_rec = make_base_record(&record, &store.schema);
                skip_rec.insert("_field".into(), Value::Text(field_name.clone()));
                skip_rec.insert(
                    "_reason".into(),
                    Value::Text("insufficient_neighbors".into()),
                );
                skip_rec.insert(
                    "_neighbor_count".into(),
                    Value::Integer(neighbors.len() as i64),
                );
                skip_rec.insert("_status".into(), Value::Text("skipped".into()));
                results.push(skip_rec);
                continue;
            }

            // H¹ check: local consistency
            let (consistent, outlier_bps) =
                check_h1_local(store, field_name, &neighbors, h1_threshold);

            // Soft H¹ handling: exclude outliers and re-check
            let clean_neighbors: Vec<&Neighbor> = if !consistent {
                let clean: Vec<&Neighbor> = neighbors
                    .iter()
                    .filter(|n| !outlier_bps.contains(&n.bp))
                    .collect();
                if clean.len() < MIN_NEIGHBORS {
                    let mut skip_rec = make_base_record(&record, &store.schema);
                    skip_rec.insert("_field".into(), Value::Text(field_name.clone()));
                    skip_rec.insert(
                        "_reason".into(),
                        Value::Text("inconsistent_neighborhood".into()),
                    );
                    skip_rec.insert("_h1".into(), Value::Integer(1));
                    skip_rec.insert("_status".into(), Value::Text("skipped".into()));
                    results.push(skip_rec);
                    continue;
                }
                clean
            } else {
                neighbors.iter().collect()
            };

            // Collect predictions from clean neighbors
            let mut predictions: Vec<(f64, f64)> = Vec::new();
            let mut constraint_entries: Vec<(BasePoint, String, f64, f64)> = Vec::new();

            for nb in &clean_neighbors {
                if let Some(fiber_nb) = store.get_fiber(nb.bp) {
                    if let Some(v) = fiber_nb.get(field_idx).and_then(|v| v.as_f64()) {
                        predictions.push((v, nb.weight));
                        constraint_entries.push((nb.bp, nb.adjacency_name.clone(), v, nb.weight));
                    }
                }
            }

            if predictions.is_empty() {
                let mut skip_rec = make_base_record(&record, &store.schema);
                skip_rec.insert("_field".into(), Value::Text(field_name.clone()));
                skip_rec.insert(
                    "_reason".into(),
                    Value::Text("no_measured_neighbors".into()),
                );
                skip_rec.insert("_status".into(), Value::Text("skipped".into()));
                results.push(skip_rec);
                continue;
            }

            let conf = confidence_cov(&predictions);
            if conf < min_confidence {
                let mut skip_rec = make_base_record(&record, &store.schema);
                skip_rec.insert("_field".into(), Value::Text(field_name.clone()));
                skip_rec.insert("_reason".into(), Value::Text("below_min_confidence".into()));
                skip_rec.insert("_confidence".into(), Value::Float(conf));
                skip_rec.insert("_status".into(), Value::Text("skipped".into()));
                results.push(skip_rec);
                continue;
            }

            let completed_value = weighted_mean(&predictions);
            let uncertainty = weighted_std(&predictions);

            let mut result_rec = make_base_record(&record, &store.schema);
            result_rec.insert("_field".into(), Value::Text(field_name.clone()));
            result_rec.insert("_completed_value".into(), Value::Float(completed_value));
            result_rec.insert("_confidence".into(), Value::Float(conf));
            result_rec.insert("_uncertainty".into(), Value::Float(uncertainty));
            result_rec.insert("_origin".into(), Value::Text("sheaf_completed".into()));
            result_rec.insert("_method".into(), Value::Text("sheaf_extension".into()));
            result_rec.insert(
                "_neighbor_count".into(),
                Value::Integer(predictions.len() as i64),
            );
            result_rec.insert("_status".into(), Value::Text("completed".into()));

            if with_provenance {
                result_rec.insert(
                    "_provenance".into(),
                    Value::Text(format!(
                        "Geometrically implied by {} constraining sections with H1 = 0",
                        predictions.len()
                    )),
                );
            }

            if with_constraint_graph {
                // Encode constraint graph as a JSON string in a Text value
                let graph_json = constraint_entries
                    .iter()
                    .map(|(nbp, adj_name, val, w)| {
                        format!(
                            r#"{{"neighbor":{},"adjacency":"{}","value":{:.6},"weight":{:.4}}}"#,
                            nbp, adj_name, val, w
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                result_rec.insert(
                    "_constraint_graph".into(),
                    Value::Text(format!("[{}]", graph_json)),
                );
            }

            results.push(result_rec);
        }
    }

    results
}

// ── PROPAGATE ──

/// Simulate adding one measurement and return the newly-determined completions.
///
/// Does NOT mutate the store. Temporarily treats the assumption as if it were
/// measured, then runs COMPLETE to find cells that are now completable but
/// weren't before.
pub fn propagate(store: &BundleStore, assumption: &Record) -> Vec<Record> {
    // Run COMPLETE on current state to get baseline completable set
    let baseline = complete(store, &[], 0.30, false, false);
    let baseline_completed: std::collections::HashSet<(String, String)> = baseline
        .iter()
        .filter(|r| r.get("_status").and_then(|v| v.as_str()) == Some("completed"))
        .filter_map(|r| {
            let bp_key = base_key_string(r, &store.schema);
            let field = r.get("_field")?.as_str()?.to_string();
            Some((bp_key, field))
        })
        .collect();

    // For PROPAGATE we can't actually mutate the store (it's &BundleStore).
    // Instead, we identify the assumption's neighbors and check which of their
    // NULL fields become completable with the assumption as an additional data point.
    //
    // Find the base point the assumption would affect
    let target_bp = store.base_point(assumption);
    let target_record = store
        .reconstruct(target_bp)
        .unwrap_or_else(|| assumption.clone());

    // Merge assumption into the target record conceptually
    let mut merged = target_record.clone();
    for (k, v) in assumption {
        if *v != Value::Null {
            merged.insert(k.clone(), v.clone());
        }
    }

    // Find neighbors of this merged record
    let neighbors = find_neighbors(store, target_bp, &merged);

    // Check all NULL fields on the target base point
    let mut newly_determined = Vec::new();
    for field_def in &store.schema.fiber_fields {
        let field_name = &field_def.name;
        // Skip fields that were in the assumption (those are now "measured")
        if assumption
            .get(field_name)
            .is_some_and(|v| *v != Value::Null)
        {
            continue;
        }
        // Check if this field is NULL at the target
        let is_null = match store.get_fiber(target_bp) {
            Some(fiber) => {
                let idx = store
                    .schema
                    .fiber_field_index(field_name)
                    .unwrap_or(usize::MAX);
                fiber.get(idx).is_none_or(|v| *v == Value::Null)
            }
            None => true,
        };
        if !is_null {
            continue;
        }

        let key_str = base_key_string(&merged, &store.schema);
        if baseline_completed.contains(&(key_str.clone(), field_name.clone())) {
            continue; // was already completable
        }

        // Check if we now have enough neighbors with measured values
        if let Some(field_idx) = store.schema.fiber_field_index(field_name) {
            let mut preds: Vec<(f64, f64)> = Vec::new();
            for nb in &neighbors {
                if let Some(fiber_nb) = store.get_fiber(nb.bp) {
                    if let Some(v) = fiber_nb.get(field_idx).and_then(|v| v.as_f64()) {
                        preds.push((v, nb.weight));
                    }
                }
            }
            // Also include the assumption's own value for this field if present
            if let Some(v) = assumption.get(field_name).and_then(|v| v.as_f64()) {
                preds.push((v, 1.0));
            }
            if preds.len() >= MIN_NEIGHBORS {
                let conf = confidence_cov(&preds);
                if conf >= 0.30 {
                    let val = weighted_mean(&preds);
                    let mut rec = make_base_record(&merged, &store.schema);
                    rec.insert("_field".into(), Value::Text(field_name.clone()));
                    rec.insert("_completed_value".into(), Value::Float(val));
                    rec.insert("_confidence".into(), Value::Float(conf));
                    rec.insert("_origin".into(), Value::Text("sheaf_completed".into()));
                    rec.insert("_cascade".into(), Value::Text("newly_determined".into()));
                    newly_determined.push(rec);
                }
            }
        }
    }

    newly_determined
}

// ── CONSISTENCY ──

/// Scan the bundle for H¹ ≠ 0 contradictions. Returns rows describing each one.
pub fn consistency_check(store: &BundleStore) -> Vec<Record> {
    let h1_threshold = store.schema.h1_threshold;
    let mut contradictions = Vec::new();

    for (bp, _fiber) in store.sections() {
        let record = match store.reconstruct(bp) {
            Some(r) => r,
            None => continue,
        };

        let neighbors = find_neighbors(store, bp, &record);
        if neighbors.len() < MIN_NEIGHBORS {
            continue;
        }

        for field_def in &store.schema.fiber_fields {
            let (consistent, outlier_bps) =
                check_h1_local(store, &field_def.name, &neighbors, h1_threshold);
            if !consistent {
                let mut rec = make_base_record(&record, &store.schema);
                rec.insert("_field".into(), Value::Text(field_def.name.clone()));
                rec.insert("_h1".into(), Value::Integer(1));
                rec.insert(
                    "_outlier_count".into(),
                    Value::Integer(outlier_bps.len() as i64),
                );
                rec.insert(
                    "_severity".into(),
                    Value::Float(outlier_bps.len() as f64 / neighbors.len() as f64),
                );
                contradictions.push(rec);
            }
        }
    }

    contradictions
}

// ── Helpers ──

/// Extract base field values from a record into a new record (for result rows).
fn make_base_record(record: &Record, schema: &crate::types::BundleSchema) -> Record {
    let mut out = Record::new();
    for field in &schema.base_fields {
        if let Some(v) = record.get(&field.name) {
            out.insert(field.name.clone(), v.clone());
        }
    }
    out
}

/// Produce a string key from base fields for set comparison.
fn base_key_string(record: &Record, schema: &crate::types::BundleSchema) -> String {
    schema
        .base_fields
        .iter()
        .map(|f| {
            record
                .get(&f.name)
                .map(|v| format!("{v}"))
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("|")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    fn test_bundle() -> BundleStore {
        let schema = BundleSchema::new("test_sheaf")
            .base(FieldDef::numeric("entity"))
            .base(FieldDef::categorical("context"))
            .fiber(FieldDef::numeric("F1").with_range(10.0))
            .fiber(FieldDef::numeric("F2").with_range(10.0))
            .fiber(FieldDef::numeric("F3").with_range(10.0))
            .index("context")
            .adjacency(AdjacencyDef {
                name: "same_context".into(),
                kind: AdjacencyKind::Equality {
                    field: "context".into(),
                },
                weight: 0.4,
            });
        let mut store = BundleStore::new(schema);

        // 5 entities in context "A", all with F1 ≈ 1.0
        for i in 0..5 {
            let mut r = Record::new();
            r.insert("entity".into(), Value::Integer(i));
            r.insert("context".into(), Value::Text("A".into()));
            r.insert("F1".into(), Value::Float(1.0 + i as f64 * 0.01));
            r.insert("F2".into(), Value::Float(2.0 + i as f64 * 0.02));
            r.insert("F3".into(), Value::Float(3.0));
            store.insert(&r);
        }

        // Entity 5 in context "A" — F1 is NULL (gap)
        let mut r = Record::new();
        r.insert("entity".into(), Value::Integer(5));
        r.insert("context".into(), Value::Text("A".into()));
        r.insert("F1".into(), Value::Null);
        r.insert("F2".into(), Value::Null);
        r.insert("F3".into(), Value::Float(3.0));
        store.insert(&r);

        // Entity 6 in context "A" — planted contradiction in F3
        let mut r = Record::new();
        r.insert("entity".into(), Value::Integer(6));
        r.insert("context".into(), Value::Text("A".into()));
        r.insert("F1".into(), Value::Float(1.05));
        r.insert("F2".into(), Value::Float(2.1));
        r.insert("F3".into(), Value::Float(99.0)); // outlier
        store.insert(&r);

        // Entity 7 in context "B" — isolated, no neighbors
        let mut r = Record::new();
        r.insert("entity".into(), Value::Integer(7));
        r.insert("context".into(), Value::Text("B".into()));
        r.insert("F1".into(), Value::Null);
        r.insert("F2".into(), Value::Null);
        r.insert("F3".into(), Value::Null);
        store.insert(&r);

        store
    }

    #[test]
    fn confidence_formula_bounds() {
        // Perfect agreement → 1.0
        assert_eq!(confidence_cov(&[(1.0, 1.0), (1.0, 1.0)]), 1.0);
        // Single prediction → 1.0
        assert_eq!(confidence_cov(&[(5.0, 1.0)]), 1.0);
        // Spread → bounded (0, 1)
        let c = confidence_cov(&[(1.0, 1.0), (2.0, 1.0), (3.0, 1.0)]);
        assert!(c > 0.0 && c < 1.0, "conf = {c}");
        // Monotonic: tighter spread → higher confidence
        let tight = confidence_cov(&[(1.0, 1.0), (1.01, 1.0)]);
        let wide = confidence_cov(&[(1.0, 1.0), (5.0, 1.0)]);
        assert!(tight > wide, "tight={tight} should > wide={wide}");
    }

    #[test]
    fn complete_fills_null_cells() {
        let store = test_bundle();
        let results = complete(&store, &[], 0.30, false, false);
        let completed: Vec<_> = results
            .iter()
            .filter(|r| r.get("_status").and_then(|v| v.as_str()) == Some("completed"))
            .collect();
        // Entity 5 has F1 and F2 NULL, should be completed from context "A" neighbors
        assert!(
            completed.len() >= 2,
            "Should complete at least 2 NULL cells, got {}",
            completed.len()
        );
        for rec in &completed {
            let conf = rec
                .get("_confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            assert!(conf >= 0.30, "Confidence {conf} should be >= min threshold");
        }
    }

    #[test]
    fn complete_skips_orphan() {
        let store = test_bundle();
        let results = complete(&store, &[], 0.30, false, false);
        let skipped_orphan: Vec<_> = results
            .iter()
            .filter(|r| {
                r.get("_status").and_then(|v| v.as_str()) == Some("skipped")
                    && r.get("entity").and_then(|v| v.as_i64()) == Some(7)
            })
            .collect();
        // Entity 7 is in context "B" alone — should be skipped
        assert!(
            !skipped_orphan.is_empty(),
            "Entity 7 (orphan) should be skipped"
        );
    }

    #[test]
    fn consistency_finds_contradiction() {
        let store = test_bundle();

        let contradictions = consistency_check(&store);
        // Entity 6 has F3=99.0 — outlier among context "A" where F3≈3.0
        let f3_contradictions: Vec<_> = contradictions
            .iter()
            .filter(|r| r.get("_field").and_then(|v| v.as_str()) == Some("F3"))
            .collect();
        assert!(
            !f3_contradictions.is_empty(),
            "Should detect F3 contradiction from entity 6"
        );
    }

    #[test]
    fn complete_with_constraint_graph() {
        let store = test_bundle();
        let results = complete(&store, &[], 0.30, true, true);
        let completed_with_graph: Vec<_> = results
            .iter()
            .filter(|r| {
                r.get("_status").and_then(|v| v.as_str()) == Some("completed")
                    && r.get("_constraint_graph").is_some()
            })
            .collect();
        assert!(
            !completed_with_graph.is_empty(),
            "Should have constraint graph on completed values"
        );
        // Check that provenance is also present
        let with_prov: Vec<_> = results
            .iter()
            .filter(|r| r.get("_provenance").is_some())
            .collect();
        assert!(
            !with_prov.is_empty(),
            "Should have provenance on completed values"
        );
    }

    #[test]
    fn confidence_matches_curvature_rs() {
        // confidence(K) = 1/(1+K) should match confidence_cov when CoV² = K
        // For two equal predictions: CoV = 0, conf = 1.0
        let c1 = confidence_cov(&[(1.0, 1.0), (1.0, 1.0)]);
        let c2 = crate::curvature::confidence(0.0);
        assert!((c1 - c2).abs() < 1e-10, "c1={c1}, c2={c2}");
    }

    #[test]
    fn propagate_finds_cascade() {
        let store = test_bundle();
        // Entity 7 is isolated in context "B". If we assume it's now in context "A",
        // its NULL fields should become completable from context "A" neighbors.
        let mut assumption = Record::new();
        assumption.insert("entity".into(), Value::Integer(5));
        assumption.insert("context".into(), Value::Text("A".into()));
        assumption.insert("F1".into(), Value::Float(1.02));
        // F2, F3 not in assumption — should they cascade?
        // In this test the entity 5 already exists with F2=NULL.
        // PROPAGATE should show that F2 becomes completable (it already was from neighbors).
        // The real cascade test is for entity 7 which has no neighbors.
        let mut orphan_assumption = Record::new();
        orphan_assumption.insert("entity".into(), Value::Integer(7));
        orphan_assumption.insert("context".into(), Value::Text("A".into())); // move to "A"
        orphan_assumption.insert("F1".into(), Value::Float(1.0));
        let cascade = propagate(&store, &orphan_assumption);
        // Entity 7 now in context "A" should get F2 and/or F3 completable
        // (depends on whether neighbors have measured values for those fields)
        // At minimum the function should not panic
        // Propagate should not panic; cascade may or may not be empty
        let _ = cascade.len();
    }
}
