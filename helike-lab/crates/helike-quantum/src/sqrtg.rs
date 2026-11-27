//! Flux-method §5.11, the unfolded chart (docs/research/helike-flux-method-amendment-2026-11-25.md).
//!
//! The reduced field space is flat R^9 (the 3x3 matrix q) folded by colour. Write q = P O with P symmetric and O in
//! SO(3); then G = q q^T = P^2. The trial space is built in P, the tube's cross-section around the colour ring, where
//! the vacuum is nearly Gaussian:
//!   coordinates   P = mu + A zeta (six entries in the order 00, 11, 22, 01, 02, 12)
//!   measure       dmu = det(G)^{-1/2} dG  =>  rho_P = prod_{i<j} |l_i + l_j|  (constants cancel)
//!   kinetic       1/2 A(x)_{ab} d_a f d_b f in the Gram entries x  =>  g_zeta = A^{-1} J A(x) J^T A^{-T}, J = dP/dx
//!   potential     the physical V(q) at q = P (signed on both sheets of the fold)
//!   basis         phi_s(zeta) = prod_j h_{s_j}(zeta_j) exp(-|zeta|^2/2), total degree <= Mq (not orthonormal in dmu)
//!   problem       H c = E S c, assembled densely by Gauss-Hermite quadrature in zeta (weight e^{-|zeta|^2})
//! The integrands are rational (rho g = N_6 / w, w the orbit volume), smooth near the vacuum; the order seam decides.
//! The frame (mu, A) is made self-consistent from the ground state's mean and covariance of P (as in §5.10).

use crate::basis::{states_up_to, State};
use crate::hermite;
use crate::model::{a_at, eigh6, inv6, Model, M6};
use faer::linalg::matmul::matmul;
use faer::linalg::triangular_solve::{solve_lower_triangular_in_place, solve_upper_triangular_in_place};
use faer::{Accum, Mat, Par, Side};
use rayon::prelude::*;
use serde::Serialize;

pub const KEYS: [(usize, usize); 6] = [(0, 0), (1, 1), (2, 2), (0, 1), (0, 2), (1, 2)];

/// Test-only mechanism removals (HELIKE_SQRTG_MUTANT): no_rho, j_not_inverted, face_unchecked. Never set in a
/// registered run; the CLI refuses to write a receipt without recording it.
pub fn mutant() -> String {
    std::env::var("HELIKE_SQRTG_MUTANT").unwrap_or_default()
}

#[derive(Clone, Debug, Serialize)]
pub struct Frame {
    pub mu: [f64; 6],
    /// P = mu + A zeta; A = (2 C)^{1/2} for a vacuum covariance C of P
    pub a: M6,
}

impl Frame {
    pub fn from_covariance(mu: [f64; 6], c: &M6) -> Self {
        let mut twoc = *c;
        for r in twoc.iter_mut() {
            for v in r.iter_mut() {
                *v *= 2.0;
            }
        }
        let (e, u) = eigh6(&twoc);
        let mut a = [[0.0; 6]; 6];
        for i in 0..6 {
            for j in 0..6 {
                a[i][j] = (0..6).map(|k| u[i][k] * e[k].max(0.0).sqrt() * u[j][k]).sum();
            }
        }
        Frame { mu, a }
    }
}

fn sym3(p: &[f64; 6]) -> [[f64; 3]; 3] {
    let mut m = [[0.0; 3]; 3];
    for (k, &(i, j)) in KEYS.iter().enumerate() {
        m[i][j] = p[k];
        m[j][i] = p[k];
    }
    m
}

