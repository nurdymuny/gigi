//! Causal States CV4 — regime classifier.
//!
//! The paper's central diagnostic question, made operational: given a
//! [`Commutator`] result, is the system in the **sofic** regime (saturating,
//! KL diverges, TV near 1), the **smooth** regime (finite KL, small TV),
//! or in a **borderline** band where the classification is ambiguous?
//!
//! Decision rule (paper §6.5, calibrated against H5/R1/R2):
//!
//! 1. If `kl == KlValue::Divergent` → `Sofic`.
//! 2. Else if `tv ≥ tv_high_threshold` → `Sofic` (saturated TV with finite KL
//!    is rare but possible at the regime crossover; treat as sofic).
//! 3. Else if `tv ≤ tv_low_threshold` → `Smooth`.
//! 4. Otherwise → `Borderline`.
//!
//! Defaults: `tv_high = 0.95`, `tv_low = 0.30`. These bands come from the
//! paper's noisy-HMM scan (§6.4): for (α, β) in (0.05, 0.45)², TV stays
//! below 0.25 (smooth); the Even Process at any interior belief sits at
//! TV = 1 (sofic). The borderline band (0.30, 0.95) is what gets reported
//! to the operator as "needs disambiguating model selection."

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    classify_regime, commutator, EvenU0, EvenU1, HmmUpdate, Regime, RegimeBands,
};

const EPS_TIGHT: f64 = 1e-12;

// ─── Sofic: Even Process at interior beliefs ─────────────────────────────

#[test]
fn cv4_even_process_at_mu_classifies_sofic() {
    let mu = vec![2.0 / 3.0, 1.0 / 3.0];
    let omega = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    let regime = classify_regime(&omega, RegimeBands::default());
    assert_eq!(regime, Regime::Sofic,
               "Even Process @ μ should be Sofic (KL Divergent), got {regime:?}");
}

#[test]
fn cv4_even_process_at_half_classifies_sofic() {
    let half = vec![0.5, 0.5];
    let omega = commutator(&EvenU0, &EvenU1, &half).unwrap();
    let regime = classify_regime(&omega, RegimeBands::default());
    assert_eq!(regime, Regime::Sofic);
}

// ─── Smooth: HMM in non-synchronizing regime ─────────────────────────────

#[test]
fn cv4_hmm_at_reference_point_classifies_smooth() {
    // (α, β) = (0.2, 0.3) at μ = (.5, .5): TV ≈ 0.1062, KL finite.
    let mu = vec![0.5, 0.5];
    let u_0 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.2, beta: 0.3, symbol: 1 };
    let omega = commutator(&u_0, &u_1, &mu).unwrap();
    let regime = classify_regime(&omega, RegimeBands::default());
    assert_eq!(regime, Regime::Smooth,
               "HMM @ (0.2, 0.3) should be Smooth (TV ≈ 0.106), got {regime:?}");
}

#[test]
fn cv4_hmm_at_alpha_half_classifies_smooth() {
    // α = 1/2 → iid: TV = 0 exactly. Definitely Smooth.
    let mu = vec![0.5, 0.5];
    let u_0 = HmmUpdate { alpha: 0.5, beta: 0.3, symbol: 0 };
    let u_1 = HmmUpdate { alpha: 0.5, beta: 0.3, symbol: 1 };
    let omega = commutator(&u_0, &u_1, &mu).unwrap();
    let regime = classify_regime(&omega, RegimeBands::default());
    assert_eq!(regime, Regime::Smooth);
}

// ─── Borderline ──────────────────────────────────────────────────────────

#[test]
fn cv4_synthetic_borderline_classifies_borderline() {
    // Construct a synthetic Commutator with TV = 0.6, finite KL, to force
    // the borderline branch directly (no operator in our test set produces
    // this naturally — the paper's §6.5 calibration shows H^TV smoothly
    // bridges 0 → 1 across the parameter space, so the borderline band is
    // possible at the regime crossover).
    use gigi::causal_states::{Commutator, KlValue};
    let omega = Commutator {
        forward: vec![0.7, 0.3],
        backward: vec![0.1, 0.9],
        tv: 0.6,
        hellinger: 0.5,
        kl: KlValue::Finite(0.4),
    };
    assert_eq!(classify_regime(&omega, RegimeBands::default()), Regime::Borderline);
}

// ─── Custom bands ────────────────────────────────────────────────────────

#[test]
fn cv4_custom_bands_shift_thresholds() {
    use gigi::causal_states::{Commutator, KlValue};
    let omega = Commutator {
        forward: vec![0.6, 0.4],
        backward: vec![0.4, 0.6],
        tv: 0.2,
        hellinger: 0.18,
        kl: KlValue::Finite(0.05),
    };
    // Default tv_low = 0.30 → 0.2 is Smooth.
    assert_eq!(classify_regime(&omega, RegimeBands::default()), Regime::Smooth);
    // Tighten tv_low to 0.10 → same omega becomes Borderline.
    let tight = RegimeBands { tv_low: 0.10, tv_high: 0.95 };
    assert_eq!(classify_regime(&omega, tight), Regime::Borderline);
}

#[test]
fn cv4_high_tv_with_finite_kl_classifies_sofic() {
    // TV ≥ tv_high with finite KL also routes to Sofic (saturation-as-tell).
    use gigi::causal_states::{Commutator, KlValue};
    let omega = Commutator {
        forward: vec![0.98, 0.02],
        backward: vec![0.02, 0.98],
        tv: 0.96,
        hellinger: 0.93,
        kl: KlValue::Finite(5.0),
    };
    assert_eq!(classify_regime(&omega, RegimeBands::default()), Regime::Sofic);
}

// ─── Band invariants ─────────────────────────────────────────────────────

#[test]
fn cv4_default_bands_within_unit_interval() {
    let bands = RegimeBands::default();
    assert!(bands.tv_low > 0.0 && bands.tv_low < 1.0);
    assert!(bands.tv_high > bands.tv_low && bands.tv_high < 1.0 + EPS_TIGHT);
}
