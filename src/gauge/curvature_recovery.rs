//! RECOVER_CURVATURE — two-ring closed linear curvature recovery.
//!
//! Engine form of `scripts/ricci_pg_tworing.py` (paper: *Reading the Curve,
//! Not Counting the Points*, `docs/papers/two-ring-pg/`). Spec:
//! `docs/RECOVER_CURVATURE_SPEC.md`.
//!
//! For each tetrahedral cell of a closed mesh on a quotient of S³ (vertices
//! are unit quaternions, scalar first) and a 1-cochain `a` of edge integrals,
//! recover the closed linear curvature `P_K(z) = c + B z`, `tr B = 0`, in the
//! cell's chart, by equality-constrained least squares:
//!
//! * the cell's own four face fluxes are held exactly;
//! * every other distinct face within `rings` face-steps is fitted in least
//!   squares;
//! * every neighbour is first carried onto the cell's sheet of the cover by
//!   the left quaternion multiplication fixed at a shared vertex, and refused
//!   if its shared vertices then fail to coincide.
//!
//! The chart frame is canonical (Gram–Schmidt, positive diagonal), so the
//! coefficients are reproducible. Amplification, condition and fluxes do not
//! depend on the frame's signs.
//!
//! Every failure is a refusal naming the cell. Nothing is guessed.

use nalgebra::{DMatrix, DVector, Matrix3, Vector3, Vector4};
use std::collections::HashMap;
use std::fmt;

/// Local edges (i, j), i < j, in the order the cell's `edges` / `reverse` use.
pub const EDGES: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
/// Local faces (i, j, k), boundary i → j → k.
pub const FACES: [(usize, usize, usize); 4] = [(0, 1, 2), (0, 1, 3), (0, 2, 3), (1, 2, 3)];

/// Relative singular-value cutoff for rank and pseudo-inverses.
pub const RCOND: f64 = 1e-12;
/// Largest admissible condition number of a cell's flux map.
pub const COND_CAP: f64 = 1e12;
/// Largest admissible vertex gap after transport onto a cell's sheet.
pub const COINCIDE: f64 = 1e-12;
/// Admissible range of `rings`.
pub const MAX_RINGS: usize = 3;
/// SU(2): largest admissible | |link| − 1 |.
pub const UNIT_TOL: f64 = 1e-9;

/// One tetrahedral cell.
#[derive(Clone, Debug)]
pub struct Cell {
    /// Unit quaternions (q0, q1, q2, q3), scalar first.
    pub vertices: [[f64; 4]; 4],
    /// Global edge id of each local edge in [`EDGES`] order.
    pub edges: [usize; 6],
    /// True if the stored edge runs opposite to the local edge (i → j).
    pub reverse: [bool; 6],
}

/// The recovered curvature of one cell and its diagnostics.
#[derive(Clone, Debug)]
pub struct CellRecovery {
    /// Constant part of the closed linear proxy, in the cell's chart.
    pub c: [f64; 3],
    /// Linear part, traceless, in the cell's chart.
    pub b: [[f64; 3]; 3],
    /// The eleven basis coefficients (3 constant, 6 off-diagonal, 2 diagonal).
    pub coefficients: [f64; 11],
    /// 2-norm of the linear map from neighbourhood fluxes to coefficients.
    pub amplification: f64,
    /// Ratio of largest to smallest singular value of the flux map.
    pub condition: f64,
    /// Numerical rank of the flux map (11 when accepted).
    pub rank: usize,
    /// Distinct faces read.
    pub faces: usize,
    /// Cells in the neighbourhood, the cell itself included.
    pub cells: usize,
    /// Largest |recovered flux − data flux| over all faces read.
    pub residual: f64,
    /// Largest |recovered flux − data flux| over the cell's own four faces.
    pub own_residual: f64,
}

/// A refusal. Each names what was violated and where.
#[derive(Clone, Debug, PartialEq)]
pub enum RecoveryError {
    InvalidRings(usize),
    EdgeOutOfRange { cell: usize, edge: usize, n_edges: usize },
    OpenMesh { faces_not_shared_twice: usize },
    NoCoincidence { cell: usize, neighbour: usize, gap: f64 },
    OutsideChart { cell: usize },
    DegenerateCell { cell: usize },
    RankDeficient { cell: usize, rank: usize },
    IllConditioned { cell: usize, condition: f64 },
    OwnFluxNotReproduced { cell: usize, residual: f64 },
    /// SU(2): there must be exactly one link per edge the cells name.
    LinkCountMismatch { links: usize, edges_needed: usize },
    /// SU(2): a link is not a unit quaternion.
    NonUnitLink { edge: usize, deviation: f64 },
    /// SU(2): a face holonomy is too close to −1 for its logarithm to be defined.
    HolonomyNearMinusOne { cell: usize, face_cell: usize },
}

impl fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use RecoveryError::*;
        match self {
            InvalidRings(r) => write!(f, "RECOVER_CURVATURE: RINGS must be in 1..={MAX_RINGS} (got {r})"),
            EdgeOutOfRange { cell, edge, n_edges } => write!(
                f, "RECOVER_CURVATURE: cell {cell} names edge {edge}, but the edge data has only {n_edges} edges"),
            OpenMesh { faces_not_shared_twice } => write!(
                f, "RECOVER_CURVATURE: the mesh is not closed — {faces_not_shared_twice} faces are not shared by exactly two cells"),
            NoCoincidence { cell, neighbour, gap } => write!(
                f, "RECOVER_CURVATURE: cell {neighbour} does not coincide with cell {cell} after transport onto its sheet (gap {gap:.3e} > {COINCIDE:e})"),
            OutsideChart { cell } => write!(
                f, "RECOVER_CURVATURE: a neighbourhood vertex of cell {cell} lies outside the cell's projective chart"),
            DegenerateCell { cell } => write!(
                f, "RECOVER_CURVATURE: cell {cell} is degenerate (its vertices do not span a 3-plane)"),
            RankDeficient { cell, rank } => write!(
                f, "RECOVER_CURVATURE: the closed linear flux map of cell {cell} has rank {rank} < 11"),
            IllConditioned { cell, condition } => write!(
                f, "RECOVER_CURVATURE: the flux map of cell {cell} has condition {condition:.3e}, above the cap {COND_CAP:e}"),
            OwnFluxNotReproduced { cell, residual } => write!(
                f, "RECOVER_CURVATURE: the own face fluxes of cell {cell} are not reproduced (residual {residual:.3e})"),
            LinkCountMismatch { links, edges_needed } => write!(
                f, "RECOVER_CURVATURE GROUP SU2: {links} links given, but the cells name {edges_needed} edges"),
            NonUnitLink { edge, deviation } => write!(
                f, "RECOVER_CURVATURE GROUP SU2: the link on edge {edge} is not a unit quaternion (| |q| - 1 | = {deviation:.3e} > {UNIT_TOL:e})"),
            HolonomyNearMinusOne { cell, face_cell } => write!(
                f, "RECOVER_CURVATURE GROUP SU2: a face holonomy read by cell {cell} (from cell {face_cell}) is near -1; its logarithm is not defined"),
        }
    }
}

