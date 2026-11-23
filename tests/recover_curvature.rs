//! RECOVER_CURVATURE kernel gates (spec: docs/RECOVER_CURVATURE_SPEC.md).
//!
//! Fixture: the 600-cell, a closed tetrahedral mesh of S³ whose 120 vertices
//! are the binary icosahedral group. Built here from first principles, so the
//! gates need no external files. Parity against the paper's frozen quotient
//! meshes is a separate harness (e2e/probes/test_recover_curvature_parity.py).
#![cfg(feature = "gauge")]

use gigi::gauge::curvature_recovery::{
    build_face_index, recover_cell, recover_curvature, recover_curvature_with, Cell, Mechanism,
    RecoveryError, EDGES,
};
use nalgebra::{Matrix3, Vector3, Vector4};
use std::collections::HashMap;

// ── deterministic randomness ─────────────────────────────────────────────

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() * n as f64) as usize % n
    }
}

// ── the 600-cell ─────────────────────────────────────────────────────────

fn binary_icosahedral() -> Vec<[f64; 4]> {
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let mut out = Vec::new();
    for i in 0..4 {
        for s in [1.0, -1.0] {
            let mut q = [0.0; 4];
            q[i] = s;
            out.push(q);
        }
    }
    for m in 0..16 {
        out.push(std::array::from_fn(|i| if m >> i & 1 == 1 { -0.5 } else { 0.5 }));
    }
    let base = [0.0, 0.5, phi / 2.0, 1.0 / (2.0 * phi)];
    let even = [
        [0, 1, 2, 3], [0, 2, 3, 1], [0, 3, 1, 2], [1, 0, 3, 2], [1, 2, 0, 3], [1, 3, 2, 0],
        [2, 0, 1, 3], [2, 1, 3, 0], [2, 3, 0, 1], [3, 0, 2, 1], [3, 1, 0, 2], [3, 2, 1, 0],
    ];
    for p in even {
        for m in 0..8 {
            let signed = [
                base[0],
                if m & 1 == 1 { -base[1] } else { base[1] },
                if m & 2 == 2 { -base[2] } else { base[2] },
                if m & 4 == 4 { -base[3] } else { base[3] },
            ];
            out.push(std::array::from_fn(|i| signed[p[i]]));
        }
    }
    assert_eq!(out.len(), 120);
    out
}

struct Mesh {
    cells: Vec<Cell>,
    n_edges: usize,
}

fn dot(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    (0..4).map(|i| a[i] * b[i]).sum()
}

/// The 600-cell. Each cell's local vertex order is shuffled, so `reverse`
/// flags are exercised.
fn six_hundred_cell(rng: &mut Lcg) -> Mesh {
    let v = binary_icosahedral();
    let near = (1.0 + 5f64.sqrt()) / 4.0; // cos 36°
    let adj = |i: usize, j: usize| (dot(&v[i], &v[j]) - near).abs() < 1e-9;
    let mut edge_id: HashMap<(usize, usize), usize> = HashMap::new();
    for i in 0..120 {
        for j in (i + 1)..120 {
            if adj(i, j) {
                let n = edge_id.len();
                edge_id.insert((i, j), n);
            }
        }
    }
    let mut cells = Vec::new();
    for a in 0..120 {
        for b in (a + 1)..120 {
            if !adj(a, b) { continue; }
            for c in (b + 1)..120 {
                if !(adj(a, c) && adj(b, c)) { continue; }
                for d in (c + 1)..120 {
                    if !(adj(a, d) && adj(b, d) && adj(c, d)) { continue; }
                    let mut order = [a, b, c, d];
                    for i in (1..4).rev() {
                        order.swap(i, rng.below(i + 1));
                    }
                    let edges = EDGES.map(|(i, j)| edge_id[&(order[i].min(order[j]), order[i].max(order[j]))]);
                    let reverse = EDGES.map(|(i, j)| order[i] > order[j]);
                    cells.push(Cell { vertices: order.map(|k| v[k]), edges, reverse });
                }
            }
        }
    }
    assert_eq!(cells.len(), 600, "600-cell has 600 tetrahedra");
    assert_eq!(edge_id.len(), 720, "600-cell has 720 edges");
    Mesh { cells, n_edges: edge_id.len() }
}

fn random_cochain(n: usize, rng: &mut Lcg) -> Vec<f64> {
    (0..n).map(|_| rng.next() * 2.0 - 1.0).collect()
}

