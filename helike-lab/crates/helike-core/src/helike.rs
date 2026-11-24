//! The circle-symmetric SU(2) family on the Berger spheres (t, 1, 1): M_0 = x tau_0, M_1 = y tau_1, M_2 = y tau_2.
//!
//! This is one instance of the general machinery (`sector`, `charge`), plus the closed forms of the Floor Theorem and
//! *By Enough*: the twist's eigenvalue, the fibre field, the flux spectrum, the ladder's sufficient floor condition, and
//! the edge functions phi_A (operator H + G G^*, by charge blocks) and phi_S (gauge-orthogonal spectrum).

use crate::charge::{charge_value, ChargeSplit, CircleGenerator};
use crate::lie::LieAlgebra;
use crate::linalg;
use crate::rep::Rep;
use crate::roots;
use crate::sector::{Field, Geometry, HessianGauge, Sector};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct Member {
    pub x: f64,
    pub y: f64,
    pub t: f64,
}

impl Member {
    pub fn new(x: f64, y: f64, t: f64) -> Self {
        Self { x, y, t }
    }

    /// The Berger field (the Levi-Civita connection of (t,1,1) read as an SU(2) connection).
    pub fn berger(t: f64) -> Self {
        let (x, y) = berger_xy(t);
        Self { x, y, t }
    }

    pub fn geometry(&self) -> Geometry {
        Geometry::new(LieAlgebra::su2_frame(), vec![self.t, 1.0, 1.0])
    }

    pub fn field(&self) -> Field {
        Field::new(LieAlgebra::so3_cross(), vec![vec![self.x, 0.0, 0.0], vec![0.0, self.y, 0.0], vec![0.0, 0.0, self.y]])
    }

    /// Fibre field B = y^2 - 2 sqrt(t) x (F_12 = B tau_0).
    pub fn fibre_field(&self) -> f64 {
        self.y * self.y - 2.0 * self.t.sqrt() * self.x
    }

    /// Cross field Theta = y (x - 2/sqrt t) (F_01 = Theta tau_2).
    pub fn cross_field(&self) -> f64 {
        self.y * (self.x - 2.0 / self.t.sqrt())
    }

    /// The twist's eigenvalue in sector j: (x + (j+2)/sqrt t)^2 + 2 sqrt(t) x - y^2.
    pub fn twist(&self, j: usize) -> f64 {
        let s = self.t.sqrt();
        (self.x + (j as f64 + 2.0) / s).powi(2) + 2.0 * s * self.x - self.y * self.y
    }

    /// Closed-form spectrum of the 9 x 9 flux matrix phi (Phi = phi (x) I_V), sorted.
    pub fn flux_spectrum(&self) -> [f64; 9] {
        let (b, k) = (self.fibre_field(), self.cross_field());
        let r = (b * b + 8.0 * k * k).sqrt();
        let mut v = [-b, -b, b, k, -k, k, -k, (b + r) / 2.0, (b - r) / 2.0];
        v.sort_by(|a, c| a.partial_cmp(c).unwrap());
        v
    }

    /// Analytic bound on ||G||^2 in sector j.
    pub fn g2_bound(&self, j: usize) -> f64 {
        let l = j as f64 / 2.0;
        (2.0 * l / self.t.sqrt() + self.x.abs()).powi(2) + 2.0 * (2.0 * l + self.y.abs()).powi(2)
    }

    /// The Helike Ladder's sufficient condition: if it holds, the twist is the lowest gauge-orthogonal eigenvalue of
    /// sector j (By Enough v2, `floor_sufficient` of helike_curl_spectrum.py).
    pub fn floor_sufficient(&self, j: usize) -> bool {
        let (b, k) = (self.fibre_field(), self.cross_field());
        if b <= k.abs() {
            return false;
        }
        let delta = b - k.abs();
        let mu2 = (self.x + (j as f64 + 2.0) / self.t.sqrt()).powi(2);
        if j == 0 {
            delta >= mu2
        } else {
            delta >= mu2 * (1.0 + self.g2_bound(j) / (2.0 * j as f64))
        }
    }

    pub fn sector_rep(j: usize) -> Rep {
        Rep::spin(j)
    }
}

pub fn berger_xy(t: f64) -> (f64, f64) {
    ((2.0 - t) / t.sqrt(), t.sqrt())
}

/// The Floor law lambda_t(j) = [j(j+2) - 2(t-3) j - 2(t-2)(t+4)]/t.
pub fn floor_law(t: f64, j: usize) -> f64 {
    let j = j as f64;
    (j * (j + 2.0) - 2.0 * (t - 3.0) * j - 2.0 * (t - 2.0) * (t + 4.0)) / t
}