impl std::error::Error for RecoveryError {}

// ── quaternions ───────────────────────────────────────────────────────────

fn qmul(p: &Vector4<f64>, q: &Vector4<f64>) -> Vector4<f64> {
    let (pv, qv) = (Vector3::new(p[1], p[2], p[3]), Vector3::new(q[1], q[2], q[3]));
    let s = p[0] * q[0] - pv.dot(&qv);
    let v = qv * p[0] + pv * q[0] + pv.cross(&qv);
    Vector4::new(s, v[0], v[1], v[2])
}

fn qconj(p: &Vector4<f64>) -> Vector4<f64> {
    Vector4::new(p[0], -p[1], -p[2], -p[3])
}

type Verts = [Vector4<f64>; 4];

fn verts(cell: &Cell) -> Verts {
    cell.vertices.map(|v| Vector4::new(v[0], v[1], v[2], v[3]))
}

// ── mesh combinatorics ────────────────────────────────────────────────────

fn edge_index(i: usize, j: usize) -> usize {
    EDGES.iter().position(|&e| e == (i.min(j), i.max(j))).expect("local edge")
}

/// A face, identified by its three global edge ids (sorted).
fn face_key(cell: &Cell, face: (usize, usize, usize)) -> [usize; 3] {
    let (i, j, k) = face;
    let mut key = [
        cell.edges[edge_index(i, j)],
        cell.edges[edge_index(i, k)],
        cell.edges[edge_index(j, k)],
    ];
    key.sort_unstable();
    key
}

/// Corners of a face, keyed by the two global face edges meeting there, in
/// face order. Shared faces of two cells have the same keys.
fn corners(cell: &Cell, face: (usize, usize, usize)) -> Vec<((usize, usize), usize)> {
    let f = [face.0, face.1, face.2];
    f.iter()
        .map(|&v| {
            let others: Vec<usize> = f.iter().copied().filter(|&w| w != v).collect();
            let (a, b) = (cell.edges[edge_index(v, others[0])], cell.edges[edge_index(v, others[1])]);
            ((a.min(b), a.max(b)), v)
        })
        .collect()
}

/// Flux of the cochain through a face, boundary i → j → k.
fn data_flux(cell: &Cell, a: &[f64], face: (usize, usize, usize)) -> f64 {
    let (i, j, k) = face;
    let s = |e: usize| if cell.reverse[e] { -1.0 } else { 1.0 };
    let (eij, ejk, eik) = (edge_index(i, j), edge_index(j, k), edge_index(i, k));
    s(eij) * a[cell.edges[eij]] + s(ejk) * a[cell.edges[ejk]] - s(eik) * a[cell.edges[eik]]
}

pub type FaceIndex = HashMap<[usize; 3], Vec<(usize, usize)>>;

fn face_index(cells: &[Cell]) -> Result<FaceIndex, RecoveryError> {
    let mut index: FaceIndex = HashMap::new();
    for (ci, c) in cells.iter().enumerate() {
        for (fi, &f) in FACES.iter().enumerate() {
            index.entry(face_key(c, f)).or_default().push((ci, fi));
        }
    }
    let bad = index.values().filter(|v| v.len() != 2).count();
    if bad > 0 {
        return Err(RecoveryError::OpenMesh { faces_not_shared_twice: bad });
    }
    Ok(index)
}

/// Cells within `rings` face-steps of `ci`, each carried onto `ci`'s sheet.
/// Insertion order is breadth-first; the first sheet a cell is reached on wins.
fn neighbourhood(
    cells: &[Cell],
    faces: &FaceIndex,
    ci: usize,
    rings: usize,
    transport: bool,
) -> Result<Vec<(usize, Verts)>, RecoveryError> {
    let mut out: Vec<(usize, Verts)> = vec![(ci, verts(&cells[ci]))];
    let mut slot: HashMap<usize, usize> = HashMap::from([(ci, 0)]);
    let mut frontier = vec![ci];
    for _ in 0..rings {
        let mut fresh = Vec::new();
        for &cj in &frontier {
            let c = out[slot[&cj]].1;
            for &face in FACES.iter() {
                for &(nj, fnj) in &faces[&face_key(&cells[cj], face)] {
                    if nj == cj {
                        continue;
                    }
                    let ck = corners(&cells[cj], face);
                    let cn: HashMap<(usize, usize), usize> =
                        corners(&cells[nj], FACES[fnj]).into_iter().collect();
                    let mut v = verts(&cells[nj]);
                    if transport {
                        let (k0, vk) = ck[0];
                        let h = qmul(&c[vk], &qconj(&v[cn[&k0]]));
                        v = v.map(|p| qmul(&h, &p));
                    }
                    let gap = ck
                        .iter()
                        .map(|(k, vk)| (c[*vk] - v[cn[k]]).amax())
                        .fold(0.0_f64, f64::max);
                    if gap > COINCIDE {
                        return Err(RecoveryError::NoCoincidence { cell: cj, neighbour: nj, gap });
                    }
                    if slot.contains_key(&nj) {
                        continue;
                    }
                    slot.insert(nj, out.len());
                    out.push((nj, v));
                    fresh.push(nj);
                }
            }
        }
        frontier = fresh;
    }
    Ok(out)
}

// ── chart ─────────────────────────────────────────────────────────────────

struct Chart {
    n: Vector4<f64>,
    c: f64,
    cen: Vector4<f64>,
    q: [Vector4<f64>; 3],
    diam: f64,
}

