//! Phase 4.4 — Blind-classification confusion matrix.
//!
//! The regime classifier (CV4) must satisfy:
//!
//!   (a) **Belief-invariance**: the regime is a property of the process,
//!       not of the base belief. classify_regime(Even Process Ω) ≡ Sofic
//!       at every reachable interior belief.
//!   (b) **Strong-criterion accuracy**: on processes whose mathematical
//!       regime is unambiguous (TV ≤ 0.1 → Smooth; KL Divergent or
//!       TV ≥ 0.97 → Sofic), the default classifier is 100% accurate.
//!   (c) **Perturbation robustness**: small noise in (α, β) does not
//!       flip clear-cut Smooth or Sofic classifications.
//!   (d) **Finite-sample robustness**: at N = 10k samples, sampling
//!       noise in the empirical commutator does not flip Smooth/Sofic
//!       on clearly-separated points (within their margins).
//!
//! Phase 4.4 is the "classifier passes its own audit" sub-phase.

#![cfg(feature = "causal_states")]

use gigi::causal_states::{
    classify_regime, commutator, hmm_closed_form_tv,
    sim::{empirical_belief_after_pair, Lcg},
    tv, EvenU0, EvenU1, HmmUpdate, Regime, RegimeBands,
};

// ─── (a) Belief-invariance for the Even Process ─────────────────────────

#[test]
fn p4_4_even_process_sofic_at_every_interior_reachable_belief() {
    // Even Process orbit: {(2/3, 1/3), (1/2, 1/2), (1, 0), (0, 1)}.
    // The two interior points (2/3, 1/3) and (1/2, 1/2) have admissible
    // commutator and must classify Sofic regardless of which we start at.
    for base in &[vec![2.0 / 3.0, 1.0 / 3.0], vec![0.5, 0.5]] {
        let omega = commutator(&EvenU0, &EvenU1, base).unwrap();
        assert_eq!(
            classify_regime(&omega, RegimeBands::default()),
            Regime::Sofic,
            "Even Process not Sofic at base {base:?}"
        );
    }
}

// ─── (b) Strong-criterion accuracy ───────────────────────────────────────

#[test]
fn p4_4_strong_smooth_criterion_classifier_accuracy() {
    // Build a labeled "Smooth" subset: HMM points with closed-form
    // TV ≤ 0.1 (well below tv_low default 0.30). Classifier must
    // return Smooth for every such point.
    let mu = vec![0.5, 0.5];
    let mut n_total = 0;
    let mut n_smooth = 0;
    for alpha_t in 5..=45u32 {
        for beta_t in 5..=45u32 {
            let alpha = f64::from(alpha_t) / 100.0;
            let beta = f64::from(beta_t) / 100.0;
            let closed = hmm_closed_form_tv(alpha, beta);
            if closed > 0.1 {
                continue;
            }
            n_total += 1;
            let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
            let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
            let omega = commutator(&u_0, &u_1, &mu).unwrap();
            if classify_regime(&omega, RegimeBands::default()) == Regime::Smooth {
                n_smooth += 1;
            }
        }
    }
    assert!(n_total > 100, "labeled-Smooth set too small: {n_total}");
    let accuracy = f64::from(n_smooth) / f64::from(n_total);
    assert!(
        accuracy >= 0.999,
        "Smooth accuracy {accuracy:.4} below threshold ({n_smooth}/{n_total})"
    );
}

#[test]
fn p4_4_strong_sofic_criterion_classifier_accuracy() {
    // Build a labeled "Sofic" subset: KL Divergent (Even Process) or
    // TV ≥ 0.97 (synthetic). Classifier must return Sofic for all.
    use gigi::causal_states::{Commutator, KlValue};

    // Even Process — natural Sofic anchor.
    let mu = vec![0.5, 0.5];
    let omega_even = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    assert_eq!(classify_regime(&omega_even, RegimeBands::default()), Regime::Sofic);

    // Synthetic high-TV-finite-KL anchors (band 2 path).
    let mut n_total = 0;
    let mut n_sofic = 0;
    for tv_step in 0..30 {
        let tv_val = 0.97 + 0.001 * f64::from(tv_step);
        if tv_val > 1.0 {
            break;
        }
        n_total += 1;
        let omega = Commutator {
            forward: vec![tv_val, 1.0 - tv_val],
            backward: vec![1.0 - tv_val, tv_val],
            tv: tv_val,
            hellinger: tv_val * 0.9,
            kl: KlValue::Finite(5.0 + f64::from(tv_step)),
        };
        if classify_regime(&omega, RegimeBands::default()) == Regime::Sofic {
            n_sofic += 1;
        }
    }
    let accuracy = f64::from(n_sofic) / f64::from(n_total);
    assert!(
        accuracy >= 0.999,
        "Sofic accuracy {accuracy:.4} below threshold ({n_sofic}/{n_total})"
    );
}

// ─── (c) Perturbation robustness ────────────────────────────────────────

