//! Query engine — sheaf evaluation, query algebra, confidence annotation.
//!
//! Implements §8: Query Engine (Layer 2).
//! Feature #10: Cost-based query planner (Definitions 10.1–10.3, Theorem 10.1–10.2).

use std::collections::HashMap;

use crate::bundle::{BundleStore, FieldStats, QueryCondition};
use crate::types::{Record, Value};

/// Query result annotated with geometric metadata (§8.2).
#[derive(Debug)]
pub struct QueryResult {
    pub records: Vec<Record>,
    /// 1/(1+K) — from local curvature.
    pub confidence: f64,
    /// K at query region.
    pub curvature: f64,
    /// C = τ/K — Davis capacity.
    pub capacity: f64,
    /// Average deviation norm across results.
    pub deviation_norm: f64,
}

/// Recall and deviation from the double cover (Def 6.1).
///
/// S = |correct returned| / |total correct|
/// d = √(1 - S)
/// S + d² = 1 (Theorem 6.1)
pub fn recall_deviation(returned: usize, total_correct: usize) -> (f64, f64) {
    if total_correct == 0 {
        return if returned == 0 {
            (1.0, 0.0)
        } else {
            (0.0, 1.0)
        };
    }
    let s = returned.min(total_correct) as f64 / total_correct as f64;
    let d = (1.0 - s).max(0.0).sqrt();
    (s, d)
}

// ── Feature #10: Query Cost Planner ─────────────────────────────────────

/// Physical access method for query execution (Definition 10.1).
#[derive(Debug, Clone, PartialEq)]
pub enum AccessMethod {
    /// Full sequential scan — O(N).
    FullScan,
    /// Single-field index lookup — O(|I_f(v)|).
    IndexLookup { field: String, values: Vec<Value> },
    /// Multi-field index intersection — O(min_card * k).
    IndexIntersection {
        lookups: Vec<(String, Vec<Value>)>,
    },
}

/// A query execution plan (Definition 10.1).
#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub access: AccessMethod,
    pub residual_predicates: Vec<QueryCondition>,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
}

/// Cost coefficients for plan estimation (Definition 10.2).
#[derive(Debug, Clone)]
pub struct CostEstimator {
    /// Per-record scan cost (predicate check). Default: 1.0
    pub row_cost: f64,
    /// Per-record fetch from base point. Default: 2.0
    pub fetch_cost: f64,
    /// HashMap/index lookup overhead. Default: 10.0
    pub lookup_cost: f64,
    /// Per-element bitmap AND cost. Default: 0.1
    pub bitmap_cost: f64,
}

impl Default for CostEstimator {
    fn default() -> Self {
        Self {
            row_cost: 1.0,
            fetch_cost: 2.0,
            lookup_cost: 10.0,
            bitmap_cost: 0.1,
        }
    }
}

/// Statistics snapshot for query planning — extracted from BundleStore.
#[derive(Debug, Clone)]
pub struct BundleStats {
    /// Total records in bundle.
    pub record_count: usize,
    /// Per-field: number of distinct values in the index.
    pub index_distinct: HashMap<String, usize>,
    /// Per-field: bitmap cardinalities for specific values.
    /// field → value → cardinality.
    pub index_cardinalities: HashMap<String, HashMap<Value, u64>>,
    /// Per-field running stats (min, max, count).
    pub field_stats: HashMap<String, FieldStats>,
}

impl BundleStats {
    /// Extract stats from a live BundleStore.
    pub fn from_bundle(store: &BundleStore) -> Self {
        let record_count = store.len();
        let mut index_distinct = HashMap::new();
        let mut index_cardinalities = HashMap::new();

        for (field, val_map) in &store.field_index {
            index_distinct.insert(field.clone(), val_map.len());
            let mut cards = HashMap::new();
            for (val, bm) in val_map {
                cards.insert(val.clone(), bm.len());
            }
            index_cardinalities.insert(field.clone(), cards);
        }

        Self {
            record_count,
            index_distinct,
            index_cardinalities,
            field_stats: store.field_stats.clone(),
        }
    }
}

