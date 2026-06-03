//! `ShardedBundle` — Phase B runtime integration of the sharded
//! substrate with the existing `BundleStore`.
//!
//! Phase B is the **trivial-atlas** sprint per `SHARDING_SPEC.md` §9
//! Phase B: takes any single-node bundle and wraps it as a sharded
//! bundle with `n_shards = 1`. The wrapped form has the same on-disk
//! semantics (records still live in the inner `BundleStore`) but
//! exposes the `Atlas` accessor and chart routing scaffolding that
//! Phase C, D, E, F will fill in for multi-shard.
//!
//! Zero behavioral change vs. raw `BundleStore` when the atlas is
//! trivial. This is intentional: it validates the codec, the routing
//! layer, and the API surface before any real partitioning lands.

use crate::bundle::BundleStore;
use crate::sharded::atlas::{Atlas, ChartId};
use crate::sharded::ShardId;
use crate::types::Record;

/// A bundle wrapped with an atlas declaration.
///
/// For Phase B this is a thin newtype around `(BundleStore, Atlas)`
/// that delegates record access to the inner store, with the atlas
/// available for chart-routing decisions (currently trivial).
///
/// In Phase C–F this type's methods will dispatch reads/writes per
/// chart, with the atlas's `find_chart_for_pk_hash` routing the work
/// across multiple inner stores (one per chart).
pub struct ShardedBundle {
    inner: BundleStore,
    atlas: Atlas,
}

impl ShardedBundle {
    /// Wrap an existing `BundleStore` with a trivial single-chart atlas.
    ///
    /// The resulting `ShardedBundle` has one chart owning all records,
    /// no transitions, `SpectralRegime::NaturallyCluster`, and
    /// `delta_cocycle_budget = 0.0`. Reads route to the single chart;
    /// behavior is byte-equivalent to the inner store.
    pub fn wrap_trivial(inner: BundleStore, shard_id: ShardId) -> Self {
        Self {
            inner,
            atlas: Atlas::trivial(shard_id),
        }
    }

    /// Wrap an existing `BundleStore` with a user-supplied atlas.
    ///
    /// Phase B does NOT validate that the atlas's chart partitioning
    /// is consistent with the inner store's record set — that's a
    /// Phase C concern. Phase B just stores the atlas alongside.
    pub fn with_atlas(inner: BundleStore, atlas: Atlas) -> Self {
        Self { inner, atlas }
    }

    /// The atlas declaration. Use this to inspect the chart partition,
    /// transitions, spectral regime, and cocycle budget.
    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    /// The inner `BundleStore`. Phase B keeps this accessor public so
    /// existing code can be migrated incrementally. Phase C will
    /// constrain this in favor of per-chart accessors.
    pub fn inner(&self) -> &BundleStore {
        &self.inner
    }

    /// Mutable access to the inner store. Same Phase B caveat as
    /// `inner()`.
    pub fn inner_mut(&mut self) -> &mut BundleStore {
        &mut self.inner
    }

    /// Point query routed through the atlas.
    ///
    /// For Phase B (trivial atlas), this delegates directly to the
    /// inner store's `point_query`. The routing step is a no-op when
    /// there's one chart. For Phase C+ multi-shard, this method will
    /// hash the primary key, look up the chart via
    /// `Atlas::find_chart_for_pk_hash`, and dispatch to the
    /// corresponding inner store.
    pub fn point_query(&self, key: &Record) -> Option<Record> {
        // Trivial atlas: single chart owns everything; no routing needed.
        // (Validated below by the chart_routing test.)
        let _chart = self.route_pk(key);
        self.inner.point_query(key)
    }

