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
    build_face_index, prepare_su2_cell, read_prepared_su2, vertex_ids, Cell, FaceIndex, Mechanism,
    Su2Flux, Su2Mechanism, Su2Prepared,
};
use super::ricci_simplicial::{adjoint, cell_defects};
use super::ricci_whitney::{tetra_mass, TetraMass};
use nalgebra::{DMatrix, Matrix3, Matrix4, SMatrix, Vector3, Vector4};


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

/// Everything about a mesh that does not depend on the links: per cell the mass
/// matrices, the quadrature data and (two-ring) the prepared reading; and which
/// cells read each link. Built once; used for the force and for every Jacobian
/// column.
pub struct ForceContext {
    method: ForceMethod,
    weights: Vec<f64>,
    mass: Vec<TetraMass>,
    quad: Vec<Vec<QuadPoint>>,
    prepared: Vec<Option<Su2Prepared>>,
    vid: Vec<[usize; 4]>,
    /// readers[edge]: the cells whose contribution depends on that link.
    readers: Vec<Vec<usize>>,
}

impl ForceContext {
    pub fn new(
        cells: &[Cell],
        weights: &[f64],
        n_links: usize,
        metric: [f64; 3],
        order: usize,
        method: ForceMethod,
    ) -> Result<Self, String> {
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
            if let Some(&e) = c.edges.iter().find(|&&e| e >= n_links) {
                return Err(format!("LINK_FORCE: cell {ci} names edge {e}, but only {n_links} links are given"));
            }
        }
        let mass = cells.iter().map(|c| tetra_mass(c.vertices, metric, order)).collect::<Result<Vec<_>, _>>()?;
        let (quad, prepared, vid) = match method {
            ForceMethod::Native => (vec![], (0..cells.len()).map(|_| None).collect(), vec![]),
            ForceMethod::Pg2 { rings, .. } => {
                let faces: FaceIndex = build_face_index(cells).map_err(|e| e.to_string())?;
                let quad = cells.iter().map(|c| quadrature_points(c.vertices, metric, order)).collect::<Result<Vec<_>, _>>()?;
                let prepared = (0..cells.len())
                    .map(|ci| prepare_su2_cell(cells, &faces, ci, rings, Mechanism::FULL).map(Some).map_err(|e| e.to_string()))
                    .collect::<Result<Vec<_>, _>>()?;
                (quad, prepared, vertex_ids(cells))
            }
        };
        let mut readers = vec![Vec::new(); n_links];
        for ci in 0..cells.len() {
            let mut reads: Vec<usize> = match &prepared[ci] {
                None => cells[ci].edges.to_vec(),
                Some(p) => p.neighbourhood().iter().flat_map(|&nj| cells[nj].edges).collect(),
            };
            reads.sort_unstable();
            reads.dedup();
            for e in reads {
                readers[e].push(ci);
            }
        }
        Ok(ForceContext { method, weights: weights.to_vec(), mass, quad, prepared, vid, readers })
    }

    /// One cell's weighted contribution to its six stored edges.
    fn cell(&self, cells: &[Cell], links: &[[f64; 4]], ci: usize) -> Result<([Vector3<f64>; 6], f64), String> {
        let cell = &cells[ci];
        let local: [[f64; 4]; 6] = std::array::from_fn(|e| {
            let mut q = links[cell.edges[e]];
            if cell.reverse[e] {
                for x in &mut q[1..] {
                    *x = -*x;
                }
            }
            q
        });
        let d = cell_defects(local)?;
        let m = self.mass[ci].face_mass;
        let mut h = [[0.0; 4]; 4];
        for i in 0..4 {
            for c in 0..4 {
                h[i][c] = (0..4).map(|j| m[(i, j)] * d.value[j][c]).sum();
            }
        }
        let energy = 2.0 * (0..4).map(|i| (0..4).map(|c| h[i][c] * d.value[i][c]).sum::<f64>()).sum::<f64>();
        if let ForceMethod::Pg2 { register, .. } = self.method {
            let su = Su2Mechanism { register, abelianised: false, flux: Su2Flux::Defect };
            let rec = read_prepared_su2(self.prepared[ci].as_ref().unwrap(), cells, &self.vid, links, su)
                .map_err(|e| e.to_string())?;
            let jinv = rec.chart_j.try_inverse().ok_or("LINK_FORCE: singular chart")?;
            let detj = rec.chart_j.determinant();
            let basis = basis();
            let mut hvec = [[0.0; 3]; 4];
            for q in &self.quad[ci] {
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
        let w = self.weights[ci];
        let out = std::array::from_fn(|e| {
            let mut ae = Vector3::zeros();
            for axis in 0..3 {
                let dof = 3 * e + axis;
                let mut s = 0.0;
                for f in 0..4 {
                    for c in 0..4 {
                        s += d.derivative[f][c][dof] * h[f][c];
                    }
                }
                ae[axis] = 4.0 * s;
            }
            let lift = if cell.reverse[e] { -adjoint(links[cell.edges[e]]) } else { Matrix3::identity() };
            w * (lift.transpose() * ae)
        });
        Ok((out, w * energy))
    }
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
    let ctx = ForceContext::new(cells, weights, links.len(), metric, order, method)?;
    let mut gradient = vec![Vector3::<f64>::zeros(); links.len()];
    let mut energy = 0.0;
    for ci in 0..cells.len() {
        let (contrib, e) = ctx.cell(cells, links, ci)?;
        energy += e;
        for (k, v) in contrib.iter().enumerate() {
            gradient[cells[ci].edges[k]] += v;
        }
    }
    let volume = ctx.mass.iter().zip(&ctx.weights).map(|(m, w)| w * m.volume).sum();
    Ok(LinkForce {
        gradient: gradient.into_iter().map(|v| [v[0], v[1], v[2]]).collect(),
        energy,
        volume,
        variational: method.variational(),
    })
}

/// The linearised force K (YM3): column (g, k) is the central difference of the
/// force under U_g → U_g exp(±ε τ_k), right-trivialised, with only the cells that
/// read link g recomputed. Returned as sparse triplets (row = 3·edge + axis);
/// entries that are exactly zero are dropped.
pub struct Jacobian {
    pub n: usize,
    pub rows: Vec<u32>,
    pub cols: Vec<u32>,
    pub vals: Vec<f64>,
}

/// Link product, stored-link convention (as curvature_recovery's).
fn lmul4(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let (av, bv) = (Vector3::new(a[1], a[2], a[3]), Vector3::new(b[1], b[2], b[3]));
    let v = bv * a[0] + av * b[0] - av.cross(&bv);
    [a[0] * b[0] - av.dot(&bv), v[0], v[1], v[2]]
}

pub fn link_force_jacobian(
    cells: &[Cell],
    weights: &[f64],
    links: &[[f64; 4]],
    metric: [f64; 3],
    order: usize,
    method: ForceMethod,
    eps: f64,
) -> Result<Jacobian, String> {
    if !(eps.is_finite() && eps > 0.0) {
        return Err("LINK_FORCE JACOBIAN: a positive step is required".into());
    }
    let ctx = ForceContext::new(cells, weights, links.len(), metric, order, method)?;
    let n = 3 * links.len();
    let (mut rows, mut cols, mut vals) = (Vec::new(), Vec::new(), Vec::new());
    let mut work = links.to_vec();
    for g in 0..links.len() {
        for k in 0..3 {
            let mut col: std::collections::BTreeMap<usize, f64> = std::collections::BTreeMap::new();
            for sign in [1.0_f64, -1.0] {
                let mut step = [(sign * eps).cos(), 0.0, 0.0, 0.0];
                step[1 + k] = (sign * eps).sin();
                work[g] = lmul4(links[g], step);
                for &ci in &ctx.readers[g] {
                    let (contrib, _) = ctx.cell(cells, &work, ci)?;
                    for (e, v) in contrib.iter().enumerate() {
                        for axis in 0..3 {
                            *col.entry(3 * cells[ci].edges[e] + axis).or_insert(0.0) += sign * v[axis] / (2.0 * eps);
                        }
                    }
                }
            }
            work[g] = links[g];
            for (r, v) in col {
                if v != 0.0 {
                    rows.push(r as u32);
                    cols.push((3 * g + k) as u32);
                    vals.push(v);
                }
            }
        }
    }
    Ok(Jacobian { n, rows, cols, vals })
}
