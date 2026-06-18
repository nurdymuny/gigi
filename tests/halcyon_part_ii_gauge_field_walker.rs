//! TDD-HAL-II.7 — Gold-walker swap.
//!
//! The Part I gold gate (`tdd_hal_i_6_bit_identity_face_holonomy_gold`)
//! exercised `walk_loop` through a synthetic `UFinalConnection`
//! built ad-hoc inside the test crate. Part II ships
//! `SU2GaugeField` as the first production `EdgeConnection` impl —
//! this gate replays the same fixture data through that production
//! impl and asserts byte-equal-to-tolerance (≤ 1e-12) match against
//! the gold face holonomies.
//!
//! The architectural payoff (Bee's locked decision 6): the walker
//! reads through `&dyn EdgeConnection`. Only the concrete type
//! behind the trait changes (`UFinalConnection` →
//! `SU2GaugeField`). When a future `U1GaugeField` or
//! `SU3GaugeField` ships, this same gold gate replays through that
//! new impl with zero changes to the walker, the registry, the
//! parser, or the HTTP routes.
//!
//! Fixture: `tests/fixtures/halcyon/buckyball_su2_u_final_gold.json`
//! (90 × 4 quaternions, scalar-first) and
//! `tests/fixtures/halcyon/buckyball_face_holonomy_gold.json`
//! (32 × 4 quaternions). Both fixtures are the same files Part I
//! used — see `tests/halcyon_part_i_bit_identity.rs` for the
//! original gate and `buckyball_gold_provenance.json` for the
//! harvest contract.

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::gauge::dense_link_buffer::DenseLinkBuffer;
use gigi::gauge::edge_connection::EdgeConnection;
use gigi::gauge::group::Group;
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::holonomy::walk_loop;
use gigi::gauge::su2_gauge_field::{GaugeFieldInit, SU2GaugeField};
use gigi::lattice::topology::truncated_icosahedron::{
    buckyball_with_signed_faces, signed_face_to_walker,
};

