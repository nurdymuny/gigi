//! JTBD demo for Branch VII — Cognitive Geometry verbs (CAPACITY,
//! HORIZON, DEPTH).
//!
//! ## The job a builder is hiring GIGI to do
//!
//! "Given a substrate I just loaded, tell me — *before* I commit a
//! routing decision — whether the geometry has room for multiple
//! interpretations (proceed with one), is at a fork (ask for
//! clarification), or is overloaded (refuse). And tell me how many
//! positions can be coherently attributed before non-abelian
//! composition mixes them into an inseparable product."
//!
//! That decision today is made by intuition or by a fine-tuned
//! classifier. With CAPACITY / HORIZON / DEPTH it becomes a direct
//! geometric query — same audit trail every time, no labeled
//! training, no domain-specific gradient descent.
//!
//! ## What this demo does
//!
//! Builds two `BundleStore`s with the SAME schema but different
//! data distributions, then exercises the three verbs on each:
//!
//!   - **Bundle A — peaceful**: real sensor data (20 records,
//!     small variance, low effective K).
//!   - **Bundle B — volatile**: synthetic high-variance traces on
//!     the same schema (20 records, large variance, high K).
//!
//! For each bundle, prints:
//!   - K, λ₁, C(τ) at five τ values, s_max(τ) at the same τ values,
//!     DEPTH classification.
//!   - The concrete routing decision the verbs recommend.
//!
//! Run:
//!     cargo run --release --features kahler --example cognitive_geometry_demo

#![cfg(feature = "kahler")]

use gigi::curvature::{
    capacity, encoding_depth, encoding_depth_with, horizon_with, scalar_curvature, DepthConfig,
    EncodingDepth, HorizonConfig,
};
use gigi::spectral::spectral_gap;
use gigi::types::{BundleSchema, FieldDef, Value};
use gigi::BundleStore;
use std::collections::HashMap;
use std::fs;

// ── presentation helpers ───────────────────────────────────────────

fn header(title: &str) {
    println!();
    println!("══ {} ══", title);
}

fn line(k: &str, v: impl std::fmt::Display) {
    println!("  {:<32} {}", k, v);
}

// ── shared schema ──────────────────────────────────────────────────

fn sensor_schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
        .base(FieldDef::categorical("sensor_id"))
        .base(FieldDef::timestamp("timestamp", 1.0))
        .fiber(FieldDef::numeric("temperature"))
        .fiber(FieldDef::numeric("humidity"))
        .fiber(FieldDef::numeric("pressure"))
        .fiber(FieldDef::categorical("unit"))
        .fiber(FieldDef::categorical("status"))
}

// ── bundle A: peaceful real sensor data ───────────────────────────

fn build_peaceful_bundle() -> BundleStore {
    let path = std::env::var("CARGO_MANIFEST_DIR")
        .map(|d| format!("{}/test_data/sensor_data.json", d))
        .expect("CARGO_MANIFEST_DIR not set");
    let text = fs::read_to_string(&path).expect("read sensor_data.json");
    let parsed: serde_json::Value = serde_json::from_str(&text).expect("parse JSON");
    let mut store = BundleStore::new(sensor_schema("peaceful_sensors"));
    for item in parsed.as_array().expect("array") {
        let obj = item.as_object().expect("object");
        let mut rec: HashMap<String, Value> = HashMap::new();
        for (k, v) in obj {
            let val = match v {
                serde_json::Value::String(s) => Value::Text(s.clone()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Value::Integer(i)
                    } else {
                        Value::Float(n.as_f64().expect("f64"))
                    }
                }
                serde_json::Value::Bool(b) => Value::Bool(*b),
                _ => panic!("unexpected"),
            };
            rec.insert(k.clone(), val);
        }
        store.insert(&rec);
    }
    store
}

// ── bundle B: volatile synthetic high-variance trace ──────────────

fn build_volatile_bundle() -> BundleStore {
    let mut store = BundleStore::new(sensor_schema("volatile_sensors"));
    // Same 20 timestamps as the real fixture, but synthetic readings
    // with order-of-magnitude swings. Deterministic Lehmer-shift PRNG.
    let mut s: u64 = 0xCAFE_F00D_DEAD_BEEF;
    let mut next = || -> f64 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        (s as f64 / u64::MAX as f64) - 0.5
    };
    for i in 0..20_i64 {
        let mut rec: HashMap<String, Value> = HashMap::new();
        rec.insert("sensor_id".into(), Value::Text("V-001".into()));
        rec.insert("timestamp".into(), Value::Integer(1_710_000_000 + i * 60));
        // High-amplitude swings: t centered at 22 °C ± 40 °C, h ± 60 %,
        // p ± 80 hPa. This is the "volatile" half of the comparison.
        rec.insert("temperature".into(), Value::Float(22.0 + 80.0 * next()));
        rec.insert("humidity".into(), Value::Float(48.0 + 120.0 * next()));
        rec.insert("pressure".into(), Value::Float(1013.0 + 160.0 * next()));
        rec.insert("unit".into(), Value::Text("metric".into()));
        rec.insert(
            "status".into(),
            Value::Text(if next() > 0.0 { "alert".into() } else { "normal".into() }),
        );
        store.insert(&rec);
    }
    store
}

