//! RECOVER_CURVATURE GROUP SU2 kernel gates (spec: docs/YANG_MILLS_ENGINE_SPEC.md, YM1).
//!
//! Self-contained, on the 600-cell (built here, as in tests/recover_curvature.rs).
//! Parity against the research script on the paper's quotient meshes is a
//! separate harness (e2e/probes/test_recover_curvature_su2_parity.py).
//!
//! G0  an Abelian cochain embedded along one su(2) axis reproduces the U(1) verb
//! G1  gauge covariance: C' = Ad(g at the frame vertex) C
//! M1  removing the register breaks G1 (mechanism removal)
//! M2  the abelianised flux equals the full one on Abelian data (and only there)
//! R   refusals by name: non-unit link, wrong link count, holonomy at -1
#![cfg(feature = "gauge")]

use gigi::gauge::curvature_recovery::{
    recover_curvature, recover_curvature_su2, recover_curvature_su2_with, vertex_ids, Cell,
    Mechanism, RecoveryError, Su2CellRecovery, Su2Flux, Su2Mechanism, EDGES,
};
use nalgebra::Vector3;
use std::collections::HashMap;

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
    out
}

fn dot(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    (0..4).map(|i| a[i] * b[i]).sum()
}

fn six_hundred_cell(rng: &mut Lcg) -> (Vec<Cell>, usize) {
    let v = binary_icosahedral();
    let near = (1.0 + 5f64.sqrt()) / 4.0;
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
    assert_eq!(cells.len(), 600);
    (cells, edge_id.len())
}

// stored-link convention (scripts/ricci_consistency_math.py compose / inverse)
fn lmul(a: &[f64; 4], b: &[f64; 4]) -> [f64; 4] {
    let (av, bv) = (Vector3::new(a[1], a[2], a[3]), Vector3::new(b[1], b[2], b[3]));
    let v = bv * a[0] + av * b[0] - av.cross(&bv);
    [a[0] * b[0] - av.dot(&bv), v[0], v[1], v[2]]
}
fn linv(q: &[f64; 4]) -> [f64; 4] {
    [q[0], -q[1], -q[2], -q[3]]
}
fn qexp(x: Vector3<f64>) -> [f64; 4] {
    let t = x.norm();
    if t < 1e-300 {
        return [1.0, 0.0, 0.0, 0.0];
    }
    let s = t.sin() / t;
    [t.cos(), s * x[0], s * x[1], s * x[2]]
}
fn ad(g: &[f64; 4], x: [f64; 3]) -> [f64; 3] {
    let r = lmul(&lmul(g, &[0.0, x[0], x[1], x[2]]), &linv(g));
    [r[1], r[2], r[3]]
}
fn random_vec(rng: &mut Lcg, scale: f64) -> Vector3<f64> {
    Vector3::from_fn(|_, _| (rng.next() * 2.0 - 1.0) * scale)
}

fn rel_diff(a: &[Su2CellRecovery], b: &[[[f64; 11]; 3]]) -> f64 {
    let (mut num, mut den) = (0.0_f64, 0.0_f64);
    for (x, y) in a.iter().zip(b) {
        for c in 0..3 {
            for k in 0..11 {
                num = num.max((x.coefficients[c][k] - y[c][k]).abs());
                den = den.max(y[c][k].abs());
            }
        }
    }
    num / den
}

/// U' = g_tail U g_head⁻¹ for every stored edge (tail → head as stored).
fn gauge_transform(cells: &[Cell], links: &[[f64; 4]], g: &[[f64; 4]]) -> Vec<[f64; 4]> {
    let vid = vertex_ids(cells);
    let mut out = links.to_vec();
    let mut done = vec![false; links.len()];
    for (ci, c) in cells.iter().enumerate() {
        for (e, &(i, j)) in EDGES.iter().enumerate() {
            let id = c.edges[e];
            if done[id] { continue; }
            done[id] = true;
            let (t, h) = if c.reverse[e] { (j, i) } else { (i, j) };
            out[id] = lmul(&lmul(&g[vid[ci][t]], &links[id]), &linv(&g[vid[ci][h]]));
        }
    }
    out
}

fn smooth_links(n: usize, rng: &mut Lcg) -> Vec<[f64; 4]> {
    (0..n).map(|_| qexp(random_vec(rng, 0.2))).collect()
}

#[test]
fn g0_embedded_u1_reproduces_the_u1_verb() {
    let mut rng = Lcg(11);
    let (cells, ne) = six_hundred_cell(&mut rng);
    let a: Vec<f64> = (0..ne).map(|_| (rng.next() * 2.0 - 1.0) * 0.1).collect();
    let u1 = recover_curvature(&cells, &a, 2).expect("U(1)");
    for axis in 0..3 {
        let links: Vec<[f64; 4]> =
            a.iter().map(|&x| qexp(Vector3::from_fn(|i, _| if i == axis { x } else { 0.0 }))).collect();
        let su = recover_curvature_su2(&cells, &links, 2).expect("SU(2)");
        let want: Vec<[[f64; 11]; 3]> = u1
            .iter()
            .map(|r| std::array::from_fn(|c| if c == axis { r.coefficients } else { [0.0; 11] }))
            .collect();
        let d = rel_diff(&su, &want);
        assert!(d < 1e-12, "axis {axis}: embedded U(1) differs from the U(1) verb by {d:e}");
    }
}

