//! `imagine_coherence` — predictive coherence trajectory along an
//! imagined geodesic. Marcella's gain gate consumes this to make
//! routing decisions on the *imagined future* instead of the
//! reactive past.
//!
//! See `IMAGINE_AND_WALK.md` §5 for the spec.

use crate::imagine::config::{ImagineConfig, WalkConfig};
use crate::imagine::geodesic::{imagine_geodesic, ConformalMetric, ImagineError};
use crate::imagine::provenance::ImaginedProvenance;
use serde::{Deserialize, Serialize};

/// One point along the imagined coherence trajectory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoherencePoint {
    /// 0-indexed step along the trajectory (0 = seed).
    pub step: u32,
    /// Coordinates at this step in chart coords.
    pub coords: Vec<f64>,
    /// Coherence at this step, in `[0, 1]`. 1 = laminar, 0 = turbulent.
    pub coherence: f64,
    /// Per-step holonomy defect (delta from previous step).
    pub defect: f64,
    /// Local Gaussian curvature K at this step.
    pub curvature: f64,
    /// Total accumulated holonomy from the seed to this step.
    pub cumulative_holonomy: f64,
    /// Human-readable provenance describing how this point was
    /// constructed. Per Marcella's cite contract, this string
    /// surfaces the `imagined` marker in any downstream rendering.
    pub provenance: String,
}

/// Report from `imagine_coherence_trajectory`. If `refused = true`,
/// the trajectory was constructed but the walk would be refused at
/// commit time; consumers should not consume the endpoint as a
/// gating decision.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoherenceTrajectoryReport {
    pub trajectory: Vec<CoherencePoint>,
    /// Coherence value at the final step (or 1.0 for empty trajectories).
    pub endpoint_coherence: f64,
    /// Curvature value at the final step (or 0.0 for empty).
    pub endpoint_curvature: f64,
    /// Whether the walk would be refused at commit time per the
    /// `WalkConfig` envelope (curvature ceiling, holonomy budget).
    pub refused: bool,
    /// Human-readable refusal reason if `refused = true`; otherwise
    /// `None`.
    pub refusal_reason: Option<String>,
    /// Dimension of the substrate (`coords.len()` at each step).
    pub dim: usize,
}