#[test]
fn p4_4_smooth_stable_under_parameter_perturbation() {
    // Anchor: HMM @ (0.20, 0.30) — Smooth, TV ≈ 0.106. Perturb (α, β)
    // by up to ±0.005; classification must stay Smooth.
    let mu = vec![0.5, 0.5];
    let alpha_0 = 0.20;
    let beta_0 = 0.30;
    let mut rng = Lcg::new(0xc0_c0_a0_a0);
    let mut flipped = 0u32;
    let mut total = 0u32;
    for _ in 0..100 {
        let da = (rng.next_f64() - 0.5) * 0.01;
        let db = (rng.next_f64() - 0.5) * 0.01;
        let alpha = (alpha_0 + da).max(0.01).min(0.49);
        let beta = (beta_0 + db).max(0.01).min(0.49);
        let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
        let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
        let omega = commutator(&u_0, &u_1, &mu).unwrap();
        total += 1;
        if classify_regime(&omega, RegimeBands::default()) != Regime::Smooth {
            flipped += 1;
        }
    }
    assert!(flipped == 0,
            "perturbation flipped {flipped}/{total} Smooth points");
}

// ─── (d) Finite-sample classifier robustness ────────────────────────────

#[test]
fn p4_4_clear_smooth_robust_to_finite_sample_noise() {
    // At N = 10k, well-separated Smooth points (TV ≈ 0.106 vs threshold
    // 0.30) should classify as Smooth across many seeds.
    let alpha = 0.2;
    let beta = 0.3;
    let mu = vec![0.5, 0.5];

    let mut flipped = 0u32;
    let n_seeds = 20u32;
    for seed_off in 0..n_seeds {
        let mut rng = Lcg::new(0x4242_4242 + u64::from(seed_off));
        let (emp_01, _) = empirical_belief_after_pair(&mu, alpha, beta, 0, 1, 10_000, &mut rng);
        let (emp_10, _) = empirical_belief_after_pair(&mu, alpha, beta, 1, 0, 10_000, &mut rng);
        let emp_tv = tv(&emp_01, &emp_10);
        // Synthesise a Commutator from empirical numbers — KL we don't
        // need to estimate here since the classifier reads it only via
        // Divergent detection; empirical KL on observed data is always
        // finite, so KlValue::Finite(_) suffices.
        let omega = gigi::causal_states::Commutator {
            forward: emp_01,
            backward: emp_10,
            tv: emp_tv,
            hellinger: 0.0, // unused by classifier
            kl: gigi::causal_states::KlValue::Finite(0.05),
        };
        let r = classify_regime(&omega, RegimeBands::default());
        if r != Regime::Smooth {
            flipped += 1;
        }
    }
    assert!(
        flipped == 0,
        "{flipped}/{n_seeds} empirical Smooth classifications flipped"
    );
}

// ─── (e) Confusion matrix headline ──────────────────────────────────────

#[test]
fn p4_4_confusion_matrix_clean_diagonal_on_strong_labels() {
    // Build a labeled corpus:
    //   - "Sofic" labels: Even Process @ μ; +20 synthetic TV ≥ 0.97 points
    //   - "Smooth" labels: HMM at every (α, β) with closed-form TV ≤ 0.1
    //   - No labeled "Borderline" — the entire test is on unambiguous cases.
    //
    // Run the classifier; report the confusion matrix.
    //
    // Off-diagonal accuracy must be 0 (no misclassifications).
    use gigi::causal_states::{Commutator, KlValue};

    let mut conf = [[0u32; 3]; 3];
    fn idx(r: Regime) -> usize {
        match r {
            Regime::Sofic => 0,
            Regime::Smooth => 1,
            Regime::Borderline => 2,
        }
    }

    let mu = vec![0.5, 0.5];

    // True label = Sofic.
    let omega_even = commutator(&EvenU0, &EvenU1, &mu).unwrap();
    conf[idx(Regime::Sofic)][idx(classify_regime(&omega_even, RegimeBands::default()))] += 1;
    for tv_step in 0..20 {
        let tv_val = 0.97 + 0.001 * f64::from(tv_step);
        let omega = Commutator {
            forward: vec![tv_val, 1.0 - tv_val],
            backward: vec![1.0 - tv_val, tv_val],
            tv: tv_val,
            hellinger: tv_val * 0.9,
            kl: KlValue::Finite(5.0),
        };
        conf[idx(Regime::Sofic)][idx(classify_regime(&omega, RegimeBands::default()))] += 1;
    }

    // True label = Smooth.
    for alpha_t in 5..=45u32 {
        for beta_t in 5..=45u32 {
            let alpha = f64::from(alpha_t) / 100.0;
            let beta = f64::from(beta_t) / 100.0;
            if hmm_closed_form_tv(alpha, beta) > 0.1 {
                continue;
            }
            let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
            let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
            let omega = commutator(&u_0, &u_1, &mu).unwrap();
            conf[idx(Regime::Smooth)][idx(classify_regime(&omega, RegimeBands::default()))] += 1;
        }
    }

    // Off-diagonal cells: assertion is no misclassifications on the
    // strong-labeled set.
    for true_idx in 0..3 {
        for pred_idx in 0..3 {
            if true_idx == pred_idx {
                continue;
            }
            assert_eq!(
                conf[true_idx][pred_idx], 0,
                "confusion[true={true_idx}, pred={pred_idx}] = {} (expected 0)",
                conf[true_idx][pred_idx]
            );
        }
    }
    // Diagonal totals: at least 1 Sofic + several hundred Smooth.
    assert!(conf[0][0] >= 21, "Sofic diagonal too small: {}", conf[0][0]);
    assert!(conf[1][1] >= 100, "Smooth diagonal too small: {}", conf[1][1]);
}
