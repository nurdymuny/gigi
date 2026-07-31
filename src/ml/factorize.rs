//! FACTORIZE — recommender matrix factorization (rank-k latent factors + biases, SGD).
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Request for `POST /v1/bundles/{name}/factorize`.
#[derive(Deserialize)]
pub struct FactorizeRequest {
    /// Categorical field naming the user (row).
    pub user: String,
    /// Categorical field naming the item (column).
    pub item: String,
    /// Numeric field with the observed rating.
    pub rating: String,
    /// Latent-factor rank. Default 10.
    #[serde(default = "default_mf_rank")]
    pub rank: usize,
    /// SGD epochs. Default 40.
    #[serde(default = "default_mf_epochs")]
    pub epochs: usize,
    /// L2 regularization on factors and biases. Default 0.05.
    #[serde(default = "default_mf_reg")]
    pub reg: f64,
    /// SGD learning rate. Default 0.02.
    #[serde(default = "default_mf_lr")]
    pub learning_rate: f64,
}
pub fn default_mf_rank() -> usize { 10 }
pub fn default_mf_epochs() -> usize { 40 }
pub fn default_mf_reg() -> f64 { 0.05 }
pub fn default_mf_lr() -> f64 { 0.02 }

#[derive(Debug)]
pub struct FactorizeResult {
    pub base: String,
    pub rmse: f64,
    pub baseline_rmse: f64,
    pub rank: usize,
    pub n_users: usize,
    pub n_items: usize,
    pub n_ratings: usize,
    pub predictions: Vec<(String, f64)>,
    pub notes: Vec<String>,
}

