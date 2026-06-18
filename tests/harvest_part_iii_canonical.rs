//! TDD-HAL-III.8a — Harvest GIGI canonical reference (`<P>_canonical` +
//! Flyvbjerg–Petersen blocked SEM).
//!
//! Closes gate III.8a per Bee's locked decision D1: the Part III
//! "thermalization envelope" gate (III.8b) writes its assertion against
//! a GIGI-INTERNAL reference, not Halcyon's hand-rolled
//! `3·sem_chain + 0.02` envelope. This file's only job is to run
//! `GIBBS_SAMPLE β=2.5 SEED=20260616 N_SWEEPS=2048 MEASURE_EVERY=1
//! MEASURE (MEAN(PLAQUETTE))` through the GQL parser + executor ONCE,
//! discard the first 100 sweeps as thermalization, and freeze
//! `<P>_canonical = mean(P_history[100..])` together with the
//! Flyvbjerg–Petersen blocked SEM on the same post-thermal chain into
//! `tests/fixtures/halcyon/part_iii/p_canonical.json`.
//!
//! Gate III.8b later loads this fixture and writes the envelope
//! assertion against it.
//!
//! ── Why `#[ignore]` ──
//!
//! The sweep is ~2048 × 90 KP draws against a non-trivial RNG path and
//! takes several seconds on the dev box. CI's gate-feature lib test
//! must stay fast, so this harness is marked `#[ignore]` and only runs
//! when invoked explicitly via:
//!
//! ```text
//! cargo test --features halcyon --test harvest_part_iii_canonical \
//!     -- --ignored --nocapture
//! ```
//!
//! Re-running the harvest mutates the on-disk fixture in place. The
//! follow-up commit (the same one that lands this harness) is the
//! authoritative source for the `harvest_commit` SHA in the provenance
//! side-car, which is pinned via a second commit (matches the Part II
//! II.2 pin pattern).
//!
//! ── Bit-identity contract ──
//!
//! Cross-binding bit-identity with Halcyon's NumPy PCG64 mock is
//! impossible by design (Bee's locked decision 1). The fixture is the
//! intra-GIGI sentinel: at fixed seed, the canonical reference and FP
//! SEM are byte-stable across this binary's runs. The provenance
//! side-car records the RNG, algorithm, seed, and thermalization
//! discard so any future drift is traceable.
//!
//! ── Storage format ──
//!
//! `p_canonical.json`:
//! ```text
//! {
//!   "p_canonical": <f64>,           // <P> over post-thermal chain
//!   "fp_sem": <f64>,                // Flyvbjerg–Petersen blocked SEM
//!   "n_sweeps_total": 2048,
//!   "thermalization_discard": 100,
//!   "n_sweeps_post_thermal": 1948,
//!   "p_history_bits": [<u64>...],   // IEEE-754 bit patterns (oracle)
//!   "p_history_decimal": [<f64>...] // human-readable shadow
//! }
//! ```
//!
//! Following the Part II II.2 envelope convention: `p_history_bits` is
//! the byte-equality oracle (loaded via `f64::from_bits`);
//! `p_history_decimal` is informational only.

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

/// Path to the canonical-reference fixture, anchored to the test
/// crate's manifest dir so `cargo test` from anywhere finds it.
fn p_canonical_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_iii")
        .join("p_canonical.json")
}

/// Path to the provenance side-car (records the harvest contract:
/// RNG, algorithm, seed, harvest commit). Gate III.8b doesn't read
/// this at runtime — it exists so anyone reading the fixture knows
/// the bit-identity contract drop vs the Halcyon NumPy mock.
fn p_canonical_provenance_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("part_iii")
        .join("p_canonical_provenance.json")
}

