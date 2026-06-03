//! Non-vacuity and cocycle-budget gates (Davis 2026a §A5 + 2026b Def 21).
//!
//! Phase A: gate functions are concrete; they consume the chart's
//! declared parameters and return Ok/Err. Phase B wires them into the
//! ingest path.

use crate::sharded::atlas::{Atlas, ChartId, ChartMetadata};

/// Reasons a non-vacuity gate can reject a chart configuration.
#[derive(Debug, Clone, PartialEq)]
pub enum GateError {
    /// Davis 2026a §A5: kappa_soft - 2 * epsilon * R <= 0.
    /// The configuration margin is destroyed by distortion.
    NonVacuityViolated {
        chart: ChartId,
        kappa_soft: f64,
        eps_dist: f64,
        geodesic_radius: f64,
    },
    /// Davis 2026b Def 21: observed cocycle slack exceeds declared
    /// budget. The atlas's chart transitions are inconsistent.
    CocycleBudgetExceeded {
        pair: (ChartId, ChartId),
        budget: f64,
        observed: f64,
    },
}

/// Per-chart non-vacuity check (Davis Manifold A5).
///
/// `kappa_soft - 2 * eps_dist * geodesic_radius > 0` must hold; otherwise
/// the configuration margin is consumed by distortion and the chart
/// cannot be trusted at its declared operational horizon.
///
/// Returns `Ok(())` if the chart satisfies the gate, `Err` otherwise.
pub fn non_vacuity_check(chart: &ChartMetadata, eps_dist: f64) -> Result<(), GateError> {
    let kappa = chart.kappa_soft;
    let r = chart.geodesic_radius;
    if kappa - 2.0 * eps_dist * r <= 0.0 {
        return Err(GateError::NonVacuityViolated {
            chart: chart.id,
            kappa_soft: kappa,
            eps_dist,
            geodesic_radius: r,
        });
    }
    Ok(())
}

/// Cocycle-budget gate (Davis 2026b Def 21).
///
/// Given an observed cocycle slack on an overlap, reject if it exceeds
/// the atlas's declared budget. Phase A: caller supplies the observed
/// value. Phase B: engine measures it directly from the transitions.
pub fn cocycle_budget_check(
    atlas: &Atlas,
    pair: (ChartId, ChartId),
    observed_slack: f64,
) -> Result<(), GateError> {
    if observed_slack > atlas.delta_cocycle_budget {
        return Err(GateError::CocycleBudgetExceeded {
            pair,
            budget: atlas.delta_cocycle_budget,
            observed: observed_slack,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharded::atlas::{ChartRegion, TransitionKey};
    use crate::sharded::regime::SpectralRegime;
    use crate::sharded::ShardId;
    use std::collections::HashMap;

    fn chart_meta(id: u32, kappa_soft: f64, geodesic_radius: f64) -> ChartMetadata {
        ChartMetadata {
            id: ChartId(id),
            shard_id: ShardId(0),
            region: ChartRegion::HashBucket { bucket_index: id, n_buckets: 4 },
            operational_horizon: 1.0,
            kappa_soft,
            geodesic_radius,
        }
    }

    #[test]
    fn non_vacuity_passes_when_margin_clear() {
        let c = chart_meta(0, /*kappa_soft*/ 1.0, /*R*/ 1.0);
        // eps * 2 * R = 0.2 * 2 = 0.4 < kappa_soft = 1.0
        assert!(non_vacuity_check(&c, 0.2).is_ok());
    }

    #[test]
    fn non_vacuity_fails_when_margin_eaten() {
        let c = chart_meta(1, /*kappa_soft*/ 0.5, /*R*/ 1.0);
        // eps * 2 * R = 0.3 * 2 = 0.6 > kappa_soft = 0.5
        let err = non_vacuity_check(&c, 0.3);
        assert!(matches!(err, Err(GateError::NonVacuityViolated { .. })));
    }

    #[test]
    fn cocycle_budget_passes_when_observed_under() {
        let atlas = Atlas {
            charts: HashMap::new(),
            transitions: HashMap::new(),
            delta_cocycle_budget: 0.1,
            spectral_regime: SpectralRegime::NaturallyCluster,
        };
        assert!(cocycle_budget_check(&atlas, (ChartId(0), ChartId(1)), 0.05).is_ok());
    }

    #[test]
    fn cocycle_budget_fails_when_observed_over() {
        let atlas = Atlas {
            charts: HashMap::new(),
            transitions: HashMap::new(),
            delta_cocycle_budget: 0.05,
            spectral_regime: SpectralRegime::NaturallyCluster,
        };
        let err = cocycle_budget_check(&atlas, (ChartId(0), ChartId(1)), 0.10);
        assert!(matches!(err, Err(GateError::CocycleBudgetExceeded { .. })));
    }

    // Force-use of TransitionKey for the lint
    #[test]
    fn transition_key_compiles() {
        let _k = TransitionKey::canonical(ChartId(0), ChartId(1));
    }
}
