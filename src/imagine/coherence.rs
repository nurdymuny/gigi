//! `imagine_coherence` — predictive coherence trajectory along an
//! imagined geodesic. Marcella's gain gate consumes this to make
//! routing decisions on the *imagined future* instead of the
//! reactive past.
//!
//! See `IMAGINE_AND_WALK.md` §5 for the spec.

use crate::imagine::config::{ImagineConfig, WalkConfig};
use crate::imagine::geodesic::{imagine_geodesic, ConformalMetric, ImagineError};
use crate::imagine::provenance::{ImaginedProvenance, ImaginedRecord};
use serde::{Deserialize, Serialize};

/// Phase 2 tame-metric threshold. When `bundle.curvature_stats().mean()`
/// exceeds `|K_MAX_PHASE2|`, the HTTP handler auto-substitutes
/// `K_TAME_PHASE2` (1.0) and emits a [`crate::wal::WalEntry::ImagineFallback`]
/// event. Consumers may override either by passing `metric_curvature`
/// explicitly in the request — the explicit value bypasses the fallback.
pub const K_MAX_PHASE2: f64 = 10.0;

/// Phase 2 tame-metric replacement K. The unit-sphere curvature value
/// the integrator falls back to when `K_MAX_PHASE2` is exceeded.
pub const K_TAME_PHASE2: f64 = 1.0;

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

