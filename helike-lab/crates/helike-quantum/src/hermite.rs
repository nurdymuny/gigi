//! Gauss–Hermite quadrature and normalized Hermite polynomials (the conventions of `helike_positive_cone.py`).
//!
//! `normalized(x, nmax)` returns h_0..h_nmax with h_0 = 1, h_1 = √2 x and
//! h_{n+1} = √(2/(n+1)) x h_n − √(n/(n+1)) h_{n−1}: orthonormal for the probability weight e^{−x²}/√π.
//! `gauss(q)` returns nodes and weights normalized to sum 1 (scipy's `roots_hermite` weights divided by √π).

use faer::{Mat, Side};

/// h_0(x) .. h_nmax(x).
pub fn normalized(x: f64, nmax: usize) -> Vec<f64> {
    let mut h = vec![0.0; nmax + 1];
    h[0] = 1.0;
    if nmax >= 1 {
        h[1] = std::f64::consts::SQRT_2 * x;
    }
    for n in 1..nmax {
        let nf = n as f64;
        h[n + 1] = (2.0 / (nf + 1.0)).sqrt() * x * h[n] - (nf / (nf + 1.0)).sqrt() * h[n - 1];
    }
    h
}

/// Gauss–Hermite rule of order q for the probability weight e^{−x²}/√π: (nodes ascending, weights summing to 1).
/// Exact for polynomials of degree ≤ 2q − 1. Golub–Welsch, then Newton on h_q, then Christoffel weights.
pub fn gauss(q: usize) -> (Vec<f64>, Vec<f64>) {
    assert!(q >= 1, "quadrature order must be positive");
    let mut j = Mat::<f64>::zeros(q, q);
    for k in 1..q {
        let b = (k as f64 / 2.0).sqrt();
        j[(k, k - 1)] = b;
        j[(k - 1, k)] = b;
    }
    let mut x: Vec<f64> = if q == 1 {
        vec![0.0]
    } else {
        j.self_adjoint_eigenvalues(Side::Lower).expect("Golub-Welsch eigenvalues")
    };
    for xi in x.iter_mut() {
        for _ in 0..4 {
            let h = normalized(*xi, q);
            let dh = (2.0 * q as f64).sqrt() * h[q - 1];
            if dh == 0.0 {
                break;
            }
            let step = h[q] / dh;
            *xi -= step;
            if step.abs() < 1e-16 * (1.0 + xi.abs()) {
                break;
            }
        }
    }
    // exact symmetry
    for k in 0..q / 2 {
        let m = 0.5 * (x[q - 1 - k] - x[k]);
        x[k] = -m;
        x[q - 1 - k] = m;
    }
    if q % 2 == 1 {
        x[q / 2] = 0.0;
    }
    let w: Vec<f64> = x
        .iter()
        .map(|&xi| 1.0 / normalized(xi, q - 1).iter().map(|v| v * v).sum::<f64>())
        .collect();
    (x, w)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_moments() {
        // E[x^{2k}] for e^{-x^2}/sqrt(pi) is (2k-1)!!/2^k
        for q in [3usize, 8, 14, 20] {
            let (x, w) = gauss(q);
            assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-14);
            for k in 0..q {
                let want: f64 = (1..=k).map(|i| (2 * i - 1) as f64 / 2.0).product();
                let got: f64 = x.iter().zip(&w).map(|(xi, wi)| wi * xi.powi(2 * k as i32)).sum();
                assert!((got - want).abs() <= 1e-12 * want.max(1.0), "q={q} k={k}: {got} vs {want}");
            }
        }
    }

    #[test]
    fn orthonormal_hermites() {
        let (x, w) = gauss(12);
        for a in 0..10 {
            for b in 0..10 {
                let s: f64 = x.iter().zip(&w).map(|(&xi, wi)| {
                    let h = normalized(xi, 10);
                    wi * h[a] * h[b]
                }).sum();
                assert!((s - if a == b { 1.0 } else { 0.0 }).abs() < 1e-13);
            }
        }
    }
}
