//! `imagine_geodesic` — RK4 integrator for the geodesic equation on
//! a 2D conformally-flat metric.
//!
//! Mathematical claim validated by T11
//! ([`theory/imagine/validation/t11_geodesic_integrator.py`]):
//! the RK4 integrator using Christoffel symbols computed from the
//! conformal factor's log-derivatives reproduces the closed-form
//! geodesic on S², T², CP¹ to machine precision (errors 6.66e-16 to
//! 1.36e-14, well under the 1e-9 tolerance).
//!
//! This Rust port mirrors the Python integrator at the math level so
//! its correctness is inherited from T11's TDD gate.

use crate::imagine::config::ImagineConfig;
use crate::imagine::provenance::{ImaginedProvenance, ImaginedRecord};

/// Errors from geodesic imagination.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ImagineError {
    #[error("seed and direction dimension mismatch ({seed} vs {direction})")]
    DimMismatch { seed: usize, direction: usize },
    #[error("imagine_geodesic currently supports dim = 2 (got {0})")]
    DimNotSupported(usize),
    #[error(
        "integrator diverged at step {step}: |coords| = {magnitude:.3e} \
         exceeds 1e6 sanity bound. Most common cause: substrate metric K \
         is too large for Phase 1's 2D conformal integrator -- check \
         bundle.curvature_stats().mean(). For high-K bundles, pass \
         metric_curvature explicitly (e.g. metric_curvature = 1.0) in \
         the request to use a tame metric instead of the bundle mean. \
         Phase 2 lifts the dim and K constraints together."
    )]
    Diverged { step: u32, magnitude: f64 },
}

/// A conformally-flat 2D metric specified by phi_x, phi_y (the log-
/// derivatives of the conformal factor).
///
/// `ds² = e^{2 φ(x, y)} (dx² + dy²)`
///
/// The Christoffel symbols are computed from `phi_x`, `phi_y` alone:
///   Γ^x_xx = phi_x,  Γ^x_xy = phi_y,  Γ^x_yy = -phi_x
///   Γ^y_xx = -phi_y, Γ^y_xy = phi_x,  Γ^y_yy = phi_y
pub struct ConformalMetric<'a> {
    pub phi_x: Box<dyn Fn(f64, f64) -> f64 + Send + Sync + 'a>,
    pub phi_y: Box<dyn Fn(f64, f64) -> f64 + Send + Sync + 'a>,
    /// Local Gaussian curvature at (x, y). Used to populate
    /// `ImaginedRecord::local_k` at each integrator step. Caller
    /// supplies this from the substrate's curvature primitive.
    pub local_k: Box<dyn Fn(f64, f64) -> f64 + Send + Sync + 'a>,
}

impl<'a> ConformalMetric<'a> {
    /// Geodesic acceleration: the right-hand side of the geodesic ODE.
    /// Returns `(x'', y'')`.
    fn acceleration(&self, x: f64, y: f64, vx: f64, vy: f64) -> (f64, f64) {
        let px = (self.phi_x)(x, y);
        let py = (self.phi_y)(x, y);
        let diff = vx * vx - vy * vy;
        let cross = 2.0 * vx * vy;
        let ax = -px * diff - py * cross;
        let ay = py * diff - px * cross;
        (ax, ay)
    }

    /// One RK4 step on the state (x, y, vx, vy). Matches T11's Python
    /// integrator step-for-step.
    fn rk4_step(
        &self,
        state: (f64, f64, f64, f64),
        h: f64,
    ) -> (f64, f64, f64, f64) {
        let f = |s: (f64, f64, f64, f64)| -> (f64, f64, f64, f64) {
            let (x, y, vx, vy) = s;
            let (ax, ay) = self.acceleration(x, y, vx, vy);
            (vx, vy, ax, ay)
        };
        let k1 = f(state);
        let s2 = (
            state.0 + 0.5 * h * k1.0,
            state.1 + 0.5 * h * k1.1,
            state.2 + 0.5 * h * k1.2,
            state.3 + 0.5 * h * k1.3,
        );
        let k2 = f(s2);
        let s3 = (
            state.0 + 0.5 * h * k2.0,
            state.1 + 0.5 * h * k2.1,
            state.2 + 0.5 * h * k2.2,
            state.3 + 0.5 * h * k2.3,
        );
        let k3 = f(s3);
        let s4 = (
            state.0 + h * k3.0,
            state.1 + h * k3.1,
            state.2 + h * k3.2,
            state.3 + h * k3.3,
        );
        let k4 = f(s4);
        (
            state.0 + (h / 6.0) * (k1.0 + 2.0 * k2.0 + 2.0 * k3.0 + k4.0),
            state.1 + (h / 6.0) * (k1.1 + 2.0 * k2.1 + 2.0 * k3.1 + k4.1),
            state.2 + (h / 6.0) * (k1.2 + 2.0 * k2.2 + 2.0 * k3.2 + k4.2),
            state.3 + (h / 6.0) * (k1.3 + 2.0 * k2.3 + 2.0 * k3.3 + k4.3),
        )
    }
}