#[test]
fn g1_gauge_covariance_and_m1_register_removal() {
    let mut rng = Lcg(12);
    let (cells, ne) = six_hundred_cell(&mut rng);
    let links = smooth_links(ne, &mut rng);
    let nv = 1 + vertex_ids(&cells).iter().flatten().copied().max().unwrap();
    assert_eq!(nv, 120, "600-cell has 120 vertices");
    let g: Vec<[f64; 4]> = (0..nv).map(|_| qexp(random_vec(&mut rng, 1.0))).collect();
    let moved = gauge_transform(&cells, &links, &g);
    for (register, must_hold) in [(true, true), (false, false)] {
        let su = Su2Mechanism { register, abelianised: false, flux: Su2Flux::Log };
        let r0 = recover_curvature_su2_with(&cells, &links, 2, Mechanism::FULL, su).unwrap();
        let r1 = recover_curvature_su2_with(&cells, &moved, 2, Mechanism::FULL, su).unwrap();
        let want: Vec<[[f64; 11]; 3]> = r0
            .iter()
            .map(|r| {
                let gr = g[r.frame_vertex];
                let mut out = [[0.0; 11]; 3];
                for k in 0..11 {
                    let v = ad(&gr, [r.coefficients[0][k], r.coefficients[1][k], r.coefficients[2][k]]);
                    for c in 0..3 { out[c][k] = v[c]; }
                }
                out
            })
            .collect();
        let d = rel_diff(&r1, &want);
        if must_hold {
            assert!(d < 1e-10, "G1: covariance violated by {d:e}");
        } else {
            assert!(d > 1e-3, "M1: without the register covariance should fail, but the defect is only {d:e}");
        }
    }
}

#[test]
fn m2_abelianised_flux_matches_only_on_abelian_data() {
    let mut rng = Lcg(13);
    let (cells, ne) = six_hundred_cell(&mut rng);
    let abel = Su2Mechanism { register: true, abelianised: true, flux: Su2Flux::Log };
    let a: Vec<f64> = (0..ne).map(|_| (rng.next() * 2.0 - 1.0) * 0.1).collect();
    let links: Vec<[f64; 4]> = a.iter().map(|&x| qexp(Vector3::new(0.0, 0.0, x))).collect();
    let full = recover_curvature_su2(&cells, &links, 2).unwrap();
    let ab = recover_curvature_su2_with(&cells, &links, 2, Mechanism::FULL, abel).unwrap();
    let want: Vec<[[f64; 11]; 3]> = full.iter().map(|r| r.coefficients).collect();
    assert!(rel_diff(&ab, &want) < 1e-12, "on Abelian data the abelianised flux must agree");
    let links = smooth_links(ne, &mut rng);
    let full = recover_curvature_su2(&cells, &links, 2).unwrap();
    let ab = recover_curvature_su2_with(&cells, &links, 2, Mechanism::FULL, abel).unwrap();
    let want: Vec<[[f64; 11]; 3]> = full.iter().map(|r| r.coefficients).collect();
    assert!(rel_diff(&ab, &want) > 1e-3, "on non-commuting data the abelianised flux must differ");
}

#[test]
fn r_refusals_by_name() {
    let mut rng = Lcg(14);
    let (cells, ne) = six_hundred_cell(&mut rng);
    let mut links = smooth_links(ne, &mut rng);
    links[5] = [1.1, 0.0, 0.0, 0.0];
    assert!(matches!(recover_curvature_su2(&cells, &links, 2), Err(RecoveryError::NonUnitLink { edge: 5, .. })));
    let short = smooth_links(ne - 1, &mut rng);
    assert!(matches!(recover_curvature_su2(&cells, &short, 2), Err(RecoveryError::EdgeOutOfRange { .. })));
    let long = smooth_links(ne + 1, &mut rng);
    assert!(matches!(recover_curvature_su2(&cells, &long, 2), Err(RecoveryError::LinkCountMismatch { .. })));
    let mut flat = vec![[1.0, 0.0, 0.0, 0.0]; ne];
    flat[cells[0].edges[0]] = [-1.0, 0.0, 0.0, 0.0];
    assert!(matches!(recover_curvature_su2(&cells, &flat, 2), Err(RecoveryError::HolonomyNearMinusOne { .. })));
    assert!(matches!(recover_curvature_su2(&cells, &flat, 0), Err(RecoveryError::InvalidRings(0))));
}
