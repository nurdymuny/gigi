//! Config types for IMAGINE and WALK.
//!
//! `WalkConfig::max_imagined_curvature` is the load-bearing safety
//! field per Marcella's feedback #3. Default 4.0 = K(CP¹ Fubini-Study).

use serde::{Deserialize, Serialize};

/// Configuration for `imagine_geodesic`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImagineConfig {
    /// Total path length to integrate.
    pub path_length: f64,
    /// Number of RK4 integrator steps. 1000 matches T11's gate
    /// (machine-precision agreement with closed forms).
    pub n_steps: u32,
    /// If true, adaptive step size based on local curvature
    /// (Phase 2; current Phase 1 uses fixed step).
    pub adaptive: bool,
}

impl Default for ImagineConfig {
    fn default() -> Self {
        Self {
            path_length: 1.0,
            n_steps: 1000,
            adaptive: false,
        }
    }
}

/// Configuration for `imagine_halo`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HaloConfig {
    /// Maximum number of records to project per chart pair. Bounds
    /// the halo size to control storage overhead.
    pub max_halo_records: usize,
    /// Number of nearest neighbors used in halo selection. Should
    /// match the k used by per-record K computation in the substrate.
    pub k_neighbors: usize,
}

impl Default for HaloConfig {
    fn default() -> Self {
        Self {
            max_halo_records: 64,
            k_neighbors: 8,
        }
    }
}

/// Configuration for `walk`.
///
/// **Trust envelope (Marcella feedback #3 — load-bearing).**
/// `max_imagined_curvature` is required, default 4.0.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalkConfig {
    /// Lift the path to the double cover if any seam crossing has
    /// non-trivial Z₂ monodromy. Strongly recommended.
    pub use_double_cover: bool,

    /// Run the SUDOKU pre-flight check on the endpoint constraints.
    pub sudoku_preflight: bool,

    /// Maximum Gaussian curvature K allowed at any imagined point
    /// along the walked path.
    ///
    /// **Default 4.0 = K(CP¹ Fubini-Study).** Walking into regions of
    /// higher Gaussian curvature than complex projective space
    /// requires explicit opt-in. CP¹ is the natural ceiling because
    /// it is the simplest closed Kähler manifold the substrate
    /// supports (see `theory/poincare_to_sharding/validation/t3_sharded_curvature.py`
    /// — K = 4 exactly is the closed-form CP¹ FS curvature). Anything
    /// more curved than CP¹ is, by construction, more curved than the
    /// substrate has been calibrated for, so the engine refuses
    /// rather than walking into unmapped regime.
    pub max_imagined_curvature: f64,

    /// Maximum accumulated holonomy budget along the walked path.
    pub max_accumulated_holonomy: f64,

    /// Whether to materialize the imagined records as real records
    /// in the substrate post-walk. Default false — WALK is
    /// observation, not commit.
    pub materialize_on_success: bool,
}

impl WalkConfig {
    /// The default `max_imagined_curvature`. 4.0 = K(CP¹ Fubini-Study),
    /// the simplest closed Kähler manifold the substrate is calibrated
    /// for. Anchored as a `const` so callers (and audit code) can
    /// compare against it without re-typing the magic number.
    pub const DEFAULT_MAX_IMAGINED_CURVATURE: f64 = 4.0;

    /// If `max_imagined_curvature` has been raised above the default
    /// trust ceiling, return the audit struct that should be logged.
    /// Returns `None` when the threshold is at or below default — the
    /// no-drift case.
    ///
    /// Per Marcella round-3 feedback #2: refusal-on-too-curved fires
    /// when curvature exceeds the threshold, but if the threshold
    /// itself is silently raised the trust envelope has a gap. This
    /// method gives operations a deterministic way to detect drift:
    /// every endpoint that consumes a `WalkConfig` should consult
    /// `audit_threshold_drift()` and propagate the result to its
    /// response (and/or log).
    pub fn audit_threshold_drift(&self) -> Option<CurvatureGateRaisedAboveDefault> {
        if self.max_imagined_curvature > Self::DEFAULT_MAX_IMAGINED_CURVATURE {
            Some(CurvatureGateRaisedAboveDefault {
                configured: self.max_imagined_curvature,
                default: Self::DEFAULT_MAX_IMAGINED_CURVATURE,
            })
        } else {
            None
        }
    }
}

