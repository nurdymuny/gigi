//! Kato–Temple ordering of Helike against the competitor at the saved controls
//! (docs/research/helike-temple-ordering-registration-2026-11-25.md).
//!
//! - `linear_moment`: s = ⟨ψ, (H − E)ψ⟩ by the residual's exact centred moment sum, linear instead of squared:
//!   for each group, ⟨P, e^{α·z} Q⟩ = e^{Σα²/(4w)} ⟨P(z + μ), Q(z + μ)⟩ with μ = α/(2w), in centred form.
//! - `lemma1_mu`: the Rayleigh quotient of the trimmed, projected state, E + μ with
//!   μ ∈ (s ± e)/[q², 1], e = 2tr + ℓ + c(r + ℓ) (registration §2).
//! - `temple_lower`: λ* ≥ E + μ_lo − R²/(ρ − E − μ_hi) for the largest eigenvalue below ρ (registration §3).
//! - `decide`: Lemma 3 (two disjoint charge-0 Weinstein windows below ρ_C), the frozen ρ_C, and the verdict
//!   ORDERED (conditional on (A) and S_C) or UNRESOLVED. The reverse order is never claimed.
//!
//! Every quantity is an outward-rounded interval. `TempleMutation` carries the nine registered faults.

use crate::interval::{Cx, Iv, ONE, ZERO};
use crate::projection::{self, ArbControl};
use crate::residual::{self, tree_sum, Group, Mutation, SavedControl, Space, EXPONENT_LIMIT};
use rayon::prelude::*;
use serde::Serialize;

/// The registered faults (§7.3); each must be caught by an assertion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TempleMutation {
    None,
    DropTailTerm,
    DropLeakageTerm,
    DropChargeTerm,
    /// use the centre E in place of the Rayleigh quotient E + μ
    CentreForQuotient,
    /// μ_lo in the Temple denominator
    LowerMuInDenominator,
    /// omit the q² division
    NoQSquared,
    SkipLemma3,
    /// claim ORDERED without S_C
    IgnoreMissingSeparation,
    /// claim the reverse order when not ordered
    ClaimReverse,
}

/// The frozen separation hypothesis of §5. Supplying it is an explicit act; without it nothing is ORDERED.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum Separation {
    /// in charge 0, at most two eigenvalues lie below ρ_C = E₀ᶜ + ω_H⁺ + ω_H⁻ − ω_min/4
    SC,
}

fn cols(space: &Space, f: impl Fn(usize) -> Iv) -> Vec<Iv> {
    let k = space.ncol;
    let n = space.states.len();
    (0..k).map(|j| tree_sum(&(0..n).map(|s| f(s * k + j)).collect::<Vec<_>>())).collect()
}

/// s_j = Re⟨P_j, Σ_a e^{α_a·z} Q_a,j⟩ in the Gaussian measure (unnormalised), per column.
pub fn linear_moment(space: &Space, p: &[Cx], groups: &[Group], mutation: Mutation) -> Result<Vec<Iv>, String> {
    if groups.is_empty() {
        return Err("nonempty exponential-polynomial required".into());
    }
    let conj = |a: &Cx, b: &Cx| -> Iv {
        if mutation == Mutation::NoConjugate {
            a.re * b.re - a.im * b.im
        } else {
            a.re_conj_mul(b)
        }
    };
    let len = space.len();
    let combined: Vec<Cx> = (0..len)
        .map(|i| {
            Cx::new(
                tree_sum(&groups.iter().map(|g| g.coeffs[i].re).collect::<Vec<_>>()),
                tree_sum(&groups.iter().map(|g| g.coeffs[i].im).collect::<Vec<_>>()),
            )
        })
        .collect();
    let base = cols(space, |i| conj(&p[i], &combined[i]));
    let parts: Vec<Result<Vec<Iv>, String>> = groups
        .par_iter()
        .map(|g| {
            let e = tree_sum(&(0..6).map(|k| g.alpha[k].sqr() / (Iv::int(4) * Iv::point(space.w[k]))).collect::<Vec<_>>());
            if !(e.hi <= EXPONENT_LIMIT) {
                return Err("completed-square factor exceeds numerical contract".into());
            }
            let mu: [Iv; 6] = std::array::from_fn(|k| g.alpha[k] / (Iv::int(2) * Iv::point(space.w[k])));
            let dp = space.shift_increment(p, &mu);
            let dq = space.shift_increment(&g.coeffs, &mu);
            let baseline = cols(space, |s| conj(&p[s], &g.coeffs[s]));
            let cross = cols(space, |s| conj(&p[s], &dq[s]) + conj(&dp[s], &g.coeffs[s]) + conj(&dp[s], &dq[s]));
            let em = if mutation == Mutation::DropExponential { ZERO } else { e.expm1() };
            Ok((0..space.ncol)
                .map(|c| {
                    let cr = if mutation == Mutation::DropCross { ZERO } else { cross[c] };
                    cr + em * (baseline[c] + cr)
                })
                .collect())
        })
        .collect();
    let mut per_col: Vec<Vec<Iv>> = (0..space.ncol).map(|c| vec![base[c]]).collect();
    for r in parts {
        let r = r?;
        for c in 0..space.ncol {
            per_col[c].push(r[c]);
        }
    }
    let out: Vec<Iv> = per_col.iter().map(|v| tree_sum(v)).collect();
    if out.iter().any(|v| !v.is_finite()) {
        return Err("nonfinite linear moment".into());
    }
    Ok(out)
}