/// Generalised cross product: the vector orthogonal to three vectors of R⁴.
fn cross4(a: &Vector4<f64>, b: &Vector4<f64>, c: &Vector4<f64>) -> Vector4<f64> {
    let m = |cols: [usize; 3]| {
        Matrix3::new(
            a[cols[0]], a[cols[1]], a[cols[2]],
            b[cols[0]], b[cols[1]], b[cols[2]],
            c[cols[0]], c[cols[1]], c[cols[2]],
        )
        .determinant()
    };
    Vector4::new(m([1, 2, 3]), -m([0, 2, 3]), m([0, 1, 3]), -m([0, 1, 2]))
}

fn chart(k: &Verts, cell: usize) -> Result<Chart, RecoveryError> {
    let p0 = k[0];
    let b = [k[1] - p0, k[2] - p0, k[3] - p0];
    let mut n = cross4(&b[0], &b[1], &b[2]);
    let nn = n.norm();
    if !(nn > 0.0) {
        return Err(RecoveryError::DegenerateCell { cell });
    }
    n /= nn;
    let mut c = n.dot(&p0);
    if c < 0.0 {
        n = -n;
        c = -c;
    }
    // Gram–Schmidt with one re-orthogonalisation pass: Q with R's diagonal > 0,
    // the unique such frame for the column span of b.
    let mut q = [Vector4::zeros(); 3];
    for i in 0..3 {
        let mut v = b[i];
        for _ in 0..2 {
            for j in 0..i {
                let proj = q[j].dot(&v);
                v -= q[j] * proj;
            }
        }
        let norm = v.norm();
        if !(norm > 0.0) {
            return Err(RecoveryError::DegenerateCell { cell });
        }
        q[i] = v / norm;
    }
    let mean = (k[0] + k[1] + k[2] + k[3]) / 4.0;
    let cen = mean * (c / mean.dot(&n));
    let mut diam = 0.0_f64;
    for i in 0..4 {
        for j in (i + 1)..4 {
            diam = diam.max((k[i] - k[j]).norm());
        }
    }
    Ok(Chart { n, c, cen, q, diam })
}

impl Chart {
    fn to_orth(&self, v: &Verts, cell: usize) -> Result<[Vector3<f64>; 4], RecoveryError> {
        let mut out = [Vector3::zeros(); 4];
        for (i, p) in v.iter().enumerate() {
            let den = p.dot(&self.n);
            if !(den > 0.0) {
                return Err(RecoveryError::OutsideChart { cell });
            }
            let w = p * (self.c / den) - self.cen;
            out[i] = Vector3::new(self.q[0].dot(&w), self.q[1].dot(&w), self.q[2].dot(&w)) / self.diam;
        }
        Ok(out)
    }
}

// ── closed linear basis ───────────────────────────────────────────────────

/// Basis of closed linear fields c + B z with tr B = 0: three constants, six
/// off-diagonal B entries in (i, j) order, then diag(1,−1,0) and diag(0,1,−1).
/// The closed linear basis, public for the force (`ricci_pg_force`).
pub fn closed_linear_basis() -> [(Vector3<f64>, Matrix3<f64>); 11] {
    basis()
}

fn basis() -> [(Vector3<f64>, Matrix3<f64>); 11] {
    let mut out = [(Vector3::zeros(), Matrix3::zeros()); 11];
    for i in 0..3 {
        out[i].0[i] = 1.0;
    }
    let mut k = 3;
    for i in 0..3 {
        for j in 0..3 {
            if i != j {
                out[k].1[(i, j)] = 1.0;
                k += 1;
            }
        }
    }
    out[9].1 = Matrix3::from_diagonal(&Vector3::new(1.0, -1.0, 0.0));
    out[10].1 = Matrix3::from_diagonal(&Vector3::new(0.0, 1.0, -1.0));
    out
}

/// Exact flux of each basis field through a straight chart triangle.
fn flux_row(tri: [Vector3<f64>; 3], basis: &[(Vector3<f64>, Matrix3<f64>); 11]) -> [f64; 11] {
    let area = (tri[1] - tri[0]).cross(&(tri[2] - tri[0])) / 2.0;
    let cen = (tri[0] + tri[1] + tri[2]) / 3.0;
    std::array::from_fn(|k| (basis[k].0 + basis[k].1 * cen).dot(&area))
}

// ── linear algebra ────────────────────────────────────────────────────────

// nalgebra 0.33's SVD was measured to reconstruct the rank-deficient 11×4
// own-face matrix with error 2.6e-3 (it should be ~1e-16), which corrupted
// the pseudo-inverse and broke the equality constraint. The matrices here are
// tiny (at most ~60×11), so a one-sided Jacobi SVD is used instead: it is
// simple, and it computes small singular values to high relative accuracy.

/// One-sided Jacobi SVD of a tall matrix (rows ≥ cols): returns U (m×n, columns
/// orthonormal where s > 0), s (n, unsorted) and V (n×n) with M = U diag(s) Vᵀ.
fn jacobi_svd(m: &DMatrix<f64>) -> (DMatrix<f64>, Vec<f64>, DMatrix<f64>) {
    debug_assert!(m.nrows() >= m.ncols());
    let n = m.ncols();
    let mut u = m.clone();
    let mut v = DMatrix::<f64>::identity(n, n);
    for _sweep in 0..80 {
        let mut rotated = false;
        for p in 0..n {
            for q in (p + 1)..n {
                let alpha = u.column(p).norm_squared();
                let beta = u.column(q).norm_squared();
                let gamma = u.column(p).dot(&u.column(q));
                if gamma == 0.0 || gamma.abs() <= 1e-15 * (alpha * beta).sqrt() {
                    continue;
                }
                rotated = true;
                let zeta = (beta - alpha) / (2.0 * gamma);
                let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = c * t;
                for (mat, rows) in [(&mut u, m.nrows()), (&mut v, n)] {
                    for r in 0..rows {
                        let (xp, xq) = (mat[(r, p)], mat[(r, q)]);
                        mat[(r, p)] = c * xp - s * xq;
                        mat[(r, q)] = s * xp + c * xq;
                    }
                }
            }
        }
        if !rotated {
            break;
        }
    }
    let s: Vec<f64> = (0..n).map(|j| u.column(j).norm()).collect();
    for (j, &sj) in s.iter().enumerate() {
        if sj > 0.0 {
            u.column_mut(j).scale_mut(1.0 / sj);
        }
    }
    (u, s, v)
}

fn singular_values(m: &DMatrix<f64>) -> Vec<f64> {
    if m.nrows() < m.ncols() {
        return singular_values(&m.transpose());
    }
    jacobi_svd(m).1
}

