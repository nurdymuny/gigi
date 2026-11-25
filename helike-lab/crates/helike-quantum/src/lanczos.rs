//! Block Lanczos with full reorthogonalization for the lowest eigenpairs of a symmetric operator given only by
//! its action on blocks of vectors. The block size must be at least the largest multiplicity sought (the Helike
//! doublet is exactly degenerate by symmetry in the physical problem and nearly so in the trial space).
//!
//! Convergence is decided on true residuals ‖Hx − θx‖ of the returned Ritz pairs, computed with an explicit extra
//! application of H, not on the Lanczos estimate alone.

use faer::{Mat, Side};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct Eigs {
    pub values: Vec<f64>,
    /// eigenvectors, one Vec per value
    #[serde(skip)]
    pub vectors: Vec<Vec<f64>>,
    /// true residual norms ‖Hx − θx‖
    pub residuals: Vec<f64>,
    pub applications: usize,
    pub krylov_dimension: usize,
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

fn axpy(y: &mut [f64], a: f64, x: &[f64]) {
    for (p, q) in y.iter_mut().zip(x) {
        *p += a * q;
    }
}

/// Deterministic pseudo-random vector (splitmix64), so runs replay exactly.
pub fn seeded(n: usize, seed: u64) -> Vec<f64> {
    let mut s = seed.wrapping_add(0x9E3779B97F4A7C15);
    (0..n)
        .map(|_| {
            s = s.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = s;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^= z >> 31;
            (z as f64 / u64::MAX as f64) - 0.5
        })
        .collect()
}

/// Orthonormalize `w` against `basis` (twice) and within itself; returns the coefficient block of the in-block
/// QR (b×b, row-major: w_old = Q R) and replaces rank-deficient directions by fresh seeded vectors (R row zero).
fn orthonormalize(w: &mut [Vec<f64>], basis: &[Vec<f64>], seed: &mut u64) -> Vec<f64> {
    let b = w.len();
    for _ in 0..2 {
        for v in w.iter_mut() {
            for q in basis {
                let c = dot(q, v);
                axpy(v, -c, q);
            }
        }
    }
    let mut r = vec![0.0; b * b];
    let scale = w.iter().map(|v| dot(v, v).sqrt()).fold(0.0, f64::max).max(1e-300);
    for k in 0..b {
        for _ in 0..2 {
            for j in 0..k {
                let c = dot(&w[j], &w[k]);
                r[j * b + k] += c;
                let (head, tail) = w.split_at_mut(k);
                axpy(&mut tail[0], -c, &head[j]);
            }
        }
        let nrm = dot(&w[k], &w[k]).sqrt();
        if nrm > 1e-10 * scale {
            r[k * b + k] = nrm;
            w[k].iter_mut().for_each(|x| *x /= nrm);
        } else {
            // deflation: a fresh direction orthogonal to everything so far
            r[k * b + k] = 0.0;
            loop {
                *seed += 1;
                let mut v = seeded(w[k].len(), *seed);
                for _ in 0..2 {
                    for q in basis.iter().chain(w[..k].iter()) {
                        let c = dot(q, &v);
                        axpy(&mut v, -c, q);
                    }
                }
                let n = dot(&v, &v).sqrt();
                if n > 1e-8 {
                    v.iter_mut().for_each(|x| *x /= n);
                    w[k] = v;
                    break;
                }
            }
        }
    }
    r
}

/// Lowest `nev` eigenpairs. `apply` maps a block of vectors to H times each. `start` seeds the first block (it is
/// completed with seeded random vectors to `block` columns).
pub fn lowest<F>(n: usize, nev: usize, block: usize, start: &[Vec<f64>], tol: f64, max_dim: usize, apply: F) -> Eigs
where
    F: Fn(&[Vec<f64>]) -> Vec<Vec<f64>>,
{
    assert!(block >= 1 && nev >= 1 && nev <= n);
    let max_dim = max_dim.min(n);
    let mut seed = 20261124u64;
    let mut q0: Vec<Vec<f64>> = start.iter().take(block).cloned().collect();
    while q0.len() < block {
        seed += 1;
        q0.push(seeded(n, seed));
    }
    orthonormalize(&mut q0, &[], &mut seed);
    let mut basis: Vec<Vec<f64>> = q0.clone();
    let mut hq: Vec<Vec<f64>> = vec![]; // H applied to each basis vector
    let mut applications = 0;
    let mut current = q0;
    let mut check_every = 2usize;
    let mut tm: Vec<Vec<f64>> = vec![]; // projected matrix, grown incrementally
    loop {
        let w0 = apply(&current);
        applications += current.len();
        hq.extend(w0.iter().cloned());
        let mut w = w0;
        let dim_before = basis.len();
        let can_grow = dim_before + block <= max_dim;
        let steps = dim_before / block;
        // Rayleigh–Ritz on the whole basis every few blocks (full projection: robust, and N is modest)
        if steps % check_every == 0 || !can_grow {
            let m = basis.len();
            let old = tm.len();
            for row in tm.iter_mut() {
                row.resize(m, 0.0);
            }
            tm.resize(m, vec![0.0; m]);
            for i in 0..m {
                for j in old.max(i)..m {
                    let v = 0.5 * (dot(&basis[i], &hq[j]) + dot(&basis[j], &hq[i]));
                    tm[i][j] = v;
                    tm[j][i] = v;
                }
            }
            for i in old..m {
                for j in 0..old {
                    let v = 0.5 * (dot(&basis[i], &hq[j]) + dot(&basis[j], &hq[i]));
                    tm[i][j] = v;
                    tm[j][i] = v;
                }
            }
            let t = Mat::from_fn(m, m, |i, j| tm[i][j]);
            let e = t.self_adjoint_eigen(Side::Lower).expect("projected eigenproblem");
            let s = e.S().column_vector();
            let u = e.U();
            let k = nev.min(m);
            if m >= nev + block || !can_grow {
                let mut vecs = vec![];
                let mut hvecs = vec![];
                let mut vals = vec![];
                for c in 0..k {
                    let mut x = vec![0.0; n];
                    let mut hx = vec![0.0; n];
                    for i in 0..m {
                        axpy(&mut x, u[(i, c)], &basis[i]);
                        axpy(&mut hx, u[(i, c)], &hq[i]);
                    }
                    vals.push(s[c]);
                    vecs.push(x);
                    hvecs.push(hx);
                }
                let res: Vec<f64> = (0..k)
                    .map(|c| {
                        let mut r = hvecs[c].clone();
                        axpy(&mut r, -vals[c], &vecs[c]);
                        dot(&r, &r).sqrt()
                    })
                    .collect();
                let done = res.iter().zip(&vals).all(|(r, v)| *r <= tol * v.abs().max(1.0));
                if done || !can_grow {
                    // confirm with a fresh application (the hq columns are exact, but say so explicitly)
                    let hx = apply(&vecs);
                    applications += vecs.len();
                    let residuals = (0..k)
                        .map(|c| {
                            let mut r = hx[c].clone();
                            axpy(&mut r, -vals[c], &vecs[c]);
                            dot(&r, &r).sqrt()
                        })
                        .collect();
                    return Eigs { values: vals, vectors: vecs, residuals, applications, krylov_dimension: m };
                }
            }
            if steps > 8 {
                check_every = 4;
            }
        }
        // next block: orthogonalize H·current against the whole basis
        orthonormalize(&mut w, &basis, &mut seed);
        basis.extend(w.iter().cloned());
        current = w;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_with_doublet() {
        let n = 300;
        let mut d: Vec<f64> = (0..n).map(|i| 1.0 + 0.37 * i as f64).collect();
        d[5] = d[4]; // exact doublet
        let apply = |vs: &[Vec<f64>]| vs.iter().map(|v| v.iter().zip(&d).map(|(a, b)| a * b).collect()).collect();
        let r = lowest(n, 8, 4, &[], 1e-10, 300, apply);
        let mut want = d.clone();
        want.sort_by(|a, b| a.partial_cmp(b).unwrap());
        for k in 0..8 {
            assert!((r.values[k] - want[k]).abs() < 1e-9, "{k}: {} vs {}", r.values[k], want[k]);
            assert!(r.residuals[k] < 1e-9);
        }
    }
}
