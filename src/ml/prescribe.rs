//! PRESCRIBE — geometric fingerprint diagnostic (curvature / dim ratio / spectral clarity).
//!
//! Extracted mechanically from `src/bin/gigi_stream.rs` (stream-extraction
//! phase 1). The HTTP handler stays in the binary as a thin wrapper.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Top-`kmax` eigenvalues of a symmetric matrix via power iteration + deflation.
/// Shared by /prescribe for both the global covariance (intrinsic dimension) and each
/// point's local covariance (curvature). Eigenvalues come out largest-first.
pub fn prescribe_top_eigs(m: &[Vec<f64>], kmax: usize, lcg: &mut dyn FnMut() -> f64) -> Vec<f64> {
    let d = m.len();
    let matvec = |v: &[f64]| -> Vec<f64> {
        (0..d).map(|a| (0..d).map(|b| m[a][b] * v[b]).sum::<f64>()).collect() };
    let mut comps: Vec<Vec<f64>> = Vec::new();
    let mut eigs: Vec<f64> = Vec::new();
    for _ in 0..kmax.min(d) {
        let mut v: Vec<f64> = (0..d).map(|_| lcg()).collect();
        for _ in 0..200 {
            let mut mv = matvec(&v);
            for u in &comps {
                let dd: f64 = mv.iter().zip(u).map(|(a, b)| a * b).sum();
                for t in 0..d { mv[t] -= dd * u[t]; }
            }
            let nrm = mv.iter().map(|z| z * z).sum::<f64>().sqrt();
            if nrm < 1e-12 { break; }
            v = mv.into_iter().map(|z| z / nrm).collect();
        }
        let lam: f64 = v.iter().zip(matvec(&v)).map(|(a, b)| a * b).sum();
        comps.push(v); eigs.push(lam.max(0.0));
    }
    eigs
}

/// Request for `POST /v1/bundles/{name}/prescribe`.
#[derive(Deserialize)]
pub struct PrescribeRequest {
    /// Neighbors used for the local-geometry probes (default 10).
    #[serde(default = "default_prescribe_neighbors")]
    pub neighbors: usize,
    /// Cap on points used for the O(n²) probes; the data is strided down to at most
    /// this many for the curvature + spectral reads (default 500).
    #[serde(default = "default_prescribe_sample")]
    pub sample: usize,
    /// Numeric fibers to leave out of the fingerprint.
    #[serde(default)]
    pub exclude: Vec<String>,
}
pub fn default_prescribe_neighbors() -> usize { 10 }
pub fn default_prescribe_sample() -> usize { 500 }

/// A geometric read of a bundle's shape — no performance prediction, only description.
#[derive(Debug)]
pub struct PrescribeResult {
    pub n: usize,
    pub ambient_dim: usize,
    pub intrinsic_dim: usize,
    pub dim_ratio: f64,
    pub curvature: f64,
    pub spectral_clarity: f64,
    pub reads: Vec<String>,
    pub recommendations: Vec<String>,
    pub feature_names: Vec<String>,
    pub notes: Vec<String>,
}

