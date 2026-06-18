//! Truncated-icosahedron (buckyball) constructor.
//!
//! Closes TDD-HAL-I.2 and is the indexing dependency of
//! TDD-HAL-I.6. Faithful port of
//! `davis-wilson-lattice/inertia_damping/buckyball_graph.py::
//! build_truncated_icosahedron` so the substrate's edge / face
//! indexing matches the Halcyon reference bit-identically. The U_final
//! gold fixture was committed against that indexing; without the
//! same construction order, the gold gate (I.6) cannot bit-match
//! per-face.
//!
//! Construction:
//!
//! 1. Build 60 vertex coordinates from the 3-orbit recipe (orbits
//!    A / B / C with the cyclic-shift pattern from
//!    `_build_vertices`).
//! 2. Enumerate edges as `(i, j)` pairs with `i < j` and
//!    `‖V[i] − V[j]‖² = 4.0` exactly (the truncated-icosahedron
//!    nearest-neighbour squared distance under this normalization).
//!    Storage order is the natural `i < j` lexicographic order from
//!    the double loop.
//! 3. Build the rotation system: for each vertex, sort its 3
//!    neighbours by `atan2` angle in the local tangent plane
//!    (`e1 = normalize(V[adj[0]] − V_v − proj)`, `e2 = nv × e1`).
//! 4. Trace faces via the combinatorial-map `next_in_face` rule.
//! 5. Orient each face outward; emit pentagons first, then
//!    hexagons.
//!
//! Output: V = 60, E = 90, F = 32 (12 pentagons + 20 hexagons),
//! Euler χ = 2, topology `"S2"`. Face storage uses signed edges —
//! `face_signed_edges()` returns the `(edge_index, sign)` cycle
//! per face that matches Halcyon's `graph.faces[f]`.

use super::lattice::{EdgeOrientation, Lattice, VertexId};

/// Golden ratio. `phi = (1 + √5) / 2`.
fn phi() -> f64 {
    (1.0 + 5.0_f64.sqrt()) / 2.0
}

const EDGE_LEN_SQ: f64 = 4.0;
const TOL_DIST: f64 = 1e-9;

/// Build the 60 buckyball vertex coordinates in the order Halcyon
/// commits to. Direct port of
/// `buckyball_graph.py::_build_vertices` — orbits A (12), B (24),
/// C (24); within each orbit the sign loops are `s1, s2[, s3]` and
/// the inner `shift in 0..3` rotates the base tuple.
fn build_vertices() -> Vec<[f64; 3]> {
    let p = phi();
    let mut verts: Vec<[f64; 3]> = Vec::with_capacity(60);

    // Orbit A: (0, ±1, ±3φ) cyclic perms. 4 × 3 = 12 vertices.
    for s1 in [1.0_f64, -1.0] {
        for s2 in [1.0_f64, -1.0] {
            let base = [0.0_f64, s1 * 1.0, s2 * 3.0 * p];
            for shift in 0..3 {
                let v = [
                    base[(0_isize - shift as isize).rem_euclid(3) as usize],
                    base[(1_isize - shift as isize).rem_euclid(3) as usize],
                    base[(2_isize - shift as isize).rem_euclid(3) as usize],
                ];
                verts.push(v);
            }
        }
    }
    // Orbit B: (±1, ±(2+φ), ±2φ) cyclic perms. 8 × 3 = 24 vertices.
    for s1 in [1.0_f64, -1.0] {
        for s2 in [1.0_f64, -1.0] {
            for s3 in [1.0_f64, -1.0] {
                let base = [s1 * 1.0, s2 * (2.0 + p), s3 * 2.0 * p];
                for shift in 0..3 {
                    let v = [
                        base[(0_isize - shift as isize).rem_euclid(3) as usize],
                        base[(1_isize - shift as isize).rem_euclid(3) as usize],
                        base[(2_isize - shift as isize).rem_euclid(3) as usize],
                    ];
                    verts.push(v);
                }
            }
        }
    }
    // Orbit C: (±φ, ±2, ±(2φ+1)) cyclic perms. 8 × 3 = 24 vertices.
    for s1 in [1.0_f64, -1.0] {
        for s2 in [1.0_f64, -1.0] {
            for s3 in [1.0_f64, -1.0] {
                let base = [s1 * p, s2 * 2.0, s3 * (2.0 * p + 1.0)];
                for shift in 0..3 {
                    let v = [
                        base[(0_isize - shift as isize).rem_euclid(3) as usize],
                        base[(1_isize - shift as isize).rem_euclid(3) as usize],
                        base[(2_isize - shift as isize).rem_euclid(3) as usize],
                    ];
                    verts.push(v);
                }
            }
        }
    }
    assert_eq!(verts.len(), 60);
    verts
}

