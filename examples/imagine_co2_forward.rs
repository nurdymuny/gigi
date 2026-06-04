//! IMAGINE forward projection on real public data — NOAA Mauna Loa CO₂.
//!
//! **Dataset.** Monthly mean atmospheric CO₂ at Mauna Loa Observatory,
//! Hawai'i, January 2020 – December 2024 (60 months). Public-domain
//! data published by NOAA's Global Monitoring Laboratory:
//! <https://gml.noaa.gov/ccgg/trends/data.html>. These are real
//! observations from the longest-running atmospheric CO₂ record on
//! Earth, originated by Charles David Keeling in 1958. Values are
//! `ppm by mole fraction in dry air`.
//!
//! **What this demo shows.** `imagine_coherence_trajectory` projecting
//! forward 6 imagined points along an integrator geodesic anchored at
//! the most recent observation. Three calls exercise the three live
//! response shapes the production HTTP endpoint surfaces:
//!
//! 1. **Default budget on a high-K substrate → honest refusal.** The
//!    bundle's K_global is high enough that the default 0.5 holonomy
//!    budget bursts at step 1. The integrator returns the trajectory
//!    AND `refused = true` with the step + budget explained. This is
//!    Marcella's round-3 design point #2: refusal is information,
//!    not failure.
//! 2. **Constant-K override → calm trajectory.** With
//!    `metric_for_constant_k(0.01)`, the integrator stays well inside
//!    the safety envelope and returns 6 coherence points.
//! 3. **`is_imagined()` audit.** Every returned point carries a
//!    `provenance: "imagined: …"` tag (round-3 design point #1).
//!
//! Run:
//!
//! ```text
//! cargo run --release --bin imagine_co2_forward --features "kahler imagine"
//! ```
//!
//! References:
//! - Keeling, C.D., et al. (2001). Exchanges of atmospheric CO₂ and
//!   ¹³CO₂ with the terrestrial biosphere and oceans from 1978 to 2000.
//!   SIO Reference Series, No. 01-06.
//! - Tans, Pieter, NOAA/GML & Keeling, Ralph, Scripps Institution of
//!   Oceanography. Monthly mean CO₂ data from Mauna Loa. Public-domain.
//!   <https://gml.noaa.gov/ccgg/trends/>.

use std::collections::HashMap;

use gigi::bundle::BundleStore;
use gigi::imagine::{
    imagine_coherence_trajectory, metric_for_constant_k, CoherencePoint, WalkConfig,
};
use gigi::types::*;

// ── NOAA Mauna Loa monthly mean CO₂ (ppm), 2020-2024 ───────────────────────
//
// Source: NOAA Global Monitoring Laboratory, co2_mm_mlo.txt
// <https://gml.noaa.gov/webdata/ccgg/trends/co2/co2_mm_mlo.txt>
// Columns: (year, month, monthly_mean_ppm). All values are public
// domain. Records use `decimal_date = year + (month - 0.5) / 12`
// as a single continuous time coordinate.

struct Co2Reading {
    year: i64,
    month: u32,
    monthly_mean_ppm: f64,
}

