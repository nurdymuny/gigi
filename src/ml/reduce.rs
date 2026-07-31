//! REDUCE — PCA dimensionality reduction (power iteration + deflation).
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Request for `POST /v1/bundles/{name}/reduce`.
#[derive(Deserialize)]
pub struct ReduceRequest {
    /// Target dimensionality (number of principal components). Default 2.
    #[serde(default = "default_reduce_k")]
    pub k: usize,
    /// Whiten: rescale each component to unit variance (decorrelated, equal-scale
    /// coords — useful as a preprocessing step). Default false.
    #[serde(default)]
    pub whiten: bool,
    /// Fields to exclude from the feature set.
    #[serde(default)]
    pub exclude: Vec<String>,
}
pub fn default_reduce_k() -> usize { 2 }

/// Result of PCA dimensionality reduction.
#[derive(Debug)]
pub struct ReduceResult {
    pub base: String,
    pub ids: Vec<String>,
    pub coords: Vec<Vec<f64>>,        // n × k reduced coordinates
    pub components: Vec<Vec<f64>>,    // k × dim loadings (principal directions)
    pub explained: Vec<f64>,          // per-PC explained-variance ratio
    pub cumulative: f64,
    pub reconstruction_rmse: f64,
    pub variance_retained: f64,
    pub feature_names: Vec<String>,
    pub notes: Vec<String>,
}