/// Largest x with B >= |Theta| (the twist minimises the flux), for 0 < y < 2 sqrt t.
pub fn x_flux(y: f64, t: f64) -> f64 {
    let s = t.sqrt();
    (y * (y + 2.0 / s) / (2.0 * s + y)).min(y * (y - 2.0 / s) / (2.0 * s - y))
}

/// The rotation about the long circle: frame e_0/2, gauge tau_0. Charge q = m + c_f + c_l.
pub fn long_circle() -> CircleGenerator {
    CircleGenerator { frame_gen: vec![0.5, 0.0, 0.0], gauge_gen: vec![1.0, 0.0, 0.0] }
}

/// Everything computed in one sector at one member.
pub struct SectorData {
    pub j: usize,
    pub hg: HessianGauge,
    pub split: ChargeSplit,
}

impl SectorData {
    pub fn new(m: &Member, j: usize) -> Self {
        let geo = m.geometry();
        let field = m.field();
        let rep = Member::sector_rep(j);
        let sec = Sector::new(&geo, &field, &rep);
        let hg = sec.hessian_and_gauge();
        let split = ChargeSplit::new(&sec, &hg, &long_circle());
        Self { j, hg, split }
    }

    /// True for the two twist blocks, |q| = l + 2.
    pub fn is_twist_charge(&self, q: f64) -> bool {
        (q.abs() - (self.j as f64 / 2.0 + 2.0)).abs() < 1e-6
    }

    /// min over non-twist charge blocks of [lambda_min(B_q) - lambda_j] for H + G G^*, with the minimising charge.
    pub fn margin_a(&self, lam: f64) -> (f64, f64) {
        let a = self.hg.h_plus_ggstar();
        self.split
            .block_minima(&a)
            .into_iter()
            .filter(|(q, _)| !self.is_twist_charge(charge_value(*q)))
            .map(|(q, v)| (v - lam, charge_value(q)))
            .fold((f64::INFINITY, f64::NAN), |acc, x| if x.0 < acc.0 { x } else { acc })
    }

    /// Lowest gauge-orthogonal eigenvalue after removing two copies of lambda_j, minus lambda_j.
    pub fn margin_s(&self, lam: f64) -> f64 {
        let mut mu = crate::charge::flatten(&self.split.slice_block_spectra(&self.hg));
        linalg::remove_copies(&mut mu, lam, 2, 1e-8);
        mu.first().copied().unwrap_or(f64::INFINITY) - lam
    }
}

/// phi_A(x) = min_{j <= jmax, q non-twist} [lambda_min(B_{j,q}) - lambda_j], with the arg (j, q).
pub fn phi_a(m: &Member, jmax: usize) -> (f64, usize, f64) {
    (0..=jmax)
        .into_par_iter()
        .map(|j| {
            let sd = SectorData::new(m, j);
            let (v, q) = sd.margin_a(m.twist(j));
            (v, j, q)
        })
        .reduce(|| (f64::INFINITY, 0, f64::NAN), |a, b| if b.0 < a.0 || (b.0 == a.0 && b.1 < a.1) { b } else { a })
}

/// phi_S(x) = min_{j <= jmax} [mu_min,j - lambda_j] on the gauge slice, with the arg j.
pub fn phi_s(m: &Member, jmax: usize) -> (f64, usize) {
    (0..=jmax)
        .into_par_iter()
        .map(|j| (SectorData::new(m, j).margin_s(m.twist(j)), j))
        .reduce(|| (f64::INFINITY, 0), |a, b| if b.0 < a.0 || (b.0 == a.0 && b.1 < a.1) { b } else { a })
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Spectrum {
    /// H + G G^*, by charge blocks (the edge x*_A).
    A,
    /// Gauge-orthogonal (the edge x*_S).
    S,
}

pub fn phi(m: &Member, which: Spectrum, jmax: usize) -> f64 {
    match which {
        Spectrum::A => phi_a(m, jmax).0,
        Spectrum::S => phi_s(m, jmax).0,
    }
}

/// The largest x in [lo, hi] at which phi changes sign from >= 0 to < 0 (scan of n points, then Brent to 1e-12).
pub fn edge(which: Spectrum, y: f64, t: f64, jmax: usize, lo: f64, hi: f64, n: usize) -> Option<f64> {
    let xs = roots::linspace(lo, hi, n);
    let vals: Vec<f64> = xs.par_iter().map(|&x| phi(&Member::new(x, y, t), which, jmax)).collect();
    let (down, _) = roots::sign_changes(&vals);
    let k = *down.last()?;
    roots::brent(|x| phi(&Member::new(x, y, t), which, jmax), xs[k], xs[k + 1], 1e-12, 200)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twist_on_berger_is_floor_law() {
        for t in [1.0, 2.0, 2.75, 3.0, 6.5, 10.0] {
            for j in 0..10 {
                let m = Member::berger(t);
                assert!((m.twist(j) - floor_law(t, j)).abs() < 1e-12);
            }
        }
    }
}