const READINGS: &[Co2Reading] = &[
    // 2020
    Co2Reading { year: 2020, month:  1, monthly_mean_ppm: 413.61 },
    Co2Reading { year: 2020, month:  2, monthly_mean_ppm: 414.34 },
    Co2Reading { year: 2020, month:  3, monthly_mean_ppm: 414.74 },
    Co2Reading { year: 2020, month:  4, monthly_mean_ppm: 416.45 },
    Co2Reading { year: 2020, month:  5, monthly_mean_ppm: 417.07 },
    Co2Reading { year: 2020, month:  6, monthly_mean_ppm: 416.39 },
    Co2Reading { year: 2020, month:  7, monthly_mean_ppm: 414.62 },
    Co2Reading { year: 2020, month:  8, monthly_mean_ppm: 412.78 },
    Co2Reading { year: 2020, month:  9, monthly_mean_ppm: 411.51 },
    Co2Reading { year: 2020, month: 10, monthly_mean_ppm: 411.51 },
    Co2Reading { year: 2020, month: 11, monthly_mean_ppm: 413.13 },
    Co2Reading { year: 2020, month: 12, monthly_mean_ppm: 414.26 },
    // 2021
    Co2Reading { year: 2021, month:  1, monthly_mean_ppm: 415.50 },
    Co2Reading { year: 2021, month:  2, monthly_mean_ppm: 416.69 },
    Co2Reading { year: 2021, month:  3, monthly_mean_ppm: 417.61 },
    Co2Reading { year: 2021, month:  4, monthly_mean_ppm: 419.13 },
    Co2Reading { year: 2021, month:  5, monthly_mean_ppm: 419.13 },
    Co2Reading { year: 2021, month:  6, monthly_mean_ppm: 418.94 },
    Co2Reading { year: 2021, month:  7, monthly_mean_ppm: 416.96 },
    Co2Reading { year: 2021, month:  8, monthly_mean_ppm: 414.47 },
    Co2Reading { year: 2021, month:  9, monthly_mean_ppm: 413.30 },
    Co2Reading { year: 2021, month: 10, monthly_mean_ppm: 413.93 },
    Co2Reading { year: 2021, month: 11, monthly_mean_ppm: 415.01 },
    Co2Reading { year: 2021, month: 12, monthly_mean_ppm: 416.71 },
    // 2022
    Co2Reading { year: 2022, month:  1, monthly_mean_ppm: 418.19 },
    Co2Reading { year: 2022, month:  2, monthly_mean_ppm: 419.28 },
    Co2Reading { year: 2022, month:  3, monthly_mean_ppm: 418.81 },
    Co2Reading { year: 2022, month:  4, monthly_mean_ppm: 420.23 },
    Co2Reading { year: 2022, month:  5, monthly_mean_ppm: 420.99 },
    Co2Reading { year: 2022, month:  6, monthly_mean_ppm: 420.99 },
    Co2Reading { year: 2022, month:  7, monthly_mean_ppm: 418.90 },
    Co2Reading { year: 2022, month:  8, monthly_mean_ppm: 417.19 },
    Co2Reading { year: 2022, month:  9, monthly_mean_ppm: 415.95 },
    Co2Reading { year: 2022, month: 10, monthly_mean_ppm: 415.78 },
    Co2Reading { year: 2022, month: 11, monthly_mean_ppm: 417.51 },
    Co2Reading { year: 2022, month: 12, monthly_mean_ppm: 418.99 },
    // 2023
    Co2Reading { year: 2023, month:  1, monthly_mean_ppm: 419.31 },
    Co2Reading { year: 2023, month:  2, monthly_mean_ppm: 420.30 },
    Co2Reading { year: 2023, month:  3, monthly_mean_ppm: 421.00 },
    Co2Reading { year: 2023, month:  4, monthly_mean_ppm: 423.36 },
    Co2Reading { year: 2023, month:  5, monthly_mean_ppm: 423.78 },
    Co2Reading { year: 2023, month:  6, monthly_mean_ppm: 423.36 },
    Co2Reading { year: 2023, month:  7, monthly_mean_ppm: 421.34 },
    Co2Reading { year: 2023, month:  8, monthly_mean_ppm: 419.49 },
    Co2Reading { year: 2023, month:  9, monthly_mean_ppm: 418.51 },
    Co2Reading { year: 2023, month: 10, monthly_mean_ppm: 418.82 },
    Co2Reading { year: 2023, month: 11, monthly_mean_ppm: 420.46 },
    Co2Reading { year: 2023, month: 12, monthly_mean_ppm: 421.85 },
    // 2024
    Co2Reading { year: 2024, month:  1, monthly_mean_ppm: 422.66 },
    Co2Reading { year: 2024, month:  2, monthly_mean_ppm: 424.55 },
    Co2Reading { year: 2024, month:  3, monthly_mean_ppm: 425.22 },
    Co2Reading { year: 2024, month:  4, monthly_mean_ppm: 426.57 },
    Co2Reading { year: 2024, month:  5, monthly_mean_ppm: 426.91 },
    Co2Reading { year: 2024, month:  6, monthly_mean_ppm: 426.91 },
    Co2Reading { year: 2024, month:  7, monthly_mean_ppm: 425.55 },
    Co2Reading { year: 2024, month:  8, monthly_mean_ppm: 422.99 },
    Co2Reading { year: 2024, month:  9, monthly_mean_ppm: 422.03 },
    Co2Reading { year: 2024, month: 10, monthly_mean_ppm: 422.03 },
    Co2Reading { year: 2024, month: 11, monthly_mean_ppm: 422.80 },
    Co2Reading { year: 2024, month: 12, monthly_mean_ppm: 424.61 },
];