fn qmul(p: &[f64; 4], q: &[f64; 4]) -> [f64; 4] {
    let (p0, pv) = (p[0], Vector3::new(p[1], p[2], p[3]));
    let (q0, qv) = (q[0], Vector3::new(q[1], q[2], q[3]));
    let v = qv * p0 + pv * q0 + pv.cross(&qv);
    [p0 * q0 - pv.dot(&qv), v[0], v[1], v[2]]
}

fn random_unit(rng: &mut Lcg) -> [f64; 4] {
    let q: [f64; 4] = std::array::from_fn(|_| rng.next() * 2.0 - 1.0);
    let n = dot(&q, &q).sqrt();
    q.map(|x| x / n)
}

/// Store every cell on its own random sheet: left-multiply its vertices by a
/// random unit quaternion. Geometry and combinatorics are unchanged; only the
/// transport can bring neighbours back into register.
fn resheet(mesh: &Mesh, rng: &mut Lcg) -> Vec<Cell> {
    mesh.cells
        .iter()
        .map(|c| {
            let g = random_unit(rng);
            Cell { vertices: c.vertices.map(|v| qmul(&g, &v)), ..c.clone() }
        })
        .collect()
}

fn max_coef_diff(a: &[gigi::gauge::curvature_recovery::CellRecovery],
                 b: &[gigi::gauge::curvature_recovery::CellRecovery]) -> f64 {
    a.iter()
        .zip(b)
        .flat_map(|(x, y)| x.coefficients.iter().zip(y.coefficients.iter()).map(|(p, q)| (p - q).abs()))
        .fold(0.0, f64::max)
}

// ── T1 exactness on planted closed linear fields ────────────────────────

fn planted(rng: &mut Lcg) -> (Vector3<f64>, Matrix3<f64>) {
    let c = Vector3::from_fn(|_, _| rng.next() * 2.0 - 1.0);
    let mut b = Matrix3::from_fn(|_, _| rng.next() * 2.0 - 1.0);
    let t = b.trace() / 3.0;
    b -= Matrix3::identity() * t;
    (c, b)
}

#[test]
fn t1_planted_closed_linear_field_is_recovered_exactly() {
    let mut rng = Lcg(7);
    let mesh = six_hundred_cell(&mut rng);
    let faces = build_face_index(&mesh.cells).unwrap();
    let a = vec![0.0; mesh.n_edges];
    for rings in [1, 2] {
        for ci in [0, 17, 233, 599] {
            let (c, b) = planted(&mut rng);
            let field = move |tri: [Vector3<f64>; 3]| {
                let area = (tri[1] - tri[0]).cross(&(tri[2] - tri[0])) / 2.0;
                let cen = (tri[0] + tri[1] + tri[2]) / 3.0;
                (c + b * cen).dot(&area)
            };
            let r = recover_cell(&mesh.cells, &faces, &a, ci, rings, Mechanism::FULL, Some(&field)).unwrap();
            let err_c = (0..3).map(|i| (r.c[i] - c[i]).abs()).fold(0.0, f64::max);
            let err_b = (0..9).map(|k| (r.b[k / 3][k % 3] - b[(k / 3, k % 3)]).abs()).fold(0.0, f64::max);
            assert!(err_c < 1e-10 && err_b < 1e-10,
                    "rings {rings} cell {ci}: planted field not recovered (c {err_c:.2e}, B {err_b:.2e})");
        }
    }
}

// ── T2 one ring is exactly determined; two rings oversample ──────────────

#[test]
fn t2_one_ring_exactly_determined_two_rings_oversampled() {
    let mut rng = Lcg(11);
    let mesh = six_hundred_cell(&mut rng);
    let a = random_cochain(mesh.n_edges, &mut rng);
    let one = recover_curvature(&mesh.cells, &a, 1).unwrap();
    let two = recover_curvature(&mesh.cells, &a, 2).unwrap();
    for (r1, r2) in one.iter().zip(&two) {
        assert_eq!(r1.faces, 16, "one-ring star must read exactly 16 faces");
        assert!(r1.residual < 1e-9, "exactly determined fit must interpolate (residual {:.2e})", r1.residual);
        assert!(r2.faces > 16, "two rings must read more faces than the star");
        assert!(r2.residual > 1e-6, "rough data cannot be interpolated by an overdetermined fit");
        assert!(r2.amplification < r1.amplification,
                "oversampling must lower amplification ({} vs {})", r2.amplification, r1.amplification);
        assert_eq!(r1.rank, 11);
    }
}