/// Matrix factorization for recommendation: learn rank-k latent user/item factors
/// (plus global mean + per-user/item biases) by regularized SGD over the observed
/// (user, item, rating) triples — r̂(u,i) = μ + bᵤ + bᵢ + pᵤ·qᵢ. Reports held-out
/// RMSE against the global-mean baseline and fills ratings for records where the
/// rating is missing.
pub fn factorize_matrix(
    engine: &Engine, name: &str, user: &str, item: &str, rating: &str,
    rank: usize, epochs: usize, reg: f64, lr: f64,
) -> Result<FactorizeResult, (StatusCode, String)> {
    use crate::types::FieldType;
    const MF_MAX_RATINGS: usize = 500_000;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "factorize requires a single base-key field".into()));
    }
    let base = schema.base_fields[0].name.clone();
    let field = |nm: &str| schema.fiber_fields.iter().find(|f| f.name == nm);
    for (nm, label) in [(user, "user"), (item, "item"), (rating, "rating")] {
        if field(nm).is_none() {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("{label} field '{nm}' not found among the bundle's fibers")));
        }
    }
    if !matches!(field(rating).map(|f| &f.field_type), Some(FieldType::Numeric)) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("rating field '{rating}' must be NUMERIC")));
    }
    if rank < 1 { return Err((StatusCode::UNPROCESSABLE_ENTITY, "rank must be >= 1".into())); }
    let records: Vec<crate::types::Record> = store.records().collect();
    let base_of = |r: &crate::types::Record| r.get(&base).map(|v| format!("{}", v)).unwrap_or_default();
    let mut users: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut items: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut obs: Vec<(usize, usize, f64)> = Vec::new();
    let mut query: Vec<(String, String, String)> = Vec::new();   // (base_id, user, item) with missing rating
    for r in &records {
        let (u, it) = match (r.get(user), r.get(item)) {
            (Some(u), Some(it)) => (format!("{}", u), format!("{}", it)),
            _ => continue,
        };
        match r.get(rating).and_then(|v| v.as_f64()) {
            Some(val) => {
                let ui = { let n = users.len(); *users.entry(u).or_insert(n) };
                let ii = { let n = items.len(); *items.entry(it).or_insert(n) };
                obs.push((ui, ii, val));
            }
            None => query.push((base_of(r), u, it)),
        }
    }
    if obs.len() < 20 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "factorize needs more observed ratings (found {})", obs.len())));
    }
    if obs.len() > MF_MAX_RATINGS {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "factorize: {} ratings exceeds the limit ({}) in this version", obs.len(), MF_MAX_RATINGS)));
    }
    let (nu, ni) = (users.len(), items.len());
    let mut seed: u64 = 0x243F6A8885A308D3;
    let mut lcg = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64 };
    // regularized-SGD factorization over a training subset → (P, Q, bu, bi, μ)
    let epochs = epochs.clamp(1, 500);
    let fit = |tr: &[(usize, usize, f64)], lcg: &mut dyn FnMut() -> f64|
        -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, f64) {
        let mu = tr.iter().map(|t| t.2).sum::<f64>() / tr.len().max(1) as f64;
        let mut p: Vec<f64> = (0..nu * rank).map(|_| (lcg() - 0.5) * 0.1).collect();
        let mut q: Vec<f64> = (0..ni * rank).map(|_| (lcg() - 0.5) * 0.1).collect();
        let mut bu = vec![0.0f64; nu];
        let mut bi = vec![0.0f64; ni];
        let mut order: Vec<usize> = (0..tr.len()).collect();
        for _ in 0..epochs {
            for a in (1..order.len()).rev() { let b = (lcg() * (a + 1) as f64) as usize % (a + 1); order.swap(a, b); }
            for &idx in &order {
                let (u, it, r) = tr[idx];
                let dot: f64 = (0..rank).map(|f| p[u * rank + f] * q[it * rank + f]).sum();
                let e = r - (mu + bu[u] + bi[it] + dot);
                bu[u] += lr * (e - reg * bu[u]);
                bi[it] += lr * (e - reg * bi[it]);
                for f in 0..rank {
                    let pf = p[u * rank + f];
                    p[u * rank + f] += lr * (e * q[it * rank + f] - reg * pf);
                    q[it * rank + f] += lr * (e * pf - reg * q[it * rank + f]);
                }
            }
        }
        (p, q, bu, bi, mu)
    };
    let predict = |m: &(Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>, f64), u: Option<usize>, it: Option<usize>| -> f64 {
        let (p, q, bu, bi, mu) = m;
        let mut r = *mu;
        if let Some(u) = u { r += bu[u]; }
        if let Some(it) = it { r += bi[it]; }
        if let (Some(u), Some(it)) = (u, it) {
            r += (0..rank).map(|f| p[u * rank + f] * q[it * rank + f]).sum::<f64>();
        }
        r
    };
    // held-out RMSE: deterministic 80/20 split
    let mut idx: Vec<usize> = (0..obs.len()).collect();
    for a in (1..idx.len()).rev() { let b = (lcg() * (a + 1) as f64) as usize % (a + 1); idx.swap(a, b); }
    let cut = (idx.len() as f64 * 0.8) as usize;
    let tr: Vec<(usize, usize, f64)> = idx[..cut].iter().map(|&i| obs[i]).collect();
    let te: Vec<(usize, usize, f64)> = idx[cut..].iter().map(|&i| obs[i]).collect();
    let model_cv = fit(&tr, &mut lcg);
    let mu_tr = model_cv.4;
    let (mut se, mut se_base) = (0.0f64, 0.0f64);
    for &(u, it, r) in &te {
        se += (r - predict(&model_cv, Some(u), Some(it))).powi(2);
        se_base += (r - mu_tr).powi(2);
    }
    let nte = te.len().max(1) as f64;
    let (rmse, baseline_rmse) = ((se / nte).sqrt(), (se_base / nte).sqrt());
    // final model on all observed → fill missing ratings
    let model = fit(&obs, &mut lcg);
    let predictions: Vec<(String, f64)> = query.iter().map(|(id, u, it)| {
        let pr = predict(&model, users.get(u).copied(), items.get(it).copied());
        (id.clone(), (pr * 1000.0).round() / 1000.0)
    }).collect();
    let notes = vec![format!(
        "matrix factorization: rank {rank}, {nu} users × {ni} items, {} ratings, {epochs} SGD epochs", obs.len())];
    Ok(FactorizeResult { base, rmse, baseline_rmse, rank, n_users: nu, n_items: ni,
        n_ratings: obs.len(), predictions, notes })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// Matrix factorization recovers latent structure: on a synthetic rank-2 rating
    /// matrix, held-out RMSE beats the global-mean baseline, and a held-out (user,
    /// item) pair gets a filled rating.
    #[test]
    fn factorize_beats_mean_baseline() {
        let mut s: u64 = 20260719;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 };
        let (nu, ni, r) = (60usize, 40usize, 2usize);
        let pu: Vec<Vec<f64>> = (0..nu).map(|_| (0..r).map(|_| rnd() * 2.0 - 1.0).collect()).collect();
        let qi: Vec<Vec<f64>> = (0..ni).map(|_| (0..r).map(|_| rnd() * 2.0 - 1.0).collect()).collect();
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut held = None;
        for u in 0..nu {
            for it in 0..ni {
                if rnd() > 0.5 { continue; }   // observe ~50%
                let rating = 3.0 + 1.5 * (0..r).map(|f| pu[u][f] * qi[it][f]).sum::<f64>() + (rnd() - 0.5) * 0.15;
                let has_rating = !(u == 5 && it == 7);   // hold one out to fill
                if !has_rating { held = Some(rating); }
                let mut rec = vec![
                    ("id".to_string(), V::Text(format!("u{u}_i{it}"))),
                    ("u".to_string(), V::Text(format!("u{u}"))),
                    ("it".to_string(), V::Text(format!("i{it}"))),
                ];
                if has_rating { rec.push(("r".to_string(), V::Float(rating))); }
                rows.push(rec.into_iter().collect());
            }
        }
        let schema = BundleSchema::new("rate")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::categorical("u")).fiber(FieldDef::categorical("it")).fiber(FieldDef::numeric("r"));
        let (dir, engine) = scan_env("factorize", "rate", schema, rows);
        let fr = factorize_matrix(&engine, "rate", "u", "it", "r", 2, 80, 0.02, 0.02).expect("factorize");
        assert!(fr.rmse < fr.baseline_rmse, "MF RMSE ({}) should beat global-mean baseline ({})", fr.rmse, fr.baseline_rmse);
        assert!(fr.rmse.is_finite() && fr.rmse > 0.0);
        // the held-out (u5,i7) rating was filled
        if held.is_some() {
            assert!(fr.predictions.iter().any(|(id, v)| id == "u5_i7" && v.is_finite()), "held-out pair should be predicted");
        }
        cleanup(&dir);
    }

    /// Factorize gives actionable errors, not panics.
    #[test]
    fn factorize_guards_are_actionable() {
        let rows: Vec<_> = (0..30).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("u", V::Text(format!("u{}", i % 5))),
            ("it", V::Text(format!("i{}", i % 6))), ("r", V::Float((i % 5) as f64)),
        ])).collect();
        let schema = BundleSchema::new("rt")
            .base(FieldDef::categorical("id")).fiber(FieldDef::categorical("u"))
            .fiber(FieldDef::categorical("it")).fiber(FieldDef::numeric("r"));
        let (dir, engine) = scan_env("factorize_guard", "rt", schema, rows);
        // unknown field → clear error
        assert!(factorize_matrix(&engine, "rt", "nope", "it", "r", 5, 10, 0.05, 0.02).unwrap_err().1.contains("not found"));
        // rating field not numeric → clear error
        assert!(factorize_matrix(&engine, "rt", "u", "it", "u", 5, 10, 0.05, 0.02).unwrap_err().1.contains("NUMERIC"));
        // missing bundle → 404
        assert_eq!(factorize_matrix(&engine, "no", "u", "it", "r", 5, 10, 0.05, 0.02).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
