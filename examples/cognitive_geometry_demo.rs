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
    capacity, encoding_depth_with, horizon_with, perceive, scalar_curvature, DepthConfig,
    EncodingDepth, HorizonConfig,
};
use gigi::geometry::forms::{ClosedTwoForm, TwoForm};
use gigi::geometry::transport::{flat_transport, BSource, TransportSegment};
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
    // Use the substrate-aware constructor so the JTBD demo shows the
    // out-of-the-box correct behavior on sensor bundles. `auto_for`
    // inspects `spectral_gap(store)` and picks the continuous-substrate
    // defaults (λ₁ cuts zeroed) when the graph estimator is degenerate
    // — exactly the case the original demo flagged. Same surface call
    // works for graph substrates too, where it falls through to the
    // published Theorem 8.14 defaults.
    let dcfg = DepthConfig::auto_for(store, 1e-9);
    let dcfg_note = if dcfg == DepthConfig::for_continuous_substrate() {
        "auto-selected `for_continuous_substrate()` (λ₁ degenerate)".to_string()
    } else {
        "auto-selected `for_graph_substrate()` (published Theorem 8.14)".to_string()
    };
    let depth = encoding_depth_with(k, lambda1, &dcfg);
    println!("  ┌── DEPTH classification");
    line("  config", dcfg_note);
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

    header("RAW DEFAULTS vs `auto_for` — DepthConfig substrate selection");
    println!(
        r#"  The published Theorem 8.14 defaults (graph-Laplacian
  thresholds) collapse both bundles to IV (Topological) because
  `spectral_gap` returns ~0 on sensor-style substrates and the
  default rule `λ₁ < 0.01 → Topological` catches them. CAPACITY
  and HORIZON are unaffected — they expose raw geometric
  quantities without thresholding.

  The fix shipped on this branch is `DepthConfig::auto_for(store)`,
  which inspects the bundle's spectral gap and picks the
  continuous-substrate defaults (λ₁ cuts zeroed) when the graph
  estimator is degenerate. The main DEPTH block above already uses
  it. Side-by-side, the same two bundles classified under:

    A. raw `DepthConfig::default()` (graph-substrate thresholds)
    B. `DepthConfig::auto_for(store, 1e-9)` (substrate-aware)
    C. manual per-axis override
       (`lambda1_topological = -1.0` releases the cascade)
"#
    );
    let raw = DepthConfig::default();
    let d_p_raw = encoding_depth_with(k_p, l_p, &raw);
    let d_v_raw = encoding_depth_with(k_v, l_v, &raw);
    line(
        "A.  raw   peaceful",
        format!("→ {} ({:?})", d_p_raw.label(), d_p_raw),
    );
    line(
        "A.  raw   volatile",
        format!("→ {} ({:?})", d_v_raw.label(), d_v_raw),
    );

    let auto_p = DepthConfig::auto_for(&peaceful, 1e-9);
    let auto_v = DepthConfig::auto_for(&volatile, 1e-9);
    let d_p_auto = encoding_depth_with(k_p, l_p, &auto_p);
    let d_v_auto = encoding_depth_with(k_v, l_v, &auto_v);
    line(
        "B.  auto  peaceful",
        format!("→ {} ({:?})", d_p_auto.label(), d_p_auto),
    );
    line(
        "B.  auto  volatile",
        format!("→ {} ({:?})", d_v_auto.label(), d_v_auto),
    );

    let permissive = DepthConfig {
        lambda1_topological: -1.0,
        ..DepthConfig::default()
    };
    let d_p_perm = encoding_depth_with(k_p, l_p, &permissive);
    let d_v_perm = encoding_depth_with(k_v, l_v, &permissive);
    line(
        "C.  manual peaceful",
        format!("→ {} ({:?})", d_p_perm.label(), d_p_perm),
    );
    line(
        "C.  manual volatile",
        format!("→ {} ({:?})", d_v_perm.label(), d_v_perm),
    );
    println!();
    println!(
        r#"  GQL forms:
    DEPTH <bundle>                              -- raw defaults
    DEPTH <bundle> LAMBDA1_TOPOLOGICAL -1.0     -- per-axis override
  HTTP form:
    GET /v1/bundles/{{name}}/depth?lambda1_topological=-1.0
  Rust API:
    encoding_depth_with(k, λ₁, &DepthConfig::auto_for(store, 1e-9))

  All four thresholds can be overridden independently and
  optionally; unspecified ones keep the published Theorem 8.14
  defaults. The HTTP response echoes `config_used` so the caller
  can audit which thresholds produced the verdict.

  The published defaults are not wrong — they're calibrated for
  graph-Laplacian substrates where λ₁ is a non-degenerate signal.
  `auto_for` picks the right substrate-type defaults by
  introspecting `spectral_gap(store)`, so DEPTH works out of the
  box on both substrate types. Per-bundle calibration (fitting
  thresholds from the joint (K, λ₁) distribution at bundle-load
  time) is the v2 follow-up — same move as the δ recalibration
  0.657 → 0.74."#
    );

    perceive_section();
}