/// Moore–Penrose pseudo-inverse with a cutoff relative to the largest
/// singular value (numpy's `pinv(…, rcond)` convention).
fn pinv(m: &DMatrix<f64>) -> DMatrix<f64> {
    if m.nrows() < m.ncols() {
        return pinv(&m.transpose()).transpose();
    }
    let (u, s, v) = jacobi_svd(m);
    let smax = s.iter().copied().fold(0.0_f64, f64::max);
    let cut = RCOND * smax;
    let mut out = DMatrix::zeros(m.ncols(), m.nrows());
    for (i, &si) in s.iter().enumerate() {
        if si > cut {
            out += (v.column(i) * u.column(i).transpose()) / si;
        }
    }
    out
}

// ── the recovery ──────────────────────────────────────────────────────────

/// Mechanism switches. Production uses [`Mechanism::FULL`]; the others exist
/// so tests can remove one mechanism and watch a gate turn red.
#[derive(Clone, Copy, Debug)]
pub struct Mechanism {
    pub transport: bool,
    pub equality: bool,
}

impl Mechanism {
    pub const FULL: Mechanism = Mechanism { transport: true, equality: true };
}

/// Recover one cell. `flux` overrides the cochain fluxes with a field
/// evaluated on chart triangles (planted-field tests).
pub fn recover_cell(
    cells: &[Cell],
    faces: &FaceIndex,
    a: &[f64],
    ci: usize,
    rings: usize,
    mech: Mechanism,
    flux: Option<&dyn Fn([Vector3<f64>; 3]) -> f64>,
) -> Result<CellRecovery, RecoveryError> {
    let fit = fit_map(cells, faces, ci, rings, mech)?;
    let basis = basis();
    let d = DVector::from_iterator(
        fit.faces.len(),
        fit.faces.iter().map(|(nj, f, tri)| match flux {
            Some(g) => g(*tri),
            None => data_flux(&cells[*nj], a, *f),
        }),
    );
    let x = &fit.fmap * &d;

    let scale = d.iter().take(fit.own).fold(1.0_f64, |m, v| m.max(v.abs()));
    let own_res = (&fit.am * &x - d.rows(0, fit.own)).amax();
    if mech.equality && own_res > 1e-12 * scale {
        return Err(RecoveryError::OwnFluxNotReproduced { cell: ci, residual: own_res });
    }
    let amplification = singular_values(&fit.fmap).into_iter().fold(0.0_f64, f64::max);
    let residual = (&fit.rows * &x - &d).amax();

    let mut cvec = [0.0; 3];
    let mut bmat = Matrix3::zeros();
    for k in 0..11 {
        for i in 0..3 {
            cvec[i] += x[k] * basis[k].0[i];
        }
        bmat += basis[k].1 * x[k];
    }
    Ok(CellRecovery {
        c: cvec,
        b: std::array::from_fn(|i| std::array::from_fn(|j| bmat[(i, j)])),
        coefficients: std::array::from_fn(|k| x[k]),
        amplification,
        condition: fit.condition,
        rank: fit.rank,
        faces: fit.faces.len(),
        cells: fit.nb.len(),
        residual,
        own_residual: own_res,
    })
}

/// One distinct face a cell reads: the neighbour it was read from, the local
/// face, and its chart triangle.
type ReadFace = (usize, (usize, usize, usize), [Vector3<f64>; 3]);

/// The fit of one cell: its neighbourhood, the distinct faces it reads (own
/// faces first, then the rest, breadth-first), and the linear map from those
/// faces' fluxes to the eleven coefficients. Shared by the U(1) and SU(2)
/// readings, which differ only in the fluxes they feed it.
struct Fit {
    nb: Vec<(usize, Verts)>,
    faces: Vec<ReadFace>,
    own: usize,
    am: DMatrix<f64>,
    rows: DMatrix<f64>,
    fmap: DMatrix<f64>,
    condition: f64,
    rank: usize,
    chart_j: Matrix3<f64>,
    chart_t0: Vector3<f64>,
}

fn fit_map(
    cells: &[Cell],
    faces: &FaceIndex,
    ci: usize,
    rings: usize,
    mech: Mechanism,
) -> Result<Fit, RecoveryError> {
    let nb = neighbourhood(cells, faces, ci, rings, mech.transport)?;
    let ch = chart(&nb[0].1, ci)?;
    let basis = basis();
    let mut seen = std::collections::HashSet::new();
    let (mut own_rows, mut own_f, mut rest_rows, mut rest_f) = (vec![], vec![], vec![], vec![]);
    for (nj, v) in &nb {
        let o = ch.to_orth(v, ci)?;
        for &f in FACES.iter() {
            if !seen.insert(face_key(&cells[*nj], f)) {
                continue;
            }
            let tri = [o[f.0], o[f.1], o[f.2]];
            let row = flux_row(tri, &basis);
            if *nj == ci {
                own_rows.push(row);
                own_f.push((*nj, f, tri));
            } else {
                rest_rows.push(row);
                rest_f.push((*nj, f, tri));
            }
        }
    }
    let to_mat = |rows: &[[f64; 11]]| DMatrix::from_fn(rows.len(), 11, |i, j| rows[i][j]);
    let (am, rm) = (to_mat(&own_rows), to_mat(&rest_rows));
    let all_rows: Vec<[f64; 11]> = own_rows.iter().chain(rest_rows.iter()).copied().collect();
    let rows = to_mat(&all_rows);
    let read: Vec<ReadFace> = own_f.into_iter().chain(rest_f).collect();

    let s = singular_values(&rows);
    let smax = s.iter().copied().fold(0.0_f64, f64::max);
    let smin = s.iter().copied().fold(f64::INFINITY, f64::min);
    let rank = s.iter().filter(|&&x| x > RCOND * smax).count();
    if rank < 11 {
        return Err(RecoveryError::RankDeficient { cell: ci, rank });
    }
    let condition = smax / smin;
    if condition > COND_CAP {
        return Err(RecoveryError::IllConditioned { cell: ci, condition });
    }

    // Linear map from all neighbourhood fluxes to the eleven coefficients.
    let fmap = if mech.equality {
        // Null-space method: own faces exact, the rest in least squares within
        // the freedom the own faces leave. With P = I − A⁺A the projector onto
        // null(A), N (R N)⁺ = (R P)⁺ for any orthonormal basis N of null(A).
        let ap = pinv(&am);
        let p = DMatrix::<f64>::identity(11, 11) - &ap * &am;
        let rpp = pinv(&(&rm * &p));
        let left = &ap - &rpp * &rm * &ap;
        let mut f = DMatrix::zeros(11, all_rows.len());
        f.columns_mut(0, own_rows.len()).copy_from(&left);
        f.columns_mut(own_rows.len(), rest_rows.len()).copy_from(&rpp);
        f
    } else {
        pinv(&rows)
    };
    // z = J ξ + t0, ξ the barycentric λ₁..λ₃ of the cell's own vertices (as pg.chart).
    let k = nb[0].1;
    let orth = |p: Vector4<f64>| Vector3::new(ch.q[0].dot(&p), ch.q[1].dot(&p), ch.q[2].dot(&p)) / ch.diam;
    let chart_j = Matrix3::from_columns(&[orth(k[1] - k[0]), orth(k[2] - k[0]), orth(k[3] - k[0])]);
    let chart_t0 = orth(k[0] - ch.cen);
    Ok(Fit { own: own_rows.len(), nb, faces: read, am, rows, fmap, condition, rank, chart_j, chart_t0 })
}

