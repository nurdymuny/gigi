//! Cross-atlas joins: types + verbs for two-bundle bridge operations.
//!
//! The math is locked by T8, T9, T10 (`theory/poincare_to_sharding/
//! validation/`). This module ships the Rust scaffold and the wired
//! primitives:
//!
//! - `cross_atlas_cocycle_check` — T8 bridge cocycle bound validation
//!   on observed deltas
//! - `cross_atlas_write_resolve` — T10 atlas-agnostic Clean Finger Move
//!   resolver for cross-atlas conflict sets
//!
//! T9 (cross-atlas BETTI via fiber-product Mayer-Vietoris) requires the
//! same `BettiNumbers` infrastructure as `shard_betti_mayer_vietoris`
//! and is gated by `feature = "kahler"`.
//!
//! See [`theory/poincare_to_sharding/CROSS_ATLAS_JOINS.md`] for the
//! full theory + engineering plan (Marcella + PRISM as the canonical
//! consumer pair).

use crate::sharded::atlas::{Atlas, ChartId};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Identifier for an atlas in a cross-atlas join. Typically 1 (the
/// "primary" atlas) and 2 (the "joined" atlas); the join itself can
/// extend to N atlases in principle but Phase D ships 2-atlas joins.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct AtlasId(pub u32);

/// Canonical key for a bridge transition between two charts in
/// different atlases. Stored canonically by (atlas, chart) pair
/// ordering so lookup is direction-agnostic.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct BridgeChartKey {
    pub atlas_a: AtlasId,
    pub chart_a: ChartId,
    pub atlas_b: AtlasId,
    pub chart_b: ChartId,
}

impl BridgeChartKey {
    /// Canonical key by lexicographic ordering of (atlas, chart) pairs.
    /// `key.canonical(a, b) == key.canonical(b, a)`.
    pub fn canonical(
        atlas_a: AtlasId,
        chart_a: ChartId,
        atlas_b: AtlasId,
        chart_b: ChartId,
    ) -> Self {
        let left = (atlas_a, chart_a);
        let right = (atlas_b, chart_b);
        if left <= right {
            Self {
                atlas_a,
                chart_a,
                atlas_b,
                chart_b,
            }
        } else {
            Self {
                atlas_a: atlas_b,
                chart_a: chart_b,
                atlas_b: atlas_a,
                chart_b: chart_a,
            }
        }
    }
}

/// A bridge transition `B_{ij}^{ab}: V_i^{A_a} -> V_j^{A_b}` between
/// charts in different atlases.
///
/// Carries the empirical Lipschitz estimate and whether the round-trip
/// `B_{ji}^{ba} ∘ B_{ij}^{ab}` is approximately identity (the asymmetric
/// vs symmetric bridge distinction from T8).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BridgeTransition {
    pub from_atlas: AtlasId,
    pub from_chart: ChartId,
    pub to_atlas: AtlasId,
    pub to_chart: ChartId,
    /// Empirical Lipschitz constant on the bridge overlap. T8 uses
    /// this in the first-order cocycle bound.
    pub lipschitz_estimate: f64,
    /// Whether the bridge is invertible. Most analytic bridges are;
    /// learned bridges may not be.
    pub invertible: bool,
}

/// Two atlases joined by a set of bridge transitions.
///
/// `delta_bridge_budget` controls the cocycle slack across atlas pairs
/// (T8 §3); `delta_asymm_budget` controls the round-trip asymmetry
/// (`||B_{ji} ∘ B_{ij} - id||`, T8 §5).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrossAtlasJoin {
    pub atlas_a_id: AtlasId,
    pub atlas_b_id: AtlasId,
    pub atlas_a: Atlas,
    pub atlas_b: Atlas,
    pub bridges: HashMap<BridgeChartKey, BridgeTransition>,
    pub delta_bridge_budget: f64,
    pub delta_asymm_budget: f64,
}

