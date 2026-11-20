//! CORRELATOR and EFFECTIVE_MASS (Q1, docs/HALCYON_QUANTUM_GAP_SPEC.md).
//!
//! Engine form of Halcyon's M1 estimator (davis-wilson-lattice/inertia_damping/
//! su2_4d_glueball.py): the periodic connected time-slice correlator, its
//! jackknife errors, and the cosh effective mass, with the same definitions.
//!
//! What is new is the plateau. Halcyon's auto-window always returns a fit (on
//! its own time-shuffled null control it returned 1.118). Here the plateau rule
//! is fixed in the verb, and when no window satisfies it the verb refuses,
//! naming the time slice where the rule failed:
//!
//! * at least `MIN_LEN` consecutive t with a finite effective mass and a
//!   positive jackknife error;
//! * consecutive values agree within one combined standard error;
//! * the constant fit over the window has χ²/dof ≤ `MAX_CHI2_DOF`.
//!
//! The earliest such window is taken and extended while the rule holds.

/// Minimum plateau length (time slices).
pub const MIN_LEN: usize = 3;
/// Largest admissible χ²/dof of the constant fit.
pub const MAX_CHI2_DOF: f64 = 2.0;

/// Periodic connected correlator C(t) = (1/L) Σ_t' ⟨p(t') p(t'+t)⟩ − ⟨p⟩²,
/// averaged over configurations (Halcyon's `connected_correlator_t`).
pub fn connected_correlator(p: &[Vec<f64>]) -> Result<Vec<f64>, String> {
    let n = p.len();
    if n == 0 {
        return Err("CORRELATOR: no configurations".into());
    }
    let l = p[0].len();
    if l < 3 || p.iter().any(|r| r.len() != l) {
        return Err("CORRELATOR: every configuration needs the same number (>= 3) of time slices".into());
    }
    if p.iter().flatten().any(|x| !x.is_finite()) {
        return Err("CORRELATOR: a time-slice value is not finite".into());
    }
    // Centred form: (1/L) sum_t <(p(t) - mu)(p(t+tau) - mu)> equals <p p> - mu^2 exactly (the
    // per-configuration origin averages of p(t) and p(t+tau) both average to mu), but avoids the
    // cancellation of two numbers near <p>^2 whose difference is C.
    let mean = p.iter().flatten().sum::<f64>() / (n * l) as f64;
    let q: Vec<Vec<f64>> = p.iter().map(|r| r.iter().map(|x| x - mean).collect()).collect();
    Ok((0..l)
        .map(|tau| {
            q.iter()
                .map(|row| (0..l).map(|t| row[t] * row[(t + tau) % l]).sum::<f64>() / l as f64)
                .sum::<f64>()
                / n as f64
        })
        .collect())
}

/// Cosh effective mass m(t) = arccosh((C(t−1) + C(t+1)) / 2C(t)) for t = 1..L−2;
/// `None` where C(t) ≤ 0 or the argument is below 1 (Halcyon's `effective_mass_t`).
pub fn effective_mass_cosh(c: &[f64]) -> Vec<Option<f64>> {
    let l = c.len();
    (0..l)
        .map(|t| {
            if t == 0 || t + 1 >= l || c[t] <= 0.0 {
                return None;
            }
            let arg = (c[t - 1] + c[t + 1]) / (2.0 * c[t]);
            if arg >= 1.0 { Some(arg.acosh()) } else { None }
        })
        .collect()
}

fn without(p: &[Vec<f64>], j: usize) -> Vec<Vec<f64>> {
    p.iter().enumerate().filter(|(i, _)| *i != j).map(|(_, r)| r.clone()).collect()
}

/// Jackknife error of C(t): sqrt((n−1)/n Σ_j (C_j − C̄)²).
pub fn jackknife_correlator(p: &[Vec<f64>]) -> Result<(Vec<f64>, Vec<f64>), String> {
    let full = connected_correlator(p)?;
    let n = p.len();
    if n < 2 {
        return Err("CORRELATOR: at least two configurations are needed for errors".into());
    }
    let jk: Vec<Vec<f64>> = (0..n).map(|j| connected_correlator(&without(p, j))).collect::<Result<_, _>>()?;
    let l = full.len();
    let err = (0..l)
        .map(|t| {
            let m = jk.iter().map(|r| r[t]).sum::<f64>() / n as f64;
            (((n - 1) as f64 / n as f64) * jk.iter().map(|r| (r[t] - m).powi(2)).sum::<f64>()).sqrt()
        })
        .collect();
    Ok((full, err))
}

fn jackknife_m_samples(p: &[Vec<f64>]) -> Result<(Vec<Option<f64>>, Vec<Vec<Option<f64>>>), String> {
    let full = effective_mass_cosh(&connected_correlator(p)?);
    if p.len() < 2 {
        return Err("EFFECTIVE_MASS: at least two configurations are needed for errors".into());
    }
    let jk = (0..p.len())
        .map(|j| connected_correlator(&without(p, j)).map(|c| effective_mass_cosh(&c)))
        .collect::<Result<_, _>>()?;
    Ok((full, jk))
}

