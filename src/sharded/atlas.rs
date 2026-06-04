//! `Atlas`, `ChartId`, `ChartMetadata`, `Transition` — the on-disk
//! atlas-cover storage format for sharded GIGI.
//!
//! See `SHARDING_SPEC.md` §2 + §3 for the design rationale.

use crate::sharded::regime::SpectralRegime;
use crate::sharded::ShardId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identifier for a chart in an atlas. A chart is the unit of locality
/// for sharded computation; each shard holds one or more charts.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ChartId(pub u32);

/// Per-chart metadata: which shard owns the chart, what region of the
/// manifold it covers, connection 1-form data, metric data.
///
/// Phase A skeleton: the heavy fields (`connection`, `metric`,
/// `region`) are placeholder types. Phase B fills these in.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChartMetadata {
    pub id: ChartId,
    pub shard_id: ShardId,
    /// What region of the manifold this chart covers. Phase A: bounding
    /// box on primary-key hash space; Phase D: Fiedler-vector partition.
    pub region: ChartRegion,
    /// Operational horizon L_star (Davis Manifold paper A5).
    pub operational_horizon: f64,
    /// kappa_soft margin (Davis Manifold paper §A5 non-vacuity).
    pub kappa_soft: f64,
    /// Geodesic radius of interest R (Davis Manifold paper §A5).
    pub geodesic_radius: f64,
}

/// What region of the manifold a chart covers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ChartRegion {
    /// Each chart owns records whose primary-key hash falls into a
    /// specific bucket. The standard relational-shard convention.
    HashBucket {
        bucket_index: u32,
        n_buckets: u32,
    },
    /// Each chart owns one cluster from a Fiedler-vector recursive-
    /// bisection partition. The partition preserves the neighborhood
    /// graph that K and BETTI are computed against — see
    /// `theory/poincare_to_sharding/validation/tfp1_fiedler_preserves_curvature.py`.
    ///
    /// Routing is not closed-form — it requires the precomputed
    /// (record-pk -> chart) assignment table held by the
    /// `ShardedBundle`. Atlases declaring this region are not
    /// queryable by `find_chart_for_pk_hash`; use
    /// `ShardedBundle::route_pk_fiedler` instead.
    FiedlerCluster {
        cluster_index: u32,
        total_clusters: u32,
    },
    /// Placeholder for richer region types.
    Other,
}

/// A chart-transition function T_ij: U_i -> U_j on overlap U_i ∩ U_j.
///
/// Phase A: stored as opaque serialized data (function representation
/// TBD in Phase B). The crucial invariant maintained even in Phase A is
/// the `lipschitz_estimate` field used in cocycle-bound checks.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    pub from: ChartId,
    pub to: ChartId,
    /// Empirical Lipschitz constant on the overlap region. Used in the
    /// cocycle-discrepancy first-order bound (T2 §3.2).
    pub lipschitz_estimate: f64,
    /// Whether the transition is invertible (most natural transitions
    /// are; learned approximate ones may not be).
    pub invertible: bool,
    /// Opaque function representation. Phase A placeholder; Phase B
    /// makes this a concrete enum (Closed-form / NeuralNet / PiecewiseLinear).
    pub representation: TransitionRepresentation,
}

/// Phase A placeholder for transition function representation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransitionRepresentation {
    /// Identity: T_ij(v) = v. Used when two charts share the same
    /// coordinate system (e.g., the trivial atlas for n_shards=1).
    Identity,
    /// Phase B will add: ClosedForm(expression), NeuralNet(weights),
    /// PiecewiseLinear(grid), etc.
    Placeholder,
}

/// The atlas of a sharded bundle.
///
/// Stores chart metadata, pairwise transitions, the declared cocycle
/// slack budget, and the spectral regime declaration.
///
/// This is the on-disk representation of "the chart-stitching data"
/// from *Geometry of Sameness* §4 — first-class, queryable, indexed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Atlas {
    /// All charts in this atlas, indexed by ChartId.
    pub charts: HashMap<ChartId, ChartMetadata>,

    /// Pairwise overlaps. Key is canonicalized as (min_id, max_id).
    pub transitions: HashMap<TransitionKey, Transition>,

    /// The declared cocycle slack budget. From *Geometry of Sameness*
    /// Definition 21: ||T_jk ∘ T_ij - T_ik|| <= delta_cocycle.
    pub delta_cocycle_budget: f64,

    /// Per-atlas spectral regime declaration. Routes SPECTRAL queries
    /// per `SHARDING_SPEC.md` §5.6.
    pub spectral_regime: SpectralRegime,
}