impl CrossAtlasJoin {
    /// Construct a join with empty bridges. Caller adds bridges with
    /// `add_bridge`.
    pub fn new(
        atlas_a_id: AtlasId,
        atlas_b_id: AtlasId,
        atlas_a: Atlas,
        atlas_b: Atlas,
        delta_bridge_budget: f64,
        delta_asymm_budget: f64,
    ) -> Self {
        Self {
            atlas_a_id,
            atlas_b_id,
            atlas_a,
            atlas_b,
            bridges: HashMap::new(),
            delta_bridge_budget,
            delta_asymm_budget,
        }
    }

    /// Add a bridge transition. The key is canonicalized; calling
    /// `add_bridge` twice with the same chart pair (in either order)
    /// overwrites the first.
    pub fn add_bridge(&mut self, b: BridgeTransition) {
        let key = BridgeChartKey::canonical(b.from_atlas, b.from_chart, b.to_atlas, b.to_chart);
        self.bridges.insert(key, b);
    }

    /// Look up a bridge by chart pair (direction-agnostic).
    pub fn bridge(
        &self,
        atlas_a: AtlasId,
        chart_a: ChartId,
        atlas_b: AtlasId,
        chart_b: ChartId,
    ) -> Option<&BridgeTransition> {
        let key = BridgeChartKey::canonical(atlas_a, chart_a, atlas_b, chart_b);
        self.bridges.get(&key)
    }
}

/// Errors from cross-atlas checks.
#[derive(Debug, Clone, PartialEq)]
pub enum CrossAtlasError {
    /// The observed bridge cocycle slack exceeds the declared budget.
    BridgeCocycleViolated {
        chart_a: ChartId,
        chart_b: ChartId,
        observed: f64,
        budget: f64,
    },
    /// The observed round-trip asymmetry exceeds the declared budget.
    AsymmetryViolated {
        chart_a: ChartId,
        chart_b: ChartId,
        observed: f64,
        budget: f64,
    },
    /// A canonical bridge key was looked up but no bridge exists.
    MissingBridge {
        chart_a: ChartId,
        chart_b: ChartId,
    },
}

/// Verify the bridge cocycle bound (T8 §3) against a set of observed
/// deltas: each entry maps a (chart_a, chart_b) pair to the maximum
/// observed `||B_{jk} ∘ T_{ij} - B_{ik}||` slack across sampled points
/// in the overlap.
///
/// Per T8: for analytic atlases the observed delta is ~0 to machine
/// precision; for learned bridges with perturbation ε it grows ~ε.
/// In either case, the engineering check is: observed ≤ budget.
pub fn cross_atlas_cocycle_check(
    join: &CrossAtlasJoin,
    observed_bridge_deltas: &HashMap<(ChartId, ChartId), f64>,
    observed_asymmetries: &HashMap<(ChartId, ChartId), f64>,
) -> Result<(), CrossAtlasError> {
    for (&(c_a, c_b), &observed) in observed_bridge_deltas {
        if observed > join.delta_bridge_budget {
            return Err(CrossAtlasError::BridgeCocycleViolated {
                chart_a: c_a,
                chart_b: c_b,
                observed,
                budget: join.delta_bridge_budget,
            });
        }
    }
    for (&(c_a, c_b), &observed) in observed_asymmetries {
        if observed > join.delta_asymm_budget {
            return Err(CrossAtlasError::AsymmetryViolated {
                chart_a: c_a,
                chart_b: c_b,
                observed,
                budget: join.delta_asymm_budget,
            });
        }
    }
    Ok(())
}

// ============================================================================
// Cross-atlas Clean Finger Move (T10)
// ============================================================================

/// A write conflict in the cross-atlas setting.
///
/// `atlas_id` is the primary atlas; for `in_bridge = true` conflicts
/// (visible to both atlases), the atlas_id is the canonical choice.
/// `partner_id` may live in either atlas — the resolver is
/// atlas-agnostic per T10.
#[derive(Clone, Debug, PartialEq)]
pub struct CrossAtlasConflict {
    pub id: u64,
    pub atlas_id: AtlasId,
    /// +1 / -1; canceling partners have opposite signs.
    pub sign: i8,
    pub partner_id: u64,
    /// True if this conflict lives at a bridge identification (visible
    /// to both atlases). Informational only.
    pub in_bridge: bool,
}