/// Recover every cell of a closed mesh from a cochain of edge integrals.
pub fn recover_curvature(
    cells: &[Cell],
    a: &[f64],
    rings: usize,
) -> Result<Vec<CellRecovery>, RecoveryError> {
    recover_curvature_with(cells, a, rings, Mechanism::FULL)
}

/// [`recover_curvature`] with explicit mechanism switches (tests).
pub fn recover_curvature_with(
    cells: &[Cell],
    a: &[f64],
    rings: usize,
    mech: Mechanism,
) -> Result<Vec<CellRecovery>, RecoveryError> {
    if rings == 0 || rings > MAX_RINGS {
        return Err(RecoveryError::InvalidRings(rings));
    }
    for (ci, c) in cells.iter().enumerate() {
        if let Some(&e) = c.edges.iter().find(|&&e| e >= a.len()) {
            return Err(RecoveryError::EdgeOutOfRange { cell: ci, edge: e, n_edges: a.len() });
        }
    }
    let faces = face_index(cells)?;
    (0..cells.len())
        .map(|ci| recover_cell(cells, &faces, a, ci, rings, mech, None))
        .collect()
}

/// Face index for callers that drive [`recover_cell`] directly (tests).
pub fn build_face_index(cells: &[Cell]) -> Result<FaceIndex, RecoveryError> {
    face_index(cells)
}

// ── SU(2): the covariant reading ──────────────────────────────────────────
//
// Engine form of scripts/ricci_su2_recovery.py. The fit map is the U(1) one,
// unchanged; what is new is what it is fed:
//
//   face flux   X_f = log(U_ij · U_jk · U_ki)      holonomy at the face's base vertex i
//   register    Y_f = W_i X_f W_i⁻¹                 W_i: transport from i to the cell's vertex 0
//   fit         C (3 × 11) = Y (3 × faces) · F_Kᵀ   one closed linear field per su(2) component
//
// Group product and inverse are the repository's stored-link convention
// (ricci_consistency_math.compose / inverse): a stored link on edge tail → head
// maps the fibre at head to the fibre at tail, and becomes g_tail U g_head⁻¹
// under a gauge change g.

/// Mechanism switches for the SU(2) reading (production: [`Su2Mechanism::FULL`]).
#[derive(Clone, Copy, Debug)]
pub struct Su2Mechanism {
    /// Carry every face flux to the cell's vertex-0 frame before fitting.
    pub register: bool,
    /// Replace log(holonomy) by the sum of the three edge logarithms.
    pub abelianised: bool,
    /// What a face contributes: log(holonomy) (the recovery, S1), or the
    /// holonomy's vector part, the native action's face defect (the force, S2a).
    pub flux: Su2Flux,
}

/// The face quantity the SU(2) reading fits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Su2Flux {
    /// log(U_ij U_jk U_ki): the curvature reading (RECOVER_CURVATURE GROUP SU2).
    Log,
    /// vec(U_ij U_jk U_ki): the face defect, as the two-ring force reads it.
    Defect,
}

impl Su2Mechanism {
    pub const FULL: Su2Mechanism = Su2Mechanism { register: true, abelianised: false, flux: Su2Flux::Log };
}

/// The covariant curvature of one cell: one closed linear field per su(2)
/// component, in the cell's chart and in the fibre frame at the cell's vertex 0.
#[derive(Clone, Debug)]
pub struct Su2CellRecovery {
    /// `coefficients[a][k]`: su(2) component `a`, basis coefficient `k`.
    pub coefficients: [[f64; 11]; 3],
    pub amplification: f64,
    pub condition: f64,
    pub rank: usize,
    pub faces: usize,
    pub cells: usize,
    /// Largest |recovered − registered flux| over the cell's own faces, all
    /// components. Zero to rounding for commuting links; for non-commuting links
    /// it measures the non-Abelian Stokes remainder (reported, never refused).
    pub own_residual: f64,
    /// Global vertex id whose fibre frame the coefficients are expressed in.
    pub frame_vertex: usize,
    /// The cell's chart as an affine map from reference coordinates ξ (the
    /// barycentric λ₁..λ₃) to chart coordinates: z = J ξ + t0.
    pub chart_j: Matrix3<f64>,
    pub chart_t0: Vector3<f64>,
}

const ONE: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

fn q4(q: [f64; 4]) -> Vector4<f64> {
    Vector4::new(q[0], q[1], q[2], q[3])
}

/// Link product, stored-link convention: (a0 b0 − a·b, a0 b + b0 a − a × b).
fn lmul(a: &Vector4<f64>, b: &Vector4<f64>) -> Vector4<f64> {
    let (av, bv) = (Vector3::new(a[1], a[2], a[3]), Vector3::new(b[1], b[2], b[3]));
    let s = a[0] * b[0] - av.dot(&bv);
    let v = bv * a[0] + av * b[0] - av.cross(&bv);
    Vector4::new(s, v[0], v[1], v[2])
}

fn linv(q: &Vector4<f64>) -> Vector4<f64> {
    Vector4::new(q[0], -q[1], -q[2], -q[3])
}

/// log of a unit quaternion, as its imaginary 3-vector; `None` near −1.
fn qlog(q: &Vector4<f64>) -> Option<Vector3<f64>> {
    let v = Vector3::new(q[1], q[2], q[3]);
    let s = v.norm();
    if q[0] < -1.0 + 1e-6 && s < 1e-3 {
        return None;
    }
    if s < 1e-300 {
        return Some(Vector3::zeros());
    }
    Some(v * (s.atan2(q[0]) / s))
}

