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

/// Cap on interaction context lenses kept per scan (chosen by descending median
/// cohort size; every pair dropped for budget is named in `notes`).
const SCAN_MAX_INTERACTION_LENSES: usize = 4;
/// Robust-z clip for lens normalization: `norm = min(zraw, CAP)/CAP`. Below the
/// cap norm is LINEAR in robust-sigma — the discrimination band where rank
/// normalization used to flatten a noise lens's ~2-3σ top into a tie with a
/// ≥5σ true cohort deviation under max fusion.
const SCAN_NORM_Z_CAP: f64 = 6.0;

/// Normalize one lens's raw scores to [0,1] while PRESERVING magnitude.
///
/// Returns `(norm, zraw)`: `zraw_i = max(0, (x_i - center)/scale)` with
/// center = median and scale = 1.4826·MAD (falls back to (mean, sd) when the
/// MAD is degenerate; all-zeros when both scales are), and
/// `norm_i = min(zraw_i, SCAN_NORM_Z_CAP)/SCAN_NORM_Z_CAP`. `norm` keeps the
/// JSON lens-breakdown contract ([0,1]); `zraw` is the unclipped magnitude
/// channel the fusion tiebreak uses so records clipped at 1.0 stay strictly
/// ordered by how far out they actually are.
pub(crate) fn scan_normalize(vals: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = vals.len();
    if n == 0 {
        return (Vec::new(), Vec::new());
    }
    let med = |xs: &mut Vec<f64>| -> f64 {
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if xs.len() % 2 == 1 { xs[xs.len() / 2] } else { (xs[xs.len() / 2 - 1] + xs[xs.len() / 2]) / 2.0 }
    };
    let median = med(&mut vals.to_vec());
    let mad = med(&mut vals.iter().map(|x| (x - median).abs()).collect());
    let (center, scale) = if mad >= f64::EPSILON {
        (median, 1.4826 * mad)
    } else {
        let mu = vals.iter().sum::<f64>() / n as f64;
        let sd = (vals.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n as f64).sqrt();
        if sd >= f64::EPSILON { (mu, sd) } else { return (vec![0.0; n], vec![0.0; n]); }
    };
    let zraw: Vec<f64> = vals.iter().map(|x| ((x - center) / scale).max(0.0)).collect();
    // Strictly monotone squash z/(z+cap): bounded [0,1), calibrated in MAD
    // units, and INJECTIVE — a hard clip (min(z,cap)/cap) tied every value
    // above the cap at exactly 1.0, collapsing the ranking among the most
    // extreme records (measured: expert weighted-lens PR-AUC 0.70 -> 0.49 on
    // heavy-tailed amounts). The squash keeps cross-lens magnitude comparable
    // while never creating ties.
    let norm: Vec<f64> = zraw.iter().map(|z| z / (z + SCAN_NORM_Z_CAP)).collect();
    (norm, zraw)
}