/// PERCEIVE — Theorem 8.6 (Davis 2026). The fourth and final Cognitive
/// Geometry verb. Where CAPACITY / HORIZON / DEPTH read scalar features
/// off the bundle's static geometry, PERCEIVE acts on a vector:
///
///   v_perceived = R_acc · v
///   bias        = ‖R_acc − I‖_F
///
/// R_acc is the rotation the substrate accumulated while transporting v
/// along some path. The bias scalar quantifies how much v has drifted
/// from its starting frame.
///
/// The JTBD: a builder has a retrieved vector that came back from a
/// curved sub-bundle. PERCEIVE tells them (a) what the system actually
/// perceives that vector to be after the transport, and (b) how much
/// to trust that perception (low bias ⇒ near-identity rotation ⇒ no
/// distortion; high bias ⇒ substantial frame drift, hedge before
/// acting).
fn perceive_section() {
    header("PERCEIVE — Theorem 8.6 (R_acc on a real transport)");
    println!(
        r#"  PERCEIVE is the fourth Cognitive Geometry verb. Where the
  other three read static geometric scalars (K, λ₁), PERCEIVE
  acts on a vector — it asks "what does the substrate actually
  perceive this vector to be after parallel transport, and how
  much should we trust that perception?"

  The chain: TRANSPORT → R_acc (from TransportResult.rotation,
  shipped on this branch) → perceive(R_acc, v) → (v_perceived, bias).
"#
    );

    // 1) Set up a small representative transport — a magnetic flow in
    //    3D about the z-axis. Builders running the demo see all the
    //    moving parts in one place.
    let b_strength = 0.6_f64;
    let bias_mat = vec![
        0.0, -b_strength, 0.0,
        b_strength, 0.0, 0.0,
        0.0, 0.0, 0.0,
    ];
    let bias = ClosedTwoForm::new_constant(
        TwoForm::new(bias_mat, 3).expect("antisymmetric"),
    );
    let seg = TransportSegment::new(
        vec![0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![1.0, 0.0, 0.5],
    )
    .expect("valid segment");
    let dt = 1e-4;
    let n_steps = 5_000usize;
    let t = dt * n_steps as f64; // total transport time

    let r = flat_transport(&seg, Some(&bias), dt, n_steps, BSource::Override)
        .expect("transport succeeds");
    let rotation = r.rotation.as_ref().expect("R_acc present on success");

    line("transport bias", format!("constant B = {} dx∧dy in (x,y) plane", b_strength));
    line(
        "transport time T",
        format!("{:.3} (dt={:.0e} × {} steps)", t, dt, n_steps),
    );
    line("initial v", format!("{:?}", &[1.0_f64, 0.0, 0.5]));
    line("final v (after RK4)", format!("{:?}", r.final_velocity));
    line(
        "holonomy_norm",
        format!("{:.6}  (‖v_final − v_initial‖)", r.holonomy_norm),
    );

    println!();
    println!("  ┌── R_acc (3×3 row-major)");
    for i in 0..3 {
        println!(
            "    [ {:>10.6}, {:>10.6}, {:>10.6} ]",
            rotation[i * 3],
            rotation[i * 3 + 1],
            rotation[i * 3 + 2],
        );
    }
    println!(
        "    expected: rotation by θ = b·T = {:.3} rad about z-axis",
        b_strength * t
    );

    // 2) PERCEIVE on the initial velocity.
    println!();
    let v_initial = vec![1.0_f64, 0.0, 0.5];
    let res = perceive(rotation, &v_initial, 3).expect("perceive succeeds");

    println!("  ┌── PERCEIVE(R_acc, v_initial)");
    line("  v_perceived", format!("{:?}", res.v_perceived));
    line(
        "  matches final_velocity?",
        if res
            .v_perceived
            .iter()
            .zip(r.final_velocity.iter())
            .all(|(a, b)| (a - b).abs() < 1e-5)
        {
            "✓ to RK4 tolerance"
        } else {
            "✗ mismatch (BUG)"
        },
    );
    line("  bias = ‖R - I‖_F", format!("{:.6}", res.bias));

    // 3) Interpret the bias.
    let theta = b_strength * t;
    let closed_form = (4.0 * (1.0 - theta.cos())).sqrt();
    line(
        "  closed-form bias",
        format!("sqrt(4·(1-cos θ)) = {:.6}", closed_form),
    );

    let interpretation = if res.bias < 0.1 {
        "low — substrate has not meaningfully distorted v"
    } else if res.bias < 1.0 {
        "moderate — v_perceived diverges; reweight before acting"
    } else {
        "high — substantial drift; treat v_perceived as low-confidence"
    };
    line("  regime", interpretation);

    // 4) Pivot to a second vector to show PERCEIVE's per-vector nature.
    println!();
    println!("  ┌── PERCEIVE on a different vector (same R_acc)");
    let v_alt = vec![0.0_f64, 1.0, 0.0]; // pure y-axis input
    let res_alt = perceive(rotation, &v_alt, 3).expect("perceive succeeds");
    line("  v_alt          ", format!("{:?}", v_alt));
    line("  v_perceived_alt", format!("{:?}", res_alt.v_perceived));
    line(
        "  bias (unchanged)",
        format!("{:.6}  (bias is a property of R, not v)", res_alt.bias),
    );

    // 5) Wire surfaces summary.
    println!();
    println!(
        r#"  ┌── Wire surfaces

  Rust API:
    let r = flat_transport(...).unwrap();
    let res = perceive(&r.rotation.unwrap(), &v, dim).unwrap();
    // res.v_perceived, res.bias

  HTTP (POST):
    /v1/bundles/{{name}}/perceive
    body: {{ rotation: [r00,...], vector: [v0,...], dim: N }}
    resp: {{ v_perceived, bias, dim, bundle, interpretation }}

  JTBD recap (4-verb routing pipeline):

    1. CAPACITY → can we hold the interpretation? (PROCEED / HEDGE / ASK)
    2. HORIZON  → how deep is coherent context here? (token budget)
    3. DEPTH    → what's the erasure energy of writing? (write strategy)
    4. PERCEIVE → what does the substrate actually see this vector as?
                  How much do we trust that perception?

  PERCEIVE is the only verb that acts on a *specific* retrieved
  value. It closes the loop: not just "what does the geometry tell
  us about this region" but "what does the geometry do to this
  particular answer when we move it through the substrate"."#
    );
}