    /// Number of records across all charts. For Phase B (trivial), this
    /// is the inner store's `len`. For Phase C+, this will sum
    /// per-chart counts.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// True if the bundle holds no records.
    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    /// Iterator over all records across all charts. For Phase B
    /// (trivial), this is the inner store's iterator. For Phase C+,
    /// this will be a chained iterator across per-chart stores.
    pub fn records(&self) -> Box<dyn Iterator<Item = Record> + '_> {
        self.inner.records()
    }

    /// The mutation counter for cache invalidation. Phase B: inner
    /// store's counter. Phase C+: max across per-chart counters.
    pub fn mutation_counter(&self) -> u64 {
        self.inner.mutation_counter()
    }

    /// Route a record's primary key to a chart.
    ///
    /// For Phase B: returns `Some(ChartId(0))` for any key when the
    /// atlas is trivial (one chart owning all records). Returns `None`
    /// only if the atlas has no charts at all.
    ///
    /// For Phase C+: hashes the primary key, looks up the chart via
    /// `Atlas::find_chart_for_pk_hash`. The hash function is
    /// `FxHash` on the primary-key bytes to match GIGI's existing
    /// hash-index convention.
    pub fn route_pk(&self, _key: &Record) -> Option<ChartId> {
        // Phase B: trivial routing — single chart owns everything.
        // For non-trivial atlas, route via the atlas's region predicate.
        if self.atlas.charts.is_empty() {
            return None;
        }
        // Phase B simplification: return the first chart.
        // Phase C+ will compute a real hash and route via
        // `self.atlas.find_chart_for_pk_hash(hash)`.
        Some(*self.atlas.charts.keys().next().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharded::regime::SpectralRegime;
    use crate::types::{BundleSchema, FieldDef, Value};

    fn make_test_bundle() -> BundleStore {
        let schema = BundleSchema::new("sharded_test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("score").with_range(100.0));
        let mut store = BundleStore::new(schema);
        for (id, name, score) in [
            (1, "alice", 75.0),
            (2, "bob", 82.0),
            (3, "carol", 91.0),
        ] {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(id));
            r.insert("name".into(), Value::Text(name.into()));
            r.insert("score".into(), Value::Float(score));
            store.insert(&r);
        }
        store
    }

    fn key_for(id: i64) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id));
        r
    }

    #[test]
    fn wrap_trivial_produces_single_chart_atlas() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        assert_eq!(shard.atlas().charts.len(), 1);
        assert_eq!(shard.atlas().transitions.len(), 0);
        assert_eq!(
            shard.atlas().spectral_regime,
            SpectralRegime::NaturallyCluster
        );
        assert_eq!(shard.atlas().delta_cocycle_budget, 0.0);
    }

    #[test]
    fn point_query_matches_inner_store() {
        let raw = make_test_bundle();
        let raw_result = raw.point_query(&key_for(2));
        let shard = ShardedBundle::wrap_trivial(raw, ShardId(0));
        let shard_result = shard.point_query(&key_for(2));
        assert_eq!(raw_result, shard_result);
        assert!(shard_result.is_some());
        assert_eq!(
            shard_result.unwrap().get("name"),
            Some(&Value::Text("bob".into()))
        );
    }

    #[test]
    fn len_and_is_empty_delegate() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        assert_eq!(shard.len(), 3);
        assert!(!shard.is_empty());
    }

    #[test]
    fn records_iterator_yields_all() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        let all: Vec<Record> = shard.records().collect();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn route_pk_returns_single_chart_for_trivial() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        let routed = shard.route_pk(&key_for(1));
        assert_eq!(routed, Some(ChartId(0)));
        // Same chart for any other PK
        assert_eq!(shard.route_pk(&key_for(2)), Some(ChartId(0)));
        assert_eq!(shard.route_pk(&key_for(99999)), Some(ChartId(0)));
    }

    #[test]
    fn atlas_serde_round_trips() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(7));
        let json = serde_json::to_string(shard.atlas()).unwrap();
        let back: Atlas = serde_json::from_str(&json).unwrap();
        assert_eq!(back.charts.len(), shard.atlas().charts.len());
        assert_eq!(back.transitions.len(), 0);
        assert_eq!(back.delta_cocycle_budget, 0.0);
        assert_eq!(back.spectral_regime, SpectralRegime::NaturallyCluster);
    }

    #[test]
    fn mutation_counter_delegates() {
        let store = make_test_bundle();
        let mc_before = store.mutation_counter();
        let mut shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        assert_eq!(shard.mutation_counter(), mc_before);
        // Insert through inner_mut and verify counter bumps
        let mut new_record = Record::new();
        new_record.insert("id".into(), Value::Integer(4));
        new_record.insert("name".into(), Value::Text("dave".into()));
        new_record.insert("score".into(), Value::Float(67.0));
        shard.inner_mut().insert(&new_record);
        assert!(shard.mutation_counter() > mc_before);
        assert_eq!(shard.len(), 4);
    }

    #[test]
    fn with_atlas_accepts_user_supplied_atlas() {
        let store = make_test_bundle();
        let mut atlas = Atlas::new(SpectralRegime::Expander, 0.05);
        // Add a single chart so route_pk doesn't return None
        atlas.charts.insert(
            ChartId(42),
            crate::sharded::atlas::ChartMetadata {
                id: ChartId(42),
                shard_id: ShardId(1),
                region: crate::sharded::atlas::ChartRegion::HashBucket {
                    bucket_index: 0,
                    n_buckets: 1,
                },
                operational_horizon: 1.0,
                kappa_soft: 0.8,
                geodesic_radius: 1.0,
            },
        );
        let shard = ShardedBundle::with_atlas(store, atlas);
        assert_eq!(shard.atlas().spectral_regime, SpectralRegime::Expander);
        assert_eq!(shard.atlas().delta_cocycle_budget, 0.05);
        assert_eq!(shard.route_pk(&key_for(1)), Some(ChartId(42)));
    }
}
