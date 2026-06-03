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
pub mod provenance;
pub mod walk;

pub use coherence::{
    imagine_coherence_trajectory, metric_for_constant_k, CoherencePoint, CoherenceTrajectoryReport,
};
pub use config::{HaloConfig, ImagineConfig, WalkConfig};
pub use geodesic::{imagine_geodesic, ConformalMetric, ImagineError};
pub use halo::imagine_halo;
pub use provenance::{ImaginedProvenance, ImaginedRecord};
pub use walk::{walk, WalkError, WalkOutcome};
