//! Atlas-cover sharding model for GIGI.
//!
//! This module is the Phase A skeleton for sharded GIGI as specified in
//! [`theory/poincare_to_sharding/SHARDING_SPEC.md`]. Types are fully
//! defined; execution bodies are `todo!()` until Phase B wires them into
//! `BundleStore`.
//!
//! All math in this module is gated by TDD claims under
//! `theory/poincare_to_sharding/validation/`:
//!
//! - T1 (`t1_mayer_vietoris_betti.py`): sharded BETTI via Mayer-Vietoris
//! - T2 (`t2_cocycle_bound.py`): cocycle bound on chart transitions
//! - T3 (`t3_sharded_curvature.py`): per-chart CURVATURE = global K
//! - T4 (`t4_sharded_holonomy.py`): closed-loop HOLONOMY w/ gauge transition
//! - T5 (`t5_cauchy_interlacing_lambda1.py`): honest λ_1 bounds
//! - T6 (`t6_clean_finger_move.py`): conflict resolver termination
//! - T7 (`t7_distributed_lanczos_spectral.py`): distributed Lanczos
//!   universal sharded SPECTRAL
//!
//! All seven tests GREEN as of commit `1aba0d8`.
//!
//! References:
//! - Davis, B. R. (2026a). *The Davis Manifold*.
//! - Davis, B. R. (2026b). *The Geometry of Sameness*.
//! - Davis, B. R. (2026c). *The Smooth 4D Poincaré Conjecture*.

#![cfg(feature = "sharded")]

pub mod atlas;
pub mod execution;
pub mod fiedler;
pub mod gates;
pub mod regime;
pub mod resolver;
pub mod sharded_bundle;

pub use atlas::{Atlas, ChartId, ChartMetadata, Transition};
pub use execution::{shard_curvature, ShardedCurvatureReport, ShardedExecError};
#[cfg(feature = "kahler")]
pub use execution::{shard_betti_disjoint, ShardedBettiReport};
pub use fiedler::{fiedler_partition, FiedlerConfig, FiedlerError};
pub use gates::{non_vacuity_check, GateError};
pub use regime::SpectralRegime;
pub use resolver::{sharded_write_resolve, ResolverError, ResolverTrace, WriteConflict};
pub use sharded_bundle::ShardedBundle;

/// Identifier for a shard (a process / machine / storage volume holding
/// one or more charts).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ShardId(pub u32);
