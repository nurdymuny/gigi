//! brain_tour_demo — every brain primitive on one bundle, in one run.
//!
//! Unified replacement for the fragmented coverage across
//! `dream_demo`, `dream_anisotropic_demo`, `predictive_coding_demo`,
//! and `attention_memory_demo`. Builds a single synthetic bundle
//! (40 records, 4D anisotropic fiber) and walks all 12 brain
//! primitives end-to-end against it.
//!
//! Per Bee's 2026-05-26 cleanup audit: one demo, one bundle, 12
//! primitives, real numbers, no special-casing.
//!
//! Run:
//!     cargo run --release --features kahler --bin brain_tour_demo
//!
//! Catalog: `theory/brain_primitives/catalog.md`.

#![cfg(feature = "kahler")]

use gigi::bundle::BundleStore;
use gigi::geometry::{
    attend, confidence_normalized, episodic_events_with_floor, explain, focus,
    from_diagonal_gaussian, from_isotropic_gaussian, inpaint, kernel_density_confidence,
    predict_one_step, semantic_gist, ClosedTwoForm, ComplexStructure, FlowConfig,
    KahlerStructure, TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn header(n: &str, title: &str) {
    println!();
    println!("══ §{} {} ══", n, title);
}

fn line(k: &str, v: impl std::fmt::Display) {
    println!("  {:<40} {}", k, v);
}

fn note(s: &str) {
    println!("  {}", s);
}

fn main() {
    println!("┌───────────────────────────────────────────────────────────────────────┐");
    println!("│  GIGI brain primitives — unified demo                                 │");
    println!("│  All 12 primitives on a single synthetic bundle (40 records, 4D)     │");
    println!("│  Catalog: theory/brain_primitives/catalog.md                          │");
    println!("└───────────────────────────────────────────────────────────────────────┘");

    // ── Bundle setup ─────────────────────────────────────────────
    //
    // Synthetic 4D anisotropic bundle, deliberately structured to
    // exercise every primitive:
    //   - fields ax, ay: tight cluster around origin
    //   - field bx: wider spread (exercises diagonal vs iso)
    //   - field cy: clustered with a clear inter-cluster gap
    //                (exercises EPISODIC)

    let kahler = KahlerStructure::new(
        ComplexStructure::standard(2),
        ClosedTwoForm::new_constant(
            TwoForm::new(
                vec![
                    0.0, 0.0, -1.0, 0.0,
                    0.0, 0.0, 0.0, -1.0,
                    1.0, 0.0, 0.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                ],
                4,
            )
            .unwrap(),
        ),
    );
    let schema = BundleSchema::new("brain_tour")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("ax").with_range(2.0))
        .fiber(FieldDef::numeric("ay").with_range(2.0))
        .fiber(FieldDef::numeric("bx").with_range(8.0))
        .fiber(FieldDef::numeric("cy").with_range(20.0))
        .with_kahler(kahler);
    let mut store = BundleStore::new(schema);

    let mut s: u64 = 0xCAFE_BABE_DEAD_BEEF;
    let mut rand_norm = || -> f64 {
        s ^= s << 13; s ^= s >> 7; s ^= s << 17;
        let u1 = (s as f64 / u64::MAX as f64).max(1e-12);
        s ^= s << 13; s ^= s >> 7; s ^= s << 17;
        let u2 = s as f64 / u64::MAX as f64;
        (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
    };

    // 20 records in cluster A, 20 in cluster B (clear EPISODIC gap on cy).
    for i in 0..20 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("ax".into(), Value::Float(0.3 * rand_norm()));
        r.insert("ay".into(), Value::Float(0.3 * rand_norm()));
        r.insert("bx".into(), Value::Float(2.0 * rand_norm()));
        r.insert("cy".into(), Value::Float(0.5 * rand_norm()));
        store.insert(&r);
    }
    for i in 20..40 {
        let mut r = Record::new();
        r.insert("id".into(), Value::Integer(i));
        r.insert("ax".into(), Value::Float(0.3 * rand_norm()));
        r.insert("ay".into(), Value::Float(0.3 * rand_norm()));
        r.insert("bx".into(), Value::Float(2.0 * rand_norm()));
        r.insert("cy".into(), Value::Float(15.0 + 0.5 * rand_norm())); // big jump on cy
        store.insert(&r);
    }

    // Pull samples / stats once.
    let stats = store.field_stats();
    let mu = vec![
        stats["ax"].mean,
        stats["ay"].mean,
        stats["bx"].mean,
        stats["cy"].mean,
    ];
    let sigma_sq_per_field = vec![
        stats["ax"].variance().max(1e-3),
        stats["ay"].variance().max(1e-3),
        stats["bx"].variance().max(1e-3),
        stats["cy"].variance().max(1e-3),
    ];
    let sigma_sq_iso = sigma_sq_per_field.iter().sum::<f64>() / 4.0;

    let mut samples: Vec<Vec<f64>> = Vec::new();
    for (_bp, rec) in store.sections() {
        let row: Vec<f64> = rec
            .iter()
            .map(|v| match v {
                Value::Float(f) => *f,
                Value::Integer(i) => *i as f64,
                _ => 0.0,
            })
            .collect();
        samples.push(row);
    }

    println!();
    line("records inserted", samples.len());
    line("μ (ax, ay, bx, cy)",
         format!("({:.3}, {:.3}, {:.3}, {:.3})", mu[0], mu[1], mu[2], mu[3]));
    line("σ² per field (anisotropic)",
         format!("[{:.3}, {:.3}, {:.3}, {:.3}]",
                 sigma_sq_per_field[0], sigma_sq_per_field[1],
                 sigma_sq_per_field[2], sigma_sq_per_field[3]));

    // Build the canonical symplectic B once (4D block form).
    let b = || -> ClosedTwoForm {
        ClosedTwoForm::new_constant(
            TwoForm::new(
                vec![
                    0.0, 0.0, -1.0, 0.0,
                    0.0, 0.0, 0.0, -1.0,
                    1.0, 0.0, 0.0, 0.0,
                    0.0, 1.0, 0.0, 0.0,
                ],
                4,
            )
            .unwrap(),
        )
    };

    // ── §2 SAMPLE — Langevin draws from p ───────────────────────
    header("2", "SAMPLE — canonical Langevin draws");
    let iso_flow = from_isotropic_gaussian(b(), mu.clone(), sigma_sq_iso).unwrap();
    let cfg_sample = FlowConfig {
        dt: 0.01, temperature: 1.0, n_steps: 1, burn_in: 1500, seed: Some(42),
    };
    let draws = iso_flow.sample_many(&[0.0; 4], &cfg_sample, 8, 1).unwrap();
    for (i, d) in draws.iter().take(3).enumerate() {
        line(&format!("  draw[{}]", i),
             format!("({:.3}, {:.3}, {:.3}, {:.3})", d[0], d[1], d[2], d[3]));
    }
    line("(showing 3 of 8 draws; isotropic fit)", "");

    // ── §3 FORECAST — Hamilton-flow extension ───────────────────
    header("3", "FORECAST — deterministic Hamilton flow from a seed");
    let cfg_forecast = FlowConfig::forecasting();
    let path = iso_flow.forecast(&[3.0, 0.0, 5.0, 10.0], &cfg_forecast).unwrap();
    line("start", format!("({:.2}, {:.2}, {:.2}, {:.2})",
                          path[0][0], path[0][1], path[0][2], path[0][3]));
    line("after 500 steps",
         format!("({:.2}, {:.2}, {:.2}, {:.2})",
                 path[500][0], path[500][1], path[500][2], path[500][3]));
    line("after 1000 steps",
         format!("({:.2}, {:.2}, {:.2}, {:.2})",
                 path[1000][0], path[1000][1], path[1000][2], path[1000][3]));
    note("(orbital — Hamilton conserves H along the trajectory)");

    // ── §4 DREAM — high-T trajectory ────────────────────────────
    header("4", "DREAM — high-temperature Langevin (T = 4)");
    let cfg_dream = FlowConfig { temperature: 4.0, ..FlowConfig::forecasting() };
    let dream_path = iso_flow.dream(&mu, &cfg_dream).unwrap();
    let max_dist = dream_path
        .iter()
        .map(|p| {
            ((p[0] - mu[0]).powi(2) + (p[1] - mu[1]).powi(2)
             + (p[2] - mu[2]).powi(2) + (p[3] - mu[3]).powi(2)).sqrt()
        })
        .fold(0.0_f64, f64::max);
    line("max wander distance from μ", format!("{:.2}", max_dist));
    note("(T=4 → ~2× spread vs SAMPLE; dream_anisotropic_demo for iso vs diag)");

    // ── §5 RECONSTRUCT — descent to MAP ─────────────────────────
    header("5", "RECONSTRUCT — T=0 descent to MAP");
    let cfg_recon = FlowConfig::reconstructing();
    let map = iso_flow.reconstruct(&[20.0; 4], &cfg_recon).unwrap();
    line("from (20, 20, 20, 20)", format!("({:.3}, {:.3}, {:.3}, {:.3})",
                                           map[0], map[1], map[2], map[3]));
    line("vs μ",
         format!("({:.3}, {:.3}, {:.3}, {:.3})", mu[0], mu[1], mu[2], mu[3]));
    note("(unimodal isotropic → MAP equals μ)");

    // ── §6 INPAINT — constrained Langevin ───────────────────────
    header("6", "INPAINT — lock ax=8, sample (ay, bx, cy)");
    let cfg_in = FlowConfig {
        dt: 0.05, temperature: 1.0, n_steps: 1, burn_in: 3000, seed: Some(123),
    };
    let filled = inpaint(&iso_flow, &[8.0, 0.0, 0.0, 0.0], &[0], &cfg_in).unwrap();
    line("ax (locked at 8.0)", format!("{:.4}", filled[0]));
    line("ay (sampled)",       format!("{:.4}", filled[1]));
    line("bx (sampled)",       format!("{:.4}", filled[2]));
    line("cy (sampled)",       format!("{:.4}", filled[3]));

    // ── §7 PREDICT — one-step natural gradient ──────────────────
    header("7", "PREDICT — single Fisher-natural step from (5,5,5,5)");
    let next = predict_one_step(&iso_flow, &[5.0, 5.0, 5.0, 5.0], 0.2).unwrap();
    line("from (5, 5, 5, 5)",
         format!("({:.4}, {:.4}, {:.4}, {:.4})", next[0], next[1], next[2], next[3]));
    line("shifts toward μ",
         format!("({:.3}, {:.3}, {:.3}, {:.3})", mu[0], mu[1], mu[2], mu[3]));

    // ── §8 ATTEND — softmax retrieval ───────────────────────────
    header("8", "ATTEND — query near cluster A (close to record 5)");
    let q_attend = vec![samples[5][0], samples[5][1], samples[5][2], samples[5][3]];
    let weights = attend(&samples, &q_attend, 0.5);
    let mut paired: Vec<_> = weights.iter().enumerate().collect();
    paired.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    line("top-1 attended index", paired[0].0);
    line("top-1 attention weight", format!("{:.4}", paired[0].1));
    line("top-3 indices",
         format!("[{}, {}, {}]", paired[0].0, paired[1].0, paired[2].0));
    line("Σ weights (should be 1.0)",
         format!("{:.6}", weights.iter().sum::<f64>()));

    // ── §9 FOCUS — top-k attended ───────────────────────────────
    header("9", "FOCUS — top-3 closest to query");
    let top3 = focus(&samples, &q_attend, 0.5, 3);
    for (idx, w) in &top3 {
        line(&format!("  idx {}", idx), format!("weight {:.4}", w));
    }

    // ── §10 EPISODIC — change-point detection ───────────────────
    header("10", "EPISODIC — change-point on cy (0→15 cluster jump)");
    let cy_values: Vec<f64> = samples.iter().map(|s| s[3]).collect();
    let events = episodic_events_with_floor(&cy_values, 50.0, 1e-6);
    line("events found",
         events.len());
    if let Some(top) = events.first() {
        line("  top gap", format!("{:.2}", top.gap));
        line("  persistence_ratio", format!("{:.1}×", top.persistence_ratio));
    }

    // ── §11 SEMANTIC — Morse-compressed gist ────────────────────
    header("11", "SEMANTIC — Morse-compressed bundle topology");
    match semantic_gist(&store) {
        Some(m) => {
            line("Betti (b₀, b₁, b₂)",
                 format!("({}, {}, {})", m.betti.b0, m.betti.b1, m.betti.b2));
            line("critical / original",
                 format!("{} / {}", m.n_critical(), m.n_original()));
            line("compression ratio", format!("{:.2}×", m.compression_ratio()));
        }
        None => line("Morse compression", "(degenerate or too small)"),
    }

    // ── §12 SELF-MONITOR — Fisher-precision confidence ──────────
    header("12", "SELF-MONITOR — confidence gate (known vs unknown)");
    let q_known = vec![samples[0][0], samples[0][1], samples[0][2], samples[0][3]];
    let q_unknown = vec![100.0, 100.0, 100.0, 100.0];
    let c_known   = kernel_density_confidence(&samples, &q_known,   1.0);
    let c_unknown = kernel_density_confidence(&samples, &q_unknown, 1.0);
    let n_known   = confidence_normalized(&samples, &q_known,   1.0);
    let n_unknown = confidence_normalized(&samples, &q_unknown, 1.0);
    line("at known record (raw / norm)",
         format!("{:.4} / {:.4}", c_known, n_known));
    line("at far outlier (raw / norm)",
         format!("{:.3e} / {:.3e}", c_unknown, n_unknown));
    line("ratio known / unknown",
         format!("{:.2e}× (the 'I don't know' signal)",
                 c_known / c_unknown.max(1e-300)));

    // ── §13 EXPLAIN — geodesic to nearest known ─────────────────
    header("13", "EXPLAIN — interpolation path to nearest record");
    let novel_query = vec![15.0, 15.0, 15.0, 5.0]; // not near any cluster
    let exp = explain(&samples, &novel_query, 4);
    line("query",
         format!("({:.2}, {:.2}, {:.2}, {:.2})",
                 novel_query[0], novel_query[1], novel_query[2], novel_query[3]));
    if let Some(nearest) = &exp.nearest_record {
        line("nearest record",
             format!("({:.2}, {:.2}, {:.2}, {:.2})  idx {}",
                     nearest[0], nearest[1], nearest[2], nearest[3],
                     exp.nearest_index.unwrap()));
        line("nearest_distance", format!("{:.3}", exp.nearest_distance));
        line(&format!("path (n_steps = {})", exp.n_steps),
             format!("{} interpolation points", exp.path.len()));
        line("  start (= query)",
             format!("({:.2}, {:.2}, {:.2}, {:.2})",
                     exp.path[0][0], exp.path[0][1], exp.path[0][2], exp.path[0][3]));
        let mid = &exp.path[exp.path.len() / 2];
        line("  middle",
             format!("({:.2}, {:.2}, {:.2}, {:.2})", mid[0], mid[1], mid[2], mid[3]));
        let last = exp.path.last().unwrap();
        line("  end (= nearest)",
             format!("({:.2}, {:.2}, {:.2}, {:.2})", last[0], last[1], last[2], last[3]));
    } else {
        line("(empty bundle)", "no nearest record");
    }

    // ── L13.3 / L13.6 diagonal-fit summary ──────────────────────
    header("L13.3+6", "Anisotropic diagonal fit (Marcella probe Finding 3)");
    let diag_flow = from_diagonal_gaussian(b(), mu.clone(), sigma_sq_per_field.clone()).unwrap();
    let diag_draws = diag_flow.sample_many(&[0.0; 4], &cfg_sample, 50, 1).unwrap();
    // Compute per-axis variance of the draws.
    let mut means = [0.0_f64; 4];
    for s in &diag_draws { for i in 0..4 { means[i] += s[i]; } }
    for m in &mut means { *m /= diag_draws.len() as f64; }
    let mut vars = [0.0_f64; 4];
    for s in &diag_draws {
        for i in 0..4 { vars[i] += (s[i] - means[i]).powi(2); }
    }
    for v in &mut vars { *v /= diag_draws.len() as f64; }
    line("target σ² per field",
         format!("[{:.3}, {:.3}, {:.3}, {:.3}]",
                 sigma_sq_per_field[0], sigma_sq_per_field[1],
                 sigma_sq_per_field[2], sigma_sq_per_field[3]));
    line("empirical σ² (50 draws)",
         format!("[{:.3}, {:.3}, {:.3}, {:.3}]",
                 vars[0], vars[1], vars[2], vars[3]));
    note("(per-axis anisotropy preserved instead of averaged away)");

    println!();
    println!("┌───────────────────────────────────────────────────────────────────────┐");
    println!("│  Done — all 12 brain primitives exercised on one bundle.             │");
    println!("│  Catalog (math + proof sketches): theory/brain_primitives/catalog.md │");
    println!("│  HTTP wire shapes: tests/kahler_brain_endpoints_contract.rs          │");
    println!("│  Consumer guide: BRAIN_PRIMITIVES_CONSUMER_GUIDE.md (top-level)      │");
    println!("└───────────────────────────────────────────────────────────────────────┘");
}
