//! `inject` — `GAUGE_FIELD … INIT FROM BUNDLE <bundle>` (chosen-field →
//! gauge-registry seam, 2026-07-18).
//!
//! The complement of `u1_flux`: where INIT FLUX materializes a *theta
//! bundle* from a spec (registry → bundle direction), INIT FROM BUNDLE
//! reads an existing edge-endpoint bundle and plants its chosen per-edge
//! group elements into a registry `DenseLinkBuffer` (bundle → registry
//! direction), then registers the field so HOLONOMY / CHERN_CLASS read it
//! back. This is the seam that makes the lens-space p-sweep receipt live:
//! a chosen twisted-BC SU(2) field → `HOLONOMY AROUND CYCLE AXIS z` returns
//! `order = p = |π₁(L(p,q))|`, previously reachable only through the
//! test-only `SU2GaugeField::from_buffer` factory.
//!
//! ORIENTATION IS LOAD-BEARING. The buffer slot for edge `eid` holds the
//! canonical element on the lattice's stored direction `edges[eid] = (u,
//! v)` (meaning `u → v`), and `SU2GaugeField::edge_element(eid, Forward)`
//! returns it as-is while `edge_element(eid, Reverse)` returns its inverse
//! (quaternion conjugate). So to make HOLONOMY read back the *intended* Ω
//! on the directed edge `va → vb` a record declares — not Ω† — we store:
//!   - Ω           when `resolve_edge(va, vb) == Forward`  (edges[eid]=(va,vb))
//!   - Ω.inverse() when `resolve_edge(va, vb) == Reverse`  (edges[eid]=(vb,va))
//! so that `edge_element(eid, resolve_orient)` recovers Ω exactly. Pinned
//! by the round-trip + reverse-orientation unit tests below (byte-exact
//! forward and reverse reads). NOTE: the live lens p-sweep is orientation-
//! *blind* — `order_estimate` and `re_trace` are functions of the holonomy
//! scalar `q0` alone, identical under Ω ↔ Ω†, so it validates the p → order
//! map, not orientation; orientation faithfulness rests on the round-trip
//! and reverse-orientation tests, not the p-sweep receipt.
//!
//! GROUP SUPPORT: SU(2) (the Poincaré need) and U(1) (the Navier–Stokes
//! linking-number reading `∮_C A·dl = κ·Lk` via HOLONOMY on a chosen U(1)
//! vortex field) both ship. The U(1) arm (`u1_buffer_from_bundle`) lit up
//! once live U(1) group math (`GroupElement::U1` compose/inverse/re_trace)
//! and a U(1) `DenseLinkBuffer` arm landed (2026-07-18). SU(3) chosen-field
//! injection is the remaining fast-follow.
//!
//! NORMALIZATION: a non-unit SU(2) quaternion is REJECTED with a typed
//! error, not silently renormalized — `inverse == conjugate` (hence the
//! round-trip and the order estimate) only holds for `|q| = 1`, so a
//! silent renorm would hide emitter bugs and flip the reverse-edge read.
//! U(1) carries a single unconstrained phase θ (no unit-norm gate): the
//! injector stores the emitter's chosen θ raw so the round-trip is
//! byte-exact and the HOLONOMY circulation sum stays unwrapped (a linking
//! multiplicity `n·κ` must survive, not fold into `(-π, π]`).

use crate::lattice::{EdgeOrientation, Lattice};
use crate::types::{BundleSchema, Record};

use super::dense_link_buffer::DenseLinkBuffer;
use super::error::GaugeFieldError;
use super::group::Group;

/// Canonical scalar-first SU(2) fiber columns — the same frozen
/// convention `ingest::SU2_FIBER_NAMES` and `su2_gauge_field` pin.
const SU2_FIBER_NAMES: [&str; 4] = ["q0", "q1", "q2", "q3"];

/// Canonical U(1) fiber column — the single phase `theta`, the same
/// convention `ingest::U1_FIBER_NAMES` and `u1_flux` (INIT FLUX) pin.
const U1_FIBER_NAMES: [&str; 1] = ["theta"];

