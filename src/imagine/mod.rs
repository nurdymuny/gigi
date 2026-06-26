//! Extrapolation verbs on the geometric substrate.
//!
//! IMAGINE and WALK — the layer of the verb stack that constructs
//! points the substrate has not seen by extending the geometric
//! structure (geodesics, parallel transport, double cover) forward
//! from points it *has* seen.
//!
//! **Cognitive analog (Bee's framing):** humans imagine the path
//! before walking it. We solve a geodesic in our head — *given where
//! I am and which way I'm facing, what comes next?* — and we describe
//! the path before we move. The math GIGI already has is the engine
//! that does this. This module names it.
//!
//! **Trust envelope (Marcella's two highest-priority feedback items):**
//!
//! 1. **Provenance is load-bearing.** Every [`ImaginedRecord`] carries
//!    a required [`ImaginedProvenance`] enum that travels through
//!    every downstream consumer — cite rendering, audit logs, the
//!    LOCAL_HOLONOMY history. Imagined records cannot be silently
//!    rendered as retrieved records. See [`provenance`] module.
//!
//! 2. **`max_imagined_curvature` is a required field on `WalkConfig`,
//!    default 4.0 = K(CP¹ Fubini-Study).** Walking into regions of
//!    higher Gaussian curvature than complex projective space requires
//!    explicit opt-in. See [`config`] module.
//!
//! **Math gates** (all GREEN as of commit `036a586`):
//! - T11: Geodesic integrator on S²/T²/CP¹ — `t11_geodesic_integrator.py`
//! - T12: Halo-as-IMAGINE makes sharded CURVATURE partition-invariant
//!   (exactly zero residual across {2, 4, 8} chart partitions) —
//!   `t12_halo_partition_invariance.py`
//! - T13: Double cover monodromy with discourse-state seam at
//!   `act_history=("qy",)` — `t13_double_cover_monodromy.py`
//!
//! See [`theory/imagine/IMAGINE_AND_WALK.md`] for the full spec.

#![cfg(feature = "imagine")]

pub mod coherence;
pub mod config;
pub mod geodesic;
pub mod halo;
pub mod observables;
pub mod path_registry;
pub mod provenance;
pub mod routing;
pub mod walk;

#[cfg(feature = "wish")]
pub mod wish;

pub use observables::{
    evaluate_canonical as evaluate_observable_canonical, is_canonical as is_canonical_observable,
    trapezoidal_integrate as integrate_observable_along, ObservableError, CANONICAL_NAMES,
};
pub use path_registry::{
    bind as bind_path, clear as clear_paths, get as get_path, list as list_paths,
    unbind as unbind_path, BoundPath, PathSource,
};

pub use coherence::{
    imagine_coherence_trajectory, imagine_coherence_trajectory_phase_2, integrate_geodesic_phase_2,
    metric_for_constant_k, CoherencePoint, CoherenceTrajectoryReport, K_MAX_PHASE2, K_TAME_PHASE2,
};
pub use config::{
    CurvatureGateRaisedAboveDefault, HaloConfig, ImagineConfig, WalkConfig,
};
pub use geodesic::{imagine_geodesic, ConformalMetric, ImagineError};
pub use halo::imagine_halo;
pub use provenance::{
    ImaginedProvenance, ImaginedRecord, WishBlockReason, WishTargetProvenance,
    WishWaypointInfo,
};
pub use routing::{
    route_forecast_or_imagine, RoutingAdvisory, RoutingDecision, THETA_DENSITY,
};
pub use walk::{walk, WalkError, WalkOutcome};
