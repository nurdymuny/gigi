//! CLUSTER — spectral / k-means / GMM / DBSCAN clustering over the numeric fibers.
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Request for `POST /v1/bundles/{name}/cluster`.
#[derive(Deserialize)]
pub struct ClusterRequest {
    /// "spectral" (default), "kmeans", or "dbscan".
    #[serde(default = "default_cluster_method")]
    pub method: String,
    /// Number of clusters (spectral/kmeans; = embedding dim). Default 3.
    #[serde(default = "default_cluster_k")]
    pub k: usize,
    /// k-NN graph degree for spectral. Default 10.
    #[serde(default = "default_cluster_neighbors")]
    pub neighbors: usize,
    /// DBSCAN neighborhood radius. Omitted → auto-estimated from the data.
    #[serde(default)]
    pub eps: Option<f64>,
    /// DBSCAN core-point threshold (min points in an ε-neighborhood). Default 4.
    #[serde(default = "default_cluster_min_pts")]
    pub min_pts: usize,
    /// GMM covariance: "full" (default), "diagonal", or "spherical". Diagonal/
    /// spherical are much faster on high-dimensional data.
    #[serde(default = "default_covariance")]
    pub covariance: String,
    /// Max EM iterations (gmm) / restarts hint. Default 100 for gmm.
    #[serde(default = "default_max_iters")]
    pub max_iters: usize,
    /// k-means / gmm-init restarts (keep the lowest-inertia). Omitted → adaptive.
    #[serde(default)]
    pub restarts: Option<usize>,
    /// Spectral: use the symmetric-normalized Laplacian (often cleaner clusters).
    #[serde(default)]
    pub normalized: bool,
    /// Spectral: number of eigenvectors to embed with (default k). A richer eigenmap
    /// sharpens the clustering — fitting a GMM head in a 20-D Laplacian eigenspace
    /// separates manifold-tangled clusters far better than in the raw features.
    #[serde(default)]
    pub embed_dim: Option<usize>,
    /// Spectral head over the eigenmap: "kmeans" (default) or "gmm".
    #[serde(default = "default_spectral_head")]
    pub head: String,
    /// Fields to keep out of the geometry (e.g. an id or label column).
    #[serde(default)]
    pub exclude: Vec<String>,
}
pub fn default_spectral_head() -> String { "kmeans".to_string() }
pub fn default_cluster_method() -> String { "spectral".to_string() }
pub fn default_cluster_k() -> usize { 3 }
pub fn default_cluster_neighbors() -> usize { 10 }
pub fn default_cluster_min_pts() -> usize { 4 }
pub fn default_covariance() -> String { "full".to_string() }
pub fn default_max_iters() -> usize { 100 }

/// Advanced clustering knobs — carried together so casual callers pass one value
/// and power users tune it. `Default` reproduces the plain, sensible behavior.
#[derive(Clone)]
pub struct ClusterOpts {
    pub covariance: String,        // gmm: "full" | "diagonal" | "spherical"
    pub max_iters: usize,          // gmm EM cap
    pub restarts: Option<usize>,   // kmeans / gmm-init restarts (None = adaptive)
    pub normalized: bool,          // spectral: symmetric-normalized Laplacian
    pub embed_dim: Option<usize>,  // spectral: eigenvectors to embed with (None = k)
    pub head: String,              // spectral head over the embedding: "kmeans" | "gmm"
}
impl Default for ClusterOpts {
    fn default() -> Self { Self { covariance: "full".into(), max_iters: 100, restarts: None,
        normalized: false, embed_dim: None, head: "kmeans".into() } }
}

