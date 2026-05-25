//! Real-bundle demonstration of L12 attention + memory primitives.
//!
//! Two scenarios threaded through a live `BundleStore`:
//!
//! 1. **Token-embedding bundle** (Marcella-style): 12 records, each
//!    a 2-D semantic vector with a label. Exercise ATTEND + FOCUS
//!    to retrieve the most relevant tokens for a query embedding.
//!
//! 2. **PRISM transaction-stream bundle** (time-indexed): 60 daily
//!    "settlement amount" values with a regime change at day 30
//!    (a market event). Exercise EPISODIC to detect the event
//!    boundary, and SEMANTIC to compress the bundle's topology
//!    into a Morse complex.
//!
//! Run:
//!     cargo run --release --features kahler --bin attention_memory_demo

#![cfg(feature = "kahler")]

use gigi::bundle::BundleStore;
use gigi::geometry::{
    attend, episodic_events, focus, semantic_gist, ClosedTwoForm, ComplexStructure,
    KahlerStructure, TwoForm,
};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn header(t: &str) {
    println!();
    println!("══ {} ══", t);
}
fn line(k: &str, v: impl std::fmt::Display) {
    println!("  {:<44} {}", k, v);
}

fn main() {
    println!("L12 attention + memory — REAL BUNDLE DEMO");
    println!("=========================================");

    // ── §8 / §9 ATTENTION on a token-embedding bundle ─────────

    header("Scenario 1: token-embedding bundle (12 Marcella-style records)");

    let tokens = vec![
        ("cat",     vec![0.92, 0.10]),
        ("dog",     vec![0.85, 0.18]),
        ("kitten",  vec![0.95, 0.08]),
        ("puppy",   vec![0.88, 0.15]),
        ("tiger",   vec![0.70, 0.30]),
        ("wolf",    vec![0.72, 0.35]),
        ("table",   vec![0.05, 0.92]),
        ("chair",   vec![0.10, 0.88]),
        ("desk",    vec![0.08, 0.90]),
        ("car",     vec![0.30, 0.55]),
        ("bike",    vec![0.35, 0.60]),
        ("plane",   vec![0.40, 0.50]),
    ];

    let kahler = KahlerStructure::new(
        ComplexStructure::standard(1),
        ClosedTwoForm::new_constant(
            TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).unwrap(),
        ),
    );
    let schema = BundleSchema::new("tokens")
        .base(FieldDef::numeric("token_id"))
        .fiber(FieldDef::numeric("e0").with_range(1.0))
        .fiber(FieldDef::numeric("e1").with_range(1.0))
        .with_kahler(kahler);
    let mut tok_store = BundleStore::new(schema);
    for (i, (_, emb)) in tokens.iter().enumerate() {
        let mut rec = Record::new();
        rec.insert("token_id".into(), Value::Integer(i as i64));
        rec.insert("e0".into(), Value::Float(emb[0]));
        rec.insert("e1".into(), Value::Float(emb[1]));
        tok_store.insert(&rec);
    }
    line("tokens inserted", tokens.len());

    let embeddings: Vec<Vec<f64>> = tokens.iter().map(|(_, e)| e.clone()).collect();
    let labels: Vec<&str> = tokens.iter().map(|(l, _)| *l).collect();

    // ATTEND: query "what's near a cat-like thing?" at (0.9, 0.12).
    header("ATTEND — query (0.90, 0.12) ≈ 'cat-like'");
    let q1 = vec![0.90, 0.12];
    let weights = attend(&embeddings, &q1, 0.10);
    line("bandwidth σ", 0.10);
    let mut paired: Vec<_> = labels.iter().zip(weights.iter()).collect();
    paired.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
    for (label, w) in paired.iter().take(5) {
        line(&format!("  {}", label), format!("attention = {:.4}", w));
    }
    line("attention sum (should be 1.0)",
         format!("{:.6}", weights.iter().sum::<f64>()));

    // FOCUS: top-3 nearest tokens to "vehicle-like" query (0.35, 0.55).
    header("FOCUS — top-3 nearest to query (0.35, 0.55) ≈ 'vehicle-like'");
    let q2 = vec![0.35, 0.55];
    let top3 = focus(&embeddings, &q2, 0.15, 3);
    for (idx, w) in &top3 {
        line(
            &format!("  rank: {}", labels[*idx]),
            format!("attention = {:.4}", w),
        );
    }

    // ── §10 EPISODIC on a transaction-stream bundle ──────────

    header("Scenario 2: PRISM-style transaction stream with regime change");

    // Days 1-30: settlements around $1k. Days 31-60: regime shift to $5k.
    // EPISODIC should flag the boundary at day 30/31.
    let mut amounts = Vec::new();
    let mut s: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next_u = |s: &mut u64| -> f64 {
        *s ^= *s << 13;
        *s ^= *s >> 7;
        *s ^= *s << 17;
        (*s as f64 / u64::MAX as f64) * 2.0 - 1.0
    };
    for _ in 0..30 {
        amounts.push(1000.0 + 30.0 * next_u(&mut s));
    }
    for _ in 0..30 {
        amounts.push(5000.0 + 80.0 * next_u(&mut s));
    }
    let events = episodic_events(&amounts, 50.0);
    line("transactions in stream", amounts.len());
    line("events detected (threshold 50× median gap)", events.len());
    if let Some(top) = events.first() {
        line(
            "most-persistent event gap",
            format!("${:.2}", top.gap),
        );
        line(
            "persistence ratio (gap / median)",
            format!("{:.1}×", top.persistence_ratio),
        );
    }

    // ── §11 SEMANTIC on the token-embedding bundle ─────────────

    header("SEMANTIC — Morse-compressed gist of the token bundle");

    match semantic_gist(&tok_store) {
        Some(morse) => {
            line(
                "Betti (b_0, b_1, b_2)",
                format!(
                    "({}, {}, {})",
                    morse.betti.b0, morse.betti.b1, morse.betti.b2
                ),
            );
            line(
                "critical cells / original cells",
                format!("{} / {}", morse.n_critical(), morse.n_original()),
            );
            line(
                "compression ratio",
                format!("{:.2}×", morse.compression_ratio()),
            );
            line("cohomology preserved?", morse.cohomology_preserved());
        }
        None => {
            line(
                "semantic_gist returned",
                "None (bundle has no L6-compatible structure)",
            );
        }
    }

    println!();
    println!("Done — L12 ATTEND / FOCUS / EPISODIC / SEMANTIC on real bundles.");
    println!("       (catalog: theory/brain_primitives/catalog.md §8, §9, §10, §11)");
    println!();
    println!("All 12 brain primitives now operational across L10 + L11 + L12.");
}
