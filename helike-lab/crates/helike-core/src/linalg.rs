//! Dense complex linear algebra on top of `faer`: the few operations the builders need.

use faer::{Mat, Side};
pub use num_complex::Complex64 as C64;

/// Dense complex matrix.
pub type CMat = Mat<C64>;

pub const I: C64 = C64 { re: 0.0, im: 1.0 };
pub const ZERO: C64 = C64 { re: 0.0, im: 0.0 };
pub const ONE: C64 = C64 { re: 1.0, im: 0.0 };

pub fn c(re: f64) -> C64 {
    C64::new(re, 0.0)
}

pub fn zeros(r: usize, k: usize) -> CMat {
    Mat::zeros(r, k)
}

pub fn eye(n: usize) -> CMat {
    Mat::from_fn(n, n, |i, j| if i == j { ONE } else { ZERO })
}

/// Real matrix (row-major slice) as a complex matrix.
pub fn from_real(rows: usize, cols: usize, a: &[f64]) -> CMat {
    Mat::from_fn(rows, cols, |i, j| c(a[i * cols + j]))
}

/// Kronecker product a ⊗ b.
pub fn kron(a: &CMat, b: &CMat) -> CMat {
    let (ar, ac, br, bc) = (a.nrows(), a.ncols(), b.nrows(), b.ncols());
    Mat::from_fn(ar * br, ac * bc, |i, j| a[(i / br, j / bc)] * b[(i % br, j % bc)])
}

/// Conjugate transpose.
pub fn adjoint(a: &CMat) -> CMat {
    Mat::from_fn(a.ncols(), a.nrows(), |i, j| a[(j, i)].conj())
}

/// Plain transpose (no conjugation).
pub fn transpose(a: &CMat) -> CMat {
    Mat::from_fn(a.ncols(), a.nrows(), |i, j| a[(j, i)])
}

/// (A + A^H)/2.
pub fn hermitize(a: &CMat) -> CMat {
    Mat::from_fn(a.nrows(), a.ncols(), |i, j| (a[(i, j)] + a[(j, i)].conj()) * 0.5)
}

pub fn scale(a: &CMat, s: C64) -> CMat {
    Mat::from_fn(a.nrows(), a.ncols(), |i, j| a[(i, j)] * s)
}

/// Largest entrywise modulus.
pub fn max_abs(a: &CMat) -> f64 {
    let mut m = 0.0f64;
    for j in 0..a.ncols() {
        for i in 0..a.nrows() {
            m = m.max(a[(i, j)].norm());
        }
    }
    m
}

/// Largest entrywise |a - b|.
pub fn max_abs_diff(a: &CMat, b: &CMat) -> f64 {
    assert_eq!((a.nrows(), a.ncols()), (b.nrows(), b.ncols()));
    let mut m = 0.0f64;
    for j in 0..a.ncols() {
        for i in 0..a.nrows() {
            m = m.max((a[(i, j)] - b[(i, j)]).norm());
        }
    }
    m
}

/// Commutator AB - BA.
pub fn commutator(a: &CMat, b: &CMat) -> CMat {
    a * b - b * a
}

/// Eigenvalues of a Hermitian matrix, nondecreasing. The input is hermitized first.
pub fn eigvalsh(a: &CMat) -> Vec<f64> {
    if a.nrows() == 0 {
        return vec![];
    }
    hermitize(a)
        .self_adjoint_eigenvalues(Side::Lower)
        .expect("self-adjoint eigenvalues did not converge")
}

/// Eigenvalues and orthonormal eigenvectors (columns) of a Hermitian matrix, nondecreasing.
pub fn eigh(a: &CMat) -> (Vec<f64>, CMat) {
    if a.nrows() == 0 {
        return (vec![], zeros(0, 0));
    }
    let h = hermitize(a);
    let e = h.self_adjoint_eigen(Side::Lower).expect("self-adjoint eigen did not converge");
    let n = h.nrows();
    let s = e.S().column_vector();
    let vals = (0..n).map(|k| s[k].re).collect();
    (vals, e.U().to_owned())
}

/// Orthonormal basis (columns) of ker A^H = (range A)^perp, with the numerical rank of A decided by
/// sv > rel_tol * max(sv). Returns (basis, rank). This is the gauge slice ker G^* when A = G.
pub fn cokernel(a: &CMat, rel_tol: f64) -> (CMat, usize) {
    let m = a.nrows();
    if a.ncols() == 0 || m == 0 {
        return (eye(m), 0);
    }
    let svd = a.svd().expect("svd did not converge");
    let s = svd.S().column_vector();
    let k = s.nrows();
    let smax = (0..k).map(|i| s[i].re).fold(0.0f64, f64::max);
    let rank = if smax > 0.0 { (0..k).filter(|&i| s[i].re > rel_tol * smax).count() } else { 0 };
    let u = svd.U();
    (Mat::from_fn(m, m - rank, |i, j| u[(i, rank + j)]), rank)
}

/// Eigenvalues of K^H A K (the compression of A to the columns of K).
pub fn compressed_eigvalsh(a: &CMat, k: &CMat) -> Vec<f64> {
    if k.ncols() == 0 {
        return vec![];
    }
    eigvalsh(&(adjoint(k) * a * k))
}

/// Remove up to `copies` values within tol*max(1,|v|) of `v` from a sorted list. Returns how many were removed.
pub fn remove_copies(vals: &mut Vec<f64>, v: f64, copies: usize, rel_tol: f64) -> usize {
    let mut removed = 0;
    for _ in 0..copies {
        let (k, d) = vals
            .iter()
            .enumerate()
            .map(|(k, &w)| (k, (w - v).abs()))
            .fold((usize::MAX, f64::INFINITY), |acc, x| if x.1 < acc.1 { x } else { acc });
        if k != usize::MAX && d < rel_tol * v.abs().max(1.0) {
            vals.remove(k);
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kron_and_adjoint() {
        let a = Mat::from_fn(2, 2, |i, j| C64::new((i + 2 * j) as f64, i as f64));
        let b = eye(3);
        let k = kron(&a, &b);
        assert_eq!(k.nrows(), 6);
        assert_eq!(k[(4, 1)], a[(1, 0)]);
        assert_eq!(k[(4, 2)], ZERO);
        assert_eq!(k[(3, 3)], a[(1, 1)]);
        let ah = adjoint(&a);
        assert_eq!(ah[(0, 1)], a[(1, 0)].conj());
    }

    #[test]
    fn cokernel_of_rank_one() {
        // A = u v^H with u = (1, 1, 0)/sqrt 2: ker A^H is spanned by (1,-1,0) and (0,0,1).
        let a = Mat::from_fn(3, 2, |i, _| if i < 2 { c(1.0) } else { ZERO });
        let (k, r) = cokernel(&a, 1e-10);
        assert_eq!(r, 1);
        assert_eq!(k.ncols(), 2);
        let res = adjoint(&a) * &k;
        assert!(max_abs(&res) < 1e-14);
    }
}
