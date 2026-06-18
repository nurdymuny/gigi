//! TDD-HAL-II.4b — BundleStore-backed gauge field persistence.
//!
//! Three integration tests against a live `Engine`:
//!
//!   1. `tdd_hal_ii_4b_gauge_field_survives_wal_replay` — declare a
//!      lattice + a HAAR_RANDOM gauge field with the durable path on a
//!      fresh data dir; capture the buffer bytes; drop the engine; reopen
//!      pointing at the same data dir; the lattice and the gauge field
//!      both come back via the in-process registries with the buffer
//!      byte-identical to the pre-restart capture.
//!
//!   2. `tdd_hal_ii_4b_in_memory_field_does_not_persist` — declare via
//!      the non-durable `gauge::registry::register` path; reopen; the
//!      field is absent. The in-memory path is unchanged.
//!
//!   3. `tdd_hal_ii_4b_wal_compact_preserves_gauge_field` — declare with
//!      the durable path; call `engine.snapshot()` (compacts the WAL
//!      down to `[CreateBundle* CreateTrigger* LatticeDeclare*
//!      GaugeFieldDeclare* Checkpoint]`); reopen; field still present
//!      with byte-identical buffer. Mirrors the
//!      `test_9_8_trigger_survives_restart` gate exactly.
//!
//! Bee's locked decision 1: same GIGI binary + same seed →
//! byte-identical buffer. The replay reconstruction calls
//! `SU2GaugeField::new(name, lattice, HaarRandom, Some(seed))` which
//! goes back through `DenseLinkBuffer::new_haar` → the xorshift64*
//! SmallRng → Marsaglia rejection. Re-running the recipe IS the
//! contract — we don't need to persist the buffer bytes themselves at
//! this gate, just the {name, lattice_name, group, init_kind, seed}
//! metadata.
//!
//! Bee's locked decision 7: this test file is gated on `halcyon`
//! (cross-side gold check); under `--no-default-features` it does not
//! compile or run.

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::{registry as gauge_registry, GaugeFieldInit, SU2GaugeField};
use gigi::lattice::registry as lattice_registry;
use gigi::lattice::topology::truncated_icosahedron::buckyball;
use std::sync::{Arc, Mutex, OnceLock};

/// Process-wide mutex serializing every test in this file. The lattice
/// and gauge registries are process singletons (`gauge::registry` /
/// `lattice::registry`) — Engine::open calls `clear()` on both during
/// the replay pass, so two tests running in parallel that each open an
/// engine race the cleared registry. Holding this mutex for the
/// duration of each test serializes the engine-open + registry-read
/// path. Same trick the gauge::registry unit tests use implicitly
/// (they run on the lib's --test-threads=1 default lane).
fn registry_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// TDD-HAL-II.4b: a gauge field declared via the engine's durable
/// `declare_gauge_field_durable` path survives a process restart. The
/// lattice it's bound to also persists (gauge fields are useless
/// without their base topology). Reconstruction is from the WAL
/// metadata alone, no full-buffer round-trip — Bee's locked decision
/// 1's intra-binding bit-identity invariant makes the metadata
/// sufficient.
#[test]
fn tdd_hal_ii_4b_gauge_field_survives_wal_replay() {
    // Use a unique name per registry to avoid the singleton registry
    // races other Part-II tests have. Tempdir scopes the on-disk WAL
    // to this test alone.
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_4b_a_bb";
    let field_name = "tdd_hal_ii_4b_a_U";
    let seed: u64 = 20260616;

    // Reference buffer the post-restart load should match byte-for-byte.
    let reference_buffer = {
        let bb = {
            let mut b = buckyball();
            b.name = lat_name.into();
            b
        };
        SU2GaugeField::new(
            field_name.into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(seed),
        )
        .expect("haar init")
        .buffer
    };

    // Phase 1 — declare durably, drop the engine.
    {
        gauge_registry::clear();
        lattice_registry::clear();
        let mut engine = Engine::open(dir.path()).expect("engine open");
        // Lattice goes through the engine's durable path so it survives
        // the WAL replay. The buckyball topology is canonical; just
        // renamed.
        let mut bb = buckyball();
        bb.name = lat_name.into();
        engine
            .declare_lattice_durable(bb.clone())
            .expect("declare lattice durable");

        let field = SU2GaugeField::new(
            field_name.into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(seed),
        )
        .expect("haar init");
        engine
            .declare_gauge_field_durable(Arc::new(field))
            .expect("declare gauge field durable");

        // Sanity: in-process registries see the durable handle.
        assert!(gauge_registry::get(field_name).is_some());
        assert!(lattice_registry::get(lat_name).is_some());
    }

    // Phase 2 — clear both registries (simulate fresh process), reopen
    // the engine pointing at the same data dir. The replay must
    // re-populate both registries from WAL alone.
    gauge_registry::clear();
    lattice_registry::clear();
    assert!(gauge_registry::get(field_name).is_none());
    assert!(lattice_registry::get(lat_name).is_none());
    let _engine = Engine::open(dir.path()).expect("engine reopen");

    // Receipt: lattice came back.
    let lat_back = lattice_registry::get(lat_name).expect("lattice replayed");
    assert_eq!(lat_back.name, lat_name);
    assert_eq!(lat_back.n_vertices, 60);
    assert_eq!(lat_back.n_edges(), 90);

    // Receipt: gauge field came back, byte-identical buffer.
    let field_back = gauge_registry::get(field_name).expect("gauge field replayed");
    assert_eq!(field_back.name(), field_name);
    assert_eq!(field_back.lattice_name(), lat_name);
    let (kind, seed_back) = field_back.init_metadata();
    assert_eq!(kind, GaugeFieldInit::HaarRandom);
    assert_eq!(seed_back, Some(seed));

    let buf_back = field_back.as_dense_buffer();
    assert_eq!(buf_back.group, reference_buffer.group);
    assert_eq!(buf_back.n_edges, reference_buffer.n_edges);
    assert_eq!(buf_back.repr_dim, reference_buffer.repr_dim);
    assert_eq!(
        buf_back.data, reference_buffer.data,
        "post-restart buffer must be byte-identical to pre-restart \
         (intra-binding bit-identity contract)"
    );
}

