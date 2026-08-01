//! Post-Kähler PK-4 REST logic (stream-extraction phase 2, family 2; see
//! EXTRACTION_MAP.md). The free logic for the `topology/persistence`
//! endpoint, moved verbatim out of `src/bin/gigi_stream.rs` — the handler
//! there stays as a thin wrapper that acquires the engine lock and calls
//! into this module. Gated on `post_kahler_phase1`, exactly as the binary
//! item was. The PK-1..3 siblings live in `crate::geometry::pk_http`.

use crate::stream_shared::{bad_request, extract_field_samples, heap_or_promote, not_found, ErrorResponse};
use axum::{http::StatusCode, Json};

/// PK-4 — GET /v1/bundles/{name}/topology/persistence[?fields=x,y&gap_factor=2]
///
/// H₀ persistent homology of the point cloud (records × chosen fibers)
/// via the Euclidean MST + elder rule: the `n−1` MST edge weights are
/// the finite death times, one bar survives forever. The persistence
/// gap (a `gap_factor×` drop off a genuine inter-cluster bridge) yields
/// the cluster count.
pub fn persistence(
    engine: &crate::Engine,
    name: &str,
    params: &std::collections::HashMap<String, String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let store = engine
        .bundle(&name)
        .ok_or_else(|| not_found(&format!("Bundle '{}' not found", name)))?;
    let mut _promoted: Option<crate::BundleStore> = None;
    let heap = heap_or_promote(&store, &mut _promoted);
    let fields: Vec<String> = match params.get("fields") {
        Some(s) => s
            .split(',')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect(),
        None => heap.schema.fiber_fields.iter().map(|fd| fd.name.clone()).collect(),
    };
    if fields.is_empty() {
        return Err(bad_request("persistence needs at least one numeric fiber field"));
    }
    let gap_factor: f64 = params
        .get("gap_factor")
        .and_then(|s| s.parse().ok())
        .filter(|g: &f64| *g > 1.0)
        .unwrap_or(2.0);

    let (points, _) = extract_field_samples(heap, &fields).map_err(|e| bad_request(&e))?;
    // Bound the O(n²) MST for a live endpoint.
    const MAX_POINTS: usize = 4000;
    if points.len() > MAX_POINTS {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ErrorResponse {
                error: format!(
                    "persistence caps at {MAX_POINTS} points (got {}); filter the bundle first",
                    points.len()
                ),
            }),
        ));
    }
    let intervals = crate::discrete::h0_persistence(&points).map_err(|e| bad_request(&e.to_string()))?;
    let edges = crate::discrete::mst_merge_edges(&points).map_err(|e| bad_request(&e.to_string()))?;
    let k = crate::discrete::cluster_count(&points, gap_factor).unwrap_or(1);
    let bars: Vec<serde_json::Value> = intervals
        .iter()
        .map(|iv| {
            serde_json::json!({
                "birth": iv.birth,
                "death": if iv.death.is_infinite() { serde_json::Value::Null } else { serde_json::json!(iv.death) },
            })
        })
        .collect();
    Ok(Json(serde_json::json!({
        "bundle": name,
        "fields": fields,
        "n_points": points.len(),
        "dims": fields.len(),
        "estimated_clusters": k,
        "gap_factor": gap_factor,
        "mst_merge_edges": edges,
        "h0_intervals": bars,
        "notes": [
            "H0 persistence via Euclidean MST + elder rule; n-1 finite bars (MST edges) + 1 infinite bar (null death).",
            "estimated_clusters = 1 + (bridges above the first gap_factor-drop off an above-mean-length edge)."
        ],
    })))
}
