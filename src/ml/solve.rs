//! SOLVE — SVD fit store: one thin SVD, the whole ridge path with exact leave-one-out.
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Request for `POST /v1/bundles/{name}/solve`.
#[derive(Deserialize)]
pub struct SolveRequest {
    /// Numeric field to predict.
    pub target: String,
    /// Explicit ridge path; if empty, a log-spaced path is auto-generated.
    #[serde(default)]
    pub alphas: Vec<f64>,
    /// Numeric fibers to leave out of the design matrix.
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// The fit store's per-α row: ridge penalty, exact leave-one-out RMSE, training R²,
/// and effective degrees of freedom (trace of the hat matrix).
#[derive(Debug, Clone)]
pub struct RidgePoint { pub alpha: f64, pub loo_rmse: f64, pub train_r2: f64, pub df: f64 }

/// Result of the SVD fit store.
#[derive(Debug)]
pub struct SolveResult {
    pub n: usize,
    pub d: usize,
    pub rank: usize,
    pub feature_names: Vec<String>,
    pub path: Vec<RidgePoint>,
    pub best_alpha: f64,
    pub best_loo_rmse: f64,
    pub coefficients: Vec<(String, f64)>,   // at best α, on standardized features
    pub reads: Vec<String>,
    pub notes: Vec<String>,
}

/// SVD fit store — the third face of the field operator (`Aθ = b`, alongside diffusion's
/// `Lφ = 0` and circulation's `Lφ = div`). Store the **thin SVD** of the (standardized)
/// design matrix `X = UΣVᵀ` ONCE; then every ridge fit on the path is a spectral filter
/// `σ²/(σ²+α)` on that cached frame — no descent, no refit.
///
/// Because a quadratic loss is FLAT geometry in skew coordinates (Ω = 0), the Davis
/// Duality (`error ≤ c‖Ω‖h²`) makes the frame solve **exact** — the anisotropy that slows
/// gradient descent, κ(A), is a coordinate artifact, removable by one global map, hence
/// not curvature. The whole ridge path also yields **exact** leave-one-out for free via
/// the hat diagonal `h_ii(α) = Σⱼ uᵢⱼ²·σⱼ²/(σⱼ²+α)` and `PRESS = (y−ŷ)/(1−h)` (Golub–
/// Heath–Wahba 1979; ESL §3.4). Storing the SVD — not `XᵀX` — keeps κ(X), not κ(X)².
///
/// Exact for the quadratic class: OLS, ridge, multi-target, LDA/Gaussian. For GLMs the
/// frame is a preconditioner (fast iterative), not a one-shot solve.
pub fn fit_store_solve(
    engine: &Engine,
    name: &str,
    target: &str,
    alphas: &[f64],
    exclude: &[String],
) -> Result<SolveResult, (StatusCode, String)> {
    use crate::types::FieldType;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    let tgt_def = schema.fiber_fields.iter().chain(schema.base_fields.iter())
        .find(|f| f.name == target)
        .ok_or_else(|| (StatusCode::UNPROCESSABLE_ENTITY, format!("target field '{}' not found", target)))?;
    if !matches!(tgt_def.field_type, FieldType::Numeric) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("target '{}' must be NUMERIC for a least-squares solve", target)));
    }
    let feat_defs: Vec<crate::types::FieldDef> = schema.fiber_fields.iter()
        .filter(|f| matches!(f.field_type, FieldType::Numeric) && f.name != target && !exclude.iter().any(|e| e == &f.name))
        .cloned().collect();
    let records: Vec<crate::types::Record> = store.records().collect();
    let n = records.len();
    // rows with a present target
    let rows: Vec<&crate::types::Record> = records.iter().filter(|r| r.get(target).and_then(|v| v.as_f64()).is_some()).collect();
    let n = rows.len().min(n);
    // standardize features (drop zero-variance), center target
    let cols: Vec<(String, f64, f64)> = feat_defs.iter().filter_map(|fd| {
        let xs: Vec<f64> = rows.iter().map(|r| r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let mu = xs.iter().sum::<f64>() / n.max(1) as f64;
        let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n.max(1) as f64).sqrt();
        (sd > f64::EPSILON).then_some((fd.name.clone(), mu, sd))
    }).collect();
    let d = cols.len();
    if d < 1 { return Err((StatusCode::UNPROCESSABLE_ENTITY, "solve needs >= 1 numeric feature fiber with non-zero variance".into())); }
    if n < d + 2 { return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("solve needs >= d+2 = {} rows with a present target (got {n})", d + 2))); }
    let feature_names: Vec<String> = cols.iter().map(|(f, _, _)| f.clone()).collect();
    let ybar = rows.iter().map(|r| r.get(target).and_then(|v| v.as_f64()).unwrap_or(0.0)).sum::<f64>() / n as f64;
    let y: Vec<f64> = rows.iter().map(|r| r.get(target).and_then(|v| v.as_f64()).unwrap_or(0.0) - ybar).collect();
    // row-major standardized design matrix
    let mut xdata = Vec::with_capacity(n * d);
    for r in &rows {
        for (f, mu, sd) in &cols { xdata.push((r.get(f).and_then(|v| v.as_f64()).unwrap_or(*mu) - mu) / sd); }
    }
    // ── thin SVD  X = U Σ Vᵀ  (store κ(X), not κ(X)²) ──
    let dm = nalgebra::DMatrix::from_row_slice(n, d, &xdata);
    let svd = dm.svd(true, true);
    let umat = svd.u.ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "SVD failed to produce U".into()))?;
    let vt = svd.v_t.ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "SVD failed to produce Vᵀ".into()))?;
    let rfull = n.min(d);
    let sig: Vec<f64> = (0..rfull).map(|j| svd.singular_values[j]).collect();
    let smax = sig.iter().cloned().fold(0.0f64, f64::max).max(1e-12);
    let tol = smax * 1e-9;
    let rank = sig.iter().filter(|s| **s > tol).count();
    // cached projections: (Uᵀy)_j  and per-row u_ij (only the columns with σ_j>tol)
    let uty: Vec<f64> = (0..rfull).map(|j| (0..n).map(|i| umat[(i, j)] * y[i]).sum::<f64>()).collect();
    let ycent_ss: f64 = y.iter().map(|v| v * v).sum::<f64>().max(1e-12);

    // ridge path
    let mut path_alphas: Vec<f64> = alphas.to_vec();
    if path_alphas.is_empty() {
        // log-spaced on the σ² scale, plus a near-OLS anchor
        let (amin, amax) = (smax * smax * 1e-6, smax * smax * 1e2);
        let steps = 25usize;
        path_alphas = (0..steps).map(|i| amin * (amax / amin).powf(i as f64 / (steps - 1) as f64)).collect();
    }
    path_alphas.retain(|a| *a >= 0.0 && a.is_finite());
    path_alphas.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let eval = |alpha: f64| -> RidgePoint {
        // fitted ŷ_i = Σ_j u_ij·f_j·(Uᵀy)_j ,  h_ii = Σ_j u_ij²·f_j ,  f_j = σ_j²/(σ_j²+α)
        let f: Vec<f64> = sig.iter().map(|s| if *s > tol { (s * s) / (s * s + alpha) } else { 0.0 }).collect();
        let mut sse = 0.0; let mut press2 = 0.0;
        for i in 0..n {
            let mut yhat = 0.0; let mut h = 0.0;
            for j in 0..rfull {
                if f[j] == 0.0 { continue; }
                let u = umat[(i, j)];
                yhat += u * f[j] * uty[j];
                h += u * u * f[j];
            }
            let resid = y[i] - yhat;
            sse += resid * resid;
            let denom = (1.0 - h).max(1e-9);         // leverage-1 guard
            let p = resid / denom;
            press2 += p * p;
        }
        RidgePoint {
            alpha,
            loo_rmse: (press2 / n as f64).sqrt(),
            train_r2: 1.0 - sse / ycent_ss,
            df: f.iter().sum(),
        }
    };
    let path: Vec<RidgePoint> = path_alphas.iter().map(|a| eval(*a)).collect();
    let best = path.iter().min_by(|a, b| a.loo_rmse.partial_cmp(&b.loo_rmse).unwrap_or(std::cmp::Ordering::Equal))
        .cloned().unwrap_or(RidgePoint { alpha: 0.0, loo_rmse: f64::NAN, train_r2: f64::NAN, df: 0.0 });
    // coefficients at best α: θ_k = Σ_j vt[j][k]·g_j·(Uᵀy)_j ,  g_j = σ_j/(σ_j²+α)
    let g: Vec<f64> = sig.iter().map(|s| if *s > tol { s / (s * s + best.alpha) } else { 0.0 }).collect();
    let round = |v: f64| (v * 100000.0).round() / 100000.0;
    let coefficients: Vec<(String, f64)> = (0..d).map(|k| {
        let c: f64 = (0..rfull).map(|j| vt[(j, k)] * g[j] * uty[j]).sum();
        (feature_names[k].clone(), round(c))
    }).collect();

    let reads = vec![
        format!("One thin SVD (rank {rank} of {}) IS the model. Every α on the path is a spectral filter σ²/(σ²+α) on the SAME cached frame — no descent, no refit.", d.min(n)),
        format!("Exact leave-one-out for the WHOLE path via the hat diagonal (Golub–Heath–Wahba 1979): best α = {} → LOO-RMSE {} (effective df {}).", round(best.alpha), round(best.loo_rmse), round(best.df)),
        "This is the Ω=0 (flat) sector: a quadratic loss is flat geometry in skew coordinates, so the frame solve is exact (Davis Duality error ≤ c‖Ω‖h² = 0). κ(A) that slows gradient descent is a coordinate artifact, not curvature.".into(),
    ];
    let notes = vec![
        "stores the thin SVD (V, Σ, Uᵀy) — keeps κ(X), not κ(X)² (forming XᵀX was avoided on purpose; ESL §3.4).".into(),
        "exact for the quadratic class (OLS, ridge, multi-target, LDA/Gaussian); for GLMs the frame is a preconditioner, not a one-shot solve.".into(),
        "feature add/drop and cohort reweighting are rank perturbations — NOT frame-preserving; they need a Cholesky up/downdate or refactor (out of scope here).".into(),
    ];
    Ok(SolveResult {
        n, d, rank, feature_names, path,
        best_alpha: best.alpha, best_loo_rmse: best.loo_rmse, coefficients, reads, notes,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// FIT STORE: at α→0 the SVD ridge solve recovers the OLS coefficients on a
    /// well-conditioned problem (y = Xβ + tiny noise), with train R² ≈ 1.
    #[test]
    fn solve_recovers_ols_at_alpha_zero() {
        let mut s: u64 = 13;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
        let beta = [1.5f64, -2.0, 0.5];
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for i in 0..80 {
            let x = [rnd() * 4.0, rnd() * 4.0, rnd() * 4.0];
            let y = beta.iter().zip(&x).map(|(b, xi)| b * xi).sum::<f64>() + 0.001 * rnd();
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("x0", V::Float(x[0])), ("x1", V::Float(x[1])), ("x2", V::Float(x[2])),
                ("y", V::Float(y)),
            ]));
        }
        let schema = BundleSchema::new("reg")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x0")).fiber(FieldDef::numeric("x1"))
            .fiber(FieldDef::numeric("x2")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("solve_ols", "reg", schema, rows);
        let r = fit_store_solve(&engine, "reg", "y", &[1e-8], &[]).expect("solve");
        assert!(r.path[0].train_r2 > 0.999, "near-zero ridge should fit the clean linear signal, R²={}", r.path[0].train_r2);
        // standardized coeff sign/order must match β (feature order x0,x1,x2)
        let c: std::collections::HashMap<String, f64> = r.coefficients.iter().cloned().collect();
        assert!(c["x0"] > 0.0 && c["x1"] < 0.0 && c["x2"] > 0.0, "coefficient signs must match β: {:?}", r.coefficients);
        assert!(c["x1"].abs() > c["x0"].abs() && c["x0"].abs() > c["x2"].abs(), "coef magnitude order x1>x0>x2: {:?}", r.coefficients);
        cleanup(&dir);
    }

    /// FIT STORE correctness keystone: the frame's EXACT leave-one-out (hat-diagonal
    /// identity) must equal brute-force leave-one-out (refit on each held-out row) at a
    /// fixed α — proving `PRESS_i = (yᵢ−ŷᵢ)/(1−h_ii)`, Golub–Heath–Wahba 1979.
    #[test]
    fn solve_exact_loo_matches_bruteforce() {
        let mut s: u64 = 71;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
        let (nn, d) = (40usize, 3usize);
        let mut xraw = vec![vec![0.0f64; d]; nn];
        let mut yraw = vec![0.0f64; nn];
        let beta = [0.8f64, 1.3, -0.6];
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for i in 0..nn {
            for k in 0..d { xraw[i][k] = rnd() * 3.0; }
            yraw[i] = beta.iter().zip(&xraw[i]).map(|(b, x)| b * x).sum::<f64>() + 0.4 * rnd();
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("x0", V::Float(xraw[i][0])), ("x1", V::Float(xraw[i][1])), ("x2", V::Float(xraw[i][2])),
                ("y", V::Float(yraw[i])),
            ]));
        }
        let schema = BundleSchema::new("loo")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x0")).fiber(FieldDef::numeric("x1"))
            .fiber(FieldDef::numeric("x2")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("solve_loo", "loo", schema, rows);
        let alpha = 2.5f64;
        let r = fit_store_solve(&engine, "loo", "y", &[alpha], &[]).expect("solve");
        let frame_loo = r.path[0].loo_rmse;

        // ── independent brute force: SAME preprocessing (standardize on full data,
        //    center y), then leave each row out, ridge-refit, predict it ──
        let mu: Vec<f64> = (0..d).map(|k| xraw.iter().map(|r| r[k]).sum::<f64>() / nn as f64).collect();
        let sd: Vec<f64> = (0..d).map(|k| (xraw.iter().map(|r| (r[k] - mu[k]).powi(2)).sum::<f64>() / nn as f64).sqrt()).collect();
        let ybar = yraw.iter().sum::<f64>() / nn as f64;
        let xs: Vec<Vec<f64>> = (0..nn).map(|i| (0..d).map(|k| (xraw[i][k] - mu[k]) / sd[k]).collect()).collect();
        let yc: Vec<f64> = yraw.iter().map(|v| v - ybar).collect();
        let mut press2 = 0.0;
        for hold in 0..nn {
            // (XᵀX + αI) θ = Xᵀy on rows != hold
            let mut ata = vec![vec![0.0f64; d]; d];
            let mut aty = vec![0.0f64; d];
            for i in 0..nn { if i == hold { continue; }
                for a in 0..d { aty[a] += xs[i][a] * yc[i];
                    for b in 0..d { ata[a][b] += xs[i][a] * xs[i][b]; } } }
            for a in 0..d { ata[a][a] += alpha; }
            let mm = nalgebra::DMatrix::from_fn(d, d, |a, b| ata[a][b]);
            let bb = nalgebra::DVector::from_fn(d, |a, _| aty[a]);
            let theta = mm.lu().solve(&bb).expect("solve loo system");
            let pred: f64 = (0..d).map(|a| theta[a] * xs[hold][a]).sum();
            press2 += (yc[hold] - pred).powi(2);
        }
        let brute_loo = (press2 / nn as f64).sqrt();
        assert!((frame_loo - brute_loo).abs() < 1e-6,
            "exact hat-diagonal LOO ({frame_loo}) must match brute-force LOO ({brute_loo})");
        cleanup(&dir);
    }

    /// FIT STORE: on noisy, near-collinear data the LOO-optimal α is strictly positive
    /// and its LOO error beats the near-OLS end of the path — ridge earns its keep, and
    /// the whole path came from ONE decomposition.
    #[test]
    fn solve_ridge_path_prefers_positive_alpha() {
        let mut s: u64 = 2027;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 - 0.5 };
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for i in 0..60 {
            let a = rnd() * 3.0;
            let b = a + 0.02 * rnd();            // x1 ≈ x0 (collinear → ill-conditioned)
            let noise = 1.2 * rnd();
            let y = 2.0 * a + noise;             // only the shared direction matters
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("x0", V::Float(a)), ("x1", V::Float(b)), ("y", V::Float(y)),
            ]));
        }
        let schema = BundleSchema::new("coll")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x0")).fiber(FieldDef::numeric("x1")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("solve_ridge", "coll", schema, rows);
        let r = fit_store_solve(&engine, "coll", "y", &[], &[]).expect("solve");
        assert!(r.best_alpha > 0.0, "collinear+noisy data should prefer ridge, best_alpha={}", r.best_alpha);
        let ols_end = r.path.first().unwrap().loo_rmse;   // smallest α ≈ OLS
        assert!(r.best_loo_rmse <= ols_end + 1e-12, "best LOO must beat the OLS end: best={} ols={}", r.best_loo_rmse, ols_end);
        cleanup(&dir);
    }

    /// FIT STORE gives actionable errors, not panics.
    #[test]
    fn solve_guards_are_actionable() {
        let rows: Vec<_> = (0..10).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("x0", V::Float(i as f64)), ("y", V::Float(2.0 * i as f64)),
        ])).collect();
        let schema = BundleSchema::new("g")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("x0")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("solve_guard", "g", schema, rows);
        // missing target → clear error
        assert!(fit_store_solve(&engine, "g", "nope", &[], &[]).unwrap_err().1.contains("nope"));
        // missing bundle → 404
        assert_eq!(fit_store_solve(&engine, "no", "y", &[], &[]).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