impl CostEstimator {
    /// Selectivity of a single predicate (Definition 10.3).
    /// Returns fraction of N expected to match [0.0, 1.0].
    pub fn selectivity(&self, cond: &QueryCondition, stats: &BundleStats) -> f64 {
        if stats.record_count == 0 {
            return 0.0;
        }
        let n = stats.record_count as f64;
        match cond {
            QueryCondition::Eq(field, value) => {
                // Use exact bitmap cardinality if available
                if let Some(cards) = stats.index_cardinalities.get(field.as_str()) {
                    if let Some(&card) = cards.get(value) {
                        return card as f64 / n;
                    }
                    // Indexed but value not seen → selectivity = 0
                    if !cards.is_empty() {
                        return 0.0;
                    }
                }
                // Fallback: uniform assumption 1/distinct
                if let Some(&distinct) = stats.index_distinct.get(field.as_str()) {
                    if distinct > 0 {
                        return 1.0 / distinct as f64;
                    }
                }
                // No stats → assume 10%
                0.1
            }
            QueryCondition::In(field, values) => {
                // Sum of individual Eq selectivities
                values.iter().map(|v| {
                    self.selectivity(
                        &QueryCondition::Eq(field.clone(), v.clone()),
                        stats,
                    )
                }).sum::<f64>().min(1.0)
            }
            QueryCondition::Between(field, low, high) => {
                // Range selectivity from FieldStats (Def 10.3b)
                if let Some(fs) = stats.field_stats.get(field.as_str()) {
                    let range = fs.range();
                    if range > f64::EPSILON {
                        let lo = low.as_f64().unwrap_or(fs.min);
                        let hi = high.as_f64().unwrap_or(fs.max);
                        return ((hi - lo) / range).clamp(0.0, 1.0);
                    }
                }
                0.1
            }
            QueryCondition::Lt(field, value) | QueryCondition::Lte(field, value) => {
                if let Some(fs) = stats.field_stats.get(field.as_str()) {
                    let range = fs.range();
                    if range > f64::EPSILON {
                        let v = value.as_f64().unwrap_or(fs.max);
                        return ((v - fs.min) / range).clamp(0.0, 1.0);
                    }
                }
                0.5
            }
            QueryCondition::Gt(field, value) | QueryCondition::Gte(field, value) => {
                if let Some(fs) = stats.field_stats.get(field.as_str()) {
                    let range = fs.range();
                    if range > f64::EPSILON {
                        let v = value.as_f64().unwrap_or(fs.min);
                        return ((fs.max - v) / range).clamp(0.0, 1.0);
                    }
                }
                0.5
            }
            QueryCondition::IsNull(_) => 0.05, // assume 5% null
            QueryCondition::IsNotNull(_) => 0.95,
            _ => 0.1, // Contains, StartsWith, Regex, etc.
        }
    }

    /// Conjunction selectivity: independence assumption (Def 10.3c).
    pub fn conjunction_selectivity(
        &self,
        conds: &[QueryCondition],
        stats: &BundleStats,
    ) -> f64 {
        conds
            .iter()
            .map(|c| self.selectivity(c, stats))
            .product::<f64>()
    }

    /// Estimate cost of a full scan plan (Definition 10.2a).
    pub fn full_scan_cost(&self, n: usize) -> f64 {
        n as f64 * self.row_cost
    }

    /// Estimate cost of an index lookup plan (Definition 10.2b).
    pub fn index_lookup_cost(&self, cardinality: u64) -> f64 {
        self.lookup_cost + cardinality as f64 * self.fetch_cost
    }

