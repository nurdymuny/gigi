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

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            use_double_cover: true,
            sudoku_preflight: true,
            // 4.0 = K(CP^1 Fubini-Study). See doc comment above.
            max_imagined_curvature: 4.0,
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
    fn imagine_config_defaults_match_t11_gate() {
        let c = ImagineConfig::default();
        assert_eq!(c.n_steps, 1000, "1000 steps matches T11's machine-precision result");
    }
}