/// Canonical key for storing pairwise transitions: (min ChartId, max ChartId).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TransitionKey(pub ChartId, pub ChartId);

impl TransitionKey {
    pub fn canonical(a: ChartId, b: ChartId) -> Self {
        if a.0 <= b.0 {
            Self(a, b)
        } else {
            Self(b, a)
        }
    }
}

impl Atlas {
    /// Construct an empty atlas with the given regime declaration and
    /// cocycle budget. Phase A only -- charts/transitions are added
    /// post-construction by the ingest pipeline.
    pub fn new(spectral_regime: SpectralRegime, delta_cocycle_budget: f64) -> Self {
        Self {
            charts: HashMap::new(),
            transitions: HashMap::new(),
            delta_cocycle_budget,
            spectral_regime,
        }
    }

    /// Trivial single-chart atlas. Used during Phase B migration of
    /// existing single-node bundles into the sharded storage format
    /// without changing any operational behavior.
    pub fn trivial(shard_id: ShardId) -> Self {
        let mut charts = HashMap::new();
        charts.insert(
            ChartId(0),
            ChartMetadata {
                id: ChartId(0),
                shard_id,
                region: ChartRegion::HashBucket {
                    bucket_index: 0,
                    n_buckets: 1,
                },
                operational_horizon: 1.0,
                kappa_soft: 1.0,
                geodesic_radius: 1.0,
            },
        );
        Self {
            charts,
            transitions: HashMap::new(),
            delta_cocycle_budget: 0.0,
            spectral_regime: SpectralRegime::NaturallyCluster,
        }
    }

    /// Look up the transition T_ij if it exists (canonical key).
    pub fn transition(&self, a: ChartId, b: ChartId) -> Option<&Transition> {
        self.transitions.get(&TransitionKey::canonical(a, b))
    }

    /// All transitions originating from the given chart.
    pub fn transitions_from(
        &self,
        chart: ChartId,
    ) -> impl Iterator<Item = (ChartId, &Transition)> {
        self.transitions.iter().filter_map(move |(k, t)| {
            if k.0 == chart {
                Some((k.1, t))
            } else if k.1 == chart {
                Some((k.0, t))
            } else {
                None
            }
        })
    }

    /// Phase A stub: identifies which chart owns a given point. In
    /// Phase B this dispatches on the `ChartRegion` variant.
    pub fn find_chart_for_pk_hash(&self, pk_hash: u64) -> Option<ChartId> {
        for (id, meta) in &self.charts {
            if let ChartRegion::HashBucket { bucket_index, n_buckets } = meta.region {
                if (pk_hash % u64::from(n_buckets)) as u32 == bucket_index {
                    return Some(*id);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivial_atlas_has_one_chart_and_no_transitions() {
        let a = Atlas::trivial(ShardId(0));
        assert_eq!(a.charts.len(), 1);
        assert_eq!(a.transitions.len(), 0);
        assert!(a.charts.contains_key(&ChartId(0)));
    }

    #[test]
    fn trivial_atlas_routes_all_pks_to_chart_zero() {
        let a = Atlas::trivial(ShardId(0));
        for pk in [0u64, 1, 7, 42, 9999, u64::MAX] {
            assert_eq!(a.find_chart_for_pk_hash(pk), Some(ChartId(0)));
        }
    }

    #[test]
    fn transition_key_is_canonical() {
        let k1 = TransitionKey::canonical(ChartId(2), ChartId(5));
        let k2 = TransitionKey::canonical(ChartId(5), ChartId(2));
        assert_eq!(k1, k2);
        assert_eq!(k1.0, ChartId(2));
        assert_eq!(k1.1, ChartId(5));
    }

    #[test]
    fn round_trip_serde() {
        let a = Atlas::trivial(ShardId(7));
        let json = serde_json::to_string(&a).unwrap();
        let back: Atlas = serde_json::from_str(&json).unwrap();
        assert_eq!(back.charts.len(), 1);
        assert_eq!(back.delta_cocycle_budget, 0.0);
        assert_eq!(back.spectral_regime, SpectralRegime::NaturallyCluster);
    }
}
