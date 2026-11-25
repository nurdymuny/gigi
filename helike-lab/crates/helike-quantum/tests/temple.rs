//! Contract tests for the Kato–Temple ordering (docs/research/helike-temple-ordering-registration-2026-11-25.md, §7).
//! Manufactured cases only: every number has a closed form, and every registered fault must break one assertion.

use helike_quantum::interval::{Cx, Iv, CZERO};
use helike_quantum::residual::{self, Group, Space};
use helike_quantum::temple::{self, Separation, StateBounds, TempleMutation as M};

fn planted() -> (Space, Vec<Cx>, Vec<Group>) {
    // P = h_0 (Gaussian), one group exp(0.3 z_0) h_1(z_0), w = 1: s = e^{α²/4} α/√2 exactly
    let space = Space::new([1.0; 6], 3, 1).unwrap();
    let mut p = space.zeros();
    p[space.index[&[0u8; 6]]] = Cx::real(Iv::point(1.0));
    let mut q = vec![CZERO; space.len()];
    q[space.index[&[1u8, 0, 0, 0, 0, 0]]] = Cx::real(Iv::point(1.0));
    let mut alpha = [Iv::point(0.0); 6];
    alpha[0] = Iv::point(0.3);
    (space, p, vec![Group { key: [0, 0, 0], alpha, coeffs: q }])
}

const S_EXACT: f64 = 0.216_959_4; // e^{0.0225} · 0.3/√2 = 0.2169594...

fn s_exact() -> f64 {
    (0.0225f64).exp() * 0.3 / 2f64.sqrt()
}

#[test]
fn planted_linear_moment_is_enclosed() {
    let (space, p, groups) = planted();
    let s = temple::linear_moment(&space, &p, &groups, residual::Mutation::None).unwrap()[0];
    assert!(s.contains(s_exact()), "{s:?} vs {}", s_exact());
    assert!(s.width() < 1e-14);
    assert!((s_exact() - S_EXACT).abs() < 1e-6);
    // the linear moment's own faults move it off the exact value
    for m in [residual::Mutation::DropExponential, residual::Mutation::DropCross] {
        let v = temple::linear_moment(&space, &p, &groups, m).unwrap()[0];
        assert!(!v.contains(s_exact()), "{m:?} was not caught");
    }
}

fn st(name: &str, center: f64, r: f64, ell: f64, t: f64, c: f64, q: f64) -> StateBounds {
    StateBounds {
        name: name.into(),
        center,
        s: [0.0, 0.0],
        r_upper: r,
        leakage_upper: ell,
        tail_upper: t,
        charge_upper: c,
        q_lower: q,
        residual_upper: (r + ell) / q * (1.0 + 1e-15),
    }
}

/// e = 2tr + ℓ + c(r + ℓ) = 0.01 + 0.01 + 0.0022 = 0.0222; μ ∈ ±e/q² = ±0.0274074…
fn lemma1_case() -> StateBounds {
    st("x", 0.0, 0.1, 0.01, 0.05, 0.02, 0.9)
}

fn lemma1_holds(mu: Iv) -> bool {
    let e = 0.0222 / 0.81;
    mu.lo <= -e && mu.hi >= e && mu.hi < e * (1.0 + 1e-12) && mu.lo > -e * (1.0 + 1e-12)
}

#[test]
fn lemma1_keeps_every_term() {
    assert!(lemma1_holds(temple::lemma1_mu(&lemma1_case(), M::None).unwrap()));
}

fn temple_holds(l: f64) -> bool {
    // centre 1, μ ∈ [−0.01, 0.02], R = 0.1, ρ = 2: L = 0.99 − 0.01/0.98
    let want = 0.99 - 0.01 / 0.98;
    l <= want && l > want - 1e-12
}

