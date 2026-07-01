//! Concept 1 (Ask 1) — LATTICE FROM CUBIC OBC AXIS <k>.
//!
//! RED test file for the SU(2) 4D L=24 β=2.3 OBC sectoral SPECTRAL_GAUGE
//! workflow. These tests exercise:
//!
//! 1. The Rust-level `cubic()` constructor extended with an
//!    `obc_axis: Option<usize>` trailing argument. When `Some(k)`, wrap
//!    edges and plaquettes crossing the axis-k boundary at the L-1 slice
//!    are omitted; vertex count stays L^D.
//! 2. The parser-level grammar `LATTICE name FROM CUBIC L=<int> DIM=<int>
//!    OBC AXIS <int>;` which flows through the existing
//!    `Statement::LatticeFromCanonical { params }` sidecar as a new entry
//!    keyed `"OBC_AXIS"` with `Literal::Integer(k)`.
//! 3. Composition guards: OPEN + OBC AXIS together errors; OBC AXIS with
//!    an out-of-range axis errors; PERIODIC (no OBC clause) still works
//!    byte-identically as today.
//!
//! These tests are the RED signal — they fail to compile against the
//! current `cubic(name, l, d, periodic: bool)` signature, because they
//! call `cubic(name, l, d, periodic, obc_axis)` with a fifth argument.
//! The parser tests also expect `params` to contain an `"OBC_AXIS"`
//! entry, which the current parser does not emit.
//!
//! The GREEN commit will land the fifth argument on `cubic()`, add
//! `obc_axis: Option<usize>` to `ConstructorArgs`, and teach the parser
//! loop to recognize the three-token `OBC AXIS <int>` sequence.

#![cfg(feature = "lattice")]

use gigi::lattice::topology::cubic::cubic;
use gigi::parser;

// ── (1) Direct-constructor tests ─────────────────────────────────────

/// L=4 D=2 OBC AXIS 0 — the smallest fixture that exercises the
/// wrap-edge drop and the boundary-plaquette drop.
///
/// PERIODIC L=4 D=2 has E = L^D · D = 32.
/// OBC AXIS 0 drops L^(D-1) = 4 wrap edges on axis 0.
/// Expect E = 28.
#[test]
fn test_cubic_obc_axis_0_l4_d2_edge_count() {
    let lwm = cubic("l4_obc0", 4, 2, true, Some(0));
    let lat = lwm.lattice();

    assert_eq!(lat.n_vertices, 16, "OBC keeps sites: V = L^D = 16");
    assert_eq!(
        lat.n_edges(),
        28,
        "OBC AXIS 0 drops L^(D-1) = 4 wrap edges: E = 32 - 4 = 28"
    );
}

/// L=4 D=2 OBC AXIS 0 face count.
///
/// PERIODIC L=4 D=2 has F = L^D · C(D,2) = 16 · 1 = 16.
/// OBC AXIS 0 drops (D-1) · L^(D-1) = 1 · 4 = 4 boundary plaquettes
/// that would wrap through the axis-0 boundary.
/// Expect F = 12.
#[test]
fn test_cubic_obc_axis_0_l4_d2_face_count() {
    let lwm = cubic("l4_obc0", 4, 2, true, Some(0));
    let lat = lwm.lattice();

    assert_eq!(
        lat.n_faces(),
        12,
        "OBC AXIS 0 drops (D-1)·L^(D-1) = 4 boundary plaquettes: F = 16 - 4 = 12"
    );
}