/// Full-covariance GMM (EM, k-means init) returning hard labels — a compact head
/// for spectral clustering, where fitting Gaussians in the Laplacian eigenspace
/// (rather than the ambient features) separates manifold-tangled clusters cleanly.
pub fn gmm_labels(pts: &[Vec<f64>], k: usize) -> Vec<usize> {
    let n = pts.len();
    let d = pts.first().map(|p| p.len()).unwrap_or(0);
    if d == 0 || n < k { return vec![0; n]; }
    let init = kmeans_lloyd(pts, k, None);
    let mut mu = vec![vec![0.0f64; d]; k];
    let mut cnt = vec![0usize; k];
    for i in 0..n { cnt[init[i]] += 1; for t in 0..d { mu[init[i]][t] += pts[i][t]; } }
    for c in 0..k { if cnt[c] > 0 { for t in 0..d { mu[c][t] /= cnt[c] as f64; } } }
    let gmean: Vec<f64> = (0..d).map(|t| pts.iter().map(|p| p[t]).sum::<f64>() / n as f64).collect();
    let mut gcov = vec![vec![0.0f64; d]; d];
    for p in pts { for a in 0..d { for b in 0..d { gcov[a][b] += (p[a] - gmean[a]) * (p[b] - gmean[b]); } } }
    for a in 0..d { for b in 0..d { gcov[a][b] /= n as f64; } }
    let mut sig = vec![gcov; k];
    let mut pi = vec![1.0 / k as f64; k];
    let mut resp = vec![vec![0.0f64; k]; n];
    let ln2pi = (2.0 * std::f64::consts::PI).ln();
    let reg = 1e-4;
    for _ in 0..60 {
        let mut inv = Vec::with_capacity(k);
        let mut logdet = vec![0.0f64; k];
        for c in 0..k {
            let mut s = sig[c].clone();
            for a in 0..d { s[a][a] += reg; }
            match mat_inv_logdet(&s) {
                Some((iv, ld)) => { inv.push(iv); logdet[c] = ld; }
                None => { inv.push((0..d).map(|i| (0..d).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect()); logdet[c] = 0.0; }
            }
        }
        for i in 0..n {
            let mut lp = vec![0.0f64; k];
            for c in 0..k {
                let dv: Vec<f64> = (0..d).map(|t| pts[i][t] - mu[c][t]).collect();
                let maha: f64 = (0..d).map(|a| (0..d).map(|b| dv[a] * inv[c][a][b] * dv[b]).sum::<f64>()).sum();
                lp[c] = pi[c].max(1e-12).ln() - 0.5 * (d as f64 * ln2pi + logdet[c] + maha);
            }
            let mx = lp.iter().cloned().fold(f64::MIN, f64::max);
            let s = mx + lp.iter().map(|l| (l - mx).exp()).sum::<f64>().ln();
            for c in 0..k { resp[i][c] = (lp[c] - s).exp(); }
        }
        for c in 0..k {
            let nc: f64 = (0..n).map(|i| resp[i][c]).sum::<f64>().max(1e-9);
            pi[c] = nc / n as f64;
            for t in 0..d { mu[c][t] = (0..n).map(|i| resp[i][c] * pts[i][t]).sum::<f64>() / nc; }
            for a in 0..d { for b in 0..d {
                sig[c][a][b] = (0..n).map(|i| resp[i][c] * (pts[i][a] - mu[c][a]) * (pts[i][b] - mu[c][b])).sum::<f64>() / nc;
            } }
        }
    }
    (0..n).map(|i| (0..k).max_by(|&a, &b| resp[i][a].partial_cmp(&resp[i][b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap()).collect()
}

/// Result of spectral clustering — shared by the handler and tests.
#[derive(Debug)]
pub struct ClusterResult {
    pub base: String,
    pub ids: Vec<String>,
    pub labels: Vec<i64>,        // cluster index, or -1 for noise (dbscan)
    pub coords: Vec<Vec<f64>>,   // embedding (spectral) / feature coords (kmeans/dbscan)
    pub eigenvalues: Vec<f64>,
    pub sizes: Vec<usize>,       // size of each non-noise cluster
    pub n_noise: usize,
    pub method: String,
    pub notes: Vec<String>,
}

/// Invert a square matrix and return (inverse, ln|det|) via Gauss-Jordan with
/// partial pivoting on [M | I]. None if singular. Used by the GMM density scorer.
pub fn mat_inv_logdet(m: &[Vec<f64>]) -> Option<(Vec<Vec<f64>>, f64)> {
    let n = m.len();
    let mut a: Vec<Vec<f64>> = (0..n).map(|i| {
        let mut row = m[i].clone();
        row.extend((0..n).map(|j| if i == j { 1.0 } else { 0.0 }));
        row
    }).collect();
    let mut logdet = 0.0;
    for c in 0..n {
        let piv = (c..n).max_by(|&r1, &r2| a[r1][c].abs()
            .partial_cmp(&a[r2][c].abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        if a[piv][c].abs() < 1e-12 { return None; }
        a.swap(c, piv);
        logdet += a[c][c].abs().ln();
        let piv_val = a[c][c];
        for r in 0..n {
            if r != c {
                let f = a[r][c] / piv_val;
                for k in c..2 * n { a[r][k] -= f * a[c][k]; }
            }
        }
    }
    let inv: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| a[i][n + j] / a[i][i]).collect()).collect();
    Some((inv, logdet))
}

/// k-means with proper k-means++ (D²-weighted) initialization and multiple
/// restarts, keeping the lowest-inertia solution — matching the quality of a
/// standard library on hard (many-cluster / high-dim) problems, where single-shot
/// farthest-point init lands in bad local optima. Deterministic via a fixed LCG.
/// Shared by the `kmeans` method, the spectral eigenmap head, and GMM init.
pub fn kmeans_lloyd(pts: &[Vec<f64>], k: usize, restarts_opt: Option<usize>) -> Vec<usize> {
    let n = pts.len();
    let d = pts.first().map(|p| p.len()).unwrap_or(0);
    let dist2 = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| (p - q) * (p - q)).sum::<f64>();
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut unif = move || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 11) as f64 / (1u64 << 53) as f64 };   // in [0,1)
    // bound total work: fewer restarts on large problems (still capped at n<=8000 upstream)
    let restarts = restarts_opt.map(|r| r.clamp(1, 50))
        .unwrap_or(if n * k > 10_000 { 3 } else { 6 });
    let mut best_labels = vec![0usize; n];
    let mut best_inertia = f64::MAX;
    for _ in 0..restarts {
        // ── k-means++ init: first center uniform, rest sampled ∝ D²(x) ──
        let mut cen: Vec<Vec<f64>> = vec![pts[(unif() * n as f64) as usize % n].clone()];
        while cen.len() < k {
            let d2: Vec<f64> = pts.iter().map(|p| cen.iter().map(|c| dist2(p, c)).fold(f64::MAX, f64::min)).collect();
            let sum: f64 = d2.iter().sum();
            let pick = if sum <= 0.0 { (unif() * n as f64) as usize % n } else {
                let mut r = unif() * sum;
                let mut idx = n - 1;
                for (i, &w) in d2.iter().enumerate() { r -= w; if r <= 0.0 { idx = i; break; } }
                idx
            };
            cen.push(pts[pick].clone());
        }
        // ── Lloyd ──
        let mut labels = vec![0usize; n];
        for _ in 0..50 {
            let mut changed = false;
            for i in 0..n {
                let c = (0..k).min_by(|&a, &b| dist2(&pts[i], &cen[a])
                    .partial_cmp(&dist2(&pts[i], &cen[b])).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
                if c != labels[i] { labels[i] = c; changed = true; }
            }
            let mut sum = vec![vec![0.0; d]; k];
            let mut cnt = vec![0usize; k];
            for i in 0..n { cnt[labels[i]] += 1; for t in 0..d { sum[labels[i]][t] += pts[i][t]; } }
            for c in 0..k { if cnt[c] > 0 { for t in 0..d { cen[c][t] = sum[c][t] / cnt[c] as f64; } } }
            if !changed { break; }
        }
        let inertia: f64 = (0..n).map(|i| dist2(&pts[i], &cen[labels[i]])).sum();
        if inertia < best_inertia { best_inertia = inertia; best_labels = labels; }
    }
    best_labels
}

/// Cluster records by one of three geometric methods:
///   • `spectral` — bottom-k eigenvectors of the k-NN graph Laplacian L = D − A
///     (shifted power iteration + Gram-Schmidt deflation) → R^k embedding →
///     k-means head; the embedding is the Laplacian-Eigenmaps manifold layout.
///   • `kmeans`   — k-means directly on the standardized numeric fibers.
///   • `dbscan`   — density clusters over an ε-neighborhood graph, with noise
///     (label −1); ε auto-estimated from the minPts-NN distance if not given.
/// Deterministic (fixed init) so results are reproducible.
pub fn cluster_records(
    engine: &Engine,
    name: &str,
    method: &str,
    k: usize,
    neighbors: usize,
    eps: Option<f64>,
    min_pts: usize,
    opts: &ClusterOpts,
    exclude: &[String],
) -> Result<ClusterResult, (StatusCode, String)> {
    use crate::types::FieldType;
    const CLUSTER_MAX_N: usize = 8000;   // exact-neighbor cost cap
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "cluster requires a single base-key field".into()));
    }
    let base = schema.base_fields[0].name.clone();
    if !matches!(method, "spectral" | "kmeans" | "dbscan" | "gmm") {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "unknown cluster method '{}' (expected 'spectral', 'kmeans', 'dbscan', or 'gmm')", method)));
    }
    if method != "dbscan" && k < 2 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "k must be >= 2 for spectral/kmeans".into()));
    }
    let usable = |f: &crate::types::FieldDef| !exclude.iter().any(|e| e == &f.name);
    let num_defs: Vec<crate::types::FieldDef> = schema.fiber_fields.iter()
        .filter(|f| usable(f) && matches!(f.field_type, FieldType::Numeric))
        .cloned().collect();
    let records: Vec<crate::types::Record> = store.records().collect();
    let n = records.len();
    let base_of = |r: &crate::types::Record| r.get(&base).map(|v| format!("{}", v)).unwrap_or_default();
    let ids: Vec<String> = records.iter().map(&base_of).collect();
    let mut notes: Vec<String> = Vec::new();
    // standardize numeric fibers; drop zero-variance
    let cols: Vec<(String, f64, f64)> = num_defs.iter().filter_map(|fd| {
        let xs: Vec<f64> = records.iter().map(|r| r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let mu = xs.iter().sum::<f64>() / n.max(1) as f64;
        let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n.max(1) as f64).sqrt();
        (sd > f64::EPSILON).then_some((fd.name.clone(), mu, sd))
    }).collect();
    let dim = cols.len();
    if dim < 1 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "cluster needs >= 1 numeric fiber with non-zero variance to build the neighbor graph (bundle '{}' has none usable)", name)));
    }
    if method != "dbscan" && n < 2 * k {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "cluster needs at least 2·k = {} records to form {} clusters (bundle has {})", 2 * k, k, n)));
    }
    if n > CLUSTER_MAX_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "cluster: {n} records exceeds the exact-neighbor limit ({CLUSTER_MAX_N}) in this version")));
    }
    let x: Vec<Vec<f64>> = records.iter().map(|r| cols.iter()
        .map(|(f, mu, sd)| (r.get(f).and_then(|v| v.as_f64()).unwrap_or(*mu) - mu) / sd).collect()).collect();
    let dist2 = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| (p - q) * (p - q)).sum::<f64>();

    // ── k-means: partition the standardized features directly (no graph) ──
    if method == "kmeans" {
        let labels: Vec<i64> = kmeans_lloyd(&x, k, opts.restarts).into_iter().map(|l| l as i64).collect();
        let mut sizes = vec![0usize; k];
        for &l in &labels { sizes[l as usize] += 1; }
        let kd = k.min(dim);
        let coords: Vec<Vec<f64>> = x.iter().map(|xi| xi[..kd].to_vec()).collect();
        notes.push(format!("k-means on {dim} numeric fibers into k={k} clusters (deterministic k-means++ init, Lloyd)"));
        return Ok(ClusterResult { base, ids, labels, coords, eigenvalues: Vec::new(),
            sizes, n_noise: 0, method: method.to_string(), notes });
    }

    // ── GMM: full-covariance Gaussian mixture, fit by EM (soft clustering) ──
    if method == "gmm" {
        let d = dim;
        // init from k-means (robust — avoids EM local optima on elongated clusters):
        // component means, per-component covariance, and mixing weights
        let init = kmeans_lloyd(&x, k, opts.restarts);
        let cov_diag = opts.covariance == "diagonal" || opts.covariance == "spherical";
        let spherical = opts.covariance == "spherical";
        if !matches!(opts.covariance.as_str(), "full" | "diagonal" | "spherical") {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
                "unknown covariance '{}' (expected 'full', 'diagonal', or 'spherical')", opts.covariance)));
        }
        let em_iters = opts.max_iters.clamp(1, 1000);
        let mut mu = vec![vec![0.0f64; d]; k];
        let mut cnt = vec![0usize; k];
        for i in 0..n { cnt[init[i]] += 1; for t in 0..d { mu[init[i]][t] += x[i][t]; } }
        for c in 0..k { if cnt[c] > 0 { for t in 0..d { mu[c][t] /= cnt[c] as f64; } } }
        let gmean: Vec<f64> = (0..d).map(|t| x.iter().map(|xi| xi[t]).sum::<f64>() / n as f64).collect();
        let mut gcov = vec![vec![0.0f64; d]; d];
        for xi in &x { for a in 0..d { for b in 0..d { gcov[a][b] += (xi[a] - gmean[a]) * (xi[b] - gmean[b]); } } }
        for a in 0..d { for b in 0..d { gcov[a][b] /= n as f64; } }
        let mut sig: Vec<Vec<Vec<f64>>> = vec![gcov.clone(); k];
        for c in 0..k {
            if cnt[c] > d {  // enough points to estimate a component covariance
                let mut sc = vec![vec![0.0f64; d]; d];
                for i in 0..n { if init[i] == c {
                    for a in 0..d { for b in 0..d { sc[a][b] += (x[i][a] - mu[c][a]) * (x[i][b] - mu[c][b]); } }
                } }
                for a in 0..d { for b in 0..d { sc[a][b] /= cnt[c] as f64; } }
                sig[c] = sc;
            }
        }
        // diagonal-variance representation (used for "diagonal"/"spherical")
        let mut var: Vec<Vec<f64>> = if cov_diag {
            (0..k).map(|c| {
                let mut v: Vec<f64> = (0..d).map(|t| sig[c][t][t].max(1e-6)).collect();
                if spherical { let m = v.iter().sum::<f64>() / d as f64; v = vec![m.max(1e-6); d]; }
                v
            }).collect()
        } else { Vec::new() };
        let mut pi: Vec<f64> = (0..k).map(|c| cnt[c].max(1) as f64 / n as f64).collect();
        let mut resp = vec![vec![0.0f64; k]; n];
        let ln2pi = (2.0 * std::f64::consts::PI).ln();
        let reg = 1e-6;
        let mut prev_ll = f64::NEG_INFINITY;
        let mut iters_run = 0;
        for _ in 0..em_iters {
            iters_run += 1;
            // precompute per-component ln|Σ| (+ Σ⁻¹ for the full path)
            let mut inv: Vec<Vec<Vec<f64>>> = Vec::new();
            let mut logdet = vec![0.0f64; k];
            if cov_diag {
                for c in 0..k { logdet[c] = (0..d).map(|t| (var[c][t] + reg).ln()).sum(); }
            } else {
                inv.reserve(k);
                for c in 0..k {
                    let mut s = sig[c].clone();
                    for a in 0..d { s[a][a] += reg; }
                    match mat_inv_logdet(&s) {
                        Some((iv, ld)) => { inv.push(iv); logdet[c] = ld; }
                        None => { inv.push((0..d).map(|i| (0..d).map(|j| if i == j { 1.0 } else { 0.0 }).collect()).collect()); logdet[c] = 0.0; }
                    }
                }
            }
            // E-step: responsibilities via log-sum-exp; accumulate total log-likelihood
            let mut ll = 0.0;
            for i in 0..n {
                let mut lp = vec![0.0f64; k];
                for c in 0..k {
                    let dv: Vec<f64> = (0..d).map(|t| x[i][t] - mu[c][t]).collect();
                    let maha: f64 = if cov_diag {
                        (0..d).map(|t| dv[t] * dv[t] / (var[c][t] + reg)).sum()
                    } else {
                        (0..d).map(|a| (0..d).map(|b| dv[a] * inv[c][a][b] * dv[b]).sum::<f64>()).sum()
                    };
                    lp[c] = pi[c].max(1e-12).ln() - 0.5 * (d as f64 * ln2pi + logdet[c] + maha);
                }
                let mx = lp.iter().cloned().fold(f64::MIN, f64::max);
                let s = mx + lp.iter().map(|l| (l - mx).exp()).sum::<f64>().ln();
                ll += s;
                for c in 0..k { resp[i][c] = (lp[c] - s).exp(); }
            }
            // converged? (relative improvement in total log-likelihood is tiny)
            if (ll - prev_ll).abs() < 1e-7 * ll.abs().max(1.0) { break; }
            prev_ll = ll;
            // M-step
            for c in 0..k {
                let nc: f64 = (0..n).map(|i| resp[i][c]).sum::<f64>().max(1e-9);
                pi[c] = nc / n as f64;
                for t in 0..d { mu[c][t] = (0..n).map(|i| resp[i][c] * x[i][t]).sum::<f64>() / nc; }
                if cov_diag {
                    for t in 0..d { var[c][t] = (0..n).map(|i| resp[i][c] * (x[i][t] - mu[c][t]).powi(2)).sum::<f64>() / nc; }
                    if spherical { let m = var[c].iter().sum::<f64>() / d as f64; for t in 0..d { var[c][t] = m; } }
                } else {
                    for a in 0..d { for b in 0..d {
                        sig[c][a][b] = (0..n).map(|i| resp[i][c] * (x[i][a] - mu[c][a]) * (x[i][b] - mu[c][b])).sum::<f64>() / nc;
                    } }
                }
            }
        }
        let labels: Vec<i64> = (0..n).map(|i| (0..k).max_by(|&a, &b| resp[i][a]
            .partial_cmp(&resp[i][b]).unwrap_or(std::cmp::Ordering::Equal)).unwrap() as i64).collect();
        let mut sizes = vec![0usize; k];
        for &l in &labels { sizes[l as usize] += 1; }
        notes.push(format!("GMM ({} covariance, {k} components, EM converged in {iters_run} iterations) on {dim} numeric fibers; coords = soft responsibilities", opts.covariance));
        return Ok(ClusterResult { base, ids, labels, coords: resp, eigenvalues: Vec::new(),
            sizes, n_noise: 0, method: method.to_string(), notes });
    }

    // ── DBSCAN: density clusters + noise over an ε-neighborhood graph ──
    if method == "dbscan" {
        let mp = min_pts.max(2);
        // auto-ε: 75th percentile of each point's distance to its mp-th neighbor
        let epsv = if let Some(e) = eps { e } else {
            let mut kd: Vec<f64> = (0..n).map(|i| {
                let mut ds: Vec<f64> = (0..n).filter(|&j| j != i).map(|j| dist2(&x[i], &x[j])).collect();
                let idx = (mp - 1).min(ds.len().saturating_sub(1));
                ds.select_nth_unstable_by(idx, |a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                ds[idx].sqrt()
            }).collect();
            kd.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let e = kd[((0.75 * (n - 1) as f64) as usize).min(n - 1)];
            notes.push(format!("ε auto-estimated at {:.4} (75th pct of {mp}-NN distance)", e));
            e.max(f64::EPSILON)
        };
        let eps2 = epsv * epsv;
        let nbhd: Vec<Vec<usize>> = (0..n).map(|i|
            (0..n).filter(|&j| dist2(&x[i], &x[j]) <= eps2).collect()).collect();
        let mut labels = vec![-2i64; n];   // -2 unvisited, -1 noise, >=0 cluster
        let mut cid: i64 = 0;
        for p in 0..n {
            if labels[p] != -2 { continue; }
            if nbhd[p].len() < mp { labels[p] = -1; continue; }   // not core → provisional noise
            labels[p] = cid;
            let mut seeds = nbhd[p].clone();
            let mut qi = 0;
            while qi < seeds.len() {
                let q = seeds[qi]; qi += 1;
                if labels[q] == -1 { labels[q] = cid; }           // border: absorb former noise
                if labels[q] != -2 { continue; }
                labels[q] = cid;
                if nbhd[q].len() >= mp { seeds.extend_from_slice(&nbhd[q]); }  // core → expand
            }
            cid += 1;
        }
        let n_clusters = cid as usize;
        let mut sizes = vec![0usize; n_clusters];
        let mut n_noise = 0;
        for &l in &labels { if l < 0 { n_noise += 1; } else { sizes[l as usize] += 1; } }
        let kd = 3.min(dim);
        let coords: Vec<Vec<f64>> = x.iter().map(|xi| xi[..kd].to_vec()).collect();
        notes.push(format!("DBSCAN on {dim} numeric fibers: ε={epsv:.4}, minPts={mp} → {n_clusters} cluster(s), {n_noise} noise"));
        return Ok(ClusterResult { base, ids, labels, coords, eigenvalues: Vec::new(),
            sizes, n_noise, method: method.to_string(), notes });
    }

    // ── spectral (default): needs the k-NN graph ──
    let deg_k = neighbors.clamp(2, n - 1);
    if deg_k != neighbors {
        notes.push(format!("neighbors clamped to {deg_k} (must be in [2, n-1])"));
    }
    // ── symmetric k-NN adjacency: edge if j∈kNN(i) OR i∈kNN(j) ──
    let mut adj: Vec<std::collections::BTreeSet<usize>> = vec![std::collections::BTreeSet::new(); n];
    for i in 0..n {
        let mut d: Vec<(f64, usize)> = (0..n).filter(|&j| j != i).map(|j| (dist2(&x[i], &x[j]), j)).collect();
        d.select_nth_unstable_by(deg_k - 1, |a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        for (_, j) in &d[..deg_k] { adj[i].insert(*j); adj[*j].insert(i); }
    }
    let adjv: Vec<Vec<usize>> = adj.into_iter().map(|s| s.into_iter().collect()).collect();
    let deg: Vec<f64> = adjv.iter().map(|a| a.len() as f64).collect();
    // connected components (union-find) — the β0 structure of the neighbor graph
    let comp_id: Vec<usize> = {
        let mut par: Vec<usize> = (0..n).collect();
        fn find(p: &mut Vec<usize>, mut x: usize) -> usize { while p[x] != x { p[x] = p[p[x]]; x = p[x]; } x }
        for i in 0..n { for &j in &adjv[i] { let (a, b) = (find(&mut par, i), find(&mut par, j)); par[a] = b; } }
        (0..n).map(|i| find(&mut par, i)).collect()
    };
    let ncomp = comp_id.iter().collect::<std::collections::HashSet<_>>().len();
    let dist2c = &dist2;

    let (labels, coords, eigenvalues) = if ncomp >= k {
        // ── well-separated: the graph already splits into ≥ k pieces, so a spectral
        // cut is unnecessary (and power iteration converges slowly on the degenerate
        // null space). Cluster by component: the k largest are seeds, smaller
        // components join the nearest seed by feature centroid. Exact and fast. ──
        let mut members: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for i in 0..n { members.entry(comp_id[i]).or_default().push(i); }
        let mut by_size: Vec<(usize, usize)> = members.iter().map(|(c, m)| (*c, m.len())).collect();
        by_size.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let centroid = |m: &[usize]| -> Vec<f64> {
            let mut c = vec![0.0; dim];
            for &i in m { for t in 0..dim { c[t] += x[i][t]; } }
            for t in 0..dim { c[t] /= m.len() as f64; }
            c
        };
        let seeds: Vec<usize> = by_size.iter().take(k).map(|(c, _)| *c).collect();
        let seed_cent: Vec<Vec<f64>> = seeds.iter().map(|c| centroid(&members[c])).collect();
        let seed_of: std::collections::HashMap<usize, usize> = members.keys().map(|c| {
            if let Some(pos) = seeds.iter().position(|s| s == c) { (*c, pos) }
            else {
                let cc = centroid(&members[c]);
                let pos = (0..k).min_by(|&a, &b| dist2c(&cc, &seed_cent[a])
                    .partial_cmp(&dist2c(&cc, &seed_cent[b])).unwrap_or(std::cmp::Ordering::Equal)).unwrap();
                (*c, pos)
            }
        }).collect();
        let labels: Vec<usize> = (0..n).map(|i| seed_of[&comp_id[i]]).collect();
        // exact Laplacian-eigenmap for disconnected components = cluster indicator
        let coords: Vec<Vec<f64>> = (0..n).map(|i| (0..k)
            .map(|c| if c == labels[i] { 1.0 } else { 0.0 }).collect()).collect();
        notes.push(format!("{ncomp} connected component(s) >= k={k}: clustered by components (no spectral cut needed)"));
        (labels, coords, vec![0.0; k])
    } else {
        // ── connected graph: spectral cut via bottom-k Laplacian eigenmaps ──
        // B = cI − L so L's smallest eigenpairs are B's largest (power iteration).
        // `normalized` uses the symmetric-normalized Laplacian L_sym = I − D^{-½}AD^{-½}
        // (eigenvalues in [0,2]; often cleaner clusters, per Ng-Jordan-Weiss).
        let dsqrt: Vec<f64> = deg.iter().map(|d| if *d > 0.0 { d.sqrt() } else { 1.0 }).collect();
        let matvec_l = |v: &[f64], i: usize| -> f64 {
            if opts.normalized {
                v[i] - adjv[i].iter().map(|&j| v[j] / (dsqrt[i] * dsqrt[j])).sum::<f64>()
            } else {
                deg[i] * v[i] - adjv[i].iter().map(|&j| v[j]).sum::<f64>()
            }
        };
        let shift = if opts.normalized { 2.0 } else { 2.0 * deg.iter().cloned().fold(0.0, f64::max) + 1.0 };
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        let mut lcg = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (seed >> 33) as f64 / (1u64 << 31) as f64 - 1.0 };
        let iters = 300;
        // embed with `embed_dim` eigenvectors (default k). A richer eigenmap gives a
        // GMM/k-means head more room and often sharpens the clustering.
        let ed = opts.embed_dim.unwrap_or(k).clamp(k, (n - 1).min(64));
        let mut vecs: Vec<Vec<f64>> = Vec::with_capacity(ed);
        let mut eigenvalues: Vec<f64> = Vec::with_capacity(ed);
        for _ in 0..ed {
            let mut v: Vec<f64> = (0..n).map(|_| lcg()).collect();
            for _ in 0..iters {
                let bv: Vec<f64> = (0..n).map(|i| shift * v[i] - matvec_l(&v, i)).collect();
                v = bv;
                for u in &vecs {  // deflate against previously found eigenvectors
                    let d: f64 = v.iter().zip(u).map(|(a, b)| a * b).sum();
                    for i in 0..n { v[i] -= d * u[i]; }
                }
                let nrm = v.iter().map(|z| z * z).sum::<f64>().sqrt();
                if nrm < 1e-12 { break; }
                for z in &mut v { *z /= nrm; }
            }
            let lam: f64 = (0..n).map(|i| v[i] * matvec_l(&v, i)).sum();
            vecs.push(v);
            eigenvalues.push((lam * 10000.0).round() / 10000.0);
        }
        let mut coords: Vec<Vec<f64>> = (0..n).map(|i| (0..ed).map(|c| vecs[c][i]).collect()).collect();
        if opts.normalized {   // row-normalize the embedding (Ng-Jordan-Weiss)
            for row in &mut coords {
                let nrm = row.iter().map(|z| z * z).sum::<f64>().sqrt();
                if nrm > 1e-12 { for z in row.iter_mut() { *z /= nrm; } }
            }
        }
        // head over the eigenmap: k-means (default) or a full-covariance GMM
        let gmm_head = opts.head == "gmm";
        let labels = if gmm_head { gmm_labels(&coords, k) } else { kmeans_lloyd(&coords, k, opts.restarts) };
        notes.push(format!("spectral clustering on {dim} numeric fibers: symmetric {deg_k}-NN graph, bottom-{ed} {}Laplacian eigenmaps + {} head",
            if opts.normalized { "normalized " } else { "" }, if gmm_head { "GMM" } else { "k-means" }));
        (labels, coords, eigenvalues)
    };
    let labels: Vec<i64> = labels.into_iter().map(|l| l as i64).collect();
    let mut sizes = vec![0usize; k];
    for &l in &labels { sizes[l as usize] += 1; }
    Ok(ClusterResult { base, ids, labels, coords, eigenvalues, sizes,
        n_noise: 0, method: method.to_string(), notes })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// Spectral clustering recovers well-separated groups: three tight blobs in
    /// 2-D fiber space are split cleanly into three clusters by the Laplacian
    /// eigenmap + k-means head — a job the axis-wise/anomaly lenses don't do.
    #[test]
    fn cluster_spectral_recovers_blobs() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for (b, (ox, oy)) in [(0.0, 0.0), (10.0, 0.0), (5.0, 9.0)].iter().enumerate() {
            for j in 0..20 {
                let a = j as f64 * 0.31;   // deterministic spread within the blob
                rows.push(scan_rec(&[
                    ("id", V::Text(format!("b{b}_{j}"))),
                    ("x", V::Float(ox + 0.4 * a.cos())),
                    ("y", V::Float(oy + 0.4 * a.sin())),
                ]));
            }
        }
        let schema = BundleSchema::new("blob3")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("x"))
            .fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("cluster_blobs", "blob3", schema, rows);
        // all three partitioning methods must recover the three blobs
        for method in ["spectral", "kmeans"] {
            let cr = cluster_records(&engine, "blob3", method, 3, 6, None, 4, &ClusterOpts::default(), &[]).expect("cluster should build");
            assert_eq!(cr.labels.len(), 60, "{method}");
            assert_eq!(cr.sizes.iter().sum::<usize>(), 60, "{method}");
            // records() need not preserve insertion order, so map each result back to
            // its true blob via the returned id ("b{blob}_{j}"). Every blob's members
            // must share one cluster, and the three blobs occupy three distinct clusters.
            let mut blob_cluster = [-999i64; 3];
            for (pos, id) in cr.ids.iter().enumerate() {
                let b: usize = id[1..id.find('_').unwrap()].parse().unwrap();
                if blob_cluster[b] == -999 { blob_cluster[b] = cr.labels[pos]; }
                else { assert_eq!(cr.labels[pos], blob_cluster[b], "{method}: blob {b} split across clusters"); }
            }
            let distinct: std::collections::HashSet<i64> = blob_cluster.iter().copied().collect();
            assert_eq!(distinct.len(), 3, "{method}: three blobs should occupy three distinct clusters, got {:?}", blob_cluster);
        }
        cleanup(&dir);
    }

    /// GMM (full-covariance EM) recovers two well-separated clusters — each true
    /// cluster lands in one component, and the soft responsibilities per record
    /// sum to ~1 (exercising the E-step's log-sum-exp normalization).
    #[test]
    fn cluster_gmm_recovers_and_normalizes() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut s: u64 = 77;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 33) as f64 / (1u64 << 31) as f64 - 0.5 };
        for (c, (ox, oy)) in [(0usize, (0.0, 0.0)), (1usize, (7.0, 7.0))] {
            for _ in 0..40 {
                rows.push(scan_rec(&[
                    ("id", V::Text(format!("g{c}_{}", rows.len()))),
                    ("x", V::Float(ox + rnd() * 1.2)),
                    ("y", V::Float(oy + rnd() * 1.2)),
                ]));
            }
        }
        let schema = BundleSchema::new("ell")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("x")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("cluster_gmm", "ell", schema, rows);
        let cr = cluster_records(&engine, "ell", "gmm", 2, 10, None, 4, &ClusterOpts::default(), &[]).expect("gmm should build");
        assert_eq!(cr.sizes.iter().sum::<usize>(), 80);
        // soft responsibilities (coords) sum to ~1 per record
        for row in &cr.coords {
            let tot: f64 = row.iter().sum();
            assert!((tot - 1.0).abs() < 1e-6, "responsibilities should sum to 1, got {tot}");
        }
        // each true cluster (id prefix "g0"/"g1") lands in a single component
        let mut cl = [-1i64; 2];
        for (pos, id) in cr.ids.iter().enumerate() {
            let c: usize = id[1..2].parse().unwrap();
            if cl[c] == -1 { cl[c] = cr.labels[pos]; }
            else { assert_eq!(cr.labels[pos], cl[c], "cluster {c} split across GMM components"); }
        }
        assert_ne!(cl[0], cl[1], "the two clusters should map to two distinct components");
        // all three covariance types recover the two blobs without NaN
        for cov in ["full", "diagonal", "spherical"] {
            let opts = ClusterOpts { covariance: cov.to_string(), ..Default::default() };
            let c = cluster_records(&engine, "ell", "gmm", 2, 10, None, 4, &opts, &[]).unwrap();
            assert_eq!(c.sizes.iter().filter(|&&s| s > 0).count(), 2, "{cov}: should find 2 non-empty clusters");
            for row in &c.coords { assert!(row.iter().all(|v| v.is_finite()), "{cov}: no NaN in responsibilities"); }
        }
        // an unknown covariance type is a clean error
        let badopts = ClusterOpts { covariance: "banana".to_string(), ..Default::default() };
        assert!(cluster_records(&engine, "ell", "gmm", 2, 10, None, 4, &badopts, &[]).is_err());
        // spectral with the normalized Laplacian also runs and clusters cleanly
        let nopts = ClusterOpts { normalized: true, ..Default::default() };
        let sc = cluster_records(&engine, "ell", "spectral", 2, 10, None, 4, &nopts, &[]).unwrap();
        assert_eq!(sc.sizes.iter().sum::<usize>(), 80);
        // spectral with a GMM head over a richer (embed_dim) eigenmap also runs cleanly
        let hopts = ClusterOpts { head: "gmm".into(), embed_dim: Some(4), ..Default::default() };
        let hc = cluster_records(&engine, "ell", "spectral", 2, 10, None, 4, &hopts, &[]).unwrap();
        assert_eq!(hc.sizes.iter().sum::<usize>(), 80);
        assert!(hc.coords.iter().all(|r| r.iter().all(|v| v.is_finite())), "GMM head: finite embedding");
        cleanup(&dir);
    }

    /// DBSCAN recovers two dense blobs and marks a lone point as NOISE. Blobs are
    /// balanced 5×5 grids separated on the diagonal (so standardization doesn't
    /// distort them); ε is auto-estimated.
    #[test]
    fn cluster_dbscan_finds_noise() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for (b, (ox, oy)) in [(0.0, 0.0), (2.0, 2.0)].iter().enumerate() {
            for xi in 0..5 {
                for yi in 0..5 {
                    rows.push(scan_rec(&[
                        ("id", V::Text(format!("d{b}_{xi}{yi}"))),
                        ("x", V::Float(ox + 0.1 * xi as f64)),
                        ("y", V::Float(oy + 0.1 * yi as f64)),
                    ]));
                }
            }
        }
        // a lone point in the empty middle → density noise
        rows.push(scan_rec(&[("id", V::Text("noise".into())), ("x", V::Float(1.0)), ("y", V::Float(1.0))]));
        let schema = BundleSchema::new("db2")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("x")).fiber(FieldDef::numeric("y"));
        let (dir, engine) = scan_env("cluster_db", "db2", schema, rows);
        let cr = cluster_records(&engine, "db2", "dbscan", 3, 10, None, 3, &ClusterOpts::default(), &[]).expect("dbscan should build");
        let pos = cr.ids.iter().position(|i| i == "noise").unwrap();
        assert_eq!(cr.labels[pos], -1, "the lone middle point should be labeled noise (-1), got {}", cr.labels[pos]);
        // the two blobs land in two distinct non-noise clusters
        let blob_labels: std::collections::HashSet<i64> = cr.ids.iter().enumerate()
            .filter(|(_, id)| id.starts_with('d'))
            .map(|(p, _)| cr.labels[p]).filter(|&l| l >= 0).collect();
        assert_eq!(blob_labels.len(), 2, "two dense blobs should form two clusters, got {:?}", blob_labels);
        cleanup(&dir);
    }

    /// Cluster endpoint gives actionable errors, not panics, on bad input.
    #[test]
    fn cluster_guards_are_actionable() {
        let rows: Vec<_> = (0..6).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("v", V::Float(i as f64)),
        ])).collect();
        let schema = BundleSchema::new("tiny")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("v"));
        let (dir, engine) = scan_env("cluster_tiny", "tiny", schema, rows);
        // k too large for the record count → clear error
        let err = cluster_records(&engine, "tiny", "spectral", 5, 4, None, 4, &ClusterOpts::default(), &[]).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("clusters"), "error should explain the k-vs-n constraint, got: {}", err.1);
        // k < 2 → clear error
        assert!(cluster_records(&engine, "tiny", "spectral", 1, 4, None, 4, &ClusterOpts::default(), &[]).is_err());
        // unknown method → clear error
        let m = cluster_records(&engine, "tiny", "banana", 2, 4, None, 4, &ClusterOpts::default(), &[]).unwrap_err();
        assert!(m.1.contains("unknown cluster method"), "got: {}", m.1);
        // missing bundle → 404
        assert_eq!(cluster_records(&engine, "nope", "spectral", 2, 4, None, 4, &ClusterOpts::default(), &[]).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
