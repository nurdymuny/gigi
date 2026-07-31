//! CHANGEPOINTS — regime-shift detection via a sliding two-sample t-statistic.
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Request for `POST /v1/bundles/{name}/changepoints`.
#[derive(Deserialize)]
pub struct ChangepointRequest {
    /// Numeric field(s) to monitor. Omitted → all numeric fibers (except the time field).
    #[serde(default)]
    pub value: Option<String>,
    /// Field to order the series by. Omitted → a time-named numeric fiber, else record order.
    #[serde(default)]
    pub time: Option<String>,
    /// Sliding-window half-width (also the minimum segment length). Default 40.
    #[serde(default = "default_cp_window")]
    pub window: usize,
    /// Detection threshold on the two-sample t-statistic. Lower = more sensitive. Default 6.
    #[serde(default = "default_cp_threshold")]
    pub threshold: f64,
    /// Cap on the number of changepoints returned (strongest first). 0 = all.
    #[serde(default)]
    pub max_changepoints: usize,
}
pub fn default_cp_window() -> usize { 40 }
pub fn default_cp_threshold() -> f64 { 6.0 }

#[derive(Debug)]
pub struct ChangepointResult {
    pub base: String,
    pub n: usize,
    pub window: usize,
    pub threshold: f64,
    pub time_field: Option<String>,
    pub value_fields: Vec<String>,
    pub changepoints: Vec<(usize, String, f64, String, f64, f64)>,  // index, time, stat, top field, mean_before, mean_after
    pub notes: Vec<String>,
}

