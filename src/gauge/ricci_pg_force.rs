//! LINK_FORCE: the SU(2) force on every link of a closed tetrahedral mesh of a
//! quotient of S³ (spec: docs/YANG_MILLS_ENGINE_SPEC.md, YM2).
//!
//! Engine form of scripts/ricci_su2_pg_force.py (paper: *Reading the Curve*, §10).
//!
//! * `Native` is the RF2B Whitney force: the gradient of the cell action
//!   E = Σ_K w_K · 2 Σ_ij M_ij D_i · D_j (variational).
//! * `Pg2` is the two-ring Petrov–Galerkin force. Per cell it replaces the
//!   native face response M·D (components 1..3) by the Whitney pairing of the
//!   covariantly recovered field, ∫ P · g · W_face, and keeps the scalar
//!   component. It is **not** the gradient of an energy.
//!
//! In both, the cell's 18 right-trivialised derivatives are
//! a = 4 Σ_face Σ_comp dD[face][comp] h[face][comp], lifted to stored edges by
//! −Ad(U) on reversed edges and summed with the cell weights.
use super::curvature_recovery::{
    build_face_index, recover_cell_su2, vertex_ids, Cell, Mechanism, Su2Flux, Su2Mechanism,
};
use super::ricci_simplicial::{adjoint, cell_defects};
use super::ricci_whitney::tetra_mass;
use nalgebra::{DMatrix, Matrix3, Matrix4, SMatrix, Vector3, Vector4};

const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
const FACES: [(usize, usize, usize); 4] = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];

/// Which force.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ForceMethod {
    /// RF2B Whitney force (variational).
    Native,
    /// Two-ring Petrov–Galerkin force (not variational). `register: false` is
    /// the mechanism removal of the paper's Table 6.
    Pg2 { rings: usize, register: bool },
}

impl ForceMethod {
    /// Whether the force is the gradient of an energy. Part of the result.
    pub fn variational(self) -> bool {
        matches!(self, ForceMethod::Native)
    }
}

/// The force on every stored edge (right-trivialised, three components) and
/// the native cell-action energy at the same links.
#[derive(Clone, Debug)]
pub struct LinkForce {
    pub gradient: Vec<[f64; 3]>,
    pub energy: f64,
    pub volume: f64,
    pub variational: bool,
}

/// Per-quadrature-point data on a radial spherical tetrahedron, built exactly as
/// `tetra_mass` builds its integrands.
struct QuadPoint {
    xi: Vector3<f64>,     // barycentric λ₁..λ₃
    g: Matrix3<f64>,      // metric in reference coordinates
    w: f64,               // weight · √det g / det g
    two: [Vector3<f64>; 4],
}

fn quadrature_points(vertices: [[f64; 4]; 4], metric: [f64; 3], order: usize) -> Result<Vec<QuadPoint>, String> {
    let points = vertices.map(|v| Vector4::from_row_slice(&v));
    let p = Matrix4::from_columns(&points);
    let b = SMatrix::<f64, 4, 3>::from_columns(&[points[1] - points[0], points[2] - points[0], points[3] - points[0]]);
    let gradients = [Vector3::new(-1., -1., -1.), Vector3::new(1., 0., 0.), Vector3::new(0., 1., 0.), Vector3::new(0., 0., 1.)];
    let diagonal = Matrix3::from_diagonal(&Vector3::from_row_slice(&metric));
    let n = order;
    let mut jacobi = DMatrix::<f64>::zeros(n, n);
    for i in 1..n {
        let off = i as f64 / ((4 * i * i - 1) as f64).sqrt();
        jacobi[(i - 1, i)] = off;
        jacobi[(i, i - 1)] = off;
    }
    let eigen = jacobi.symmetric_eigen();
    let nodes: Vec<(f64, f64)> =
        (0..n).map(|i| ((eigen.eigenvalues[i] + 1.) / 2., eigen.eigenvectors[(0, i)].powi(2))).collect();
    let mut out = Vec::with_capacity(n * n * n);
    for &(u, wu) in &nodes {
        for &(v, wv) in &nodes {
            for &(w, ww) in &nodes {
                let (a, beta, c) = (u, (1. - u) * v, (1. - u) * (1. - v) * w);
                let lambda = [1. - a - beta - c, a, beta, c];
                let y = p * Vector4::from_row_slice(&lambda);
                let length = y.norm();
                if length < 1e-10 {
                    return Err("LINK_FORCE: radial map approaches zero; refine the cell".into());
                }
                let x = y / length;
                let dx = (b - x * (x.transpose() * b)) / length;
                let mut maurer = Matrix3::zeros();
                for i in 0..3 {
                    for axis in 0..3 {
                        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
                        maurer[(i, axis)] = x[0] * dx[(i + 1, axis)] - dx[(0, axis)] * x[i + 1]
                            - x[j + 1] * dx[(k + 1, axis)]
                            + x[k + 1] * dx[(j + 1, axis)];
                    }
                }
                let g = maurer.transpose() * diagonal * maurer;
                let det = g.determinant();
                if !det.is_finite() || det <= 0. {
                    return Err("LINK_FORCE: nonpositive metric determinant".into());
                }
                let weight = wu * wv * ww * (1. - u).powi(2) * (1. - v) * det.sqrt();
                let two = FACES.map(|(i, j, k)| {
                    2. * (lambda[i] * gradients[j].cross(&gradients[k]) - lambda[j] * gradients[i].cross(&gradients[k])
                        + lambda[k] * gradients[i].cross(&gradients[j]))
                });
                out.push(QuadPoint { xi: Vector3::new(a, beta, c), g, w: weight / det, two });
            }
        }
    }
    Ok(out)
}

