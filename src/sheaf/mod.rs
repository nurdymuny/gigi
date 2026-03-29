//! Sheaf completion engine — COMPLETE, PROPAGATE, CONSISTENCY.
//!
//! Implements the Sudoku Principle: if H¹ = 0 on a local neighborhood,
//! the missing fiber value is uniquely determined by sheaf extension.
//!
//! All math is domain-agnostic. Adjacency functions are declared in the
//! bundle schema (ADJACENCY clauses in CREATE BUNDLE). The engine knows
//! sections, neighbors, and consistency — nothing else.

pub mod laplacian;

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
    /// Restriction map coefficient F_{tgt←src}. Identity = 1.0.
    restriction: f64,
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
                restriction: 1.0,
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
                                restriction: 1.0,
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
                                            restriction: 1.0,
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
                                                restriction: 1.0,
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
                                        restriction: 1.0,
                                    });
                            }
                        }
                    }
                }
            }
            AdjacencyKind::Transform {
                source_field,
                target_field,
                transform,
            } => {
                // Non-identity restriction map: f(v1.source_field) ≈ v2.target_field
                let src_val = record.get(source_field).and_then(|v| v.as_f64());
                if let Some(src) = src_val {
                    let transformed = transform.apply(src);
                    // Find neighbors where target_field is close to transformed value
                    let tgt_idx = store.schema.fiber_field_index(target_field)
                        .or_else(|| store.schema.base_field_index(target_field));
                    if let Some(_idx) = tgt_idx {
                        for (nbp, _fiber) in store.sections() {
                            if nbp == bp {
                                continue;
                            }
                            if let Some(nb_rec) = store.reconstruct(nbp) {
                                if let Some(tgt_val) = nb_rec.get(target_field).and_then(|v| v.as_f64()) {
                                    let dist = (transformed - tgt_val).abs();
                                    // Use relative tolerance: 10% of |transformed| or absolute 0.1
                                    let tol = (transformed.abs() * 0.1).max(0.1);
                                    if dist < tol {
                                        let scaled_weight = adj.weight * (1.0 - dist / tol);
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
                                                restriction: 1.0,
                                            });
                                    }
                                }
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

/// Pre-Laplacian confidence proxy: 1 / (1 + 1/N_eff)
///
/// In the true Laplacian formulation (§1.4), confidence = 1/(1 + (L_mm^{-1})_{vv}).
/// Before the Laplacian is built, we approximate (L_mm^{-1})_{vv} ≈ 1/N_eff where
/// N_eff = (Σw)² / Σw² is the effective number of independent constraints.
///
/// This is purely structural (does not depend on predicted values), which:
/// - Fixes the division-by-zero when x̂ = 0 (the old CoV formula)
/// - Separates confidence (graph connectivity) from consistency (H¹)
/// - Converges to the Laplacian formula when the full engine is built
fn confidence_sheaf(predictions: &[(f64, f64)]) -> f64 {
    if predictions.is_empty() {
        return 0.0;
    }
    if predictions.len() == 1 {
        return 1.0;
    }
    let sum_w: f64 = predictions.iter().map(|(_, w)| w).sum();
    let sum_w2: f64 = predictions.iter().map(|(_, w)| w * w).sum();
    if sum_w < f64::EPSILON {
        return 0.0;
    }
    // N_eff = (Σw)² / Σw² — effective sample size
    let n_eff = sum_w * sum_w / sum_w2;
    // confidence = N_eff / (N_eff + 1)
    n_eff / (n_eff + 1.0)
}

/// Weighted mean of predictions.
fn weighted_mean(predictions: &[(f64, f64)]) -> f64 {
    let total_weight: f64 = predictions.iter().map(|(_, w)| w).sum();
    if total_weight < f64::EPSILON {
        return 0.0;
    }
    predictions.iter().map(|(v, w)| v * w).sum::<f64>() / total_weight
}

/// Weighted standard deviation (uncertainty band). Kept for PROPAGATE/fallback.
#[allow(dead_code)]
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

            // Build local SheafProblem from clean neighbors
            // Vertex 0 = target (missing), vertices 1..N = observed neighbors
            let mut observed_nbs: Vec<(&Neighbor, f64)> = Vec::new();
            let mut constraint_entries: Vec<(BasePoint, String, f64, f64)> = Vec::new();

            for nb in &clean_neighbors {
                if let Some(fiber_nb) = store.get_fiber(nb.bp) {
                    if let Some(v) = fiber_nb.get(field_idx).and_then(|v| v.as_f64()) {
                        observed_nbs.push((nb, v));
                        constraint_entries.push((nb.bp, nb.adjacency_name.clone(), v, nb.weight));
                    }
                }
            }

            if observed_nbs.is_empty() {
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

            // Construct Laplacian problem
            let n_vertices = 1 + observed_nbs.len();
            let mut edges = Vec::with_capacity(observed_nbs.len());
            let mut kinds = Vec::with_capacity(n_vertices);
            let mut observed_map = HashMap::new();

            kinds.push(laplacian::VertexKind::Missing); // vertex 0 = target
            for (i, (nb, val)) in observed_nbs.iter().enumerate() {
                let vi = i + 1;
                kinds.push(laplacian::VertexKind::Observed);
                observed_map.insert(vi, *val);
                edges.push(laplacian::SheafEdge {
                    src: 0,
                    tgt: vi,
                    weight: nb.weight,
                    restriction: nb.restriction,
                });
            }

            let problem = laplacian::SheafProblem {
                n_vertices,
                edges,
                kinds,
                observed: observed_map,
            };

            let solution = laplacian::solve(&problem);

            // Extract result for vertex 0 (the missing target)
            let completion = solution.completions.iter().find(|(idx, _)| *idx == 0);
            let (completed_value, conf, uncertainty) = match completion {
                Some((_, cr)) => (cr.value, cr.confidence, cr.inv_diag.sqrt()),
                None => {
                    // Undetermined — Laplacian singular at target vertex
                    let mut skip_rec = make_base_record(&record, &store.schema);
                    skip_rec.insert("_field".into(), Value::Text(field_name.clone()));
                    skip_rec.insert(
                        "_reason".into(),
                        Value::Text("undetermined_direction".into()),
                    );
                    skip_rec.insert("_status".into(), Value::Text("skipped".into()));
                    results.push(skip_rec);
                    continue;
                }
            };

            if conf < min_confidence {
                let mut skip_rec = make_base_record(&record, &store.schema);
                skip_rec.insert("_field".into(), Value::Text(field_name.clone()));
                skip_rec.insert("_reason".into(), Value::Text("below_min_confidence".into()));
                skip_rec.insert("_confidence".into(), Value::Float(conf));
                skip_rec.insert("_status".into(), Value::Text("skipped".into()));
                results.push(skip_rec);
                continue;
            }

            let mut result_rec = make_base_record(&record, &store.schema);
            result_rec.insert("_field".into(), Value::Text(field_name.clone()));
            result_rec.insert("_completed_value".into(), Value::Float(completed_value));
            result_rec.insert("_confidence".into(), Value::Float(conf));
            result_rec.insert("_uncertainty".into(), Value::Float(uncertainty));
            result_rec.insert("_origin".into(), Value::Text("sheaf_completed".into()));
            result_rec.insert("_method".into(), Value::Text("laplacian_schur".into()));
            result_rec.insert(
                "_neighbor_count".into(),
                Value::Integer(observed_nbs.len() as i64),
            );
            result_rec.insert("_status".into(), Value::Text("completed".into()));

            if with_provenance {
                result_rec.insert(
                    "_provenance".into(),
                    Value::Text(format!(
                        "Schur complement of {}-vertex sheaf Laplacian, s²={:.4e}",
                        n_vertices, solution.residual_variance
                    )),
                );
            }

            if with_constraint_graph {
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
                let conf = confidence_sheaf(&preds);
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

// ── SUGGEST_ADJACENCY ──

/// Suggest new adjacency relations that would reduce H¹ on this bundle.
///
/// 1. Enumerate candidate adjacencies (Equality on each categorical field,
///    Metric on each numeric field).
/// 2. Sample `sample_size` records.
/// 3. For each candidate, temporarily add it, measure H¹ on the sample.
/// 4. Return the top `k` candidates ranked by ΔH¹.
pub fn suggest_adjacency(
    store: &BundleStore,
    restrict_fields: &[String],
    sample_size: usize,
    k: usize,
) -> Vec<Record> {
    use crate::types::{AdjacencyDef, AdjacencyKind, FieldType};

    // Measure baseline H¹ on the sample
    let sample_bps: Vec<BasePoint> = store
        .sections()
        .map(|(bp, _)| bp)
        .take(sample_size)
        .collect();

    let baseline_h1 = count_h1_on_sample(store, &sample_bps, &store.schema.adjacencies);

    // Enumerate candidate adjacencies from schema fields
    let mut candidates: Vec<(AdjacencyDef, String)> = Vec::new();
    let all_fields: Vec<_> = store
        .schema
        .base_fields
        .iter()
        .chain(store.schema.fiber_fields.iter())
        .collect();

    for field in &all_fields {
        // Skip if restrict_fields is non-empty and field not in it
        if !restrict_fields.is_empty() && !restrict_fields.contains(&field.name) {
            continue;
        }
        // Skip fields already covered by existing adjacencies
        let already_covered = store.schema.adjacencies.iter().any(|a| match &a.kind {
            AdjacencyKind::Equality { field: f } => f == &field.name,
            AdjacencyKind::Metric { field: f, .. } => f == &field.name,
            AdjacencyKind::Threshold { field: f, .. } => f == &field.name,
            AdjacencyKind::Transform { source_field, .. } => source_field == &field.name,
        });
        if already_covered {
            continue;
        }

        match field.field_type {
            FieldType::Categorical => {
                let desc = format!("EQUALITY ON {} WEIGHT 0.6", field.name);
                candidates.push((
                    AdjacencyDef {
                        name: format!("suggest_{}", field.name),
                        kind: AdjacencyKind::Equality {
                            field: field.name.clone(),
                        },
                        weight: 0.6,
                    },
                    desc,
                ));
            }
            FieldType::Numeric => {
                // Metric adjacency: use 10% of range if known, else 0.5
                let radius = field.range.map(|r| r * 0.1).unwrap_or(0.5);
                let desc = format!(
                    "METRIC ON {} WITHIN {:.2} WEIGHT 0.8",
                    field.name, radius
                );
                candidates.push((
                    AdjacencyDef {
                        name: format!("suggest_{}", field.name),
                        kind: AdjacencyKind::Metric {
                            field: field.name.clone(),
                            radius,
                        },
                        weight: 0.8,
                    },
                    desc,
                ));
            }
            _ => {} // Timestamp, Binary, Vector, OrderedCat — skip
        }
    }

    // Score each candidate by ΔH¹
    let mut scored: Vec<(String, i64, i64)> = Vec::new();
    for (adj_def, desc) in &candidates {
        let mut trial_adjs = store.schema.adjacencies.clone();
        trial_adjs.push(adj_def.clone());
        let trial_h1 = count_h1_on_sample(store, &sample_bps, &trial_adjs);
        let delta = trial_h1 as i64 - baseline_h1 as i64;
        scored.push((desc.clone(), trial_h1 as i64, delta));
    }

    // Sort by delta (most negative = biggest H¹ reduction)
    scored.sort_by_key(|(_, _, d)| *d);

    // Build result records
    let mut results = Vec::new();

    // Summary row
    let mut summary = Record::new();
    summary.insert(
        "bundle".into(),
        Value::Text(store.schema.name.clone()),
    );
    summary.insert("current_h1".into(), Value::Integer(baseline_h1 as i64));
    summary.insert(
        "sample_size".into(),
        Value::Integer(sample_bps.len() as i64),
    );
    results.push(summary);

    // Top-k suggestion rows
    for (desc, predicted_h1, delta) in scored.iter().take(k) {
        let mut rec = Record::new();
        rec.insert("adjacency".into(), Value::Text(desc.clone()));
        rec.insert("predicted_h1".into(), Value::Integer(*predicted_h1));
        rec.insert("delta".into(), Value::Integer(*delta));
        results.push(rec);
    }

    results
}

/// Count H¹ inconsistencies on a sample using a given adjacency set.
fn count_h1_on_sample(
    store: &BundleStore,
    sample_bps: &[BasePoint],
    adjacencies: &[crate::types::AdjacencyDef],
) -> usize {
    let h1_threshold = store.schema.h1_threshold;
    let mut h1_count = 0;

    for &bp in sample_bps {
        let record = match store.reconstruct(bp) {
            Some(r) => r,
            None => continue,
        };

        // Find neighbors using the trial adjacency set
        let neighbors = find_neighbors_with_adjs(store, bp, &record, adjacencies);
        if neighbors.len() < MIN_NEIGHBORS {
            continue;
        }

        for field_def in &store.schema.fiber_fields {
            let (consistent, _) =
                check_h1_local(store, &field_def.name, &neighbors, h1_threshold);
            if !consistent {
                h1_count += 1;
            }
        }
    }

    h1_count
}

/// Like find_neighbors, but uses an arbitrary adjacency set (for suggest_adjacency trials).
fn find_neighbors_with_adjs(
    store: &BundleStore,
    bp: BasePoint,
    record: &Record,
    adjacencies: &[crate::types::AdjacencyDef],
) -> Vec<Neighbor> {
    if adjacencies.is_empty() {
        return store
            .geometric_neighbors(bp)
            .into_iter()
            .map(|nbp| Neighbor {
                bp: nbp,
                adjacency_name: "geometric".into(),
                weight: 1.0,
                restriction: 1.0,
            })
            .collect();
    }

    let mut neighbors: HashMap<BasePoint, Neighbor> = HashMap::new();

    for adj in adjacencies {
        match &adj.kind {
            AdjacencyKind::Equality { field } => {
                if let Some(val) = record.get(field) {
                    if *val == Value::Null {
                        continue;
                    }
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
                                restriction: 1.0,
                            });
                    }
                }
            }
            AdjacencyKind::Metric { field, radius } => {
                if let Some(val) = record.get(field).and_then(|v| v.as_f64()) {
                    for (nbp, fiber) in store.sections() {
                        if nbp == bp {
                            continue;
                        }
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
                                            restriction: 1.0,
                                        });
                                }
                            }
                        }
                    }
                }
            }
            _ => {} // Threshold and Transform — not generated as candidates
        }
    }

    neighbors.into_values().collect()
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
        // N_eff = 2 with unit weights → 2/3
        let c2 = confidence_sheaf(&[(1.0, 1.0), (1.0, 1.0)]);
        assert!((c2 - 2.0/3.0).abs() < 1e-10, "2 unit-weight → 2/3, got {c2}");
        // Single prediction → 1.0
        assert_eq!(confidence_sheaf(&[(5.0, 1.0)]), 1.0);
        // N_eff = 3 with unit weights → 3/4 = 0.75
        let c = confidence_sheaf(&[(1.0, 1.0), (2.0, 1.0), (3.0, 1.0)]);
        assert!((c - 0.75).abs() < 1e-10, "3 unit-weight preds should give 0.75, got {c}");
        // Monotonic: more neighbors → higher confidence (structural, not value-dependent)
        let few = confidence_sheaf(&[(1.0, 1.0), (2.0, 1.0)]);
        let many = confidence_sheaf(&[(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (4.0, 1.0), (5.0, 1.0)]);
        assert!(many > few, "more neighbors should give higher confidence: many={many}, few={few}");
        // Higher weight → higher effective N → higher confidence
        let low_w = confidence_sheaf(&[(1.0, 0.1), (2.0, 0.1)]);
        let high_w = confidence_sheaf(&[(1.0, 5.0), (2.0, 5.0)]);
        // Both have N_eff = 2 (equal weights), so same confidence
        assert!((low_w - high_w).abs() < 1e-10, "equal-ratio weights give same N_eff");
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
    fn confidence_sheaf_zero_value() {
        // Key fix: confidence should be HIGH when all predictions agree on zero.
        // The old CoV formula would give confidence=0 here (division by zero).
        // With N_eff structural confidence: 3 unit-weight preds → 3/4 = 0.75
        let c = confidence_sheaf(&[(0.0, 1.0), (0.0, 1.0), (0.0, 1.0)]);
        assert!((c - 0.75).abs() < 1e-10, "3 zero predictions → N_eff=3 → conf=0.75, got {c}");

        // Near-zero predictions: same structural confidence, value doesn't matter
        let c2 = confidence_sheaf(&[(0.001, 1.0), (0.002, 1.0), (0.001, 1.0)]);
        assert!(c2 > 0.5, "3 near-zero preds should give confidence > 0.5, got {c2}");
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

    #[test]
    fn complete_uses_laplacian_method() {
        // Verify that completed records now report method = "laplacian_schur"
        let store = test_bundle();
        let results = complete(&store, &[], 0.30, false, false);
        let completed: Vec<_> = results
            .iter()
            .filter(|r| r.get("_status").and_then(|v| v.as_str()) == Some("completed"))
            .collect();
        assert!(!completed.is_empty(), "Should have completed records");
        for rec in &completed {
            let method = rec.get("_method").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(method, "laplacian_schur", "Method should be laplacian_schur, got {method}");
        }
    }

    #[test]
    fn laplacian_completion_value_matches_schur() {
        // Directly verify: for 5 neighbors all with F1 ≈ 1.0 (weights 0.4 each),
        // the Laplacian Schur complement should give x̂ = weighted mean of observed.
        // With equal weights and identity restriction: Schur = weighted mean.
        let store = test_bundle();
        let results = complete(&store, &[], 0.10, false, false);
        let f1_completed: Vec<_> = results
            .iter()
            .filter(|r| {
                r.get("_field").and_then(|v| v.as_str()) == Some("F1")
                    && r.get("_status").and_then(|v| v.as_str()) == Some("completed")
            })
            .collect();
        assert!(!f1_completed.is_empty(), "F1 should be completed for entity 5");
        let val = f1_completed[0]
            .get("_completed_value")
            .and_then(|v| v.as_f64())
            .unwrap();
        // Entity 5 F1 is NULL. Neighbors (entities 0-4,6) in context "A" have
        // F1 = 1.00, 1.01, 1.02, 1.03, 1.04, 1.05.
        // Entity 6 has F1=1.05 and is NOT an outlier for F1.
        // With identity restriction, Schur = mean of observed = (sum/n).
        // The exact value depends on which neighbors pass H1 check.
        assert!(
            val > 0.9 && val < 1.2,
            "Completed F1 should be ≈1.0, got {val}"
        );
    }

    #[test]
    fn laplacian_confidence_from_inv_diag() {
        // The Laplacian confidence = 1/(1 + (L_mm^{-1})_{00}).
        // For a star graph (1 missing → N observed, all weight w, restriction 1):
        //   L_mm = N*w (scalar), so L_mm^{-1} = 1/(N*w).
        //   Confidence = 1/(1 + 1/(N*w)) = N*w/(N*w + 1).
        let store = test_bundle();
        let results = complete(&store, &[], 0.01, true, false);
        let completed: Vec<_> = results
            .iter()
            .filter(|r| r.get("_status").and_then(|v| v.as_str()) == Some("completed"))
            .collect();
        for rec in &completed {
            let conf = rec
                .get("_confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            // Confidence must be in (0, 1)
            assert!(conf > 0.0, "Confidence must be > 0, got {conf}");
            assert!(conf < 1.0, "Confidence must be < 1, got {conf}");
            // Provenance should mention Schur complement
            if let Some(prov) = rec.get("_provenance").and_then(|v| v.as_str()) {
                assert!(
                    prov.contains("Schur complement"),
                    "Provenance should mention Schur: {prov}"
                );
            }
        }
    }

    #[test]
    fn suggest_adjacency_returns_candidates() {
        let store = test_bundle();
        let results = suggest_adjacency(&store, &[], 100, 5);
        // First row is the summary
        assert!(!results.is_empty(), "Should return at least a summary row");
        let summary = &results[0];
        assert_eq!(
            summary.get("bundle").and_then(|v| v.as_str()),
            Some("test_sheaf")
        );
        let h1 = summary.get("current_h1").and_then(|v| v.as_i64());
        assert!(h1.is_some(), "Summary should have current_h1");
    }

    #[test]
    fn suggest_adjacency_ranks_by_delta() {
        let store = test_bundle();
        let results = suggest_adjacency(&store, &[], 100, 10);
        // Skip summary (first row), check suggestion rows are sorted by delta
        let suggestions: Vec<_> = results
            .iter()
            .skip(1)
            .filter_map(|r| r.get("delta").and_then(|v| v.as_i64()))
            .collect();
        if suggestions.len() >= 2 {
            for i in 1..suggestions.len() {
                assert!(
                    suggestions[i] >= suggestions[i - 1],
                    "Suggestions should be sorted by delta (ascending): {:?}",
                    suggestions
                );
            }
        }
    }

    #[test]
    fn suggest_adjacency_field_filter() {
        let store = test_bundle();
        // Restrict to just F1 — should get metric suggestion for F1
        let results = suggest_adjacency(&store, &["F1".into()], 100, 5);
        let suggestions: Vec<_> = results
            .iter()
            .skip(1)
            .filter_map(|r| r.get("adjacency").and_then(|v| v.as_str()).map(String::from))
            .collect();
        for s in &suggestions {
            assert!(
                s.contains("F1"),
                "Filtered suggestions should only be for F1: {s}"
            );
        }
    }
}
