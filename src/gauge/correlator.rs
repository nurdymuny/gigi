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
//! **Amendments Q1-1 and Q1-2 (adopted 2026-11-19, docs/HALCYON_Q1_RESULTS.md and
//! docs/HALCYON_Q2_RESULTS.md).** The rule as first fixed accepted noise points beside a real
//! one (Q1, L = 16) and slid into the noise after two ordinary 1σ fluctuations (Q2, Wilson β = 4).
//! The rule the verb now applies:
//!
//! * Q1-1: every t in the window has C(t−1), C(t), C(t+1) each at least `MIN_SIGNAL_SIGMA`
//!   standard errors above zero (the three values m_eff(t) is built from);
//! * Q1-2: consecutive effective masses agree within `AGREE_SIGMA` combined standard errors.
//!
//! The rule as first fixed (`PlateauRule::AS_FIXED`) is still computed and returned beside it,
//! so every result can be read against the pre-registered rule.
//!
//! **Amendment Q1-3 (adopted 2026-11-19, before any Q3 data).** The error of the verb's plateau is
//! the blocked jackknife of the plateau fit itself: the window and the weights are fixed from the
//! full sample, and the weighted mean is recomputed in every leave-a-block-out sample. The fit's
//! 1/√Σw treats the m_eff(t) as independent; they are correlated, and on long windows that error was
//! about 25% too small (Q2 fresh run, β = 8). χ²/dof in the rule is unchanged. The as-fixed plateau
//! keeps the fit's error, as it was pre-registered.
//!
//! Errors are a (blocked) jackknife: leave out each block of `block` consecutive
//! configurations; `block = 1` is the plain leave-one-out jackknife.

/// Minimum plateau length (time slices).
pub const MIN_LEN: usize = 3;
/// Largest admissible χ²/dof of the constant fit.
pub const MAX_CHI2_DOF: f64 = 2.0;
/// Amendment Q1-1: minimum significance of each correlator value a window point uses.
pub const MIN_SIGNAL_SIGMA: f64 = 3.0;
/// Amendment Q1-2: consecutive effective masses agree within this many combined errors.
pub const AGREE_SIGMA: f64 = 2.0;

/// The parameters of the plateau rule.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlateauRule {
    /// consecutive m_eff agree within this many combined standard errors
    pub agree_sigma: f64,
    /// if set, C(t−1), C(t), C(t+1) must each be at least this many errors above zero
    pub min_signal_sigma: Option<f64>,
}

impl PlateauRule {
    /// The rule as first fixed in the spec, before any data.
    pub const AS_FIXED: PlateauRule = PlateauRule { agree_sigma: 1.0, min_signal_sigma: None };
    /// The rule with amendments Q1-1 and Q1-2; the verb's rule.
    pub const AMENDED: PlateauRule = PlateauRule { agree_sigma: AGREE_SIGMA, min_signal_sigma: Some(MIN_SIGNAL_SIGMA) };
}

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