/// Ad_g x = g x g⁻¹ for an imaginary quaternion x.
fn ad(g: &Vector4<f64>, x: &Vector3<f64>) -> Vector3<f64> {
    let r = lmul(&lmul(g, &Vector4::new(0.0, x[0], x[1], x[2])), &linv(g));
    Vector3::new(r[1], r[2], r[3])
}

/// Global vertex ids on the quotient, by union-find over shared stored edges:
/// the tail (head) of a stored edge is one vertex in every cell using it. Ids
/// are numbered by first appearance scanning cells, then local vertices
/// (the research script's numbering, which fixes the transport tree).
pub fn vertex_ids(cells: &[Cell]) -> Vec<[usize; 4]> {
    let n = 4 * cells.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut [usize], mut x: usize) -> usize {
        while parent[x] != x {
            parent[x] = parent[parent[x]];
            x = parent[x];
        }
        x
    }
    let mut ends: HashMap<usize, (usize, usize)> = HashMap::new();
    for (ci, c) in cells.iter().enumerate() {
        for (e, &(i, j)) in EDGES.iter().enumerate() {
            let (tail, head) = if c.reverse[e] { (4 * ci + j, 4 * ci + i) } else { (4 * ci + i, 4 * ci + j) };
            match ends.get(&c.edges[e]).copied() {
                Some((t0, h0)) => {
                    for (a, b) in [(t0, tail), (h0, head)] {
                        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                        if ra != rb {
                            parent[ra.max(rb)] = ra.min(rb);
                        }
                    }
                }
                None => {
                    ends.insert(c.edges[e], (tail, head));
                }
            }
        }
    }
    let mut ids: HashMap<usize, usize> = HashMap::new();
    let mut out = vec![[0usize; 4]; cells.len()];
    for ci in 0..cells.len() {
        for v in 0..4 {
            let r = find(&mut parent, 4 * ci + v);
            let next = ids.len();
            out[ci][v] = *ids.entry(r).or_insert(next);
        }
    }
    out
}

/// The link for local edge i → j of a cell: maps the fibre at j to the fibre at i.
fn local_link(cell: &Cell, links: &[[f64; 4]], i: usize, j: usize) -> Vector4<f64> {
    let e = edge_index(i, j);
    let mut u = q4(links[cell.edges[e]]);
    if cell.reverse[e] {
        u = linv(&u);
    }
    if i < j { u } else { linv(&u) }
}

/// The link-independent part of one cell's SU(2) reading: its neighbourhood,
/// chart and fit map (geometry only). Built once, read with many link sets
/// (the Jacobian's columns).
pub struct Su2Prepared {
    fit: Fit,
    amplification: f64,
    ci: usize,
}

impl Su2Prepared {
    /// Cells in the neighbourhood (the cell itself first).
    pub fn neighbourhood(&self) -> Vec<usize> {
        self.fit.nb.iter().map(|(nj, _)| *nj).collect()
    }
}

/// Prepare one cell's reading (geometry only).
pub fn prepare_su2_cell(
    cells: &[Cell],
    faces: &FaceIndex,
    ci: usize,
    rings: usize,
    mech: Mechanism,
) -> Result<Su2Prepared, RecoveryError> {
    let fit = fit_map(cells, faces, ci, rings, mech)?;
    let amplification = singular_values(&fit.fmap).into_iter().fold(0.0_f64, f64::max);
    Ok(Su2Prepared { fit, amplification, ci })
}

/// Recover one cell from SU(2) links.
#[allow(clippy::too_many_arguments)]
pub fn recover_cell_su2(
    cells: &[Cell],
    faces: &FaceIndex,
    vid: &[[usize; 4]],
    links: &[[f64; 4]],
    ci: usize,
    rings: usize,
    mech: Mechanism,
    su: Su2Mechanism,
) -> Result<Su2CellRecovery, RecoveryError> {
    let prep = prepare_su2_cell(cells, faces, ci, rings, mech)?;
    read_prepared_su2(&prep, cells, vid, links, su)
}

/// Read a prepared cell with a set of links.
pub fn read_prepared_su2(
    prep: &Su2Prepared,
    cells: &[Cell],
    vid: &[[usize; 4]],
    links: &[[f64; 4]],
    su: Su2Mechanism,
) -> Result<Su2CellRecovery, RecoveryError> {
    let fit = &prep.fit;
    let ci = prep.ci;

    // Transport W[v]: fibre at v → fibre at the cell's vertex 0, along a
    // breadth-first tree over the neighbourhood's edges (sorted adjacency).
    let mut adj: HashMap<usize, std::collections::BTreeMap<usize, Vector4<f64>>> = HashMap::new();
    for (nj, _) in &fit.nb {
        for &(i, j) in EDGES.iter() {
            let (a, b) = (vid[*nj][i], vid[*nj][j]);
            let u = local_link(&cells[*nj], links, i, j);
            adj.entry(a).or_default().insert(b, u);
            adj.entry(b).or_default().insert(a, linv(&u));
        }
    }
    let root = vid[ci][0];
    let mut w: HashMap<usize, Vector4<f64>> = HashMap::from([(root, q4(ONE))]);
    let mut queue = std::collections::VecDeque::from([root]);
    while let Some(u) = queue.pop_front() {
        let wu = w[&u];
        for (&v, link) in &adj[&u] {
            if let std::collections::hash_map::Entry::Vacant(slot) = w.entry(v) {
                slot.insert(lmul(&wu, link));
                queue.push_back(v);
            }
        }
    }

    let mut y = DMatrix::<f64>::zeros(fit.faces.len(), 3);
    for (row, (nj, (i, j, k), _)) in fit.faces.iter().enumerate() {
        let c = &cells[*nj];
        let (uij, ujk, uki) = (local_link(c, links, *i, *j), local_link(c, links, *j, *k), local_link(c, links, *k, *i));
        let near = RecoveryError::HolonomyNearMinusOne { cell: ci, face_cell: *nj };
        let x = if su.abelianised {
            let l = |q: &Vector4<f64>| qlog(q).ok_or_else(|| near.clone());
            l(&uij)? + l(&ujk)? + l(&uki)?
        } else {
            let h = lmul(&lmul(&uij, &ujk), &uki);
            match su.flux {
                Su2Flux::Log => qlog(&h).ok_or(near)?,
                Su2Flux::Defect => Vector3::new(h[1], h[2], h[3]),
            }
        };
        let x = if su.register { ad(&w[&vid[*nj][*i]], &x) } else { x };
        for comp in 0..3 {
            y[(row, comp)] = x[comp];
        }
    }

    let coef = &fit.fmap * &y; // 11 × 3
    // Not a refusal, unlike U(1): the four registered own fluxes of a cell sum to
    // zero only for commuting links. With non-commuting links the non-Abelian
    // Stokes remainder (commutator terms) is a genuine inconsistency, which a
    // closed field cannot match exactly. It is reported, as the research script
    // implies by never checking it.
    let own_res = (&fit.am * &coef - y.rows(0, fit.own)).amax();
    Ok(Su2CellRecovery {
        coefficients: std::array::from_fn(|a| std::array::from_fn(|k| coef[(k, a)])),
        amplification: prep.amplification,
        condition: fit.condition,
        rank: fit.rank,
        faces: fit.faces.len(),
        cells: fit.nb.len(),
        own_residual: own_res,
        frame_vertex: root,
        chart_j: fit.chart_j,
        chart_t0: fit.chart_t0,
    })
}