/// Tolerance on `|q|² - 1` for the SU(2) unit-norm gate. `1e-6` is safe:
/// the lens golden Ω = (cos, 0, 0, sin) is unit to f64 (passes), a
/// passing quaternion is stored verbatim so the 1e-12 round-trip still
/// holds, and an emitter that ships a non-unit quaternion (a real bug) is
/// caught rather than silently renormalized.
const UNIT_NORM_TOL: f64 = 1e-6;

/// Build a registry `DenseLinkBuffer` for an SU(2) field on `lattice` from
/// the records of an edge-endpoint bundle.
///
/// The bundle schema must carry base endpoints `vertex_a` / `vertex_b` and
/// the four canonical SU(2) fiber columns `q0..q3` (the same schema INIT
/// FLUX / INGEST AS GAUGE_FIELD emit). Each record plants its chosen
/// quaternion on the directed edge `vertex_a → vertex_b`, orientation
/// handled per the module docstring.
///
/// COMPLETENESS CONTRACT: the caller owns edge coverage. Edges with no
/// record stay identity by design (the buffer starts at `new_identity`);
/// this function does NOT require a record per lattice edge, so a partial
/// emit silently leaves the un-recorded edges identity. An emitter that
/// drops a chosen (e.g. z-wrap) link would read back a *trivial* holonomy
/// there with no error, so emitters must plant every non-identity edge.
/// The shipped `build_su2_bundle` emits one record per lattice edge, so
/// completeness holds for the p-sweep receipt.
///
/// Errors (all typed, never panics — mapped to 4xx at the executor):
/// - [`GaugeFieldError::FiberArityMismatch`] — the schema does not carry
///   all four SU(2) fiber columns (e.g. a theta bundle → GROUP SU(2)).
/// - [`GaugeFieldError::BundleFieldMissing`] — `vertex_a` / `vertex_b` or a
///   fiber value is absent.
/// - [`GaugeFieldError::NonLatticeEdge`] — a record's `(vertex_a,
///   vertex_b)` is not an edge of the bound lattice in either orientation.
/// - [`GaugeFieldError::NonNormalizedQuaternion`] — a chosen quaternion is
///   not unit-norm to within `1e-6` on `|q|²`.
/// - [`GaugeFieldError::BundleEmpty`] — the bundle has zero records.
pub fn su2_buffer_from_bundle(
    bundle_name: &str,
    schema: &BundleSchema,
    records: impl Iterator<Item = Record>,
    lattice: &Lattice,
) -> Result<DenseLinkBuffer, GaugeFieldError> {
    // 1. Schema arity — all four canonical SU(2) fiber columns must be
    //    present (checked over base ∪ fiber so a hand-built bundle that
    //    parks q0..q3 in either group is accepted). Catches a theta bundle
    //    pointed at GROUP SU(2) (got = 0) and a truncated SU(2) bundle.
    let field_names: std::collections::HashSet<&str> = schema
        .base_fields
        .iter()
        .chain(schema.fiber_fields.iter())
        .map(|fd| fd.name.as_str())
        .collect();
    let present = SU2_FIBER_NAMES
        .iter()
        .filter(|n| field_names.contains(**n))
        .count();
    if present != SU2_FIBER_NAMES.len() {
        return Err(GaugeFieldError::FiberArityMismatch {
            bundle: bundle_name.to_string(),
            expected: SU2_FIBER_NAMES.len(),
            got: present,
        });
    }
    for col in ["vertex_a", "vertex_b"] {
        if !field_names.contains(col) {
            return Err(GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: col.to_string(),
            });
        }
    }

    // 2. Identity everywhere; overwrite only the chosen edges.
    let n_edges = lattice.n_edges();
    let mut buf = DenseLinkBuffer::new_identity(Group::SU2, n_edges)?;

    let mut seen = 0usize;
    for rec in records {
        seen += 1;
        let va = rec
            .get("vertex_a")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: "vertex_a".to_string(),
            })?;
        let vb = rec
            .get("vertex_b")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: "vertex_b".to_string(),
            })?;
        let mut q = [0.0_f64; 4];
        for (j, name) in SU2_FIBER_NAMES.iter().enumerate() {
            q[j] = rec.get(*name).and_then(|v| v.as_f64()).ok_or_else(|| {
                GaugeFieldError::BundleFieldMissing {
                    bundle: bundle_name.to_string(),
                    column: (*name).to_string(),
                }
            })?;
        }

        // Resolve the DECLARED directed edge (va → vb).
        let (eid, orient) = resolve_directed(lattice, va, vb)?;

        // Unit-norm gate (inverse == conjugate needs |q| = 1).
        let norm2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        if (norm2 - 1.0).abs() > UNIT_NORM_TOL {
            return Err(GaugeFieldError::NonNormalizedQuaternion {
                edge: eid,
                norm: norm2.sqrt(),
            });
        }

        // Orientation (load-bearing): store Ω on the canonical slot so the
        // DECLARED-direction read returns Ω, not Ω†.
        let stored = match orient {
            EdgeOrientation::Forward => q,
            EdgeOrientation::Reverse => [q[0], -q[1], -q[2], -q[3]],
        };
        buf.write_su2_row(eid, stored);
    }

    if seen == 0 {
        return Err(GaugeFieldError::BundleEmpty(bundle_name.to_string()));
    }

    Ok(buf)
}

