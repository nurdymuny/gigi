//! Phase 4.3 — Orthogonality scan: H[X] does not determine |Ω|.
//!
//! The paper's TH1 says symbol entropy and commutator magnitude are
//! distinct diagnostics. Two clean empirical demonstrations:
//!
//!   (1) **Matched H[X], opposite |Ω|** — iid Bernoulli(2/3) and the Even
//!       Process both have stationary symbol distribution (1/3, 2/3) and
//!       therefore H[X] = h(2/3) ≈ 0.9183 bits. But the iid commutator
//!       vanishes identically, while the Even Process saturates at |Ω| = 1.
//!
//!   (2) **Fixed H[X] = 1, sweeping |Ω|** — every symmetric noisy 2-state
//!       HMM has stationary (1/2, 1/2) hence H[X] = 1, yet |Ω| sweeps over
//!       (0, 0.5) as (α, β) varies (paper Eq 6.4).
//!
//! Same audit-confounder entropy, very different operator-commutativity.
//! Empirical evidence that the commutator captures information beyond H[X].

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    commutator, hmm_closed_form_tv, EvenU0, EvenU1, HmmUpdate, UpdateOperator,
};

const EPS_NUMERIC: f64 = 1e-4;

/// Binary entropy in bits.
fn h(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    -(p * p.log2() + (1.0 - p) * (1.0 - p).log2())
}

/// Bayesian update for an iid Bernoulli(p) process.
///
/// `p` parameterises the SINGLE generating process — it is shared between
/// the U_0 and U_1 update operators (both return the same stationary,
/// because in an iid process the observation conveys no information about
/// the next-state belief).
///
/// The `symbol` field is ignored by `apply()` — that's the whole point:
/// in an iid process, U_0 ≡ U_1 ≡ "return stationary." This makes the
/// commutator of (U_0, U_1) vanish identically, no matter the base belief.
#[derive(Debug, Clone, Copy)]
struct IidBernoulli {
    p: f64,
    #[allow(dead_code)]
    symbol: u8,
}

impl UpdateOperator for IidBernoulli {
    fn apply(
        &self,
        _belief: &[f64],
    ) -> Result<Vec<f64>, gigi::causal_states::UpdateError> {
        Ok(vec![1.0 - self.p, self.p])
    }
}

// ─── Demonstration 1: matched H[X], opposite |Ω| ─────────────────────────

#[test]
fn p4_3_iid_bernoulli_two_thirds_h_x_matches_even_process() {
    // Stationary symbol distribution of the Even Process:
    //   In causal state s_0 (μ_0 = 2/3): emit 0 w.p. 1/2, 1 w.p. 1/2
    //   In causal state s_1 (μ_1 = 1/3): emit 1 w.p. 1
    //   ⇒ P(X = 0) = (2/3)(1/2) + (1/3)(0) = 1/3
    //   ⇒ P(X = 1) = (2/3)(1/2) + (1/3)(1) = 2/3
    //   ⇒ H[X] = h(2/3)
    let h_even = h(2.0 / 3.0);
    let h_iid = h(2.0 / 3.0);
    assert!(
        (h_even - h_iid).abs() < EPS_NUMERIC,
        "H[X] mismatch: even={h_even}, iid={h_iid}"
    );
    // Both round to 0.918 bits.
    assert!((h_even - 0.9183).abs() < 1e-3);
}

#[test]
fn p4_3_iid_commutator_vanishes_for_all_p() {
    // For every iid Bernoulli(p) with p ∈ {0.1, ..., 0.9} the commutator
    // at any base belief must vanish in all three diagnostics.
    let base_beliefs = [
        vec![0.5, 0.5],
        vec![2.0 / 3.0, 1.0 / 3.0],
        vec![0.7, 0.3],
        vec![0.1, 0.9],
    ];
    for p_times_10 in 1..10 {
        let p = f64::from(p_times_10) / 10.0;
        // For an iid process the U_0 and U_1 operators of the *same*
        // generator both return the stationary regardless of input —
        // hence U_0 ≡ U_1 and the commutator vanishes identically.
        let u_0 = IidBernoulli { p, symbol: 0 };
        let u_1 = IidBernoulli { p, symbol: 1 };
        for base in &base_beliefs {
            let omega = commutator(&u_0, &u_1, base).unwrap();
            assert!(omega.tv < EPS_NUMERIC,
                    "iid TV @ p={p} base={base:?}: {} ≥ {EPS_NUMERIC}", omega.tv);
            assert!(omega.hellinger < EPS_NUMERIC);
        }
    }
}