/// L=24 D=4 OBC AXIS 0 — Hallie's target Halcyon workflow dimensions.
///
/// Rather than assert absolute magic numbers (the design brief noted an
/// L=12 vs L=24 confusion), we assert the DIFF from PERIODIC — the
/// invariant that Concept 1's OBC path is expected to preserve:
///
///   V(OBC) == V(PERIODIC)              (open BC keeps sites)
///   E(OBC) == E(PERIODIC) - L^(D-1)    (drops wrap edges on axis k)
///   F(OBC) == F(PERIODIC) - (D-1)·L^(D-1)
///                                       (drops boundary plaquettes on
///                                        every axis pair touching k)
#[test]
fn test_cubic_obc_axis_0_l24_d4_counts_match_letter() {
    let l = 24usize;
    let d = 4usize;

    let periodic = cubic("l24_periodic", l, d, true, None);
    let obc = cubic("l24_obc0", l, d, true, Some(0));

    let plat = periodic.lattice();
    let olat = obc.lattice();

    // Sites unchanged.
    assert_eq!(
        olat.n_vertices, plat.n_vertices,
        "OBC preserves V = L^D = {}",
        l.pow(d as u32)
    );

    let wrap_edges_dropped = l.pow((d - 1) as u32);
    assert_eq!(
        plat.n_edges() - olat.n_edges(),
        wrap_edges_dropped,
        "OBC AXIS 0 drops L^(D-1) = {wrap_edges_dropped} wrap edges"
    );

    let boundary_faces_dropped = (d - 1) * l.pow((d - 1) as u32);
    assert_eq!(
        plat.n_faces() - olat.n_faces(),
        boundary_faces_dropped,
        "OBC AXIS 0 drops (D-1)·L^(D-1) = {boundary_faces_dropped} boundary plaquettes"
    );
}

/// PERIODIC (obc_axis = None) still produces the historical Halcyon
/// §3.3 counts. This is the byte-identical backwards-compat guard.
#[test]
fn test_cubic_default_still_periodic() {
    let lwm = cubic("l12_periodic", 12, 4, true, None);
    let lat = lwm.lattice();

    assert_eq!(lat.n_vertices, 20_736, "V = 12^4 = 20736");
    assert_eq!(lat.n_edges(), 82_944, "E = 12^4 · 4 = 82944");
    assert_eq!(lat.n_faces(), 124_416, "F = 12^4 · 6 = 124416");
    assert_eq!(lat.topology.as_deref(), Some("CUBIC_L12_D4"));
}

/// L=24 D=4 OBC AXIS 0 — assert the ABSOLUTE cell counts, not just the
/// DIFF from PERIODIC. This is the sanity gate the review flagged:
/// PERIODIC E = D · L^D = 4 · 24^4 = 1_327_104; drop L^(D-1) = 13_824
/// wrap edges → OBC E = 1_313_280. PERIODIC F = C(D,2) · L^D
/// = 6 · 24^4 = 1_990_656; drop (D-1) · L^(D-1) = 3 · 13_824 = 41_472
/// boundary plaquettes → OBC F = 1_949_184. Vertices unchanged at
/// L^D = 331_776.
///
/// A dimensional slip that stated the counts as `E = 68_928` /
/// `F = 82_944` (missing a factor of L on the periodic base) would be
/// caught here — those numbers are L^(D-1) accounting where L^D was
/// intended, and cannot survive this assertion.
#[test]
fn test_cubic_obc_axis_0_l24_d4_absolute_counts() {
    let lwm = cubic("l24_obc0_absolute", 24, 4, true, Some(0));
    let lat = lwm.lattice();

    assert_eq!(
        lat.n_vertices, 331_776,
        "V = 24^4 = 331_776 (OBC keeps sites)"
    );
    assert_eq!(
        lat.n_edges(),
        1_313_280,
        "E_obc = D·L^D − L^(D−1) = 4·24^4 − 24^3 = 1_327_104 − 13_824 = 1_313_280"
    );
    assert_eq!(
        lat.n_faces(),
        1_949_184,
        "F_obc = C(D,2)·L^D − (D−1)·L^(D−1) = 6·24^4 − 3·24^3 = 1_990_656 − 41_472 = 1_949_184"
    );
}

/// OBC AXIS >= DIM must error (out-of-range axis index).
#[test]
#[should_panic(expected = "OBC AXIS")]
fn test_cubic_obc_axis_out_of_bounds_errors() {
    // D=3 so valid axes are 0, 1, 2. Axis 3 is out of range.
    let _ = cubic("bad", 4, 3, true, Some(3));
}

