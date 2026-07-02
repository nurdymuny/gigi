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
        // The sum-of-squares form suffers catastrophic cancellation when
        // mean^2 >> variance (e.g. values ~1e6 with spread ~1e-2) and can
        // dip below zero in floating point, which would turn stddev() into
        // NaN. Clamp at zero; FieldStats uses Welford's m2 and is the
        // preferred accumulator when incremental update is available.
        ((self.sum_sq / n) - (self.sum / n).powi(2)).max(0.0)
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

/// Autocorrelation-honest error bar for a mean over an ORDERED sample
/// sequence (Monte Carlo chains, time series).
///
/// The naive standard error sqrt(var/n) assumes independent samples;
/// correlated chains violate that and the naive bar is always too small —
/// flattering, and wrong. This computes:
///
/// - `tau_int`: integrated autocorrelation time, 1/2 + Σ ρ(k), summed with
///   an initial-positive-sequence window (stop at the first non-positive
///   autocorrelation, capped at n/4). For iid data τ ≈ 0.5.
/// - `n_eff = n / (2 τ_int)`: effective independent sample count.
/// - `err = err_naive · sqrt(2 τ_int)`: the autocorrelation-corrected bar.
/// - `err_jack`: delete-one-block jackknife error with block length
///   ceil(2 τ_int) — a second, independent estimate; if the two disagree
///   badly the chain is under-sampled and neither should be trusted.
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorBar {
    pub mean: f64,
    pub err: f64,
    pub err_naive: f64,
    pub err_jack: f64,
    pub tau_int: f64,
    pub n_eff: f64,
    pub n: usize,
    pub blocks: usize,
}

/// Compute an [`ErrorBar`] for the mean of `samples`, which must already be
/// in chain/time order. Returns None for fewer than 4 samples (no
/// meaningful error estimate exists).
pub fn jackknife(samples: &[f64]) -> Option<ErrorBar> {
    let n = samples.len();
    if n < 4 {
        return None;
    }
    let nf = n as f64;
    let mean = samples.iter().sum::<f64>() / nf;
    let dev: Vec<f64> = samples.iter().map(|x| x - mean).collect();
    let c0 = dev.iter().map(|d| d * d).sum::<f64>() / nf;
    if c0 <= 0.0 {
        // constant series: exact mean, zero error
        return Some(ErrorBar {
            mean,
            err: 0.0,
            err_naive: 0.0,
            err_jack: 0.0,
            tau_int: 0.5,
            n_eff: nf,
            n,
            blocks: n,
        });
    }

    // integrated autocorrelation time, initial-positive-sequence window
    let mut tau_int = 0.5;
    let max_lag = n / 4;
    for k in 1..=max_lag {
        let ck = dev[..n - k]
            .iter()
            .zip(&dev[k..])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / nf;
        let rho = ck / c0;
        if rho <= 0.0 {
            break;
        }
        tau_int += rho;
    }

    let var = c0 * nf / (nf - 1.0); // unbiased sample variance
    let err_naive = (var / nf).sqrt();
    let err = err_naive * (2.0 * tau_int).sqrt();
    let n_eff = nf / (2.0 * tau_int);

    // blocked jackknife on the mean, block length ~ 2 tau_int
    let block_len = ((2.0 * tau_int).ceil() as usize).clamp(1, n / 4);
    let blocks = n / block_len;
    let used = blocks * block_len;
    let total: f64 = samples[..used].iter().sum();
    let mut jack_means = Vec::with_capacity(blocks);
    for b in 0..blocks {
        let block_sum: f64 = samples[b * block_len..(b + 1) * block_len].iter().sum();
        jack_means.push((total - block_sum) / (used - block_len) as f64);
    }
    let jbar = jack_means.iter().sum::<f64>() / blocks as f64;
    let jvar = jack_means.iter().map(|m| (m - jbar).powi(2)).sum::<f64>()
        * (blocks as f64 - 1.0)
        / blocks as f64;
    let err_jack = jvar.sqrt();

    Some(ErrorBar {
        mean,
        err,
        err_naive,
        err_jack,
        tau_int,
        n_eff,
        n,
        blocks,
    })
}