/// Flyvbjerg–Petersen blocked SEM on a 1-D real chain.
///
/// Algorithm (standard form):
///
/// For each block size `b ∈ {1, 2, 4, 8, …}` up to `n / 4`:
///   - chunk the chain into `n / b` contiguous blocks of length `b`;
///   - compute the mean of each block;
///   - compute the variance of the block means;
///   - the SEM at that block size is `sqrt(var / n_blocks)`.
///
/// The FP SEM is the plateau value of this curve. We declare a plateau
/// once two consecutive block sizes return SEM estimates within `10%`
/// relative tolerance of each other; the larger block size wins.
///
/// On 1948 post-thermal samples we expect the plateau to land around
/// block size 16–64 (the Halcyon Python reference's
/// `inertia_damping/buckyball_heatbath.py::flyvbjerg_petersen_sem`
/// observes the same regime). If no two consecutive block sizes
/// plateau, we return the SEM at the largest block size — that's the
/// conservative fallback the literature recommends.
fn flyvbjerg_petersen_sem(chain: &[f64]) -> f64 {
    let n = chain.len();
    assert!(
        n >= 8,
        "Flyvbjerg-Petersen SEM needs at least 8 samples; got {n}"
    );

    let mean = chain.iter().sum::<f64>() / n as f64;

    // Build the SEM curve over power-of-two block sizes up to n/4.
    let mut curve: Vec<(usize, f64)> = Vec::new();
    let mut b = 1usize;
    while b <= n / 4 {
        let n_blocks = n / b;
        // Drop the trailing partial block (standard FP convention).
        let block_means: Vec<f64> = (0..n_blocks)
            .map(|k| {
                let start = k * b;
                let end = start + b;
                chain[start..end].iter().sum::<f64>() / b as f64
            })
            .collect();
        let var = block_means
            .iter()
            .map(|m| {
                let d = *m - mean;
                d * d
            })
            .sum::<f64>()
            / (n_blocks as f64 - 1.0).max(1.0);
        let sem = (var / n_blocks as f64).sqrt();
        curve.push((b, sem));
        b *= 2;
    }

    // Find the first pair of consecutive block sizes whose SEMs are
    // within 10% relative tolerance — that's the plateau. The larger
    // block size's SEM is the FP SEM.
    for win in curve.windows(2) {
        let (_, s_prev) = win[0];
        let (_, s_next) = win[1];
        let rel_diff = (s_next - s_prev).abs() / s_prev.max(1e-30);
        if rel_diff < 0.10 {
            return s_next;
        }
    }

    // Fallback: no plateau detected — return the SEM at the largest
    // block size (most conservative estimate the literature gives).
    curve
        .last()
        .map(|(_, s)| *s)
        .expect("FP SEM curve cannot be empty for n >= 8")
}