// ── per-bundle report ─────────────────────────────────────────────

fn report_bundle(label: &str, store: &BundleStore) -> (f64, f64, EncodingDepth) {
    header(label);
    let k = scalar_curvature(store);
    let lambda1 = spectral_gap(store);
    line("records", store.records().count());
    line("scalar curvature K", format!("{:.6}", k));
    line("spectral gap λ₁", format!("{:.6}", lambda1));
    line(
        "correlation length ℓ_c = 1/√λ₁",
        if lambda1 > 0.0 {
            format!("{:.4}", 1.0 / lambda1.sqrt())
        } else {
            "(undefined, λ₁≈0)".into()
        },
    );

    println!();
    println!("  ┌── CAPACITY C(τ) = τ/K — interpretations the geometry can hold");
    let taus = [0.1, 0.5, 1.0, 2.0, 5.0];
    for &tau in &taus {
        let c = capacity(tau, k);
        let regime = if !c.is_finite() {
            "flat (K≈0)"
        } else if c > 10.0 {
            "low load — decisive"
        } else if c > 2.0 {
            "moderate"
        } else if c > 1.0 {
            "high load — borderline"
        } else if c > 0.0 {
            "overloaded — ask"
        } else {
            "critical — refuse"
        };
        line(
            &format!("  τ = {:>4.1}", tau),
            format!("C = {:>10.4}    [{}]", c, regime),
        );
    }

    println!();
    println!("  ┌── HORIZON s_max(τ) = τ / (K · ℓ_c) — coherent context depth");
    // Use the calibrated path with default HorizonConfig (SpectralGap
    // primary + WelfordRadius fallback). On sensor bundles the
    // fallback fires because λ₁ ≈ 0.
    let hcfg = HorizonConfig::default();
    let first = horizon_with(taus[0], k, store, lambda1, &hcfg);
    let estimator_note = if first.fallback_engaged {
        format!(
            "  (using fallback estimator `{:?}` — λ₁ ≈ 0, ℓ_c = {:.4})",
            first.estimator_used, first.l_c
        )
    } else {
        format!(
            "  (using `{:?}` — λ₁ = {:.4}, ℓ_c = {:.4})",
            first.estimator_used, lambda1, first.l_c
        )
    };
    line("estimator", estimator_note);
    for &tau in &taus {
        let r = horizon_with(tau, k, store, lambda1, &hcfg);
        line(
            &format!("  τ = {:>4.1}", tau),
            if r.s_max.is_finite() {
                format!("s_max = {:>10.2} positions", r.s_max)
            } else {
                "s_max = ∞    (flat geometry)".to_string()
            },
        );
    }

    println!();
    let depth = encoding_depth(k, lambda1);
    println!("  ┌── DEPTH classification");
    line("  level", format!("{} ({:?})", depth.label(), depth));
    line(
        "  erasure energy",
        match depth {
            EncodingDepth::Tangent => "low",
            EncodingDepth::Connection => "moderate",
            EncodingDepth::Metric => "high",
            EncodingDepth::Topological => "infinite",
        },
    );
    println!("  description:");
    for chunk in depth.description().split_whitespace().fold(
        Vec::<String>::new(),
        |mut acc, w| {
            if let Some(last) = acc.last_mut() {
                if last.len() + 1 + w.len() <= 64 {
                    last.push(' ');
                    last.push_str(w);
                    return acc;
                }
            }
            acc.push(w.to_string());
            acc
        },
    ) {
        println!("    {}", chunk);
    }

    (k, lambda1, depth)
}

// ── JTBD: what should a builder DO with these numbers? ────────────