    /// Estimate cost of an index intersection plan (Definition 10.2c).
    pub fn index_intersection_cost(
        &self,
        cardinalities: &[u64],
        result_card: u64,
    ) -> f64 {
        let k = cardinalities.len() as f64;
        let min_card = cardinalities.iter().copied().min().unwrap_or(0) as f64;
        k * self.lookup_cost + min_card * k * self.bitmap_cost + result_card as f64 * self.fetch_cost
    }

    /// Plan a query: enumerate candidate plans and select min cost (Theorem 10.1).
    pub fn plan(
        &self,
        conditions: &[QueryCondition],
        stats: &BundleStats,
    ) -> QueryPlan {
        let n = stats.record_count;
        let mut best = self.plan_full_scan(conditions, n);

        // Try single-index lookup for each indexable condition
        for cond in conditions {
            if let Some(plan) = self.plan_index_lookup(cond, conditions, stats) {
                if plan.estimated_cost < best.estimated_cost {
                    best = plan;
                }
            }
        }

        // Try multi-index intersection if ≥2 indexable conditions
        if let Some(plan) = self.plan_index_intersection(conditions, stats) {
            if plan.estimated_cost < best.estimated_cost {
                best = plan;
            }
        }

        best
    }

    fn plan_full_scan(&self, conditions: &[QueryCondition], n: usize) -> QueryPlan {
        QueryPlan {
            access: AccessMethod::FullScan,
            residual_predicates: conditions.to_vec(),
            estimated_cost: self.full_scan_cost(n),
            estimated_rows: n,
        }
    }

    fn plan_index_lookup(
        &self,
        cond: &QueryCondition,
        all_conditions: &[QueryCondition],
        stats: &BundleStats,
    ) -> Option<QueryPlan> {
        let (field, values) = match cond {
            QueryCondition::Eq(f, v) => (f.clone(), vec![v.clone()]),
            QueryCondition::In(f, vs) => (f.clone(), vs.clone()),
            _ => return None,
        };

        // Must be an indexed field
        stats.index_cardinalities.get(&field)?;

        let cardinality: u64 = values
            .iter()
            .filter_map(|v| {
                stats
                    .index_cardinalities
                    .get(&field)
                    .and_then(|m| m.get(v).copied())
            })
            .sum();

        let cost = self.index_lookup_cost(cardinality);
        let residual: Vec<QueryCondition> = all_conditions
            .iter()
            .filter(|c| !std::ptr::eq(*c, cond))
            .cloned()
            .collect();

        Some(QueryPlan {
            access: AccessMethod::IndexLookup { field, values },
            residual_predicates: residual,
            estimated_cost: cost,
            estimated_rows: cardinality as usize,
        })
    }

    fn plan_index_intersection(
        &self,
        conditions: &[QueryCondition],
        stats: &BundleStats,
    ) -> Option<QueryPlan> {
        let mut lookups: Vec<(String, Vec<Value>, u64)> = Vec::new();
        let mut residual: Vec<QueryCondition> = Vec::new();

        for cond in conditions {
            match cond {
                QueryCondition::Eq(f, v) if stats.index_cardinalities.contains_key(f) => {
                    let card = stats
                        .index_cardinalities
                        .get(f)
                        .and_then(|m| m.get(v).copied())
                        .unwrap_or(0);
                    lookups.push((f.clone(), vec![v.clone()], card));
                }
                QueryCondition::In(f, vs) if stats.index_cardinalities.contains_key(f) => {
                    let card: u64 = vs
                        .iter()
                        .filter_map(|v| {
                            stats
                                .index_cardinalities
                                .get(f)
                                .and_then(|m| m.get(v).copied())
                        })
                        .sum();
                    lookups.push((f.clone(), vs.clone(), card));
                }
                _ => residual.push(cond.clone()),
            }
        }

        if lookups.len() < 2 {
            return None; // Need ≥2 indexes for intersection
        }

        // Sort by cardinality (smallest first — Theorem 4.1)
        lookups.sort_by_key(|(_, _, c)| *c);

        let cardinalities: Vec<u64> = lookups.iter().map(|(_, _, c)| *c).collect();
        let min_card = *cardinalities.iter().min().unwrap_or(&0);
        // Estimate result cardinality via conjunction selectivity (independence assumption):
        // Each lookup's selectivity = card_i / N. Product gives joint selectivity.
        let n = stats.record_count as f64;
        let joint_sel: f64 = if n > 0.0 {
            cardinalities.iter().map(|&c| c as f64 / n).product()
        } else {
            0.0
        };
        let result_card = ((joint_sel * n) as u64).max(1).min(min_card);
        let cost = self.index_intersection_cost(&cardinalities, result_card);

        let lookup_pairs: Vec<(String, Vec<Value>)> = lookups
            .into_iter()
            .map(|(f, vs, _)| (f, vs))
            .collect();

        Some(QueryPlan {
            access: AccessMethod::IndexIntersection {
                lookups: lookup_pairs,
            },
            residual_predicates: residual,
            estimated_cost: cost,
            estimated_rows: result_card as usize,
        })
    }
}