/// Rigorous inputs for one state, all for the normalised ψ.
#[derive(Clone, Debug, Serialize)]
pub struct StateBounds {
    pub name: String,
    pub center: f64,
    /// s / ‖ψ‖², enclosed
    pub s: [f64; 2],
    pub r_upper: f64,
    pub leakage_upper: f64,
    pub tail_upper: f64,
    pub charge_upper: f64,
    pub q_lower: f64,
    /// R = (r + ℓ)/q ≥ ‖(H − E)φ‖
    pub residual_upper: f64,
}

/// Lemma 1: μ = ⟨φ, (H − E)φ⟩ ∈ (s ± e)/[q², 1], e = 2tr + ℓ + c(r + ℓ).
pub fn lemma1_mu(b: &StateBounds, m: TempleMutation) -> Result<Iv, String> {
    if m == TempleMutation::CentreForQuotient {
        return Ok(ZERO);
    }
    let (r, l, t, c) = (Iv::point(b.r_upper), Iv::point(b.leakage_upper), Iv::point(b.tail_upper), Iv::point(b.charge_upper));
    let mut e = ZERO;
    if m != TempleMutation::DropTailTerm {
        e = e + Iv::int(2) * t * r;
    }
    if m != TempleMutation::DropLeakageTerm {
        e = e + l;
    }
    if m != TempleMutation::DropChargeTerm {
        e = e + c * (r + l);
    }
    let e = Iv::point(e.hi);
    let lo = Iv::point(b.s[0]) - e;
    let hi = Iv::point(b.s[1]) + e;
    if !(b.q_lower > 0.0 && b.q_lower <= 1.0) {
        return Err("q_lower must lie in (0, 1]".into());
    }
    let q2 = if m == TempleMutation::NoQSquared { ONE } else { Iv::point(b.q_lower).sqr() };
    let a = Iv::point(lo.lo) / Iv::point(q2.lo);
    let z = Iv::point(hi.hi) / Iv::point(q2.lo);
    Ok(Iv::new(lo.lo.min(a.lo), hi.hi.max(z.hi)))
}

/// Lemma 2: the largest eigenvalue below ρ is ≥ E + μ_lo − R²/(ρ − E − μ_hi). Refuses on a nonpositive denominator.
pub fn temple_lower(center: f64, mu: Iv, big_r: f64, rho: Iv, m: TempleMutation) -> Result<f64, String> {
    let mu_den = if m == TempleMutation::LowerMuInDenominator { mu.lo } else { mu.hi };
    let den = Iv::point(rho.lo) - Iv::point(center) - Iv::point(mu_den);
    if !(den.lo > 0.0) {
        return Err("Temple denominator is not positive: refuse".into());
    }
    let l = Iv::point(center) + Iv::point(mu.lo) - Iv::point(big_r).sqr() / den;
    Ok(l.lo)
}