fn routing_decision(label: &str, k: f64, _lambda1: f64, depth: EncodingDepth) {
    header(format!("ROUTING DECISION — {}", label).as_str());
    let c_default = capacity(1.0, k);

    if !c_default.is_finite() {
        line("verdict", "PROCEED (flat geometry, infinite capacity)");
        line("rationale", "K ≈ 0 — the substrate has unlimited room.");
    } else if c_default > 5.0 {
        line("verdict", "PROCEED — substrate is decisive");
        line(
            "rationale",
            format!("C(1.0) = {:.2} > 5 — substrate can hold many readings.", c_default),
        );
    } else if c_default > 1.0 {
        line("verdict", "PROCEED WITH HEDGE — substrate is loaded");
        line(
            "rationale",
            format!("C(1.0) = {:.2} — moderate load, voice uncertainty.", c_default),
        );
    } else {
        line("verdict", "ASK — substrate is at capacity");
        line(
            "rationale",
            format!("C(1.0) = {:.2} ≤ 1 — overloaded, request clarification.", c_default),
        );
    }

    let depth_advice = match depth {
        EncodingDepth::Tangent => {
            "Tangent depth — facts here update freely. Cheap writes; users can correct."
        }
        EncodingDepth::Connection => {
            "Connection depth — skill-level persistence. Writes update neighborhoods, \
             not single facts. Be deliberate about updates."
        }
        EncodingDepth::Metric => {
            "Metric depth — beliefs resist counter-argument. Don't try to argue users \
             out of these; route to a different substrate region if needed."
        }
        EncodingDepth::Topological => {
            "Topological depth — irrecoverable structure. Trauma / foundational axioms / \
             identity layer. Do not attempt to overwrite; route to human."
        }
    };
    line("encoding advice", depth_advice);
}

// ── main ──────────────────────────────────────────────────────────

fn main() {
    println!("Branch VII — Cognitive Geometry verbs — JTBD demo");
    println!("==================================================");
    println!();
    println!("Two bundles, same schema, different data. Three verbs.");
    println!("Concrete routing decision per bundle.");

    let peaceful = build_peaceful_bundle();
    let volatile = build_volatile_bundle();

    let (k_p, l_p, d_p) = report_bundle("BUNDLE A — peaceful (real sensor data, 20 records)", &peaceful);
    let (k_v, l_v, d_v) = report_bundle("BUNDLE B — volatile (synthetic high-variance, 20 records)", &volatile);

    routing_decision("BUNDLE A peaceful", k_p, l_p, d_p);
    routing_decision("BUNDLE B volatile", k_v, l_v, d_v);

    header("WHAT JUST HAPPENED");
    println!(
        r#"  Same schema. Same record count. Different geometry.

  Bundle A (real, peaceful): K = {:.4}, depth = {} ({:?})
  Bundle B (synthetic, volatile): K = {:.4}, depth = {} ({:?})

  The three verbs gave concrete, audit-ready routing decisions
  without any labeled training data and without any
  domain-specific classifier. A builder calling these from a
  live query path now has a principled criterion for:

    • PROCEED / HEDGE / ASK (CAPACITY)
    • coherent-context-depth budgeting (HORIZON)
    • erasure / write strategy (DEPTH)

  No fine-tuning, no gradient descent, deterministic per
  (K, λ₁). The decision is one inner-product computation and
  one comparison.

  This is the substrate equivalent of "epistemic restraint":
  the database itself tells the application when to ask."#,
        k_p, d_p.label(), d_p, k_v, d_v.label(), d_v
    );

    header("WITHOUT OVERRIDE vs WITH OVERRIDE — DepthConfig in action");
    println!(
        r#"  The default classification collapses both bundles to IV
  (Topological) because `spectral_gap` returns ~0 on sensor-style
  bundles and the default rule `λ₁ < 0.01 → Topological` catches
  them. CAPACITY and HORIZON are unaffected — they expose raw
  geometric quantities without thresholding.

  Builders who know their substrate type can override the
  threshold via the new DepthConfig surface. Below: same two
  bundles, classified with `lambda1_topological = -1.0` (so the
  topological cut never trips, releasing the cascade to consider
  K alone):
"#
    );
    let permissive = DepthConfig {
        lambda1_topological: -1.0,
        ..DepthConfig::default()
    };
    let d_p_perm = encoding_depth_with(k_p, l_p, &permissive);
    let d_v_perm = encoding_depth_with(k_v, l_v, &permissive);
    line(
        "Bundle A peaceful (was IV)",
        format!("→ {} ({:?})", d_p_perm.label(), d_p_perm),
    );
    line(
        "Bundle B volatile (was IV)",
        format!("→ {} ({:?})", d_v_perm.label(), d_v_perm),
    );
    println!();
    println!(
        r#"  GQL form:    DEPTH <bundle> LAMBDA1_TOPOLOGICAL -1.0
  HTTP form:   GET /v1/bundles/{{name}}/depth?lambda1_topological=-1.0
  All four thresholds can be overridden independently and
  optionally; unspecified ones keep the published Theorem 8.14
  defaults. The HTTP response echoes `config_used` so the caller
  can audit which thresholds produced the verdict.

  The published defaults are not wrong — they're calibrated for
  graph-Laplacian substrates where λ₁ is a non-degenerate signal.
  For dense-vector / continuous substrates (sensor data, BGE
  embeddings), the per-substrate-type override is the right
  surface. Per-bundle calibration (fitting thresholds from the
  joint (K, λ₁) distribution at bundle-load time) is the v2
  follow-up — same move as the δ recalibration 0.657 → 0.74."#
    );
}
