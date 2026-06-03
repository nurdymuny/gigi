//! Per-verb sharded execution stubs.
//!
//! Phase A: function signatures + `todo!()` bodies. Phase C (BETTI),
//! Phase D (CURVATURE et al.), and the SPECTRAL sprint fill these in
//! with the recipes from `SHARDING_SPEC.md` §5.

use crate::sharded::atlas::{Atlas, ChartId};

/// Sharded execution errors.
#[derive(Debug, Clone, PartialEq)]
pub enum ShardedExecError {
    /// SPECTRAL was queried on an `Expander`-regime atlas without the
    /// distributed-Lanczos path being implemented in this phase.
    /// Phase E lifts this restriction.
    ExpanderRegimeUnsupportedSpectral,
    /// A point or query targets a chart that does not exist in the
    /// atlas.
    UnknownChart { chart: ChartId },
    /// The atlas is missing a transition between two charts that the
    /// query traversal requires.
    MissingTransition { from: ChartId, to: ChartId },
    /// Phase A stub; will be replaced by concrete implementations.
    NotImplementedYet { phase: &'static str },
}

/// CURVATURE at a point in a sharded bundle (T3 §3.3).
///
/// Sharded recipe: locate the chart containing the point, ask that
/// shard to compute K from its local metric. No inter-shard coordination.
///
/// Phase A stub.
pub fn shard_curvature_at(
    _atlas: &Atlas,
    _chart: ChartId,
    _point_in_chart_coords: &[f64],
) -> Result<f64, ShardedExecError> {
    Err(ShardedExecError::NotImplementedYet { phase: "Phase D" })
}

/// HOLONOMY around a closed loop crossing chart boundaries (T4 §3.4).
///
/// Sharded recipe: per-chart parallel transport, with the atlas's
/// transition function applied at each seam crossing. Gauge invariance
/// of closed-loop holonomy ensures the result equals direct integration
/// in a single global gauge.
///
/// Phase A stub.
pub fn shard_holonomy_around_loop(
    _atlas: &Atlas,
    _loop_points_with_charts: &[(ChartId, Vec<f64>)],
) -> Result<Vec<f64>, ShardedExecError> {
    Err(ShardedExecError::NotImplementedYet { phase: "Phase C" })
}

/// BETTI numbers of a sharded bundle (T1 §3.1).
///
/// Sharded recipe: each shard reports its local chain-complex boundary
/// matrices; the consumer assembles via Mayer-Vietoris. Exact recovery
/// of global Betti numbers from per-chart data.
///
/// Phase A stub.
pub fn shard_betti(_atlas: &Atlas, _max_dim: usize) -> Result<Vec<u32>, ShardedExecError> {
    Err(ShardedExecError::NotImplementedYet { phase: "Phase C" })
}

/// λ_1 of the bundle's Laplacian (T5 §3.5 / T7 §3.5 follow-up).
///
/// Sharded recipe is regime-routed:
/// - `SpectralRegime::NaturallyCluster` or `CertifiedClusteredAt`:
///   compute per-shard λ_1, take min. O(1 round).
/// - `SpectralRegime::Expander`: distributed Lanczos (T7). O(K rounds).
///
/// Phase A stub.
pub fn shard_lambda_1(atlas: &Atlas) -> Result<f64, ShardedExecError> {
    if atlas.spectral_regime.requires_distributed_lanczos() {
        // Phase E ships the distributed Lanczos path; until then,
        // the engine refuses rather than silently lying.
        return Err(ShardedExecError::ExpanderRegimeUnsupportedSpectral);
    }
    Err(ShardedExecError::NotImplementedYet { phase: "Phase D" })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharded::regime::SpectralRegime;
    use crate::sharded::ShardId;

    #[test]
    fn lambda_1_on_expander_refuses_in_phase_a() {
        let atlas = Atlas::new(SpectralRegime::Expander, 0.0);
        let err = shard_lambda_1(&atlas);
        assert_eq!(err, Err(ShardedExecError::ExpanderRegimeUnsupportedSpectral));
    }

    #[test]
    fn lambda_1_on_clustered_returns_not_implemented_in_phase_a() {
        let atlas = Atlas::new(SpectralRegime::NaturallyCluster, 0.0);
        let err = shard_lambda_1(&atlas);
        assert!(matches!(err, Err(ShardedExecError::NotImplementedYet { .. })));
    }

    #[test]
    fn curvature_returns_not_implemented() {
        let atlas = Atlas::trivial(ShardId(0));
        let err = shard_curvature_at(&atlas, ChartId(0), &[0.0, 0.0]);
        assert!(matches!(err, Err(ShardedExecError::NotImplementedYet { .. })));
    }
}