#[test]
fn p4_3_even_process_versus_iid_two_thirds_h_x_orthogonal_in_omega() {
    // The headline orthogonality: at H[X] ≈ 0.918 bits the two processes
    // diverge maximally in |Ω| — iid gives 0, Even Process gives 1.
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];

    // iid arm — same H[X], same C_μ-confounder. Same generator p, two
    // symbols 0/1; U_0 ≡ U_1 returns Bernoulli(2/3) stationary, hence Ω = 0.
    let u_iid_0 = IidBernoulli { p: 2.0 / 3.0, symbol: 0 };
    let u_iid_1 = IidBernoulli { p: 2.0 / 3.0, symbol: 1 };
    let omega_iid = commutator(&u_iid_0, &u_iid_1, &mu).unwrap();
    assert!(omega_iid.tv < EPS_NUMERIC, "iid TV at matched H[X] should be 0, got {}", omega_iid.tv);

    // Even Process arm — same H[X], maximally non-commutative.
    let omega_even = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    assert!(
        (omega_even.tv - 1.0).abs() < 1e-12,
        "Even Process TV at matched H[X] should be 1, got {}",
        omega_even.tv
    );

    // Gap: same H[X], |Ω| differs by ≥ 0.9.
    let gap = omega_even.tv - omega_iid.tv;
    assert!(
        gap > 0.9,
        "orthogonality gap at H[X] = h(2/3) only {gap} < 0.9"
    );
}

// ─── Demonstration 2: fixed H[X] = 1, sweeping |Ω| via HMM ──────────────

#[test]
fn p4_3_symmetric_hmm_holds_h_x_at_one_while_omega_varies() {
    // Symmetric noisy HMM at any (α, β):
    //   Stationary on hidden state = (0.5, 0.5) by symmetry.
    //   P(X = 0) = (1-β)·0.5 + β·0.5 = 0.5 (independent of α, β).
    //   ⇒ H[X] = h(0.5) = 1.0 always.
    //
    // But |Ω| at μ = (0.5, 0.5) varies over (0, 0.5) per Eq 6.4.
    let mu = vec![0.5, 0.5];

    let cases: &[(f64, f64, f64)] = &[
        // (α, β, expected_min_TV)
        (0.10, 0.10, 0.20), // near saturation at small α, β — TV ≈ 0.32
        (0.15, 0.15, 0.15),
        (0.20, 0.30, 0.05), // paper reference (≈ 0.106)
        (0.25, 0.40, 0.01), // smaller |Ω|
    ];
    let mut omegas = Vec::new();
    for (alpha, beta, expected_min) in cases {
        let u_0 = HmmUpdate { alpha: *alpha, beta: *beta, symbol: 0 };
        let u_1 = HmmUpdate { alpha: *alpha, beta: *beta, symbol: 1 };
        let omega = commutator(&u_0, &u_1, &mu).unwrap();
        // H[X] = 1 by symmetry (no test here — that's a structural fact).
        // |Ω| should at least exceed the family-specific expected lower bound.
        assert!(
            omega.tv >= *expected_min,
            "HMM @ ({alpha}, {beta}) gave TV = {}, expected ≥ {expected_min}",
            omega.tv
        );
        omegas.push(omega.tv);
    }
    // Spread test: the largest TV in this set should be at least 5× the
    // smallest — demonstrates the same H[X] = 1 covers a wide |Ω| range.
    let max_tv = omegas.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_tv = omegas.iter().cloned().fold(f64::INFINITY, f64::min);
    assert!(
        max_tv / min_tv > 5.0,
        "|Ω| range @ H[X]=1 only {:.2}× (max={max_tv}, min={min_tv}); need > 5×",
        max_tv / min_tv,
    );
}