/// The OBC topology hint carries the axis index so downstream verbs
/// (BETTI, CHERN_CLASS, SPECTRAL_GAUGE) can dispatch on it.
#[test]
fn test_cubic_obc_axis_topology_hint_carries_axis() {
    let lwm = cubic("l4_obc0", 4, 2, true, Some(0));
    let lat = lwm.lattice();

    assert_eq!(
        lat.topology.as_deref(),
        Some("CUBIC_L4_D2_OBC_AXIS0"),
        "OBC topology hint must name the open axis"
    );
}

// ── (2) Parser tests ─────────────────────────────────────────────────

/// `LATTICE l24 FROM CUBIC L=24 DIM=4 OBC AXIS 0;` parses to a
/// `LatticeFromCanonical` statement whose `params` contains an
/// `"OBC_AXIS"` entry with `Literal::Integer(0)`.
#[test]
fn test_lattice_parse_obc_axis_0() {
    let src = "LATTICE l24 FROM CUBIC L=24 DIM=4 OBC AXIS 0;";
    let stmt = parser::parse(src).expect("parse OBC AXIS 0");

    match stmt {
        parser::Statement::LatticeFromCanonical {
            name,
            canonical,
            params,
            ..
        } => {
            assert_eq!(name, "l24");
            assert_eq!(canonical, "CUBIC");

            // Locate the OBC_AXIS param.
            let obc_axis_param = params
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("OBC_AXIS"))
                .expect("params must contain OBC_AXIS entry");
            match &obc_axis_param.1 {
                parser::Literal::Integer(n) => assert_eq!(
                    *n, 0,
                    "OBC_AXIS param value must be the parsed axis index"
                ),
                other => panic!("OBC_AXIS param must be Integer(0), got {other:?}"),
            }
        }
        other => panic!("expected LatticeFromCanonical, got {other:?}"),
    }
}

/// Backwards-compat: the historical `LATTICE ... FROM CUBIC L=12 DIM=4
/// PERIODIC;` grammar still parses with no `OBC_AXIS` entry.
#[test]
fn test_lattice_parse_periodic_still_works() {
    let src = "LATTICE my4d FROM CUBIC L=12 DIM=4 PERIODIC;";
    let stmt = parser::parse(src).expect("parse PERIODIC");

    match stmt {
        parser::Statement::LatticeFromCanonical { params, .. } => {
            assert!(
                params
                    .iter()
                    .all(|(k, _)| !k.eq_ignore_ascii_case("OBC_AXIS")),
                "PERIODIC form must NOT emit an OBC_AXIS param"
            );
            assert!(
                params
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("PERIODIC")),
                "PERIODIC keyword must still surface in params"
            );
        }
        other => panic!("expected LatticeFromCanonical, got {other:?}"),
    }
}

/// Composition guard: `OPEN` and `OBC AXIS <k>` together are ambiguous
/// — the executor's `resolve_constructor_args` must reject the combo
/// so users pick one clear form.
#[test]
fn test_lattice_parse_obc_and_open_together_errors() {
    let src = "LATTICE bad FROM CUBIC L=4 DIM=2 OPEN OBC AXIS 0;";

    // Either the parser rejects it or the executor rejects it at
    // resolve time. We only assert on the executor path since the
    // parser accumulates params generically; the guard lives in the
    // CUBIC arm of resolve_constructor_args.
    let stmt = parser::parse(src).expect("parse should accept the raw grammar");

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");
    gigi::lattice::registry::clear();

    let err = parser::execute(&mut engine, &stmt).expect_err(
        "OPEN + OBC AXIS together must error at execute time",
    );
    let msg = format!("{err}");
    assert!(
        msg.contains("OPEN") && msg.contains("OBC"),
        "error must name both OPEN and OBC to be actionable: {msg}"
    );
}