/// Load a 2-D array of f64 quaternions from a gold JSON file. The
/// fixture uses the human-readable `data` field (decimal f64); the
/// Part I gate's loader matches this shape, so we mirror it.
fn load_quat_array(path: PathBuf) -> Vec<[f64; 4]> {
    let body = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let data = v["data"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: missing `data` array", path.display()));
    data.iter()
        .map(|row| {
            let r = row.as_array().expect("row not an array");
            [
                r[0].as_f64().expect("q0 not f64"),
                r[1].as_f64().expect("q1 not f64"),
                r[2].as_f64().expect("q2 not f64"),
                r[3].as_f64().expect("q3 not f64"),
            ]
        })
        .collect()
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join(name)
}

/// TDD-HAL-II.7 — Part I gold gate replayed through the production
/// `SU2GaugeField` `EdgeConnection` impl.
///
/// Loads the same `buckyball_su2_u_final_gold.json` Part I used,
/// flattens it into a `DenseLinkBuffer { group: SU2, n_edges: 90,
/// repr_dim: 4 }`, wraps that buffer in an `SU2GaugeField` via the
/// `#[cfg(test)]` `from_buffer` factory (production code always
/// goes through `SU2GaugeField::new` with an INIT clause; this
/// gate is loading a fixture, not running INIT), registers it
/// against the buckyball lattice, and walks every signed face
/// through `walk_loop` reading via the `&dyn EdgeConnection` for
/// the field.
///
/// Pass criterion: every face holonomy matches the gold within
/// 1e-12 per quaternion component (same tolerance the Part I gate
/// holds). In practice the walker is pure multiply-add on
/// already-quaternion inputs (no trig), so the receipt lands ≤
/// 1e-14.
#[test]
fn tdd_hal_ii_7_gold_walker_through_gauge_field() {
    let bb = buckyball_with_signed_faces();
    assert_eq!(bb.lattice.n_edges(), 90);
    assert_eq!(bb.lattice.n_faces(), 32);
    assert_eq!(bb.signed_faces.len(), 32);

    // Load the same Part I gold fixtures.
    let u_final = load_quat_array(fixture("buckyball_su2_u_final_gold.json"));
    let face_gold = load_quat_array(fixture("buckyball_face_holonomy_gold.json"));
    assert_eq!(u_final.len(), 90, "U_final must have 90 edges");
    assert_eq!(face_gold.len(), 32, "face_holonomy must have 32 faces");

    // Flatten the (90, 4) gold into a row-major Vec<f64> and build
    // the DenseLinkBuffer directly (the buffer is group-erased
    // storage; this is the same shape `new_identity` /
    // `new_haar` produce).
    let mut flat = Vec::with_capacity(90 * 4);
    for q in &u_final {
        flat.push(q[0]);
        flat.push(q[1]);
        flat.push(q[2]);
        flat.push(q[3]);
    }
    let buffer = DenseLinkBuffer {
        group: Group::SU2,
        n_edges: 90,
        repr_dim: 4,
        data: flat,
    };

    // Wrap as an SU2GaugeField via the test-only factory. The
    // `init_kind` records that this field was hydrated from a
    // pre-built buffer (Identity is the closest production
    // equivalent — there was no INIT routine).
    let field = SU2GaugeField::from_buffer(
        "U_gold".into(),
        bb.lattice.name.clone(),
        buffer,
        GaugeFieldInit::Identity,
        None,
    );

    // Architectural contract: the walker reads through
    // `&dyn EdgeConnection`, never the concrete `SU2GaugeField`.
    // The Part I `UFinalConnection` is gone from this code path
    // entirely (it stays only as a `#[cfg(test)]` helper in the
    // Part I bit-identity test file, used by
    // `tdd_hal_i_5_orientation_sensitivity`).
    let conn: &dyn EdgeConnection = &field;

    let tol = 1e-12;
    let mut mismatches: Vec<(usize, [f64; 4], [f64; 4])> = Vec::new();

    for fidx in 0..bb.lattice.n_faces() {
        let edges = signed_face_to_walker(&bb.signed_faces[fidx]);
        let h = walk_loop(&bb.lattice, &edges, conn);
        let walked = match h {
            GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
            _ => panic!("expected SU2"),
        };
        let gold = face_gold[fidx];
        let diffs = [
            (walked[0] - gold[0]).abs(),
            (walked[1] - gold[1]).abs(),
            (walked[2] - gold[2]).abs(),
            (walked[3] - gold[3]).abs(),
        ];
        if diffs.iter().any(|&d| d >= tol) {
            mismatches.push((fidx, walked, gold));
        }
    }

    assert!(
        mismatches.is_empty(),
        "face holonomies disagree (tol = {tol:.0e}):\n{}",
        mismatches
            .iter()
            .take(8)
            .map(|(f, w, g)| format!(
                "  face {f}: walked = [{:.6e}, {:.6e}, {:.6e}, {:.6e}], gold = [{:.6e}, {:.6e}, {:.6e}, {:.6e}], max-diff = {:.3e}",
                w[0], w[1], w[2], w[3], g[0], g[1], g[2], g[3],
                (0..4).map(|i| (w[i] - g[i]).abs()).fold(0.0_f64, f64::max)
            ))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Architectural-contract receipt: a `Box<dyn EdgeConnection>`
/// over an `SU2GaugeField` built from the gold buffer walks the
/// same face holonomies as the bare-reference version. This is
/// the trait-object proof — the walker, the registry, and the
/// HTTP routes never name the concrete `SU2GaugeField` type.
#[test]
fn tdd_hal_ii_7_gauge_field_works_as_trait_object() {
    let bb = buckyball_with_signed_faces();
    let u_final = load_quat_array(fixture("buckyball_su2_u_final_gold.json"));
    let face_gold = load_quat_array(fixture("buckyball_face_holonomy_gold.json"));

    let mut flat = Vec::with_capacity(90 * 4);
    for q in &u_final {
        flat.extend_from_slice(q);
    }
    let buffer = DenseLinkBuffer {
        group: Group::SU2,
        n_edges: 90,
        repr_dim: 4,
        data: flat,
    };
    let field = SU2GaugeField::from_buffer(
        "U_gold".into(),
        bb.lattice.name.clone(),
        buffer,
        GaugeFieldInit::Identity,
        None,
    );

    // Trait-object path: the field is hidden behind a Box<dyn …>.
    let boxed: Box<dyn EdgeConnection> = Box::new(field);

    let tol = 1e-12;
    for fidx in 0..bb.lattice.n_faces() {
        let edges = signed_face_to_walker(&bb.signed_faces[fidx]);
        let h = walk_loop(&bb.lattice, &edges, boxed.as_ref());
        let walked = match h {
            GroupElement::SU2 { q0, q1, q2, q3 } => [q0, q1, q2, q3],
            _ => panic!("expected SU2"),
        };
        let gold = face_gold[fidx];
        for i in 0..4 {
            assert!(
                (walked[i] - gold[i]).abs() < tol,
                "face {fidx} component {i}: walked = {}, gold = {}",
                walked[i],
                gold[i]
            );
        }
    }
}