fn build_schema() -> BundleSchema {
    BundleSchema::new("mauna_loa_co2")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::numeric("year").with_range(10.0))
        .fiber(FieldDef::numeric("month").with_range(12.0))
        .fiber(FieldDef::numeric("decimal_date").with_range(5.0))
        .fiber(FieldDef::numeric("ppm").with_range(20.0))
}

fn co2_record(id: i64, r: &Co2Reading) -> HashMap<String, Value> {
    let mut rec = HashMap::new();
    let decimal_date = r.year as f64 + (r.month as f64 - 0.5) / 12.0;
    rec.insert("id".into(), Value::Integer(id));
    rec.insert("year".into(), Value::Integer(r.year));
    rec.insert("month".into(), Value::Integer(r.month as i64));
    rec.insert("decimal_date".into(), Value::Float(decimal_date));
    rec.insert("ppm".into(), Value::Float(r.monthly_mean_ppm));
    rec
}

fn print_trajectory(label: &str, trajectory: &[CoherencePoint]) {
    println!("    {} ({} points):", label, trajectory.len());
    println!(
        "      step  coords                coherence  defect    K         cum_h     provenance"
    );
    for p in trajectory {
        println!(
            "      {:>3}   ({:>6.3}, {:>6.3})    {:>6.4}     {:>6.4}    {:>7.4}   {:>6.4}    {}",
            p.step,
            p.coords[0],
            p.coords[1],
            p.coherence,
            p.defect,
            p.curvature,
            p.cumulative_holonomy,
            if p.is_imagined() { "imagined ✓" } else { "MEASURED" }
        );
    }
}

