//! INFER — supervised prediction (local-linear / kNN / OLS / GP / diffusion; kNN-vote / SVM / label-propagation).
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;
use super::scan::{default_fit_folds, scan_solve};

/// Request for `POST /v1/bundles/{name}/infer`.
#[derive(Deserialize)]
pub struct SupervisedPredictRequest {
    /// Field to predict. Numeric → regression, categorical → classification.
    pub target: String,
    /// Regression: "local_linear" (default), "knn", or "ols".
    /// Classification: always distance-weighted k-NN vote.
    #[serde(default = "default_predict_method")]
    pub method: String,
    /// Neighbors for the local methods. Default 20.
    #[serde(default = "default_predict_k")]
    pub k: usize,
    /// local_linear slope-shrinkage ridge. Higher → smoother (toward kNN-mean);
    /// lower → more local slope. Default 0.5.
    #[serde(default = "default_predict_ridge")]
    pub ridge: f64,
    /// Cross-validation folds for the held-out score. Default 5.
    #[serde(default = "default_fit_folds")]
    pub folds: usize,
    /// Extra fields to exclude from the feature set.
    #[serde(default)]
    pub exclude: Vec<String>,
}
pub fn default_predict_method() -> String { "local_linear".to_string() }
pub fn default_predict_k() -> usize { 20 }
pub fn default_predict_ridge() -> f64 { 0.5 }

/// Result of supervised prediction — shared by the handler and tests.
#[derive(Debug)]
pub struct PredictResult {
    pub base: String,
    pub task: String,           // "regression" | "classification"
    pub method: String,
    pub metric: serde_json::Value,     // {rmse,r2} or {accuracy}
    pub baseline: serde_json::Value,   // flat kNN-mean / majority-class, for comparison
    pub n_train: usize,
    pub predictions: Vec<(String, serde_json::Value)>,   // for records with a missing target
    pub notes: Vec<String>,
}

/// Weighted least-squares intercept at the query point: fit y ≈ β₀ + β·(x − x_q)
/// over the (distance, index) neighbors with tricube weights; return β₀ (the value
/// at x_q). A ridge penalty shrinks the SLOPE terms (not the intercept) toward 0,
/// so when the local neighborhood is too sparse to support a linear fit (high dim /
/// few neighbors) the estimate degrades gracefully to the weighted mean instead of
/// extrapolating wildly. Falls back to the weighted mean if the system is singular.
pub fn local_linear_at(xq: &[f64], nbrs: &[(f64, usize)], x: &[Vec<f64>], y: &[f64], dim: usize, ridge_frac: f64) -> f64 {
    let dmax = nbrs.last().map(|(d, _)| *d).unwrap_or(1.0).max(1e-9);
    let m = dim + 1;
    let mut a = vec![vec![0.0f64; m]; m];
    let mut b = vec![0.0f64; m];
    let mut wsum = 0.0;
    let mut wy = 0.0;
    for &(d, j) in nbrs {
        let u = d / dmax;
        let w = if u < 1.0 { (1.0 - u.powi(3)).powi(3) } else { 0.0 };
        wsum += w; wy += w * y[j];
        let mut basis = vec![1.0];
        for t in 0..dim { basis.push(x[j][t] - xq[t]); }
        for r in 0..m { for c in 0..m { a[r][c] += w * basis[r] * basis[c]; } b[r] += w * basis[r] * y[j]; }
    }
    let ridge = ridge_frac.max(0.0) * a[0][0];   // a[0][0] = Σ w (intercept diagonal)
    for t in 1..m { a[t][t] += ridge; }
    match scan_solve(&mut a, &b) {
        Some(beta) if beta[0].is_finite() => beta[0],
        _ => if wsum > 0.0 { wy / wsum } else { 0.0 },
    }
}

/// Local-linear prediction plus a difficulty scale s(x): the tricube-weighted RMS
/// residual of the neighbors around the local fit. Larger s ⇒ the target is noisier
/// / less linear locally ⇒ a wider predictive interval (the `gp` method scales
/// conformal intervals by s). Returns (mean, s).
pub fn local_linear_scaled(xq: &[f64], nbrs: &[(f64, usize)], x: &[Vec<f64>], y: &[f64], dim: usize, ridge_frac: f64) -> (f64, f64) {
    let dmax = nbrs.last().map(|(d, _)| *d).unwrap_or(1.0).max(1e-9);
    let m = dim + 1;
    let mut a = vec![vec![0.0f64; m]; m];
    let mut b = vec![0.0f64; m];
    let mut ws: Vec<(f64, usize)> = Vec::with_capacity(nbrs.len());
    let (mut wsum, mut wy) = (0.0, 0.0);
    for &(d, j) in nbrs {
        let u = d / dmax;
        let w = if u < 1.0 { (1.0 - u.powi(3)).powi(3) } else { 0.0 };
        ws.push((w, j)); wsum += w; wy += w * y[j];
        let mut basis = vec![1.0];
        for t in 0..dim { basis.push(x[j][t] - xq[t]); }
        for r in 0..m { for c in 0..m { a[r][c] += w * basis[r] * basis[c]; } b[r] += w * basis[r] * y[j]; }
    }
    let ridge = ridge_frac.max(0.0) * a[0][0];
    for t in 1..m { a[t][t] += ridge; }
    let beta = scan_solve(&mut a, &b).filter(|bt| bt[0].is_finite());
    let mean = match &beta { Some(bt) => bt[0], None => if wsum > 0.0 { wy / wsum } else { 0.0 } };
    // weighted RMS residual of neighbors around the fit
    let mut num = 0.0;
    for &(w, j) in &ws {
        let pred_j = match &beta {
            Some(bt) => bt[0] + (0..dim).map(|t| bt[t + 1] * (x[j][t] - xq[t])).sum::<f64>(),
            None => mean,
        };
        num += w * (y[j] - pred_j).powi(2);
    }
    let s = if wsum > 0.0 { (num / wsum).sqrt() } else { 0.0 };
    (mean, s + 1e-6)
}