/// Audit-log signal emitted (in the response payload) when a caller
/// has raised `max_imagined_curvature` above the default 4.0 trust
/// ceiling. Per Marcella round-3 feedback #2, this is the sibling
/// signal to `OverCurvatureRefused` — refusal-on-too-curved fires at
/// commit; threshold-raised-above-default fires at config time.
///
/// Surfaced in `imagine_coherence` HTTP responses as
/// `threshold_drift`. Production callers should route this to their
/// audit log; not opt-in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurvatureGateRaisedAboveDefault {
    /// The value the caller set in `WalkConfig::max_imagined_curvature`.
    pub configured: f64,
    /// The default trust ceiling that the caller has raised above
    /// (`WalkConfig::DEFAULT_MAX_IMAGINED_CURVATURE`).
    pub default: f64,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            use_double_cover: true,
            sudoku_preflight: true,
            // 4.0 = K(CP^1 Fubini-Study). See doc comment above.
            max_imagined_curvature: Self::DEFAULT_MAX_IMAGINED_CURVATURE,
            max_accumulated_holonomy: 0.5,
            materialize_on_success: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_config_default_max_curvature_is_cp1_fs_value() {
        // Load-bearing per Marcella feedback #3.
        let c = WalkConfig::default();
        assert_eq!(c.max_imagined_curvature, 4.0,
                   "default max_imagined_curvature must be K(CP^1 Fubini-Study) = 4.0");
    }

    #[test]
    fn walk_config_defaults_to_safe_values() {
        let c = WalkConfig::default();
        assert!(c.use_double_cover, "double cover must be on by default for safety");
        assert!(c.sudoku_preflight, "SUDOKU pre-flight must be on by default");
        assert!(!c.materialize_on_success, "walk is observation by default, not commit");
    }

    #[test]
    fn walk_config_serde_round_trips() {
        let c = WalkConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let back: WalkConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.max_imagined_curvature, c.max_imagined_curvature);
        assert_eq!(back.use_double_cover, c.use_double_cover);
    }

    #[test]
    fn audit_threshold_drift_is_none_at_default() {
        // Per Marcella round-3 feedback #2: no drift at default.
        let c = WalkConfig::default();
        assert!(c.audit_threshold_drift().is_none());
    }

    #[test]
    fn audit_threshold_drift_is_none_when_lowered() {
        // Lowering the threshold (more conservative) should NOT
        // emit a drift signal -- drift fires on raised ceilings, not
        // lowered ones.
        let c = WalkConfig {
            max_imagined_curvature: 2.0,
            ..WalkConfig::default()
        };
        assert!(c.audit_threshold_drift().is_none());
    }

    #[test]
    fn audit_threshold_drift_fires_when_raised_above_default() {
        // Per Marcella round-3 feedback #2: raising the threshold
        // silently is the trust-envelope gap. Catch it with an
        // explicit audit signal.
        let c = WalkConfig {
            max_imagined_curvature: 10.0,
            ..WalkConfig::default()
        };
        let drift = c.audit_threshold_drift().expect("drift must fire");
        assert_eq!(drift.configured, 10.0);
        assert_eq!(drift.default, 4.0);
    }

    #[test]
    fn audit_threshold_drift_default_constant_matches_kahler_ceiling() {
        // The constant must equal K(CP^1 FS) = 4.0 -- if this drifts,
        // every consumer that compares against it silently breaks.
        assert_eq!(WalkConfig::DEFAULT_MAX_IMAGINED_CURVATURE, 4.0);
    }

    #[test]
    fn curvature_gate_raised_above_default_serde_round_trips() {
        let drift = CurvatureGateRaisedAboveDefault {
            configured: 7.5,
            default: 4.0,
        };
        let json = serde_json::to_string(&drift).unwrap();
        let back: CurvatureGateRaisedAboveDefault = serde_json::from_str(&json).unwrap();
        assert_eq!(drift, back);
    }

    #[test]
    fn imagine_config_defaults_match_t11_gate() {
        let c = ImagineConfig::default();
        assert_eq!(c.n_steps, 1000, "1000 steps matches T11's machine-precision result");
    }
}