/// Amendment Q1-3: the blocked-jackknife error of a plateau fit, with the window and the weights
/// 1/err(t)² fixed from the full sample. Every leave-a-block-out m_eff in the window is defined,
/// because the rule admits a t only when its jackknife error is finite.
pub fn plateau_jackknife_error(p: &[Vec<f64>], block: usize, pl: &Plateau, m_err: &[f64]) -> Result<f64, String> {
    let (_, jk) = jk_correlators(p, block)?;
    let w: Vec<f64> = (pl.t_lo..=pl.t_hi).map(|t| 1.0 / (m_err[t] * m_err[t])).collect();
    let sw: f64 = w.iter().sum();
    let samples: Vec<f64> = jk
        .iter()
        .map(|c| {
            let m = effective_mass_cosh(c);
            (pl.t_lo..=pl.t_hi)
                .zip(&w)
                .map(|(t, wt)| m[t].ok_or_else(|| format!("EFFECTIVE_MASS: jackknife sample undefined at t = {t}")).map(|v| v * wt))
                .sum::<Result<f64, String>>()
                .map(|x| x / sw)
        })
        .collect::<Result<_, _>>()?;
    Ok(jk_error(&samples))
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

/// The plateau rule as first fixed (no correlator inputs needed). Never falls back to a best guess.
pub fn plateau(m: &[Option<f64>], err: &[f64]) -> Result<Plateau, NoPlateau> {
    plateau_with(m, err, &[], &[], PlateauRule::AS_FIXED)
}

/// The plateau rule with explicit parameters; `c`, `c_err` are needed when the rule has a
/// signal floor. Never falls back to a best guess.
pub fn plateau_with(
    m: &[Option<f64>],
    err: &[f64],
    c: &[f64],
    c_err: &[f64],
    rule: PlateauRule,
) -> Result<Plateau, NoPlateau> {
    let significant = |t: usize| match rule.min_signal_sigma {
        None => true,
        Some(k) => t >= 1
            && t + 1 < c.len()
            && (t - 1..=t + 1).all(|u| c_err[u] > 0.0 && c[u] >= k * c_err[u]),
    };
    let ok = |t: usize| m[t].is_some() && err[t] > 0.0 && err[t].is_finite() && significant(t);
    let agree = |t: usize| {
        let (a, b) = (m[t].unwrap(), m[t + 1].unwrap());
        (a - b).abs() <= rule.agree_sigma * (err[t] * err[t] + err[t + 1] * err[t + 1]).sqrt()
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
                let nx = t_hi + 1;
                let why = if nx < m.len() && ok(nx) {
                    format!("consecutive effective masses disagree by more than {} combined standard errors", rule.agree_sigma)
                } else if nx < m.len() && m[nx].is_some() && err[nx] > 0.0 && err[nx].is_finite() {
                    format!("the correlator is less than {} standard errors from zero", rule.min_signal_sigma.unwrap_or(0.0))
                } else {
                    "the effective mass is undefined or unstable under the jackknife".to_string()
                };
                first_break = Some(NoPlateau { t: nx, reason: why });
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
    /// the verb's plateau: the amended rule (Q1-1, Q1-2)
    pub plateau: Result<Plateau, NoPlateau>,
    /// the rule as first fixed, for reading results against the pre-registered rule
    pub plateau_as_fixed: Result<Plateau, NoPlateau>,
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
    let mut plateau = plateau_with(&m, &me, &c, &ce, PlateauRule::AMENDED);
    if let Ok(pl) = plateau.as_mut() {
        pl.error = plateau_jackknife_error(p, block, pl, &me)?;
    }
    let plateau_as_fixed = plateau_with(&m, &me, &c, &ce, PlateauRule::AS_FIXED);
    Ok(EffectiveMass { correlator: c, correlator_error: ce, m_eff: m, m_eff_error: me, plateau, plateau_as_fixed })
}

/// The variational step of Q3 (docs/HALCYON_Q3_RUN_SPEC.md): a generalized eigenvalue problem
/// C(t1) v = λ C(t0) v on the matrix correlator of K operators, and the projection of the operator
/// series onto the eigenvector of the largest λ (the lowest state).
///
/// The projected series p(t) = Σ_a v_a O_a(t) has the correlator vᵀC(t)v, so it goes through
/// EFFECTIVE_MASS like any single operator. The vector is fixed from the full sample and not
/// re-solved in the jackknife (recorded in the run spec).
#[derive(Clone, Debug)]
pub struct Gevp {
    pub t0: usize,
    pub t1: usize,
    /// eigenvalues λ of C(t0)^{-1/2} C(t1) C(t0)^{-1/2}, descending
    pub eigenvalues: Vec<f64>,
    /// the eigenvector of the largest λ, normalised so vᵀC(t0)v = 1
    pub vector: Vec<f64>,
    /// condition number of C(t0)
    pub condition_t0: f64,
}

/// Largest admissible condition number of C(t0); above it the GEVP refuses.
pub const GEVP_MAX_CONDITION: f64 = 1e12;

/// The symmetrised connected matrix correlator C_ab(τ), periodic origin average.
pub fn matrix_correlator(ops: &[Vec<Vec<f64>>]) -> Result<Vec<Vec<Vec<f64>>>, String> {
    let k = ops.len();
    if k == 0 {
        return Err("GEVP: no operators".into());
    }
    let l = check(&ops[0])?;
    let n = ops[0].len();
    for o in ops {
        if check(o)? != l || o.len() != n {
            return Err("GEVP: every operator needs the same configurations and time slices".into());
        }
    }
    let q: Vec<Vec<Vec<f64>>> = ops
        .iter()
        .map(|o| {
            let mu = o.iter().flatten().sum::<f64>() / (n * l) as f64;
            o.iter().map(|r| r.iter().map(|x| x - mu).collect()).collect()
        })
        .collect();
    let mut c = vec![vec![vec![0.0; k]; k]; l];
    for tau in 0..l {
        for a in 0..k {
            for b in 0..k {
                let mut acc = 0.0;
                for cfg in 0..n {
                    for t in 0..l {
                        acc += q[a][cfg][t] * q[b][cfg][(t + tau) % l];
                    }
                }
                c[tau][a][b] = acc / (n * l) as f64;
            }
        }
        for a in 0..k {
            for b in 0..a {
                let m = 0.5 * (c[tau][a][b] + c[tau][b][a]);
                c[tau][a][b] = m;
                c[tau][b][a] = m;
            }
        }
    }
    Ok(c)
}

fn cholesky(a: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let k = a.len();
    let mut l = vec![vec![0.0; k]; k];
    for i in 0..k {
        for j in 0..=i {
            let mut v = a[i][j];
            for q in 0..j {
                v -= l[i][q] * l[j][q];
            }
            if i == j {
                if v <= 0.0 {
                    return None;
                }
                l[i][i] = v.sqrt();
            } else {
                l[i][j] = v / l[j][j];
            }
        }
    }
    Some(l)
}

/// Symmetric eigen-decomposition by cyclic Jacobi; returns (eigenvalues, eigenvectors as columns).
fn jacobi_eigen(mut a: Vec<Vec<f64>>) -> (Vec<f64>, Vec<Vec<f64>>) {
    let k = a.len();
    let mut v = vec![vec![0.0; k]; k];
    for i in 0..k {
        v[i][i] = 1.0;
    }
    for _ in 0..100 {
        let off: f64 = (0..k).flat_map(|i| (0..k).filter(move |&j| j != i).map(move |j| (i, j))).map(|(i, j)| a[i][j] * a[i][j]).sum();
        if off < 1e-30 {
            break;
        }
        for p in 0..k {
            for q in p + 1..k {
                if a[p][q].abs() < 1e-300 {
                    continue;
                }
                let theta = (a[q][q] - a[p][p]) / (2.0 * a[p][q]);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for r in 0..k {
                    let (arp, arq) = (a[r][p], a[r][q]);
                    a[r][p] = c * arp - s * arq;
                    a[r][q] = s * arp + c * arq;
                }
                for r in 0..k {
                    let (apr, aqr) = (a[p][r], a[q][r]);
                    a[p][r] = c * apr - s * aqr;
                    a[q][r] = s * apr + c * aqr;
                }
                for r in 0..k {
                    let (vrp, vrq) = (v[r][p], v[r][q]);
                    v[r][p] = c * vrp - s * vrq;
                    v[r][q] = s * vrp + c * vrq;
                }
            }
        }
    }
    ((0..k).map(|i| a[i][i]).collect(), v)
}

/// Solve the GEVP at (t0, t1). Refuses when C(t0) is not positive definite or is worse
/// conditioned than `GEVP_MAX_CONDITION` (operators too close to linearly dependent).
pub fn gevp(c: &[Vec<Vec<f64>>], t0: usize, t1: usize) -> Result<Gevp, String> {
    if t1 <= t0 || t1 >= c.len() {
        return Err(format!("GEVP: need t0 < t1 < L (t0 = {t0}, t1 = {t1})"));
    }
    let k = c[t0].len();
    let (ev0, _) = jacobi_eigen(c[t0].clone());
    let (lo, hi) = ev0.iter().fold((f64::INFINITY, 0.0f64), |(a, b), &x| (a.min(x), b.max(x.abs())));
    if lo <= 0.0 {
        return Err(format!("GEVP: C(t0 = {t0}) is not positive definite (smallest eigenvalue {lo:.3e})"));
    }
    let cond = hi / lo;
    if cond > GEVP_MAX_CONDITION {
        return Err(format!("GEVP: C(t0 = {t0}) has condition number {cond:.3e} > {GEVP_MAX_CONDITION:e}"));
    }
    let l = cholesky(&c[t0]).ok_or("GEVP: Cholesky of C(t0) failed")?;
    // M = L⁻¹ C(t1) L⁻ᵀ
    let solve_lower = |b: &[f64]| -> Vec<f64> {
        let mut y = vec![0.0; k];
        for i in 0..k {
            let mut v = b[i];
            for q in 0..i {
                v -= l[i][q] * y[q];
            }
            y[i] = v / l[i][i];
        }
        y
    };
    let cols: Vec<Vec<f64>> = (0..k).map(|j| solve_lower(&(0..k).map(|i| c[t1][i][j]).collect::<Vec<_>>())).collect();
    // cols[j] = L⁻¹ C(t1) e_j ; now M = (L⁻¹ (L⁻¹ C(t1))ᵀ)ᵀ
    let mut m = vec![vec![0.0; k]; k];
    for i in 0..k {
        let row: Vec<f64> = (0..k).map(|j| cols[j][i]).collect(); // (L⁻¹ C(t1))[i][·]
        let y = solve_lower(&row); // L⁻¹ applied to row i of (L⁻¹C)ᵀ ... symmetric result
        for j in 0..k {
            m[i][j] = y[j];
        }
    }
    for i in 0..k {
        for j in 0..i {
            let a = 0.5 * (m[i][j] + m[j][i]);
            m[i][j] = a;
            m[j][i] = a;
        }
    }
    let (ev, vecs) = jacobi_eigen(m);
    let mut idx: Vec<usize> = (0..k).collect();
    idx.sort_by(|&a, &b| ev[b].partial_cmp(&ev[a]).unwrap());
    let w: Vec<f64> = (0..k).map(|i| vecs[i][idx[0]]).collect();
    // v = L⁻ᵀ w
    let mut v = vec![0.0; k];
    for i in (0..k).rev() {
        let mut s = w[i];
        for q in i + 1..k {
            s -= l[q][i] * v[q];
        }
        v[i] = s / l[i][i];
    }
    let norm: f64 = (0..k).map(|a| (0..k).map(|b| v[a] * c[t0][a][b] * v[b]).sum::<f64>()).sum::<f64>().sqrt();
    let v: Vec<f64> = v.iter().map(|x| x / norm).collect();
    Ok(Gevp { t0, t1, eigenvalues: idx.iter().map(|&i| ev[i]).collect(), vector: v, condition_t0: cond })
}

/// The operator series projected on a GEVP vector: p_c(t) = Σ_a v_a O_a(c, t).
pub fn project(ops: &[Vec<Vec<f64>>], v: &[f64]) -> Vec<Vec<f64>> {
    let (n, l) = (ops[0].len(), ops[0][0].len());
    (0..n).map(|c| (0..l).map(|t| ops.iter().zip(v).map(|(o, va)| va * o[c][t]).sum()).collect()).collect()
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

    /// A single-mass process: Gaussian time series whose covariance is exactly the periodic
    /// cosh kernel cosh(m(d − L/2)) (the lattice Green's function shape), via Cholesky.
    fn cosh_ensemble(m: f64, l: usize, n: usize, seed: u64) -> Vec<Vec<f64>> {
        let k = |d: usize| (m * (d as f64 - l as f64 / 2.0)).cosh() / (m * l as f64 / 2.0).cosh();
        let mut ch = vec![vec![0.0; l]; l];
        for i in 0..l {
            for j in 0..=i {
                let d = (i as isize - j as isize).unsigned_abs();
                let mut v = k(d.min(l - d));
                for q in 0..j {
                    v -= ch[i][q] * ch[j][q];
                }
                ch[i][j] = if i == j { v.sqrt() } else { v / ch[j][j] };
            }
        }
        let mut s = seed;
        let mut gauss = || {
            let u = (lcg(&mut s) + 0.5).max(1e-300);
            let v = lcg(&mut s) + 0.5;
            (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
        };
        (0..n)
            .map(|_| {
                let z: Vec<f64> = (0..l).map(|_| gauss()).collect();
                (0..l).map(|i| 1.0 + (0..=i).map(|q| ch[i][q] * z[q]).sum::<f64>()).collect()
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
    fn amended_rule_needs_signal_and_tolerates_two_sigma() {
        // m_eff on the true value with one 1.2-sigma step: as fixed breaks, amended accepts
        let m = vec![None, Some(1.0), Some(1.0), Some(1.024), Some(1.0), None, None];
        let e = vec![0.0, 0.01, 0.01, 0.01, 0.01, 0.0, 0.0];
        let c = vec![1.0, 0.4, 0.15, 0.06, 0.02, 0.008, 0.003];
        let ce = vec![0.001; 7];
        assert!(plateau_with(&m, &e, &c, &ce, PlateauRule::AS_FIXED).is_err());
        let p = plateau_with(&m, &e, &c, &ce, PlateauRule::AMENDED).unwrap();
        assert_eq!((p.t_lo, p.t_hi), (1, 4));
        // the same masses where C(3) is only 2 sigma from zero: amended refuses, naming t = 2
        let mut ce2 = ce.clone();
        ce2[3] = 0.03;
        let r = plateau_with(&m, &e, &c, &ce2, PlateauRule::AMENDED).unwrap_err();
        assert_eq!(r.t, 2, "{r}");
    }

    #[test]
    fn signal_ensemble_has_a_plateau_and_the_shuffled_null_refuses() {
        let p = cosh_ensemble(0.5, 16, 4000, 7);
        let r = effective_mass(&p).unwrap();
        let pl = r.plateau.clone().unwrap();
        assert!((pl.mass - 0.5).abs() < 3.0 * pl.error, "{pl:?}");
        let rb = effective_mass_blocked(&p, 40).unwrap();
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

    #[test]
    fn gevp_removes_an_exact_excited_state() {
        // two states, two operators: C_ab(t) = sum_n Z_an Z_bn (e^{-m_n t} + e^{-m_n (L-t)})
        let (l, m) = (24usize, [0.4f64, 1.1f64]);
        let z = [[1.0, 0.8], [0.6, -0.9]];
        let c: Vec<Vec<Vec<f64>>> = (0..l)
            .map(|t| {
                (0..2)
                    .map(|a| {
                        (0..2)
                            .map(|b| (0..2).map(|s| z[a][s] * z[b][s] * ((-m[s] * t as f64).exp() + (-m[s] * (l - t) as f64).exp())).sum())
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let g = gevp(&c, 1, 2).unwrap();
        let proj: Vec<f64> = (0..l).map(|t| (0..2).map(|a| (0..2).map(|b| g.vector[a] * c[t][a][b] * g.vector[b]).sum::<f64>()).sum()).collect();
        for (t, me) in effective_mass_cosh(&proj).iter().enumerate() {
            if (1..l / 2).contains(&t) {
                assert!((me.unwrap() - m[0]).abs() < 1e-9, "t={t} {me:?}");
            }
        }
        // a single operator is contaminated at small t
        let diag: Vec<f64> = (0..l).map(|t| c[t][0][0]).collect();
        assert!(effective_mass_cosh(&diag)[1].unwrap() - m[0] > 0.05);
        // a singular C(t0) refuses
        let sing: Vec<Vec<Vec<f64>>> = (0..l).map(|t| vec![vec![c[t][0][0]; 2]; 2]).collect();
        assert!(gevp(&sing, 1, 2).is_err());
    }
}
