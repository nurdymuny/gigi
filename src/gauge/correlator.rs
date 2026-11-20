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
//!
//! Errors are a (blocked) jackknife: leave out each block of `block` consecutive
//! configurations; `block = 1` is the plain leave-one-out jackknife.

/// Minimum plateau length (time slices).
pub const MIN_LEN: usize = 3;
/// Largest admissible χ²/dof of the constant fit.
pub const MAX_CHI2_DOF: f64 = 2.0;

fn check(p: &[Vec<f64>]) -> Result<usize, String> {
    if p.is_empty() {
        return Err("CORRELATOR: no configurations".into());
    }
    let l = p[0].len();
    if l < 3 || p.iter().any(|r| r.len() != l) {
        return Err("CORRELATOR: every configuration needs the same number (>= 3) of time slices".into());
    }
    if p.iter().flatten().any(|x| !x.is_finite()) {
        return Err("CORRELATOR: a time-slice value is not finite".into());
    }
    Ok(l)
}

/// Per-configuration sums about a fixed reference mu0 (the mean of the full set):
/// a_c(tau) = (1/L) sum_t q(t) q(t+tau) and m_c = (1/L) sum_t q(t), with q = p - mu0.
/// For any subset S, C_S(tau) = mean_S a_c(tau) - (mean_S m_c)^2 equals the periodic connected
/// correlator of S exactly (the correlator is invariant under a constant shift, because the
/// origin averages of q(t) and q(t+tau) coincide). With mu0 the full mean the subtracted term is
/// tiny, so there is no cancellation of two numbers near <p>^2, and each leave-a-block-out
/// sample costs O(L) once the sums exist.
struct Sums {
    a: Vec<Vec<f64>>,
    m: Vec<f64>,
    tot_a: Vec<f64>,
    tot_m: f64,
}

fn sums(p: &[Vec<f64>], l: usize) -> Sums {
    let mu0 = p.iter().flatten().sum::<f64>() / (p.len() * l) as f64;
    let mut a = Vec::with_capacity(p.len());
    let mut m = Vec::with_capacity(p.len());
    for row in p {
        let q: Vec<f64> = row.iter().map(|x| x - mu0).collect();
        a.push((0..l).map(|tau| (0..l).map(|t| q[t] * q[(t + tau) % l]).sum::<f64>() / l as f64).collect::<Vec<f64>>());
        m.push(q.iter().sum::<f64>() / l as f64);
    }
    let mut tot_a = vec![0.0; l];
    for r in &a {
        for (t, x) in r.iter().enumerate() {
            tot_a[t] += x;
        }
    }
    let tot_m = m.iter().sum();
    Sums { a, m, tot_a, tot_m }
}

fn corr_from(tot_a: &[f64], tot_m: f64, n: f64) -> Vec<f64> {
    let mm = tot_m / n;
    tot_a.iter().map(|x| x / n - mm * mm).collect()
}

/// Full correlator and one leave-one-block-out correlator per block of `block` consecutive
/// configurations (the last block may be shorter).
fn jk_correlators(p: &[Vec<f64>], block: usize) -> Result<(Vec<f64>, Vec<Vec<f64>>), String> {
    let l = check(p)?;
    let n = p.len();
    if block == 0 || n < 2 * block {
        return Err(format!(
            "CORRELATOR: at least two blocks of {block} configurations are needed for errors (have {n})"
        ));
    }
    let s = sums(p, l);
    let full = corr_from(&s.tot_a, s.tot_m, n as f64);
    let jk = (0..n)
        .step_by(block)
        .map(|b0| {
            let b1 = (b0 + block).min(n);
            let mut ta = s.tot_a.clone();
            let mut tm = s.tot_m;
            for c in b0..b1 {
                for t in 0..l {
                    ta[t] -= s.a[c][t];
                }
                tm -= s.m[c];
            }
            corr_from(&ta, tm, (n - (b1 - b0)) as f64)
        })
        .collect();
    Ok((full, jk))
}