/// Execute `INTEGRATE ... MEASURE avg(f), ... WITH JACKKNIFE ALONG order` —
/// shared by both query executors.
///
/// `measures` is (output_column_base, field). Records are grouped by
/// `over` (if any), ordered within each group by `order_field`, and each
/// measure gets a full [`ErrorBar`]: `<base>`, `<base>_err`,
/// `<base>_err_naive`, `<base>_err_jack`, `<base>_tau_int`, `<base>_n_eff`.
pub fn jackknife_rows<I>(
    records: I,
    over: Option<&str>,
    order_field: &str,
    skip_first: usize,
    measures: &[(String, String)],
) -> Result<Vec<crate::types::Record>, String>
where
    I: IntoIterator<Item = crate::types::Record>,
{
    use std::collections::HashMap as Map;
    // group -> ordered list of (order_value, one sample per measure)
    let mut groups: Map<Value, Vec<(Value, Vec<Option<f64>>)>> = Map::new();
    for rec in records {
        let group_val = match over {
            Some(g) => match rec.get(g) {
                Some(v) => v.clone(),
                None => continue,
            },
            None => Value::Null,
        };
        let Some(order_val) = rec.get(order_field).cloned() else {
            continue;
        };
        let samples: Vec<Option<f64>> = measures
            .iter()
            .map(|(_, f)| rec.get(f.as_str()).and_then(|v| v.as_f64()))
            .collect();
        groups.entry(group_val).or_default().push((order_val, samples));
    }

    let mut rows = Vec::new();
    for (group_val, mut series) in groups {
        series.sort_by(|a, b| a.0.cmp(&b.0));
        // SKIP FIRST n — thermalization cut, applied AFTER ordering so
        // "first" means first along the chain, not insertion order.
        if skip_first > 0 {
            if skip_first >= series.len() {
                return Err(format!(
                    "SKIP FIRST {skip_first} discards every sample \
                     (group has {} ordered samples); JACKKNIFE needs at \
                     least 4 remaining",
                    series.len()
                ));
            }
            series.drain(..skip_first);
        }
        let mut row: crate::types::Record = Map::new();
        if let Some(g) = over {
            row.insert(g.to_string(), group_val);
        }
        for (mi, (base, field)) in measures.iter().enumerate() {
            let samples: Vec<f64> = series.iter().filter_map(|(_, s)| s[mi]).collect();
            match jackknife(&samples) {
                Some(eb) => {
                    row.insert(base.clone(), Value::Float(eb.mean));
                    row.insert(format!("{base}_err"), Value::Float(eb.err));
                    row.insert(format!("{base}_err_naive"), Value::Float(eb.err_naive));
                    row.insert(format!("{base}_err_jack"), Value::Float(eb.err_jack));
                    row.insert(format!("{base}_tau_int"), Value::Float(eb.tau_int));
                    row.insert(format!("{base}_n_eff"), Value::Float(eb.n_eff));
                }
                None => {
                    return Err(format!(
                        "JACKKNIFE needs at least 4 ordered samples of '{field}' \
                         per group; got {}{}",
                        samples.len(),
                        if skip_first > 0 {
                            format!(" (after SKIP FIRST {skip_first})")
                        } else {
                            String::new()
                        }
                    ));
                }
            }
        }
        rows.push(row);
    }
    Ok(rows)
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

    /// iid samples: tau_int ≈ 0.5, corrected bar ≈ naive bar, and the
    /// blocked jackknife agrees with both.
    #[test]
    fn test_jackknife_iid() {
        // xorshift64* — same PRNG family the engine uses; fixed seed
        let mut s: u64 = 0x9E3779B97F4A7C15;
        let mut next = move || {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            (s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
        };
        let samples: Vec<f64> = (0..20_000).map(|_| next() - 0.5).collect();
        let eb = jackknife(&samples).unwrap();
        assert!((eb.mean).abs() < 0.01, "mean {}", eb.mean);
        assert!(eb.tau_int < 0.7, "iid tau_int should be ~0.5, got {}", eb.tau_int);
        let ratio = eb.err / eb.err_naive;
        assert!((0.9..1.25).contains(&ratio), "err/naive {}", ratio);
        let jratio = eb.err_jack / eb.err;
        assert!((0.7..1.4).contains(&jratio), "jack/err {}", jratio);
    }

    /// AR(1) with phi = 0.8 has analytic tau_int = (1+phi)/(2(1-phi)) = 4.5;
    /// the corrected bar must be ~3x the naive bar (sqrt(2*4.5) = 3.0) and
    /// the jackknife must agree with the corrected bar, not the naive one.
    #[test]
    fn test_jackknife_ar1_known_tau() {
        let phi = 0.8f64;
        let mut s: u64 = 42;
        let mut next = move || {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            (s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        let mut x = 0.0f64;
        let samples: Vec<f64> = (0..100_000)
            .map(|_| {
                x = phi * x + next();
                x
            })
            .collect();
        let eb = jackknife(&samples).unwrap();
        assert!(
            (3.2..6.0).contains(&eb.tau_int),
            "AR(1) phi=0.8 tau_int should be near 4.5, got {}",
            eb.tau_int
        );
        let inflation = eb.err / eb.err_naive;
        assert!(
            (2.4..3.6).contains(&inflation),
            "err inflation should be near 3.0, got {inflation}"
        );
        let jratio = eb.err_jack / eb.err;
        assert!(
            (0.6..1.5).contains(&jratio),
            "jackknife should agree with corrected bar, got ratio {jratio}"
        );
        assert!(eb.n_eff < 20_000.0, "n_eff must be far below n, got {}", eb.n_eff);
    }

    #[test]
    fn test_jackknife_too_few_samples() {
        assert!(jackknife(&[1.0, 2.0, 3.0]).is_none());
        let eb = jackknife(&[5.0; 100]).unwrap();
        assert_eq!(eb.err, 0.0);
        assert_eq!(eb.mean, 5.0);
    }

    /// Build one record with an order field and a value field.
    fn rec(order: i64, v: f64) -> crate::types::Record {
        let mut r = crate::types::Record::new();
        r.insert("sweep".into(), Value::Integer(order));
        r.insert("x".into(), Value::Float(v));
        r
    }

    /// SKIP FIRST is a thermalization cut: with a hot burn-in at the
    /// head of the chain, the uncut mean is biased and the cut result
    /// must equal a jackknife over exactly the clean tail.
    #[test]
    fn test_jackknife_skip_first_cuts_burn_in() {
        let mut s: u64 = 7;
        let mut next = move || {
            s ^= s >> 12;
            s ^= s << 25;
            s ^= s >> 27;
            (s.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64 - 0.5
        };
        let n_burn = 200usize;
        let n_total = 1200usize;
        let values: Vec<f64> = (0..n_total)
            .map(|i| if i < n_burn { 50.0 + next() } else { next() })
            .collect();
        // insertion order shuffled-ish (reverse) to prove the cut is
        // applied along the ORDER field, not arrival order
        let records: Vec<_> =
            (0..n_total).rev().map(|i| rec(i as i64, values[i])).collect();
        let measures = vec![("x".to_string(), "x".to_string())];

        let uncut =
            jackknife_rows(records.clone(), None, "sweep", 0, &measures).unwrap();
        let uncut_mean = uncut[0]["x"].as_f64().unwrap();
        assert!(
            uncut_mean > 5.0,
            "burn-in should bias the uncut mean, got {uncut_mean}"
        );

        let cut =
            jackknife_rows(records, None, "sweep", n_burn, &measures).unwrap();
        let cut_mean = cut[0]["x"].as_f64().unwrap();
        let clean = jackknife(&values[n_burn..]).unwrap();
        assert_eq!(
            cut_mean, clean.mean,
            "cut result must equal a jackknife over exactly the clean tail"
        );
        assert!(cut_mean.abs() < 0.1, "clean-tail mean near 0, got {cut_mean}");
    }

    /// A cut that discards everything is a loud error, not an empty row.
    #[test]
    fn test_jackknife_skip_first_all_is_error() {
        let records: Vec<_> = (0..10).map(|i| rec(i, i as f64)).collect();
        let measures = vec![("x".to_string(), "x".to_string())];
        let err = jackknife_rows(records, None, "sweep", 10, &measures).unwrap_err();
        assert!(err.contains("discards every sample"), "{err}");
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
