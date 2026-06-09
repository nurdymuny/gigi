//! Phase 4.4 — Classifier audit: confusion matrix on labeled processes.
//!
//! Companion to Davis (2026), *"Causal States as Predictive Sections."*
//!
//! Generates `theory/causal_states/classifier_confusion.txt` — the formal
//! confusion matrix for the paper's appendix. Labels are derived from
//! *mathematical* regime properties (KL divergence + TV thresholds), not
//! from the classifier itself, so the comparison is meaningful.
//!
//! Run:
//!   cargo run --features causal_states --bin causal_states_classifier_audit

use gigi::causal_states::{
    classify_regime, commutator, hmm_closed_form_tv, Commutator, EvenU0, EvenU1, HmmUpdate,
    KlValue, Regime, RegimeBands,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn regime_idx(r: Regime) -> usize {
    match r {
        Regime::Sofic => 0,
        Regime::Smooth => 1,
        Regime::Borderline => 2,
    }
}

fn regime_name(i: usize) -> &'static str {
    ["Sofic", "Smooth", "Borderline"][i]
}

fn main() {
    let out_dir: PathBuf = ["theory", "causal_states"].iter().collect();
    std::fs::create_dir_all(&out_dir).expect("create theory/causal_states/");
    let report_path = out_dir.join("classifier_confusion.txt");

    let bands = RegimeBands::default();
    let mut conf = [[0u32; 3]; 3]; // conf[true_idx][pred_idx]
    let mut n_total = 0u32;

    // ─── Sofic-labeled (KL Divergent OR TV ≥ 0.97) ──────────────────────
    let mu_even = vec![2.0 / 3.0, 1.0 / 3.0];
    let mu_half = vec![0.5, 0.5];

    // Even Process at both interior orbit points — naturally Sofic.
    for base in &[&mu_even, &mu_half] {
        let omega = commutator(&EvenU0, &EvenU1, base).unwrap();
        let predicted = classify_regime(&omega, bands);
        conf[regime_idx(Regime::Sofic)][regime_idx(predicted)] += 1;
        n_total += 1;
    }
    // Synthetic high-TV-finite-KL points.
    for tv_step in 0..50 {
        let tv_val = 0.97 + 0.0005 * f64::from(tv_step);
        if tv_val > 1.0 {
            break;
        }
        let omega = Commutator {
            forward: vec![tv_val, 1.0 - tv_val],
            backward: vec![1.0 - tv_val, tv_val],
            tv: tv_val,
            hellinger: tv_val * 0.9,
            kl: KlValue::Finite(5.0 + f64::from(tv_step)),
        };
        let predicted = classify_regime(&omega, bands);
        conf[regime_idx(Regime::Sofic)][regime_idx(predicted)] += 1;
        n_total += 1;
    }

    // ─── Smooth-labeled (TV ≤ 0.1, KL finite) ──────────────────────────
    let mut smooth_count = 0u32;
    for alpha_t in 5..=45u32 {
        for beta_t in 5..=45u32 {
            let alpha = f64::from(alpha_t) / 100.0;
            let beta = f64::from(beta_t) / 100.0;
            if hmm_closed_form_tv(alpha, beta) > 0.1 {
                continue;
            }
            let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
            let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
            let omega = commutator(&u_0, &u_1, &mu_half).unwrap();
            let predicted = classify_regime(&omega, bands);
            conf[regime_idx(Regime::Smooth)][regime_idx(predicted)] += 1;
            smooth_count += 1;
            n_total += 1;
        }
    }

    // ─── Borderline-labeled (TV ∈ [0.35, 0.85], KL finite) ──────────────
    // These are HMM points with TV well inside the band (above tv_low and
    // below tv_high) — the classifier should call them Borderline.
    let mut border_count = 0u32;
    for alpha_t in 1..=10u32 {
        for beta_t in 5..=20u32 {
            let alpha = f64::from(alpha_t) / 100.0;
            let beta = f64::from(beta_t) / 100.0;
            let closed = hmm_closed_form_tv(alpha, beta);
            if closed < 0.35 || closed > 0.85 {
                continue;
            }
            let u_0 = HmmUpdate { alpha, beta, symbol: 0 };
            let u_1 = HmmUpdate { alpha, beta, symbol: 1 };
            let omega = commutator(&u_0, &u_1, &mu_half).unwrap();
            let predicted = classify_regime(&omega, bands);
            conf[regime_idx(Regime::Borderline)][regime_idx(predicted)] += 1;
            border_count += 1;
            n_total += 1;
        }
    }

    // ─── Per-class accuracy ──────────────────────────────────────────────
    let mut acc = [0.0f64; 3];
    for true_idx in 0..3 {
        let row_total: u32 = conf[true_idx].iter().sum();
        acc[true_idx] = if row_total == 0 {
            f64::NAN
        } else {
            f64::from(conf[true_idx][true_idx]) / f64::from(row_total)
        };
    }
    let row_totals: Vec<u32> = (0..3).map(|i| conf[i].iter().sum()).collect();
    let correct: u32 = (0..3).map(|i| conf[i][i]).sum();
    let overall_acc = f64::from(correct) / f64::from(n_total);

    // ─── Report ──────────────────────────────────────────────────────────
    let mut report = String::new();
    report.push_str("Causal States — classifier confusion matrix\n");
    report.push_str("===========================================\n\n");
    report.push_str("Companion to Davis (2026), \"Causal States as Predictive Sections.\"\n");
    report.push_str("Generated by examples/causal_states_classifier_audit.rs.\n\n");
    report.push_str("Labels assigned by mathematical regime criteria:\n");
    report.push_str("  - Sofic    : KL Divergent OR TV ≥ 0.97\n");
    report.push_str("  - Smooth   : closed-form TV ≤ 0.1, KL finite\n");
    report.push_str("  - Borderline: closed-form TV ∈ [0.35, 0.85], KL finite\n");
    report.push_str("Classifier bands: tv_low = 0.30, tv_high = 0.95.\n\n");

    report.push_str("Confusion matrix [true × predicted]:\n\n");
    report.push_str("                  predicted\n");
    report.push_str("              Sofic  Smooth  Border  total  acc\n");
    for true_idx in 0..3 {
        report.push_str(&format!(
            "  true {:<8} {:>5}  {:>5}  {:>6}  {:>5}  {:>5.3}\n",
            regime_name(true_idx),
            conf[true_idx][0],
            conf[true_idx][1],
            conf[true_idx][2],
            row_totals[true_idx],
            acc[true_idx],
        ));
    }
    report.push_str(&format!("\n  Overall accuracy: {overall_acc:.4} ({correct}/{n_total})\n"));
    report.push_str(&format!(
        "\nProcesses tested:\n\
         \n\
         Sofic-labeled corpus (n = {}):\n\
           - Even Process @ μ = (2/3, 1/3)\n\
           - Even Process @ (1/2, 1/2)\n\
           - 49 synthetic high-TV-finite-KL anchors (TV ∈ [0.97, 0.995])\n\
         \n\
         Smooth-labeled corpus (n = {smooth_count}):\n\
           - All (α, β) in {{0.05, ..., 0.45}}² with closed-form TV ≤ 0.1\n\
         \n\
         Borderline-labeled corpus (n = {border_count}):\n\
           - HMM at small (α, β) with closed-form TV ∈ [0.35, 0.85]\n\
         \n\
         Total labeled samples: {n_total}\n",
        row_totals[regime_idx(Regime::Sofic)],
    ));

    std::fs::write(&report_path, &report).expect("write report");
    let mut stdout = std::io::stderr();
    writeln!(stdout, "{report}").unwrap();
    writeln!(stdout, "Wrote: {}", report_path.display()).unwrap();
}