/// Recover every cell of a closed mesh from SU(2) links (one unit quaternion
/// per edge, scalar first, stored-link convention).
pub fn recover_curvature_su2(
    cells: &[Cell],
    links: &[[f64; 4]],
    rings: usize,
) -> Result<Vec<Su2CellRecovery>, RecoveryError> {
    recover_curvature_su2_with(cells, links, rings, Mechanism::FULL, Su2Mechanism::FULL)
}

/// [`recover_curvature_su2`] with explicit mechanism switches (tests).
pub fn recover_curvature_su2_with(
    cells: &[Cell],
    links: &[[f64; 4]],
    rings: usize,
    mech: Mechanism,
    su: Su2Mechanism,
) -> Result<Vec<Su2CellRecovery>, RecoveryError> {
    if rings == 0 || rings > MAX_RINGS {
        return Err(RecoveryError::InvalidRings(rings));
    }
    for (ci, c) in cells.iter().enumerate() {
        if let Some(&e) = c.edges.iter().find(|&&e| e >= links.len()) {
            return Err(RecoveryError::EdgeOutOfRange { cell: ci, edge: e, n_edges: links.len() });
        }
    }
    let needed = 1 + cells.iter().flat_map(|c| c.edges).max().unwrap_or(0);
    if links.len() != needed {
        return Err(RecoveryError::LinkCountMismatch { links: links.len(), edges_needed: needed });
    }
    for (e, q) in links.iter().enumerate() {
        let deviation = (q4(*q).norm() - 1.0).abs();
        if !(deviation <= UNIT_TOL) {
            return Err(RecoveryError::NonUnitLink { edge: e, deviation });
        }
    }
    let faces = face_index(cells)?;
    let vid = vertex_ids(cells);
    (0..cells.len())
        .map(|ci| recover_cell_su2(cells, &faces, &vid, links, ci, rings, mech, su))
        .collect()
}

// ── engine surface ────────────────────────────────────────────────────────

/// Why a bundle-level request could not be answered.
#[derive(Clone, Debug, PartialEq)]
pub enum BundleRecoveryError {
    /// A named bundle does not exist (HTTP 404).
    NotFound(String),
    /// The bundles exist but cannot be honestly read as a mesh and a cochain,
    /// or the recovery refused (HTTP 422).
    Refused(String),
}

impl fmt::Display for BundleRecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BundleRecoveryError::NotFound(m) | BundleRecoveryError::Refused(m) => f.write_str(m),
        }
    }
}

fn refuse(msg: String) -> BundleRecoveryError {
    BundleRecoveryError::Refused(format!("RECOVER_CURVATURE: {msg}"))
}

/// Read a field that must hold a whole number in `0..limit`.
fn index_field(rec: &crate::types::Record, field: &str, what: &str, limit: usize) -> Result<usize, BundleRecoveryError> {
    let v = rec
        .get(field)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| refuse(format!("{what} record lacks numeric field '{field}'")))?;
    if v.fract() != 0.0 || v < 0.0 || v >= limit as f64 {
        return Err(refuse(format!("{what} field '{field}' = {v} is not a whole number in 0..{limit}")));
    }
    Ok(v as usize)
}

fn number_field(rec: &crate::types::Record, field: &str, what: &str) -> Result<f64, BundleRecoveryError> {
    rec.get(field)
        .and_then(|v| v.as_f64())
        .filter(|v| v.is_finite())
        .ok_or_else(|| refuse(format!("{what} record lacks a finite numeric field '{field}'")))
}

/// Read the mesh from `cells_bundle` and the cochain `field` from
/// `edges_bundle`, then recover. Bundle shapes: docs/RECOVER_CURVATURE_SPEC.md.
pub fn recover_curvature_from_bundles(
    engine: &crate::engine::Engine,
    cells_bundle: &str,
    edges_bundle: &str,
    field: &str,
    rings: usize,
) -> Result<Vec<CellRecovery>, BundleRecoveryError> {
    let cstore = engine
        .bundle(cells_bundle)
        .ok_or_else(|| BundleRecoveryError::NotFound(format!("Bundle '{cells_bundle}' not found")))?;
    let estore = engine
        .bundle(edges_bundle)
        .ok_or_else(|| BundleRecoveryError::NotFound(format!("Bundle '{edges_bundle}' not found")))?;

    let erecs: Vec<crate::types::Record> = estore.records().collect();
    if erecs.is_empty() {
        return Err(refuse(format!("edge bundle '{edges_bundle}' is empty")));
    }
    let n_edges = erecs.len();
    let mut a = vec![f64::NAN; n_edges];
    for r in &erecs {
        let id = index_field(r, "edge", "edge", n_edges)?;
        if !a[id].is_nan() {
            return Err(refuse(format!("edge id {id} appears more than once in '{edges_bundle}'")));
        }
        a[id] = number_field(r, field, "edge")?;
    }

    let crecs: Vec<crate::types::Record> = cstore.records().collect();
    if crecs.is_empty() {
        return Err(refuse(format!("cell bundle '{cells_bundle}' is empty")));
    }
    let n_cells = crecs.len();
    let mut slots: Vec<Option<Cell>> = vec![None; n_cells];
    for r in &crecs {
        let id = index_field(r, "cell", "cell", n_cells)?;
        if slots[id].is_some() {
            return Err(refuse(format!("cell id {id} appears more than once in '{cells_bundle}'")));
        }
        let mut vertices = [[0.0; 4]; 4];
        for (i, vtx) in vertices.iter_mut().enumerate() {
            for (k, comp) in vtx.iter_mut().enumerate() {
                *comp = number_field(r, &format!("v{i}_{k}"), "cell")?;
            }
        }
        let mut edges = [0usize; 6];
        let mut reverse = [false; 6];
        for e in 0..6 {
            edges[e] = index_field(r, &format!("e{e}"), "cell", n_edges)?;
            let flag = number_field(r, &format!("r{e}"), "cell")?;
            reverse[e] = match flag {
                x if x == 0.0 => false,
                x if x == 1.0 => true,
                x => return Err(refuse(format!("cell field 'r{e}' = {x} must be 0 or 1"))),
            };
        }
        slots[id] = Some(Cell { vertices, edges, reverse });
    }
    let cells: Vec<Cell> = slots.into_iter().map(|c| c.expect("every id 0..N-1 filled once")).collect();
    recover_curvature(&cells, &a, rings).map_err(|e| BundleRecoveryError::Refused(e.to_string()))
}

