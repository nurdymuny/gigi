//! SCAN — zero-config geometric anomaly lenses (+ /scan/fit supervised weights).
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Character-trigram set of a string (lowercased), for the SCAN text lens.
pub fn scan_trigrams(s: &str) -> std::collections::HashSet<String> {
    let chars: Vec<char> = s.to_lowercase().chars().collect();
    if chars.len() < 3 {
        return std::iter::once(chars.iter().collect::<String>()).collect();
    }
    (0..=chars.len() - 3).map(|i| chars[i..i + 3].iter().collect::<String>()).collect()
}
/// Jaccard similarity of two trigram sets ∈ [0,1].
pub fn scan_jaccard(a: &std::collections::HashSet<String>, b: &std::collections::HashSet<String>) -> f64 {
    let inter = a.intersection(b).count() as f64;
    let uni = a.union(b).count() as f64;
    if uni == 0.0 { 0.0 } else { inter / uni }
}

/// Solve the linear system `a · x = b` in place by Gaussian elimination with
/// partial pivoting (`a` is consumed). Returns `None` if the matrix is singular.
/// Used by the completion lens' inverse-power iteration.
pub fn scan_solve(a: &mut [Vec<f64>], b: &[f64]) -> Option<Vec<f64>> {
    let n = a.len();
    let mut b = b.to_vec();
    for c in 0..n {
        let piv = (c..n).max_by(|&r1, &r2| a[r1][c].abs()
            .partial_cmp(&a[r2][c].abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        if a[piv][c].abs() < 1e-12 { return None; }
        a.swap(c, piv); b.swap(c, piv);
        for r in 0..n {
            if r != c {
                let f = a[r][c] / a[c][c];
                for k in c..n { a[r][k] -= f * a[c][k]; }
                b[r] -= f * b[c];
            }
        }
    }
    Some((0..n).map(|i| b[i] / a[i][i]).collect())
}

/// Request for `POST /v1/bundles/{name}/scan`.
#[derive(Deserialize)]
pub struct ScanRequest {
    /// Review budget: flag the top `budget` fraction of records. Default 0.05.
    #[serde(default = "default_scan_budget")]
    pub budget: f64,
    /// Optional per-lens weights for a supervised linear combiner
    /// (lens-name → weight). When present, the fused score is
    /// `Σ wₗ · normalized_lensₗ` instead of the default max-fusion — supply
    /// weights learned from confirmed frauds to lift recall past the
    /// unsupervised plateau. When absent, lenses fuse by max (OR-semantics).
    #[serde(default)]
    pub weights: Option<std::collections::HashMap<String, f64>>,
    /// Cap on returned rows (0 = all, already sorted most-anomalous first).
    #[serde(default)]
    pub limit: usize,
    /// Fields to keep OUT of the geometry — e.g. an outcome/label column, or
    /// any field that would leak. Unsupervised /scan cannot know a column is
    /// "leaky"; name it here and it is excluded from every lens.
    #[serde(default)]
    pub exclude: Vec<String>,
}
pub fn default_scan_budget() -> f64 { 0.05 }

/// POST /v1/bundles/{name}/scan
///
/// GIGI's one-call, zero-config anomaly detector. Auto-introspects the schema
/// and fuses a battery of geometric lenses — **global** curvature, **contextual**
/// curvature (cohort-local field stats per categorical field), **velocity** over
/// (highest-cardinality entity × time), **text** (typo-squat/rare values on
/// name-like fields), **relational** (auto-foreign-key attribute mismatch),
/// **completion** (local tangent-plane residual — records off the manifold their
/// neighbors define), and **density** (Local Outlier Factor — records on the
/// manifold and in-range but in a locally sparse pocket) —
/// into a per-record score WITH lens attribution. The only input is the bundle
/// name; no feature engineering. Returns records sorted most-anomalous first,
/// each with the lens that fired and the per-lens breakdown.
/// Shared SCAN engine: introspect the schema, build every geometric lens, and
/// rank-normalize each to [0,1]. Returns (lens_names, per-lens normalized maps,
/// record ids, base-key name, n). Used by both /scan (fusion) and /scan/fit
/// (supervised weight learning).
/// Result of SCAN's lens computation — shared by /scan and /scan/fit.
#[derive(Debug)]
pub struct ScanLenses {
    pub lens_names: Vec<String>,
    pub norm: Vec<std::collections::HashMap<String, f64>>,
    pub ids: Vec<String>,
    pub base: String,
    pub n: usize,
    /// Human-readable diagnostics: which lenses were built and why others were skipped.
    pub notes: Vec<String>,
}

pub fn scan_compute_lenses(
    engine: &Engine,
    name: &str,
    exclude: &[String],
) -> Result<ScanLenses, (StatusCode, String)> {
    use crate::types::{FieldType, Value};
    use std::collections::HashMap;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "SCAN requires a single base-key field".into()));
    }
    let base = schema.base_fields[0].name.clone();
    let mut notes: Vec<String> = Vec::new();
    // excluded fields (e.g. the fit label) never enter the geometry
    let usable = |f: &crate::types::FieldDef| !exclude.iter().any(|e| e == &f.name);
    let cats: Vec<String> = schema.fiber_fields.iter()
        .filter(|f| usable(f) && matches!(f.field_type, FieldType::Categorical | FieldType::OrderedCat { .. }))
        .map(|f| f.name.clone()).collect();
    let num_defs: Vec<crate::types::FieldDef> = schema.fiber_fields.iter()
        .filter(|f| usable(f) && matches!(f.field_type, FieldType::Numeric))
        .cloned().collect();
    if num_defs.is_empty() {
        notes.push("no numeric fibers: the global/contextual/velocity lenses are unavailable — add a NUMERIC fiber to enable them".into());
    }

    let records: Vec<crate::types::Record> = store.records().collect();
    let n = records.len();
    if n == 0 {
        return Ok(ScanLenses { lens_names: Vec::new(), norm: Vec::new(), ids: Vec::new(), base, n: 0, notes });
    }
    let idof = |r: &crate::types::Record| r.get(&base).map(|v| format!("{}", v)).unwrap_or_default();
    let ids: Vec<String> = records.iter().map(&idof).collect();

    // Text-like fields (names/memos — ≥30% of distinct values contain a space or
    // dot, i.e. not ID codes) route to the TEXT lens. They are excluded from the
    // contextual-curvature lens below, where per-cohort curvature over free text
    // is noise (the amount-context signal is covered by the true code fields).
    let is_text_field = |f: &str| {
        let vals: std::collections::HashSet<String> =
            records.iter().filter_map(|r| r.get(f).map(|v| format!("{}", v))).collect();
        !vals.is_empty() && {
            let hit = vals.iter().filter(|v| v.contains(' ') || v.contains('.')).count();
            (hit as f64) / (vals.len() as f64) >= 0.30
        }
    };
    let text_fields: Vec<String> = cats.iter().filter(|c| is_text_field(c)).cloned().collect();
    if text_fields.is_empty() {
        notes.push("text lens skipped: no text-like categorical fields (values are mostly short codes, not names/memos)".into());
    } else {
        notes.push(format!("text lens on: {}", text_fields.join(", ")));
    }

    // lens-name → (record-id → raw signal)
    let mut lenses: Vec<(String, HashMap<String, f64>)> = Vec::new();

    // ── global curvature anomaly ──
    // Computed over `num_defs` (which honors `exclude`) rather than
    // store.compute_anomalies, so an excluded field (e.g. the fit label) can
    // never leak into the geometry through the bundle's own field stats.
    if !num_defs.is_empty() {
        let mut stats: HashMap<String, crate::bundle::FieldStats> = HashMap::new();
        for r in &records {
            for fd in &num_defs {
                if let Some(x) = r.get(&fd.name).and_then(|v| v.as_f64()) {
                    stats.entry(fd.name.clone()).or_default().update(x);
                }
            }
        }
        let ks: Vec<f64> = records.iter().map(|r| {
            let vals: Vec<Value> = num_defs.iter()
                .map(|fd| r.get(&fd.name).cloned().unwrap_or(Value::Null)).collect();
            crate::bundle::compute_record_k(&stats, &vals, &num_defs)
        }).collect();
        let cn = ks.len() as f64;
        let mu = ks.iter().sum::<f64>() / cn;
        let sd = (ks.iter().map(|k| (k - mu).powi(2)).sum::<f64>() / cn).sqrt();
        let m = records.iter().zip(&ks).map(|(r, k)| {
            let z = if sd < f64::EPSILON { 0.0 } else { (k - mu) / sd };
            (idof(r), z.max(0.0))
        }).collect();
        lenses.push(("global".to_string(), m));
    }

    // ── contextual curvature: cohort-local field stats per categorical field ──
    for cf in &cats {
        if text_fields.contains(cf) { continue; }   // text fields → text lens, not curvature
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (ix, r) in records.iter().enumerate() {
            if let Some(v) = r.get(cf) { groups.entry(format!("{}", v)).or_default().push(ix); }
        }
        let d = groups.len();
        if d < 4 {
            notes.push(format!("context:{cf} skipped: only {d} distinct value(s) (need >= 4)"));
            continue;
        }
        if (d as f64) > (n as f64) / 5.0 {
            notes.push(format!("context:{cf} skipped: {d} distinct values, too unique to form cohorts"));
            continue;
        }
        let mut sizes: Vec<usize> = groups.values().map(|v| v.len()).collect();
        sizes.sort_unstable();
        if sizes[sizes.len() / 2] < 3 {
            notes.push(format!("context:{cf} skipped: cohorts too small (median < 3 records)"));
            continue;
        }
        if num_defs.is_empty() { break; }
        let mut m: HashMap<String, f64> = HashMap::new();
        for idxs in groups.values() {
            if idxs.len() < 3 { for &ix in idxs { m.insert(ids[ix].clone(), 0.0); } continue; }
            // cohort-local Welford field stats
            let mut stats: HashMap<String, crate::bundle::FieldStats> = HashMap::new();
            for &ix in idxs {
                for fd in &num_defs {
                    if let Some(x) = records[ix].get(&fd.name).and_then(|v| v.as_f64()) {
                        stats.entry(fd.name.clone()).or_default().update(x);
                    }
                }
            }
            // cohort-local K per record, then z within the cohort
            let ks: Vec<(usize, f64)> = idxs.iter().map(|&ix| {
                let vals: Vec<Value> = num_defs.iter()
                    .map(|fd| records[ix].get(&fd.name).cloned().unwrap_or(Value::Null)).collect();
                (ix, crate::bundle::compute_record_k(&stats, &vals, &num_defs))
            }).collect();
            let cn = ks.len() as f64;
            let mu = ks.iter().map(|(_, k)| k).sum::<f64>() / cn;
            let sd = (ks.iter().map(|(_, k)| (k - mu).powi(2)).sum::<f64>() / cn).sqrt();
            for (ix, k) in ks {
                let z = if sd < f64::EPSILON { 0.0 } else { (k - mu) / sd };
                m.insert(ids[ix].clone(), z.max(0.0));
            }
        }
        lenses.push((format!("context:{}", cf), m));
    }

    // ── velocity: (highest-cardinality entity × most-granular TIME) burst count ──
    // Pick the time axis by NAME (day/date/time/hour/…), not by distinct-count —
    // otherwise `amount` (thousands of distinct values) would masquerade as time.
    let time_named = |nm: &str| {
        let l = nm.to_lowercase();
        ["day", "date", "time", "hour", "ts", "week", "month"].iter().any(|k| l.contains(k))
    };
    let time_field = num_defs.iter().filter(|f| time_named(&f.name))
        .max_by_key(|f| records.iter()
            .filter_map(|r| r.get(&f.name).and_then(|v| v.as_f64()).map(|x| x as i64))
            .collect::<std::collections::HashSet<_>>().len())
        .map(|f| f.name.clone());
    if !num_defs.is_empty() && (cats.is_empty() || time_field.is_none()) {
        notes.push("velocity skipped: needs a categorical entity field and a time-like numeric field (name containing day/date/time/hour/...)".into());
    }
    if let (false, Some(time_f)) = (cats.is_empty(), time_field) {
        let entity = cats.iter().max_by_key(|c| records.iter()
            .filter_map(|r| r.get(*c).map(|v| format!("{}", v)))
            .collect::<std::collections::HashSet<_>>().len()).unwrap().clone();
        let bucket = |r: &crate::types::Record| (
            r.get(&entity).map(|v| format!("{}", v)).unwrap_or_default(),
            r.get(&time_f).and_then(|v| v.as_f64()).unwrap_or(0.0) as i64,
        );
        let mut counts: HashMap<(String, i64), usize> = HashMap::new();
        for r in &records { *counts.entry(bucket(r)).or_insert(0) += 1; }
        let m = records.iter().map(|r| (idof(r), *counts.get(&bucket(r)).unwrap_or(&1) as f64)).collect();
        lenses.push(("velocity".to_string(), m));
    }

    // ── text lens: typo-squat (near-duplicate to a known value) + rare value ──
    // Runs on text-like categorical fields (≥30% of distinct values contain a
    // space or a dot — names/memos, not ID codes). Catches entity/description
    // fraud that the numeric-curvature lenses are blind to.
    {
        if !text_fields.is_empty() {
            let mut m: HashMap<String, f64> = ids.iter().map(|i| (i.clone(), 0.0)).collect();
            for tf in &text_fields {
                let mut freq: HashMap<String, usize> = HashMap::new();
                for r in &records {
                    if let Some(v) = r.get(tf) { *freq.entry(format!("{}", v)).or_insert(0) += 1; }
                }
                let maxf = *freq.values().max().unwrap_or(&1) as f64;
                let low_card = freq.len() <= 25;   // memo-like: rarity is a clean signal
                let tris: HashMap<String, std::collections::HashSet<String>> =
                    freq.keys().map(|v| (v.clone(), scan_trigrams(v))).collect();
                let frequent: Vec<&String> = freq.iter().filter(|(_, c)| **c >= 8).map(|(v, _)| v).collect();
                // score per distinct value, then broadcast to records
                let mut vscore: HashMap<String, f64> = HashMap::new();
                for (v, c) in &freq {
                    // near-duplicate to a DIFFERENT frequent value, in the "near but
                    // not exact" band → typo-squat (Amazon.com vs Arnaz0n)
                    let nd = if *c < 8 {
                        let best = frequent.iter().filter(|fv| **fv != v)
                            .map(|fv| scan_jaccard(&tris[v], &tris[*fv]))
                            .fold(0.0_f64, f64::max);
                        if (0.45..=0.97).contains(&best) { best } else { 0.0 }
                    } else { 0.0 };
                    let rare = 1.0 - (*c as f64) / maxf;
                    vscore.insert(v.clone(), nd.max(if low_card { 0.85 * rare } else { 0.4 * rare }));
                }
                for r in &records {
                    if let Some(v) = r.get(tf) {
                        let s = *vscore.get(&format!("{}", v)).unwrap_or(&0.0);
                        let e = m.get_mut(&idof(r)).unwrap();
                        if s > *e { *e = s; }
                    }
                }
            }
            lenses.push(("text".to_string(), m));
        }
    }

    // ── relational lens: auto-foreign-key join → attribute mismatch ──
    // For any categorical fiber that equals ANOTHER bundle's base key, join to
    // that bundle and flag records whose attribute disagrees with the joined
    // entity's corresponding attribute (fields matched automatically by value-
    // domain overlap, e.g. txn.region vs account.home_region → impossible travel),
    // weighted by the record's amount percentile.
    {
        let amt_field = num_defs.iter()
            .find(|f| { let l = f.name.to_lowercase(); l.contains("amount") || l.contains("amt") })
            .or_else(|| num_defs.first()).map(|f| f.name.clone());
        let amt_sorted: Vec<f64> = amt_field.as_ref().map(|af| {
            let mut v: Vec<f64> = records.iter().filter_map(|r| r.get(af).and_then(|x| x.as_f64())).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            v
        }).unwrap_or_default();
        let amt_pct = |a: f64| if amt_sorted.is_empty() { 1.0 }
            else { amt_sorted.partition_point(|&x| x < a) as f64 / amt_sorted.len() as f64 };
        let other_names: Vec<String> = engine.bundle_names().iter()
            .filter(|b| **b != name && !b.starts_with("_gigi"))
            .map(|s| s.to_string()).collect();
        for cf in &cats {
            // find a bundle whose base key == cf (the foreign-key target)
            let Some(ob) = other_names.iter().find(|ob| engine.bundle(ob)
                .and_then(|os| os.schema().base_fields.first().map(|f| f.name == *cf))
                .unwrap_or(false)) else { continue };
            let Some(ostore) = engine.bundle(ob) else { continue };
            let obase = ostore.schema().base_fields[0].name.clone();
            let ocats: Vec<String> = ostore.schema().fiber_fields.iter()
                .filter(|f| matches!(f.field_type, FieldType::Categorical | FieldType::OrderedCat { .. }))
                .map(|f| f.name.clone()).collect();
            let orecs: Vec<crate::types::Record> = ostore.records().collect();
            let right: HashMap<String, &crate::types::Record> = orecs.iter()
                .filter_map(|r| r.get(&obase).map(|v| (format!("{}", v), r))).collect();
            // match left↔right categorical fields by shared value universe
            let domain = |recs: &[crate::types::Record], field: &str| -> std::collections::HashSet<String> {
                recs.iter().filter_map(|r| r.get(field).map(|v| format!("{}", v))).collect() };
            let mut pairs: Vec<(String, String)> = Vec::new();
            for lc in cats.iter().filter(|c| *c != cf) {
                let ld = domain(&records, lc);
                for rc in &ocats {
                    let rd = domain(&orecs, rc);
                    let inter = ld.intersection(&rd).count();
                    let uni = ld.union(&rd).count();
                    if inter >= 2 && uni > 0 && (inter as f64 / uni as f64) > 0.5 {
                        pairs.push((lc.clone(), rc.clone()));
                    }
                }
            }
            if pairs.is_empty() { continue; }
            let mut m: HashMap<String, f64> = ids.iter().map(|i| (i.clone(), 0.0)).collect();
            for r in &records {
                let fk = r.get(cf).map(|v| format!("{}", v)).unwrap_or_default();
                let Some(rr) = right.get(&fk) else { continue };
                let mut best = 0.0f64;
                for (lc, rc) in &pairs {
                    if let (Some(lv), Some(rv)) = (r.get(lc).map(|v| format!("{}", v)), rr.get(rc).map(|v| format!("{}", v))) {
                        if lv != rv {
                            let a = amt_field.as_ref().and_then(|af| r.get(af)).and_then(|x| x.as_f64()).unwrap_or(0.0);
                            best = best.max(0.4 + 0.6 * amt_pct(a));
                        }
                    }
                }
                if best > 0.0 { *m.get_mut(&idof(r)).unwrap() = best; }
            }
            lenses.push((format!("relational:{}~{}", cf, ob), m));
        }
    }

    // ── completion + density lenses: two signals off a shared local k-NN ──
    // Both read the SAME exact k nearest neighbors (in standardized numeric space):
    //   • completion — orthogonal residual OFF the local tangent hyperplane (the
    //     least-variance direction of the neighbor covariance, via inverse-power
    //     iteration): catches records that sit off the manifold their neighbors
    //     define, i.e. globally ordinary on every axis but locally inconsistent.
    //   • density — Local Outlier Factor: a record's local reachability density
    //     relative to its neighbors', catching records that are ON the manifold and
    //     in-range yet sit in a locally SPARSE pocket.
    // Both are blind spots of the axis-wise global/contextual lenses (and of flat
    // detectors like Isolation Forest).
    {
        const COMPLETION_MAX_N: usize = 8000;   // exact-neighbor cost cap
        const COMPLETION_MIN_N: usize = 20;      // need enough records for stable local fits
        // standardize numeric fibers (global z-score); drop zero-variance fields
        let cols: Vec<(String, f64, f64)> = num_defs.iter().filter_map(|fd| {
            let xs: Vec<f64> = records.iter()
                .map(|r| r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
            let mu = xs.iter().sum::<f64>() / n as f64;
            let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n as f64).sqrt();
            (sd > f64::EPSILON).then_some((fd.name.clone(), mu, sd))
        }).collect();
        let dim = cols.len();
        if dim < 2 {
            if !num_defs.is_empty() {
                notes.push("completion & density lenses skipped: need >= 2 numeric fibers with non-zero variance to define a local manifold".into());
            }
        } else if n < COMPLETION_MIN_N {
            notes.push(format!("completion & density lenses skipped: {n} records is too few for stable local fits (need >= {COMPLETION_MIN_N})"));
        } else if n > COMPLETION_MAX_N {
            notes.push(format!("completion & density lenses skipped: {n} records exceeds the exact-neighbor limit ({COMPLETION_MAX_N}) in this version"));
        } else {
            let x: Vec<Vec<f64>> = records.iter().map(|r| cols.iter()
                .map(|(f, mu, sd)| (r.get(f).and_then(|v| v.as_f64()).unwrap_or(*mu) - mu) / sd)
                .collect()).collect();
            let k = (2 * dim + 1).max(10).min(n - 1);
            let dist2 = |a: &[f64], b: &[f64]| a.iter().zip(b)
                .map(|(p, q)| (p - q) * (p - q)).sum::<f64>();
            // shared exact k-NN: neighbor indices + euclidean distances, per record
            let mut nbr: Vec<Vec<usize>> = Vec::with_capacity(n);
            let mut ndist: Vec<Vec<f64>> = Vec::with_capacity(n);
            for i in 0..n {
                let mut d: Vec<(f64, usize)> = (0..n).filter(|&j| j != i)
                    .map(|j| (dist2(&x[i], &x[j]), j)).collect();
                d.select_nth_unstable_by(k - 1, |a, b| a.0
                    .partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                let top = &d[..k];
                nbr.push(top.iter().map(|(_, j)| *j).collect());
                ndist.push(top.iter().map(|(dd, _)| dd.sqrt()).collect());
            }
            // ── completion: orthogonal residual off the local tangent hyperplane ──
            let mut comp: HashMap<String, f64> = HashMap::new();
            for i in 0..n {
                let nbrs = &nbr[i];
                // neighbor mean (leave-one-out: excludes i) and covariance
                let mut mu = vec![0.0; dim];
                for &j in nbrs { for t in 0..dim { mu[t] += x[j][t]; } }
                for t in 0..dim { mu[t] /= k as f64; }
                let mut c = vec![vec![0.0f64; dim]; dim];
                let mut trace = 0.0;
                for &j in nbrs {
                    for a in 0..dim { for b in 0..dim {
                        c[a][b] += (x[j][a] - mu[a]) * (x[j][b] - mu[b]);
                    }}
                }
                for a in 0..dim { for b in 0..dim { c[a][b] /= k as f64; } trace += c[a][a]; }
                // regularize so the covariance is invertible even when rank-deficient
                let reg = 1e-6 * (trace / dim as f64).max(f64::EPSILON);
                for a in 0..dim { c[a][a] += reg; }
                // inverse-power iteration → eigenvector of the SMALLEST eigenvalue
                // of C (the local normal / least-variance direction)
                let mut v: Vec<f64> = (0..dim).map(|t| 1.0 / (t as f64 + 1.0)).collect();
                let vn = v.iter().map(|z| z * z).sum::<f64>().sqrt();
                for z in &mut v { *z /= vn; }
                let mut ok = true;
                for _ in 0..16 {
                    match scan_solve(&mut c.clone(), &v) {
                        Some(u) => {
                            let un = u.iter().map(|z| z * z).sum::<f64>().sqrt();
                            if un < f64::EPSILON { ok = false; break; }
                            v = u.into_iter().map(|z| z / un).collect();
                        }
                        None => { ok = false; break; }
                    }
                }
                // orthogonal residual: distance of x[i] off the local tangent plane
                let residual = if ok {
                    (0..dim).map(|t| (x[i][t] - mu[t]) * v[t]).sum::<f64>().abs()
                } else { 0.0 };
                comp.insert(ids[i].clone(), residual);
            }
            lenses.push(("completion".to_string(), comp));
            // ── density: Local Outlier Factor over the same k-NN ──
            // kdist(o) = distance to o's k-th neighbor; reach(i,o) = max(kdist(o), d(i,o));
            // lrd(i) = k / Σ_o reach(i,o); LOF(i) = mean_o lrd(o) / lrd(i)  (>1 ⇒ sparse).
            let kdist: Vec<f64> = ndist.iter()
                .map(|v| v.iter().cloned().fold(0.0_f64, f64::max)).collect();
            let lrd: Vec<f64> = (0..n).map(|i| {
                let s: f64 = nbr[i].iter().zip(&ndist[i]).map(|(&o, &d)| kdist[o].max(d)).sum();
                if s < f64::EPSILON { f64::MAX.sqrt() } else { k as f64 / s }
            }).collect();
            let mut dens: HashMap<String, f64> = HashMap::new();
            for i in 0..n {
                let mean_nbr_lrd = nbr[i].iter().map(|&o| lrd[o]).sum::<f64>() / k as f64;
                let lof = if lrd[i] < f64::EPSILON { 1.0 } else { mean_nbr_lrd / lrd[i] };
                dens.insert(ids[i].clone(), lof);
            }
            lenses.push(("density".to_string(), dens));
            notes.push(format!("completion + density lenses on {dim} numeric fibers (k={k} neighbors; shared exact k-NN)"));
        }
    }

    if lenses.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "SCAN built no lenses for bundle '{}'. It needs at least one NUMERIC fiber (for curvature/velocity) or a text-like CATEGORICAL fiber (names/memos). Present: base='{}', categorical=[{}], numeric=[{}]. Diagnostics: {}",
            name, base, cats.join(", "),
            num_defs.iter().map(|f| f.name.clone()).collect::<Vec<_>>().join(", "),
            if notes.is_empty() { "none".to_string() } else { notes.join("; ") })));
    }

    // ── rank-normalize each lens to [0,1] over all records ──
    let lens_names: Vec<String> = lenses.iter().map(|(nm, _)| nm.clone()).collect();
    let norm: Vec<HashMap<String, f64>> = lenses.iter().map(|(_, m)| {
        let mut distinct: Vec<f64> = ids.iter().map(|i| *m.get(i).unwrap_or(&0.0)).collect();
        distinct.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        distinct.dedup();
        let denom = (distinct.len().saturating_sub(1)).max(1) as f64;
        let rankmap: HashMap<u64, f64> = distinct.iter().enumerate()
            .map(|(k, v)| (v.to_bits(), k as f64 / denom)).collect();
        ids.iter().map(|i| {
            let raw = *m.get(i).unwrap_or(&0.0);
            (i.clone(), *rankmap.get(&raw.to_bits()).unwrap_or(&0.0))
        }).collect()
    }).collect();

    Ok(ScanLenses { lens_names, norm, ids, base, n, notes })
}