/// Changepoint detection: order the records into a series, then slide a two-window
/// two-sample t-statistic |μ_after − μ_before| / (s·√(2/w)) across it; peaks above
/// `threshold` (non-max-suppressed, ≥ window apart) are regime shifts. Multivariate
/// fields aggregate as the RMS per-field statistic. This is detection, not
/// forecasting (which GIGI does not do).
pub fn detect_changepoints(
    engine: &Engine, name: &str, value: &Option<String>, time: &Option<String>,
    window: usize, threshold: f64, max_cp: usize,
) -> Result<ChangepointResult, (StatusCode, String)> {
    use crate::types::FieldType;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "changepoints requires a single base-key field".into()));
    }
    let base = schema.base_fields[0].name.clone();
    let numeric: Vec<String> = schema.fiber_fields.iter()
        .filter(|f| matches!(f.field_type, FieldType::Numeric)).map(|f| f.name.clone()).collect();
    // choose the time-ordering field
    let time_named = |nm: &str| { let l = nm.to_lowercase();
        ["day", "date", "time", "hour", "ts", "week", "month", "order", "seq", "index", "step"].iter().any(|k| l.contains(k)) };
    let time_field = match time {
        Some(t) => {
            if schema.fiber_fields.iter().any(|f| &f.name == t) { Some(t.clone()) }
            else { return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("time field '{t}' not found"))); }
        }
        None => numeric.iter().find(|f| time_named(f)).cloned(),
    };
    // value fields
    let value_fields: Vec<String> = match value {
        Some(v) => {
            if !numeric.contains(v) { return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("value field '{v}' must be a NUMERIC fiber"))); }
            vec![v.clone()]
        }
        None => numeric.iter().filter(|f| Some(*f) != time_field.as_ref()).cloned().collect(),
    };
    if value_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "changepoints needs at least one NUMERIC value fiber".into()));
    }
    let mut records: Vec<crate::types::Record> = store.records().collect();
    // order the series
    if let Some(tf) = &time_field {
        records.sort_by(|a, b| {
            let av = a.get(tf).and_then(|v| v.as_f64());
            let bv = b.get(tf).and_then(|v| v.as_f64());
            match (av, bv) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal),
                _ => a.get(tf).map(|v| format!("{}", v)).cmp(&b.get(tf).map(|v| format!("{}", v))),
            }
        });
    }
    let n = records.len();
    let w = window.max(2);
    if n < 2 * w + 1 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "changepoints needs at least 2·window+1 = {} records (have {}); reduce `window`", 2 * w + 1, n)));
    }
    // standardized value series per field
    let series: Vec<Vec<f64>> = value_fields.iter().map(|f| {
        let raw: Vec<f64> = records.iter().map(|r| r.get(f).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let mu = raw.iter().sum::<f64>() / n as f64;
        let sd = (raw.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n as f64).sqrt().max(1e-9);
        raw.iter().map(|x| (x - mu) / sd).collect()
    }).collect();
    // sliding-window two-sample statistic (RMS over fields) + per-field means for attribution
    let mut stat = vec![0.0f64; n];
    for i in w..n - w {
        let mut sumsq = 0.0;
        for s in &series {
            let (a, b) = (&s[i - w..i], &s[i..i + w]);
            let (ma, mb) = (a.iter().sum::<f64>() / w as f64, b.iter().sum::<f64>() / w as f64);
            let va = a.iter().map(|x| (x - ma).powi(2)).sum::<f64>() / w as f64;
            let vb = b.iter().map(|x| (x - mb).powi(2)).sum::<f64>() / w as f64;
            let sp = ((va + vb) / 2.0).sqrt().max(1e-9);
            let t = (mb - ma).abs() / (sp * (2.0 / w as f64).sqrt());
            sumsq += t * t;
        }
        stat[i] = (sumsq / series.len() as f64).sqrt();
    }
    // non-max suppression: peaks above threshold, ≥ w apart
    let mut cps: Vec<(usize, f64)> = Vec::new();
    let mut i = w;
    while i < n - w {
        let lo = i.saturating_sub(w).max(w);
        let hi = (i + w).min(n - w);
        let local_max = stat[lo..hi].iter().cloned().fold(0.0, f64::max);
        if stat[i] > threshold && (stat[i] - local_max).abs() < 1e-12 {
            cps.push((i, stat[i]));
            i += w;
        } else { i += 1; }
    }
    // attribute each changepoint to its top field + report before/after means (raw)
    let raw_series: Vec<Vec<f64>> = value_fields.iter().map(|f|
        records.iter().map(|r| r.get(f).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect()).collect();
    cps.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if max_cp > 0 { cps.truncate(max_cp); }
    let time_at = |i: usize| time_field.as_ref()
        .and_then(|tf| records[i].get(tf)).map(|v| format!("{}", v))
        .unwrap_or_else(|| i.to_string());
    let changepoints: Vec<(usize, String, f64, String, f64, f64)> = cps.iter().map(|&(i, s)| {
        // field with the largest local mean shift
        let (fi, _) = (0..value_fields.len()).map(|f| {
            let before = raw_series[f][i - w..i].iter().sum::<f64>() / w as f64;
            let after = raw_series[f][i..i + w].iter().sum::<f64>() / w as f64;
            (f, (after - before).abs())
        }).max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
        let before = raw_series[fi][i - w..i].iter().sum::<f64>() / w as f64;
        let after = raw_series[fi][i..i + w].iter().sum::<f64>() / w as f64;
        (i, time_at(i), (s * 1000.0).round() / 1000.0, value_fields[fi].clone(),
         (before * 1000.0).round() / 1000.0, (after * 1000.0).round() / 1000.0)
    }).collect();
    let mut notes = vec![format!("changepoints: window {w}, threshold {threshold}, {} value field(s) [{}]",
        value_fields.len(), value_fields.join(", "))];
    match &time_field {
        Some(tf) => notes.push(format!("series ordered by '{tf}'")),
        None => notes.push("no time-like field found — series left in record order".into()),
    }
    Ok(ChangepointResult { base, n, window: w, threshold, time_field, value_fields, changepoints, notes })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// Changepoint detection recovers the regime shifts of a piecewise-constant
    /// series (mean jumps at known indices) and leaves flat regions unflagged.
    #[test]
    fn changepoints_recover_regime_shifts() {
        let mut s: u64 = 918273;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            (s >> 11) as f64 / (1u64 << 53) as f64 };   // U[0,1)
        let mut gauss = || { let (u1, u2): (f64, f64) = (rnd().max(1e-9), rnd());
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() };
        // segments with means 0, 4, 1 → true changepoints at 100 and 220
        let segs = [(0.0, 100usize), (4.0, 120usize), (1.0, 100usize)];
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut t = 0;
        for (m, len) in segs {
            for _ in 0..len {
                rows.push(scan_rec(&[
                    ("id", V::Text(format!("t{t:04}"))),
                    ("t", V::Float(t as f64)),
                    ("v", V::Float(m + gauss())),
                ]));
                t += 1;
            }
        }
        let schema = BundleSchema::new("ts")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("t")).fiber(FieldDef::numeric("v"));
        let (dir, engine) = scan_env("changepoints", "ts", schema, rows);
        let cr = detect_changepoints(&engine, "ts", &Some("v".into()), &Some("t".into()), 30, 6.0, 0).expect("changepoints");
        // exactly the two true shifts (±window tolerance), no spurious ones
        assert_eq!(cr.changepoints.len(), 2, "should find 2 changepoints, got {:?}",
            cr.changepoints.iter().map(|c| c.0).collect::<Vec<_>>());
        let idxs: Vec<usize> = cr.changepoints.iter().map(|c| c.0).collect();
        assert!(idxs.iter().any(|&i| (i as i64 - 100).abs() <= 30), "expected a changepoint near 100, got {idxs:?}");
        assert!(idxs.iter().any(|&i| (i as i64 - 220).abs() <= 30), "expected a changepoint near 220, got {idxs:?}");
        // the strongest changepoint reports a real before/after mean gap
        let top = &cr.changepoints[0];
        assert!((top.5 - top.4).abs() > 1.5, "changepoint should show a mean shift, got {} → {}", top.4, top.5);
        cleanup(&dir);
    }

    /// Changepoints endpoint gives actionable errors, not panics.
    #[test]
    fn changepoints_guards_are_actionable() {
        let rows: Vec<_> = (0..20).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("t", V::Float(i as f64)), ("v", V::Float((i % 3) as f64)),
        ])).collect();
        let schema = BundleSchema::new("s")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("t")).fiber(FieldDef::numeric("v"));
        let (dir, engine) = scan_env("cp_guard", "s", schema, rows);
        // window too big for the series → clear error
        let e = detect_changepoints(&engine, "s", &Some("v".into()), &Some("t".into()), 40, 6.0, 0).unwrap_err();
        assert_eq!(e.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(e.1.contains("window"), "got: {}", e.1);
        // unknown value field → clear error
        assert!(detect_changepoints(&engine, "s", &Some("nope".into()), &None, 5, 6.0, 0).unwrap_err().1.contains("value field"));
        // missing bundle → 404
        assert_eq!(detect_changepoints(&engine, "no", &None, &None, 5, 6.0, 0).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
