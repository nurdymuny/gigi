//! TDD-HAL-II.4b — BundleStore-backed gauge field persistence helpers.
//!
//! The hard work lives in three places:
//!
//!   1. `src/wal.rs` carries the new `LatticeDeclare` /
//!      `GaugeFieldDeclare` `WalEntry` variants (gated on the `gauge`
//!      feature) plus the `WalWriter::log_lattice_declare` /
//!      `log_gauge_field_declare` writer methods.
//!   2. `src/engine.rs` exposes the durable surface
//!      (`declare_lattice_durable` / `declare_gauge_field_durable`)
//!      that user-facing call sites hit. Replay (`do_replay`) restores
//!      the in-process registries from WAL bytes alone — no buffer
//!      bytes on disk; the metadata is enough to call
//!      `SU2GaugeField::new(...)` and re-derive the same buffer per
//!      Bee's locked decision 1 (intra-binding bit-identity).
//!   3. This file is the thin re-materialization helper —
//!      `materialize_field` turns a `(name, lattice_name, group,
//!      init_kind, init_seed)` WAL tuple into an
//!      `Arc<dyn GaugeFieldHandle>` ready for `gauge::registry::register`.
//!
//! The helper exists so the engine's `do_replay` arm and the
//! `compact_wal_to_schemas` re-emit loop don't have to duplicate the
//! group-tag → constructor dispatch. Future U(1) / SU(3) / Z(N) groups
//! add a new arm here and a new `GaugeFieldHandle` impl in their own
//! module; the engine never names a concrete field type.

use std::io;
use std::sync::Arc;

use crate::lattice::Lattice;

use super::group::Group;
use super::registry::GaugeFieldHandle;
use super::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use super::su3_gauge_field::SU3GaugeField;

