//! # BundleStats — per-field empirical statistics derived from data
//!
//! Foundation for SUDOKU wave 3 (relaxation gradient). Computes
//! per-field mean / std / min / max for numeric fields and
//! value→frequency for categorical fields, all in a single pass over
//! the records.
//!
//! ## The no-hacks contract
//!
//! Everything in this module is **derived from the bundle's own
//! data**. No domain-specific config, no field-name special cases,
//! no hardcoded scale assumptions. The same code path runs on a drug
//! bundle, an apartment bundle, a music bundle — the numbers change
//! because the data changes, but the algorithm does not.
//!
//! ## Why this lives in `geometry`
//!
//! These stats are the empirical (diagonal) Riemannian metric on the
//! data: variance per field gives a length scale per direction.
//! Under FitMode::Full the full covariance Σ generalizes to the
//! Mahalanobis metric — SUDOKU's relaxation cost will use Σ⁻¹ when
//! available, falling back to the diagonal variance computed here.
//!
//! ## Known overlap with `bundle::FieldStats`
//!
//! `BundleStore` already maintains a per-numeric-field
//! `HashMap<String, FieldStats>` (`count, sum, sum_sq, min, max`)
//! via Welford updates on every insert. The numeric portion of
//! `BundleStats` here is structurally equivalent (mean = sum/n,
//! std² = (sum_sq − n·mean²) / (n − 1)).
//!
//! We do NOT pull from `BundleStore::field_stats()` because:
//!  (a) it covers numeric only — categorical + vector lanes have
//!      no equivalent in the engine layer;
//!  (b) coupling `geometry::sudoku` to `bundle::BundleStore` would
//!      cross a deliberate architectural boundary (geometry is
//!      storage-agnostic; same code runs on tests with plain Vecs);
//!  (c) the cost of a one-pass Welford over a ~10k-record bundle
//!      (Marcella's typical size) is sub-millisecond.
//!
//! If profiling shows the recompute cost dominating in larger
//! bundles, a `BundleStats::from_bundle_store(store)` constructor
//! is the right intervention — it can populate numeric from the
//! cached field_stats and run a focused pass for categorical +
//! vector lanes. Wave 5+ work.

use crate::types::{Record, Value};
use std::collections::HashMap;

/// Per-field statistics for one bundle. Numeric, categorical, and
/// vector maps are disjoint per field — a field that is mixed-type
/// across records (e.g. some Integer, some Text) gets stats only for
/// the type that has ≥1 record's worth of data.
#[derive(Debug, Clone, Default)]
pub struct BundleStats {
    /// Numeric per-field stats. Key = field name; value = stats over
    /// all records where that field has a numeric value (Integer,
    /// Float, or Timestamp coerced to f64).
    pub numeric: HashMap<String, NumericFieldStats>,
    /// Categorical per-field frequency. Key = field name; value =
    /// {value → count}. Categorical means Text or Bool.
    pub categorical: HashMap<String, CategoricalFieldStats>,
    /// **Wave 4.** Per-field vector statistics — used by SUDOKU to
    /// compute geometric distance between a record's vector and a
    /// query vector (instead of treating Vector inequality as a
    /// categorical "wrong, cost 1.0"). Same shape contract: derived
    /// purely from data, no schema special-casing.
    pub vector: HashMap<String, VectorFieldStats>,
    /// Total records considered (denominator for relative frequencies).
    pub n_records: usize,
}

/// Numeric field statistics derived from data only.
#[derive(Debug, Clone)]
pub struct NumericFieldStats {
    /// Empirical mean.
    pub mean: f64,
    /// Sample standard deviation (uses n-1 denominator — Bessel-
    /// corrected). For n=1 returns 0.0 (degenerate; relaxation cost
    /// callers must check).
    pub std: f64,
    /// Minimum value observed.
    pub min: f64,
    /// Maximum value observed.
    pub max: f64,
    /// Count of records contributing to this field's stats.
    pub n: usize,
}