fn mul3(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut c = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            c[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    c
}

fn vec6(m: &[[f64; 3]; 3]) -> [f64; 6] {
    std::array::from_fn(|k| m[KEYS[k].0][KEYS[k].1])
}

/// Eigenvalues of a symmetric 3x3 matrix (closed form, trigonometric).
fn eig3(m: &[[f64; 3]; 3]) -> [f64; 3] {
    let p1 = m[0][1] * m[0][1] + m[0][2] * m[0][2] + m[1][2] * m[1][2];
    let q = (m[0][0] + m[1][1] + m[2][2]) / 3.0;
    if p1 == 0.0 {
        return [m[0][0], m[1][1], m[2][2]];
    }
    let p2 = (m[0][0] - q).powi(2) + (m[1][1] - q).powi(2) + (m[2][2] - q).powi(2) + 2.0 * p1;
    let p = (p2 / 6.0).sqrt();
    let b: [[f64; 3]; 3] = std::array::from_fn(|i| std::array::from_fn(|j| (m[i][j] - if i == j { q } else { 0.0 }) / p));
    let r = (b[0][0] * (b[1][1] * b[2][2] - b[1][2] * b[2][1]) - b[0][1] * (b[1][0] * b[2][2] - b[1][2] * b[2][0])
        + b[0][2] * (b[1][0] * b[2][1] - b[1][1] * b[2][0]))
        / 2.0;
    let phi = r.clamp(-1.0, 1.0).acos() / 3.0;
    let e1 = q + 2.0 * p * phi.cos();
    let e3 = q + 2.0 * p * (phi + 2.0 * std::f64::consts::PI / 3.0).cos();
    [e1, 3.0 * q - e1 - e3, e3]
}

pub struct NodeData {
    pub rho: f64,
    /// symmetric square root of the cometric in zeta
    pub gh: M6,
    pub v: f64,
    /// an eigenvalue of P is non-positive (the vacuum's sheet has all positive)
    pub off_sheet: bool,
}

pub fn node_data(zeta: &[f64; 6], frame: &Frame, ai: &M6, model: &Model) -> NodeData {
    let p: [f64; 6] = std::array::from_fn(|i| frame.mu[i] + (0..6).map(|j| frame.a[i][j] * zeta[j]).sum::<f64>());
    let pm = sym3(&p);
    let l = eig3(&pm);
    let mt = mutant();
    let rho = if mt == "no_rho" { 1.0 } else { ((l[0] + l[1]) * (l[0] + l[2]) * (l[1] + l[2])).abs() };
    let g = mul3(&pm, &pm);
    // dx/dP: column k is the Gram-entry change for the unit symmetric direction E_k (dG = P E + E P)
    let mut dxdp = [[0.0; 6]; 6];
    for (k, &(i, j)) in KEYS.iter().enumerate() {
        let mut e = [[0.0; 3]; 3];
        e[i][j] = 1.0;
        e[j][i] = 1.0;
        let pe = mul3(&pm, &e);
        let ep = mul3(&e, &pm);
        let d: [[f64; 3]; 3] = std::array::from_fn(|r| std::array::from_fn(|c| pe[r][c] + ep[r][c]));
        let col = vec6(&d);
        for r in 0..6 {
            dxdp[r][k] = col[r];
        }
    }
    let jm = if mt == "j_not_inverted" { dxdp } else { inv6(&dxdp) };
    let ax = a_at(&vec6(&g));
    // g_zeta = Ai J A J^T Ai^T
    let aij: M6 = std::array::from_fn(|r| std::array::from_fn(|c| (0..6).map(|k| ai[r][k] * jm[k][c]).sum()));
    let t1: M6 = std::array::from_fn(|r| std::array::from_fn(|c| (0..6).map(|k| aij[r][k] * ax[k][c]).sum()));
    let gz: M6 = std::array::from_fn(|r| std::array::from_fn(|c| (0..6).map(|k| t1[r][k] * aij[c][k]).sum()));
    let (e, u) = eigh6(&gz);
    let gh: M6 = std::array::from_fn(|r| std::array::from_fn(|c| (0..6).map(|k| u[r][k] * e[k].max(0.0).sqrt() * u[c][k]).sum()));
    NodeData { rho, gh, v: model.potential(&pm), off_sheet: mt != "face_unchecked" && l.iter().any(|&x| x <= 0.0) }
}

pub struct Assembled {
    pub h: Mat<f64>,
    pub s: Mat<f64>,
    pub states: Vec<State>,
}

/// Dense H and S by product Gauss-Hermite quadrature of the given order, parallel over node chunks.
pub fn assemble(model: &Model, frame: &Frame, mq: usize, order: usize) -> Assembled {
    let states = states_up_to(mq);
    let n = states.len();
    let (x, w) = hermite::gauss(order);
    let ai = inv6(&frame.a);
    let total = order.pow(6);
    let chunk = 512usize;
    let starts: Vec<usize> = (0..total).step_by(chunk).collect();
    let (h, s) = starts
        .par_iter()
        .fold(
            || (Mat::<f64>::zeros(n, n), Mat::<f64>::zeros(n, n)),
            |(mut hacc, mut sacc), &st| {
                let m = chunk.min(total - st);
                let mut b = Mat::<f64>::zeros(m, n);
                let mut q: Vec<Mat<f64>> = (0..6).map(|_| Mat::<f64>::zeros(m, n)).collect();
                let mut wr = vec![0.0; m];
                let mut wv = vec![0.0; m];
                let mut ghs = Vec::with_capacity(m);
                for r in 0..m {
                    let mut idx = st + r;
                    let mut zeta = [0.0; 6];
                    let mut wt = 1.0;
                    for ax in (0..6).rev() {
                        let k = idx % order;
                        idx /= order;
                        zeta[ax] = x[k];
                        wt *= w[k];
                    }
                    let nd = node_data(&zeta, frame, &ai, model);
                    wr[r] = wt * nd.rho;
                    wv[r] = wt * nd.rho * nd.v;
                    ghs.push(nd.gh);
                    let hv: Vec<Vec<f64>> = (0..6).map(|ax| hermite::normalized(zeta[ax], mq)).collect();
                    let dh: Vec<Vec<f64>> = (0..6)
                        .map(|ax| (0..=mq).map(|nn| if nn == 0 { 0.0 } else { (2.0 * nn as f64).sqrt() * hv[ax][nn - 1] }).collect())
                        .collect();
                    for (c, stt) in states.iter().enumerate() {
                        let vals: [f64; 6] = std::array::from_fn(|ax| hv[ax][stt[ax] as usize]);
                        let prod: f64 = vals.iter().product();
                        b[(r, c)] = prod;
                        for j in 0..6 {
                            let mut d = dh[j][stt[j] as usize];
                            for ax in 0..6 {
                                if ax != j {
                                    d *= vals[ax];
                                }
                            }
                            q[j][(r, c)] = d - zeta[j] * prod;
                        }
                    }
                }
                // S += B^T diag(w rho) B,  H += B^T diag(w rho V) B + 1/2 sum_k R_k^T diag(w rho) R_k
                let bw = Mat::<f64>::from_fn(m, n, |r, c| wr[r] * b[(r, c)]);
                let bv = Mat::<f64>::from_fn(m, n, |r, c| wv[r] * b[(r, c)]);
                matmul(sacc.as_mut(), Accum::Add, b.transpose(), bw.as_ref(), 1.0, Par::Seq);
                matmul(hacc.as_mut(), Accum::Add, b.transpose(), bv.as_ref(), 1.0, Par::Seq);
                for k in 0..6 {
                    let rk = Mat::<f64>::from_fn(m, n, |r, c| (0..6).map(|j| ghs[r][k][j] * q[j][(r, c)]).sum::<f64>());
                    let rkw = Mat::<f64>::from_fn(m, n, |r, c| wr[r] * rk[(r, c)]);
                    matmul(hacc.as_mut(), Accum::Add, rk.transpose(), rkw.as_ref(), 0.5, Par::Seq);
                }
                (hacc, sacc)
            },
        )
        .reduce(|| (Mat::<f64>::zeros(n, n), Mat::<f64>::zeros(n, n)), |a, b| (a.0 + b.0, a.1 + b.1));
    Assembled { h, s, states }
}

/// The lowest `nev` generalized eigenpairs: (values ascending, coefficient columns).
pub fn solve(a: &Assembled, nev: usize) -> Result<(Vec<f64>, Mat<f64>), String> {
    let n = a.s.nrows();
    let ssym = Mat::<f64>::from_fn(n, n, |i, j| 0.5 * (a.s[(i, j)] + a.s[(j, i)]));
    let llt = ssym.llt(Side::Lower).map_err(|e| format!("S is not positive definite: {e:?}"))?;
    let l = llt.L();
    let mut y = Mat::<f64>::from_fn(n, n, |i, j| 0.5 * (a.h[(i, j)] + a.h[(j, i)]));
    solve_lower_triangular_in_place(l, y.as_mut(), Par::rayon(0));
    let mut z = y.transpose().to_owned();
    solve_lower_triangular_in_place(l, z.as_mut(), Par::rayon(0));
    let zs = Mat::<f64>::from_fn(n, n, |i, j| 0.5 * (z[(i, j)] + z[(j, i)]));
    let eig = zs.self_adjoint_eigen(Side::Lower).map_err(|e| format!("eigensolve: {e:?}"))?;
    let vals: Vec<f64> = (0..nev).map(|k| eig.S().column_vector()[k]).collect();
    let mut c = Mat::<f64>::from_fn(n, nev, |i, k| eig.U()[(i, k)]);
    solve_upper_triangular_in_place(l.transpose(), c.as_mut(), Par::rayon(0));
    Ok((vals, c))
}

/// Mean and covariance of P in psi0^2 dmu, and the vacuum weight on nodes off the vacuum's sheet.
pub fn ground_moments(model: &Model, frame: &Frame, states: &[State], c0: &[f64], mq: usize, order: usize)
    -> ([f64; 6], M6, f64) {
    let (x, w) = hermite::gauss(order);
    let ai = inv6(&frame.a);
    let total = order.pow(6);
    let (m0, m1, m2, off) = (0..total)
        .into_par_iter()
        .fold(
            || (0.0f64, [0.0f64; 6], [[0.0f64; 6]; 6], 0.0f64),
            |(mut m0, mut m1, mut m2, mut off), node| {
                let mut idx = node;
                let mut zeta = [0.0; 6];
                let mut wt = 1.0;
                for ax in (0..6).rev() {
                    let k = idx % order;
                    idx /= order;
                    zeta[ax] = x[k];
                    wt *= w[k];
                }
                let nd = node_data(&zeta, frame, &ai, model);
                let hv: Vec<Vec<f64>> = (0..6).map(|ax| hermite::normalized(zeta[ax], mq)).collect();
                let f: f64 = states.iter().zip(c0).map(|(s, &cc)| cc * (0..6).map(|ax| hv[ax][s[ax] as usize]).product::<f64>()).sum();
                let wt = wt * nd.rho * f * f;
                let p: [f64; 6] = std::array::from_fn(|i| frame.mu[i] + (0..6).map(|j| frame.a[i][j] * zeta[j]).sum::<f64>());
                m0 += wt;
                for i in 0..6 {
                    m1[i] += wt * p[i];
                    for j in 0..6 {
                        m2[i][j] += wt * p[i] * p[j];
                    }
                }
                if nd.off_sheet {
                    off += wt;
                }
                (m0, m1, m2, off)
            },
        )
        .reduce(
            || (0.0, [0.0; 6], [[0.0; 6]; 6], 0.0),
            |a, b| {
                let mut m1 = a.1;
                let mut m2 = a.2;
                for i in 0..6 {
                    m1[i] += b.1[i];
                    for j in 0..6 {
                        m2[i][j] += b.2[i][j];
                    }
                }
                (a.0 + b.0, m1, m2, a.3 + b.3)
            },
        );
    let mean: [f64; 6] = std::array::from_fn(|i| m1[i] / m0);
    let cov: M6 = std::array::from_fn(|i| std::array::from_fn(|j| m2[i][j] / m0 - mean[i] * mean[j]));
    (mean, cov, off / m0)
}

#[derive(Clone, Debug, Serialize)]
pub struct Iterate {
    pub iteration: usize,
    pub levels: Vec<f64>,
    pub gaps: Vec<f64>,
    pub frame_change: f64,
    pub off_sheet_weight: f64,
    pub seconds: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SqrtgResult {
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub order: usize,
    pub dim: usize,
    pub iterations: Vec<Iterate>,
    pub settled: bool,
    pub frame: Frame,
    pub covariance: M6,
}

/// Self-consistent solve: start from `frame`, iterate until the centre changes by < 1e-6 (and the widths by < 1e-6
/// relative), at most `max_iter` iterations (0 = solve once in the given frame).
pub fn self_consistent(model: &Model, frame: Frame, cov: M6, mq: usize, order: usize, max_iter: usize, nev: usize)
    -> Result<SqrtgResult, String> {
    let mut frame = frame;
    let mut cov = cov;
    let mut its = vec![];
    let mut settled = false;
    for it in 0..=max_iter {
        let t0 = std::time::Instant::now();
        let a = assemble(model, &frame, mq, order);
        let (vals, c) = solve(&a, nev)?;
        let c0: Vec<f64> = (0..c.nrows()).map(|i| c[(i, 0)]).collect();
        let (mean, cnew, off) = ground_moments(model, &frame, &a.states, &c0, mq, order);
        let change = (0..6).map(|i| (mean[i] - frame.mu[i]).abs()).fold(0.0, f64::max);
        let ew = eigh6(&cnew).0;
        let eo = eigh6(&cov).0;
        let wchange = (0..6).map(|i| ((ew[i] - eo[i]) / eo[i]).abs()).fold(0.0, f64::max);
        its.push(Iterate {
            iteration: it,
            gaps: vals[1..].iter().map(|v| v - vals[0]).collect(),
            levels: vals,
            frame_change: change.max(wchange),
            off_sheet_weight: off,
            seconds: t0.elapsed().as_secs_f64(),
        });
        if max_iter == 0 {
            break;
        }
        frame = Frame::from_covariance(mean, &cnew);
        cov = cnew;
        if change < 1e-6 && wchange < 1e-6 {
            settled = true;
            break;
        }
    }
    Ok(SqrtgResult { mq, order, dim: states_up_to(mq).len(), iterations: its, settled, frame, covariance: cov })
}