/// TDD-HAL-II.4b: declaring via the in-memory `gauge::registry::register`
/// path (not the engine's durable path) does NOT persist. After restart
/// the field is absent. This is the opt-in contract: durability is
/// explicit, the default stays in-memory.
#[test]
fn tdd_hal_ii_4b_in_memory_field_does_not_persist() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let field_name = "tdd_hal_ii_4b_b_U_inmem";

    {
        gauge_registry::clear();
        lattice_registry::clear();
        let _engine = Engine::open(dir.path()).expect("engine open");
        let bb = buckyball();
        lattice_registry::register(bb.clone());

        let field = SU2GaugeField::new(
            field_name.into(),
            &bb,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gauge_registry::register(Arc::new(field));
        assert!(gauge_registry::get(field_name).is_some());
    }

    gauge_registry::clear();
    lattice_registry::clear();
    let _engine = Engine::open(dir.path()).expect("engine reopen");
    assert!(
        gauge_registry::get(field_name).is_none(),
        "non-durable register() must NOT survive restart"
    );
}

/// TDD-HAL-II.4b: a durable gauge field survives WAL compaction —
/// i.e. `engine.snapshot()` which rewrites the WAL to
/// `[CreateBundle* CreateTrigger* LatticeDeclare* GaugeFieldDeclare*
/// Checkpoint]`. Mirrors `test_9_8_trigger_survives_restart` exactly
/// (Bee's spec D). Without this gate, gauge fields would be silently
/// erased on the first auto-checkpoint.
#[test]
fn tdd_hal_ii_4b_wal_compact_preserves_gauge_field() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_4b_c_bb";
    let field_name = "tdd_hal_ii_4b_c_U";
    let seed: u64 = 20260616;

    let reference_buffer = {
        let mut bb = buckyball();
        bb.name = lat_name.into();
        SU2GaugeField::new(
            field_name.into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(seed),
        )
        .unwrap()
        .buffer
    };

    {
        gauge_registry::clear();
        lattice_registry::clear();
        let mut engine = Engine::open(dir.path()).expect("engine open");
        let mut bb = buckyball();
        bb.name = lat_name.into();
        engine.declare_lattice_durable(bb.clone()).unwrap();
        let field = SU2GaugeField::new(
            field_name.into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(seed),
        )
        .unwrap();
        engine
            .declare_gauge_field_durable(Arc::new(field))
            .unwrap();

        // Force WAL compaction. After this the WAL contains only
        // [CreateBundle* CreateTrigger* LatticeDeclare* GaugeFieldDeclare*
        // Checkpoint] — gauge fields must be in that emission set.
        engine.compact_wal_to_schemas().expect("compact WAL");
    }

    gauge_registry::clear();
    lattice_registry::clear();
    let _engine = Engine::open(dir.path()).expect("engine reopen");
    let lat_back = lattice_registry::get(lat_name).expect("lattice replayed");
    assert_eq!(lat_back.n_edges(), 90);
    let field_back = gauge_registry::get(field_name).expect("gauge field survived compaction");
    assert_eq!(field_back.lattice_name(), lat_name);
    assert_eq!(
        field_back.as_dense_buffer().data,
        reference_buffer.data,
        "post-compaction buffer must be byte-identical (metadata-only \
         WAL variant + intra-binding bit-identity)"
    );
}

/// TDD-HAL-II.4b: IDENTITY-initialized fields also survive durable
/// restart. Their init metadata is `(Identity, None)` — no seed —
/// and reconstruction must handle the seed=None path correctly.
#[test]
fn tdd_hal_ii_4b_identity_init_survives_restart() {
    let _guard = registry_lock().lock().unwrap_or_else(|p| p.into_inner());
    let dir = tempfile::tempdir().expect("tempdir");
    let lat_name = "tdd_hal_ii_4b_d_bb";
    let field_name = "tdd_hal_ii_4b_d_U_id";

    {
        gauge_registry::clear();
        lattice_registry::clear();
        let mut engine = Engine::open(dir.path()).unwrap();
        let mut bb = buckyball();
        bb.name = lat_name.into();
        engine.declare_lattice_durable(bb.clone()).unwrap();
        let field =
            SU2GaugeField::new(field_name.into(), &bb, GaugeFieldInit::Identity, None).unwrap();
        engine
            .declare_gauge_field_durable(Arc::new(field))
            .unwrap();
    }

    gauge_registry::clear();
    lattice_registry::clear();
    let _engine = Engine::open(dir.path()).unwrap();
    let field_back = gauge_registry::get(field_name).expect("identity field replayed");
    let (kind, seed_back) = field_back.init_metadata();
    assert_eq!(kind, GaugeFieldInit::Identity);
    assert_eq!(seed_back, None);
    // Identity buffer: every quaternion is (1, 0, 0, 0).
    let buf = field_back.as_dense_buffer();
    for e in 0..buf.n_edges {
        assert_eq!(buf.data[e * 4], 1.0);
        assert_eq!(buf.data[e * 4 + 1], 0.0);
        assert_eq!(buf.data[e * 4 + 2], 0.0);
        assert_eq!(buf.data[e * 4 + 3], 0.0);
    }
}