/// Shared context-lens statistic: per-cohort per-field POPULATION z.
///
/// For each record in the cohort, `raw = max_f |x_f − μ_f| / σ_f` with μ/σ
/// cohort-local over `fields`. A zero-variance field (σ < EPSILON) contributes
/// 0 — the SQL arm's COALESCE convention. Cohorts smaller than 3 score 0.
/// This replaces z-of-κ: κ AVERAGES |v−μ|/range over all numeric fibers, so a
/// single-field deviation is diluted by the ordinary fields, and z-of-|dev| is
/// an affine map of the expert |z| only under per-cohort Gaussianity — max
/// over per-field |z| is calibrated in noise units across cohorts regardless
/// of distribution shape.
pub(crate) fn scan_cohort_field_z(
    records: &[crate::types::Record],
    idxs: &[usize],
    fields: &[crate::types::FieldDef],
) -> Vec<(usize, f64)> {
    if idxs.len() < 3 {
        return idxs.iter().map(|&ix| (ix, 0.0)).collect();
    }
    let stats: Vec<(&str, f64, f64)> = fields.iter().filter_map(|fd| {
        let xs: Vec<f64> = idxs.iter()
            .filter_map(|&ix| records[ix].get(&fd.name).and_then(|v| v.as_f64())).collect();
        if xs.len() < 2 { return None; }
        let cn = xs.len() as f64;
        let mu = xs.iter().sum::<f64>() / cn;
        let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / cn).sqrt();
        (sd >= f64::EPSILON).then_some((fd.name.as_str(), mu, sd))
    }).collect();
    idxs.iter().map(|&ix| {
        let raw = stats.iter().filter_map(|(f, mu, sd)| {
            records[ix].get(*f).and_then(|v| v.as_f64()).map(|x| (x - mu).abs() / sd)
        }).fold(0.0_f64, f64::max);
        (ix, raw)
    }).collect()
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
    /// Unclipped robust-z per lens (same keys as `norm`) — the magnitude
    /// channel. `norm` clips at 6 robust-sigma to keep its [0,1] contract;
    /// fusion adds a tiny `zraw` term so records clipped to 1.0 keep strict
    /// magnitude ordering.
    pub zraw: Vec<std::collections::HashMap<String, f64>>,
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
        return Ok(ScanLenses { lens_names: Vec::new(), norm: Vec::new(), zraw: Vec::new(), ids: Vec::new(), base, n: 0, notes });
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

    // Time-like numeric fields are picked by NAME (day/date/time/hour/…), not by
    // distinct-count — otherwise `amount` (thousands of distinct values) would
    // masquerade as time. Used by the interaction-lens bucket axes and velocity.
    let time_named = |nm: &str| {
        let l = nm.to_lowercase();
        ["day", "date", "time", "hour", "ts", "week", "month"].iter().any(|k| l.contains(k))
    };

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
            for (ix, raw) in scan_cohort_field_z(&records, idxs, &num_defs) {
                m.insert(ids[ix].clone(), raw);
            }
        }
        lenses.push((format!("context:{}", cf), m));
    }

    // ── interaction contextual curvature: (categorical × binned-numeric) and
    // (categorical × categorical) cohorts. A COMBINATION anomaly — every field
    // in-distribution on its own marginal, the combination impossible — is
    // invisible to every single-field cohort above; the crossed cohort is the
    // smallest space where it stands out. ──
    if !num_defs.is_empty() {
        // Binned axes from numeric fields: time-named → 12 equal-width bins over
        // the observed range; small-integer-valued → the rounded value itself.
        struct BucketAxis { axis: String, src: String, bins: Vec<Option<i64>>, desc: String }
        let mut axes: Vec<BucketAxis> = Vec::new();
        for fd in &num_defs {
            let xs: Vec<Option<f64>> = records.iter()
                .map(|r| r.get(&fd.name).and_then(|v| v.as_f64())).collect();
            if time_named(&fd.name) {
                let present: Vec<f64> = xs.iter().flatten().cloned().collect();
                if present.is_empty() { continue; }
                let (mn, mx) = present.iter()
                    .fold((f64::MAX, f64::MIN), |(a, b), &x| (a.min(x), b.max(x)));
                if mx - mn < f64::EPSILON { continue; }
                let w = (mx - mn) / 12.0;
                axes.push(BucketAxis {
                    axis: format!("{}_bucket", fd.name),
                    src: fd.name.clone(),
                    bins: xs.iter().map(|x| x.map(|x| (((x - mn) / w).floor() as i64).clamp(0, 11))).collect(),
                    desc: format!("12 equal-width {} bins [{mn:.1},{mx:.1}]", fd.name),
                });
            } else {
                let distinct: std::collections::HashSet<i64> =
                    xs.iter().flatten().map(|x| x.round() as i64).collect();
                if (2..=24).contains(&distinct.len()) {
                    axes.push(BucketAxis {
                        axis: format!("{}_bucket", fd.name),
                        src: fd.name.clone(),
                        bins: xs.iter().map(|x| x.map(|x| x.round() as i64)).collect(),
                        desc: format!("{} integer-valued buckets of {}", distinct.len(), fd.name),
                    });
                }
            }
        }
        // Pair sides: non-text categoricals with 2..=64 distinct values (a
        // higher-cardinality side cannot form cohorts once crossed — the
        // single-field guard fires at n/5; crossing only makes it worse).
        let mut sides: Vec<&String> = Vec::new();
        for cf in cats.iter().filter(|c| !text_fields.contains(*c)) {
            let d = records.iter().filter_map(|r| r.get(cf).map(|v| format!("{}", v)))
                .collect::<std::collections::HashSet<_>>().len();
            if d > 64 {
                notes.push(format!("context:{cf}* skipped: {d} distinct values, too unique to form cohorts (interaction side limit 64)"));
            } else if d >= 2 {
                sides.push(cf);
            }
        }
        // Candidates: (lens name, per-record composite key, excluded numeric, axis desc)
        let mut candidates: Vec<(String, Vec<Option<String>>, Option<String>, Option<String>)> = Vec::new();
        for cf in &sides {
            let cvals: Vec<Option<String>> = records.iter()
                .map(|r| r.get(cf.as_str()).map(|v| format!("{}", v))).collect();
            for ax in &axes {
                let keys = cvals.iter().zip(&ax.bins).map(|(c, b)| match (c, b) {
                    (Some(c), Some(b)) => Some(format!("{c}\u{1f}{b}")),
                    _ => None,
                }).collect();
                candidates.push((format!("context:{}*{}", cf, ax.axis), keys,
                    Some(ax.src.clone()), Some(ax.desc.clone())));
            }
        }
        for i in 0..sides.len() {
            for j in (i + 1)..sides.len() {
                let (a, b) = (sides[i], sides[j]);
                let keys = records.iter().map(|r| match (r.get(a.as_str()), r.get(b.as_str())) {
                    (Some(x), Some(y)) => Some(format!("{x}\u{1f}{y}")),
                    _ => None,
                }).collect();
                candidates.push((format!("context:{a}*{b}"), keys, None, None));
            }
        }
        // Guards (same thresholds + phrasing as the single-field lens), then a
        // hard budget: keep at most SCAN_MAX_INTERACTION_LENSES by descending
        // median cohort size. Per kept lens the cost is O(n·|num_defs|) — the
        // same as one single-field context lens.
        let mut kept: Vec<(String, HashMap<String, Vec<usize>>, Option<String>, usize, Option<String>)> = Vec::new();
        for (nm, keys, excl, desc) in candidates {
            let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
            for (ix, k) in keys.iter().enumerate() {
                if let Some(k) = k { groups.entry(k.clone()).or_default().push(ix); }
            }
            let d = groups.len();
            if d < 4 {
                notes.push(format!("{nm} skipped: only {d} distinct value(s) (need >= 4)"));
                continue;
            }
            if (d as f64) > (n as f64) / 5.0 {
                notes.push(format!("{nm} skipped: {d} distinct values, too unique to form cohorts"));
                continue;
            }
            let mut sizes: Vec<usize> = groups.values().map(|v| v.len()).collect();
            sizes.sort_unstable();
            let median = sizes[sizes.len() / 2];
            if median < 3 {
                notes.push(format!("{nm} skipped: cohorts too small (median < 3 records)"));
                continue;
            }
            if let Some(ex) = excl.as_deref() {
                // within a bucket, residual variation of the binned field must not
                // dilute the partner-field z — it is excluded from scoring
                if !num_defs.iter().any(|fd| fd.name != ex) {
                    notes.push(format!("{nm} skipped: no numeric field left to score once {ex} is the bucket axis"));
                    continue;
                }
            }
            kept.push((nm, groups, excl, median, desc));
        }
        kept.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));
        if kept.len() > SCAN_MAX_INTERACTION_LENSES {
            let dropped: Vec<String> = kept[SCAN_MAX_INTERACTION_LENSES..].iter()
                .map(|c| c.0.clone()).collect();
            notes.push(format!(
                "interaction lens budget: kept {SCAN_MAX_INTERACTION_LENSES} of {} candidate pairs by median cohort size; dropped {}",
                kept.len(), dropped.join(", ")));
            kept.truncate(SCAN_MAX_INTERACTION_LENSES);
        }
        for (nm, groups, excl, _, desc) in kept {
            let flds: Vec<crate::types::FieldDef> = num_defs.iter()
                .filter(|fd| excl.as_deref() != Some(fd.name.as_str())).cloned().collect();
            let mut m: HashMap<String, f64> = HashMap::new();
            for idxs in groups.values() {
                for (ix, raw) in scan_cohort_field_z(&records, idxs, &flds) {
                    m.insert(ids[ix].clone(), raw);
                }
            }
            if let Some(d) = desc { notes.push(format!("{nm}: {d}")); }
            lenses.push((nm, m));
        }
    }

    // ── velocity: (highest-cardinality entity × most-granular TIME) burst count ──
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

    // ── normalize each lens to [0,1]: clipped robust z (median/MAD, sd
    // fallback) + an unclipped magnitude channel for fusion tiebreaks. Rank
    // normalization destroyed cross-lens magnitude under max fusion: every
    // lens's top tail mapped to ~1.0, so a pure-noise lens's top tied an 8σ
    // cohort deviation and the lens-OR flooded the review budget with noise. ──
    let lens_names: Vec<String> = lenses.iter().map(|(nm, _)| nm.clone()).collect();
    let mut norm: Vec<HashMap<String, f64>> = Vec::with_capacity(lenses.len());
    let mut zraw: Vec<HashMap<String, f64>> = Vec::with_capacity(lenses.len());
    for (_, m) in &lenses {
        let vals: Vec<f64> = ids.iter().map(|i| *m.get(i).unwrap_or(&0.0)).collect();
        let (nv, zv) = scan_normalize(&vals);
        norm.push(ids.iter().cloned().zip(nv).collect());
        zraw.push(ids.iter().cloned().zip(zv).collect());
    }

    Ok(ScanLenses { lens_names, norm, zraw, ids, base, n, notes })
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
        assert!(g["t24"] >= 0.35, "outlier should carry real global magnitude (squash units), got {}", g["t24"]);
        for (id, v) in g {
            if id != "t24" { assert!(*v < g["t24"], "outlier must be strict argmax of global; {id} has {v}"); }
        }
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
        let gw = scan_lens(&with, "global").unwrap();
        let g_with = gw["r3"];
        let g_without = scan_lens(&without, "global").unwrap()["r3"];
        assert!(g_with > 0.0, "label should register on global when included, got {}", g_with);
        for (id, v) in gw {
            if id != "r3" { assert!(*v < g_with, "r3 must be strict argmax of global when label included; {id} has {v}"); }
        }
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
        // the off-manifold record tops the completion lens (strict argmax)
        assert!(comp["anom"] >= 0.4, "off-manifold record should carry real completion magnitude (squash units), got {}", comp["anom"]);
        for (id, v) in comp {
            if id != "anom" { assert!(*v < comp["anom"], "anom must be strict argmax of completion; {id} has {v}"); }
        }
        // ...but it does NOT top the axis-wise global curvature lens (grid corners
        // deviate more on individual axes), proving the two lenses see different things
        let g = scan_lens(&sl, "global").unwrap();
        assert!(g["anom"] < comp["anom"],
            "global curvature should rank the off-manifold point below completion (global={}, completion={})",
            g["anom"], comp["anom"]);
        cleanup(&dir);
    }

    /// Deterministic LCG uniform in [-half, half] — noise for cohort fixtures.
    fn lcg_noise(seed: &mut u64, half: f64) -> f64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let u = ((*seed >> 11) as f64) / ((1u64 << 53) as f64);
        (u - 0.5) * 2.0 * half
    }

    /// RED→GREEN for the interaction lens: a COMBINATION anomaly — amount ordinary
    /// for its merchant marginal AND for its hour marginal AND globally, but far
    /// off its (merchant × 2h-bucket) cohort — must top the crossed-cohort lens,
    /// while the single-field context:merchant lens stays blind to it (the
    /// merchant-marginal amount distribution absorbs the hour modulation).
    #[test]
    fn scan_interaction_lens_catches_cohort_amount_plant() {
        let mut seed: u64 = 20260731;
        let mut rows: Vec<crate::types::Record> = Vec::new();
        // 4 merchants × 24 hours × 15 reps; amount = merchant base + hour-bucket
        // modulation (+50 on odd 12-bins) + uniform noise ±3.
        for m in 0..4u32 {
            for h in 0..24u32 {
                let b = ((h as f64) * 12.0 / 23.0).floor().min(11.0) as u32; // the 12-bin bucket the lens will build
                for rep in 0..15u32 {
                    rows.push(scan_rec(&[
                        ("id", V::Text(format!("r{m}_{h}_{rep}"))),
                        ("merchant", V::Text(format!("M{m}"))),
                        ("hour", V::Float(h as f64)),
                        ("amount", V::Float(100.0 * (m + 1) as f64 + 50.0 * (b % 2) as f64 + lcg_noise(&mut seed, 3.0))),
                    ]));
                }
            }
        }
        // Plant: merchant M0 at hour 2 (odd bucket → cohort amounts ≈150) with
        // amount 100 — ordinary for M0's marginal {100,150}, ordinary globally,
        // ~29σ off its (M0, bucket-of-hour-2) cohort.
        rows.push(scan_rec(&[
            ("id", V::Text("plant".into())),
            ("merchant", V::Text("M0".into())),
            ("hour", V::Float(2.0)),
            ("amount", V::Float(100.0)),
        ]));
        let schema = BundleSchema::new("tx")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("merchant"))
            .fiber(FieldDef::numeric("hour"))
            .fiber(FieldDef::numeric("amount"));
        let (dir, engine) = scan_env("scan_interaction", "tx", schema, rows);
        let sl = scan_compute_lenses(&engine, "tx", &[]).unwrap();
        let il = scan_lens(&sl, "context:merchant*hour_bucket")
            .expect("interaction lens context:merchant*hour_bucket should be built");
        // the plant tops the interaction lens…
        assert!(il["plant"] >= 0.5, "combination anomaly should top the interaction lens (squash units), got {}", il["plant"]);
        for (id, v) in il {
            if id != "plant" { assert!(*v < il["plant"], "plant must be strict argmax; {id} has {v}"); }
        }
        // …while the single-field merchant lens is blind to it (blindness proof)
        let sm = scan_lens(&sl, "context:merchant").expect("single-field merchant lens");
        assert!(sm["plant"] < il["plant"],
            "context:merchant should be blind to the combination anomaly (merchant={}, interaction={})",
            sm["plant"], il["plant"]);
        cleanup(&dir);
    }

    /// Interaction-lens guards: (a) a high-cardinality side is skipped with a
    /// "too unique" note and builds no lens; (b) the lens budget keeps exactly
    /// SCAN_MAX_INTERACTION_LENSES pairs and names the dropped ones in a note.
    #[test]
    fn scan_interaction_lens_guards() {
        // (a) 300-distinct customer side → side-limit note, no customer lens
        let mut seed: u64 = 7;
        let rows: Vec<_> = (0..300).map(|i| scan_rec(&[
            ("id", V::Text(format!("t{i}"))),
            ("customer", V::Text(format!("C{i}"))),
            ("hour", V::Float((i % 24) as f64)),
            ("amount", V::Float(100.0 + lcg_noise(&mut seed, 3.0))),
        ])).collect();
        let schema = BundleSchema::new("hc")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("customer"))
            .fiber(FieldDef::numeric("hour"))
            .fiber(FieldDef::numeric("amount"));
        let (dir, engine) = scan_env("scan_iguard_a", "hc", schema, rows);
        let sl = scan_compute_lenses(&engine, "hc", &[]).unwrap();
        assert!(!sl.lens_names.iter().any(|l| l.starts_with("context:customer*")),
            "300-distinct customer must not form an interaction side: {:?}", sl.lens_names);
        assert!(sl.notes.iter().any(|nt| nt.contains("context:customer*") && nt.contains("too unique")),
            "expected a side-limit note, got {:?}", sl.notes);
        cleanup(&dir);

        // (b) 6 low-card cats × 1 time axis → many candidates, exactly 4 kept + budget note
        let mut seed: u64 = 11;
        let rows: Vec<_> = (0..400).map(|i| {
            let mut fields: Vec<(&str, V)> = vec![("id", V::Text(format!("t{i}")))];
            let names = ["c0", "c1", "c2", "c3", "c4", "c5"];
            for (j, nm) in names.iter().enumerate() {
                fields.push((nm, V::Text(format!("v{}", (i + j) % 4))));
            }
            fields.push(("hour", V::Float((i % 24) as f64)));
            fields.push(("amount", V::Float(100.0 + lcg_noise(&mut seed, 3.0))));
            scan_rec(&fields)
        }).collect();
        let mut schema = BundleSchema::new("bud").base(FieldDef::categorical("id"));
        for nm in ["c0", "c1", "c2", "c3", "c4", "c5"] { schema = schema.fiber(FieldDef::categorical(nm)); }
        schema = schema.fiber(FieldDef::numeric("hour")).fiber(FieldDef::numeric("amount"));
        let (dir, engine) = scan_env("scan_iguard_b", "bud", schema, rows);
        let sl = scan_compute_lenses(&engine, "bud", &[]).unwrap();
        let inter: Vec<&String> = sl.lens_names.iter().filter(|l| l.contains('*')).collect();
        assert_eq!(inter.len(), 4, "budget must keep exactly 4 interaction lenses, got {inter:?}");
        assert!(sl.notes.iter().any(|nt| nt.contains("interaction lens budget") && nt.contains("dropped")),
            "expected a budget note naming dropped pairs, got {:?}", sl.notes);
        cleanup(&dir);
    }

    /// The binned partner field is EXCLUDED from interaction-cohort scoring:
    /// residual within-bin variation of the binned field must not score. A
    /// pure-hour deviation inside a bin stays low; a same-size amount plant
    /// tops the lens.
    #[test]
    fn scan_interaction_excludes_partner_numeric() {
        let mut seed: u64 = 20260731;
        let mut rows: Vec<crate::types::Record> = Vec::new();
        // 4 merchants × hours 0..11 (each 12-bin holds exactly one integer hour)
        for m in 0..4u32 {
            for h in 0..12u32 {
                for rep in 0..30u32 {
                    rows.push(scan_rec(&[
                        ("id", V::Text(format!("r{m}_{h}_{rep}"))),
                        ("merchant", V::Text(format!("M{m}"))),
                        ("hour", V::Float(h as f64)),
                        ("amount", V::Float(100.0 + lcg_noise(&mut seed, 3.0))),
                    ]));
                }
            }
        }
        // hp: hour 10.55 lands in the hour-11 bin — a large hour deviation
        // INSIDE the bin, amount ordinary. Must NOT top the interaction lens.
        rows.push(scan_rec(&[
            ("id", V::Text("hp".into())),
            ("merchant", V::Text("M0".into())),
            ("hour", V::Float(10.55)),
            ("amount", V::Float(100.0)),
        ]));
        // ap: amount 40 off its cohort — the real combination anomaly.
        rows.push(scan_rec(&[
            ("id", V::Text("ap".into())),
            ("merchant", V::Text("M1".into())),
            ("hour", V::Float(5.0)),
            ("amount", V::Float(140.0)),
        ]));
        let schema = BundleSchema::new("tx")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("merchant"))
            .fiber(FieldDef::numeric("hour"))
            .fiber(FieldDef::numeric("amount"));
        let (dir, engine) = scan_env("scan_ipartner", "tx", schema, rows);
        let sl = scan_compute_lenses(&engine, "tx", &[]).unwrap();
        let il = scan_lens(&sl, "context:merchant*hour_bucket")
            .expect("interaction lens should be built");
        for (id, v) in il {
            if id != "ap" { assert!(*v < il["ap"], "amount plant must be strict argmax; {id} has {v}"); }
        }
        assert!(il["hp"] < 0.5,
            "pure-hour deviation inside a bin must not score on the interaction lens (hour is the bucket axis), got {}", il["hp"]);
        cleanup(&dir);
    }

    /// scan_normalize: monotone, [0,1] range, degenerate handling, sd fallback,
    /// LINEAR magnitude below the cap, clip above it, zraw unclipped.
    #[test]
    fn scan_normalize_unit() {
        // median 0, MAD 1 → scale = 1.4826
        let vals = vec![0.0, 0.0, 0.0, 0.0, 1.0, -1.0, 2.0, -2.0, 4.0, 8.0, 20.0];
        let (norm, zraw) = scan_normalize(&vals);
        assert_eq!(norm.len(), vals.len());
        for v in &norm { assert!((0.0..=1.0).contains(v), "norm in [0,1], got {v}"); }
        let at = |x: f64| vals.iter().position(|&v| v == x).unwrap();
        // strictly monotone everywhere — INCLUDING far above the cap. The old
        // hard clip tied 20 and 8 at 1.0; the squash must order them.
        assert!(norm[at(20.0)] > norm[at(8.0)] && norm[at(8.0)] > norm[at(4.0)] && norm[at(4.0)] > norm[at(1.0)],
            "squash must be injective above the cap: n20={} n8={}", norm[at(20.0)], norm[at(8.0)]);
        // squash form: norm = z/(z+cap)
        let z8 = zraw[at(8.0)];
        assert!((norm[at(8.0)] - z8 / (z8 + SCAN_NORM_Z_CAP)).abs() < 1e-12, "norm = z/(z+cap)");
        // bounded strictly below 1
        assert!(norm[at(20.0)] < 1.0, "squash never saturates to exactly 1.0");
        assert!(zraw[at(20.0)] > SCAN_NORM_Z_CAP, "zraw stays unclipped, got {}", zraw[at(20.0)]);
        assert!((zraw[at(20.0)] - 20.0 / 1.4826).abs() < 1e-9, "zraw = (x - median)/(1.4826*MAD)");
        // negatives floor at 0
        assert_eq!(norm[at(-2.0)], 0.0);
        // all-equal input → all zeros
        let (n0, z0) = scan_normalize(&[7.0; 9]);
        assert!(n0.iter().chain(z0.iter()).all(|v| *v == 0.0), "degenerate input scores 0");
        // MAD=0 with sd>0 → (mean, sd) fallback
        let mut v = vec![0.0; 9];
        v.push(3.0);
        let (nf, zf) = scan_normalize(&v);
        // mean 0.3, population sd 0.9 → z(3.0) = 3.0 → squash 3/(3+6) = 1/3
        assert!((zf[9] - 3.0).abs() < 1e-9, "sd-fallback zraw, got {}", zf[9]);
        assert!((nf[9] - 1.0/3.0).abs() < 1e-9, "sd-fallback norm, got {}", nf[9]);
        assert_eq!(nf[0], 0.0, "below-center floors at 0");
        // empty input
        let (ne, ze) = scan_normalize(&[]);
        assert!(ne.is_empty() && ze.is_empty());
    }

    /// Cross-cohort magnitude is PRESERVED: plants at ~4σ and ~12σ in cohorts
    /// with different sigma order by true magnitude, and the 4σ plant scores
    /// materially below 1.0 — impossible under rank normalization, which put
    /// them at adjacent ranks ≈ 1.0.
    #[test]
    fn scan_context_z_preserves_magnitude() {
        let mut seed: u64 = 20260731;
        let mut rows: Vec<crate::types::Record> = Vec::new();
        // 4 groups × 300 records, per-group sigma differs (uniform half-widths)
        let half = [2.0, 20.0, 5.0, 8.0];
        let base = [1000.0, 500.0, 100.0, 2000.0];
        for g in 0..4usize {
            for i in 0..300 {
                rows.push(scan_rec(&[
                    ("id", V::Text(format!("g{g}_{i}"))),
                    ("grp", V::Text(format!("G{g}"))),
                    ("amount", V::Float(base[g] + lcg_noise(&mut seed, half[g]))),
                ]));
            }
        }
        // uniform half-width h → sigma = h/sqrt(3)
        let s1 = 20.0 / 3.0_f64.sqrt();  // G1 sigma ≈ 11.55
        let s0 = 2.0 / 3.0_f64.sqrt();   // G0 sigma ≈ 1.155
        rows.push(scan_rec(&[  // ~4σ plant in the WIDE cohort
            ("id", V::Text("p4".into())), ("grp", V::Text("G1".into())),
            ("amount", V::Float(500.0 + 4.0 * s1)),
        ]));
        rows.push(scan_rec(&[  // ~12σ plant in the TIGHT cohort
            ("id", V::Text("p12".into())), ("grp", V::Text("G0".into())),
            ("amount", V::Float(1000.0 + 12.0 * s0)),
        ]));
        let schema = BundleSchema::new("mag")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("grp"))
            .fiber(FieldDef::numeric("amount"));
        let (dir, engine) = scan_env("scan_mag", "mag", schema, rows);
        let sl = scan_compute_lenses(&engine, "mag", &[]).unwrap();
        let c = scan_lens(&sl, "context:grp").expect("context:grp lens");
        assert!(c["p12"] >= 0.65, "12σ plant lands deep in the squash tail, got {}", c["p12"]);
        assert!(c["p4"] < c["p12"], "true magnitude must order the plants ({} vs {})", c["p4"], c["p12"]);
        assert!(c["p4"] <= 0.95, "4σ plant must stay materially below 1.0 (rank-norm would tie it), got {}", c["p4"]);
        assert!(c["p4"] >= 0.3, "4σ plant still carries real magnitude, got {}", c["p4"]);
        cleanup(&dir);
    }

    /// No-regression pin on a NON-interaction fixture (no categoricals → no
    /// context/interaction lenses can build): the global and density lenses
    /// keep their behavior under the new normalization.
    #[test]
    fn scan_non_interaction_fixture_pins_global_density() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
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
        // sparse-middle record (density) + a global extreme (global)
        rows.push(scan_rec(&[("id", V::Text("mid".into())), ("x", V::Float(5.5)), ("y", V::Float(5.0))]));
        rows.push(scan_rec(&[("id", V::Text("ext".into())), ("x", V::Float(40.0)), ("y", V::Float(40.0))]));
        let schema = BundleSchema::new("pin")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x"))
            .fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("scan_pin", "pin", schema, rows);
        let sl = scan_compute_lenses(&engine, "pin", &[]).unwrap();
        assert!(!sl.lens_names.iter().any(|l| l.contains('*')),
            "no interaction lens can build without categoricals: {:?}", sl.lens_names);
        let g = scan_lens(&sl, "global").unwrap();
        for (id, v) in g {
            if id != "ext" { assert!(*v < g["ext"], "global extreme must be strict argmax of global; {id} has {v}"); }
        }
        let d = scan_lens(&sl, "density").expect("density lens");
        for (id, v) in d {
            if id != "mid" && id != "ext" {
                assert!(*v < d["mid"], "sparse-middle record must outrank every blob record on density; {id} has {v}");
            }
        }
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
