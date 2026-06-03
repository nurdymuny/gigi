//! `SpectralRegime` -- the T5 honest-disclosure type.
//!
//! Declares the spectral regime of a sharded bundle so the engine can
//! route SPECTRAL queries correctly. T5 validation showed that the naive
//! `min(per-shard λ_1)` bound is non-universal: it holds tightly for
//! naturally-clustered substrates, fails by 5-7x for expanders.
//!
//! Routing logic:
//! - `NaturallyCluster`        → naive `min(per-shard λ_1)` (cheap, K=1 round)
//! - `Expander`                → distributed Lanczos (T7, K=20-100 rounds)
//! - `CertifiedClusteredAt`    → same as `NaturallyCluster` with audit
//!   evidence

use serde::{Deserialize, Serialize};

/// The spectral regime of a sharded bundle. Required field of `Atlas`.
///
/// Declared at bundle creation; validated by audit script in Phase D.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum SpectralRegime {
    /// The substrate is naturally clustered. Per-shard λ_1 is a tight
    /// first-order bound on global λ_1. Validated for slow-mixing graphs
    /// by `t5_cauchy_interlacing_lambda1.py` Part (B).
    NaturallyCluster,

    /// The substrate is an expander. Naive sharded SPECTRAL is unreliable;
    /// engine routes to distributed Lanczos (T7).
    Expander,

    /// The substrate has been certified via Fiedler-vector-aligned
    /// partition test. Lower-confidence version of `NaturallyCluster`.
    CertifiedClusteredAt {
        /// The empirical Cheeger conductance of the partition.
        /// Lower values indicate cleaner clustering.
        conductance: f64,
    },
}

impl SpectralRegime {
    /// True if the simple `min(per-shard λ_1)` recipe is valid.
    pub fn allows_naive_recipe(&self) -> bool {
        matches!(self, Self::NaturallyCluster | Self::CertifiedClusteredAt { .. })
    }

    /// True if SPECTRAL queries must route through distributed Lanczos.
    pub fn requires_distributed_lanczos(&self) -> bool {
        matches!(self, Self::Expander)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn naturally_cluster_allows_naive() {
        assert!(SpectralRegime::NaturallyCluster.allows_naive_recipe());
        assert!(!SpectralRegime::NaturallyCluster.requires_distributed_lanczos());
    }

    #[test]
    fn expander_requires_lanczos() {
        assert!(!SpectralRegime::Expander.allows_naive_recipe());
        assert!(SpectralRegime::Expander.requires_distributed_lanczos());
    }

    #[test]
    fn certified_clustered_allows_naive() {
        let r = SpectralRegime::CertifiedClusteredAt { conductance: 0.05 };
        assert!(r.allows_naive_recipe());
        assert!(!r.requires_distributed_lanczos());
    }

    #[test]
    fn round_trip_serde() {
        for r in [
            SpectralRegime::NaturallyCluster,
            SpectralRegime::Expander,
            SpectralRegime::CertifiedClusteredAt { conductance: 0.123 },
        ] {
            let json = serde_json::to_string(&r).unwrap();
            let back: SpectralRegime = serde_json::from_str(&json).unwrap();
            assert_eq!(r, back);
        }
    }
}