/// Standard jackknife error of the effective mass: sqrt((n-1)/n * sum_j (m_j - mean)^2).
/// Where any leave-one-out sample is undefined the error is NaN: the effective mass at
/// that t is not stable under removing one configuration, so the plateau rule skips it.
pub fn jackknife_effective_mass(p: &[Vec<f64>]) -> Result<(Vec<Option<f64>>, Vec<f64>), String> {
    let (full, jk) = jackknife_m_samples(p)?;
    let n = p.len() as f64;
    let err = (0..full.len())
        .map(|t| {
            let vals: Option<Vec<f64>> = jk.iter().map(|r| r[t]).collect();
            match vals {
                Some(v) => {
                    let m = v.iter().sum::<f64>() / n;
                    (((n - 1.0) / n) * v.iter().map(|x| (x - m).powi(2)).sum::<f64>()).sqrt()
                }
                None => f64::NAN,
            }
        })
        .collect();
    Ok((full, err))
}

/// Halcyon's `jackknife_effective_mass`, normalisation included, kept ONLY for the parity
/// gate. It divides the jackknife sum by the number of finite samples as well, which makes
/// the error about sqrt(n) times smaller than the standard jackknife; the verb does not use it.
pub fn jackknife_effective_mass_halcyon(p: &[Vec<f64>]) -> Result<(Vec<Option<f64>>, Vec<f64>), String> {
    let (full, jk) = jackknife_m_samples(p)?;
    let n = p.len() as f64;
    let err = (0..full.len())
        .map(|t| {
            let vals: Vec<f64> = jk.iter().filter_map(|r| r[t]).collect();
            if vals.is_empty() {
                return 0.0; // numpy: diff of an all-NaN column is zeroed, so the error is 0
            }
            let m = vals.iter().sum::<f64>() / vals.len() as f64;
            let ss: f64 = vals.iter().map(|v| (v - m).powi(2)).sum();
            (((n - 1.0) / n) * ss / vals.len() as f64).sqrt()
        })
        .collect();
    Ok((full, err))
}

/// An accepted plateau.
#[derive(Clone, Debug, PartialEq)]
pub struct Plateau {
    pub t_lo: usize,
    pub t_hi: usize,
    pub mass: f64,
    pub error: f64,
    pub chi2_dof: f64,
}

/// A refusal: no window satisfies the rule. `t` is where the longest run broke.
#[derive(Clone, Debug, PartialEq)]
pub struct NoPlateau {
    pub t: usize,
    pub reason: String,
}

impl std::fmt::Display for NoPlateau {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EFFECTIVE_MASS: no plateau: {} (at t = {})", self.reason, self.t)
    }
}

fn fit(m: &[f64], e: &[f64]) -> (f64, f64, f64) {
    let w: Vec<f64> = e.iter().map(|x| 1.0 / (x * x)).collect();
    let sw: f64 = w.iter().sum();
    let mean = m.iter().zip(&w).map(|(a, b)| a * b).sum::<f64>() / sw;
    let chi2: f64 = m.iter().zip(e).map(|(a, b)| ((a - mean) / b).powi(2)).sum();
    let dof = (m.len() - 1).max(1) as f64;
    (mean, (1.0 / sw).sqrt(), chi2 / dof)
}

/// The plateau rule of the spec (module docs). Never falls back to a best guess.
pub fn plateau(m: &[Option<f64>], err: &[f64]) -> Result<Plateau, NoPlateau> {
    let ok = |t: usize| m[t].is_some() && err[t] > 0.0 && err[t].is_finite();
    let agree = |t: usize| {
        let (a, b) = (m[t].unwrap(), m[t + 1].unwrap());
        (a - b).abs() <= (err[t] * err[t] + err[t + 1] * err[t + 1]).sqrt()
    };
    let mut first_break: Option<NoPlateau> = None;
    for t_lo in 0..m.len() {
        if !ok(t_lo) {
            continue;
        }
        let mut t_hi = t_lo;
        while t_hi + 1 < m.len() && ok(t_hi + 1) && agree(t_hi) {
            t_hi += 1;
        }
        if t_hi + 1 - t_lo < MIN_LEN {
            if first_break.is_none() {
                let why = if t_hi + 1 < m.len() && ok(t_hi + 1) {
                    "consecutive effective masses disagree by more than one combined standard error"
                } else {
                    "the effective mass is undefined (correlator not positive or argument below 1)"
                };
                first_break = Some(NoPlateau { t: t_hi + 1, reason: why.into() });
            }
            continue;
        }
        // shrink from the right until the constant fit is acceptable
        let mut hi = t_hi;
        loop {
            let ms: Vec<f64> = (t_lo..=hi).map(|t| m[t].unwrap()).collect();
            let es: Vec<f64> = (t_lo..=hi).map(|t| err[t]).collect();
            let (mass, error, chi2_dof) = fit(&ms, &es);
            if chi2_dof <= MAX_CHI2_DOF {
                return Ok(Plateau { t_lo, t_hi: hi, mass, error, chi2_dof });
            }
            if hi + 1 - t_lo <= MIN_LEN {
                if first_break.is_none() {
                    first_break = Some(NoPlateau { t: t_lo, reason: format!("constant fit chi2/dof {chi2_dof:.2} > {MAX_CHI2_DOF}") });
                }
                break;
            }
            hi -= 1;
        }
    }
    Err(first_break.unwrap_or(NoPlateau {
        t: 0,
        reason: "no time slice has an effective mass that is defined in every leave-one-out sample".into(),
    }))
}