fn dist_sq(a: &[f64; 3], b: &[f64; 3]) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

/// Build edges + adjacency. Mirrors
/// `buckyball_graph.py::_build_edges`. Returns
/// `(pairs, adj, edge_index)`:
///
/// - `pairs[k]` = `(i, j)` with `i < j`, in the i-major
///   lexicographic enumeration order of the Python kernel.
/// - `adj[v]` = sorted list of neighbours of v.
/// - `edge_index` maps `(i, j)` (i < j) → k.
fn build_edges(v: &[[f64; 3]]) -> (Vec<(usize, usize)>, Vec<Vec<usize>>, Vec<Vec<i32>>) {
    let n = v.len();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            let d2 = dist_sq(&v[i], &v[j]);
            if (d2 - EDGE_LEN_SQ).abs() < TOL_DIST {
                pairs.push((i, j));
            }
        }
    }
    assert_eq!(pairs.len(), 90, "expected 90 buckyball edges, got {}", pairs.len());
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(i, j) in &pairs {
        adj[i].push(j);
        adj[j].push(i);
    }
    for av in adj.iter_mut() {
        av.sort();
        assert_eq!(av.len(), 3);
    }
    // edge_index[i][j] = k for ordered (i, j) — we use a 2-D map for
    // O(1) lookup. Allocates 60·60 i32 but it's tiny.
    let mut edge_index: Vec<Vec<i32>> = vec![vec![-1_i32; n]; n];
    for (k, &(i, j)) in pairs.iter().enumerate() {
        edge_index[i][j] = k as i32;
    }
    (pairs, adj, edge_index)
}

/// Per-vertex rotation system. For vertex v with 3 neighbours,
/// return them in CCW order around the local tangent plane.
/// Mirrors `_build_rotation_system`.
fn build_rotation_system(v: &[[f64; 3]], adj: &[Vec<usize>]) -> Vec<[usize; 3]> {
    let n = v.len();
    let mut rot: Vec<[usize; 3]> = vec![[0; 3]; n];
    for vi in 0..n {
        let v_v = v[vi];
        let nrm = (v_v[0].powi(2) + v_v[1].powi(2) + v_v[2].powi(2)).sqrt();
        let nv = [v_v[0] / nrm, v_v[1] / nrm, v_v[2] / nrm];
        // u_first = V[adj[v][0]] - V[v]
        let u0 = v[adj[vi][0]];
        let u_first = [u0[0] - v_v[0], u0[1] - v_v[1], u0[2] - v_v[2]];
        // e1 = u_first - (u_first·nv) · nv; normalize.
        let d = u_first[0] * nv[0] + u_first[1] * nv[1] + u_first[2] * nv[2];
        let e1_raw = [u_first[0] - d * nv[0], u_first[1] - d * nv[1], u_first[2] - d * nv[2]];
        let e1_n = (e1_raw[0].powi(2) + e1_raw[1].powi(2) + e1_raw[2].powi(2)).sqrt();
        let e1 = [e1_raw[0] / e1_n, e1_raw[1] / e1_n, e1_raw[2] / e1_n];
        // e2 = nv × e1.
        let e2 = [
            nv[1] * e1[2] - nv[2] * e1[1],
            nv[2] * e1[0] - nv[0] * e1[2],
            nv[0] * e1[1] - nv[1] * e1[0],
        ];
        // Score each neighbour by atan2(d·e2, d·e1).
        let mut scored: Vec<(f64, usize)> = adj[vi]
            .iter()
            .map(|&u| {
                let dv = [v[u][0] - v_v[0], v[u][1] - v_v[1], v[u][2] - v_v[2]];
                let de1 = dv[0] * e1[0] + dv[1] * e1[1] + dv[2] * e1[2];
                let de2 = dv[0] * e2[0] + dv[1] * e2[1] + dv[2] * e2[2];
                (de2.atan2(de1), u)
            })
            .collect();
        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        rot[vi] = [scored[0].1, scored[1].1, scored[2].1];
    }
    rot
}

/// Trace face cycles via the combinatorial-map next_in_face rule.
/// Mirrors `_trace_faces`.
fn trace_faces(rot: &[[usize; 3]]) -> Vec<Vec<usize>> {
    let n = rot.len();
    let mut visited: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    let mut cycles: Vec<Vec<usize>> = Vec::new();
    let succ = |u: usize, vv: usize| -> usize {
        let ru = &rot[u];
        let k = ru.iter().position(|&x| x == vv).expect("succ: v not in rot[u]");
        ru[(k + 1) % 3]
    };
    for v0 in 0..n {
        for &u0 in rot[v0].iter() {
            if visited.contains(&(v0, u0)) {
                continue;
            }
            let mut cyc: Vec<usize> = Vec::new();
            let mut vv = v0;
            let mut u = u0;
            loop {
                visited.insert((vv, u));
                cyc.push(vv);
                let w = succ(u, vv);
                vv = u;
                u = w;
                if (vv, u) == (v0, u0) {
                    break;
                }
            }
            cycles.push(cyc);
        }
    }
    assert_eq!(cycles.len(), 32);
    cycles
}