/// Build a registry `DenseLinkBuffer` for a U(1) field on `lattice` from
/// the records of an edge-endpoint bundle — the Navier–Stokes vortex
/// linking seam (2026-07-18).
///
/// The bundle schema must carry base endpoints `vertex_a` / `vertex_b` and
/// the single canonical U(1) fiber column `theta` (the same schema INIT
/// FLUX / INGEST AS GAUGE_FIELD emit). Each record plants its chosen phase
/// on the directed edge `vertex_a → vertex_b`.
///
/// ORIENTATION (load-bearing, mirrors the SU(2) arm): the buffer slot for
/// edge `eid` holds the canonical phase on the lattice's stored direction
/// `edges[eid] = (u, v)`, and `U1GaugeField::edge_element(eid, Forward)`
/// returns it as-is while `edge_element(eid, Reverse)` returns its inverse
/// `−θ`. So to make HOLONOMY read back the *intended* circulation on the
/// directed edge `va → vb` a record declares, we store:
///   -  θ  when `resolve_edge(va, vb) == Forward`  (edges[eid]=(va,vb))
///   - −θ  when `resolve_edge(va, vb) == Reverse`  (edges[eid]=(vb,va))
/// so `edge_element(eid, resolve_orient)` recovers θ exactly. A Forward
/// record round-trips to +θ (the intended +circulation sign).
///
/// COMPLETENESS CONTRACT (same as the SU(2) arm): edges with no record
/// stay identity (θ = 0) by design — the buffer starts at `new_identity`;
/// emitters must plant every non-identity edge.
///
/// NO unit-norm gate: U(1) carries a single unconstrained phase (θ = κ can
/// be any real circulation), stored raw so the round-trip is byte-exact
/// and the holonomy circulation sum stays unwrapped.
///
/// Errors (all typed, never panics — mapped to 4xx at the executor):
/// - [`GaugeFieldError::FiberArityMismatch`] — the schema does not carry
///   the `theta` fiber column (e.g. a `q0..q3` bundle → GROUP U(1): got=0).
/// - [`GaugeFieldError::BundleFieldMissing`] — `vertex_a` / `vertex_b` or
///   the `theta` value is absent on a record.
/// - [`GaugeFieldError::NonLatticeEdge`] — a record's `(vertex_a,
///   vertex_b)` is not an edge of the bound lattice in either orientation.
/// - [`GaugeFieldError::BundleEmpty`] — the bundle has zero records.
pub fn u1_buffer_from_bundle(
    bundle_name: &str,
    schema: &BundleSchema,
    records: impl Iterator<Item = Record>,
    lattice: &Lattice,
) -> Result<DenseLinkBuffer, GaugeFieldError> {
    // 1. Schema arity — the single canonical U(1) fiber column `theta`
    //    must be present (checked over base ∪ fiber). Catches a q0..q3
    //    (SU(2)-shaped) bundle pointed at GROUP U(1) (got = 0).
    let field_names: std::collections::HashSet<&str> = schema
        .base_fields
        .iter()
        .chain(schema.fiber_fields.iter())
        .map(|fd| fd.name.as_str())
        .collect();
    let present = U1_FIBER_NAMES
        .iter()
        .filter(|n| field_names.contains(**n))
        .count();
    if present != U1_FIBER_NAMES.len() {
        return Err(GaugeFieldError::FiberArityMismatch {
            bundle: bundle_name.to_string(),
            expected: U1_FIBER_NAMES.len(),
            got: present,
        });
    }
    for col in ["vertex_a", "vertex_b"] {
        if !field_names.contains(col) {
            return Err(GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: col.to_string(),
            });
        }
    }

    // 2. Identity everywhere (θ = 0); overwrite only the chosen edges.
    let n_edges = lattice.n_edges();
    let mut buf = DenseLinkBuffer::new_identity(Group::U1, n_edges)?;

    let mut seen = 0usize;
    for rec in records {
        seen += 1;
        let va = rec
            .get("vertex_a")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: "vertex_a".to_string(),
            })?;
        let vb = rec
            .get("vertex_b")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: "vertex_b".to_string(),
            })?;
        let theta = rec
            .get("theta")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| GaugeFieldError::BundleFieldMissing {
                bundle: bundle_name.to_string(),
                column: "theta".to_string(),
            })?;

        // Resolve the DECLARED directed edge (va → vb).
        let (eid, orient) = resolve_directed(lattice, va, vb)?;

        // Orientation (load-bearing): store θ on the canonical slot so the
        // DECLARED-direction read returns θ, not −θ. Forward = θ, Reverse
        // = −θ (the U(1) inverse).
        let stored = match orient {
            EdgeOrientation::Forward => theta,
            EdgeOrientation::Reverse => -theta,
        };
        buf.write_u1_row(eid, stored);
    }

    if seen == 0 {
        return Err(GaugeFieldError::BundleEmpty(bundle_name.to_string()));
    }

    Ok(buf)
}