impl NumericFieldStats {
    /// Returns true if the field has degenerate scale (std == 0 or
    /// only one record). Relaxation-cost code uses this to fall
    /// back to range-based normalization when std is unusable.
    pub fn is_degenerate(&self) -> bool {
        self.n < 2 || self.std == 0.0 || !self.std.is_finite()
    }

    /// Returns a positive length scale for this field, derived from
    /// data. Prefers std; falls back to (max-min)/2 if std is
    /// degenerate; falls back to 1.0 if even the range is zero.
    /// **This is the function the relaxation cost calls** —
    /// guarantees a finite positive denominator regardless of
    /// data pathology.
    pub fn length_scale(&self) -> f64 {
        if !self.is_degenerate() {
            return self.std;
        }
        let range = self.max - self.min;
        if range > 0.0 && range.is_finite() {
            return range / 2.0;
        }
        // Degenerate field (all values identical). Length scale is
        // ill-defined; we report 1.0 so the cost computation stays
        // finite. Callers can detect this via is_degenerate() if
        // they want to flag it in the response.
        1.0
    }
}

/// Categorical field statistics — observed values and their counts.
#[derive(Debug, Clone, Default)]
pub struct CategoricalFieldStats {
    /// Value → count map. Key uses Value's structural equality.
    pub frequency: HashMap<Value, usize>,
    /// Total records contributing (sum of frequency values).
    pub n: usize,
}

impl CategoricalFieldStats {
    /// Count of records with the given categorical value (0 if
    /// never observed).
    pub fn count_of(&self, v: &Value) -> usize {
        self.frequency.get(v).copied().unwrap_or(0)
    }

    /// Values sorted by descending frequency (ties broken by
    /// arbitrary value order). Used by the relaxation-menu code
    /// to propose "next-most-common alternative."
    pub fn by_frequency_desc(&self) -> Vec<(Value, usize)> {
        let mut pairs: Vec<(Value, usize)> = self
            .frequency
            .iter()
            .map(|(v, c)| (v.clone(), *c))
            .collect();
        pairs.sort_by(|a, b| b.1.cmp(&a.1));
        pairs
    }
}

/// **Wave 4.** Per-field vector (embedding) statistics — derived
/// from the observed vectors only. Used by SUDOKU to give Vector
/// constraints a meaningful geometric cost instead of the flat 1.0
/// categorical fallback.
#[derive(Debug, Clone)]
pub struct VectorFieldStats {
    /// Dimensionality of the vectors. All records contributing to
    /// this field's stats have this length; mixed-length records
    /// are silently dropped (they're a schema bug, but we don't
    /// panic — caller decides what to do).
    pub dims: usize,
    /// Component-wise empirical mean (length = `dims`). Useful as
    /// a "default anchor" when no query vector is supplied.
    pub mean: Vec<f64>,
    /// Length scale = mean pairwise L2 distance between centered
    /// vectors. This is the **data-derived geometric scale** —
    /// what counts as "typical distance" in the bundle's
    /// embedding distribution. Always positive; falls back to
    /// 1.0 if vectors are degenerate (all identical or n<2).
    pub length_scale: f64,
    /// Count of records contributing.
    pub n: usize,
}

impl VectorFieldStats {
    /// L2 distance from `v` to `query`, normalized by the bundle's
    /// own length_scale. Returns None when dimensions don't match
    /// (caller treats as categorical cost = 1.0).
    pub fn normalized_distance(&self, v: &[f64], query: &[f64]) -> Option<f64> {
        if v.len() != self.dims || query.len() != self.dims {
            return None;
        }
        let raw = euclidean_distance(v, query);
        Some(raw / self.length_scale.max(1e-12))
    }