#[test]
fn temple_lower_bound_and_refusal() {
    let l = temple::temple_lower(1.0, Iv::new(-0.01, 0.02), 0.1, Iv::point(2.0), M::None).unwrap();
    assert!(temple_holds(l), "{l}");
    assert!(temple::temple_lower(1.0, Iv::new(-0.01, 0.02), 0.1, Iv::point(1.01), M::None).is_err());
}

/// Ground at 0, Helike pair at 1.088, competitor at 1.096, frequencies of the controls: ORDERED under S_C.
fn ordered_case(ground_r: f64, comp_center: f64) -> [StateBounds; 4] {
    [
        st("ground", 0.0, ground_r, 1e-9, 1e-15, 1e-3, 0.99999),
        st("helike_plus", 1.088, 0.009, 1e-9, 1e-15, 1e-3, 0.99999),
        st("helike_minus", 1.088, 0.009, 1e-9, 1e-15, 1e-3, 0.99999),
        st("competitor", comp_center, 0.018, 1e-9, 1e-15, 2e-3, 0.99999),
    ]
}

const W: [f64; 6] = [1.0901, 1.0901, 1.1067, 2.4297, 2.4297, 2.6777];
const WH: f64 = 1.0901;

#[test]
fn ordered_only_with_the_separation_hypothesis() {
    let d = temple::decide(&ordered_case(0.006, 1.096), W, WH, Some(Separation::SC), M::None).unwrap();
    assert_eq!(d.verdict, "ORDERED", "{d:?}");
    assert!(d.lemma3);
    let d = temple::decide(&ordered_case(0.006, 1.096), W, WH, None, M::None).unwrap();
    assert_eq!(d.verdict, "UNRESOLVED_NO_SEPARATION");
}

#[test]
fn overlapping_windows_are_unresolved() {
    // ground window reaches the competitor's: Lemma 3 fails
    let d = temple::decide(&ordered_case(1.2, 1.096), W, WH, Some(Separation::SC), M::None).unwrap();
    assert!(!d.lemma3);
    assert_eq!(d.verdict, "UNRESOLVED");
}

#[test]
fn the_reverse_order_is_never_claimed() {
    // competitor centre below Helike's: the rule can only say UNRESOLVED
    let d = temple::decide(&ordered_case(0.006, 1.080), W, WH, Some(Separation::SC), M::None).unwrap();
    assert_eq!(d.verdict, "UNRESOLVED");
}

/// Each registered fault (contract §7.3) must break one of the assertions above.
#[test]
fn every_registered_fault_is_caught() {
    let mut caught = vec![];
    for m in [M::DropTailTerm, M::DropLeakageTerm, M::DropChargeTerm, M::CentreForQuotient, M::NoQSquared] {
        caught.push((m, !lemma1_holds(temple::lemma1_mu(&lemma1_case(), m).unwrap())));
    }
    let l = temple::temple_lower(1.0, Iv::new(-0.01, 0.02), 0.1, Iv::point(2.0), M::LowerMuInDenominator).unwrap();
    caught.push((M::LowerMuInDenominator, !temple_holds(l)));
    let skip = temple::decide(&ordered_case(1.2, 1.096), W, WH, Some(Separation::SC), M::SkipLemma3).unwrap();
    caught.push((M::SkipLemma3, skip.verdict == "ORDERED"));
    let nosep = temple::decide(&ordered_case(0.006, 1.096), W, WH, None, M::IgnoreMissingSeparation).unwrap();
    caught.push((M::IgnoreMissingSeparation, nosep.verdict == "ORDERED"));
    let rev = temple::decide(&ordered_case(0.006, 1.080), W, WH, Some(Separation::SC), M::ClaimReverse).unwrap();
    caught.push((M::ClaimReverse, rev.verdict != "UNRESOLVED"));
    for (m, c) in &caught {
        println!("{m:?}: {}", if *c { "caught" } else { "NOT CAUGHT" });
    }
    assert_eq!(caught.len(), 9);
    assert!(caught.iter().all(|(_, c)| *c));
}