/// Resolve `(va, vb)` to `(edge_id, orientation)`, mapping a non-edge (or
/// out-of-range vertex) to the typed [`GaugeFieldError::NonLatticeEdge`].
fn resolve_directed(
    lattice: &Lattice,
    va: i64,
    vb: i64,
) -> Result<(usize, EdgeOrientation), GaugeFieldError> {
    // Negative / out-of-range vertices can never index the lattice; a
    // lossy `as usize` cast would wrap, but `resolve_edge` then returns
    // None → the same typed NonLatticeEdge error (never a panic).
    if va < 0 || vb < 0 {
        return Err(GaugeFieldError::NonLatticeEdge {
            vertex_a: va,
            vertex_b: vb,
        });
    }
    lattice
        .resolve_edge(va as usize, vb as usize)
        .ok_or(GaugeFieldError::NonLatticeEdge {
            vertex_a: va,
            vertex_b: vb,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gauge::edge_connection::EdgeConnection;
    use crate::gauge::group_element::GroupElement;
    use crate::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
    use crate::lattice::topology::truncated_icosahedron::buckyball;
    use crate::types::{FieldDef, Value};

    fn su2_schema() -> BundleSchema {
        BundleSchema::new("b")
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("q0"))
            .fiber(FieldDef::numeric("q1"))
            .fiber(FieldDef::numeric("q2"))
            .fiber(FieldDef::numeric("q3"))
    }

    fn rec(va: usize, vb: usize, q: [f64; 4]) -> Record {
        let mut r = Record::new();
        r.insert("vertex_a".into(), Value::Integer(va as i64));
        r.insert("vertex_b".into(), Value::Integer(vb as i64));
        r.insert("q0".into(), Value::Float(q[0]));
        r.insert("q1".into(), Value::Float(q[1]));
        r.insert("q2".into(), Value::Float(q[2]));
        r.insert("q3".into(), Value::Float(q[3]));
        r
    }

    fn unpack(g: GroupElement) -> [f64; 4] {
        match g {
            GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
            other => panic!("expected SU2, got {other:?}"),
        }
    }

    /// A record in the lattice's canonical (u → v) direction stores Ω
    /// verbatim, and edge_element(Forward) reads it back exactly.
    #[test]
    fn forward_record_stores_as_is() {
        let lat = buckyball();
        let (u, v) = lat.edges[0];
        let ang = std::f64::consts::PI / 5.0;
        let om = [(ang / 2.0).cos(), 0.0, 0.0, (ang / 2.0).sin()];
        let buf = su2_buffer_from_bundle("b", &su2_schema(), vec![rec(u, v, om)].into_iter(), &lat)
            .expect("inject");
        let field =
            SU2GaugeField::from_buffer("f".into(), lat.name.clone(), buf, GaugeFieldInit::Identity, None);
        let got = unpack(field.edge_element(0, EdgeOrientation::Forward));
        for j in 0..4 {
            assert!((got[j] - om[j]).abs() < 1e-12, "comp {j}");
        }
    }

    /// A record in the REVERSED (v → u) direction stores Ω† in the canonical
    /// slot, so reading in the DECLARED direction (Reverse) returns Ω.
    #[test]
    fn reverse_record_returns_omega_not_dagger() {
        let lat = buckyball();
        let (u, v) = lat.edges[0];
        let ang = std::f64::consts::PI / 5.0;
        let om = [(ang / 2.0).cos(), 0.0, 0.0, (ang / 2.0).sin()];
        // Declared reversed: va=v, vb=u.
        let buf = su2_buffer_from_bundle("b", &su2_schema(), vec![rec(v, u, om)].into_iter(), &lat)
            .expect("inject");
        let field =
            SU2GaugeField::from_buffer("f".into(), lat.name.clone(), buf, GaugeFieldInit::Identity, None);
        // resolve_edge(v, u) is Reverse of the canonical (u, v).
        let (eid, orient) = lat.resolve_edge(v, u).expect("edge");
        assert_eq!(orient, EdgeOrientation::Reverse);
        let declared = unpack(field.edge_element(eid, orient));
        for j in 0..4 {
            assert!((declared[j] - om[j]).abs() < 1e-12, "declared comp {j}");
        }
        // Canonical Forward slot is Ω† (vector negated).
        let canon = unpack(field.edge_element(eid, EdgeOrientation::Forward));
        assert!((canon[0] - om[0]).abs() < 1e-12);
        assert!((canon[3] + om[3]).abs() < 1e-12);
    }

    /// A non-unit quaternion is rejected, not renormalized.
    #[test]
    fn non_normalized_is_rejected() {
        let lat = buckyball();
        let (u, v) = lat.edges[0];
        let err = su2_buffer_from_bundle(
            "b",
            &su2_schema(),
            vec![rec(u, v, [2.0, 0.0, 0.0, 0.0])].into_iter(),
            &lat,
        )
        .expect_err("non-unit must reject");
        matches!(err, GaugeFieldError::NonNormalizedQuaternion { .. })
            .then_some(())
            .expect("NonNormalizedQuaternion");
    }

    /// A theta-shaped schema (no q columns) → arity mismatch, got = 0.
    #[test]
    fn theta_schema_is_arity_mismatch() {
        let lat = buckyball();
        let schema = BundleSchema::new("t")
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("theta"));
        let err = su2_buffer_from_bundle("t", &schema, std::iter::empty(), &lat)
            .expect_err("theta schema must reject");
        match err {
            GaugeFieldError::FiberArityMismatch { expected, got, .. } => {
                assert_eq!(expected, 4);
                assert_eq!(got, 0);
            }
            other => panic!("expected FiberArityMismatch, got {other:?}"),
        }
    }

    /// A non-edge (va, vb) pair → typed NonLatticeEdge, not a panic.
    #[test]
    fn non_lattice_edge_is_typed() {
        let lat = buckyball();
        let err = su2_buffer_from_bundle(
            "b",
            &su2_schema(),
            vec![rec(0, 9999, [1.0, 0.0, 0.0, 0.0])].into_iter(),
            &lat,
        )
        .expect_err("non-edge must reject");
        match err {
            GaugeFieldError::NonLatticeEdge { vertex_a, vertex_b } => {
                assert_eq!(vertex_a, 0);
                assert_eq!(vertex_b, 9999);
            }
            other => panic!("expected NonLatticeEdge, got {other:?}"),
        }
    }

    /// An empty bundle → typed BundleEmpty.
    #[test]
    fn empty_bundle_is_typed() {
        let lat = buckyball();
        let err = su2_buffer_from_bundle("b", &su2_schema(), std::iter::empty(), &lat)
            .expect_err("empty must reject");
        matches!(err, GaugeFieldError::BundleEmpty(_))
            .then_some(())
            .expect("BundleEmpty");
    }

    // ── U(1) inject arm ──────────────────────────────────────────────

    fn u1_schema() -> BundleSchema {
        BundleSchema::new("t")
            .base(FieldDef::numeric("vertex_a"))
            .base(FieldDef::numeric("vertex_b"))
            .fiber(FieldDef::numeric("theta"))
    }

    fn u1_rec(va: usize, vb: usize, theta: f64) -> Record {
        let mut r = Record::new();
        r.insert("vertex_a".into(), Value::Integer(va as i64));
        r.insert("vertex_b".into(), Value::Integer(vb as i64));
        r.insert("theta".into(), Value::Float(theta));
        r
    }

    fn theta_of(g: GroupElement) -> f64 {
        match g {
            GroupElement::U1 { theta } => theta,
            other => panic!("expected U1, got {other:?}"),
        }
    }

    /// A U(1) record in the lattice's canonical (u → v) direction stores θ
    /// verbatim; read_element(Forward) reads it back exactly.
    #[test]
    fn u1_forward_record_stores_as_is() {
        let lat = buckyball();
        let (u, v) = lat.edges[0];
        let buf = u1_buffer_from_bundle("t", &u1_schema(), vec![u1_rec(u, v, 0.73)].into_iter(), &lat)
            .expect("inject");
        let (eid, orient) = lat.resolve_edge(u, v).expect("edge");
        assert_eq!(orient, EdgeOrientation::Forward);
        assert!((theta_of(buf.read_element(eid)) - 0.73).abs() < 1e-12);
    }

    /// A U(1) record in the REVERSED (v → u) direction stores −θ in the
    /// canonical slot, so reading in the DECLARED (Reverse) direction
    /// recovers +θ.
    #[test]
    fn u1_reverse_record_stores_minus_theta() {
        let lat = buckyball();
        let (u, v) = lat.edges[0];
        let buf = u1_buffer_from_bundle("t", &u1_schema(), vec![u1_rec(v, u, 0.73)].into_iter(), &lat)
            .expect("inject");
        let (eid, orient) = lat.resolve_edge(v, u).expect("edge");
        assert_eq!(orient, EdgeOrientation::Reverse);
        // Canonical Forward slot holds −θ.
        assert!((theta_of(buf.read_element(eid)) + 0.73).abs() < 1e-12);
    }

    /// A q0..q3 (SU(2)-shaped) schema pointed at the U(1) arm → arity
    /// mismatch, expected 1 (theta), got 0.
    #[test]
    fn u1_q_schema_is_arity_mismatch() {
        let lat = buckyball();
        let err = u1_buffer_from_bundle("q", &su2_schema(), std::iter::empty(), &lat)
            .expect_err("q0..q3 schema must reject for U(1)");
        match err {
            GaugeFieldError::FiberArityMismatch { expected, got, .. } => {
                assert_eq!(expected, 1);
                assert_eq!(got, 0);
            }
            other => panic!("expected FiberArityMismatch, got {other:?}"),
        }
    }
}