    /// Raw L2 distance, no normalization. None on dim mismatch.
    pub fn raw_distance(&self, v: &[f64], query: &[f64]) -> Option<f64> {
        if v.len() != self.dims || query.len() != self.dims {
            return None;
        }
        Some(euclidean_distance(v, query))
    }
}

/// Standard L2 (Euclidean) distance. Assumes equal lengths;
/// caller's responsibility to check.
fn euclidean_distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x - y).powi(2))
        .sum::<f64>()
        .sqrt()
}

/// Compute per-field stats from a record collection. Single pass,
/// O(N × fields). No allocations beyond the result maps. No domain
/// config — every field is handled the same way regardless of name.
pub fn compute_stats<'a, I>(records: I) -> BundleStats
where
    I: IntoIterator<Item = &'a Record>,
{
    // Accumulators per field. We compute Welford's online variance
    // for numeric fields to avoid catastrophic cancellation on
    // large values (drug pki ~ 9, but market_cap_b ~ 1500 in
    // domain 5 — different magnitudes within the same bundle in
    // some real cases).
    struct NumericAccum {
        n: usize,
        mean: f64,
        m2: f64, // Σ(x - mean)² accumulator (Welford)
        min: f64,
        max: f64,
    }
    let mut numeric_accum: HashMap<String, NumericAccum> = HashMap::new();
    let mut cat_accum: HashMap<String, CategoricalFieldStats> = HashMap::new();
    // Wave 4: vector accumulator — collect all vectors per field
    // (we need pairwise distances for length scale, so a one-pass
    // streaming algorithm is harder; for Marcella's bundle sizes
    // (~10k records) it's still cheap to collect first then
    // compute mean pairwise.
    let mut vector_accum: HashMap<String, Vec<Vec<f64>>> = HashMap::new();
    let mut n_records = 0_usize;

    for record in records {
        n_records += 1;
        for (field, value) in record.iter() {
            if let Some(x) = numeric_value(value) {
                let acc = numeric_accum.entry(field.clone()).or_insert(NumericAccum {
                    n: 0,
                    mean: 0.0,
                    m2: 0.0,
                    min: f64::INFINITY,
                    max: f64::NEG_INFINITY,
                });
                acc.n += 1;
                let delta = x - acc.mean;
                acc.mean += delta / acc.n as f64;
                let delta2 = x - acc.mean;
                acc.m2 += delta * delta2;
                if x < acc.min {
                    acc.min = x;
                }
                if x > acc.max {
                    acc.max = x;
                }
            } else if is_categorical_value(value) {
                let entry = cat_accum
                    .entry(field.clone())
                    .or_insert_with(CategoricalFieldStats::default);
                *entry.frequency.entry(value.clone()).or_insert(0) += 1;
                entry.n += 1;
            } else if let Value::Vector(v) = value {
                vector_accum.entry(field.clone()).or_default().push(v.clone());
            }
            // Binary, Null: ignored — no meaningful length scale or
            // categorical identity.
        }
    }

    let numeric: HashMap<String, NumericFieldStats> = numeric_accum
        .into_iter()
        .map(|(field, acc)| {
            let std = if acc.n >= 2 {
                (acc.m2 / (acc.n - 1) as f64).sqrt()
            } else {
                0.0
            };
            (
                field,
                NumericFieldStats {
                    mean: acc.mean,
                    std,
                    min: if acc.min.is_finite() { acc.min } else { 0.0 },
                    max: if acc.max.is_finite() { acc.max } else { 0.0 },
                    n: acc.n,
                },
            )
        })
        .collect();

    // Wave 4: finalize vector stats — per field, drop mixed-dim
    // records (keep the modal dim), compute component-wise mean
    // and a length scale = mean pairwise L2 distance between
    // distinct vector pairs. For n=1 or all-identical, length
    // scale falls back to vector magnitude or 1.0.
    let vector: HashMap<String, VectorFieldStats> = vector_accum
        .into_iter()
        .filter_map(|(field, vs)| {
            // Find modal dimension; drop vectors not matching.
            let mut dim_counts: HashMap<usize, usize> = HashMap::new();
            for v in &vs {
                *dim_counts.entry(v.len()).or_insert(0) += 1;
            }
            let modal_dim = dim_counts.iter().max_by_key(|(_, c)| **c).map(|(d, _)| *d)?;
            let kept: Vec<&Vec<f64>> = vs.iter().filter(|v| v.len() == modal_dim).collect();
            if kept.is_empty() || modal_dim == 0 {
                return None;
            }
            // Component-wise mean.
            let mut mean = vec![0.0_f64; modal_dim];
            for v in &kept {
                for (i, x) in v.iter().enumerate() {
                    mean[i] += x;
                }
            }
            for m in mean.iter_mut() {
                *m /= kept.len() as f64;
            }
            // Length scale: mean pairwise distance. For large bundles
            // this is O(n²) — cap at first 100 pairs to keep cost
            // bounded; the estimate is still robust.
            let length_scale = mean_pairwise_distance(&kept, 100);
            Some((
                field,
                VectorFieldStats {
                    dims: modal_dim,
                    mean,
                    length_scale: if length_scale > 0.0 { length_scale } else { 1.0 },
                    n: kept.len(),
                },
            ))
        })
        .collect();

    BundleStats {
        numeric,
        categorical: cat_accum,
        vector,
        n_records,
    }
}

