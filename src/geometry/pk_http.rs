//! Post-Kähler PK-1..4 REST logic (stream-extraction phase 2, family 2;
//! see EXTRACTION_MAP.md). The free logic + request structs for the
//! `fisher_metric` / `wasserstein` / `reeb_flow` endpoints, moved
//! verbatim out of `src/bin/gigi_stream.rs` — the handlers there stay as
//! thin wrappers that acquire the engine lock and call into this module.
//! The persistence endpoint's logic lives in `crate::discrete::pk_http`
//! (its math home). Everything here is gated on `post_kahler_phase1`,
//! exactly as the binary items were.

use crate::stream_shared::{bad_request, extract_field_samples, heap_or_promote, not_found, ErrorResponse};
use axum::{http::StatusCode, Json};
use serde::Deserialize;

/// PK-2 — GET /v1/bundles/{name}/fisher_metric[?fields=f1,f2,…]
///
/// The Fisher information metric of each numeric fiber, read for free
/// from the bundle's L4 Welford variance: for a field modeled as
/// `N(μ,σ²)` the metric in the `(μ,σ)` chart is `g = diag(1/σ², 2/σ²)`,
/// `g_μσ = 0`. Fields with zero/undefined variance are omitted.
pub fn fisher_metric(
    engine: &crate::Engine,
    name: &str,
    params: &std::collections::HashMap<String, String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let mut _promoted: Option<crate::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
    let stats = heap.field_stats();

    let fields: Vec<String> = match params.get("fields") {
        Some(s) => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        None => heap.schema.fiber_fields.iter().map(|fd| fd.name.clone()).collect(),
    };
    if fields.is_empty() {
        return Err(bad_request("no numeric fiber fields to read a Fisher metric from"));
    }

    let mut metrics = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for f in &fields {
        let s = match stats.get(f) {
            Some(s) if s.count > 0 => s,
            _ => {
                skipped.push(f.clone());
                continue;
            }
        };
        let var = s.variance();
        match crate::geometry::FisherGaussian::from_variance(var) {
            Ok(g) => metrics.push(serde_json::json!({
                "field": f,
                "mean": s.mean,
                "variance": var,
                "g_mu_mu": g.g_mu_mu,
                "g_sigma_sigma": g.g_sigma_sigma,
                "g_mu_sigma": g.g_mu_sigma,
                "det": g.determinant(),
            })),
            Err(_) => skipped.push(f.clone()),
        }
    }
    if metrics.is_empty() {
        return Err(bad_request(
            "no requested field had positive Welford variance (Fisher metric undefined)",
        ));
    }
    Ok(Json(serde_json::json!({
        "bundle": name,
        "chart": "(mu, sigma)",
        "closed_form": "g = diag(1/sigma^2, 2/sigma^2), g_mu_sigma = 0",
        "metrics": metrics,
        "skipped": skipped,
        "notes": [
            "Fisher metric of the univariate Gaussian family, read from L4 Welford variance — no extra pass.",
            "Fields with zero observations or zero variance are omitted (the metric diverges as sigma -> 0)."
        ],
    })))
}

/// PK-3 request. Either supply two raw distributions (`sample_a`,
/// `sample_b`) or split one bundle by a cohort fiber.
#[derive(Deserialize)]
pub struct WassersteinRequest {
    /// Fiber whose distribution is compared (cohort mode).
    #[serde(default)]
    pub field: Option<String>,
    /// Fiber whose value defines the two cohorts.
    #[serde(default)]
    pub cohort_field: Option<String>,
    /// Cohort A / B selector values on `cohort_field`.
    #[serde(default)]
    pub a: Option<f64>,
    #[serde(default)]
    pub b: Option<f64>,
    /// Direct mode: W₂ between these two raw samples (bypasses the bundle).
    #[serde(default)]
    pub sample_a: Option<Vec<f64>>,
    #[serde(default)]
    pub sample_b: Option<Vec<f64>>,
}