/// PCA via the top-k eigenvectors of the standardized-feature covariance
/// (power iteration + Gram-Schmidt deflation — same eigensolver family as
/// `/cluster`, but on the covariance so the *largest* eigenvalues dominate, no
/// shift). Returns the components, each record's reduced coordinates, the
/// explained-variance ratios, and the reconstruction error.
pub fn pca_reduce(
    engine: &Engine,
    name: &str,
    k: usize,
    whiten: bool,
    exclude: &[String],
) -> Result<ReduceResult, (StatusCode, String)> {
    use crate::types::FieldType;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "reduce requires a single base-key field".into()));
    }
    let base = schema.base_fields[0].name.clone();
    let feat_defs: Vec<crate::types::FieldDef> = schema.fiber_fields.iter()
        .filter(|f| matches!(f.field_type, FieldType::Numeric) && !exclude.iter().any(|e| e == &f.name))
        .cloned().collect();
    let records: Vec<crate::types::Record> = store.records().collect();
    let n = records.len();
    let base_of = |r: &crate::types::Record| r.get(&base).map(|v| format!("{}", v)).unwrap_or_default();
    let ids: Vec<String> = records.iter().map(&base_of).collect();
    let cols: Vec<(String, f64, f64)> = feat_defs.iter().filter_map(|fd| {
        let xs: Vec<f64> = records.iter().map(|r| r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let mu = xs.iter().sum::<f64>() / n.max(1) as f64;
        let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n.max(1) as f64).sqrt();
        (sd > f64::EPSILON).then_some((fd.name.clone(), mu, sd))
    }).collect();
    let dim = cols.len();
    if dim < 2 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
            "reduce needs >= 2 numeric fibers with non-zero variance".into()));
    }
    if n < 2 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "reduce needs >= 2 records".into()));
    }
    let kk = k.clamp(1, dim);
    let feature_names: Vec<String> = cols.iter().map(|(f, _, _)| f.clone()).collect();
    let x: Vec<Vec<f64>> = records.iter().map(|r| cols.iter()
        .map(|(f, mu, sd)| (r.get(f).and_then(|v| v.as_f64()).unwrap_or(*mu) - mu) / sd).collect()).collect();
    // covariance C = XᵀX / n (dim × dim; standardized ⇒ correlation matrix)
    let mut c = vec![vec![0.0f64; dim]; dim];
    for xi in &x { for a in 0..dim { for b in 0..dim { c[a][b] += xi[a] * xi[b]; } } }
    for a in 0..dim { for b in 0..dim { c[a][b] /= n as f64; } }
    let total_var: f64 = (0..dim).map(|t| c[t][t]).sum();
    let matvec = |c: &[Vec<f64>], v: &[f64]| -> Vec<f64> {
        (0..dim).map(|a| (0..dim).map(|b| c[a][b] * v[b]).sum()).collect() };
    // top-k eigenvectors via power iteration + deflation (largest eigenvalues)
    let mut seed: u64 = 0x2545F4914F6CDD1D;
    let mut lcg = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64 - 1.0 };
    let mut components: Vec<Vec<f64>> = Vec::with_capacity(kk);
    let mut explained: Vec<f64> = Vec::with_capacity(kk);
    for _ in 0..kk {
        let mut v: Vec<f64> = (0..dim).map(|_| lcg()).collect();
        for _ in 0..300 {
            let mut mv = matvec(&c, &v);
            for u in &components {
                let d: f64 = mv.iter().zip(u).map(|(a, b)| a * b).sum();
                for t in 0..dim { mv[t] -= d * u[t]; }
            }
            let nrm = mv.iter().map(|z| z * z).sum::<f64>().sqrt();
            if nrm < 1e-12 { break; }
            v = mv.into_iter().map(|z| z / nrm).collect();
        }
        let lam: f64 = v.iter().zip(matvec(&c, &v)).map(|(a, b)| a * b).sum();
        components.push(v);
        explained.push(if total_var > 0.0 { lam / total_var } else { 0.0 });
    }
    // reduced coords + reconstruction error (from the un-whitened projection)
    let mut coords: Vec<Vec<f64>> = x.iter().map(|xi|
        components.iter().map(|comp| (0..dim).map(|t| xi[t] * comp[t]).sum()).collect()).collect();
    let (mut sse, mut sst) = (0.0f64, 0.0f64);
    for (i, xi) in x.iter().enumerate() {
        for t in 0..dim {
            let recon: f64 = (0..kk).map(|pc| coords[i][pc] * components[pc][t]).sum();
            sse += (xi[t] - recon).powi(2);
            sst += xi[t] * xi[t];
        }
    }
    if whiten {   // rescale each component to unit variance (divide by √eigenvalue)
        let scale: Vec<f64> = (0..kk).map(|pc| (explained[pc] * total_var).max(1e-12).sqrt()).collect();
        for row in &mut coords { for pc in 0..kk { row[pc] /= scale[pc]; } }
    }
    let cumulative = explained.iter().sum();
    let mut notes = vec![format!("PCA on {dim} standardized numeric fibers → {kk} component(s){}", if whiten { ", whitened" } else { "" })];
    if k > dim { notes.push(format!("k clamped to {kk} (can't exceed the {dim} available features)")); }
    Ok(ReduceResult {
        base, ids, coords, components, explained, cumulative,
        reconstruction_rmse: (sse / (n * dim) as f64).sqrt(),
        variance_retained: if sst > 0.0 { 1.0 - sse / sst } else { 1.0 },
        feature_names, notes,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// PCA recovers the intrinsic dimension: 4 features built from 2 latent
    /// factors reduce to 2 PCs that retain ~all variance with tiny reconstruction
    /// error, and the 4 loadings-per-component are returned.
    #[test]
    fn reduce_pca_recovers_low_rank() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut s: u64 = 999;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 33) as f64 / (1u64 << 31) as f64 - 0.5 };
        for i in 0..400 {
            let (a, b) = (rnd() * 3.0, rnd() * 3.0);
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("f1", V::Float(a + 0.01 * rnd())),
                ("f2", V::Float(b + 0.01 * rnd())),
                ("f3", V::Float(a + b + 0.01 * rnd())),
                ("f4", V::Float(a - b + 0.01 * rnd())),
            ]));
        }
        let schema = BundleSchema::new("lr")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("f1")).fiber(FieldDef::numeric("f2"))
            .fiber(FieldDef::numeric("f3")).fiber(FieldDef::numeric("f4"));
        let (dir, engine) = scan_env("reduce_pca", "lr", schema, rows);
        let rr = pca_reduce(&engine, "lr", 2, false, &[]).expect("pca should build");
        assert_eq!(rr.coords.len(), 400);
        assert_eq!(rr.coords[0].len(), 2, "reduced to 2 dims");
        assert_eq!(rr.components.len(), 2);
        assert_eq!(rr.components[0].len(), 4, "each component has a loading per feature");
        assert!(rr.cumulative > 0.98, "top-2 PCs should retain ~all variance, got {}", rr.cumulative);
        assert!(rr.reconstruction_rmse < 0.1, "reconstruction should be near-perfect, RMSE={}", rr.reconstruction_rmse);
        // explained variance is sorted descending
        assert!(rr.explained[0] >= rr.explained[1], "PCs ordered by explained variance");
        cleanup(&dir);
    }

    /// Reduce endpoint gives actionable errors, not panics, on bad input.
    #[test]
    fn reduce_guards_are_actionable() {
        let rows: Vec<_> = (0..10).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("only", V::Float(i as f64)),
        ])).collect();
        let schema = BundleSchema::new("one")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("only"));
        let (dir, engine) = scan_env("reduce_guard", "one", schema, rows);
        // only 1 numeric fiber → can't reduce
        let e = pca_reduce(&engine, "one", 2, false, &[]).unwrap_err();
        assert_eq!(e.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(e.1.contains(">= 2 numeric"), "got: {}", e.1);
        // missing bundle → 404
        assert_eq!(pca_reduce(&engine, "nope", 2, false, &[]).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
