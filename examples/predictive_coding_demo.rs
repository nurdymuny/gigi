//! Real-bundle demonstration of L11 predictive-coding primitives.
//!
//! Constructs a `BundleStore` holding 80 patient PK records (a small
//! MIRADOR-style cohort), then exercises:
//!
//! - **INPAINT** — given a partial patient record (`weight` known,
//!   `clearance` missing), sample the missing field from the
//!   conditional density learned by the bundle.
//! - **PREDICT** — given the current state, take one Fisher-natural
//!   step forward to predict where the patient profile drifts.
//! - **SELF-MONITOR** — confidence at a "known-patient" point vs.
//!   confidence at a wildly out-of-cohort query. The "I don't know"
//!   signal.
//!
//! This is a real-bundle test in the sense that it uses
//! `BundleStore`'s actual record-insert + Welford-streaming
//! variance machinery to derive the Hamiltonian `H = -log p`, then
//! pipes through the L10 `GenerativeFlow` infrastructure.
//!
//! Run:
//!     cargo run --release --features kahler --bin predictive_coding_demo

#![cfg(feature = "kahler")]

use gigi::bundle::BundleStore;
use gigi::geometry::{
    confidence_normalized, from_isotropic_gaussian, inpaint, kernel_density_confidence,
    predict_one_step, ClosedTwoForm, ComplexStructure, FlowConfig, KahlerStructure,
    TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn header(label: &str) {
    println!();
    println!("══ {} ══", label);
}

fn line(k: &str, v: impl std::fmt::Display) {
    println!("  {:<40} {}", k, v);
}

fn main() {
    println!("L11 predictive-coding — REAL BUNDLE DEMO");
    println!("========================================");

    // ── Build a MIRADOR-style PK cohort bundle ──────────────────
    //
    // Each record: { patient_id, weight (kg), clearance (L/h) }.
    // Population: weight ~ N(75, 10²), clearance correlates loosely
    // with weight via clearance ≈ 0.06 · weight + noise.

    let kahler = KahlerStructure::new(
        ComplexStructure::standard(1),
        ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap(),
        ),
    );
    let schema = BundleSchema::new("pk_cohort")
        .base(FieldDef::numeric("patient_id"))
        .fiber(FieldDef::numeric("weight").with_range(60.0))
        .fiber(FieldDef::numeric("clearance").with_range(8.0))
        .with_kahler(kahler);

    let mut store = BundleStore::new(schema);
    let n_patients = 80;
    let mut weights = Vec::with_capacity(n_patients);
    let mut clearances = Vec::with_capacity(n_patients);
    // Deterministic PRNG state for reproducibility.
    let mut s: u64 = 0xDEAD_BEEF_CAFE_F00D;
    fn next_normal(s: &mut u64) -> f64 {
        // Lehmer + Box-Muller.
        let xorshift = |s: &mut u64| {
            *s ^= *s << 13;
            *s ^= *s >> 7;
            *s ^= *s << 17;
            *s
        };
        let u1 = (xorshift(s) as f64 / u64::MAX as f64).max(1e-300);
        let u2 = xorshift(s) as f64 / u64::MAX as f64;
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    }
    for i in 0..n_patients {
        let w = 75.0 + 10.0 * next_normal(&mut s);
        let cl = 0.06 * w + 0.5 * next_normal(&mut s);
        weights.push(w);
        clearances.push(cl);
        let mut rec = Record::new();
        rec.insert("patient_id".into(), Value::Integer(i as i64));
        rec.insert("weight".into(), Value::Float(w));
        rec.insert("clearance".into(), Value::Float(cl));
        store.insert(&rec);
    }

    // Streaming stats already computed inside BundleStore. Pull them
    // out for the generative-flow Hamiltonian.
    let stats = store.field_stats();
    let w_stat = stats.get("weight").expect("weight stats");
    let cl_stat = stats.get("clearance").expect("clearance stats");
    // FieldStats stores sum + count; mean = sum / count.
    let w_mean = w_stat.sum / w_stat.count as f64;
    let cl_mean = cl_stat.sum / cl_stat.count as f64;
    let mu = vec![w_mean, cl_mean];
    // Isotropic fit using mean of the two field variances (closest
    // matching "diagonal" assumption for the L10 isotropic helper).
    let sigma_sq = 0.5 * (w_stat.variance() + cl_stat.variance());

    header("Bundle state — what GIGI sees");
    line("records inserted", n_patients);
    line(
        "empirical (μ_weight, μ_clearance)",
        format!("({:.3}, {:.3})", mu[0], mu[1]),
    );
    line(
        "isotropic σ² (mean of field variances)",
        format!("{:.3}", sigma_sq),
    );

    // ── Build the generative flow on top ────────────────────────

    let b = ClosedTwoForm::new_constant(
        TwoForm::new(vec![0.0, 1.0, -1.0, 0.0], 2).unwrap(),
    );
    let flow = from_isotropic_gaussian(b, mu.clone(), sigma_sq).unwrap();

    // ── §6 INPAINT ────────────────────────────────────────────
    //
    // We have a new patient who weighs 90 kg. We don't know their
    // clearance. INPAINT samples a plausible clearance from the
    // bundle's conditional density.

    header("INPAINT — fill in missing clearance for weight = 90 kg");
    let config_in = FlowConfig {
        dt: 0.02,
        temperature: 1.0,
        n_steps: 1,
        burn_in: 5_000,
        seed: Some(12345),
    };
    let partial = vec![90.0, 0.0]; // clearance is a placeholder, gets resampled
    let filled = inpaint(&flow, &partial, &[0], &config_in).unwrap();
    line("locked weight (kg)", filled[0]);
    line(
        "inferred clearance (L/h) [empirical]",
        format!("{:.3}", filled[1]),
    );
    // (Closed-form linear regression would predict
    // 0.06 * 90 ≈ 5.4 L/h; the flow samples from p, not from a
    // regressed mean, so the inferred clearance is one MC draw.)

    // ── §7 PREDICT — one-step natural gradient ─────────────────
    //
    // Project where a 90 kg patient's profile drifts under one
    // step of the bundle's gradient flow.

    header("PREDICT — single Fisher-natural step from (90, 6.5)");
    let state = vec![90.0, 6.5];
    let next = predict_one_step(&flow, &state, 0.1).unwrap();
    line(
        "from (weight, clearance)",
        format!("({:.2}, {:.2})", state[0], state[1]),
    );
    line(
        "to   (weight, clearance) after one step",
        format!("({:.2}, {:.2})", next[0], next[1]),
    );
    line(
        "shifts toward population mean (μ_w, μ_cl)",
        format!("({:.2}, {:.2})", mu[0], mu[1]),
    );

    // ── §12 SELF-MONITOR ───────────────────────────────────────
    //
    // Two queries:
    //   - "Patient A" — weight 78, clearance 4.7. Well inside cohort.
    //   - "Patient B" — weight 300, clearance 0.1. Wildly out of dist.
    // Expect: A has high confidence, B has near-zero confidence.

    header("SELF-MONITOR — confidence at known vs unknown points");

    // Collect the raw samples as (weight, clearance) pairs.
    let samples: Vec<Vec<f64>> = (0..n_patients)
        .map(|i| vec![weights[i], clearances[i]])
        .collect();

    let bandwidth = sigma_sq.sqrt(); // one-σ kernel

    let q_known = vec![78.0, 4.7];
    let q_unknown = vec![300.0, 0.1];

    let c_known = kernel_density_confidence(&samples, &q_known, bandwidth);
    let c_unknown = kernel_density_confidence(&samples, &q_unknown, bandwidth);
    let cn_known = confidence_normalized(&samples, &q_known, bandwidth);
    let cn_unknown = confidence_normalized(&samples, &q_unknown, bandwidth);

    line(
        "(78, 4.7)   raw confidence (sum kernel)",
        format!("{:.4}", c_known),
    );
    line(
        "(78, 4.7)   normalized confidence ∈ [0,1]",
        format!("{:.4}", cn_known),
    );
    line(
        "(300, 0.1)  raw confidence (sum kernel)",
        format!("{:.3e}", c_unknown),
    );
    line(
        "(300, 0.1)  normalized confidence ∈ [0,1]",
        format!("{:.3e}", cn_unknown),
    );

    if c_known > 0.0 {
        line(
            "ratio known / unknown",
            format!("{:.2e}× (the 'I don't know' signal)", c_known / c_unknown.max(1e-300)),
        );
    }

    // ── Marcella-style gate decision ──────────────────────────

    header("Marcella gate decision (threshold: 1% of densest)");
    let gate_threshold = 0.01;
    line(
        "known patient: passes gate?",
        cn_known > gate_threshold,
    );
    line(
        "unknown patient: passes gate?",
        cn_unknown > gate_threshold,
    );
    println!();
    println!("Done — L11 INPAINT / PREDICT / SELF-MONITOR on a real bundle.");
    println!("       (catalog: theory/brain_primitives/catalog.md §6, §7, §12)");
}