/// Build a symmetric Gaussian-weighted k-NN graph over standardized points for
/// diffusion: edge weight exp(−‖xᵢ−xⱼ‖² / σ²) with σ² the median k-th-neighbor
/// squared distance. Returns per-node (neighbor, weight) lists.
pub fn build_diffusion_graph(x: &[Vec<f64>], k: usize, n: usize,
    dist2: &dyn Fn(&[f64], &[f64]) -> f64) -> Vec<Vec<(usize, f64)>> {
    let mut kth: Vec<f64> = (0..n).map(|i| {
        let mut d: Vec<f64> = (0..n).filter(|&j| j != i).map(|j| dist2(&x[i], &x[j])).collect();
        if d.is_empty() { return 1.0; }
        let idx = (k - 1).min(d.len() - 1);
        d.select_nth_unstable_by(idx, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        d[idx]
    }).collect();
    kth.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let sigma2 = kth[kth.len() / 2].max(1e-9);
    let mut seen: Vec<std::collections::HashMap<usize, f64>> = vec![std::collections::HashMap::new(); n];
    for i in 0..n {
        let mut d: Vec<(f64, usize)> = (0..n).filter(|&j| j != i).map(|j| (dist2(&x[i], &x[j]), j)).collect();
        let kc = k.min(d.len());
        if kc == 0 { continue; }
        d.select_nth_unstable_by(kc - 1, |a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for &(dd, j) in &d[..kc] {
            let w = (-dd / sigma2).exp();
            let e = seen[i].entry(j).or_insert(0.0); if w > *e { *e = w; }   // symmetrize (keep max)
            let e2 = seen[j].entry(i).or_insert(0.0); if w > *e2 { *e2 = w; }
        }
    }
    seen.into_iter().map(|m| m.into_iter().collect()).collect()
}

/// Harmonic label-propagation (diffusion) on a weighted graph: clamp the labeled
/// nodes and iterate each unlabeled node to the weighted average of its neighbors.
/// This is the discrete heat flow / Dirichlet-energy minimizer — the target diffuses
/// across the manifold instead of being averaged in flat feature space.
///
/// The method is the classical harmonic-function / Gaussian-field predictor
/// (Zhu–Ghahramani–Lafferty 2003), kin to Laplacian Eigenmaps (Belkin–Niyogi) and
/// diffusion maps (Coifman–Lafon) — GIGI does not claim to invent it. What is
/// GIGI-native is that the operator is *already the substrate*: this graph Laplacian
/// is the same one whose spectral gap is the mass gap, and `COMPLETE` is already its
/// Schur-complement harmonic fill — so geometric prediction is the bundle's own verb,
/// not a bolt-on. The flat methods are the baseline it is measured against.
pub fn diffuse(wadj: &[Vec<(usize, f64)>], clamp: &[Option<f64>], iters: usize) -> Vec<f64> {
    let n = clamp.len();
    let (sum, cnt) = clamp.iter().filter_map(|c| *c).fold((0.0, 0usize), |(s, c), v| (s + v, c + 1));
    let init = if cnt > 0 { sum / cnt as f64 } else { 0.0 };
    let mut f: Vec<f64> = clamp.iter().map(|c| c.unwrap_or(init)).collect();
    for _ in 0..iters {
        let mut nf = f.clone();
        for i in 0..n {
            if clamp[i].is_some() { continue; }   // labeled nodes stay clamped
            let (mut num, mut den) = (0.0, 0.0);
            for &(j, w) in &wadj[i] { num += w * f[j]; den += w; }
            if den > 1e-12 { nf[i] = num / den; }
        }
        f = nf;
    }
    f
}

/// POST /v1/bundles/{name}/infer
///
/// Supervised prediction over the numeric fibers. Numeric target → regression
/// (`local_linear` locally-weighted fit — curvature-aware — plus `knn` mean and
/// global `ols`); categorical target → distance-weighted k-NN classification.
/// Reports a held-out k-fold score against a flat baseline, and fills the target
/// for any records where it is missing.
pub fn predict_field(
    engine: &Engine,
    name: &str,
    target: &str,
    method: &str,
    k: usize,
    ridge: f64,
    folds: usize,
    exclude: &[String],
) -> Result<PredictResult, (StatusCode, String)> {
    use crate::types::FieldType;
    const PREDICT_MAX_N: usize = 8000;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "predict requires a single base-key field".into()));
    }
    let base = schema.base_fields[0].name.clone();
    let tf = schema.fiber_fields.iter().find(|f| f.name == target).ok_or_else(|| (
        StatusCode::UNPROCESSABLE_ENTITY, format!("target field '{}' not found among the bundle's fibers", target)))?;
    let is_reg = matches!(tf.field_type, FieldType::Numeric);
    if is_reg && !matches!(method, "local_linear" | "knn" | "ols" | "gp" | "diffusion") {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "unknown regression method '{}' (expected 'local_linear', 'knn', 'ols', 'gp', or 'diffusion')", method)));
    }
    // features: numeric fibers other than the target, honoring exclude
    let feat_defs: Vec<crate::types::FieldDef> = schema.fiber_fields.iter()
        .filter(|f| matches!(f.field_type, FieldType::Numeric) && f.name != target
            && !exclude.iter().any(|e| e == &f.name))
        .cloned().collect();
    let records: Vec<crate::types::Record> = store.records().collect();
    let n = records.len();
    if n > PREDICT_MAX_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "predict: {n} records exceeds the exact-neighbor limit ({PREDICT_MAX_N}) in this version")));
    }
    let cols: Vec<(String, f64, f64)> = feat_defs.iter().filter_map(|fd| {
        let xs: Vec<f64> = records.iter().map(|r| r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let mu = xs.iter().sum::<f64>() / n.max(1) as f64;
        let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n.max(1) as f64).sqrt();
        (sd > f64::EPSILON).then_some((fd.name.clone(), mu, sd))
    }).collect();
    let dim = cols.len();
    if dim < 1 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
            "predict needs >= 1 numeric feature fiber (besides the target) with non-zero variance".into()));
    }
    let base_of = |r: &crate::types::Record| r.get(&base).map(|v| format!("{}", v)).unwrap_or_default();
    let x: Vec<Vec<f64>> = records.iter().map(|r| cols.iter()
        .map(|(f, mu, sd)| (r.get(f).and_then(|v| v.as_f64()).unwrap_or(*mu) - mu) / sd).collect()).collect();
    let dist2 = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| (p - q) * (p - q)).sum::<f64>();
    let mut notes: Vec<String> = Vec::new();
    // split labeled (target present) vs query (target missing/unparseable)
    let has_target = |r: &crate::types::Record| -> bool {
        match r.get(target) {
            None | Some(crate::types::Value::Null) => false,
            Some(v) => if is_reg { v.as_f64().is_some() } else { !format!("{}", v).is_empty() },
        }
    };
    let train: Vec<usize> = (0..n).filter(|&i| has_target(&records[i])).collect();
    let query: Vec<usize> = (0..n).filter(|&i| !has_target(&records[i])).collect();
    if train.len() < 2 * k.min(5).max(2) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "predict needs more labeled records for target '{}' (found {})", target, train.len())));
    }
    let kk = k.clamp(1, train.len() - 1);
    // k nearest TRAIN neighbors of i (optionally excluding a held-out set)
    let knn_of = |i: usize, pool: &[usize]| -> Vec<(f64, usize)> {
        let mut d: Vec<(f64, usize)> = pool.iter().filter(|&&j| j != i)
            .map(|&j| (dist2(&x[i], &x[j]), j)).collect();
        let kc = kk.min(d.len());
        if kc == 0 { return Vec::new(); }
        d.select_nth_unstable_by(kc - 1, |a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        let mut top: Vec<(f64, usize)> = d[..kc].to_vec();
        top.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        top.iter().map(|(dd, j)| (dd.sqrt(), *j)).collect()
    };

    let (task, metric, baseline, predictions) = if is_reg && method == "gp" {
        // ── Gaussian-process answer: local-linear MEAN + conformalized adaptive
        // uncertainty. Interval half-width = Q · s(x), where s(x) is the local
        // difficulty scale and Q is the conformal quantile — coverage is guaranteed
        // (exchangeability), width adapts to local noise. ──
        let y: Vec<f64> = records.iter().map(|r| r.get(target).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let ybar = train.iter().map(|&i| y[i]).sum::<f64>() / train.len() as f64;
        let tss: f64 = train.iter().map(|&i| (y[i] - ybar).powi(2)).sum();
        let fld = folds.max(2);
        // held-out mean + difficulty scale per labeled record
        let mut hp: std::collections::HashMap<usize, (f64, f64)> = std::collections::HashMap::new();
        for f in 0..fld {
            let te: std::collections::HashSet<usize> = train.iter().enumerate()
                .filter(|(ix, _)| ix % fld == f).map(|(_, &i)| i).collect();
            let pool: Vec<usize> = train.iter().copied().filter(|i| !te.contains(i)).collect();
            if pool.is_empty() { continue; }
            for &q in &te {
                let nb = knn_of(q, &pool);
                let (m, s) = if nb.is_empty() { (ybar, (tss / train.len() as f64).sqrt()) }
                    else { local_linear_scaled(&x[q], &nb, &x, &y, dim, ridge) };
                hp.insert(q, (m, s));
            }
        }
        let se: f64 = train.iter().map(|&i| { let (m, _) = hp[&i]; (m - y[i]).powi(2) }).sum();
        let rmse = (se / train.len() as f64).sqrt();
        let r2 = 1.0 - se / tss.max(f64::EPSILON);
        // conformal calibration split: 2/3 to set Q, 1/3 to measure honest coverage
        let (mut cal, mut val) = (Vec::new(), Vec::new());
        for (pos, &i) in train.iter().enumerate() {
            let (m, s) = hp[&i];
            let rho = (y[i] - m).abs() / s;
            if pos % 3 == 0 { val.push((rho, s, (y[i] - m).abs())); } else { cal.push(rho); }
        }
        // split-conformal quantile with the finite-sample correction: the
        // ⌈(m+1)(1−α)⌉-th smallest score guarantees ≥ 1−α coverage on exchangeable data
        let conf_q = |mut v: Vec<f64>| -> f64 {
            if v.is_empty() { return 0.0; }
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let m = v.len();
            let rank = (((m + 1) as f64) * 0.90).ceil() as usize;
            if rank > m { f64::INFINITY } else { v[rank - 1] }
        };
        let q90 = conf_q(cal);
        let nval = val.len().max(1) as f64;
        let coverage = val.iter().filter(|(rho, _, _)| *rho <= q90).count() as f64 / nval;
        let width = val.iter().map(|(_, s, _)| 2.0 * q90 * s).sum::<f64>() / nval;
        // non-adaptive normal-theory baseline (fixed ±1.645·rmse) on the same split
        let base_cov = val.iter().filter(|(_, _, ar)| *ar <= 1.645 * rmse).count() as f64 / nval;
        // deploy quantile from ALL labeled residuals
        let q_final = conf_q(train.iter().map(|&i| { let (m, s) = hp[&i]; (y[i] - m).abs() / s }).collect());
        let rnd = |v: f64| (v * 1000.0).round() / 1000.0;
        let preds: Vec<(String, serde_json::Value)> = query.iter().map(|&q| {
            let nb = knn_of(q, &train);
            let (m, s) = if nb.is_empty() { (ybar, rmse) } else { local_linear_scaled(&x[q], &nb, &x, &y, dim, ridge) };
            let half = q_final * s;
            (base_of(&records[q]), serde_json::json!({
                "mean": rnd(m), "std": rnd(half / 1.645), "lower": rnd(m - half), "upper": rnd(m + half) }))
        }).collect();
        notes.push(format!("GP: local-linear mean + conformal adaptive intervals (difficulty s(x) × conformal quantile), {}-fold held-out", fld));
        ("regression".to_string(),
         serde_json::json!({"r2": (r2*10000.0).round()/10000.0, "coverage_90": (coverage*1000.0).round()/1000.0,
                            "mean_interval_width": (width*100.0).round()/100.0, "rmse": (rmse*100.0).round()/100.0}),
         serde_json::json!({"method": "fixed_normal_interval", "coverage_90": (base_cov*1000.0).round()/1000.0}),
         preds)
    } else if is_reg && method == "diffusion" {
        // ── GEOMETRIC predictor: diffuse the target across the manifold graph
        // (harmonic extension on the Laplacian) instead of averaging in flat space. ──
        let y: Vec<f64> = records.iter().map(|r| r.get(target).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let ybar = train.iter().map(|&i| y[i]).sum::<f64>() / train.len() as f64;
        let tss: f64 = train.iter().map(|&i| (y[i] - ybar).powi(2)).sum();
        let wadj = build_diffusion_graph(&x, kk, n, &dist2);
        let train_set: std::collections::HashSet<usize> = train.iter().copied().collect();
        let fld = folds.max(2);
        // held-out R² + flat-kNN baseline (same neighbors)
        let (mut se, mut se_base) = (0.0, 0.0);
        for f in 0..fld {
            let te: std::collections::HashSet<usize> = train.iter().enumerate()
                .filter(|(ix, _)| ix % fld == f).map(|(_, &i)| i).collect();
            let clamp: Vec<Option<f64>> = (0..n).map(|i| if train_set.contains(&i) && !te.contains(&i) { Some(y[i]) } else { None }).collect();
            let f_diff = diffuse(&wadj, &clamp, 80);
            let pool: Vec<usize> = train.iter().copied().filter(|i| !te.contains(i)).collect();
            for &q in &te {
                se += (f_diff[q] - y[q]).powi(2);
                let nb = knn_of(q, &pool);
                let base = if nb.is_empty() { ybar } else { nb.iter().map(|(_, j)| y[*j]).sum::<f64>() / nb.len() as f64 };
                se_base += (base - y[q]).powi(2);
            }
        }
        let nt = train.len() as f64;
        let r2 = 1.0 - se / tss.max(f64::EPSILON);
        let r2_base = 1.0 - se_base / tss.max(f64::EPSILON);
        // fill missing targets by diffusing from all labeled
        let clamp: Vec<Option<f64>> = (0..n).map(|i| if train_set.contains(&i) { Some(y[i]) } else { None }).collect();
        let f_all = diffuse(&wadj, &clamp, 100);
        let preds: Vec<(String, serde_json::Value)> = query.iter()
            .map(|&q| (base_of(&records[q]), serde_json::json!((f_all[q] * 100000.0).round() / 100000.0))).collect();
        notes.push(format!("diffusion regression: harmonic label-propagation on the {kk}-NN graph Laplacian (geometric), {fld}-fold held-out; the flat kNN baseline is reported alongside"));
        ("regression".to_string(),
         serde_json::json!({"r2": (r2*10000.0).round()/10000.0, "rmse": ((se/nt).sqrt()*100.0).round()/100.0}),
         serde_json::json!({"method": "flat_knn", "r2": (r2_base*10000.0).round()/10000.0}),
         preds)
    } else if is_reg {
        let y: Vec<f64> = records.iter().map(|r| r.get(target).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let ybar = train.iter().map(|&i| y[i]).sum::<f64>() / train.len() as f64;
        let tss: f64 = train.iter().map(|&i| (y[i] - ybar).powi(2)).sum();
        // global OLS weights over a training pool (normal equations, dim+1)
        let ols_fit = |pool: &[usize]| -> Vec<f64> {
            let m = dim + 1;
            let mut a = vec![vec![0.0f64; m]; m];
            let mut b = vec![0.0f64; m];
            for &i in pool {
                let mut basis = vec![1.0]; basis.extend_from_slice(&x[i]);
                for r in 0..m { for c in 0..m { a[r][c] += basis[r] * basis[c]; } b[r] += basis[r] * y[i]; }
            }
            scan_solve(&mut a, &b).unwrap_or_else(|| vec![ybar; m])
        };
        let predict_reg = |q: usize, pool: &[usize], w: &[f64]| -> (f64, f64) {
            let nb = knn_of(q, pool);
            let knn_mean = if nb.is_empty() { ybar } else { nb.iter().map(|(_, j)| y[*j]).sum::<f64>() / nb.len() as f64 };
            let main = match method {
                "knn" => knn_mean,
                "ols" => w.first().copied().unwrap_or(ybar) + (0..dim).map(|t| w[t + 1] * x[q][t]).sum::<f64>(),
                _ => if nb.is_empty() { ybar } else { local_linear_at(&x[q], &nb, &x, &y, dim, ridge) },
            };
            (main, knn_mean)
        };
        // k-fold CV → held-out RMSE/R² for the chosen method and the kNN baseline
        let mut se = 0.0; let mut se_base = 0.0;
        let fld = folds.max(2);
        for f in 0..fld {
            let te: std::collections::HashSet<usize> = train.iter().enumerate()
                .filter(|(ix, _)| ix % fld == f).map(|(_, &i)| i).collect();
            let pool: Vec<usize> = train.iter().copied().filter(|i| !te.contains(i)).collect();
            if pool.is_empty() { continue; }
            let w = if method == "ols" { ols_fit(&pool) } else { Vec::new() };
            for &q in &te {
                let (p, pb) = predict_reg(q, &pool, &w);
                se += (p - y[q]).powi(2); se_base += (pb - y[q]).powi(2);
            }
        }
        let nt = train.len() as f64;
        let (rmse, r2) = ((se / nt).sqrt(), 1.0 - se / tss.max(f64::EPSILON));
        let (rmse_b, r2_b) = ((se_base / nt).sqrt(), 1.0 - se_base / tss.max(f64::EPSILON));
        // predictions for missing-target records, trained on all labeled
        let w_all = if method == "ols" { ols_fit(&train) } else { Vec::new() };
        let preds: Vec<(String, serde_json::Value)> = query.iter().map(|&q| {
            let (p, _) = predict_reg(q, &train, &w_all);
            (base_of(&records[q]), serde_json::json!((p * 100000.0).round() / 100000.0))
        }).collect();
        notes.push(format!("regression on {dim} feature fibers, target '{target}', {}-fold held-out", fld));
        ("regression".to_string(),
         serde_json::json!({"rmse": (rmse*100000.0).round()/100000.0, "r2": (r2*10000.0).round()/10000.0}),
         serde_json::json!({"method": "knn_mean", "rmse": (rmse_b*100000.0).round()/100000.0, "r2": (r2_b*10000.0).round()/10000.0}),
         preds)
    } else {
        // classification: distance-weighted k-NN vote (default) or linear SVM
        let use_svm = method == "svm";
        let classes: Vec<String> = {
            let mut s: Vec<String> = train.iter().map(|&i| format!("{}", records[i].get(target).unwrap())).collect();
            s.sort(); s.dedup(); s
        };
        let cidx: std::collections::HashMap<String, usize> = classes.iter().enumerate().map(|(i, c)| (c.clone(), i)).collect();
        let yc: Vec<usize> = records.iter().map(|r| r.get(target).map(|v| *cidx.get(&format!("{}", v)).unwrap_or(&usize::MAX)).unwrap_or(usize::MAX)).collect();
        let ncl = classes.len();
        let vote = |nb: &[(f64, usize)]| -> usize {
            let mut w = vec![0.0f64; ncl];
            for &(d, j) in nb { if yc[j] != usize::MAX { w[yc[j]] += 1.0 / (d * d + 1e-6); } }
            (0..ncl).max_by(|&a, &b| w[a].partial_cmp(&w[b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0)
        };
        // linear SVM: one-vs-rest Pegasos SGD (hinge). Returns weights [class][dim+bias].
        let pegasos = |pool: &[usize]| -> Vec<Vec<f64>> {
            let lam = 0.01;
            let mut w = vec![vec![0.0f64; dim + 1]; ncl];
            let mut seed: u64 = 0x9E3779B97F4A7C15;
            let mut t = 1.0f64;
            for _ in 0..50 {
                let mut order: Vec<usize> = pool.iter().copied().filter(|&i| yc[i] != usize::MAX).collect();
                for a in (1..order.len()).rev() {
                    seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    order.swap(a, (seed >> 11) as usize % (a + 1));
                }
                for &i in &order {
                    t += 1.0; let eta = 1.0 / (lam * t);
                    for c in 0..ncl {
                        let yb = if yc[i] == c { 1.0 } else { -1.0 };
                        let dot = (0..dim).map(|d| w[c][d] * x[i][d]).sum::<f64>() + w[c][dim];
                        for d in 0..dim { w[c][d] *= 1.0 - eta * lam; }
                        w[c][dim] *= 1.0 - eta * lam;
                        if yb * dot < 1.0 {
                            for d in 0..dim { w[c][d] += eta * yb * x[i][d]; }
                            w[c][dim] += eta * yb;
                        }
                    }
                }
            }
            w
        };
        let svm_pred = |w: &[Vec<f64>], q: usize| -> usize {
            (0..ncl).max_by(|&a, &b| {
                let da = (0..dim).map(|d| w[a][d] * x[q][d]).sum::<f64>() + w[a][dim];
                let db = (0..dim).map(|d| w[b][d] * x[q][d]).sum::<f64>() + w[b][dim];
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            }).unwrap_or(0)
        };
        // GEOMETRIC classifier: label propagation (diffusion) on the manifold graph
        let use_diffusion = method == "diffusion";
        let wadj = if use_diffusion { build_diffusion_graph(&x, kk, n, &dist2) } else { Vec::new() };
        let labelprop = |labeled: &std::collections::HashSet<usize>| -> Vec<usize> {
            let mut scores = vec![vec![0.0f64; ncl]; n];
            for c in 0..ncl {
                let clamp: Vec<Option<f64>> = (0..n).map(|i|
                    if labeled.contains(&i) && yc[i] != usize::MAX { Some(if yc[i] == c { 1.0 } else { 0.0 }) } else { None }).collect();
                let f = diffuse(&wadj, &clamp, 60);
                for i in 0..n { scores[i][c] = f[i]; }
            }
            (0..n).map(|i| (0..ncl).max_by(|&a, &b| scores[i][a]
                .partial_cmp(&scores[i][b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(0)).collect()
        };
        let fld = folds.max(2);
        let (mut correct, mut total) = (0usize, 0usize);
        for f in 0..fld {
            let te: std::collections::HashSet<usize> = train.iter().enumerate()
                .filter(|(ix, _)| ix % fld == f).map(|(_, &i)| i).collect();
            let pool_set: std::collections::HashSet<usize> = train.iter().copied().filter(|i| !te.contains(i)).collect();
            let pool: Vec<usize> = pool_set.iter().copied().collect();
            let w = if use_svm { pegasos(&pool) } else { Vec::new() };
            let dpred = if use_diffusion { labelprop(&pool_set) } else { Vec::new() };
            for &q in &te {
                let pred = if use_svm { svm_pred(&w, q) }
                    else if use_diffusion { dpred[q] }
                    else {
                        let nb = knn_of(q, &pool);
                        if nb.is_empty() { continue; }
                        vote(&nb)
                    };
                total += 1; if pred == yc[q] { correct += 1; }
            }
        }
        let mut cnt = vec![0usize; ncl];
        for &i in &train { if yc[i] != usize::MAX { cnt[yc[i]] += 1; } }
        let maj = *cnt.iter().max().unwrap_or(&0) as f64 / train.len() as f64;
        let acc = if total > 0 { correct as f64 / total as f64 } else { 0.0 };
        let train_set: std::collections::HashSet<usize> = train.iter().copied().collect();
        let w_all = if use_svm { pegasos(&train) } else { Vec::new() };
        let dpred_all = if use_diffusion { labelprop(&train_set) } else { Vec::new() };
        let preds: Vec<(String, serde_json::Value)> = query.iter().filter_map(|&q| {
            let c = if use_svm { svm_pred(&w_all, q) }
                else if use_diffusion { dpred_all[q] }
                else {
                    let nb = knn_of(q, &train);
                    if nb.is_empty() { return None; }
                    vote(&nb)
                };
            Some((base_of(&records[q]), serde_json::json!(classes[c])))
        }).collect();
        notes.push(format!("classification ({}) on {dim} feature fibers, target '{target}', {ncl} classes, {fld}-fold held-out",
            if use_svm { "linear SVM, one-vs-rest hinge" } else if use_diffusion { "label-propagation diffusion on the graph Laplacian" } else { "distance-weighted kNN vote" }));
        ("classification".to_string(),
         serde_json::json!({"accuracy": (acc*10000.0).round()/10000.0}),
         serde_json::json!({"method": "majority_class", "accuracy": (maj*10000.0).round()/10000.0}),
         preds)
    };
    if !query.is_empty() { notes.push(format!("filled target for {} record(s) with a missing '{}'", query.len(), target)); }
    Ok(PredictResult { base, task,
        method: if is_reg { method.to_string() } else if method == "svm" { "svm".to_string() }
            else if method == "diffusion" { "diffusion".to_string() } else { "knn_vote".to_string() },
        metric, baseline, n_train: train.len(), predictions, notes })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// Regression: the curvature-aware local-linear head beats flat kNN-mean AND
    /// global OLS on a curved target y = sin(x1)·cos(x2)+0.3·x1 (held-out R²).
    #[test]
    fn predict_local_linear_beats_baselines() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut s: u64 = 12345;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 33) as f64 / (1u64 << 31) as f64 };
        for i in 0..500 {
            let (x1, x2) = (4.0 * rnd(), 4.0 * rnd());
            let y = x1.sin() * x2.cos() + 0.3 * x1;
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("x1", V::Float(x1)), ("x2", V::Float(x2)), ("y", V::Float(y)),
            ]));
        }
        let schema = BundleSchema::new("reg")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x1")).fiber(FieldDef::numeric("x2")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("predict_reg", "reg", schema, rows);
        let r2 = |m: &str| {
            let pr = predict_field(&engine, "reg", "y", m, 20, 0.5, 5, &[]).expect("predict");
            assert_eq!(pr.task, "regression");
            pr.metric["r2"].as_f64().unwrap()
        };
        let (ll, knn, ols) = (r2("local_linear"), r2("knn"), r2("ols"));
        assert!(ll > knn, "local_linear R² ({ll}) should beat flat kNN ({knn})");
        assert!(ll > ols + 0.2, "local_linear R² ({ll}) should crush global OLS ({ols}) on a curved target");
        assert!(ll > 0.9, "local_linear should fit the curve well, R²={ll}");
        cleanup(&dir);
    }

    /// GP: the conformal adaptive intervals are calibrated — held-out 90% coverage
    /// lands near 0.90 — and a missing target is filled with mean + lower/upper.
    #[test]
    fn predict_gp_intervals_are_calibrated() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut s: u64 = 424242;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 33) as f64 / (1u64 << 31) as f64 };
        // noisy target: y = 2·x1 − x2 + heteroscedastic noise
        for i in 0..400 {
            let (x1, x2) = (rnd() * 4.0, rnd() * 4.0);
            let noise = (rnd() - 0.5) * (1.0 + x1);   // noise grows with x1
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("x1", V::Float(x1)), ("x2", V::Float(x2)), ("y", V::Float(2.0 * x1 - x2 + noise)),
            ]));
        }
        rows.push(scan_rec(&[("id", V::Text("q".into())), ("x1", V::Float(2.0)), ("x2", V::Float(2.0))]));
        let schema = BundleSchema::new("gp")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x1")).fiber(FieldDef::numeric("x2")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("predict_gp", "gp", schema, rows);
        let pr = predict_field(&engine, "gp", "y", "gp", 25, 0.5, 5, &[]).expect("gp");
        let cov = pr.metric["coverage_90"].as_f64().unwrap();
        assert!((0.80..=1.0).contains(&cov), "90% conformal coverage should be near nominal, got {cov}");
        // the fill has a mean and an ordered interval
        let q = pr.predictions.iter().find(|(id, _)| id == "q").expect("q predicted");
        let (lo, mean, hi) = (q.1["lower"].as_f64().unwrap(), q.1["mean"].as_f64().unwrap(), q.1["upper"].as_f64().unwrap());
        assert!(lo < mean && mean < hi, "interval should bracket the mean: {lo} < {mean} < {hi}");
        cleanup(&dir);
    }

    /// The GEOMETRIC predictor (diffusion on the graph Laplacian) fits a target that
    /// varies along a curved manifold, and matches or beats the flat kNN baseline it
    /// reports — geometry buying something where geometry exists.
    #[test]
    fn predict_diffusion_beats_flat_on_manifold() {
        let mut s: u64 = 20260719;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 };
        // swiss-roll: (x1,x2,x3) on a rolled sheet; target varies along the roll param t
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for i in 0..400 {
            let (t, u, noise) = (rnd() * 3.0, rnd() * 3.0, (rnd() - 0.5) * 0.1);
            let tgt = (t * 2.0).sin() + 0.3 * u + noise;
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("x1", V::Float(t * (t * 2.0).cos())), ("x2", V::Float(u)), ("x3", V::Float(t * (t * 2.0).sin())),
                ("y", V::Float(tgt)),
            ]));
        }
        let schema = BundleSchema::new("roll")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x1")).fiber(FieldDef::numeric("x2")).fiber(FieldDef::numeric("x3")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("diffusion", "roll", schema, rows);
        let pr = predict_field(&engine, "roll", "y", "diffusion", 12, 0.5, 5, &[]).expect("diffusion");
        let r2 = pr.metric["r2"].as_f64().unwrap();
        let flat = pr.baseline["r2"].as_f64().unwrap();   // reported flat-kNN baseline
        assert!(r2 > 0.75, "diffusion should fit the manifold target well, R²={r2}");
        assert!(r2 >= flat - 0.03, "diffusion ({r2}) should match or beat flat kNN ({flat}) on the manifold");
        cleanup(&dir);
    }

    /// Classification: distance-weighted k-NN predicts a categorical target well
    /// above the majority-class baseline, and fills a missing label.
    #[test]
    fn predict_knn_classifies_and_fills() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for (c, (ox, oy)) in [(0.0, 0.0), (5.0, 5.0)].iter().enumerate() {
            for j in 0..30 {
                let a = j as f64 * 0.2;
                rows.push(scan_rec(&[
                    ("id", V::Text(format!("c{c}_{j}"))),
                    ("x", V::Float(ox + 0.6 * a.cos())), ("y", V::Float(oy + 0.6 * a.sin())),
                    ("label", V::Text(format!("class{c}"))),
                ]));
            }
        }
        // one record with a MISSING label, sitting in cluster 1 → should be filled class1
        rows.push(scan_rec(&[("id", V::Text("mystery".into())), ("x", V::Float(5.1)), ("y", V::Float(4.9))]));
        let schema = BundleSchema::new("cls")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x")).fiber(FieldDef::numeric("y")).fiber(FieldDef::categorical("label"));
        let (dir, engine) = scan_env("predict_cls", "cls", schema, rows);
        // both kNN and the linear SVM separate the classes and fill the missing label
        for method in ["knn", "svm"] {
            let pr = predict_field(&engine, "cls", "label", method, 7, 0.5, 5, &[]).expect("predict");
            assert_eq!(pr.task, "classification");
            let acc = pr.metric["accuracy"].as_f64().unwrap();
            let maj = pr.baseline["accuracy"].as_f64().unwrap();
            assert!(acc > 0.95, "{method} should separate the two classes, accuracy={acc}");
            assert!(acc > maj, "{method} accuracy ({acc}) should beat majority-class baseline ({maj})");
            let filled = pr.predictions.iter().find(|(id, _)| id == "mystery").expect("mystery should be predicted");
            assert_eq!(filled.1.as_str().unwrap(), "class1", "{method}: mystery point sits in cluster 1");
            assert_eq!(pr.method, if method == "svm" { "svm" } else { "knn_vote" });
        }
        cleanup(&dir);
    }

    /// Predict endpoint gives actionable errors, not panics, on bad input.
    #[test]
    fn predict_guards_are_actionable() {
        let rows: Vec<_> = (0..20).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("a", V::Float(i as f64)), ("b", V::Float((i * i) as f64)),
        ])).collect();
        let schema = BundleSchema::new("pg")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("a")).fiber(FieldDef::numeric("b"));
        let (dir, engine) = scan_env("predict_guard", "pg", schema, rows);
        // unknown target → clear error
        let e = predict_field(&engine, "pg", "nope", "local_linear", 5, 0.5, 5, &[]).unwrap_err();
        assert_eq!(e.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(e.1.contains("not found"), "got: {}", e.1);
        // unknown regression method → clear error
        assert!(predict_field(&engine, "pg", "b", "banana", 5, 0.5, 5, &[]).unwrap_err().1.contains("unknown regression method"));
        // missing bundle → 404
        assert_eq!(predict_field(&engine, "no", "b", "knn", 5, 0.5, 5, &[]).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