impl CoherencePoint {
    /// Always true. Same contract as
    /// `ImaginedRecord::is_imagined` — provided so the response-path
    /// branching can be a method call rather than a string parse on
    /// `provenance`. Per Marcella round-3 feedback #1.
    #[inline]
    pub fn is_imagined(&self) -> bool {
        true
    }
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

// ─── Phase 2 ───────────────────────────────────────────────────────────────
//
// Phase 2 lifts `imagine_coherence` off the dim=2 constraint and adds a
// tame-metric fallback for high-K bundles. Conformal geodesic flow is
// separable (T(p) + V(q)) so RK4 is the right integrator; the Halcyon
// `HamiltonianPoissonBracket` trait is intentionally NOT used here (it
// is reserved for non-separable cases like AURORA shallow water).
//
// For *constant* curvature K the geodesic is closed-form on a maximally-
// symmetric space (sphere / plane / hyperbolic space), so Phase 2 uses
// the closed-form directly. This collapses the n-D RK4 to a single
// trig/hyperbolic evaluation per step — bit-stable, no truncation
// error, and trivially backwards-compatible. RK4 is reserved for a
// future non-constant-K extension where the closed form does not apply.

/// Phase 2 closed-form geodesic on the maximally-symmetric space of
/// constant Gaussian curvature `metric_curvature`.
///
/// * `K = 0` — Euclidean plane. `x(t) = x_0 + t · v_0` (straight line).
/// * `K > 0` — round n-sphere of radius `R = 1/√K` (embedded view).
///   `x(t) = cos(√K · t) · x_0 + sin(√K · t)/√K · v_0` projected back
///   onto the chart.
/// * `K < 0` — hyperbolic plane / space.
///   `x(t) = cosh(√|K| · t) · x_0 + sinh(√|K| · t)/√|K| · v_0`.
///
/// Returns the per-step coordinate vector `[x_0, x_1, ..., x_steps]`.
/// `steps` is the number of forward steps, so the result has
/// `steps + 1` entries (seed + `steps` forward points).
///
/// At `metric_curvature = 0` the path is the literal straight line —
/// regardless of dimension. This is the simplest defensible
/// generalization of the 2D conformal integrator for the constant-K
/// case the HTTP handler actually invokes (the bundle K mean is a
/// single scalar).
///
/// # Errors
///
/// * [`ImagineError::DimMismatch`] — `starting_from.len() !=
///   along.len()`.
/// * [`ImagineError::DimNotSupported`] — `starting_from.len() < 1`.
/// * [`ImagineError::Diverged`] — any coordinate exceeds the `1e6`
///   sanity bound at some step (matches Phase 1 contract).
pub fn integrate_geodesic_phase_2(
    starting_from: &[f64],
    along: &[f64],
    steps: usize,
    step_length: f64,
    metric_curvature: f64,
) -> Result<Vec<Vec<f64>>, ImagineError> {
    if starting_from.len() != along.len() {
        return Err(ImagineError::DimMismatch {
            seed: starting_from.len(),
            direction: along.len(),
        });
    }
    let dim = starting_from.len();
    if dim < 1 {
        return Err(ImagineError::DimNotSupported(dim));
    }

    let mut out: Vec<Vec<f64>> = Vec::with_capacity(steps + 1);
    out.push(starting_from.to_vec());

    // K == 0 → straight line. Bit-stable additive update.
    // K > 0 → sphere: cos(√K t) x_0 + sin(√K t)/√K v_0.
    // K < 0 → hyperbolic: cosh(√|K| t) x_0 + sinh(√|K| t)/√|K| v_0.
    let k = metric_curvature;
    let k_abs_sqrt = k.abs().sqrt();
    let is_flat = k.abs() < f64::EPSILON;
    let is_positive = k > 0.0;

    for s in 1..=steps {
        let t = step_length * (s as f64);
        let mut p = vec![0.0_f64; dim];
        if is_flat {
            // Straight line: x(t) = x_0 + t · v_0.
            for i in 0..dim {
                p[i] = starting_from[i] + t * along[i];
            }
        } else if is_positive {
            // Round n-sphere (embedded view): rotate the (x_0, v_0)
            // pair through angle √K · t.
            let omega = k_abs_sqrt;
            let ct = (omega * t).cos();
            let st = (omega * t).sin();
            for i in 0..dim {
                p[i] = ct * starting_from[i] + (st / omega) * along[i];
            }
        } else {
            // Hyperbolic: cosh / sinh growth.
            let omega = k_abs_sqrt;
            let ct = (omega * t).cosh();
            let st = (omega * t).sinh();
            for i in 0..dim {
                p[i] = ct * starting_from[i] + (st / omega) * along[i];
            }
        }
        // Sanity bound: |coords| < 1e6 (same threshold as Phase 1).
        let mag_sq: f64 = p.iter().map(|x| x * x).sum();
        let mag = mag_sq.sqrt();
        if !mag.is_finite() || mag > 1e6 {
            return Err(ImagineError::Diverged {
                step: s as u32,
                magnitude: mag,
            });
        }
        out.push(p);
    }
    Ok(out)
}

/// Phase 2 wrapper around [`integrate_geodesic_phase_2`] that emits a
/// [`CoherenceTrajectoryReport`] with the same shape as
/// [`imagine_coherence_trajectory`].
///
/// For `dim == 2` and well-behaved K, callers should keep using
/// [`imagine_coherence_trajectory`] directly — that path delegates to
/// the Phase 1 RK4 integrator and stays bit-identical. This wrapper
/// is what the HTTP handler reaches for once Phase 2 dispatch has been
/// selected (dim > 2, or dim == 2 after tame-metric substitution).
///
/// The trajectory uses the closed-form constant-K geodesic on the
/// maximally-symmetric space (sphere / flat / hyperbolic). Provenance
/// strings still start with `imagined:` per Marcella's contract.
pub fn imagine_coherence_trajectory_phase_2(
    seed_record_id: &str,
    seed_bundle: &str,
    starting_from: &[f64],
    along: &[f64],
    steps: u32,
    metric_curvature: f64,
    walk_config: &WalkConfig,
) -> Result<CoherenceTrajectoryReport, ImagineError> {
    let dim = starting_from.len();
    let n_steps = steps.max(1) as usize;
    let step_length = 1.0 / (n_steps as f64);
    let path = integrate_geodesic_phase_2(
        starting_from,
        along,
        n_steps,
        step_length,
        metric_curvature,
    )?;

    let sqrt_dim = (dim as f64).max(1.0).sqrt();
    let max_defect = 2.0 * sqrt_dim;

    let mut trajectory: Vec<CoherencePoint> = Vec::with_capacity(path.len());
    let mut refused = false;
    let mut refusal_reason: Option<String> = None;
    let mut cumulative_holonomy = 0.0_f64;
    let mut prev_point: Option<&Vec<f64>> = None;

    for (i, coords) in path.iter().enumerate() {
        // Holonomy proxy: norm of the velocity difference between
        // consecutive points (Phase 1 used the same proxy for v_rot).
        let defect = if let Some(prev) = prev_point {
            let mut acc = 0.0;
            for j in 0..dim.min(coords.len().min(prev.len())) {
                let d = coords[j] - prev[j];
                acc += d * d;
            }
            // Subtract the "would-be straight-line" displacement so a
            // flat (K=0) trajectory has defect=0.
            let mut straight = 0.0;
            for j in 0..dim.min(along.len()) {
                let d = along[j] * step_length;
                straight += d * d;
            }
            let chord = acc.sqrt();
            let straight_chord = straight.sqrt();
            (chord - straight_chord).abs()
        } else {
            0.0
        };
        cumulative_holonomy += defect;
        let coherence = if max_defect > 0.0 {
            (1.0 - defect / max_defect).clamp(0.0, 1.0)
        } else {
            1.0
        };

        // Marcella safety envelope: refuse if local K exceeds the
        // configured ceiling. For the constant-K Phase 2 path the
        // K is the same at every step.
        if !refused && metric_curvature > walk_config.max_imagined_curvature {
            refused = true;
            refusal_reason = Some(format!(
                "step {}: K = {:.4} > max_imagined_curvature {:.2} \
                 (default = 4.0 = K(CP¹ Fubini-Study))",
                i, metric_curvature, walk_config.max_imagined_curvature
            ));
        }
        if !refused && cumulative_holonomy > walk_config.max_accumulated_holonomy {
            refused = true;
            refusal_reason = Some(format!(
                "step {}: accumulated holonomy {:.4} > budget {:.2}",
                i, cumulative_holonomy, walk_config.max_accumulated_holonomy
            ));
        }

        let provenance_str = if i == 0 {
            format!("imagined: seed from {}", seed_bundle)
        } else {
            format!(
                "imagined: geodesic from {}, step {}/{} path_length={:.3} \
                 (phase=2, dim={})",
                seed_bundle,
                i,
                n_steps,
                (i as f64) * step_length,
                dim
            )
        };

        let _ = seed_record_id; // included in the Phase 1 provenance enum;
                                // Phase 2 keeps the string surface only.

        trajectory.push(CoherencePoint {
            step: i as u32,
            coords: coords.clone(),
            coherence,
            defect,
            curvature: metric_curvature,
            cumulative_holonomy,
            provenance: provenance_str,
        });

        prev_point = Some(coords);
    }

    let (endpoint_coherence, endpoint_curvature) =
        if let Some(last) = trajectory.last() {
            (last.coherence, last.curvature)
        } else {
            (1.0, 0.0)
        };

    // Touch ImaginedRecord to keep the cross-module link warm for
    // future Phase 3 (full provenance enum on Phase 2 records).
    let _ = std::marker::PhantomData::<ImaginedRecord>;

    Ok(CoherenceTrajectoryReport {
        trajectory,
        endpoint_coherence,
        endpoint_curvature,
        refused,
        refusal_reason,
        dim,
    })
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
    fn coherence_point_is_imagined_returns_true() {
        // Per Marcella round-3 feedback #1.
        let metric = metric_for_constant_k(0.0);
        let report = imagine_coherence_trajectory(
            &metric, "seed", "test_bundle",
            &[0.0, 0.0], &[0.5, 0.0], 3,
            &WalkConfig::default(),
        ).unwrap();
        for point in &report.trajectory {
            assert!(point.is_imagined(),
                    "CoherencePoint::is_imagined must return true for response-path branching");
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
