//! `ShardedBundle` — runtime integration of the sharded substrate.
//!
//! **Phase B (committed)**: trivial single-chart atlas; one inner
//! `BundleStore` owns all records. Validates the codec, routing layer,
//! and atlas serde round-trip.
//!
//! **Phase C (this file)**: real multi-shard form. The bundle now holds
//! `n_charts` independent `BundleStore`s indexed by `ChartId`. Inserts
//! route to the chart owning the primary key (FxHash-based deterministic
//! routing). Point queries route the same way. Iteration chains across
//! all charts. `len()` sums per-chart counts.
//!
//! Phase B's behaviorally-identical guarantee is preserved for the
//! `wrap_trivial` path (which still produces a 1-chart atlas).
//!
//! Per-verb sharded execution (CURVATURE, BETTI, etc.) per
//! `SHARDING_SPEC.md` §5 lands in Phase D once the routing primitive
//! here is exercised in production.

use crate::bundle::BundleStore;
use crate::sharded::atlas::{Atlas, ChartId, ChartMetadata, ChartRegion, TransitionKey};
use crate::sharded::ShardId;
use crate::types::{BundleSchema, Record, Value};
use std::collections::HashMap;
use std::hash::Hasher;

/// A bundle wrapped with an atlas declaration, holding one `BundleStore`
/// per chart. Reads and writes are routed by the atlas's chart-region
/// predicate (Phase C: hash on primary-key field).
pub struct ShardedBundle {
    charts: HashMap<ChartId, BundleStore>,
    atlas: Atlas,
    /// Cache of the primary-key (base) field name for fast PK extraction.
    /// Derived once at construction; all per-chart stores share schema.
    base_field_name: Option<String>,
}

impl ShardedBundle {
    // ====================================================================
    // Constructors
    // ====================================================================

    /// Wrap an existing `BundleStore` with a trivial single-chart atlas.
    /// Phase B back-compat path.
    pub fn wrap_trivial(inner: BundleStore, shard_id: ShardId) -> Self {
        let base_field_name = inner
            .schema
            .base_fields
            .first()
            .map(|f| f.name.clone());
        let mut charts = HashMap::new();
        charts.insert(ChartId(0), inner);
        Self {
            charts,
            atlas: Atlas::trivial(shard_id),
            base_field_name,
        }
    }

    /// Wrap with a user-supplied atlas. The caller is responsible for
    /// providing one `BundleStore` per `ChartId` declared in the atlas.
    /// Phase C panics if the atlas / charts disagree on `ChartId` set.
    pub fn with_atlas(charts: HashMap<ChartId, BundleStore>, atlas: Atlas) -> Self {
        let base_field_name = charts
            .values()
            .next()
            .and_then(|b| b.schema.base_fields.first().map(|f| f.name.clone()));
        // Sanity: atlas chart ids must match charts map keys
        if atlas.charts.len() != charts.len()
            || !atlas.charts.keys().all(|k| charts.contains_key(k))
        {
            panic!(
                "ShardedBundle::with_atlas: atlas declares {:?} but charts has {:?}",
                atlas.charts.keys().collect::<Vec<_>>(),
                charts.keys().collect::<Vec<_>>(),
            );
        }
        Self {
            charts,
            atlas,
            base_field_name,
        }
    }