/// Request for `POST /v1/bundles/{name}/scan/fit`.
#[derive(Deserialize)]
pub struct ScanFitRequest {
    /// Name of a 0/1 or boolean fiber marking confirmed frauds.
    pub label_field: String,
    /// Cross-validation folds for the held-out estimate. Default 5.
    #[serde(default = "default_fit_folds")]
    pub folds: usize,
    /// Gradient-descent epochs. Default 400.
    #[serde(default = "default_fit_epochs")]
    pub epochs: usize,
}
pub fn default_fit_folds() -> usize { 5 }
pub fn default_fit_epochs() -> usize { 400 }

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_lens, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// A mixed bundle builds the expected lens battery; every record scored on every lens.
    #[test]
    fn scan_mixed_bundle_builds_lenses() {
        let rows: Vec<_> = (0..25).map(|i| scan_rec(&[
            ("id", V::Text(format!("t{i}"))),
            ("acct", V::Text(format!("A{}", i % 5))),
            ("amount", V::Float(if i == 24 { 9999.0 } else { 10.0 + i as f64 })),
            ("day", V::Float((i % 7) as f64)),
        ])).collect();
        let schema = BundleSchema::new("tx")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("acct"))
            .fiber(FieldDef::numeric("amount"))
            .fiber(FieldDef::numeric("day"));
        let (dir, engine) = scan_env("scan_mixed", "tx", schema, rows);
        let sl = scan_compute_lenses(&engine, "tx", &[]).unwrap();
        assert_eq!(sl.n, 25);
        assert!(sl.lens_names.contains(&"global".to_string()), "expected global lens");
        assert!(sl.lens_names.iter().any(|l| l == "context:acct"), "expected per-account contextual lens");
        assert!(sl.lens_names.iter().any(|l| l == "velocity"), "expected velocity lens (day is time-like)");
        for m in &sl.norm { assert_eq!(m.len(), 25, "each lens scores every record"); }
        // the planted outlier (id=t24, amount=9999) tops the global lens
        let g = scan_lens(&sl, "global").unwrap();
        assert!(g["t24"] >= 0.99, "outlier should rank at top of global lens, got {}", g["t24"]);
        cleanup(&dir);
    }

    /// All-numeric bundle: only the global lens, with notes explaining the skips. (general-purpose)
    #[test]
    fn scan_all_numeric_only_global() {
        let rows: Vec<_> = (0..8).map(|i| scan_rec(&[
            ("id", V::Text(format!("m{i}"))),
            ("cpu", V::Float(if i == 7 { 99.0 } else { 10.0 })),
            ("mem", V::Float(20.0 + i as f64)),
        ])).collect();
        let schema = BundleSchema::new("metrics")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("cpu"))
            .fiber(FieldDef::numeric("mem"));
        let (dir, engine) = scan_env("scan_numonly", "metrics", schema, rows);
        let sl = scan_compute_lenses(&engine, "metrics", &[]).unwrap();
        assert_eq!(sl.lens_names, vec!["global".to_string()]);
        assert!(sl.notes.iter().any(|nt| nt.contains("text lens skipped")), "notes should explain no text lens");
        assert!(sl.notes.iter().any(|nt| nt.contains("velocity skipped")), "notes should explain no velocity lens");
        cleanup(&dir);
    }

    /// No numeric fibers and only a low-cardinality categorical → an actionable error, not a panic/blank.
    #[test]
    fn scan_no_usable_fibers_errors() {
        let rows = vec![
            scan_rec(&[("id", V::Text("a".into())), ("color", V::Text("red".into()))]),
            scan_rec(&[("id", V::Text("b".into())), ("color", V::Text("blue".into()))]),
            scan_rec(&[("id", V::Text("c".into())), ("color", V::Text("red".into()))]),
        ];
        let schema = BundleSchema::new("tags")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("color"));
        let (dir, engine) = scan_env("scan_nofib", "tags", schema, rows);
        let err = scan_compute_lenses(&engine, "tags", &[]).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("no lenses") && err.1.contains("NUMERIC"), "error must be actionable: {}", err.1);
        cleanup(&dir);
    }

    /// Empty bundle → Ok with n == 0 and no lenses (caller emits a friendly message).
    #[test]
    fn scan_empty_bundle_ok_zero() {
        let schema = BundleSchema::new("empty")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x"));
        let (dir, engine) = scan_env("scan_empty", "empty", schema, vec![]);
        let sl = scan_compute_lenses(&engine, "empty", &[]).unwrap();
        assert_eq!(sl.n, 0);
        assert!(sl.lens_names.is_empty());
        cleanup(&dir);
    }

    /// Missing bundle → NOT_FOUND, not a panic.
    #[test]
    fn scan_missing_bundle_not_found() {
        let (dir, engine) = scan_env("scan_missing", "real",
            BundleSchema::new("real").base(FieldDef::categorical("id")).fiber(FieldDef::numeric("x")), vec![]);
        let err = scan_compute_lenses(&engine, "ghost", &[]).unwrap_err();
        assert_eq!(err.0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }

    /// An excluded field never influences the geometry (guards fit against label leakage).
    #[test]
    fn scan_exclude_keeps_field_out_of_geometry() {
        // `label` is 0 everywhere except one record where it is huge; amount is uniform.
        let rows: Vec<_> = (0..10).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))),
            ("amount", V::Float(50.0)),
            ("label", V::Float(if i == 3 { 1000.0 } else { 0.0 })),
        ])).collect();
        let schema = BundleSchema::new("lk")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("amount"))
            .fiber(FieldDef::numeric("label"));
        let (dir, engine) = scan_env("scan_excl", "lk", schema, rows);
        let with = scan_compute_lenses(&engine, "lk", &[]).unwrap();
        let without = scan_compute_lenses(&engine, "lk", &["label".to_string()]).unwrap();
        let g_with = scan_lens(&with, "global").unwrap()["r3"];
        let g_without = scan_lens(&without, "global").unwrap()["r3"];
        assert!(g_with >= 0.99, "label should dominate global when included");
        assert!(g_without <= 0.01, "excluded label must not drive global; got {}", g_without);
        cleanup(&dir);
    }

    /// Degenerate data (single record, and constant numeric) must not panic or emit NaN.
    #[test]
    fn scan_degenerate_inputs_no_panic() {
        // single record
        let s1 = BundleSchema::new("one").base(FieldDef::categorical("id")).fiber(FieldDef::numeric("x"));
        let (d1, e1) = scan_env("scan_one", "one", s1, vec![scan_rec(&[("id", V::Text("a".into())), ("x", V::Float(1.0))])]);
        let sl1 = scan_compute_lenses(&e1, "one", &[]).unwrap();
        assert_eq!(sl1.n, 1);
        for m in &sl1.norm { for v in m.values() { assert!(v.is_finite(), "no NaN/Inf on single record"); } }
        cleanup(&d1);
        // constant numeric across many records (zero variance)
        let rows: Vec<_> = (0..12).map(|i| scan_rec(&[
            ("id", V::Text(format!("c{i}"))), ("acct", V::Text(format!("A{}", i % 4))), ("x", V::Float(7.0)),
        ])).collect();
        let s2 = BundleSchema::new("flat")
            .base(FieldDef::categorical("id")).fiber(FieldDef::categorical("acct")).fiber(FieldDef::numeric("x"));
        let (d2, e2) = scan_env("scan_flat", "flat", s2, rows);
        let sl2 = scan_compute_lenses(&e2, "flat", &[]).unwrap();
        for m in &sl2.norm { for v in m.values() { assert!(v.is_finite(), "no NaN/Inf on constant field"); } }
        cleanup(&d2);
    }

    /// The completion lens catches an OFF-MANIFOLD record that is ordinary on every
    /// axis — the class the axis-wise global curvature lens is blind to. Records lie
    /// on the plane z = x + y; one record sits off it while keeping x, y, z each
    /// in-range, so no single-axis view flags it, but the local tangent-plane
    /// residual does.
    #[test]
    fn scan_completion_catches_off_manifold() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for xi in 0..6 {
            for yi in 0..5 {
                rows.push(scan_rec(&[
                    ("id", V::Text(format!("g{xi}_{yi}"))),
                    ("x", V::Float(xi as f64)),
                    ("y", V::Float(yi as f64)),
                    ("z", V::Float((xi + yi) as f64)), // on the plane z = x + y
                ]));
            }
        }
        // anomaly: x=2, y=2 (both ordinary), z=8 off the plane (plane would give 4).
        // z=8 is still inside the global z-range [0, 9], so no single axis is extreme.
        rows.push(scan_rec(&[
            ("id", V::Text("anom".into())),
            ("x", V::Float(2.0)), ("y", V::Float(2.0)), ("z", V::Float(8.0)),
        ]));
        let schema = BundleSchema::new("surf")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x"))
            .fiber(FieldDef::numeric("y"))
            .fiber(FieldDef::numeric("z"));
        let (dir, engine) = scan_env("scan_completion", "surf", schema, rows);
        let sl = scan_compute_lenses(&engine, "surf", &[]).unwrap();
        let comp = scan_lens(&sl, "completion").expect("completion lens should be built");
        // the off-manifold record tops the completion lens
        assert!(comp["anom"] >= 0.99, "off-manifold record should top completion lens, got {}", comp["anom"]);
        // ...but it does NOT top the axis-wise global curvature lens (grid corners
        // deviate more on individual axes), proving the two lenses see different things
        let g = scan_lens(&sl, "global").unwrap();
        assert!(g["anom"] < comp["anom"],
            "global curvature should rank the off-manifold point below completion (global={}, completion={})",
            g["anom"], comp["anom"]);
        cleanup(&dir);
    }

    /// The density lens catches records that are ON the manifold and IN-RANGE on
    /// every axis but sit in a locally SPARSE pocket — the LOF class that both the
    /// completion (off-manifold) and global (axis-wise) lenses are blind to. Two
    /// dense blobs with a pair of records marooned in the sparse middle.
    #[test]
    fn scan_density_catches_sparse_pocket() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        // two tight 5x5 blobs, one near (0,0), one near (10,10)
        for (ox, oy, tag) in [(0.0, 0.0, "a"), (10.0, 10.0, "b")] {
            for xi in 0..5 {
                for yi in 0..5 {
                    rows.push(scan_rec(&[
                        ("id", V::Text(format!("{tag}{xi}{yi}"))),
                        ("x", V::Float(ox + 0.3 * xi as f64)),
                        ("y", V::Float(oy + 0.3 * yi as f64)),
                    ]));
                }
            }
        }
        // two anomalies stranded in the sparse middle — x,y both mid-range (near the
        // global mean), so no single axis is extreme; only local density flags them.
        for (i, (ax, ay)) in [(5.5, 5.0), (5.0, 6.0)].iter().enumerate() {
            rows.push(scan_rec(&[
                ("id", V::Text(format!("mid{i}"))),
                ("x", V::Float(*ax)), ("y", V::Float(*ay)),
            ]));
        }
        let schema = BundleSchema::new("blobs")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x"))
            .fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("scan_density", "blobs", schema, rows);
        let sl = scan_compute_lenses(&engine, "blobs", &[]).unwrap();
        let dens = scan_lens(&sl, "density").expect("density lens should be built");
        let g = scan_lens(&sl, "global").unwrap();
        for a in ["mid0", "mid1"] {
            // the sparse-pocket records top the density lens...
            assert!(dens[a] >= 0.95, "sparse-pocket record {a} should top density lens, got {}", dens[a]);
            // ...but the axis-wise global lens (they sit near the mean) does not flag them
            assert!(g[a] < dens[a], "global should rank sparse record {a} below density (global={}, density={})", g[a], dens[a]);
        }
        cleanup(&dir);
    }
}