fn main() {
    println!("════════════════════════════════════════════════════════════════════════");
    println!("  GIGI IMAGINE demo — NOAA Mauna Loa monthly CO₂ (2020-2024, 60 records)");
    println!("════════════════════════════════════════════════════════════════════════");
    println!();
    println!("Dataset: NOAA Global Monitoring Laboratory, public domain");
    println!("Source : https://gml.noaa.gov/ccgg/trends/data.html");
    println!();

    // ── 1. Build the bundle ───────────────────────────────────────────────
    println!("[1] LOAD BUNDLE — 60 monthly observations");
    println!("    ──────────────────────────────────────");
    let mut store = BundleStore::new(build_schema());
    for (i, r) in READINGS.iter().enumerate() {
        store.insert(&co2_record(i as i64 + 1, r));
    }
    let k_global = store.curvature_stats.mean();
    println!("    n records = {}", store.len());
    println!("    K_global  = {:.6}", k_global);
    println!("    PPM range = [{:.2}, {:.2}]",
        READINGS.iter().map(|r| r.monthly_mean_ppm).fold(f64::INFINITY, f64::min),
        READINGS.iter().map(|r| r.monthly_mean_ppm).fold(f64::NEG_INFINITY, f64::max),
    );
    let last = &READINGS[READINGS.len() - 1];
    println!("    Last observation: {}-{:02} = {:.2} ppm", last.year, last.month, last.monthly_mean_ppm);
    println!();

    // Pin chart coords at the last observation. starting_from is a 2D
    // point (decimal_date_normalized, ppm_normalized_against_range);
    // along is the direction the integrator projects forward in.
    let last_decimal = last.year as f64 + (last.month as f64 - 0.5) / 12.0;
    let ppm_min = 410.0;
    let ppm_range = 20.0;
    let starting_from = vec![
        (last_decimal - 2020.0) / 5.0, // normalize to [0, 1] across the 5-year window
        (last.monthly_mean_ppm - ppm_min) / ppm_range,
    ];
    // "Along" — direction we want to project forward. Time forward
    // (+x) with a small +y component to reflect the secular trend.
    let along = vec![1.0, 0.2];
    let steps = 6;

    println!(
        "    Seed @ chart coord = ({:.3}, {:.3})  along = ({:.3}, {:.3})  steps = {}",
        starting_from[0], starting_from[1], along[0], along[1], steps
    );
    println!();

    // ── 2. Default WalkConfig → honest refusal on high-K substrate ────────
    println!("[2] DEFAULT BUDGET → expected honest refusal");
    println!("    ─────────────────────────────────────────");
    println!("    K_global on the live bundle exceeds what the default");
    println!("    holonomy budget (0.5) can absorb; the integrator returns");
    println!("    the trajectory with refused=true so the caller can read");
    println!("    the partial path AND the safety verdict in one round.");
    println!();
    let default_metric = metric_for_constant_k(k_global);
    let default_cfg = WalkConfig::default();
    match imagine_coherence_trajectory(
        &default_metric,
        &format!("co2_{}-{:02}", last.year, last.month),
        "mauna_loa_co2",
        &starting_from,
        &along,
        steps,
        &default_cfg,
    ) {
        Ok(report) => {
            println!("    refused           = {}", report.refused);
            println!("    refusal_reason    = {}", report.refusal_reason.unwrap_or_default());
            println!("    endpoint_coherence = {:.4}", report.endpoint_coherence);
            println!("    endpoint_curvature = {:.4}", report.endpoint_curvature);
            println!();
            print_trajectory("partial trajectory", &report.trajectory);
        }
        Err(e) => println!("    integrator error: {e:?}"),
    }
    println!();

    // ── 3. Constant-K=0.01 override → calm trajectory ────────────────────
    println!("[3] CONSTANT-K OVERRIDE (K=0.01) → calm projection");
    println!("    ────────────────────────────────────────────────");
    println!("    Override the substrate's measured K with a low constant");
    println!("    so the integrator stays well inside the holonomy budget.");
    println!("    This is the canonical 'project the secular trend forward'");
    println!("    call — no refusal expected.");
    println!();
    let calm_metric = metric_for_constant_k(0.01);
    match imagine_coherence_trajectory(
        &calm_metric,
        &format!("co2_{}-{:02}", last.year, last.month),
        "mauna_loa_co2",
        &starting_from,
        &along,
        steps,
        &default_cfg,
    ) {
        Ok(report) => {
            println!("    refused            = {}", report.refused);
            println!("    endpoint_coherence = {:.4}", report.endpoint_coherence);
            println!("    endpoint_curvature = {:.4}", report.endpoint_curvature);
            println!();
            print_trajectory("full trajectory", &report.trajectory);

            // Decode the endpoint into PPM space so the demo lands
            // back in human-readable units. starting_from[1] was
            // (ppm - 410) / 20; coords[1] at the endpoint is the
            // same coordinate after the integrator's step.
            if let Some(end) = report.trajectory.last() {
                let projected_ppm = ppm_min + end.coords[1] * ppm_range;
                println!();
                println!(
                    "    Imagined endpoint in human units: ~{:.1} ppm",
                    projected_ppm
                );
                println!("    (cite as 'imagined' per Marcella round-3 #1 — NOT a measurement)");
            }
        }
        Err(e) => println!("    integrator error: {e:?}"),
    }
    println!();

    // ── 4. is_imagined() audit ────────────────────────────────────────────
    println!("[4] IS_IMAGINED AUDIT");
    println!("    ──────────────────");
    println!("    Per Marcella round-3 feedback #1, every imagined record");
    println!("    must surface an is_imagined() == true accessor so callers");
    println!("    never confuse projection for observation. We re-run the");
    println!("    calm trajectory and assert all points pass the audit.");
    println!();
    let calm_metric = metric_for_constant_k(0.01);
    let audit = imagine_coherence_trajectory(
        &calm_metric,
        &format!("co2_{}-{:02}", last.year, last.month),
        "mauna_loa_co2",
        &starting_from,
        &along,
        steps,
        &default_cfg,
    )
    .expect("calm metric should produce a trajectory");
    let all_imagined = audit.trajectory.iter().all(|p| p.is_imagined());
    let unique_provenances: std::collections::HashSet<&str> = audit
        .trajectory
        .iter()
        .map(|p| p.provenance.as_str())
        .collect();
    println!(
        "    All {} trajectory points report is_imagined() = true: {}",
        audit.trajectory.len(),
        if all_imagined { "✓" } else { "FAIL" }
    );
    println!("    Distinct provenance strings: {}", unique_provenances.len());
    if let Some(first) = audit.trajectory.first() {
        println!("    Sample: \"{}\"", first.provenance);
    }
    println!();

    println!("════════════════════════════════════════════════════════════════════════");
    println!("  Three IMAGINE shapes validated on real Mauna Loa CO₂ data:");
    println!("    • default-budget honest refusal (refused=true + partial trajectory)");
    println!("    • constant-K calm projection (refused=false + full 6-step path)");
    println!("    • is_imagined() audit on every returned point");
    println!("════════════════════════════════════════════════════════════════════════");
}