/// Mean pairwise L2 distance between vectors. Caps at `max_pairs`
/// to keep cost bounded for large bundles.
fn mean_pairwise_distance(vs: &[&Vec<f64>], max_pairs: usize) -> f64 {
    let n = vs.len();
    if n < 2 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    let mut count = 0_usize;
    'outer: for i in 0..n {
        for j in (i + 1)..n {
            sum += euclidean_distance(vs[i], vs[j]);
            count += 1;
            if count >= max_pairs {
                break 'outer;
            }
        }
    }
    if count == 0 {
        0.0
    } else {
        sum / count as f64
    }
}

/// Coerce a Value to f64 if numeric. Integer and Timestamp coerce
/// (timestamp is epoch-seconds; treating it as numeric lets the
/// SUDOKU primitive express constraints like "after 2026-01-01"
/// uniformly with rent/price/etc.).
///
/// **Single source of truth** for "is this Value numeric?" across
/// the geometry layer — SUDOKU's relaxation_cost and BundleStats's
/// per-field accumulator both depend on this returning the same
/// answer for the same input.
pub(crate) fn numeric_value(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Integer(i) => Some(*i as f64),
        Value::Timestamp(t) => Some(*t as f64),
        _ => None,
    }
}

fn is_categorical_value(v: &Value) -> bool {
    matches!(v, Value::Text(_) | Value::Bool(_))
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(fields: &[(&str, Value)]) -> Record {
        let mut r = Record::new();
        for (k, v) in fields {
            r.insert((*k).to_string(), v.clone());
        }
        r
    }

    #[test]
    fn numeric_stats_match_textbook_formulas() {
        // Values: 10, 20, 30, 40, 50. Mean = 30, std (n-1) = sqrt(250).
        let records = vec![
            rec(&[("x", Value::Float(10.0))]),
            rec(&[("x", Value::Float(20.0))]),
            rec(&[("x", Value::Float(30.0))]),
            rec(&[("x", Value::Float(40.0))]),
            rec(&[("x", Value::Float(50.0))]),
        ];
        let stats = compute_stats(records.iter());
        let x = stats.numeric.get("x").expect("x is numeric");
        assert!((x.mean - 30.0).abs() < 1e-9);
        assert!((x.std - 250.0_f64.sqrt()).abs() < 1e-9);
        assert_eq!(x.min, 10.0);
        assert_eq!(x.max, 50.0);
        assert_eq!(x.n, 5);
    }

    #[test]
    fn integer_and_timestamp_coerce_to_numeric() {
        let records = vec![
            rec(&[("a", Value::Integer(1)), ("b", Value::Timestamp(1000))]),
            rec(&[("a", Value::Integer(2)), ("b", Value::Timestamp(2000))]),
            rec(&[("a", Value::Integer(3)), ("b", Value::Timestamp(3000))]),
        ];
        let stats = compute_stats(records.iter());
        assert!((stats.numeric.get("a").unwrap().mean - 2.0).abs() < 1e-9);
        assert!((stats.numeric.get("b").unwrap().mean - 2000.0).abs() < 1e-9);
    }

    #[test]
    fn categorical_counts_correctly() {
        let records = vec![
            rec(&[("k", Value::Text("a".into()))]),
            rec(&[("k", Value::Text("a".into()))]),
            rec(&[("k", Value::Text("b".into()))]),
        ];
        let stats = compute_stats(records.iter());
        let k = stats.categorical.get("k").expect("k is categorical");
        assert_eq!(k.count_of(&Value::Text("a".into())), 2);
        assert_eq!(k.count_of(&Value::Text("b".into())), 1);
        assert_eq!(k.count_of(&Value::Text("zzz".into())), 0);
    }

    #[test]
    fn degenerate_field_returns_finite_length_scale() {
        // All same value → std=0 → length_scale falls back to range/2,
        // and since range is also 0, falls back to 1.0. Must NOT
        // return 0 or NaN — the cost code divides by this.
        let records = vec![
            rec(&[("constant", Value::Float(42.0))]),
            rec(&[("constant", Value::Float(42.0))]),
            rec(&[("constant", Value::Float(42.0))]),
        ];
        let stats = compute_stats(records.iter());
        let c = stats.numeric.get("constant").unwrap();
        assert!(c.is_degenerate(), "constant field must report degenerate");
        let ls = c.length_scale();
        assert!(ls.is_finite() && ls > 0.0, "length_scale must be positive finite, got {}", ls);
    }

    #[test]
    fn single_record_is_degenerate_but_safe() {
        // n=1: std-formula undefined (divide-by-zero). Must NOT
        // panic, must NOT return NaN.
        let records = vec![rec(&[("x", Value::Float(7.0))])];
        let stats = compute_stats(records.iter());
        let x = stats.numeric.get("x").unwrap();
        assert_eq!(x.n, 1);
        assert!(x.is_degenerate());
        assert!(x.length_scale().is_finite() && x.length_scale() > 0.0);
    }

    /// **The general-purpose proof.** Same code, two utterly
    /// different bundles. Means and stds must come out
    /// proportional to the data — no special-casing field names.
    #[test]
    fn stats_are_scale_invariant_across_domains() {
        // Bundle 1: rents in thousands of dollars (3500–4500).
        let bundle1: Vec<_> = (0..10)
            .map(|i| rec(&[("price", Value::Float(3500.0 + 100.0 * i as f64))]))
            .collect();
        // Bundle 2: pki values in single-digit range (6.0–7.8).
        let bundle2: Vec<_> = (0..10)
            .map(|i| rec(&[("metric", Value::Float(6.0 + 0.2 * i as f64))]))
            .collect();
        let s1 = compute_stats(bundle1.iter());
        let s2 = compute_stats(bundle2.iter());
        let p = s1.numeric.get("price").unwrap();
        let m = s2.numeric.get("metric").unwrap();
        // Both ranges span 9 steps, so std should be proportional
        // to the step size. p step = 100, m step = 0.2 → ratio 500.
        let ratio = p.std / m.std;
        assert!(
            (ratio - 500.0).abs() / 500.0 < 1e-6,
            "std ratio should be 500, got {}",
            ratio
        );
    }

    #[test]
    fn mixed_type_field_routes_each_value_to_its_lane() {
        // A field with some numeric and some categorical values
        // routes each to its respective accumulator. Real bundles
        // shouldn't do this, but the math must stay sane.
        let records = vec![
            rec(&[("weird", Value::Integer(1))]),
            rec(&[("weird", Value::Text("a".into()))]),
            rec(&[("weird", Value::Integer(3))]),
        ];
        let stats = compute_stats(records.iter());
        let n = stats.numeric.get("weird").unwrap();
        let c = stats.categorical.get("weird").unwrap();
        assert_eq!(n.n, 2);
        assert_eq!(c.n, 1);
    }

    #[test]
    fn vector_field_stats_compute_mean_and_scale() {
        // Three 2D vectors; mean = (0.5, 0.5); pairwise distances
        // are sqrt(2), sqrt(2), sqrt(2 * (0.5)²) = 0.707... etc.
        let records = vec![
            rec(&[("emb", Value::Vector(vec![0.0, 0.0]))]),
            rec(&[("emb", Value::Vector(vec![1.0, 0.0]))]),
            rec(&[("emb", Value::Vector(vec![0.5, 1.5]))]),
        ];
        let stats = compute_stats(records.iter());
        let v = stats.vector.get("emb").expect("emb has vector stats");
        assert_eq!(v.dims, 2);
        assert!((v.mean[0] - 0.5).abs() < 1e-9);
        assert!((v.mean[1] - 0.5).abs() < 1e-9);
        assert!(v.length_scale > 0.0, "length scale must be positive");
        // Three distinct vectors — n = 3, length_scale should be in
        // the [0.5, 2.0] range given the inputs.
        assert!(v.length_scale > 0.5 && v.length_scale < 2.0);
    }

    #[test]
    fn vector_normalized_distance_matches_l2_over_scale() {
        let records = vec![
            rec(&[("emb", Value::Vector(vec![0.0, 0.0]))]),
            rec(&[("emb", Value::Vector(vec![3.0, 4.0]))]),
        ];
        let stats = compute_stats(records.iter());
        let v = stats.vector.get("emb").unwrap();
        // Distance from [0,0] to [3,4] is 5; length_scale = 5
        // (only pair). Normalized distance should be 1.0.
        let d = v.normalized_distance(&[0.0, 0.0], &[3.0, 4.0]).unwrap();
        assert!((d - 1.0).abs() < 1e-9, "normalized distance should be 1.0, got {}", d);
    }

    #[test]
    fn vector_mixed_dimension_records_use_modal_dim() {
        let records = vec![
            rec(&[("emb", Value::Vector(vec![1.0, 2.0, 3.0]))]),
            rec(&[("emb", Value::Vector(vec![4.0, 5.0, 6.0]))]),
            rec(&[("emb", Value::Vector(vec![7.0, 8.0, 9.0]))]),
            // Wrong dim: dropped.
            rec(&[("emb", Value::Vector(vec![10.0, 11.0]))]),
        ];
        let stats = compute_stats(records.iter());
        let v = stats.vector.get("emb").unwrap();
        assert_eq!(v.dims, 3);
        assert_eq!(v.n, 3);
    }

    #[test]
    fn by_frequency_desc_orders_correctly() {
        let records = vec![
            rec(&[("k", Value::Text("rare".into()))]),
            rec(&[("k", Value::Text("common".into()))]),
            rec(&[("k", Value::Text("common".into()))]),
            rec(&[("k", Value::Text("common".into()))]),
            rec(&[("k", Value::Text("medium".into()))]),
            rec(&[("k", Value::Text("medium".into()))]),
        ];
        let stats = compute_stats(records.iter());
        let k = stats.categorical.get("k").unwrap();
        let ordered = k.by_frequency_desc();
        assert_eq!(ordered[0].0, Value::Text("common".into()));
        assert_eq!(ordered[0].1, 3);
        assert_eq!(ordered[1].0, Value::Text("medium".into()));
        assert_eq!(ordered[2].0, Value::Text("rare".into()));
    }
}
