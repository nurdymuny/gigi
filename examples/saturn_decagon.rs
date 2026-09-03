//! Saturn's south-polar decagon, analysed as a fiber bundle.
//!
//! Data: HST/WFC3-UVIS cylindrical maps from the Outer Planet Atmospheres
//! Legacy program (OPAL, PI Amy Simon, DOI 10.17909/T9G593). Public, no
//! proprietary period. Cycle 32, Saturn 2025a and 2025b, six filters.
//!
//! The bundle structure is not a metaphor here — it is the data:
//!
//!   base space  (lat, lon)   a patch of the sphere, the south polar cap
//!   fiber       6 filters    F467M F502N F631N F763M FQ727N FQ889N
//!
//! Each filter probes a different altitude (methane bands FQ727N/FQ889N sit
//! highest), so the fiber over each ground point is a vertical column through
//! the atmosphere. A section of this bundle is one epoch's observation.
//!
//! In September 2026 Sánchez-Lavega et al. reported (Science Advances) a
//! ten-sided wave centred at 63 degrees south. This example reconstructs that
//! detection from the public maps and measures the bundle geometry on either
//! side of the jet.
//!
//! Run:  cargo run --release --example saturn_decagon -- <cells.json>

use gigi::bundle::BundleStore;
use gigi::curvature;
use gigi::spectral::{self, SpectralGap};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

const FILTERS: [&str; 6] = ["f467m", "f502n", "f631n", "f763m", "fq727n", "fq889n"];

fn schema(name: &str) -> BundleSchema {
    let mut s = BundleSchema::new(name)
        .base(FieldDef::numeric("lat"))
        .base(FieldDef::numeric("lon"));
    for f in FILTERS {
        s = s.fiber(FieldDef::numeric(f));
    }
    s
}

/// Normalised amplitude of azimuthal wavenumber `m` in one latitude ring.
fn zonal_mode(vals: &[(f64, f64)], m: usize) -> f64 {
    let n = vals.len();
    if n < 2 * m + 2 {
        return 0.0;
    }
    let mean = vals.iter().map(|(_, v)| v).sum::<f64>() / n as f64;
    let (mut re, mut im) = (0.0, 0.0);
    let mut band = 0.0;
    for mm in 5..14usize {
        let (mut r, mut i) = (0.0, 0.0);
        for (lon, v) in vals {
            let th = mm as f64 * lon.to_radians();
            r += (v - mean) * th.cos();
            i += (v - mean) * th.sin();
        }
        let a = (r * r + i * i).sqrt();
        band += a;
        if mm == m {
            re = r;
            im = i;
        }
    }
    if band <= 0.0 {
        0.0
    } else {
        (re * re + im * im).sqrt() / band
    }
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: saturn_decagon <saturn_polar_cells.json>");
        std::process::exit(2);
    });
    let raw = std::fs::read_to_string(&path).expect("read cells");
    let cells: Vec<serde_json::Value> = serde_json::from_str(&raw).expect("parse cells");

    for epoch in ["2025a", "2025b"] {
        let mut store = BundleStore::new(schema(&format!("saturn_south_{epoch}")));
        let mut rings: std::collections::BTreeMap<i64, Vec<(f64, f64)>> = Default::default();

        for c in cells.iter().filter(|c| c["epoch"] == epoch) {
            let lat = c["lat"].as_f64().unwrap();
            let lon = c["lon"].as_f64().unwrap();
            let mut r = Record::new();
            r.insert("lat".into(), Value::Float(lat));
            r.insert("lon".into(), Value::Float(lon));
            for f in FILTERS {
                r.insert(f.into(), Value::Float(c[f].as_f64().unwrap()));
            }
            store.insert(&r);
            // F631N is the continuum band the wave was first seen in
            rings.entry(lat as i64).or_default().push((lon, c["f631n"].as_f64().unwrap()));
        }

        // ---- GIGI's bundle geometry over the whole polar cap ----
        let k = curvature::scalar_curvature(&store);
        let lambda = spectral::spectral_gap(&store);
        let (b0, b1) = spectral::betti_numbers(&store);

        println!("\n=== {epoch} — south polar cap as a fiber bundle ===");
        println!("  sections (records)   {}", store.len());
        println!("  scalar curvature K   {k:.6}");
        println!("  confidence           {:.4}", curvature::confidence(k));
        println!("  Betti                b0={b0}  b1={b1}");
        match lambda {
            SpectralGap::Measured(v) => println!("  spectral gap lambda1 {v:.6}"),
            SpectralGap::Undefined { components, records } => println!(
                "  spectral gap         undefined ({components} components over {records} records)"
            ),
        }

        // ---- where does the m=10 wave live? ----
        println!("\n  {:>6}  {:>8}  {:>8}   {:>7}", "lat", "m=10", "m=6", "C=tau/K");
        let mut best = (0i64, 0.0f64);
        for (lat, vals) in &rings {
            let mut v = vals.clone();
            v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            let m10 = zonal_mode(&v, 10);
            let m6 = zonal_mode(&v, 6);
            if m10 > best.1 {
                best = (*lat, m10);
            }
            // tau: the wave's own transport scale in this ring, taken as the
            // fractional brightness contrast the mode carries.
            let mean = v.iter().map(|(_, x)| x).sum::<f64>() / v.len() as f64;
            let tau = m10 * v.iter().map(|(_, x)| (x - mean).abs()).sum::<f64>()
                / (v.len() as f64 * mean.max(1e-9));
            let cap = curvature::capacity(tau, k.abs().max(1e-12));
            let mark = if m10 > 0.25 { "  <<<" } else { "" };
            println!("  {lat:>+6}  {m10:>8.3}  {m6:>8.3}   {cap:>7.2}{mark}");
        }
        println!(
            "\n  strongest m=10 ring: {:+} deg  (published: 63 deg south)",
            best.0
        );
    }
}