/// PK-3 — POST /v1/bundles/{name}/ml/wasserstein
///
/// Exact 1D 2-Wasserstein distance `W₂` between two empirical
/// distributions via the monotone rearrangement (Hoeffding). Closed
/// form for Gaussians: `W₂² = μ_d² + σ_d²`.
pub fn wasserstein(
    engine: &crate::Engine,
    name: &str,
    req: &WassersteinRequest,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    // Direct mode: two raw samples supplied.
    let (a_vals, b_vals, mode): (Vec<f64>, Vec<f64>, serde_json::Value) =
        if let (Some(a), Some(b)) = (req.sample_a.clone(), req.sample_b.clone()) {
            (a, b, serde_json::json!({"mode": "direct"}))
        } else {
            // Cohort mode: split the bundle by cohort_field into a / b.
            let field = req
                .field
                .clone()
                .ok_or_else(|| bad_request("cohort mode needs `field` (the fiber to compare)"))?;
            let cohort_field = req.cohort_field.clone().ok_or_else(|| {
                bad_request("cohort mode needs `cohort_field`, `a`, `b` (or supply sample_a/sample_b)")
            })?;
            let (av, bv) = (
                req.a
                    .ok_or_else(|| bad_request("cohort mode needs cohort selector `a`"))?,
                req.b
                    .ok_or_else(|| bad_request("cohort mode needs cohort selector `b`"))?,
            );
            let store = engine
                .bundle(&name)
                .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
            let mut _promoted: Option<crate::BundleStore> = None;
            let heap = heap_or_promote(&store, &mut _promoted);
            let (rows, _) = extract_field_samples(heap, &[field.clone(), cohort_field.clone()])
                .map_err(|e| bad_request(&e))?;
            let mut ga = Vec::new();
            let mut gb = Vec::new();
            for r in &rows {
                if r.len() != 2 {
                    continue;
                }
                if (r[1] - av).abs() < 1e-9 {
                    ga.push(r[0]);
                } else if (r[1] - bv).abs() < 1e-9 {
                    gb.push(r[0]);
                }
            }
            (
                ga,
                gb,
                serde_json::json!({"mode": "cohort", "field": field, "cohort_field": cohort_field, "a": av, "b": bv}),
            )
        };

    if a_vals.is_empty() || b_vals.is_empty() {
        return Err(bad_request(
            "both cohorts must be non-empty (check the field/cohort selectors or the supplied samples)",
        ));
    }
    let w2_sq = crate::geometry::Wasserstein1D::compute_sq(&a_vals, &b_vals)
        .map_err(|e| bad_request(&e.to_string()))?;
    Ok(Json(serde_json::json!({
        "bundle": name,
        "w2_distance": w2_sq.sqrt(),
        "w2_squared": w2_sq,
        "n_a": a_vals.len(),
        "n_b": b_vals.len(),
        "selection": mode,
        "notes": [
            "Exact 1D W2 via monotone rearrangement (Hoeffding). For Gaussians W2^2 = (mu_a-mu_b)^2 + (sigma_a-sigma_b)^2.",
            "Unequal sample sizes are handled by an exact quantile-function integral, not a fixed grid."
        ],
    })))
}

/// PK-1 request — three numeric fibers read as the contact-ℝ³ (x,y,z).
#[derive(Deserialize)]
pub struct ReebFlowRequest {
    /// Exactly three numeric fiber fields, mapped to (x, y, z).
    pub fields: Vec<String>,
}

/// PK-1 — POST /v1/bundles/{name}/brain/reeb_flow
///
/// Surfaces the standard contact structure `α = dz − y·dx` on three
/// chosen numeric fibers and its Reeb field `R = ∂_z`, verifying the two
/// defining conditions (`α(R)=1`, `ι_R dα=0`) and non-degeneracy
/// (`α ∧ dα ≠ 0`) on the bundle's own points. `α(R) ≡ 1` along the flow
/// is the invariant the Reeb field preserves. (Honest-minimal binding:
/// the validated contact primitive on the selected coordinates — not a
/// learned sequence-flow integrator.)
pub fn reeb_flow(
    engine: &crate::Engine,
    name: &str,
    req: &ReebFlowRequest,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    if req.fields.len() != 3 {
        return Err(bad_request(
            "reeb_flow needs exactly 3 numeric fiber fields, read as contact coordinates (x, y, z)",
        ));
    }
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let mut _promoted: Option<crate::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
    let (rows, _) = extract_field_samples(heap, &req.fields).map_err(|e| bad_request(&e))?;
    let points: Vec<[f64; 3]> = rows
        .iter()
        .filter(|p| p.len() == 3)
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    if points.is_empty() {
        return Err(bad_request("no records with all three coordinate fibers present"));
    }
    let reeb = crate::geometry::ContactOneForm::reeb();
    let alpha_defect = reeb.alpha_defect(&points); // max |α(R) − 1| over the data
    let probes = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [3.0, -2.0, 5.0]];
    let dalpha_defect = reeb.dalpha_defect(&probes); // max |ι_R dα|
    // contact volume at the data mean
    let n = points.len() as f64;
    let mean = [
        points.iter().map(|p| p[0]).sum::<f64>() / n,
        points.iter().map(|p| p[1]).sum::<f64>() / n,
        points.iter().map(|p| p[2]).sum::<f64>() / n,
    ];
    let vol = crate::geometry::ContactOneForm::contact_volume(
        mean,
        reeb.vector,
        [1.0, 0.0, 0.0],
        [0.0, 1.0, 0.0],
    );
    Ok(Json(serde_json::json!({
        "bundle": name,
        "fields": req.fields,
        "coordinates": "alpha = dz - y dx on (x, y, z) = the three fields, in order",
        "reeb_field": reeb.vector,
        "n_points": points.len(),
        "alpha_of_reeb_defect": alpha_defect,   // ~0: α(R) ≡ 1 (Reeb condition I)
        "iota_r_dalpha_defect": dalpha_defect,  // ~0: ι_R dα ≡ 0 (Reeb condition II)
        "contact_volume_at_mean": vol,          // != 0: α ∧ dα is a volume form
        "is_contact": vol.abs() > 1e-12,
        "flow_invariant": "alpha(R) = 1 is preserved along the Reeb flow (translation in +z)",
        "notes": [
            "Standard contact R^3 primitive surfaced on the three chosen fibers; verifications hold structurally and on the data.",
            "Honest-minimal binding: not a learned token-sequence flow integrator."
        ],
    })))
}
