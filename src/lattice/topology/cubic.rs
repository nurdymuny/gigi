//! D-dimensional cubic lattice T^D constructor — an `L^D` periodic grid
//! returned as a [`LatticeWithMetric`]. The Halcyon §3.3 deliverable
//! (4D pure-gauge target: L=12, D=4 → V=20736, E=82944, F=124416).
//!
//! Combinatorics (closed `D`-torus, PERIODIC, `L >= 1`, `D >= 1`):
//!
//! - `V = L^D` vertices, indexed row-major over the coordinate tuple
//!   `(c_0, c_1, …, c_{D-1})` with `c_k ∈ 0..L`:
//!   `v(c) = sum_k c_k · L^k`.
//! - `E = L^D · D` directed edges. The edge set is laid out
//!   axis-major then site-major: axis `a ∈ 0..D` contributes one
//!   contiguous block of `L^D` edges in site-id order; within axis
//!   `a` the edge at site `s` is `(s, s + ê_a)` where `s + ê_a`
//!   increments `c_a` modulo `L`. (Each site therefore has exactly
//!   `D` outgoing edges and `D` incoming edges under periodicity,
//!   for a total undirected vertex degree of `2·D`.)
//! - `F = L^D · D·(D-1)/2` square plaquettes (2-cells). The face set
//!   is `(axis_a, axis_b)`-major (lexicographic over the C(D, 2)
//!   pairs `a < b`), then site-major. For each pair `(a, b)` the
//!   face at site `s` is the 4-cycle `[s, s + ê_a, s + ê_a + ê_b,
//!   s + ê_b]` traversed counter-clockwise in the `(a, b)` plane.
//! - Euler check (`D = 2` only in Phase 1 — Phase 2 owns the higher
//!   3-cell / 4-cell bookkeeping that closes χ on `D ≥ 3`):
//!   `D=2`: `V − E + F = L² − 2L² + L² = 0 = χ(T²)` ✓.
//!
//! Locked combinatorics from `GIGI_TO_HALCYON_REPLY_2026-06-26_BRIDGE_REVISED.md`
//! §3.3:
//!
//!     L = 12, D = 4, PERIODIC
//!     V = 12^4 = 20_736
//!     E = 12^4 · 4 = 82_944
//!     F = 12^4 · C(4,2) = 12^4 · 6 = 124_416
//!
//! Metric: unit-cube cells. Every edge length is `1.0`; every face
//! area is `1.0`. Dual face areas are left `None` (Phase 1 — Phase 2
//! `lattice::dec` owns the dual mesh for arbitrary `D`).
//!
//! Topology hint convention: `"CUBIC_L{L}_D{D}"` for periodic
//! (default), `"CUBIC_L{L}_D{D}_OPEN"` for fully-open (deferred to
//! Phase 2), and `"CUBIC_L{L}_D{D}_OBC_AXIS{k}"` for single-axis
//! open-boundary — periodic on all axes EXCEPT axis `k`. The "CUBIC"
//! prefix is registered in [`super::hints`]; the full per-(L,D) hint
//! is generated at construction time and stored on the [`Lattice`].
//!
//! Phase 1 scope: PERIODIC + single-axis OBC (Hallie's SU(2) 4D L=24
//! β=2.3 OBC sectoral SPECTRAL_GAUGE workflow). Fully-open boundary
//! conditions (`periodic = false` with no OBC axis) are still deferred
//! to Phase 2 (assertion in the constructor enforces this).

use crate::lattice::metric::LatticeWithMetric;
use crate::lattice::{Lattice, VertexId};