// ── T3 own faces are held exactly on rough data ──────────────────────────

#[test]
fn t3_equality_constraint_holds_own_faces() {
    let mut rng = Lcg(13);
    let mesh = six_hundred_cell(&mut rng);
    let a = random_cochain(mesh.n_edges, &mut rng);
    let out = recover_curvature_with(&mesh.cells, &a, 2, Mechanism::FULL).unwrap();
    let worst = out.iter().map(|r| r.own_residual).fold(0.0, f64::max);
    let total = out.iter().map(|r| r.residual).fold(0.0, f64::max);
    assert!(worst < 1e-11, "own faces must be held exactly (worst {worst:.2e})");
    assert!(total > 1e-6, "the rest must carry the misfit on rough data (worst {total:.2e})");
}

// ── T4 transport: re-sheeting changes nothing, and is required ───────────

#[test]
fn t4_resheeted_mesh_gives_identical_recovery() {
    let mut rng = Lcg(17);
    let mesh = six_hundred_cell(&mut rng);
    let a = random_cochain(mesh.n_edges, &mut rng);
    let base = recover_curvature(&mesh.cells, &a, 2).unwrap();
    let moved = resheet(&mesh, &mut rng);
    let again = recover_curvature(&moved, &a, 2).unwrap();
    let d = max_coef_diff(&base, &again);
    assert!(d < 1e-9, "re-sheeting must not change the recovery (max coefficient diff {d:.2e})");
}

// ── T5 refusals ──────────────────────────────────────────────────────────

#[test]
fn t5_refusals_fire_by_name() {
    let mut rng = Lcg(19);
    let mesh = six_hundred_cell(&mut rng);
    let a = random_cochain(mesh.n_edges, &mut rng);

    assert_eq!(recover_curvature(&mesh.cells, &a, 0).unwrap_err(), RecoveryError::InvalidRings(0));
    assert_eq!(recover_curvature(&mesh.cells, &a, 4).unwrap_err(), RecoveryError::InvalidRings(4));

    let open = &mesh.cells[1..];
    assert!(matches!(recover_curvature(open, &a, 2).unwrap_err(), RecoveryError::OpenMesh { .. }));

    let short = &a[..a.len() - 1];
    assert!(matches!(recover_curvature(&mesh.cells, short, 2).unwrap_err(),
                     RecoveryError::EdgeOutOfRange { .. }));

    let mut bent = mesh.cells.clone();
    bent[5].vertices[0][1] += 1e-6;
    assert!(matches!(recover_curvature(&bent, &a, 2).unwrap_err(), RecoveryError::NoCoincidence { .. }));

    let mut flat = mesh.cells.clone();
    flat[0].vertices[3] = flat[0].vertices[2];
    let err = recover_curvature(&flat, &a, 1).unwrap_err();
    assert!(matches!(err, RecoveryError::DegenerateCell { cell: 0 } | RecoveryError::NoCoincidence { .. }),
            "a collapsed cell must be refused, got {err:?}");
}

// ── Mechanism removals: each must turn a gate red ────────────────────────

#[test]
fn m1_without_transport_resheeted_mesh_is_refused() {
    let mut rng = Lcg(23);
    let mesh = six_hundred_cell(&mut rng);
    let a = random_cochain(mesh.n_edges, &mut rng);
    let moved = resheet(&mesh, &mut rng);
    let off = Mechanism { transport: false, ..Mechanism::FULL };
    assert!(matches!(recover_curvature_with(&moved, &a, 2, off).unwrap_err(),
                     RecoveryError::NoCoincidence { .. }),
            "without transport, neighbours on other sheets must fail to coincide");
}

#[test]
fn m2_without_equality_own_faces_are_not_held() {
    let mut rng = Lcg(29);
    let mesh = six_hundred_cell(&mut rng);
    let a = random_cochain(mesh.n_edges, &mut rng);
    let off = Mechanism { equality: false, ..Mechanism::FULL };
    let out = recover_curvature_with(&mesh.cells, &a, 2, off).unwrap();
    let worst = out.iter().map(|r| r.own_residual).fold(0.0, f64::max);
    assert!(worst > 1e-6,
            "with the equality constraint removed, T3's gate must go red (own residual {worst:.2e})");
}