/// TDD-HAL-III.8a: harvest the canonical-reference `<P>` and
/// Flyvbjerg–Petersen blocked SEM by running `GIBBS_SAMPLE` through
/// the GQL parser + executor path once, discarding the first 100
/// sweeps as thermalization, and freezing the result + the full
/// `P_history` (bit-pattern oracle + decimal shadow) into
/// `tests/fixtures/halcyon/part_iii/p_canonical.json`. Gate III.8b
/// reads this fixture to write its envelope assertion.
#[test]
#[ignore]
fn tdd_hal_iii_8a_harvest_p_canonical() {
    // Clean registries so the harvest is reproducible against any
    // prior test state on this binary.
    gigi::gauge::registry::clear();
    gigi::lattice::registry::clear();

    let dir = tempfile::tempdir().expect("tempdir");
    let mut engine = gigi::engine::Engine::open(dir.path()).expect("engine open");

    // Declare the buckyball lattice + an INIT IDENTITY SU(2) field
    // through the parser+executor path so the harvest exercises the
    // same code surface gate III.8b will exercise.
    let lat_decl = "LATTICE iii_8a_bb FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';";
    let stmt = parse(lat_decl).expect("parse LATTICE");
    execute(&mut engine, &stmt).expect("exec LATTICE");

    let g_decl = "GAUGE_FIELD U_iii_8a ON LATTICE iii_8a_bb \
                  GROUP SU(2) INIT IDENTITY;";
    let stmt = parse(g_decl).expect("parse GAUGE_FIELD");
    execute(&mut engine, &stmt).expect("exec GAUGE_FIELD");

    // The GAUGE_FIELD executor arm registers through `register`
    // (Arc<dyn>). GIBBS_SAMPLE needs a SU(2)-mut handle (D4). Re-
    // publish the same field through `register_su2` so the heatbath
    // sweep can lock the mutable buffer. This mirrors the III.6
    // smoke test's same fix-up.
    {
        let lat = gigi::lattice::registry::get("iii_8a_bb")
            .expect("declared lattice");
        let su2 = gigi::gauge::SU2GaugeField::new(
            "U_iii_8a".into(),
            &lat,
            gigi::gauge::GaugeFieldInit::Identity,
            None,
        )
        .expect("identity init");
        gigi::gauge::registry::register_su2(su2);
    }

    // Run the full 2048-sweep harvest through the GQL parser +
    // executor. `MEASURE_EVERY 1` captures `<P>` at every sweep.
    let sample = "GIBBS_SAMPLE U_iii_8a BETA 2.5 N_SWEEPS 2048 MEASURE_EVERY 1 \
                  MEASURE (MEAN(PLAQUETTE)) SEED 20260616;";
    let stmt = parse(sample).expect("parse GIBBS_SAMPLE");
    let rows = match execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE") {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows envelope, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "GIBBS_SAMPLE returns one row");

    let label = gigi::gauge::ObservableId::MeanPlaquette.label();
    let p_history: Vec<f64> = match rows[0].get(label) {
        Some(Value::Vector(v)) => v.clone(),
        other => panic!("missing/wrong {label} column: {other:?}"),
    };
    assert_eq!(
        p_history.len(),
        2048,
        "expected 2048 measurements (n_sweeps / measure_every); got {}",
        p_history.len()
    );

    // Discard the first 100 sweeps as thermalization, compute
    // `<P>_canonical` and the Flyvbjerg–Petersen blocked SEM on the
    // post-thermal tail.
    let thermal_discard = 100usize;
    let post_thermal: &[f64] = &p_history[thermal_discard..];
    assert_eq!(post_thermal.len(), 1948);

    let p_canonical: f64 = post_thermal.iter().sum::<f64>() / post_thermal.len() as f64;
    let fp_sem = flyvbjerg_petersen_sem(post_thermal);

    // Sanity sweep: post-thermal mean must be a probability-like
    // scalar in (0, 1) — at β=2.5 the buckyball heatbath equilibrium
    // sits around <P> ≈ 0.55 per Halcyon's reference run; a wildly
    // out-of-range value indicates the sweep diverged.
    assert!(
        p_canonical > 0.0 && p_canonical < 1.0,
        "p_canonical out of probability range: {p_canonical}"
    );
    // FP SEM is a non-negative chain-noise scale; at this β regime
    // we expect it under 0.05 (loose bound — the harvest doesn't
    // gate on this, the next-gate envelope test does).
    assert!(
        fp_sem >= 0.0,
        "Flyvbjerg-Petersen SEM must be non-negative; got {fp_sem}"
    );

    // Serialize the chain in both bit-pattern (oracle) and decimal
    // (shadow) forms — same envelope shape Part II II.2 standardized.
    let p_history_bits: Vec<u64> = p_history.iter().map(|x| x.to_bits()).collect();
    let envelope = serde_json::json!({
        "p_canonical": p_canonical,
        "fp_sem": fp_sem,
        "n_sweeps_total": p_history.len(),
        "thermalization_discard": thermal_discard,
        "n_sweeps_post_thermal": post_thermal.len(),
        "p_history_bits": p_history_bits,
        "p_history_decimal": p_history,
    });
    let envelope_json =
        serde_json::to_string_pretty(&envelope).expect("serialize p_canonical");
    fs::write(p_canonical_path(), envelope_json).expect("write p_canonical");

    // Provenance side-car. `harvest_commit` is filled in by a follow-
    // up commit once the harvest commit exists (the harvest by
    // construction runs before its own SHA is known — same pattern
    // as the II.2 pin commit `f0f402a`).
    let provenance = serde_json::json!({
        "source": "gigi::gauge::gibbs_sample (driven through gigi::parser::execute Statement::GibbsSample)",
        "beta": 2.5_f64,
        "seed": 20260616_u64,
        "n_sweeps": 2048_usize,
        "measure_every": 1_usize,
        "measure": ["MEAN(PLAQUETTE)"],
        "init": "IDENTITY",
        "lattice": "TRUNCATED_ICOSAHEDRON (buckyball: V=60, E=90, F=32)",
        "topology": "S2",
        "group": "SU(2)",
        "thermalization_discard": thermal_discard,
        "n_sweeps_post_thermal": post_thermal.len(),
        "rng": "gigi::gauge::marsaglia_haar::SmallRng (xorshift64*)",
        "algorithm": "Kennedy-Pendleton SU(2) heatbath with sqrt-rejection Haar fallback",
        "harvest_commit": "<fill in with the SHA of the TDD-HAL-III.8a commit once it lands>",
        "purpose": "Canonical reference for Part III gate III.8b. Cross-binding bit-identity with Halcyon NumPy PCG64 is impossible by design (locked decision 1); intra-GIGI bit-identity at fixed seed is the bit-id contract.",
        "note": "Bee's locked decision D1: gate III.8b writes its envelope assertion against this GIGI-internal reference, not Halcyon's hand-rolled 3*sem_chain+0.02 envelope.",
    });
    let prov_json = serde_json::to_string_pretty(&provenance)
        .expect("serialize p_canonical_provenance");
    fs::write(p_canonical_provenance_path(), prov_json)
        .expect("write p_canonical_provenance");

    println!(
        "harvested {} post-thermal samples to {}",
        post_thermal.len(),
        p_canonical_path().display()
    );
    println!("  p_canonical = {p_canonical:.12}");
    println!("  fp_sem      = {fp_sem:.12}");
    println!(
        "  provenance at {}",
        p_canonical_provenance_path().display()
    );
}