/// Everything EFFECTIVE_MASS returns.
#[derive(Clone, Debug)]
pub struct EffectiveMass {
    pub correlator: Vec<f64>,
    pub correlator_error: Vec<f64>,
    pub m_eff: Vec<Option<f64>>,
    pub m_eff_error: Vec<f64>,
    pub plateau: Result<Plateau, NoPlateau>,
}

pub fn effective_mass(p: &[Vec<f64>]) -> Result<EffectiveMass, String> {
    let (c, ce) = jackknife_correlator(p)?;
    let (m, me) = jackknife_effective_mass(p)?;
    let plateau = plateau(&m, &me);
    Ok(EffectiveMass { correlator: c, correlator_error: ce, m_eff: m, m_eff_error: me, plateau })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random numbers (no extra dependency).
    fn lcg(seed: &mut u64) -> f64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((*seed >> 11) as f64) / ((1u64 << 53) as f64) - 0.5
    }

    /// Configurations whose time slices carry a periodic AR-like signal with mass m:
    /// p(t) = sum_k a_k cosh-shaped kernels, built as a sum of exponentially smeared noise.
    fn ensemble(m: f64, l: usize, n: usize, noise: f64, seed: u64) -> Vec<Vec<f64>> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                let z: Vec<f64> = (0..l).map(|_| lcg(&mut s)).collect();
                (0..l)
                    .map(|t| {
                        let sig: f64 = (0..l)
                            .map(|u| {
                                let d = (t as isize - u as isize).unsigned_abs();
                                let d = d.min(l - d) as f64;
                                (-m * d).exp() * z[u]
                            })
                            .sum();
                        1.0 + sig + noise * lcg(&mut s)
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn exact_cosh_correlator_gives_its_mass() {
        let (l, m) = (16usize, 0.7f64);
        let c: Vec<f64> = (0..l).map(|t| (m * (t as f64 - l as f64 / 2.0)).cosh()).collect();
        for (t, v) in effective_mass_cosh(&c).iter().enumerate() {
            if (1..l - 1).contains(&t) && t != l / 2 {
                assert!((v.unwrap() - m).abs() < 1e-12, "t={t} {v:?}");
            }
        }
    }

    #[test]
    fn plateau_accepts_a_constant_and_refuses_noise() {
        let m = vec![None, Some(1.0), Some(1.01), Some(0.99), Some(1.0), None];
        let e = vec![0.0, 0.02, 0.02, 0.02, 0.02, 0.0];
        let p = plateau(&m, &e).unwrap();
        assert_eq!((p.t_lo, p.t_hi), (1, 4));
        assert!((p.mass - 1.0).abs() < 1e-12);
        // alternating values far outside one sigma: refuse, never guess
        let m = vec![None, Some(1.0), Some(2.0), Some(1.0), Some(2.0), None];
        let r = plateau(&m, &e).unwrap_err();
        assert_eq!(r.t, 2);
        // too short
        let m = vec![None, Some(1.0), Some(1.0), None, None, None];
        assert!(plateau(&m, &e).is_err());
    }

    #[test]
    fn signal_ensemble_has_a_plateau_and_the_shuffled_null_refuses() {
        let p = ensemble(0.5, 16, 400, 0.02, 7);
        let r = effective_mass(&p).unwrap();
        assert!(r.plateau.is_ok(), "{:?}", r.plateau);
        // null: break time correlation by drawing every slice from an independent configuration
        let mut s = 11u64;
        let q: Vec<Vec<f64>> = (0..p.len())
            .map(|_| (0..16).map(|t| p[((lcg(&mut s) + 0.5) * p.len() as f64) as usize % p.len()][t]).collect())
            .collect();
        let rq = effective_mass(&q).unwrap();
        assert!(rq.plateau.is_err(), "null gave {:?}", rq.plateau);
    }

    #[test]
    fn refuses_malformed_input() {
        assert!(connected_correlator(&[]).is_err());
        assert!(connected_correlator(&[vec![1.0, 2.0, 3.0], vec![1.0, 2.0]]).is_err());
        assert!(connected_correlator(&[vec![1.0, f64::NAN, 3.0]]).is_err());
        assert!(effective_mass(&[vec![1.0, 2.0, 3.0]]).is_err());
    }
}