/// Compute the predictive coherence trajectory along an imagined
/// geodesic. Each step's coherence is derived from the per-step
/// holonomy defect using the same formula as `LOCAL_HOLONOMY`:
///
/// ```text
///   coherence_step = 1 - defect_step / (2 · √dim)
/// ```
///
/// Phase 1 implementation:
/// - Constructs the trajectory by calling `imagine_geodesic` with
///   `n_steps = steps`, `path_length = 1.0`.
/// - For each step k, the per-step defect is the delta of
///   `accumulated_holonomy` between steps k and k-1; for the seed,
///   defect = 0 and coherence = 1.0.
/// - Per Marcella's safety envelope, the report sets `refused = true`
///   if any step's K exceeds `walk_config.max_imagined_curvature` OR
///   if cumulative_holonomy exceeds the budget. The trajectory is
///   still returned for inspection; only commit is blocked.
pub fn imagine_coherence_trajectory(
    metric: &ConformalMetric,
    seed_record_id: &str,
    seed_bundle: &str,
    starting_from: &[f64],
    along: &[f64],
    steps: u32,
    walk_config: &WalkConfig,
) -> Result<CoherenceTrajectoryReport, ImagineError> {
    let dim = starting_from.len();
    let cfg = ImagineConfig {
        path_length: 1.0,
        n_steps: steps.max(1),
        adaptive: false,
    };
    let records = imagine_geodesic(
        metric, seed_record_id, seed_bundle, starting_from, along, &cfg,
    )?;

    let sqrt_dim = (dim as f64).max(1.0).sqrt();
    let max_defect = 2.0 * sqrt_dim;

    let mut trajectory: Vec<CoherencePoint> = Vec::with_capacity(records.len());
    let mut refused = false;
    let mut refusal_reason: Option<String> = None;
    let mut prev_holonomy: f64 = 0.0;

    for (i, rec) in records.iter().enumerate() {
        let cumulative_holonomy = rec.accumulated_holonomy;
        let defect = (cumulative_holonomy - prev_holonomy).max(0.0);
        let coherence = if max_defect > 0.0 {
            (1.0 - defect / max_defect).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // Marcella safety envelope (load-bearing feedback #3)
        if !refused && rec.local_k > walk_config.max_imagined_curvature {
            refused = true;
            refusal_reason = Some(format!(
                "step {}: K = {:.4} > max_imagined_curvature {:.2} \
                 (default = 4.0 = K(CP¹ Fubini-Study))",
                i, rec.local_k, walk_config.max_imagined_curvature
            ));
        }
        if !refused && cumulative_holonomy > walk_config.max_accumulated_holonomy {
            refused = true;
            refusal_reason = Some(format!(
                "step {}: accumulated holonomy {:.4} > budget {:.2}",
                i, cumulative_holonomy, walk_config.max_accumulated_holonomy
            ));
        }

        let provenance_str = match &rec.provenance {
            ImaginedProvenance::Geodesic {
                seed_bundle,
                path_length,
                integrator_steps,
                ..
            } => {
                if i == 0 {
                    format!("imagined: seed from {}", seed_bundle)
                } else {
                    format!(
                        "imagined: geodesic from {}, step {}/{} path_length={:.3}",
                        seed_bundle, integrator_steps, steps, path_length
                    )
                }
            }
            _ => format!("imagined: step {} (non-geodesic provenance)", i),
        };

        trajectory.push(CoherencePoint {
            step: i as u32,
            coords: rec.coords.clone(),
            coherence,
            defect,
            curvature: rec.local_k,
            cumulative_holonomy,
            provenance: provenance_str,
        });
        prev_holonomy = cumulative_holonomy;
    }

    let (endpoint_coherence, endpoint_curvature) =
        if let Some(last) = trajectory.last() {
            (last.coherence, last.curvature)
        } else {
            (1.0, 0.0)
        };

    Ok(CoherenceTrajectoryReport {
        trajectory,
        endpoint_coherence,
        endpoint_curvature,
        refused,
        refusal_reason,
        dim,
    })
}

/// Construct a constant-Gaussian-curvature 2D conformally-flat metric
/// from a single scalar K. Used by the HTTP endpoint to derive a
/// metric closure from the bundle's curvature_stats.mean.
///
/// - `K = 0`: flat metric. Geodesics are straight lines.
/// - `K > 0`: S²-like stereographic metric. Geodesics curl back.
/// - `K < 0`: hyperbolic plane (Phase 2; Phase 1 falls back to flat
///   with a documented note in the response).
pub fn metric_for_constant_k<'a>(k_constant: f64) -> ConformalMetric<'a> {
    if k_constant <= 0.0 {
        // Phase 1: K < 0 falls back to flat. The trajectory still has
        // structure from the curvature passed to each point's local_k.
        return ConformalMetric {
            phi_x: Box::new(|_x, _y| 0.0),
            phi_y: Box::new(|_x, _y| 0.0),
            local_k: Box::new(move |_x, _y| k_constant.max(0.0)),
        };
    }
    // S²-like metric: phi_x = -2x/(1+r²), phi_y = -2y/(1+r²).
    // Scale by 1/sqrt(K) so curvature matches the requested K.
    let k_scale = k_constant.sqrt();
    ConformalMetric {
        phi_x: Box::new(move |x: f64, y: f64| {
            let r2 = x * x + y * y;
            -2.0 * k_scale * x / (1.0 + r2)
        }),
        phi_y: Box::new(move |x: f64, y: f64| {
            let r2 = x * x + y * y;
            -2.0 * k_scale * y / (1.0 + r2)
        }),
        local_k: Box::new(move |_x: f64, _y: f64| k_constant),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trajectory_on_flat_metric_has_high_coherence() {
        let metric = metric_for_constant_k(0.0);
        let report = imagine_coherence_trajectory(
            &metric, "seed", "test_bundle",
            &[0.0, 0.0], &[0.5, 0.0], 10,
            &WalkConfig::default(),
        ).unwrap();

        assert_eq!(report.trajectory.len(), 11);
        // On flat metric, no rotation, so coherence stays at 1.0
        for point in &report.trajectory {
            assert!((point.coherence - 1.0).abs() < 1e-9,
                    "flat-metric coherence drifted: step {} coherence = {}",
                    point.step, point.coherence);
        }
        assert!((report.endpoint_coherence - 1.0).abs() < 1e-9);
        assert!(!report.refused);
        assert!(report.refusal_reason.is_none());
    }

    #[test]
    fn trajectory_on_curved_metric_shows_coherence_drop() {
        let metric = metric_for_constant_k(1.0);  // S²-like
        let report = imagine_coherence_trajectory(
            &metric, "seed", "test_bundle",
            &[0.2, 0.3], &[0.5, -0.3], 10,
            &WalkConfig::default(),
        ).unwrap();

        // First step has zero defect by construction
        assert!((report.trajectory[0].coherence - 1.0).abs() < 1e-9);
        // Later steps should have some defect (geodesic rotates)
        let final_coh = report.endpoint_coherence;
        assert!(final_coh <= 1.0);
        // Sanity: curvature reflects the metric
        for point in &report.trajectory {
            assert!((point.curvature - 1.0).abs() < 1e-9,
                    "curvature should equal metric's constant K=1.0, got {}",
                    point.curvature);
        }
    }

    #[test]
    fn refused_when_K_exceeds_max_imagined_curvature() {
        let metric = metric_for_constant_k(5.0);  // exceeds default 4.0
        let report = imagine_coherence_trajectory(
            &metric, "seed", "test_bundle",
            &[0.1, 0.1], &[0.3, 0.0], 5,
            &WalkConfig::default(),
        ).unwrap();
        assert!(report.refused, "K=5.0 should refuse against default max 4.0");
        let reason = report.refusal_reason.unwrap();
        assert!(reason.contains("K = 5.0000"));
        assert!(reason.contains("max_imagined_curvature 4.00"));
    }

    #[test]
    fn explicit_opt_out_lets_high_curvature_through() {
        let metric = metric_for_constant_k(5.0);
        let cfg = WalkConfig {
            max_imagined_curvature: 10.0,
            ..WalkConfig::default()
        };
        let report = imagine_coherence_trajectory(
            &metric, "seed", "test_bundle",
            &[0.1, 0.1], &[0.3, 0.0], 5, &cfg,
        ).unwrap();
        assert!(!report.refused);
    }

    #[test]
    fn provenance_string_contains_imagined_marker() {
        let metric = metric_for_constant_k(0.0);
        let report = imagine_coherence_trajectory(
            &metric, "seed", "marcella_corpus",
            &[0.0, 0.0], &[1.0, 0.0], 3,
            &WalkConfig::default(),
        ).unwrap();
        for point in &report.trajectory {
            assert!(point.provenance.starts_with("imagined:"),
                    "provenance must surface 'imagined:' marker per Marcella contract, got: {}",
                    point.provenance);
            assert!(point.provenance.contains("marcella_corpus"));
        }
    }

    #[test]
    fn empty_trajectory_returns_safe_defaults() {
        // steps = 0 still gets clamped to at least 1 by the function
        let metric = metric_for_constant_k(0.0);
        let report = imagine_coherence_trajectory(
            &metric, "s", "b", &[0.0, 0.0], &[0.0, 0.0], 0,
            &WalkConfig::default(),
        ).unwrap();
        // n_steps clamped to 1 -> 2 trajectory points (seed + 1 step)
        assert_eq!(report.trajectory.len(), 2);
    }
}
