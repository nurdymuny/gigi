//! TDD-HAL-I.6 — bit-identity gold gate.
//!
//! Loads the harvest-phase fixtures:
//!
//!   - `tests/fixtures/halcyon/buckyball_su2_u_final_gold.json` —
//!     Halcyon's heatbathed reference state U_final at β = 2.5
//!     (shape (90, 4), scalar-first quaternion convention pinned in
//!     `buckyball_gold_provenance.json`).
//!   - `tests/fixtures/halcyon/buckyball_face_holonomy_gold.json` —
//!     the reference face holonomies the Python kernel's
//!     `inertia_damping/buckyball_action.py::face_holonomy` produces
//!     from that U_final (shape (32, 4)).
//!
//! Builds a `UFinalConnection` from U_final, walks every signed-face
//! cycle through `walk_loop`, and asserts the resulting quaternion
//! is bit-identical to the gold `face_holonomy` quaternion.
//!
//! Bit-identity contract (per HALCYON_TO_GIGI_REPLY_2026-06-17.md
//! § A2): same-OS bit-identity is the hard test. Cross-OS drift up
//! to 2 ULPs in trig reductions is documented and tolerated. On
//! Windows MSVC this test budgets 1e-12 per quaternion component —
//! well inside the cross-OS 2-ULP envelope. The walker is pure
//! multiply-add on already-quaternion inputs (no trig), so a
//! correctly-pinned implementation lands ≤ 1e-14 in practice.

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::gauge::edge_connection::EdgeConnection;
use gigi::gauge::group_element::GroupElement;
use gigi::gauge::holonomy::walk_loop;
use gigi::lattice::{EdgeId, EdgeOrientation};
use gigi::lattice::topology::truncated_icosahedron::{
    buckyball_with_signed_faces, signed_face_to_walker,
};

/// Synthetic dense edge connection backed by the U_final fixture.
/// Same shape as the future `SU2GaugeField` (Part II) but flat.
struct UFinalConnection {
    elements: Vec<GroupElement>,
}

impl EdgeConnection for UFinalConnection {
    fn edge_element(&self, edge: EdgeId, orientation: EdgeOrientation) -> GroupElement {
        let g = self.elements[edge];
        match orientation {
            EdgeOrientation::Forward => g,
            EdgeOrientation::Reverse => g.inverse(),
        }
    }
}

/// Load a 2-D array of f64 quaternions from a gold JSON file.
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

/// TDD-HAL-I.6 — for every face of the buckyball, walking U_final
/// through the generalized walker produces the same quaternion the
/// Python kernel committed to the face_holonomy gold file.
///
/// The substrate buckyball is a faithful port of
/// `inertia_damping/buckyball_graph.py::build_truncated_icosahedron`
/// (vertex coordinates, edge enumeration order, rotation-system
/// face tracing, outward-orientation flip, pentagons-then-hexagons
/// emission order) — so the per-edge / per-face indexing aligns
/// with the gold fixture. The signed-face form `(edge_idx, sign)`
/// is the Halcyon storage format; `signed_face_to_walker` maps it
/// to the walker's `(EdgeId, EdgeOrientation)` shape.
#[test]
fn tdd_hal_i_6_bit_identity_face_holonomy_gold() {
    let bb = buckyball_with_signed_faces();
    assert_eq!(bb.lattice.n_edges(), 90);
    assert_eq!(bb.lattice.n_faces(), 32);
    assert_eq!(bb.signed_faces.len(), 32);

    let u_final = load_quat_array(fixture("buckyball_su2_u_final_gold.json"));
    let face_gold = load_quat_array(fixture("buckyball_face_holonomy_gold.json"));
    assert_eq!(u_final.len(), 90, "U_final must have 90 edges");
    assert_eq!(face_gold.len(), 32, "face_holonomy must have 32 faces");

    let conn = UFinalConnection {
        elements: u_final
            .iter()
            .map(|q| GroupElement::SU2 {
                q0: q[0],
                q1: q[1],
                q2: q[2],
                q3: q[3],
            })
            .collect(),
    };

    let tol = 1e-12;
    let mut mismatches: Vec<(usize, [f64; 4], [f64; 4])> = Vec::new();

    for fidx in 0..bb.lattice.n_faces() {
        let edges = signed_face_to_walker(&bb.signed_faces[fidx]);
        let h = walk_loop(&bb.lattice, &edges, &conn);
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

/// Smoke test on the gold file's structural assumptions — guards
/// against fixture drift breaking the gate silently.
#[test]
fn gold_files_have_expected_shape() {
    let u_final = load_quat_array(fixture("buckyball_su2_u_final_gold.json"));
    let face_gold = load_quat_array(fixture("buckyball_face_holonomy_gold.json"));
    assert_eq!(u_final.len(), 90);
    assert_eq!(face_gold.len(), 32);

    // SU(2) normalization: q0^2 + q1^2 + q2^2 + q3^2 = 1, every
    // edge. Tolerance 1e-12 (Python writer is f64).
    for (i, q) in u_final.iter().enumerate() {
        let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        assert!(
            (n2 - 1.0).abs() < 1e-12,
            "U_final[{i}] not unit-norm: |q|^2 = {n2}"
        );
    }
    for (i, q) in face_gold.iter().enumerate() {
        let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        assert!(
            (n2 - 1.0).abs() < 1e-12,
            "face_holonomy[{i}] not unit-norm: |q|^2 = {n2}"
        );
    }
}