/// Construct an `L^D` D-dimensional cubic lattice with unit-cube
/// metric. Phase 1 supports PERIODIC (default) and single-axis OBC
/// (via `obc_axis = Some(k)`); fully-open (`periodic = false` with
/// `obc_axis = None`) still panics with the deferred-to-Phase-2
/// message.
///
/// Combinatorics (PERIODIC, `obc_axis = None`):
///
/// - `V = L^D`
/// - `E = L^D · D` (directed; each site has `D` outgoing edges, one
///   per axis)
/// - `F = L^D · D·(D-1)/2` (one square plaquette per (site, axis-pair))
///
/// Combinatorics (single-axis OBC, `obc_axis = Some(k)` with
/// `k ∈ 0..D`) — periodic on every axis EXCEPT axis `k`:
///
/// - `V = L^D` (open BC keeps sites; only wrap connectivity is
///   removed)
/// - `E = L^D · D − L^(D-1)` (drops one wrap edge per axis-k
///   boundary site — the `L^(D-1)` sites at coordinate `c_k = L-1`)
/// - `F = L^D · D·(D-1)/2 − (D-1) · L^(D-1)` (drops one boundary
///   plaquette per axis-pair `(a, b)` that touches axis `k`, at every
///   axis-k boundary site — there are `D-1` such pairs)
///
/// Examples:
///
/// - `L=4, D=2, OBC AXIS 0`: `V=16`, `E=32-4=28`, `F=16-4=12`.
/// - `L=24, D=4, OBC AXIS 0`: `V=331776`, `E=24^4·4 − 24^3`, and
///   `F=24^4·6 − 3·24^3`. This is Hallie's Halcyon workflow substrate.
///
/// Panics if `l < 1`, `d < 1`, `obc_axis = Some(k)` with `k >= d`,
/// or (`periodic == false` AND `obc_axis == None`).
pub fn cubic(
    name: &str,
    l: usize,
    d: usize,
    periodic: bool,
    obc_axis: Option<usize>,
) -> LatticeWithMetric {
    assert!(l >= 1, "cubic: L must be >= 1 (got {l})");
    assert!(d >= 1, "cubic: D must be >= 1 (got {d})");
    // Validate obc_axis against dimension before we do anything else.
    if let Some(k) = obc_axis {
        assert!(
            k < d,
            "cubic: OBC AXIS {k} out of range for DIM={d} (must be 0..{d})"
        );
    }
    // Fully-open boundary (periodic = false AND no OBC axis named)
    // stays deferred to Phase 2; single-axis OBC is the Phase 1 path.
    assert!(
        periodic || obc_axis.is_some(),
        "cubic: OPEN boundary deferred to Phase 2 (got periodic = false, obc_axis = None). \
         Phase 1 ships PERIODIC + single-axis OBC — pass obc_axis = Some(k) instead."
    );

    // `n_vertices = L^D`. Compute as usize.pow(d as u32) once; we'll
    // reuse it for capacity hints and assertions below.
    let n_vertices: usize = (0..d).fold(1usize, |acc, _| acc * l);

    // Row-major site indexing: v(c_0, c_1, …, c_{D-1}) = sum_k c_k · L^k.
    // Stride is L^k for axis k; precompute to avoid repeated pow() calls.
    let mut stride: Vec<usize> = vec![1usize; d];
    for k in 1..d {
        stride[k] = stride[k - 1] * l;
    }

    // Decompose a site id back into its coordinate tuple.
    let coords_of = |mut s: usize| -> Vec<usize> {
        let mut c = vec![0usize; d];
        for k in 0..d {
            c[k] = s % l;
            s /= l;
        }
        c
    };

    // Encode a coordinate tuple back to a site id.
    let site_of = |c: &[usize]| -> VertexId {
        let mut s = 0usize;
        for (k, &ck) in c.iter().enumerate() {
            s += ck * stride[k];
        }
        s
    };

    // Shift coord c by +1 along axis `a`, modulo L (periodic wrap).
    let shift_plus = |c: &[usize], a: usize| -> Vec<usize> {
        let mut c2 = c.to_vec();
        c2[a] = (c2[a] + 1) % l;
        c2
    };

    // ── Edges: axis-major then site-major ────────────────────────────
    //
    // For axis a ∈ 0..D, push L^D edges (s → s + ê_a) in site id
    // order. Total directed edges = L^D · D on PERIODIC.
    //
    // OBC AXIS k skip: when a == k AND c[k] == L-1, the (s → s+ê_a)
    // edge would wrap through the open boundary; drop it. That drops
    // exactly L^(D-1) wrap edges — one per axis-k boundary site.
    let mut edges: Vec<(VertexId, VertexId)> = Vec::with_capacity(n_vertices * d);
    for a in 0..d {
        for s in 0..n_vertices {
            let c = coords_of(s);
            if let Some(k) = obc_axis {
                if a == k && c[k] == l - 1 {
                    continue;
                }
            }
            let c2 = shift_plus(&c, a);
            edges.push((s, site_of(&c2)));
        }
    }
    // Sanity in the PERIODIC case only — the OBC path removes an
    // exact-known count of wrap edges enforced by the asserts below.
    #[cfg(debug_assertions)]
    if obc_axis.is_none() {
        debug_assert_eq!(edges.len(), n_vertices * d);
    }

    // ── Faces: (a, b)-major (lex over pairs a < b) then site-major ──
    //
    // C(D, 2) = D·(D-1)/2 pairs of distinct axes. For each pair (a, b)
    // and each site s, emit the 4-cycle [s, s+ê_a, s+ê_a+ê_b, s+ê_b]
    // counter-clockwise in the (a, b) plane.
    //
    // OBC AXIS k skip: when either axis in the pair is the open axis
    // AND the anchor coordinate for that axis is L-1, this plaquette
    // would wrap through the open boundary; drop it. That drops one
    // boundary plaquette per axis-pair (a, b) that touches axis k, at
    // every axis-k boundary site — (D-1) · L^(D-1) faces total.
    let n_pairs = d * d.saturating_sub(1) / 2;
    let n_faces_periodic = n_vertices * n_pairs;
    let mut faces: Vec<Vec<VertexId>> = Vec::with_capacity(n_faces_periodic);
    for a in 0..d {
        for b in (a + 1)..d {
            for s in 0..n_vertices {
                let c = coords_of(s);
                if let Some(k) = obc_axis {
                    if (a == k && c[a] == l - 1) || (b == k && c[b] == l - 1) {
                        continue;
                    }
                }
                let c_a = shift_plus(&c, a);
                let mut c_ab = c_a.clone();
                c_ab[b] = (c_ab[b] + 1) % l;
                let c_b = shift_plus(&c, b);
                faces.push(vec![s, site_of(&c_a), site_of(&c_ab), site_of(&c_b)]);
            }
        }
    }
    #[cfg(debug_assertions)]
    if obc_axis.is_none() {
        debug_assert_eq!(faces.len(), n_faces_periodic);
    }

    let topology = match obc_axis {
        // PERIODIC (Phase 1 default).
        None => format!("CUBIC_L{l}_D{d}"),
        // Single-axis OBC — carry the axis index so downstream verbs
        // (BETTI, CHERN_CLASS, SPECTRAL_GAUGE) can dispatch on it.
        Some(k) => format!("CUBIC_L{l}_D{d}_OBC_AXIS{k}"),
    };

    let n_edges_actual = edges.len();
    let n_faces_actual = faces.len();

    let lattice = Lattice::new(
        name.to_string(),
        n_vertices,
        edges,
        faces,
        Some(topology),
    );

    // Trivial unit-cube metric: every edge length and every face area
    // is `1.0`. Dual face areas left `None` (Phase 1 — Phase 2
    // `lattice::dec` owns the dual mesh). Vector sizes track the
    // actual (possibly OBC-reduced) edge / face counts.
    let cell_areas = vec![1.0; n_faces_actual];
    let edge_lengths = vec![1.0; n_edges_actual];

    LatticeWithMetric::from_lattice_and_metric(lattice, cell_areas, edge_lengths, None)
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// D=2, L=4 matches flat-torus-2d combinatorics exactly:
    /// V=16, E=32, F=16, χ(T²)=0. The cubic constructor is the
    /// generalization; flat_torus_2d(n) is `cubic("name", n, 2, true, None)`
    /// modulo edge-ordering convention.
    #[test]
    fn test_d2_l4_matches_flat_torus_combinatorics() {
        let lwm = cubic("c2_4", 4, 2, true, None);
        let lat = lwm.lattice();

        assert_eq!(lat.n_vertices, 16, "V = 4² = 16");
        assert_eq!(lat.n_edges(), 32, "E = 4² · 2 = 32");
        assert_eq!(lat.n_faces(), 16, "F = 4² · C(2,2) = 4² · 1 = 16");
        assert_eq!(lat.euler_characteristic(), 0, "χ(T²) = V − E + F = 0");
        assert_eq!(lat.topology.as_deref(), Some("CUBIC_L4_D2"));

        // Every vertex has degree 2·D = 4 on the closed 2-torus.
        let mut degree = vec![0usize; lat.n_vertices];
        for &(a, b) in &lat.edges {
            degree[a] += 1;
            degree[b] += 1;
        }
        for (vid, deg) in degree.iter().enumerate() {
            assert_eq!(*deg, 4, "vertex {vid} should have degree 2D=4");
        }

        // Metric: unit-cube cells.
        assert_eq!(lwm.cell_areas().len(), lat.n_faces());
        assert_eq!(lwm.edge_lengths().len(), lat.n_edges());
        assert!(lwm.dual_face_areas().is_none());
        for &a in lwm.cell_areas() {
            assert!((a - 1.0).abs() < 1e-12);
        }
        for &l in lwm.edge_lengths() {
            assert!((l - 1.0).abs() < 1e-12);
        }

        // Every face is a quad.
        for face in &lat.faces {
            assert_eq!(face.len(), 4);
        }
    }

    /// D=3, L=4 combinatorics: V=64, E=192, F=192. Each vertex has
    /// degree 2·D=6 (six nearest neighbours on the cubic 3-torus).
    #[test]
    fn test_d3_l4_combinatorics() {
        let lwm = cubic("c3_4", 4, 3, true, None);
        let lat = lwm.lattice();

        assert_eq!(lat.n_vertices, 64, "V = 4³ = 64");
        assert_eq!(lat.n_edges(), 192, "E = 4³ · 3 = 192");
        assert_eq!(lat.n_faces(), 192, "F = 4³ · C(3,2) = 4³ · 3 = 192");
        assert_eq!(lat.topology.as_deref(), Some("CUBIC_L4_D3"));

        // Every vertex has degree 2·D=6 on the closed 3-torus.
        let mut degree = vec![0usize; lat.n_vertices];
        for &(a, b) in &lat.edges {
            degree[a] += 1;
            degree[b] += 1;
        }
        for (vid, deg) in degree.iter().enumerate() {
            assert_eq!(*deg, 6, "vertex {vid} should have degree 2D=6");
        }

        // Every face is a quad.
        for face in &lat.faces {
            assert_eq!(face.len(), 4);
        }
    }

    /// D=4, L=12 — the Halcyon §3.3 locked dimensions.
    /// V = 12^4 = 20_736
    /// E = 12^4 · 4 = 82_944
    /// F = 12^4 · C(4,2) = 12^4 · 6 = 124_416
    /// Every vertex has degree 2·D=8.
    #[test]
    fn test_d4_l12_combinatorics() {
        let lwm = cubic("c4_12", 12, 4, true, None);
        let lat = lwm.lattice();

        assert_eq!(lat.n_vertices, 20_736, "V = 12^4 = 20736");
        assert_eq!(lat.n_edges(), 82_944, "E = 12^4 · 4 = 82944");
        assert_eq!(lat.n_faces(), 124_416, "F = 12^4 · 6 = 124416");
        assert_eq!(lat.topology.as_deref(), Some("CUBIC_L12_D4"));

        // Every vertex has degree 2·D=8 on the closed 4-torus.
        let mut degree = vec![0usize; lat.n_vertices];
        for &(a, b) in &lat.edges {
            degree[a] += 1;
            degree[b] += 1;
        }
        for (vid, deg) in degree.iter().enumerate() {
            assert_eq!(*deg, 8, "vertex {vid} should have degree 2D=8");
        }

        // Every face is a quad.
        for face in &lat.faces {
            assert_eq!(face.len(), 4);
        }

        // Metric: unit-cube cells, vectors sized to the (V, E, F) counts.
        assert_eq!(lwm.cell_areas().len(), 124_416);
        assert_eq!(lwm.edge_lengths().len(), 82_944);
    }

    /// Periodic-uniform-degree invariant: every vertex on an `L^D`
    /// periodic cubic has undirected degree `2·D` regardless of `L`
    /// or `D` (modulo `L=1` degeneracies that self-loop, which we
    /// exclude with `L >= 2`).
    #[test]
    fn test_vertex_degree_uniform_for_periodic() {
        for &(l, d) in &[(2usize, 2usize), (3, 2), (4, 3), (3, 4), (5, 3)] {
            let lwm = cubic("c", l, d, true, None);
            let lat = lwm.lattice();
            let mut degree = vec![0usize; lat.n_vertices];
            for &(a, b) in &lat.edges {
                degree[a] += 1;
                degree[b] += 1;
            }
            for (vid, deg) in degree.iter().enumerate() {
                assert_eq!(
                    *deg,
                    2 * d,
                    "L={l} D={d}: vertex {vid} should have degree 2D={}, got {deg}",
                    2 * d
                );
            }
        }
    }

    /// Phase 1 scope: fully-open boundary (`periodic = false` AND
    /// `obc_axis = None`) still panics with the deferred-to-Phase-2
    /// message. Phase 2 will implement the fully-open path; until then
    /// this assertion keeps the contract honest. Single-axis OBC now
    /// SHIPS (Phase 1 addition for Hallie's sectoral SPECTRAL_GAUGE
    /// workflow) — see `tests/lattice_obc_basic.rs` for that path.
    #[test]
    #[should_panic(expected = "OPEN boundary deferred to Phase 2")]
    fn test_open_boundary_not_yet_supported() {
        let _ = cubic("open_cube", 4, 3, false, None);
    }
}