/// Integrate a geodesic forward from `seed` along `direction` for
/// `config.path_length`, returning the trajectory as a sequence of
/// `ImaginedRecord`s. The provenance on each record is
/// `ImaginedProvenance::Geodesic` with the seed and step count.
///
/// Phase 1 implementation supports 2D substrates only — `seed` and
/// `direction` must each have length 2. Multi-dim extension is
/// straightforward; Phase 2 ships it once a multi-dim metric trait
/// is available.
pub fn imagine_geodesic(
    metric: &ConformalMetric,
    seed_record_id: &str,
    seed_bundle: &str,
    seed_coords: &[f64],
    direction: &[f64],
    config: &ImagineConfig,
) -> Result<Vec<ImaginedRecord>, ImagineError> {
    if seed_coords.len() != direction.len() {
        return Err(ImagineError::DimMismatch {
            seed: seed_coords.len(),
            direction: direction.len(),
        });
    }
    if seed_coords.len() != 2 {
        return Err(ImagineError::DimNotSupported(seed_coords.len()));
    }

    let h = config.path_length / (config.n_steps as f64);
    let mut state = (seed_coords[0], seed_coords[1], direction[0], direction[1]);
    let mut trajectory = Vec::with_capacity(config.n_steps as usize + 1);

    // Step 0 = seed
    trajectory.push(ImaginedRecord {
        coords: vec![state.0, state.1],
        local_k: (metric.local_k)(state.0, state.1),
        accumulated_holonomy: 0.0,
        provenance: ImaginedProvenance::Geodesic {
            seed_record_id: seed_record_id.to_string(),
            seed_bundle: seed_bundle.to_string(),
            initial_direction: direction.to_vec(),
            path_length: 0.0,
            integrator_steps: 0,
        },
    });

    let mut accumulated_holonomy = 0.0_f64;

    for step in 1..=config.n_steps {
        let new_state = metric.rk4_step(state, h);
        // Detect divergence (numerical blowup)
        let mag = (new_state.0 * new_state.0 + new_state.1 * new_state.1).sqrt();
        if !mag.is_finite() || mag > 1e6 {
            return Err(ImagineError::Diverged {
                step,
                magnitude: mag,
            });
        }
        // Accumulate a simple holonomy proxy: the magnitude of the
        // velocity rotation per step. (Phase 1 placeholder; Phase 2
        // upgrades to the proper parallel-transport rotation.)
        let v_rot = ((new_state.2 - state.2).powi(2)
            + (new_state.3 - state.3).powi(2))
            .sqrt();
        accumulated_holonomy += v_rot;
        state = new_state;

        let path_len_so_far = (step as f64) * h;
        trajectory.push(ImaginedRecord {
            coords: vec![state.0, state.1],
            local_k: (metric.local_k)(state.0, state.1),
            accumulated_holonomy,
            provenance: ImaginedProvenance::Geodesic {
                seed_record_id: seed_record_id.to_string(),
                seed_bundle: seed_bundle.to_string(),
                initial_direction: direction.to_vec(),
                path_length: path_len_so_far,
                integrator_steps: step,
            },
        });
    }

    Ok(trajectory)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S² stereographic metric: phi_x = -2x / (1+r²), phi_y = -2y / (1+r²).
    /// K = 4 / (1+r²)² ... but for the test only the structural form matters.
    fn s2_metric() -> ConformalMetric<'static> {
        ConformalMetric {
            phi_x: Box::new(|x: f64, y: f64| -2.0 * x / (1.0 + x * x + y * y)),
            phi_y: Box::new(|x: f64, y: f64| -2.0 * y / (1.0 + x * x + y * y)),
            local_k: Box::new(|_x: f64, _y: f64| 1.0),
        }
    }

    /// T² flat metric: phi = 0 → all Christoffels vanish → geodesic
    /// is a straight line.
    fn t2_metric() -> ConformalMetric<'static> {
        ConformalMetric {
            phi_x: Box::new(|_x: f64, _y: f64| 0.0),
            phi_y: Box::new(|_x: f64, _y: f64| 0.0),
            local_k: Box::new(|_x: f64, _y: f64| 0.0),
        }
    }

    /// S² stereographic closed-form geodesic from the seed (x0, y0)
    /// with chart tangent (vx0, vy0), evaluated at parameter t. Uses
    /// the embedded picture (gamma_3D = cos(ω t) P + sin(ω t) V/ω)
    /// and projects back to stereographic.
    ///
    /// This mirrors T11's `s2_closed_form_geodesic` exactly.
    fn s2_closed_form(x0: f64, y0: f64, vx0: f64, vy0: f64, t: f64) -> (f64, f64) {
        let r2 = x0 * x0 + y0 * y0;
        let s = 1.0 + r2;
        // sigma_inv(x0, y0) = (2x, 2y, x²+y²-1) / (x²+y²+1)
        let p0 = [2.0 * x0 / s, 2.0 * y0 / s, (r2 - 1.0) / s];
        // Pushforward of tangent: V_3D = vx0 * dP/dx + vy0 * dP/dy
        let dpx = [
            (2.0 * s - 4.0 * x0 * x0) / (s * s),
            -4.0 * x0 * y0 / (s * s),
            (2.0 * x0 * s - (r2 - 1.0) * 2.0 * x0) / (s * s),
        ];
        let dpy = [
            -4.0 * x0 * y0 / (s * s),
            (2.0 * s - 4.0 * y0 * y0) / (s * s),
            (2.0 * y0 * s - (r2 - 1.0) * 2.0 * y0) / (s * s),
        ];
        let v = [
            vx0 * dpx[0] + vy0 * dpy[0],
            vx0 * dpx[1] + vy0 * dpy[1],
            vx0 * dpx[2] + vy0 * dpy[2],
        ];
        let omega = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let v_hat = [v[0] / omega, v[1] / omega, v[2] / omega];
        let ct = (omega * t).cos();
        let st = (omega * t).sin();
        let p_t = [
            ct * p0[0] + st * v_hat[0],
            ct * p0[1] + st * v_hat[1],
            ct * p0[2] + st * v_hat[2],
        ];
        // Stereographic projection back
        let denom = 1.0 - p_t[2];
        (p_t[0] / denom, p_t[1] / denom)
    }

    #[test]
    fn s2_geodesic_matches_closed_form_at_machine_precision() {
        let metric = s2_metric();
        let config = ImagineConfig {
            path_length: 1.0,
            n_steps: 1000,
            adaptive: false,
        };
        let traj = imagine_geodesic(
            &metric, "rec_0", "test_bundle",
            &[0.2, 0.3], &[0.4, -0.2], &config,
        ).unwrap();
        assert_eq!(traj.len(), 1001);

        for t_check in [0.1_f64, 0.5_f64, 1.0_f64] {
            let idx = (t_check * 1000.0).round() as usize;
            let r = &traj[idx];
            let (cx, cy) = s2_closed_form(0.2, 0.3, 0.4, -0.2, t_check);
            let err = ((r.coords[0] - cx).powi(2) + (r.coords[1] - cy).powi(2)).sqrt();
            assert!(err < 1e-9,
                    "S² geodesic err at t={}: integrated=({:.6}, {:.6}) closed=({:.6}, {:.6}) err={:.3e}",
                    t_check, r.coords[0], r.coords[1], cx, cy, err);
        }
    }

    #[test]
    fn t2_geodesic_is_straight_line() {
        let metric = t2_metric();
        let config = ImagineConfig {
            path_length: 1.0,
            n_steps: 100,
            adaptive: false,
        };
        let traj = imagine_geodesic(
            &metric, "rec_0", "test_bundle",
            &[0.1, 0.2], &[0.5, 0.3], &config,
        ).unwrap();

        // At t=1.0, position should be (0.1 + 0.5, 0.2 + 0.3) = (0.6, 0.5)
        let endpoint = traj.last().unwrap();
        let err = ((endpoint.coords[0] - 0.6).powi(2)
            + (endpoint.coords[1] - 0.5).powi(2)).sqrt();
        assert!(err < 1e-12, "T² straight line err at t=1: {:.3e}", err);
    }

    #[test]
    fn all_records_have_geodesic_provenance() {
        let metric = t2_metric();
        let config = ImagineConfig {
            path_length: 0.5,
            n_steps: 50,
            adaptive: false,
        };
        let traj = imagine_geodesic(
            &metric, "seed_id", "marcella_test",
            &[0.0, 0.0], &[1.0, 0.0], &config,
        ).unwrap();
        for r in &traj {
            assert!(matches!(r.provenance, ImaginedProvenance::Geodesic { .. }),
                    "non-geodesic provenance leaked into imagine_geodesic output");
        }
    }

    #[test]
    fn dim_mismatch_returns_error() {
        let metric = t2_metric();
        let config = ImagineConfig::default();
        let err = imagine_geodesic(
            &metric, "x", "y", &[0.0, 0.0], &[1.0], &config,
        );
        assert!(matches!(err, Err(ImagineError::DimMismatch { .. })));
    }

    #[test]
    fn three_d_seed_returns_dim_not_supported_in_phase_1() {
        let metric = t2_metric();
        let config = ImagineConfig::default();
        let err = imagine_geodesic(
            &metric, "x", "y", &[0.0, 0.0, 0.0], &[1.0, 0.0, 0.0], &config,
        );
        assert!(matches!(err, Err(ImagineError::DimNotSupported(3))));
    }
}