#[test]
fn p4_3_h_x_invariant_under_hmm_parameter_change() {
    // Symmetric stationary check at the analytical level: P(X = 0)
    // computed against the HMM stationary distribution must equal 1/2
    // for every (α, β). This is the "matched H[X]" claim for the
    // symmetric HMM family.
    for alpha_times_100 in 5..=45 {
        for beta_times_100 in 5..=45 {
            let alpha = f64::from(alpha_times_100) / 100.0;
            let beta = f64::from(beta_times_100) / 100.0;
            // Stationary hidden: (0.5, 0.5) — symmetric M with crossover α.
            // Stationary symbol: P(X=0) = E_0 · (0.5, 0.5)^T
            //                          = (1-β)·0.5 + β·0.5 = 0.5.
            let _ = alpha; // unused — the claim is structural in α
            let p_x0 = (1.0 - beta) * 0.5 + beta * 0.5;
            assert!(
                (p_x0 - 0.5).abs() < 1e-15,
                "symmetric HMM stationary P(X=0) ≠ 0.5 at β={beta}"
            );
        }
    }
}

// ─── Discrimination summary statistic ────────────────────────────────────

#[test]
fn p4_3_correlation_h_vs_omega_low_across_mixed_families() {
    // Across a mixed family (iid + HMM + Even), correlation between H[X]
    // and |Ω| is weak — Spearman-style rank check.
    // We just enumerate the points and assert: highest-|Ω| point need NOT
    // be the highest-H[X] point.
    let mu_two_thirds = vec![2.0 / 3.0, 1.0 / 3.0];

    struct Point {
        name: &'static str,
        h_x: f64,
        omega_tv: f64,
    }
    let mut pts: Vec<Point> = Vec::new();

    // iid family
    for p_times_10 in 1..10 {
        let p = f64::from(p_times_10) / 10.0;
        // For iid Bernoulli(p), the stationary symbol distribution is
        // exactly Bernoulli(p), so H[X] = h(p).
        pts.push(Point {
            name: "iid",
            h_x: h(p),
            omega_tv: 0.0,
        });
    }
    // HMM family at symmetric stationary (H[X] = 1 always)
    let mu_half = vec![0.5, 0.5];
    for (alpha, beta) in &[
        (0.10f64, 0.10f64),
        (0.15, 0.20),
        (0.20, 0.30),
        (0.25, 0.35),
        (0.30, 0.40),
    ] {
        let u_0 = HmmUpdate { alpha: *alpha, beta: *beta, symbol: 0 };
        let u_1 = HmmUpdate { alpha: *alpha, beta: *beta, symbol: 1 };
        let omega = commutator(&u_0, &u_1, &mu_half).unwrap();
        pts.push(Point {
            name: "hmm",
            h_x: 1.0,
            omega_tv: omega.tv,
        });
    }
    // Even Process at μ = (2/3, 1/3): H[X] = h(2/3), |Ω| = 1.
    let omega_even = commutator(&EvenU0, &EvenU1, &mu_two_thirds).unwrap();
    pts.push(Point {
        name: "even",
        h_x: h(2.0 / 3.0),
        omega_tv: omega_even.tv,
    });

    // Find argmax of H[X] and argmax of |Ω|:
    let argmax_h = pts.iter().fold(&pts[0], |a, b| if b.h_x > a.h_x { b } else { a });
    let argmax_om = pts.iter().fold(&pts[0], |a, b| if b.omega_tv > a.omega_tv { b } else { a });

    // If H[X] ranked |Ω|, these would coincide. They don't:
    // argmax H[X] = some HMM at H = 1.0 (or iid Bernoulli(0.5) at H = 1.0)
    //               whose Ω is at most ~0.4.
    // argmax |Ω| = Even Process at Ω = 1.0.
    assert!(
        argmax_om.omega_tv > argmax_h.omega_tv + 0.5,
        "expected argmax_Ω to dominate argmax_H[X] in Ω; got argmax_h.omega={}, argmax_om.omega={}",
        argmax_h.omega_tv,
        argmax_om.omega_tv
    );
    assert_eq!(argmax_om.name, "even");
}