/// Periodic connected correlator C(t) = (1/L) Σ_t' ⟨p(t') p(t'+t)⟩ − ⟨p⟩²,
/// averaged over configurations (Halcyon's `connected_correlator_t`).
pub fn connected_correlator(p: &[Vec<f64>]) -> Result<Vec<f64>, String> {
    let l = check(p)?;
    let s = sums(p, l);
    Ok(corr_from(&s.tot_a, s.tot_m, p.len() as f64))
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

fn jk_error(samples: &[f64]) -> f64 {
    let b = samples.len() as f64;
    let m = samples.iter().sum::<f64>() / b;
    (((b - 1.0) / b) * samples.iter().map(|x| (x - m).powi(2)).sum::<f64>()).sqrt()
}

/// Blocked jackknife error of C(t): sqrt((B−1)/B Σ_b (C_b − C̄)²).
pub fn jackknife_correlator_blocked(p: &[Vec<f64>], block: usize) -> Result<(Vec<f64>, Vec<f64>), String> {
    let (full, jk) = jk_correlators(p, block)?;
    let err = (0..full.len()).map(|t| jk_error(&jk.iter().map(|r| r[t]).collect::<Vec<_>>())).collect();
    Ok((full, err))
}

/// Leave-one-out jackknife error of C(t) (Halcyon's `jackknife_correlator`).
pub fn jackknife_correlator(p: &[Vec<f64>]) -> Result<(Vec<f64>, Vec<f64>), String> {
    jackknife_correlator_blocked(p, 1)
}

/// Blocked jackknife error of the effective mass. Where any leave-a-block-out sample is
/// undefined the error is NaN: the effective mass at that t is not stable under removing
/// data, so the plateau rule skips it.
pub fn jackknife_effective_mass_blocked(
    p: &[Vec<f64>],
    block: usize,
) -> Result<(Vec<Option<f64>>, Vec<f64>), String> {
    let (full, jk) = jk_correlators(p, block)?;
    let mfull = effective_mass_cosh(&full);
    let mjk: Vec<Vec<Option<f64>>> = jk.iter().map(|c| effective_mass_cosh(c)).collect();
    let err = (0..mfull.len())
        .map(|t| match mjk.iter().map(|r| r[t]).collect::<Option<Vec<f64>>>() {
            Some(v) => jk_error(&v),
            None => f64::NAN,
        })
        .collect();
    Ok((mfull, err))
}

/// Standard leave-one-out jackknife error of the effective mass.
pub fn jackknife_effective_mass(p: &[Vec<f64>]) -> Result<(Vec<Option<f64>>, Vec<f64>), String> {
    jackknife_effective_mass_blocked(p, 1)
}

/// Halcyon's `jackknife_effective_mass`, normalisation included, kept ONLY for the parity
/// gate. It divides the jackknife sum by the number of finite samples as well, which makes
/// the error about sqrt(n) times smaller than the standard jackknife; the verb does not use it.
pub fn jackknife_effective_mass_halcyon(p: &[Vec<f64>]) -> Result<(Vec<Option<f64>>, Vec<f64>), String> {
    let (full, jk) = jk_correlators(p, 1)?;
    let mfull = effective_mass_cosh(&full);
    let mjk: Vec<Vec<Option<f64>>> = jk.iter().map(|c| effective_mass_cosh(c)).collect();
    let n = p.len() as f64;
    let err = (0..mfull.len())
        .map(|t| {
            let vals: Vec<f64> = mjk.iter().filter_map(|r| r[t]).collect();
            if vals.is_empty() {
                return 0.0; // numpy: the diff of an all-NaN column is zeroed, so the error is 0
            }
            let m = vals.iter().sum::<f64>() / vals.len() as f64;
            let ss: f64 = vals.iter().map(|v| (v - m).powi(2)).sum();
            (((n - 1.0) / n) * ss / vals.len() as f64).sqrt()
        })
        .collect();
    Ok((mfull, err))
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

/// A refusal: no window satisfies the rule. `t` is where the earliest run broke.
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
                    "the effective mass is undefined or unstable under the jackknife"
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
                    first_break = Some(NoPlateau {
                        t: t_lo,
                        reason: format!("constant fit chi2/dof {chi2_dof:.2} > {MAX_CHI2_DOF}"),
                    });
                }
                break;
            }
            hi -= 1;
        }
    }
    Err(first_break.unwrap_or(NoPlateau {
        t: 0,
        reason: "no time slice has an effective mass that is defined in every jackknife sample".into(),
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

/// EFFECTIVE_MASS with the leave-one-out jackknife.
pub fn effective_mass(p: &[Vec<f64>]) -> Result<EffectiveMass, String> {
    effective_mass_blocked(p, 1)
}

/// EFFECTIVE_MASS with a blocked jackknife (blocks of `block` consecutive configurations),
/// for autocorrelated Monte-Carlo measurements.
pub fn effective_mass_blocked(p: &[Vec<f64>], block: usize) -> Result<EffectiveMass, String> {
    let (c, ce) = jackknife_correlator_blocked(p, block)?;
    let (m, me) = jackknife_effective_mass_blocked(p, block)?;
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

    /// Configurations whose time slices carry a periodic exponentially smeared signal.
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
    fn shift_invariant_sums_match_the_direct_definition() {
        let p = ensemble(0.5, 12, 50, 0.05, 3);
        let (n, l) = (p.len(), 12usize);
        let mean = p.iter().flatten().sum::<f64>() / (n * l) as f64;
        let direct: Vec<f64> = (0..l)
            .map(|tau| {
                p.iter().map(|r| (0..l).map(|t| r[t] * r[(t + tau) % l]).sum::<f64>() / l as f64).sum::<f64>()
                    / n as f64
                    - mean * mean
            })
            .collect();
        let c = connected_correlator(&p).unwrap();
        for t in 0..l {
            assert!((c[t] - direct[t]).abs() < 1e-12 * direct[0].abs(), "t={t}");
        }
        // a leave-one-out sample equals the correlator recomputed without that configuration
        let (_, jk) = jk_correlators(&p, 1).unwrap();
        let c7 = connected_correlator(&[&p[..7], &p[8..]].concat()).unwrap();
        for t in 0..l {
            assert!((jk[7][t] - c7[t]).abs() < 1e-13, "t={t}");
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
        let rb = effective_mass_blocked(&p, 20).unwrap();
        assert!(rb.plateau.is_ok(), "{:?}", rb.plateau);
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
        assert!(effective_mass_blocked(&vec![vec![1.0, 2.0, 3.0]; 10], 6).is_err());
    }
}
