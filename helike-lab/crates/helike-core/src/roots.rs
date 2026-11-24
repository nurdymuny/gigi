//! Scalar root finding: Brent's method and sign-change scans.

/// Brent's method on [a, b] with f(a), f(b) of opposite sign (or one zero), to absolute tolerance `xtol`.
pub fn brent<F: FnMut(f64) -> f64>(mut f: F, mut a: f64, mut b: f64, xtol: f64, max_iter: usize) -> Option<f64> {
    let mut fa = f(a);
    let mut fb = f(b);
    if fa == 0.0 {
        return Some(a);
    }
    if fb == 0.0 {
        return Some(b);
    }
    if fa.signum() == fb.signum() {
        return None;
    }
    let (mut c, mut fc) = (a, fa);
    let mut d = b - a;
    let mut e = d;
    for _ in 0..max_iter {
        if fb.signum() == fc.signum() {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol = 2.0 * f64::EPSILON * b.abs() + 0.5 * xtol;
        let m = 0.5 * (c - b);
        if m.abs() <= tol || fb == 0.0 {
            return Some(b);
        }
        if e.abs() >= tol && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q);
            if a == c {
                p = 2.0 * m * s;
                q = 1.0 - s;
            } else {
                let qq = fa / fc;
                let r = fb / fc;
                p = s * (2.0 * m * qq * (qq - r) - (b - a) * (r - 1.0));
                q = (qq - 1.0) * (r - 1.0) * (s - 1.0);
            }
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            if 2.0 * p < (3.0 * m * q - (tol * q).abs()).min((e * q).abs()) {
                e = d;
                d = p / q;
            } else {
                d = m;
                e = m;
            }
        } else {
            d = m;
            e = m;
        }
        a = b;
        fa = fb;
        b += if d.abs() > tol { d } else { tol * m.signum() };
        fb = f(b);
    }
    Some(b)
}

/// Sign changes of a sampled function: indices i with v[i] >= 0 > v[i+1] ("down") or v[i] < 0 <= v[i+1] ("up").
pub fn sign_changes(v: &[f64]) -> (Vec<usize>, Vec<usize>) {
    let mut down = vec![];
    let mut up = vec![];
    for i in 0..v.len().saturating_sub(1) {
        if v[i] >= 0.0 && v[i + 1] < 0.0 {
            down.push(i);
        }
        if v[i] < 0.0 && v[i + 1] >= 0.0 {
            up.push(i);
        }
    }
    (down, up)
}

pub fn linspace(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![lo];
    }
    (0..n).map(|k| lo + (hi - lo) * k as f64 / (n - 1) as f64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brent_finds_cubic_root() {
        let r = brent(|x| x * x * x - 2.0, 0.0, 2.0, 1e-14, 200).unwrap();
        assert!((r - 2f64.cbrt()).abs() < 1e-13);
        assert!(brent(|x| x * x + 1.0, -1.0, 1.0, 1e-12, 100).is_none());
    }
}