    /// Construct a multi-shard `ShardedBundle` from a single schema +
    /// initial record set, hash-routing each record into one of
    /// `n_charts` charts. Phase C primitive — this is the natural
    /// "convert single-node to N-shard" entry point.
    ///
    /// All charts get the SAME schema (they're partitions of the same
    /// bundle, not different schemas). The atlas declares
    /// `SpectralRegime::NaturallyCluster` and `delta_cocycle_budget=0`
    /// since intra-bundle hash sharding has trivial transitions (no
    /// metric data crosses shard boundaries in this construction).
    pub fn wrap_hash_sharded(
        schema: BundleSchema,
        records: Vec<Record>,
        n_charts: u32,
        shard_id: ShardId,
    ) -> Self {
        assert!(n_charts >= 1, "n_charts must be >= 1");
        let base_field_name = schema.base_fields.first().map(|f| f.name.clone());

        // Build N empty bundle stores with the same schema
        let mut charts: HashMap<ChartId, BundleStore> = HashMap::with_capacity(n_charts as usize);
        let mut atlas_charts: HashMap<ChartId, ChartMetadata> =
            HashMap::with_capacity(n_charts as usize);
        for i in 0..n_charts {
            let chart_id = ChartId(i);
            charts.insert(chart_id, BundleStore::new(schema.clone()));
            atlas_charts.insert(
                chart_id,
                ChartMetadata {
                    id: chart_id,
                    shard_id,
                    region: ChartRegion::HashBucket {
                        bucket_index: i,
                        n_buckets: n_charts,
                    },
                    operational_horizon: 1.0,
                    kappa_soft: 1.0,
                    geodesic_radius: 1.0,
                },
            );
        }

        let atlas = Atlas {
            charts: atlas_charts,
            transitions: HashMap::<TransitionKey, _>::new(),
            delta_cocycle_budget: 0.0,
            spectral_regime: crate::sharded::regime::SpectralRegime::NaturallyCluster,
        };

        let mut me = Self {
            charts,
            atlas,
            base_field_name,
        };

        // Route each record to its chart
        for record in records {
            me.insert(&record);
        }
        me
    }

    // ====================================================================
    // Accessors
    // ====================================================================

    /// The atlas declaration.
    pub fn atlas(&self) -> &Atlas {
        &self.atlas
    }

    /// Number of charts in this sharded bundle.
    pub fn n_charts(&self) -> usize {
        self.charts.len()
    }

    /// The inner `BundleStore` for a specific chart, if it exists.
    /// Used by Phase D per-verb sharded execution recipes.
    pub fn chart_store(&self, chart_id: ChartId) -> Option<&BundleStore> {
        self.charts.get(&chart_id)
    }

    /// Mutable access to a specific chart's store.
    pub fn chart_store_mut(&mut self, chart_id: ChartId) -> Option<&mut BundleStore> {
        self.charts.get_mut(&chart_id)
    }

    /// The primary-key (base) field name used for routing.
    pub fn base_field_name(&self) -> Option<&str> {
        self.base_field_name.as_deref()
    }

    // ====================================================================
    // Routing
    // ====================================================================