/// Result of a cross-atlas Clean Finger Move resolution.
#[derive(Clone, Debug)]
pub struct CrossAtlasResolverTrace {
    pub initial_count: usize,
    pub steps: usize,
    pub residual_size: usize,
    pub residual_per_atlas: HashMap<AtlasId, usize>,
    pub monotonic_decrease_violations: u32,
    pub bridge_pairs_resolved: usize,
    pub intra_a_pairs_resolved: usize,
    pub intra_b_pairs_resolved: usize,
}

/// Errors from the cross-atlas resolver.
#[derive(Debug, Clone, PartialEq)]
pub enum CrossAtlasResolverError {
    NoCancelingPartner { conflict_id: u64 },
    SameSignPartners { a: u64, b: u64, sign: i8 },
}

/// Cross-atlas Clean Finger Move resolver. Atlas-agnostic per T10:
/// the resolver finds canceling pairs and removes them, regardless of
/// which atlas each conflict lives in or whether they're at a bridge.
///
/// Per T10: terminates in exactly `conflicts.len() / 2` steps with
/// zero residual when every conflict has a canceling partner. The
/// trace tracks how many pairs were intra-atlas-A, intra-atlas-B,
/// or bridge (cross-atlas) for diagnostics.
pub fn cross_atlas_write_resolve(
    conflicts: Vec<CrossAtlasConflict>,
    atlas_a_id: AtlasId,
    atlas_b_id: AtlasId,
) -> Result<CrossAtlasResolverTrace, CrossAtlasResolverError> {
    let initial = conflicts.len();
    let by_id: HashMap<u64, CrossAtlasConflict> =
        conflicts.into_iter().map(|c| (c.id, c)).collect();

    // Precondition validation
    for c in by_id.values() {
        let partner = by_id
            .get(&c.partner_id)
            .ok_or(CrossAtlasResolverError::NoCancelingPartner { conflict_id: c.id })?;
        if c.sign + partner.sign != 0 {
            return Err(CrossAtlasResolverError::SameSignPartners {
                a: c.id,
                b: partner.id,
                sign: c.sign,
            });
        }
    }

    let mut unresolved: HashSet<u64> = by_id.keys().copied().collect();
    let mut steps = 0;
    let mut monotonic_violations = 0;
    let mut last_size = initial;
    let mut bridge_pairs = 0;
    let mut intra_a_pairs = 0;
    let mut intra_b_pairs = 0;

    while !unresolved.is_empty() {
        // Find a canceling pair
        let mut pair: Option<(u64, u64)> = None;
        for &a_id in &unresolved {
            let a = &by_id[&a_id];
            if unresolved.contains(&a.partner_id)
                && a.sign + by_id[&a.partner_id].sign == 0
            {
                pair = Some((a_id, a.partner_id));
                break;
            }
        }
        let Some((a_id, b_id)) = pair else {
            // Precondition violated; defensive return
            let mut residual_per_atlas: HashMap<AtlasId, usize> = HashMap::new();
            for id in &unresolved {
                *residual_per_atlas.entry(by_id[id].atlas_id).or_insert(0) += 1;
            }
            return Ok(CrossAtlasResolverTrace {
                initial_count: initial,
                steps,
                residual_size: unresolved.len(),
                residual_per_atlas,
                monotonic_decrease_violations: monotonic_violations,
                bridge_pairs_resolved: bridge_pairs,
                intra_a_pairs_resolved: intra_a_pairs,
                intra_b_pairs_resolved: intra_b_pairs,
            });
        };

        // Categorize the pair
        let a = &by_id[&a_id];
        let b = &by_id[&b_id];
        if a.atlas_id != b.atlas_id {
            bridge_pairs += 1;
        } else if a.atlas_id == atlas_a_id {
            intra_a_pairs += 1;
        } else if a.atlas_id == atlas_b_id {
            intra_b_pairs += 1;
        }

        unresolved.remove(&a_id);
        unresolved.remove(&b_id);
        let new_size = unresolved.len();
        if new_size != last_size - 2 {
            monotonic_violations += 1;
        }
        last_size = new_size;
        steps += 1;
    }

    Ok(CrossAtlasResolverTrace {
        initial_count: initial,
        steps,
        residual_size: 0,
        residual_per_atlas: HashMap::new(),
        monotonic_decrease_violations: monotonic_violations,
        bridge_pairs_resolved: bridge_pairs,
        intra_a_pairs_resolved: intra_a_pairs,
        intra_b_pairs_resolved: intra_b_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharded::regime::SpectralRegime;
    use crate::sharded::ShardId;

    fn make_join() -> CrossAtlasJoin {
        CrossAtlasJoin::new(
            AtlasId(1),
            AtlasId(2),
            Atlas::trivial(ShardId(0)),
            Atlas::trivial(ShardId(1)),
            1e-9,
            1e-9,
        )
    }

    // ============================================================
    // BridgeChartKey canonicalization
    // ============================================================

    #[test]
    fn bridge_key_canonical_is_direction_agnostic() {
        let k1 = BridgeChartKey::canonical(AtlasId(1), ChartId(3), AtlasId(2), ChartId(5));
        let k2 = BridgeChartKey::canonical(AtlasId(2), ChartId(5), AtlasId(1), ChartId(3));
        assert_eq!(k1, k2);
    }

    #[test]
    fn bridge_key_canonical_orders_consistently() {
        let k1 = BridgeChartKey::canonical(AtlasId(2), ChartId(3), AtlasId(1), ChartId(5));
        assert_eq!(k1.atlas_a, AtlasId(1));
        assert_eq!(k1.chart_a, ChartId(5));
        assert_eq!(k1.atlas_b, AtlasId(2));
        assert_eq!(k1.chart_b, ChartId(3));
    }

    // ============================================================
    // CrossAtlasJoin construction + bridge lookup
    // ============================================================

    #[test]
    fn add_and_lookup_bridge_works() {
        let mut join = make_join();
        let bridge = BridgeTransition {
            from_atlas: AtlasId(1),
            from_chart: ChartId(0),
            to_atlas: AtlasId(2),
            to_chart: ChartId(0),
            lipschitz_estimate: 1.0,
            invertible: true,
        };
        join.add_bridge(bridge.clone());
        // Lookup in canonical direction
        let got = join
            .bridge(AtlasId(1), ChartId(0), AtlasId(2), ChartId(0))
            .unwrap();
        assert_eq!(got.lipschitz_estimate, 1.0);
        // Reverse direction also works
        let got_rev = join
            .bridge(AtlasId(2), ChartId(0), AtlasId(1), ChartId(0))
            .unwrap();
        assert_eq!(got_rev.lipschitz_estimate, 1.0);
    }

    #[test]
    fn missing_bridge_returns_none() {
        let join = make_join();
        assert!(join
            .bridge(AtlasId(1), ChartId(0), AtlasId(2), ChartId(0))
            .is_none());
    }

    // ============================================================
    // Cocycle check (T8 wiring)
    // ============================================================

    #[test]
    fn cocycle_check_passes_when_observed_within_budget() {
        let join = make_join();
        let mut deltas = HashMap::new();
        deltas.insert((ChartId(0), ChartId(0)), 1e-10);
        deltas.insert((ChartId(1), ChartId(2)), 1e-12);
        let mut asyms = HashMap::new();
        asyms.insert((ChartId(0), ChartId(0)), 1e-11);
        let r = cross_atlas_cocycle_check(&join, &deltas, &asyms);
        assert!(r.is_ok());
    }

    #[test]
    fn cocycle_check_fails_when_bridge_delta_exceeds_budget() {
        let join = make_join();
        let mut deltas = HashMap::new();
        deltas.insert((ChartId(0), ChartId(0)), 1e-3); // budget is 1e-9
        let asyms = HashMap::new();
        let r = cross_atlas_cocycle_check(&join, &deltas, &asyms);
        match r {
            Err(CrossAtlasError::BridgeCocycleViolated {
                observed, budget, ..
            }) => {
                assert_eq!(observed, 1e-3);
                assert_eq!(budget, 1e-9);
            }
            _ => panic!("expected BridgeCocycleViolated"),
        }
    }

    #[test]
    fn cocycle_check_fails_when_asymmetry_exceeds_budget() {
        let join = make_join();
        let deltas = HashMap::new();
        let mut asyms = HashMap::new();
        asyms.insert((ChartId(0), ChartId(0)), 1e-5); // budget is 1e-9
        let r = cross_atlas_cocycle_check(&join, &deltas, &asyms);
        match r {
            Err(CrossAtlasError::AsymmetryViolated {
                observed, budget, ..
            }) => {
                assert_eq!(observed, 1e-5);
                assert_eq!(budget, 1e-9);
            }
            _ => panic!("expected AsymmetryViolated"),
        }
    }

    // ============================================================
    // Cross-atlas Clean Finger Move resolver (T10 wiring)
    // ============================================================

    /// Build a cross-atlas conflict batch where every pair is balanced
    /// (every +1 has a -1 partner) per the resolver's preconditions.
    fn make_balanced_batch(
        n_pairs: usize,
        intra_a: usize,
        intra_b: usize,
        bridge: usize,
    ) -> Vec<CrossAtlasConflict> {
        assert_eq!(intra_a + intra_b + bridge, n_pairs);
        let mut conflicts = Vec::new();
        let mut id_counter: u64 = 0;
        // Intra-atlas-A pairs
        for _ in 0..intra_a {
            let a = id_counter;
            let b = id_counter + 1;
            conflicts.push(CrossAtlasConflict {
                id: a,
                atlas_id: AtlasId(1),
                sign: 1,
                partner_id: b,
                in_bridge: false,
            });
            conflicts.push(CrossAtlasConflict {
                id: b,
                atlas_id: AtlasId(1),
                sign: -1,
                partner_id: a,
                in_bridge: false,
            });
            id_counter += 2;
        }
        // Intra-atlas-B pairs
        for _ in 0..intra_b {
            let a = id_counter;
            let b = id_counter + 1;
            conflicts.push(CrossAtlasConflict {
                id: a,
                atlas_id: AtlasId(2),
                sign: 1,
                partner_id: b,
                in_bridge: false,
            });
            conflicts.push(CrossAtlasConflict {
                id: b,
                atlas_id: AtlasId(2),
                sign: -1,
                partner_id: a,
                in_bridge: false,
            });
            id_counter += 2;
        }
        // Bridge pairs (one in atlas A, one in atlas B)
        for _ in 0..bridge {
            let a = id_counter;
            let b = id_counter + 1;
            conflicts.push(CrossAtlasConflict {
                id: a,
                atlas_id: AtlasId(1),
                sign: 1,
                partner_id: b,
                in_bridge: false,
            });
            conflicts.push(CrossAtlasConflict {
                id: b,
                atlas_id: AtlasId(2),
                sign: -1,
                partner_id: a,
                in_bridge: false,
            });
            id_counter += 2;
        }
        conflicts
    }

    #[test]
    fn resolver_balanced_50_50_atlas_distribution() {
        // T10 Part (A): 50/50 split, all cross-atlas pairs
        let conflicts = make_balanced_batch(10, 0, 0, 10);
        let trace = cross_atlas_write_resolve(conflicts, AtlasId(1), AtlasId(2)).unwrap();
        assert_eq!(trace.steps, 10);
        assert_eq!(trace.residual_size, 0);
        assert_eq!(trace.monotonic_decrease_violations, 0);
        assert_eq!(trace.bridge_pairs_resolved, 10);
        assert_eq!(trace.intra_a_pairs_resolved, 0);
        assert_eq!(trace.intra_b_pairs_resolved, 0);
    }

    #[test]
    fn resolver_mixed_distribution_intra_and_bridge() {
        // T10 Part (B): mixed intra-A, intra-B, bridge
        let conflicts = make_balanced_batch(15, 5, 5, 5);
        let trace = cross_atlas_write_resolve(conflicts, AtlasId(1), AtlasId(2)).unwrap();
        assert_eq!(trace.steps, 15);
        assert_eq!(trace.residual_size, 0);
        assert_eq!(trace.monotonic_decrease_violations, 0);
        assert_eq!(trace.bridge_pairs_resolved, 5);
        assert_eq!(trace.intra_a_pairs_resolved, 5);
        assert_eq!(trace.intra_b_pairs_resolved, 5);
    }

    #[test]
    fn resolver_terminates_in_exactly_n_over_2_steps() {
        // T10 universal claim: N conflicts -> N/2 steps
        for n_pairs in [1, 5, 50, 100] {
            let conflicts = make_balanced_batch(n_pairs, n_pairs / 3, n_pairs / 3, n_pairs - 2 * (n_pairs / 3));
            let n_total = conflicts.len();
            let trace = cross_atlas_write_resolve(conflicts, AtlasId(1), AtlasId(2)).unwrap();
            assert_eq!(trace.steps, n_total / 2);
            assert_eq!(trace.residual_size, 0);
        }
    }

    #[test]
    fn resolver_refuses_no_canceling_partner() {
        // Lone conflict with no partner in the input
        let conflicts = vec![CrossAtlasConflict {
            id: 1,
            atlas_id: AtlasId(1),
            sign: 1,
            partner_id: 999, // doesn't exist
            in_bridge: false,
        }];
        let r = cross_atlas_write_resolve(conflicts, AtlasId(1), AtlasId(2));
        assert!(matches!(
            r,
            Err(CrossAtlasResolverError::NoCancelingPartner { conflict_id: 1 })
        ));
    }

    #[test]
    fn resolver_refuses_same_sign_partners() {
        let conflicts = vec![
            CrossAtlasConflict {
                id: 1,
                atlas_id: AtlasId(1),
                sign: 1,
                partner_id: 2,
                in_bridge: false,
            },
            CrossAtlasConflict {
                id: 2,
                atlas_id: AtlasId(1),
                sign: 1, // same sign as partner -- malformed
                partner_id: 1,
                in_bridge: false,
            },
        ];
        let r = cross_atlas_write_resolve(conflicts, AtlasId(1), AtlasId(2));
        assert!(matches!(r, Err(CrossAtlasResolverError::SameSignPartners { .. })));
    }

    #[test]
    fn resolver_handles_bridge_overlap_conflicts() {
        // T10 Part (C): conflicts tagged in_bridge=true are treated
        // as ordinary by the resolver (the tag is informational).
        let mut conflicts = make_balanced_batch(5, 0, 0, 5);
        // Tag the first bridge pair as in_bridge
        conflicts[0].in_bridge = true;
        conflicts[1].in_bridge = true;
        let trace = cross_atlas_write_resolve(conflicts, AtlasId(1), AtlasId(2)).unwrap();
        assert_eq!(trace.steps, 5);
        assert_eq!(trace.residual_size, 0);
    }

    #[test]
    fn join_serde_round_trips() {
        let mut join = make_join();
        join.add_bridge(BridgeTransition {
            from_atlas: AtlasId(1),
            from_chart: ChartId(0),
            to_atlas: AtlasId(2),
            to_chart: ChartId(0),
            lipschitz_estimate: 1.5,
            invertible: true,
        });
        let json = serde_json::to_string(&join.atlas_a).unwrap();
        let back: Atlas = serde_json::from_str(&json).unwrap();
        assert_eq!(back.charts.len(), join.atlas_a.charts.len());
        // BridgeTransition itself serde-roundtrips (the join's bridge
        // HashMap has the same JSON-key issue as Atlas.transitions
        // -- see existing follow-up note).
        let b = join.bridges.values().next().unwrap().clone();
        let bjson = serde_json::to_string(&b).unwrap();
        let bback: BridgeTransition = serde_json::from_str(&bjson).unwrap();
        assert_eq!(bback, b);
    }

    /// Unused-import suppression — keeps SpectralRegime warning quiet
    /// in case the imports section is later trimmed.
    #[test]
    fn _spectral_regime_in_scope() {
        let _ = SpectralRegime::NaturallyCluster;
    }
}