/// Re-materialize a gauge field handle from a WAL declaration tuple.
///
/// The buffer is reconstructed deterministically from the
/// `(group, init_kind, init_seed)` recipe + the lattice's edge count.
/// For `Group::SU2 + HaarRandom + Some(seed)` this yields a buffer
/// byte-identical to the one the original `declare_gauge_field_durable`
/// call captured. `FromField` is not re-runnable through the
/// constructor alone — it requires a registry lookup of the source
/// field — so this helper returns an `Other` error rather than
/// degrading silently. (Other groups also error out for now; the
/// constructor itself surfaces `GaugeFieldError::UnsupportedGroup`
/// which we map to an io error here.)
pub fn materialize_field(
    name: String,
    lattice: &Lattice,
    group: Group,
    init_kind: GaugeFieldInit,
    init_seed: Option<u64>,
) -> io::Result<Arc<dyn GaugeFieldHandle>> {
    match (group, &init_kind) {
        (Group::SU2, GaugeFieldInit::Identity)
        | (Group::SU2, GaugeFieldInit::HaarRandom) => {
            let field = SU2GaugeField::new(name, lattice, init_kind, init_seed)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            Ok(Arc::new(field))
        }
        (Group::SU2, GaugeFieldInit::FromField(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "FROM_FIELD gauge fields cannot be replayed through the WAL \
             declaration tuple alone — the source field must be \
             resolved at executor time. This is a P1 follow-up.",
        )),
        // INIT FROM BUNDLE (2026-07-18): like FROM_FIELD, the chosen
        // per-edge buffer is the real state — the recipe name alone
        // cannot rebuild it (the source bundle may not exist at replay).
        // PERSIST is rejected at declaration, so no WAL declaration tuple
        // can legitimately carry a FROM_BUNDLE init.
        (Group::SU2, GaugeFieldInit::FromBundle(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "FROM_BUNDLE gauge fields cannot be replayed through the WAL \
             declaration tuple alone — the chosen per-edge buffer must be \
             resolved from the source bundle at executor time (PERSIST is \
             rejected at declaration).",
        )),
        // Halcyon ITEM 3.1 Phase 1: SU(3) replay through the same
        // metadata-only path SU(2) uses. Same byte-identity contract
        // (intra-binding bit-identity per Bee's locked decision 1) —
        // the Mezzadri Haar sampler is deterministic per seed.
        (Group::SU3, GaugeFieldInit::Identity)
        | (Group::SU3, GaugeFieldInit::HaarRandom) => {
            let field = SU3GaugeField::new(name, lattice, init_kind, init_seed)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
            Ok(Arc::new(field))
        }
        (Group::SU3, GaugeFieldInit::FromField(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "FROM_FIELD SU(3) gauge fields cannot be replayed through the WAL \
             declaration tuple alone — the source field must be \
             resolved at executor time (same constraint as SU(2)).",
        )),
        (Group::SU3, GaugeFieldInit::FromBundle(_)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "FROM_BUNDLE SU(3) gauge fields cannot be replayed through the WAL \
             declaration tuple alone — the chosen per-edge buffer must be \
             resolved from the source bundle at executor time (same constraint \
             as SU(2); INIT FROM BUNDLE ships GROUP SU(2) this phase anyway).",
        )),
        // INIT FLUX (2026-07-16) is a U(1)-only bundle materialization;
        // PERSIST is rejected at declaration, so no WAL declaration
        // tuple can legitimately carry a flux init for SU(2)/SU(3).
        (Group::SU2, GaugeFieldInit::FluxRandom | GaugeFieldInit::FluxUniform)
        | (Group::SU3, GaugeFieldInit::FluxRandom | GaugeFieldInit::FluxUniform) => {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "INIT FLUX gauge fields are U(1) bundle materializations and are \
                 never WAL-declared (PERSIST is rejected at declaration)",
            ))
        }
        (Group::U1, _) | (Group::ZN { .. }, _) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "WAL gauge-field replay supports SU(2) and SU(3); got {}",
                group.label()
            ),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lattice::topology::truncated_icosahedron::buckyball;

    /// TDD-HAL-II.4b (lib gate): the materialize path produces a
    /// buffer byte-identical to a freshly-constructed SU2GaugeField
    /// with the same recipe. This is the contract that lets the WAL
    /// variant be metadata-only.
    #[test]
    fn tdd_hal_ii_4b_materialize_haar_byte_identical() {
        let bb = buckyball();
        let seed: u64 = 20260616;
        let reference = SU2GaugeField::new(
            "U".into(),
            &bb,
            GaugeFieldInit::HaarRandom,
            Some(seed),
        )
        .expect("haar ref");

        let handle = materialize_field(
            "U".into(),
            &bb,
            Group::SU2,
            GaugeFieldInit::HaarRandom,
            Some(seed),
        )
        .expect("materialize");

        let view = handle.as_dense_buffer();
        assert_eq!(view.group, Group::SU2);
        assert_eq!(view.n_edges, reference.buffer.n_edges);
        assert_eq!(view.repr_dim, reference.buffer.repr_dim);
        assert_eq!(view.data, reference.buffer.data);
        assert_eq!(handle.name(), "U");
        assert_eq!(handle.lattice_name(), bb.name);
        let (kind, s) = handle.init_metadata();
        assert_eq!(kind, GaugeFieldInit::HaarRandom);
        assert_eq!(s, Some(seed));
    }

    /// TDD-HAL-II.4b: identity recipe round-trips through materialize.
    #[test]
    fn tdd_hal_ii_4b_materialize_identity() {
        let bb = buckyball();
        let handle = materialize_field(
            "U_id".into(),
            &bb,
            Group::SU2,
            GaugeFieldInit::Identity,
            None,
        )
        .expect("materialize identity");
        let view = handle.as_dense_buffer();
        for e in 0..view.n_edges {
            assert_eq!(view.data[e * 4], 1.0);
            assert_eq!(view.data[e * 4 + 1], 0.0);
            assert_eq!(view.data[e * 4 + 2], 0.0);
            assert_eq!(view.data[e * 4 + 3], 0.0);
        }
    }

    /// TDD-HAL-II.4b: non-SU(2) groups error out cleanly (no panic).
    /// The error message names the group — anchored to Group::label so
    /// the test survives label changes.
    #[test]
    fn tdd_hal_ii_4b_materialize_non_su2_errors() {
        let bb = buckyball();
        let result = materialize_field(
            "U_u1".into(),
            &bb,
            Group::U1,
            GaugeFieldInit::Identity,
            None,
        );
        match result {
            Err(e) => assert!(e.to_string().contains("SU(2)")),
            Ok(_) => panic!("U(1) materialize should not succeed at launch"),
        }
    }

    /// TDD-HAL-II.4b: FROM_FIELD cannot be re-materialized through
    /// metadata alone — the executor's source-field resolution is
    /// required. This is a documented P1 follow-up; the helper
    /// surfaces it as a clean error rather than degrading silently.
    #[test]
    fn tdd_hal_ii_4b_materialize_from_field_errors() {
        let bb = buckyball();
        let result = materialize_field(
            "U_clone".into(),
            &bb,
            Group::SU2,
            GaugeFieldInit::FromField("U_src".into()),
            None,
        );
        match result {
            Err(e) => assert!(e.to_string().to_uppercase().contains("FROM_FIELD")),
            Ok(_) => panic!("FROM_FIELD materialize should not succeed without registry"),
        }
    }
}