/// Compute the geometric fingerprint of a bundle: **curvature** (mean fraction of a
/// neighborhood's variance lying off its local top-2 tangent plane — 0 = locally
/// linear, high = curved/locally high-dimensional), **dim_ratio** (intrinsic dimension
/// at 90% variance ÷ ambient), and **spectral_clarity** (largest gap among the smallest
/// normalized-Laplacian eigenvalues of the k-NN graph — wide gap = clean clusters).
///
/// This is a DIAGNOSTIC: it turns the fingerprint into plain-English reads and method
/// suggestions. It makes **no** claim about how well any method will score — the gauntlet
/// showed a cheap fingerprint can't honestly predict the geometric-vs-flat gap across
/// heterogeneous data, so /prescribe describes the shape and lets you choose.
pub fn prescribe_fingerprint(
    engine: &Engine,
    name: &str,
    neighbors: usize,
    sample: usize,
    exclude: &[String],
) -> Result<PrescribeResult, (StatusCode, String)> {
    use crate::types::FieldType;
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    if schema.base_fields.is_empty() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "prescribe requires a single base-key field".into()));
    }
    let feat_defs: Vec<crate::types::FieldDef> = schema.fiber_fields.iter()
        .filter(|f| matches!(f.field_type, FieldType::Numeric) && !exclude.iter().any(|e| e == &f.name))
        .cloned().collect();
    let records: Vec<crate::types::Record> = store.records().collect();
    let n = records.len();
    let cols: Vec<(String, f64, f64)> = feat_defs.iter().filter_map(|fd| {
        let xs: Vec<f64> = records.iter().map(|r| r.get(&fd.name).and_then(|v| v.as_f64()).unwrap_or(0.0)).collect();
        let mu = xs.iter().sum::<f64>() / n.max(1) as f64;
        let sd = (xs.iter().map(|x| (x - mu).powi(2)).sum::<f64>() / n.max(1) as f64).sqrt();
        (sd > f64::EPSILON).then_some((fd.name.clone(), mu, sd))
    }).collect();
    let dim = cols.len();
    if dim < 2 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
            "prescribe needs >= 2 numeric fibers with non-zero variance to read the geometry".into()));
    }
    if n < 10 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "prescribe needs >= 10 records to read the geometry".into()));
    }
    let feature_names: Vec<String> = cols.iter().map(|(f, _, _)| f.clone()).collect();
    let x: Vec<Vec<f64>> = records.iter().map(|r| cols.iter()
        .map(|(f, mu, sd)| (r.get(f).and_then(|v| v.as_f64()).unwrap_or(*mu) - mu) / sd).collect()).collect();
    let mut notes: Vec<String> = Vec::new();
    let mut seed: u64 = 0x2545F4914F6CDD1D;
    let mut lcg = || { seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as f64 / (1u64 << 31) as f64 - 1.0 };

    // ── dim_ratio: intrinsic dimension (90% of variance) / ambient dimension ──
    let mut cov = vec![vec![0.0f64; dim]; dim];
    for xi in &x { for a in 0..dim { for b in 0..dim { cov[a][b] += xi[a] * xi[b]; } } }
    for a in 0..dim { for b in 0..dim { cov[a][b] /= n as f64; } }
    let total_var: f64 = (0..dim).map(|t| cov[t][t]).sum();
    let cov_eigs = prescribe_top_eigs(&cov, dim, &mut lcg);
    let (mut cum, mut intrinsic) = (0.0f64, 0usize);
    for (i, e) in cov_eigs.iter().enumerate() {
        cum += e; intrinsic = i + 1;
        if total_var > 0.0 && cum / total_var >= 0.90 { break; }
    }
    let dim_ratio = intrinsic as f64 / dim as f64;

    // stride-subsample for the O(n²) local probes
    let cap = sample.clamp(50, 4000);
    let stride = (n.div_ceil(cap)).max(1);
    let samp: Vec<usize> = (0..n).step_by(stride).collect();
    let m = samp.len();
    if stride > 1 { notes.push(format!("strided to {m} of {n} records for the local-geometry probes")); }
    let dist2 = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| (p - q) * (p - q)).sum::<f64>();
    let kk = neighbors.clamp(3, (m - 1).min(50));

    // ── curvature: mean fraction of a neighborhood's variance off its local top-2
    //    tangent plane. Undefined for dim < 3 (a plane spans all features). ──
    let curvature = if dim < 3 {
        notes.push("curvature reported 0: with only 2 features a local plane spans them exactly".into());
        0.0
    } else {
        let (mut acc, mut cnt) = (0.0f64, 0.0f64);
        for &i in &samp {
            let mut d: Vec<(f64, usize)> = samp.iter().filter(|&&j| j != i)
                .map(|&j| (dist2(&x[i], &x[j]), j)).collect();
            let kn = kk.min(d.len());
            if kn < 3 { continue; }
            d.select_nth_unstable_by(kn - 1, |a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            let nbrs: Vec<usize> = d[..kn].iter().map(|(_, j)| *j).collect();
            let mut mean = vec![0.0; dim];
            for &j in &nbrs { for t in 0..dim { mean[t] += x[j][t]; } }
            for t in 0..dim { mean[t] /= kn as f64; }
            let mut lc = vec![vec![0.0f64; dim]; dim];
            for &j in &nbrs {
                for a in 0..dim { for b in 0..dim { lc[a][b] += (x[j][a] - mean[a]) * (x[j][b] - mean[b]); } }
            }
            for a in 0..dim { for b in 0..dim { lc[a][b] /= kn as f64; } }
            let tr: f64 = (0..dim).map(|t| lc[t][t]).sum();
            if tr <= 1e-12 { continue; }
            let e2 = prescribe_top_eigs(&lc, 2, &mut lcg);
            acc += (tr - e2.iter().sum::<f64>()).max(0.0) / tr; cnt += 1.0;
        }
        if cnt > 0.0 { acc / cnt } else { 0.0 }
    };

    // ── spectral clarity: largest gap among the smallest normalized-Laplacian
    //    eigenvalues of the symmetric k-NN graph (wide gap ⇒ clean clusters) ──
    let spectral_clarity = {
        let mut adj: Vec<std::collections::BTreeSet<usize>> = vec![Default::default(); m];
        for (ai, &i) in samp.iter().enumerate() {
            let mut d: Vec<(f64, usize)> = samp.iter().enumerate().filter(|&(_, &j)| j != i)
                .map(|(aj, &j)| (dist2(&x[i], &x[j]), aj)).collect();
            let kn = kk.min(d.len());
            d.select_nth_unstable_by(kn - 1, |a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
            for (_, aj) in &d[..kn] { adj[ai].insert(*aj); adj[*aj].insert(ai); }
        }
        let adjv: Vec<Vec<usize>> = adj.into_iter().map(|s| s.into_iter().collect()).collect();
        let dsqrt: Vec<f64> = adjv.iter().map(|a| (a.len().max(1) as f64).sqrt()).collect();
        let matvec_l = |v: &[f64], i: usize| -> f64 {
            v[i] - adjv[i].iter().map(|&j| v[j] / (dsqrt[i] * dsqrt[j])).sum::<f64>() };
        let shift = 2.0;
        let want = 8.min(m - 1);
        let mut vecs: Vec<Vec<f64>> = Vec::new();
        let mut eig: Vec<f64> = Vec::new();
        for _ in 0..want {
            let mut v: Vec<f64> = (0..m).map(|_| lcg()).collect();
            for _ in 0..300 {
                let mut bv: Vec<f64> = (0..m).map(|i| shift * v[i] - matvec_l(&v, i)).collect();
                for u in &vecs {
                    let dd: f64 = bv.iter().zip(u).map(|(a, b)| a * b).sum();
                    for i in 0..m { bv[i] -= dd * u[i]; }
                }
                let nrm = bv.iter().map(|z| z * z).sum::<f64>().sqrt();
                if nrm < 1e-12 { break; }
                v = bv.into_iter().map(|z| z / nrm).collect();
            }
            let lam: f64 = (0..m).map(|i| v[i] * matvec_l(&v, i)).sum();
            vecs.push(v); eig.push(lam);
        }
        eig.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        eig.windows(2).map(|w| (w[1] - w[0]).abs()).fold(0.0f64, f64::max)
    };

    let round = |v: f64| (v * 10000.0).round() / 10000.0;
    let (curvature, dim_ratio, spectral_clarity) = (round(curvature), round(dim_ratio), round(spectral_clarity.max(0.0)));
    let mut reads: Vec<String> = Vec::new();
    let mut recommendations: Vec<String> = Vec::new();

    // curvature reading
    let curved = if dim < 3 { "n/a" } else if curvature < 0.05 { "flat" } else if curvature < 0.25 { "mild" } else { "curved" };
    match curved {
        "flat" => reads.push(format!("Locally flat (curvature {curvature:.3}): neighborhoods lie close to a linear tangent plane — the data is nearly a linear subspace.")),
        "mild" => reads.push(format!("Mildly curved (curvature {curvature:.3}): neighborhoods bend modestly off a local plane.")),
        "curved" => reads.push(format!("Strongly curved (curvature {curvature:.3}): neighborhoods spread well off any single tangent plane — a genuinely nonlinear manifold (or locally high-dimensional).")),
        _ => reads.push("Curvature n/a (only 2 features, so a local plane spans them).".into()),
    }
    // dimension reading
    let lowdim = dim_ratio < 0.65;
    let highdim = dim_ratio >= 0.85;
    if lowdim {
        reads.push(format!("Compressible (intrinsic dim {intrinsic} of {dim}, ratio {dim_ratio:.2}): the real structure lives in fewer dimensions than measured."));
    } else if highdim {
        reads.push(format!("Near-full-rank (intrinsic dim {intrinsic} of {dim}, ratio {dim_ratio:.2}): features are largely independent — little to compress."));
    } else {
        reads.push(format!("Moderate intrinsic dimension ({intrinsic} of {dim}, ratio {dim_ratio:.2})."));
    }
    // cluster reading
    let clustered = spectral_clarity > 0.1;
    if clustered {
        reads.push(format!("Clear cluster structure (spectral clarity {spectral_clarity:.3}): the neighbor graph splits into well-separated groups."));
    } else if spectral_clarity > 0.03 {
        reads.push(format!("Weak cluster separation (spectral clarity {spectral_clarity:.3}): groups exist but overlap."));
    } else {
        reads.push(format!("No obvious clusters (spectral clarity {spectral_clarity:.3}): one connected mass, or heavily overlapping groups."));
    }

    // ── geometric regime (the honest quadrant of LOCAL curvature × GLOBAL linear rank).
    //    The key insight: low local curvature + high global rank is NOT "flat" — it's a
    //    nonlinear manifold (locally thin, globally space-filling, e.g. a rolled sheet)
    //    that linear PCA cannot see but geometric methods can. ──
    let locally_flat = dim >= 3 && curvature < 0.15;
    let locally_rich = dim >= 3 && curvature >= 0.25;
    if dim >= 3 {
        if locally_flat && highdim {
            reads.push("Regime: NONLINEAR MANIFOLD — locally low-dimensional yet globally full-rank (a curved/rolled sheet). Linear PCA can't see this; geometric methods can.".into());
            recommendations.push("/infer method=diffusion, /cluster spectral, /scan — manifold structure that flat & linear methods miss.".into());
        } else if locally_flat {
            reads.push("Regime: LINEAR & LOW-RANK — the data is essentially a linear subspace; geometry adds little.".into());
            recommendations.push("flat baselines are ideal (/infer method=ols, /cluster method=kmeans) — near-linear structure.".into());
        } else if locally_rich && !highdim {
            reads.push("Regime: CURVED COMPRESSIBLE MANIFOLD — genuinely nonlinear but low intrinsic dimension; geometry's sweet spot.".into());
            recommendations.push("/scan completion lens, /infer method=diffusion, /cluster spectral — curved low-dim manifold; geometric heads shine.".into());
        } else if locally_rich {
            reads.push("Regime: HIGH-DIMENSIONAL & FULL-RANK — little low-dim manifold to exploit; expect every method to need lots of data.".into());
            recommendations.push("flat and geometric methods are on even footing — try /infer method=ols/knn and compare; no strong manifold to lean on.".into());
        } else {
            reads.push("Regime: INTERMEDIATE — mild curvature and moderate rank; try a flat baseline and its geometric counterpart.".into());
        }
    } else {
        reads.push("Curvature-based regime needs >= 3 features; only the cluster read applies here.".into());
    }

    // ── recommendations: which GIGI methods the geometry SUGGESTS trying (routing
    //    hints, not performance claims). Compression + clustering axes below; the
    //    curvature×rank regime above already added its geometric-vs-flat steer. ──
    if lowdim {
        recommendations.push(format!("/reduce (PCA) — {intrinsic} component(s) retain ~90% of the variance; compress before other work."));
    }
    if clustered {
        recommendations.push("/cluster — well-separated groups; spectral or k-means both viable.".into());
    } else if spectral_clarity > 0.03 {
        recommendations.push("/cluster method=gmm — groups that touch/overlap; soft elliptical components fit better than hard k-means.".into());
    }
    if recommendations.is_empty() {
        recommendations.push("no strong steer — try a flat baseline and its geometric counterpart and compare (see GET /v1/ml).".into());
    }
    notes.push("prescribe reads your data's geometric shape and suggests which methods to TRY; it makes no performance-prediction claim.".into());
    notes.push("curvature is LOCAL non-flatness; dim_ratio is GLOBAL linear rank — a rolled manifold reads low-curvature + high-rank (see the regime line).".into());

    Ok(PrescribeResult {
        n, ambient_dim: dim, intrinsic_dim: intrinsic, dim_ratio, curvature, spectral_clarity,
        reads, recommendations, feature_names, notes,
    })
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;

    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Value as V};

    /// PRESCRIBE (geometric diagnostic — no performance prediction). A flat, low-rank
    /// dataset (a 2-D plane linearly embedded in 4-D: f3=f1+f2, f4=f1−f2) must read as
    /// locally planar (curvature≈0) and compressible (dim_ratio≈0.5 → intrinsic 2 of 4),
    /// and the recommendations must point at /reduce.
    #[test]
    fn prescribe_reads_linear_lowrank() {
        let mut rows: Vec<crate::types::Record> = Vec::new();
        let mut s: u64 = 4242;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 33) as f64 / (1u64 << 31) as f64 - 0.5 };
        for i in 0..300 {
            let (a, b) = (rnd() * 3.0, rnd() * 3.0);
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("f1", V::Float(a + 0.01 * rnd())), ("f2", V::Float(b + 0.01 * rnd())),
                ("f3", V::Float(a + b + 0.01 * rnd())), ("f4", V::Float(a - b + 0.01 * rnd())),
            ]));
        }
        let schema = BundleSchema::new("flat")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("f1")).fiber(FieldDef::numeric("f2"))
            .fiber(FieldDef::numeric("f3")).fiber(FieldDef::numeric("f4"));
        let (dir, engine) = scan_env("prescribe_flat", "flat", schema, rows);
        let p = prescribe_fingerprint(&engine, "flat", 10, 500, &[]).expect("prescribe");
        assert_eq!(p.ambient_dim, 4);
        assert_eq!(p.intrinsic_dim, 2, "a 2-D plane in 4-D has intrinsic dim 2");
        assert!(p.dim_ratio < 0.6, "low-rank should read compressible, dim_ratio={}", p.dim_ratio);
        assert!(p.curvature < 0.15, "a linear plane is locally flat, curvature={}", p.curvature);
        let joined = p.recommendations.join(" ").to_lowercase();
        assert!(joined.contains("/reduce"), "low-rank should recommend /reduce; got {:?}", p.recommendations);
        cleanup(&dir);
    }

    /// PRESCRIBE: an isotropic high-dimensional Gaussian fills its space — it must read
    /// as full-rank (dim_ratio high) and locally non-planar (curvature high: a
    /// neighborhood spreads well beyond any 2-D tangent plane).
    #[test]
    fn prescribe_reads_highdim_fullrank() {
        let d = 6usize;
        let mut s: u64 = 77;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 };
        let mut gauss = || { let (u1, u2): (f64, f64) = (rnd().max(1e-9), rnd());
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() };
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for i in 0..300 {
            let mut rec = vec![("id".to_string(), V::Text(format!("r{i}")))];
            for j in 0..d { rec.push((format!("x{j}"), V::Float(gauss()))); }
            rows.push(rec.into_iter().collect());
        }
        let mut schema = BundleSchema::new("iso").base(FieldDef::categorical("id"));
        for j in 0..d { schema = schema.fiber(FieldDef::numeric(&format!("x{j}"))); }
        let (dir, engine) = scan_env("prescribe_iso", "iso", schema, rows);
        let p = prescribe_fingerprint(&engine, "iso", 12, 500, &[]).expect("prescribe");
        assert_eq!(p.ambient_dim, 6);
        assert!(p.dim_ratio > 0.7, "isotropic Gaussian is near-full-rank, dim_ratio={}", p.dim_ratio);
        assert!(p.curvature > 0.2, "a 6-D blob spreads well off any 2-D plane, curvature={}", p.curvature);
        cleanup(&dir);
    }

    /// PRESCRIBE: three well-separated blobs must read as having clear cluster structure
    /// (high spectral clarity — the neighbor graph splits with a wide spectral gap), and
    /// the recommendations must point at /cluster.
    #[test]
    fn prescribe_reads_clear_clusters() {
        let centers = [[0.0, 0.0, 0.0], [10.0, 0.0, 0.0], [0.0, 10.0, 10.0]];
        let mut s: u64 = 20260719;
        let mut rnd = || { s = s.wrapping_mul(6364136223846793005).wrapping_add(1); (s >> 11) as f64 / (1u64 << 53) as f64 };
        let mut gauss = || { let (u1, u2): (f64, f64) = (rnd().max(1e-9), rnd());
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos() };
        let mut rows: Vec<crate::types::Record> = Vec::new();
        for i in 0..300 {
            let c = centers[i % 3];
            rows.push(scan_rec(&[
                ("id", V::Text(format!("r{i}"))),
                ("a", V::Float(c[0] + 0.3 * gauss())), ("b", V::Float(c[1] + 0.3 * gauss())),
                ("c", V::Float(c[2] + 0.3 * gauss())),
            ]));
        }
        let schema = BundleSchema::new("blobs")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("a")).fiber(FieldDef::numeric("b")).fiber(FieldDef::numeric("c"));
        let (dir, engine) = scan_env("prescribe_blobs", "blobs", schema, rows);
        let p = prescribe_fingerprint(&engine, "blobs", 10, 500, &[]).expect("prescribe");
        assert!(p.spectral_clarity > 0.1, "3 separated blobs → wide spectral gap, clarity={}", p.spectral_clarity);
        let joined = p.recommendations.join(" ").to_lowercase();
        assert!(joined.contains("/cluster"), "clear clusters should recommend /cluster; got {:?}", p.recommendations);
        cleanup(&dir);
    }

    /// PRESCRIBE gives actionable errors, not panics, on bad input.
    #[test]
    fn prescribe_guards_are_actionable() {
        let rows: Vec<_> = (0..10).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i}"))), ("only", V::Float(i as f64)),
        ])).collect();
        let schema = BundleSchema::new("one")
            .base(FieldDef::categorical("id")).fiber(FieldDef::numeric("only"));
        let (dir, engine) = scan_env("prescribe_guard", "one", schema, rows);
        // only 1 numeric fiber → can't fingerprint
        let e = prescribe_fingerprint(&engine, "one", 10, 500, &[]).unwrap_err();
        assert_eq!(e.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(e.1.contains(">= 2 numeric"), "got: {}", e.1);
        // missing bundle → 404
        assert_eq!(prescribe_fingerprint(&engine, "nope", 10, 500, &[]).unwrap_err().0, StatusCode::NOT_FOUND);
        cleanup(&dir);
    }
}