/// The closed linear basis c_k + B_k z (the recovery's, in the same order).
fn basis() -> [(Vector3<f64>, Matrix3<f64>); 11] {
    super::curvature_recovery::closed_linear_basis()
}

/// The force on every stored edge.
pub fn link_force(
    cells: &[Cell],
    weights: &[f64],
    links: &[[f64; 4]],
    metric: [f64; 3],
    order: usize,
    method: ForceMethod,
) -> Result<LinkForce, String> {
    if cells.len() != weights.len() {
        return Err("LINK_FORCE: one weight per cell required".into());
    }
    if !metric.iter().all(|g| g.is_finite() && *g > 0.0) {
        return Err("LINK_FORCE: a positive metric (g1, g2, g3) is required; none is ever substituted".into());
    }
    for (ci, c) in cells.iter().enumerate() {
        if !(weights[ci].is_finite() && weights[ci] > 0. && weights[ci] <= 1.) {
            return Err(format!("LINK_FORCE: cell {ci} has an invalid weight {}", weights[ci]));
        }
        if let Some(&e) = c.edges.iter().find(|&&e| e >= links.len()) {
            return Err(format!("LINK_FORCE: cell {ci} names edge {e}, but only {} links are given", links.len()));
        }
    }
    let (faces, vid) = match method {
        ForceMethod::Native => (None, None),
        ForceMethod::Pg2 { .. } => (
            Some(build_face_index(cells).map_err(|e| e.to_string())?),
            Some(vertex_ids(cells)),
        ),
    };
    let basis = basis();
    let mut gradient = vec![Vector3::<f64>::zeros(); links.len()];
    let (mut energy, mut volume) = (0.0, 0.0);
    for (ci, cell) in cells.iter().enumerate() {
        let local: [[f64; 4]; 6] = std::array::from_fn(|e| {
            let mut q = links[cell.edges[e]];
            if cell.reverse[e] {
                for x in &mut q[1..] {
                    *x = -*x;
                }
            }
            q
        });
        let mass = tetra_mass(cell.vertices, metric, order)?;
        let d = cell_defects(local)?;
        let m = mass.face_mass;
        volume += weights[ci] * mass.volume;
        // native face response h = M D, and the native energy
        let mut h = [[0.0; 4]; 4];
        for i in 0..4 {
            for c in 0..4 {
                h[i][c] = (0..4).map(|j| m[(i, j)] * d.value[j][c]).sum();
            }
        }
        energy += weights[ci] * 2.0 * (0..4).map(|i| (0..4).map(|c| h[i][c] * d.value[i][c]).sum::<f64>()).sum::<f64>();
        if let ForceMethod::Pg2 { rings, register } = method {
            let su = Su2Mechanism { register, abelianised: false, flux: Su2Flux::Defect };
            let rec = recover_cell_su2(cells, faces.as_ref().unwrap(), vid.as_ref().unwrap(), links, ci, rings, Mechanism::FULL, su)
                .map_err(|e| e.to_string())?;
            let jinv = rec.chart_j.try_inverse().ok_or("LINK_FORCE: singular chart")?;
            let detj = rec.chart_j.determinant();
            let quad = quadrature_points(cell.vertices, metric, order)?;
            let mut hvec = [[0.0; 3]; 4];
            for q in &quad {
                let z = rec.chart_j * q.xi + rec.chart_t0;
                for comp in 0..3 {
                    let mut pc = Vector3::zeros();
                    for k in 0..11 {
                        pc += rec.coefficients[comp][k] * (basis[k].0 + basis[k].1 * z);
                    }
                    let pref = detj * (jinv * pc);
                    for f in 0..4 {
                        hvec[f][comp] += pref.dot(&(q.g * q.two[f])) * q.w;
                    }
                }
            }
            for f in 0..4 {
                for comp in 0..3 {
                    h[f][comp + 1] = hvec[f][comp];
                }
            }
        }
        let mut a = [0.0; 18];
        for (dof, slot) in a.iter_mut().enumerate() {
            let mut s = 0.0;
            for f in 0..4 {
                for c in 0..4 {
                    s += d.derivative[f][c][dof] * h[f][c];
                }
            }
            *slot = 4.0 * s;
        }
        for e in 0..6 {
            let lift = if cell.reverse[e] { -adjoint(links[cell.edges[e]]) } else { Matrix3::identity() };
            let ae = Vector3::new(a[3 * e], a[3 * e + 1], a[3 * e + 2]);
            gradient[cell.edges[e]] += weights[ci] * (lift.transpose() * ae);
        }
    }
    let _ = EDGES;
    Ok(LinkForce {
        gradient: gradient.into_iter().map(|v| [v[0], v[1], v[2]]).collect(),
        energy,
        volume,
        variational: method.variational(),
    })
}