#[derive(Clone, Debug, Serialize)]
pub struct GapReport {
    pub hypotheses: Vec<&'static str>,
    pub ground: Option<[f64; 2]>,
    pub helike_gap: Option<[f64; 2]>,
    pub competitor_gap_lower: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Decision {
    pub mu: Vec<(String, [f64; 2])>,
    pub rho_c: [f64; 2],
    pub lemma3: bool,
    pub competitor_lower: f64,
    pub helike_upper: [f64; 2],
    pub margin_lower: f64,
    pub verdict: String,
    pub conditions: Vec<&'static str>,
    pub gap_report: GapReport,
}

/// The decision of §6 for one control. `states` in the order ground, helike_plus, helike_minus, competitor.
pub fn decide(states: &[StateBounds; 4], w: [f64; 6], omega_h: f64, sep: Option<Separation>, m: TempleMutation) -> Result<Decision, String> {
    let names = ["ground", "helike_plus", "helike_minus", "competitor"];
    for (s, n) in states.iter().zip(names) {
        if s.name != n {
            return Err(format!("state order: expected {n}, got {}", s.name));
        }
    }
    let hel: Vec<usize> = (0..6).filter(|&j| (w[j] - omega_h).abs() < 1e-6).collect();
    if hel.len() != 2 {
        return Err("Helike doublet not identified among the frequencies".into());
    }
    let wmin = w.iter().cloned().fold(f64::INFINITY, f64::min);
    let wc = (0..6).filter(|j| !hel.contains(j)).map(|j| w[j]).fold(f64::INFINITY, f64::min);
    let quarter = Iv::point(wmin) / Iv::int(4);
    let e0 = Iv::point(states[0].center);
    let rho_c = e0 + Iv::point(w[hel[0]]) + Iv::point(w[hel[1]]) - quarter;
    let rho_0 = e0 + Iv::point(wc) - quarter;
    let rho_h = e0 + Iv::point(omega_h) + Iv::point(wc) - quarter;
    let mus: Vec<Iv> = states.iter().map(|s| lemma1_mu(s, m)).collect::<Result<_, _>>()?;
    let rr: Vec<f64> = states.iter().map(|s| s.residual_upper).collect();
    let (g, hp, hm, c) = (&states[0], &states[1], &states[2], &states[3]);
    let lemma3 = m == TempleMutation::SkipLemma3
        || ((Iv::point(g.center) + Iv::point(rr[0])).hi < (Iv::point(c.center) - Iv::point(rr[3])).lo
            && (Iv::point(c.center) + Iv::point(rr[3])).hi < rho_c.lo);
    let l_c = temple_lower(c.center, mus[3], rr[3], rho_c, m)?;
    let u_hp = (Iv::point(hp.center) + Iv::point(mus[1].hi)).hi;
    let u_hm = (Iv::point(hm.center) + Iv::point(mus[2].hi)).hi;
    let u_h = u_hp.max(u_hm);
    let margin = (Iv::point(l_c) - Iv::point(u_h)).lo;
    let verdict = if sep.is_none() && m != TempleMutation::IgnoreMissingSeparation {
        "UNRESOLVED_NO_SEPARATION"
    } else if lemma3 && u_h < l_c {
        "ORDERED"
    } else if m == TempleMutation::ClaimReverse {
        "COMPETITOR_BELOW"
    } else {
        "UNRESOLVED"
    };
    // gap values (reported, not decisive): S_0 for the ground state, S_H for Helike
    let window_below = |s: &StateBounds, r: f64, rho: Iv| (Iv::point(s.center) + Iv::point(r)).hi < rho.lo;
    let l0 = if window_below(g, rr[0], rho_0) { temple_lower(g.center, mus[0], rr[0], rho_0, m).ok() } else { None };
    let u0 = (Iv::point(g.center) + Iv::point(mus[0].hi)).hi;
    let lh = if window_below(hp, rr[1], rho_h) { temple_lower(hp.center, mus[1], rr[1], rho_h, m).ok() } else { None };
    let gap_report = GapReport {
        hypotheses: vec!["(A)", "S_0", "S_H", "S_C"],
        ground: l0.map(|l| [l, u0]),
        helike_gap: match (lh, l0) {
            (Some(lh), Some(l0)) => Some([(Iv::point(lh) - Iv::point(u0)).lo, (Iv::point(u_hp) - Iv::point(l0)).hi]),
            _ => None,
        },
        competitor_gap_lower: Some((Iv::point(l_c) - Iv::point(u0)).lo),
    };
    Ok(Decision {
        mu: states.iter().zip(&mus).map(|(s, m)| (s.name.clone(), [m.lo, m.hi])).collect(),
        rho_c: [rho_c.lo, rho_c.hi],
        lemma3,
        competitor_lower: l_c,
        helike_upper: [u_hp, u_hm],
        margin_lower: margin,
        verdict: verdict.into(),
        conditions: vec!["(A) discrete spectrum in each sector (sketched, not yet written out)", "S_C"],
        gap_report,
    })
}

#[derive(Clone, Debug, Serialize)]
pub struct LinearState {
    pub name: String,
    pub center: f64,
    /// s/‖ψ‖² by the linear centred moment sum
    pub s: [f64; 2],
    /// the same by polarization of two residual enclosures (an independent route)
    pub s_polarized: [f64; 2],
}

#[derive(Clone, Debug, Serialize)]
pub struct TempleControl {
    pub g2: f64,
    #[serde(rename = "Mq")]
    pub mq: usize,
    pub polarization_shift: f64,
    pub linear: Vec<LinearState>,
    pub states: Vec<StateBounds>,
    pub decision: Decision,
    pub seconds: f64,
}

/// One control: sector bounds (the same hulled trial as the Arb record), the linear moment two ways, the decision.
pub fn temple_control(saved: &SavedControl, arb: &ArbControl, delta: f64, sep: Option<Separation>, m: TempleMutation) -> Result<TempleControl, String> {
    let t0 = std::time::Instant::now();
    let sector = projection::project_control(saved, arb, Mutation::None)?;
    let fre: Vec<f64> = arb.trial_coeff_re.iter().map(|&b| f64::from_bits(b)).collect();
    let fim: Vec<f64> = arb.trial_coeff_im.iter().map(|&b| f64::from_bits(b)).collect();
    let (space, groups, p, norms) = residual::residual_parts(saved, Some((&fre, &fim)), 0.0, Mutation::None)?;
    let s_raw = linear_moment(&space, &p, &groups, Mutation::None)?;
    let (v1, _) = residual::norm_squared(&space, &groups, Mutation::None)?;
    let (space2, groups2, _, _) = residual::residual_parts(saved, Some((&fre, &fim)), delta, Mutation::None)?;
    let (v2, _) = residual::norm_squared(&space2, &groups2, Mutation::None)?;
    let d = Iv::point(delta);
    let mut linear = vec![];
    let mut bounds = vec![];
    for (j, st) in sector.states.iter().enumerate() {
        let s = s_raw[j] / norms[j];
        let pol = (v1[j] - v2[j] + d.sqr() * norms[j]) / (Iv::int(2) * d) / norms[j];
        if s.hi < pol.lo || pol.hi < s.lo {
            return Err(format!("{}: linear moment {s:?} disagrees with its polarization {pol:?}", st.name));
        }
        linear.push(LinearState { name: st.name.clone(), center: st.center, s: [s.lo, s.hi], s_polarized: [pol.lo, pol.hi] });
        let sb = st.sector.as_ref().map_err(|e| format!("{}: no sector bound ({e})", st.name))?;
        bounds.push(StateBounds {
            name: st.name.clone(),
            center: st.center,
            s: [s.lo, s.hi],
            r_upper: st.r_upper,
            leakage_upper: st.leakage_norm_upper,
            tail_upper: st.tail_norm_upper,
            charge_upper: st.charge_upper,
            q_lower: sb.q_lower,
            residual_upper: sb.residual_upper,
        });
    }
    let arr: [StateBounds; 4] = bounds.try_into().map_err(|_| "expected four states per control".to_string())?;
    let (_, w) = residual::saved_chart(saved)?;
    let omega_h = crate::ritz::helike_frequency(saved.s, saved.t);
    let decision = decide(&arr, w, omega_h, sep, m)?;
    Ok(TempleControl {
        g2: saved.g2,
        mq: saved.mq,
        polarization_shift: delta,
        linear,
        states: arr.to_vec(),
        decision,
        seconds: t0.elapsed().as_secs_f64(),
    })
}