/// Orient a face cycle outward (centroid points outward from
/// origin). Mirrors `_orient_outward`.
fn orient_outward(v: &[[f64; 3]], cycle: &[usize]) -> Vec<usize> {
    let k = cycle.len();
    let mut centroid = [0.0_f64; 3];
    for &i in cycle {
        centroid[0] += v[i][0];
        centroid[1] += v[i][1];
        centroid[2] += v[i][2];
    }
    centroid[0] /= k as f64;
    centroid[1] /= k as f64;
    centroid[2] /= k as f64;
    let cn = (centroid[0].powi(2) + centroid[1].powi(2) + centroid[2].powi(2)).sqrt();
    let n_out = [centroid[0] / cn, centroid[1] / cn, centroid[2] / cn];
    let mut n = [0.0_f64; 3];
    for i in 0..k {
        let a = v[cycle[i]];
        let b = v[cycle[(i + 1) % k]];
        n[0] += a[1] * b[2] - a[2] * b[1];
        n[1] += a[2] * b[0] - a[0] * b[2];
        n[2] += a[0] * b[1] - a[1] * b[0];
    }
    let dot = n[0] * n_out[0] + n[1] * n_out[1] + n[2] * n_out[2];
    if dot < 0.0 {
        let mut rev: Vec<usize> = cycle.iter().rev().copied().collect();
        // Python `tuple(reversed(cycle))` does a strict reverse —
        // that's what we want.
        rev.shrink_to_fit();
        rev
    } else {
        cycle.to_vec()
    }
}

/// Face stored as `Vec<(edge_index, sign)>` matching Halcyon's
/// `graph.faces[f]` storage.
pub type SignedFace = Vec<(usize, i32)>;

/// Buckyball + signed-face table. The signed faces are the
/// indexing payload TDD-HAL-I.6 reads through.
pub struct Buckyball {
    pub lattice: Lattice,
    pub signed_faces: Vec<SignedFace>,
}

/// Build the buckyball with both the GIGI `Lattice` (vertex-cycle
/// faces) and the Halcyon `(edge_idx, sign)` signed-face table.
pub fn buckyball_with_signed_faces() -> Buckyball {
    let v = build_vertices();
    let (pairs, adj, edge_index) = build_edges(&v);
    let rot = build_rotation_system(&v, &adj);
    let raw_cycles = trace_faces(&rot);

    let mut pent_cycles: Vec<Vec<usize>> = Vec::new();
    let mut hex_cycles: Vec<Vec<usize>> = Vec::new();
    for c in &raw_cycles {
        let oc = orient_outward(&v, c);
        match oc.len() {
            5 => pent_cycles.push(oc),
            6 => hex_cycles.push(oc),
            other => panic!("unexpected face length {other}"),
        }
    }
    assert_eq!(pent_cycles.len(), 12);
    assert_eq!(hex_cycles.len(), 20);

    let mut signed_faces: Vec<SignedFace> = Vec::with_capacity(32);
    let mut vertex_faces: Vec<Vec<VertexId>> = Vec::with_capacity(32);

    let emit = |cyc: &[usize],
                signed_faces: &mut Vec<SignedFace>,
                vertex_faces: &mut Vec<Vec<VertexId>>| {
        let mut es: SignedFace = Vec::with_capacity(cyc.len());
        for i in 0..cyc.len() {
            let a = cyc[i];
            let b = cyc[(i + 1) % cyc.len()];
            if a < b {
                let k = edge_index[a][b];
                assert!(k >= 0, "edge ({a},{b}) not in edge_index");
                es.push((k as usize, 1));
            } else {
                let k = edge_index[b][a];
                assert!(k >= 0, "edge ({b},{a}) not in edge_index");
                es.push((k as usize, -1));
            }
        }
        signed_faces.push(es);
        vertex_faces.push(cyc.to_vec());
    };

    // Pentagons first, then hexagons — matches Python order.
    for c in &pent_cycles {
        emit(c, &mut signed_faces, &mut vertex_faces);
    }
    for c in &hex_cycles {
        emit(c, &mut signed_faces, &mut vertex_faces);
    }

    // Sanity: edge-sign sum is zero per edge.
    let mut sum = vec![0_i32; pairs.len()];
    for face in &signed_faces {
        for &(eidx, s) in face {
            sum[eidx] += s;
        }
    }
    assert!(sum.iter().all(|&v| v == 0), "face orientation inconsistent");

    let lat = Lattice::new(
        "buckyball",
        60,
        pairs,
        vertex_faces,
        Some("S2".to_string()),
    );

    Buckyball {
        lattice: lat,
        signed_faces,
    }
}