/// Format a query plan for EXPLAIN output.
pub fn explain(plan: &QueryPlan) -> String {
    let access = match &plan.access {
        AccessMethod::FullScan => "FullScan".to_string(),
        AccessMethod::IndexLookup { field, values } => {
            if values.len() == 1 {
                format!("IndexLookup({field} = {:?})", values[0])
            } else {
                format!("IndexLookup({field} IN {:?})", values)
            }
        }
        AccessMethod::IndexIntersection { lookups } => {
            let parts: Vec<String> = lookups
                .iter()
                .map(|(f, vs)| {
                    if vs.len() == 1 {
                        format!("{f} = {:?}", vs[0])
                    } else {
                        format!("{f} IN {:?}", vs)
                    }
                })
                .collect();
            format!("IndexIntersection({})", parts.join(" AND "))
        }
    };

    let residual = if plan.residual_predicates.is_empty() {
        "none".to_string()
    } else {
        format!("{} predicate(s)", plan.residual_predicates.len())
    };

    format!(
        "Plan: {access}\nEstimated rows: {}\nEstimated cost: {:.1}\nResidual: {residual}",
        plan.estimated_rows, plan.estimated_cost
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TDD-6.1 / TDD-6.2: Exact query → S=1, d=0.
    #[test]
    fn tdd_6_1_exact_recall() {
        let (s, d) = recall_deviation(10, 10);
        assert!((s - 1.0).abs() < 1e-15);
        assert!(d.abs() < 1e-15);
    }

    /// TDD-6.3: S + d² = 1.
    #[test]
    fn tdd_6_3_double_cover() {
        for returned in [0, 3, 7, 10] {
            let (s, d) = recall_deviation(returned, 10);
            assert!(
                (s + d * d - 1.0).abs() < 1e-14,
                "S + d² ≠ 1 for {returned}/10"
            );
        }
    }

    /// TDD-6.5: d = √(1-S).
    #[test]
    fn tdd_6_5_deviation_identity() {
        let (s, d) = recall_deviation(6, 10);
        let expected_d = (1.0 - s).sqrt();
        assert!((d - expected_d).abs() < 1e-14);
    }

    // ── Feature #10: Query Cost Planner TDD ─────────────────────────────────

    /// Helper: create BundleStats for N records with given index cardinalities.
    fn make_stats(n: usize, indexes: &[(&str, &[(&str, u64)])]) -> BundleStats {
        let mut index_distinct = HashMap::new();
        let mut index_cardinalities = HashMap::new();
        for &(field, vals) in indexes {
            index_distinct.insert(field.to_string(), vals.len());
            let mut cards = HashMap::new();
            for &(val, card) in vals {
                cards.insert(Value::Text(val.to_string()), card);
            }
            index_cardinalities.insert(field.to_string(), cards);
        }
        BundleStats {
            record_count: n,
            index_distinct,
            index_cardinalities,
            field_stats: HashMap::new(),
        }
    }

    /// TDD-10.1: Full scan cost is O(N).
    #[test]
    fn tdd_10_1_full_scan_cost() {
        let est = CostEstimator::default();
        let stats = BundleStats {
            record_count: 1_000_000,
            index_distinct: HashMap::new(),
            index_cardinalities: HashMap::new(),
            field_stats: HashMap::new(),
        };
        let conds = [QueryCondition::Contains("name".into(), "foo".into())];
        let plan = est.plan(&conds, &stats);
        assert_eq!(plan.access, AccessMethod::FullScan);
        assert!((plan.estimated_cost - 1_000_000.0).abs() < 0.01,
            "full scan cost should be N * row_cost = 1M, got {}", plan.estimated_cost);
    }

    /// TDD-10.2: Index lookup beats full scan for selective queries.
    #[test]
    fn tdd_10_2_index_beats_scan() {
        let est = CostEstimator::default();
        let stats = make_stats(1_000_000, &[
            ("organism", &[("S. aureus", 1000), ("E. coli", 999_000)]),
        ]);
        let conds = [QueryCondition::Eq("organism".into(), Value::Text("S. aureus".into()))];
        let plan = est.plan(&conds, &stats);
        match &plan.access {
            AccessMethod::IndexLookup { field, .. } => {
                assert_eq!(field, "organism");
            }
            other => panic!("expected IndexLookup, got {other:?}"),
        }
        // cost = lookup(10) + 1000 * fetch(2) = 2010
        assert!((plan.estimated_cost - 2010.0).abs() < 0.01,
            "index cost should be 2010, got {}", plan.estimated_cost);
        // Must beat full scan
        assert!(plan.estimated_cost < 1_000_000.0);
    }

    /// TDD-10.3: Index intersection for multi-field queries.
    #[test]
    fn tdd_10_3_index_intersection() {
        let est = CostEstimator::default();
        let stats = make_stats(1_000_000, &[
            ("organism", &[("S. aureus", 10_000)]),
            ("target", &[("PBP", 500)]),
        ]);
        let conds = [
            QueryCondition::Eq("organism".into(), Value::Text("S. aureus".into())),
            QueryCondition::Eq("target".into(), Value::Text("PBP".into())),
        ];
        let plan = est.plan(&conds, &stats);
        match &plan.access {
            AccessMethod::IndexIntersection { lookups } => {
                // Smallest bitmap (target=500) should be first
                assert_eq!(lookups[0].0, "target");
                assert_eq!(lookups[1].0, "organism");
            }
            other => panic!("expected IndexIntersection, got {other:?}"),
        }
        // Intersection must be cheaper than both single lookups and full scan
        let scan_cost = est.full_scan_cost(1_000_000);
        assert!(plan.estimated_cost < scan_cost);
    }

    /// TDD-10.4: Full scan when index is non-selective (selectivity > 50%).
    #[test]
    fn tdd_10_4_scan_when_non_selective() {
        let est = CostEstimator::default();
        let stats = make_stats(100, &[
            ("gender", &[("M", 50), ("F", 50)]),
        ]);
        let conds = [QueryCondition::Eq("gender".into(), Value::Text("M".into()))];
        let plan = est.plan(&conds, &stats);
        // index cost = 10 + 50*2 = 110, scan cost = 100*1 = 100
        // Scan is cheaper
        assert_eq!(plan.access, AccessMethod::FullScan);
    }

    /// TDD-10.5: Selectivity estimation — equality on indexed field.
    #[test]
    fn tdd_10_5_selectivity_equality() {
        let est = CostEstimator::default();
        // 100 distinct values, each with ~100 records out of 10K
        let vals: Vec<(&str, u64)> = (0..100).map(|i| {
            // Leak string to get &str with 'static lifetime for test
            let s: &str = Box::leak(format!("val_{i}").into_boxed_str());
            (s, 100u64)
        }).collect();
        let stats = make_stats(10_000, &[("field", &vals)]);
        let sel = est.selectivity(
            &QueryCondition::Eq("field".into(), Value::Text("val_0".into())),
            &stats,
        );
        assert!((sel - 0.01).abs() < 0.001, "sel should be ~0.01, got {sel}");
    }

    /// TDD-10.6: Selectivity estimation — range using FieldStats.
    #[test]
    fn tdd_10_6_selectivity_range() {
        let est = CostEstimator::default();
        let mut field_stats = HashMap::new();
        field_stats.insert("temp".to_string(), FieldStats {
            count: 1000,
            sum: 50000.0,
            sum_sq: 3333333.0,
            min: 0.0,
            max: 100.0,
        });
        let stats = BundleStats {
            record_count: 1000,
            index_distinct: HashMap::new(),
            index_cardinalities: HashMap::new(),
            field_stats,
        };
        let sel = est.selectivity(
            &QueryCondition::Between("temp".into(), Value::Float(20.0), Value::Float(30.0)),
            &stats,
        );
        assert!((sel - 0.1).abs() < 0.01, "sel(20..30 out of 0..100) = 0.1, got {sel}");
    }

    /// TDD-10.7: Conjunction selectivity — independence assumption.
    #[test]
    fn tdd_10_7_conjunction_selectivity() {
        let est = CostEstimator::default();
        let stats = make_stats(10_000, &[
            ("organism", &[("S. aureus", 100)]),  // sel = 100/10K = 0.01
            ("target", &[("PBP", 500)]),          // sel = 500/10K = 0.05
        ]);
        let conds = [
            QueryCondition::Eq("organism".into(), Value::Text("S. aureus".into())),
            QueryCondition::Eq("target".into(), Value::Text("PBP".into())),
        ];
        let sel = est.conjunction_selectivity(&conds, &stats);
        assert!((sel - 0.0005).abs() < 0.0001, "sel(p1 AND p2) = 0.01 * 0.05 = 0.0005, got {sel}");
    }

    /// TDD-10.8: EXPLAIN output contains key information.
    #[test]
    fn tdd_10_8_explain_output() {
        let est = CostEstimator::default();
        let stats = make_stats(1_000_000, &[
            ("organism", &[("S. aureus", 1000)]),
        ]);
        let conds = [
            QueryCondition::Eq("organism".into(), Value::Text("S. aureus".into())),
            QueryCondition::Lt("mic".into(), Value::Float(4.0)),
        ];
        let plan = est.plan(&conds, &stats);
        let output = explain(&plan);
        assert!(output.contains("IndexLookup"), "EXPLAIN should mention IndexLookup");
        assert!(output.contains("1000"), "EXPLAIN should show estimated rows");
        assert!(output.contains("Residual"), "EXPLAIN should mention residual predicates");
    }

    /// TDD-10.9: Planner determinism — same input → same plan.
    #[test]
    fn tdd_10_9_planner_determinism() {
        let est = CostEstimator::default();
        let stats = make_stats(1_000_000, &[
            ("organism", &[("S. aureus", 10_000)]),
            ("target", &[("PBP", 500)]),
        ]);
        let conds = [
            QueryCondition::Eq("organism".into(), Value::Text("S. aureus".into())),
            QueryCondition::Eq("target".into(), Value::Text("PBP".into())),
        ];
        let plan0 = est.plan(&conds, &stats);
        for _ in 0..100 {
            let plan_i = est.plan(&conds, &stats);
            assert_eq!(plan_i.access, plan0.access, "plan should be deterministic");
            assert!(
                (plan_i.estimated_cost - plan0.estimated_cost).abs() < f64::EPSILON,
                "cost should be deterministic"
            );
        }
    }
}