/// Read SU(2) links from `links_bundle`: one record per edge, numeric `edge` =
/// 0..E-1 and four numeric fields `{q}0 … {q}3` (unit quaternion, scalar first,
/// stored-link convention). (YM5, docs/YANG_MILLS_ENGINE_SPEC.md.)
pub fn read_links_bundle(
    engine: &crate::engine::Engine,
    links_bundle: &str,
    q: &str,
) -> Result<Vec<[f64; 4]>, BundleRecoveryError> {
    let store = engine
        .bundle(links_bundle)
        .ok_or_else(|| BundleRecoveryError::NotFound(format!("Bundle '{links_bundle}' not found")))?;
    let recs: Vec<crate::types::Record> = store.records().collect();
    if recs.is_empty() {
        return Err(refuse(format!("link bundle '{links_bundle}' is empty")));
    }
    let n = recs.len();
    let mut out: Vec<Option<[f64; 4]>> = vec![None; n];
    for r in &recs {
        let id = index_field(r, "edge", "link", n)?;
        if out[id].is_some() {
            return Err(refuse(format!("edge id {id} appears more than once in '{links_bundle}'")));
        }
        let mut quat = [0.0; 4];
        for (k, x) in quat.iter_mut().enumerate() {
            *x = number_field(r, &format!("{q}{k}"), "link")?;
        }
        out[id] = Some(quat);
    }
    Ok(out.into_iter().map(|x| x.expect("every id 0..E-1 filled once")).collect())
}

/// Read the mesh from `cells_bundle` (shape as RECOVER_CURVATURE), and with
/// `weights` also the numeric field `weight` of every cell.
pub fn read_cells_bundle(
    engine: &crate::engine::Engine,
    cells_bundle: &str,
    n_edges: usize,
    weights: bool,
) -> Result<(Vec<Cell>, Vec<f64>), BundleRecoveryError> {
    let cstore = engine
        .bundle(cells_bundle)
        .ok_or_else(|| BundleRecoveryError::NotFound(format!("Bundle '{cells_bundle}' not found")))?;
    let crecs: Vec<crate::types::Record> = cstore.records().collect();
    if crecs.is_empty() {
        return Err(refuse(format!("cell bundle '{cells_bundle}' is empty")));
    }
    let n_cells = crecs.len();
    let mut slots: Vec<Option<(Cell, f64)>> = vec![None; n_cells];
    for r in &crecs {
        let id = index_field(r, "cell", "cell", n_cells)?;
        if slots[id].is_some() {
            return Err(refuse(format!("cell id {id} appears more than once in '{cells_bundle}'")));
        }
        let mut vertices = [[0.0; 4]; 4];
        for (i, vtx) in vertices.iter_mut().enumerate() {
            for (k, comp) in vtx.iter_mut().enumerate() {
                *comp = number_field(r, &format!("v{i}_{k}"), "cell")?;
            }
        }
        let mut edges = [0usize; 6];
        let mut reverse = [false; 6];
        for e in 0..6 {
            edges[e] = index_field(r, &format!("e{e}"), "cell", n_edges)?;
            let flag = number_field(r, &format!("r{e}"), "cell")?;
            reverse[e] = match flag {
                x if x == 0.0 => false,
                x if x == 1.0 => true,
                x => return Err(refuse(format!("cell field 'r{e}' = {x} must be 0 or 1"))),
            };
        }
        let w = if weights { number_field(r, "weight", "cell")? } else { 1.0 };
        slots[id] = Some((Cell { vertices, edges, reverse }, w));
    }
    Ok(slots.into_iter().map(|c| c.expect("every id 0..N-1 filled once")).unzip())
}

/// RECOVER_CURVATURE … GROUP SU2 over bundles.
pub fn recover_curvature_su2_from_bundles(
    engine: &crate::engine::Engine,
    cells_bundle: &str,
    links_bundle: &str,
    q: &str,
    rings: usize,
) -> Result<Vec<Su2CellRecovery>, BundleRecoveryError> {
    let links = read_links_bundle(engine, links_bundle, q)?;
    let (cells, _) = read_cells_bundle(engine, cells_bundle, links.len(), false)?;
    recover_curvature_su2(&cells, &links, rings).map_err(|e| BundleRecoveryError::Refused(e.to_string()))
}

/// LINK_FORCE over bundles. A missing metric refuses (BW08): none is ever substituted.
pub fn link_force_from_bundles(
    engine: &crate::engine::Engine,
    cells_bundle: &str,
    links_bundle: &str,
    q: &str,
    metric: Option<[f64; 3]>,
    order: usize,
    method: super::ricci_pg_force::ForceMethod,
) -> Result<super::ricci_pg_force::LinkForce, BundleRecoveryError> {
    let metric = metric.ok_or_else(|| {
        BundleRecoveryError::Refused("LINK_FORCE: METRIC (g1, g2, g3) is required; none is ever substituted".into())
    })?;
    let links = read_links_bundle(engine, links_bundle, q)?;
    let (cells, weights) = read_cells_bundle(engine, cells_bundle, links.len(), true)?;
    super::ricci_pg_force::link_force(&cells, &weights, &links, metric, order, method)
        .map_err(BundleRecoveryError::Refused)
}