/// Convenience: just the `Lattice` (drops signed_faces). Matches the
/// gate-I.2 narrative ("declare a 4-vertex Lattice, ... declare
/// LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON"). Most callers
/// want the signed-face form via `buckyball_with_signed_faces`.
pub fn buckyball() -> Lattice {
    buckyball_with_signed_faces().lattice
}

/// Translate Halcyon's per-face `(edge_idx, sign)` cycle to the
/// substrate's `(EdgeId, EdgeOrientation)` form. `sign = +1`
/// → Forward; `sign = -1` → Reverse.
pub fn signed_face_to_walker(face: &SignedFace) -> Vec<(usize, EdgeOrientation)> {
    face.iter()
        .map(|&(eidx, sign)| {
            let orient = if sign == 1 {
                EdgeOrientation::Forward
            } else if sign == -1 {
                EdgeOrientation::Reverse
            } else {
                panic!("signed face sign must be ±1, got {sign}");
            };
            (eidx, orient)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// TDD-HAL-I.2 — buckyball topology.
    /// V = 60, E = 90, F = 32; χ = 2; face-size histogram = {12
    /// pentagons, 20 hexagons}; every edge is shared by exactly 2
    /// faces.
    #[test]
    fn tdd_hal_i_2_buckyball_topology() {
        let lat = buckyball();

        // Counts.
        assert_eq!(lat.n_vertices, 60, "V");
        assert_eq!(lat.n_edges(), 90, "E");
        assert_eq!(lat.n_faces(), 32, "F");

        // Euler χ = V − E + F = 2.
        assert_eq!(lat.euler_characteristic(), 2, "Euler characteristic");

        // Face-size histogram: {5: 12, 6: 20}.
        let mut hist: HashMap<usize, usize> = HashMap::new();
        for face in &lat.faces {
            *hist.entry(face.len()).or_insert(0) += 1;
        }
        assert_eq!(hist.get(&5).copied().unwrap_or(0), 12, "12 pentagons");
        assert_eq!(hist.get(&6).copied().unwrap_or(0), 20, "20 hexagons");
        assert_eq!(hist.len(), 2, "only pentagons and hexagons");

        // Every edge is shared by exactly 2 faces. Resolve each
        // consecutive vertex pair in every face to an `edge_id`,
        // count occurrences across all faces; every edge must
        // appear exactly twice.
        let mut edge_counts: HashMap<usize, usize> =
            (0..lat.n_edges()).map(|i| (i, 0)).collect();
        for face in &lat.faces {
            let n = face.len();
            for i in 0..n {
                let a = face[i];
                let b = face[(i + 1) % n];
                let (edge_id, _orient) = lat
                    .resolve_edge(a, b)
                    .unwrap_or_else(|| panic!("face edge ({a},{b}) not in incidence"));
                *edge_counts.entry(edge_id).or_insert(0) += 1;
            }
        }
        for (eid, count) in &edge_counts {
            assert_eq!(*count, 2, "edge {eid} appears in {count} faces, expected 2");
        }

        // Topology hint is preserved.
        assert_eq!(lat.topology.as_deref(), Some("S2"));
    }

    /// Sanity: the signed-face table is consistent with the
    /// vertex-cycle face table (each face has the same length,
    /// each edge appears in exactly 2 faces, edge-sign sum is 0).
    #[test]
    fn signed_face_table_consistency() {
        let bb = buckyball_with_signed_faces();
        assert_eq!(bb.signed_faces.len(), 32);
        for (face_idx, sf) in bb.signed_faces.iter().enumerate() {
            assert_eq!(
                sf.len(),
                bb.lattice.faces[face_idx].len(),
                "face {face_idx} length mismatch"
            );
        }
        // Each edge appears exactly twice with opposite signs.
        let mut sum = vec![0_i32; bb.lattice.n_edges()];
        let mut count = vec![0_usize; bb.lattice.n_edges()];
        for sf in &bb.signed_faces {
            for &(eidx, s) in sf {
                sum[eidx] += s;
                count[eidx] += 1;
            }
        }
        for (i, (&s, &c)) in sum.iter().zip(count.iter()).enumerate() {
            assert_eq!(c, 2, "edge {i} in {c} faces, expected 2");
            assert_eq!(s, 0, "edge {i} sign sum {s}, expected 0");
        }
    }
}
