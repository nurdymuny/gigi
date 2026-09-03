//! Saturn's polar wave modes as a fiber bundle over observation space.
//!
//! This is the restructure. The first attempt (examples/saturn_decagon.rs) put
//! (lat, lon) in the base and filter brightness in the fiber, and GIGI could
//! say nothing about wavenumber — because `scalar_curvature` is
//! `mean_f(variance/range^2)`, and both `variance()` (m2/count) and `range()`
//! (max-min) are symmetric functions of the value multiset. Longitude ORDER
//! never enters. Shuffling the ring leaves K identical to ~1e-17 while the m=10
//! amplitude collapses. Curvature was structurally blind to the thing we cared
//! about.
//!
//! The fix is to do the mode decomposition BEFORE ingestion and give each
//! wavenumber its own field:
//!
//!   base space   (year, filter)    where and when we looked
//!   fiber        a5 .. a13         the nine mode amplitudes there
//!
//! Now m is carried by field IDENTITY rather than hidden inside a
//! permutation-invariant summary, so per-field statistics are per-mode
//! statistics and GIGI is doing real work.
//!
//! WHAT K MEANS HERE, precisely. For one field, var/range^2 is a SHAPE
//! statistic, not a magnitude — it is scale-free. Popoviciu bounds it above:
//!
//!   var <= range^2 / 4    =>   var/range^2 in [0, 0.25]
//!
//! and three values are readable:
//!
//!   -> 0.25    mass at BOTH extremes and little between: BIMODAL.
//!              A mode whose amplitude sat at one level, then moved to another,
//!              and spent little time in between. That is the signature of a
//!              REGIME TRANSITION (Davis two-regime / trichotomy).
//!   ~  0.083   uniform spread (1/12): unstructured variation, i.e. noise.
//!   -> 0       tight cluster plus one far outlier: a spike, or bad data.
//!
//! So a per-mode curvature scan over an observation-space bundle asks: which
//! wavenumbers behaved bimodally across the years? That is a question about
//! regime change, and it is one GIGI can answer natively.
//!
//! Run:  cargo run --release --example saturn_mode_bundle -- <mode_amplitudes.json>

use gigi::bundle::BundleStore;
use gigi::curvature;
use gigi::types::{BundleSchema, FieldDef, Record, Value};

const MODES: [usize; 9] = [5, 6, 7, 8, 9, 10, 11, 12, 13];

const UNIFORM: f64 = 1.0 / 12.0; // var/range^2 for a uniform distribution
const BIMODAL: f64 = 0.25; // Popoviciu bound, mass at both extremes

fn schema(name: &str) -> BundleSchema {
    let mut s = BundleSchema::new(name)
        .base(FieldDef::numeric("year"))
        .base(FieldDef::categorical("filter"));
    for m in MODES {
        s = s.fiber(FieldDef::numeric(&format!("a{m}")));
    }
    s
}

/// Read `var/range^2` for one field straight from GIGI's streaming field stats —
/// the same quantity `scalar_curvature` averages.
fn field_k(store: &BundleStore, field: &str) -> Option<(f64, f64, f64)> {
    for (name, fs) in store.field_stats() {
        if name == field {
            if fs.count < 2 {
                return None;
            }
            let r = fs.range();
            if r <= f64::EPSILON {
                return None;
            }
            return Some((fs.variance() / (r * r), fs.mean, r));
        }
    }
    None
}

fn label(k: f64) -> &'static str {
    if k > 0.18 {
        "BIMODAL  <- two regimes"
    } else if k > 0.11 {
        "spread, leaning bimodal"
    } else if k > 0.055 {
        "~uniform (noise-like)"
    } else {
        "concentrated + outlier"
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: saturn_mode_bundle <mode_amplitudes.json>");
        std::process::exit(2);
    });
    let raw = std::fs::read_to_string(&path).expect("read amplitudes");
    let rows: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("parse amplitudes");

    let mut rings: Vec<String> = rows
        .iter()
        .map(|r| r["ring"].as_str().unwrap_or("?").to_string())
        .collect();
    rings.sort();
    rings.dedup();

    println!("Saturn polar modes as a bundle over (year, filter)");
    println!("  landmarks:  uniform/noise {UNIFORM:.3}   bimodal/two-regime {BIMODAL:.3}\n");

    let mut summary: Vec<(String, f64, f64)> = Vec::new();

    for ring in &rings {
        let mut store = BundleStore::new(schema(&format!("saturn_{}", ring.replace('-', "m"))));
        let mut n = 0usize;
        for r in rows.iter().filter(|r| r["ring"].as_str() == Some(ring.as_str())) {
            let mut rec = Record::new();
            rec.insert("year".into(), Value::Float(r["year"].as_f64().unwrap()));
            rec.insert(
                "filter".into(),
                Value::Text(r["filter"].as_str().unwrap_or("?").to_string()),
            );
            for m in MODES {
                let v = r[format!("a{m}")].as_f64().unwrap_or(0.0);
                rec.insert(format!("a{m}"), Value::Float(v));
            }
            store.insert(&rec);
            n += 1;
        }
        if n < 2 {
            continue;
        }

        let k = curvature::scalar_curvature(&store);
        println!("=== ring {ring}   ({n} sections over (year, filter)) ===");
        println!(
            "  GIGI scalar curvature K = {k:.4}   confidence {:.4}",
            curvature::confidence(k)
        );
        println!("  {:>5}  {:>10}  {:>10}  {:>9}   reading", "mode", "mean amp", "range", "var/rng^2");

        let mut worst = (0usize, 0.0f64);
        for m in MODES {
            if let Some((km, mean, range)) = field_k(&store, &format!("a{m}")) {
                if km > worst.1 {
                    worst = (m, km);
                }
                let star = if m == 10 || m == 6 { " *" } else { "  " };
                println!(
                    "  m={m:<3}{star}{mean:10.5}  {range:10.5}  {km:9.4}   {}",
                    label(km)
                );
            }
        }
        println!(
            "  most bimodal mode: m={}  (var/range^2 = {:.4})\n",
            worst.0, worst.1
        );
        summary.push((ring.clone(), k, worst.1));
    }

    println!("--- comparison across rings ---");
    println!("  {:>10}  {:>9}  {:>14}", "ring", "K", "peak mode K");
    for (r, k, w) in &summary {
        println!("  {r:>10}  {k:9.4}  {w:14.4}");
    }
    println!(
        "\n  The jet ring should show a HIGHER peak per-mode curvature than the\n  \
         control ring if a wavenumber there genuinely changed regime, rather than\n  \
         the whole cap drifting together with viewing geometry."
    );
}