    /// Hash the primary key of a record to a 64-bit value.
    ///
    /// Phase C uses Rust's `DefaultHasher` (SipHash13). Deterministic
    /// within a process; consistent across calls on the same key value.
    /// Phase D may swap this for FxHash for performance once a real
    /// benchmark exists.
    fn hash_pk_value(value: &Value) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        // Use the value's string representation for stable cross-type hashing.
        // Floats use their bit representation to be hash-consistent with eq.
        match value {
            Value::Integer(i) => hasher.write_i64(*i),
            Value::Float(f) => hasher.write_u64(f.to_bits()),
            Value::Text(s) => hasher.write(s.as_bytes()),
            Value::Bool(b) => hasher.write_u8(*b as u8),
            Value::Timestamp(t) => hasher.write_i64(*t),
            Value::Vector(v) => {
                for x in v {
                    hasher.write_u64(x.to_bits());
                }
            }
            Value::Binary(b) => hasher.write(b),
            Value::Null => hasher.write_u8(0xFF),
        }
        hasher.finish()
    }

    /// Route a record's primary key to a chart, returning `None` if the
    /// atlas has no charts or the record has no extractable PK.
    pub fn route_pk(&self, key: &Record) -> Option<ChartId> {
        let base_name = self.base_field_name.as_deref()?;
        let value = key.get(base_name)?;
        let h = Self::hash_pk_value(value);
        self.atlas.find_chart_for_pk_hash(h)
    }

    // ====================================================================
    // Routed reads
    // ====================================================================

    /// Point query: hash the record's PK to a chart, then query that
    /// chart's inner store.
    pub fn point_query(&self, key: &Record) -> Option<Record> {
        let chart_id = self.route_pk(key)?;
        self.charts.get(&chart_id)?.point_query(key)
    }

    /// Total record count across all charts.
    pub fn len(&self) -> usize {
        self.charts.values().map(|b| b.len()).sum()
    }

    /// True if every chart is empty.
    pub fn is_empty(&self) -> bool {
        self.charts.values().all(|b| b.len() == 0)
    }

    /// Iterator over all records across all charts. Order is
    /// chart-id-ordered for determinism; within a chart, order is the
    /// inner store's iteration order.
    pub fn records(&self) -> Box<dyn Iterator<Item = Record> + '_> {
        let mut chart_ids: Vec<ChartId> = self.charts.keys().copied().collect();
        chart_ids.sort();
        Box::new(
            chart_ids
                .into_iter()
                .flat_map(move |id| self.charts.get(&id).unwrap().records()),
        )
    }

    /// Maximum mutation counter across all charts.
    pub fn mutation_counter(&self) -> u64 {
        self.charts
            .values()
            .map(|b| b.mutation_counter())
            .max()
            .unwrap_or(0)
    }

    // ====================================================================
    // Routed writes
    // ====================================================================

    /// Insert a record, routing to the chart owning the primary key.
    ///
    /// Returns `true` if the record was routed and inserted, `false` if
    /// routing failed (no PK in record, no charts in atlas, etc).
    pub fn insert(&mut self, record: &Record) -> bool {
        let Some(chart_id) = self.route_pk(record) else {
            return false;
        };
        let Some(store) = self.charts.get_mut(&chart_id) else {
            return false;
        };
        store.insert(record);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sharded::regime::SpectralRegime;
    use crate::types::{BundleSchema, FieldDef, Value};

    fn make_schema() -> BundleSchema {
        BundleSchema::new("sharded_test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::categorical("name"))
            .fiber(FieldDef::numeric("score").with_range(100.0))
    }

    fn make_test_bundle() -> BundleStore {
        let mut store = BundleStore::new(make_schema());
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

    fn rec(id: i64, name: &str, score: f64) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id));
        r.insert("name".into(), Value::Text(name.into()));
        r.insert("score".into(), Value::Float(score));
        r
    }

    fn key_for(id: i64) -> Record {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(id));
        r
    }

    // ====================================================================
    // Phase B back-compat
    // ====================================================================

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
        assert_eq!(shard.n_charts(), 1);
    }

    #[test]
    fn wrap_trivial_routes_all_pks_to_chart_zero() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        for id in [1, 2, 3, 99, 9999999] {
            assert_eq!(shard.route_pk(&key_for(id)), Some(ChartId(0)));
        }
    }

    #[test]
    fn wrap_trivial_point_query_finds_records() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        let r = shard.point_query(&key_for(2)).unwrap();
        assert_eq!(r.get("name"), Some(&Value::Text("bob".into())));
    }

    #[test]
    fn wrap_trivial_len_and_records() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(0));
        assert_eq!(shard.len(), 3);
        assert_eq!(shard.records().count(), 3);
    }

    #[test]
    fn atlas_serde_round_trips_for_trivial() {
        let store = make_test_bundle();
        let shard = ShardedBundle::wrap_trivial(store, ShardId(7));
        let json = serde_json::to_string(shard.atlas()).unwrap();
        let back: Atlas = serde_json::from_str(&json).unwrap();
        assert_eq!(back.charts.len(), 1);
        assert_eq!(back.delta_cocycle_budget, 0.0);
    }

    // ====================================================================
    // Phase C: hash-sharded multi-shard
    // ====================================================================

    #[test]
    fn wrap_hash_sharded_creates_n_charts() {
        let records: Vec<Record> = (0..100i64)
            .map(|i| rec(i, &format!("user_{}", i), i as f64))
            .collect();
        let shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            records,
            8,
            ShardId(0),
        );
        assert_eq!(shard.n_charts(), 8);
        assert_eq!(shard.atlas().charts.len(), 8);
        assert_eq!(shard.len(), 100);
    }

    #[test]
    fn wrap_hash_sharded_distributes_records() {
        let records: Vec<Record> = (0..200i64)
            .map(|i| rec(i, &format!("user_{}", i), i as f64))
            .collect();
        let shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            records,
            8,
            ShardId(0),
        );
        // Distribution should be non-degenerate: no chart owns all records,
        // every chart owns at least one record (for n=200, k=8 buckets).
        let per_chart_counts: Vec<usize> = (0..8u32)
            .map(|i| {
                shard
                    .chart_store(ChartId(i))
                    .map(|b| b.len())
                    .unwrap_or(0)
            })
            .collect();
        let total: usize = per_chart_counts.iter().sum();
        assert_eq!(total, 200);
        assert!(per_chart_counts.iter().all(|&c| c > 0),
                "expected every chart to own at least one record, got {:?}",
                per_chart_counts);
        // Sanity: no chart has all records
        assert!(per_chart_counts.iter().all(|&c| c < 200));
    }

    #[test]
    fn hash_sharded_point_query_finds_records_across_charts() {
        let records: Vec<Record> = (0..100i64)
            .map(|i| rec(i, &format!("user_{}", i), i as f64))
            .collect();
        let shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            records,
            4,
            ShardId(0),
        );
        // Every record should be retrievable by PK
        for i in 0..100i64 {
            let result = shard.point_query(&key_for(i));
            assert!(result.is_some(), "missing record id={}", i);
            assert_eq!(
                result.unwrap().get("name"),
                Some(&Value::Text(format!("user_{}", i)))
            );
        }
    }

    #[test]
    fn hash_sharded_routing_is_deterministic() {
        let shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            vec![],
            8,
            ShardId(0),
        );
        for id in [0, 7, 42, 1000, 999999] {
            let r1 = shard.route_pk(&key_for(id));
            let r2 = shard.route_pk(&key_for(id));
            let r3 = shard.route_pk(&key_for(id));
            assert_eq!(r1, r2);
            assert_eq!(r2, r3);
            assert!(r1.is_some());
        }
    }

    #[test]
    fn hash_sharded_insert_routes_to_correct_chart() {
        let mut shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            vec![],
            4,
            ShardId(0),
        );
        let r = rec(42, "answer", 100.0);
        let target_chart = shard.route_pk(&r).unwrap();
        assert!(shard.insert(&r));

        // The record should be in the target chart, not in others
        let in_target = shard
            .chart_store(target_chart)
            .unwrap()
            .point_query(&key_for(42))
            .is_some();
        assert!(in_target, "inserted record not in target chart");

        // Aggregate point_query also finds it
        let aggregate = shard.point_query(&key_for(42));
        assert!(aggregate.is_some());
    }

    #[test]
    fn hash_sharded_records_iterator_yields_all() {
        let records: Vec<Record> = (0..50i64)
            .map(|i| rec(i, &format!("u_{}", i), i as f64))
            .collect();
        let shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            records,
            5,
            ShardId(0),
        );
        let all: Vec<Record> = shard.records().collect();
        assert_eq!(all.len(), 50);
        // Every original ID should appear once
        let mut ids: Vec<i64> = all
            .iter()
            .filter_map(|r| match r.get("id")? {
                Value::Integer(i) => Some(*i),
                _ => None,
            })
            .collect();
        ids.sort();
        assert_eq!(ids, (0..50i64).collect::<Vec<_>>());
    }

    #[test]
    fn hash_sharded_mutation_counter_is_max_across_charts() {
        let mut shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            vec![],
            4,
            ShardId(0),
        );
        assert_eq!(shard.mutation_counter(), 0);
        shard.insert(&rec(1, "a", 1.0));
        let after_one = shard.mutation_counter();
        assert!(after_one > 0);
        shard.insert(&rec(2, "b", 2.0));
        shard.insert(&rec(3, "c", 3.0));
        assert!(shard.mutation_counter() >= after_one);
    }

    #[test]
    fn hash_sharded_serde_round_trips_atlas() {
        let shard = ShardedBundle::wrap_hash_sharded(
            make_schema(),
            vec![],
            8,
            ShardId(3),
        );
        let json = serde_json::to_string(shard.atlas()).unwrap();
        let back: Atlas = serde_json::from_str(&json).unwrap();
        assert_eq!(back.charts.len(), 8);
        for i in 0..8u32 {
            let meta = back.charts.get(&ChartId(i)).unwrap();
            assert_eq!(meta.shard_id, ShardId(3));
            match meta.region {
                ChartRegion::HashBucket { bucket_index, n_buckets } => {
                    assert_eq!(bucket_index, i);
                    assert_eq!(n_buckets, 8);
                }
                _ => panic!("expected HashBucket region"),
            }
        }
    }
}
