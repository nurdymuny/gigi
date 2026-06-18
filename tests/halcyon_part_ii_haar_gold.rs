//! TDD-HAL-II.2 — Marsaglia Haar buffer regression-sentinel gold gate.
//!
//! Loads `tests/fixtures/halcyon/buckyball_haar_random_seed_20260616_gold.json`
//! and asserts that `DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616)`
//! reproduces it byte-for-byte.
//!
//! Storage format: the JSON envelope is
//! `{"n_edges": 90, "repr_dim": 4, "group": "SU(2)",
//!   "data_bits": [[u64; 4]; 90],
//!   "data_decimal": [[f64; 4]; 90]}`.
//!
//! `data_bits` is the IEEE-754 bit pattern of every f64 (load via
//! `f64::from_bits`) — that's the byte-equality oracle. `data_decimal`
//! is the human-readable shadow (ryu shortest, round-trip safe in
//! practice but the bit field is the canonical truth). Storing the
//! bits sidesteps the ULP-level drift that decimal round-tripping can
//! introduce on edge-case f64 values.
//!
//! Per Bee's locked decision 1 the gold is harvested from GIGI's own
//! output (intra-binding bit-identity sentinel), NOT from NumPy
//! PCG64 — see the sibling provenance file
//! `buckyball_haar_random_seed_20260616_gold_provenance.json` for
//! the full record of which RNG + algorithm + draw order produced
//! these bytes. The test flags if any of those three pieces silently
//! drift (e.g. switching xorshift64* → another PRNG, or swapping the
//! per-edge draw order).
//!
//! Harvest helper: `harvest_haar_gold` (gated `#[ignore]`) builds
//! the buffer and writes the JSON + provenance side-by-side. To
//! re-harvest:
//!
//! ```text
//! cargo test --features halcyon --test halcyon_part_ii_haar_gold \
//!     harvest_haar_gold -- --ignored --nocapture
//! ```
//!
//! Then re-run the gold gate to confirm the new fixture round-trips.

#![cfg(feature = "halcyon")]

use std::fs;
use std::path::PathBuf;

use gigi::gauge::{DenseLinkBuffer, Group};

/// Path to the regression-sentinel fixture, anchored to the test
/// crate's manifest dir so `cargo test` from anywhere finds it.
fn gold_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("buckyball_haar_random_seed_20260616_gold.json")
}

/// Path to the provenance side-car (records the harvest contract:
/// which RNG, which algorithm, which seed, which draw order). The
/// gate doesn't read this file at runtime — it exists so anyone
/// reading the gold knows the mock-vs-live byte equality contract
/// with NumPy PCG64 was dropped at the CSPRNG decision.
fn provenance_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("halcyon")
        .join("buckyball_haar_random_seed_20260616_gold_provenance.json")
}

/// Load a 90×4 f64 array from the gold JSON via the `data_bits`
/// field (IEEE-754 bit patterns → `f64::from_bits`). The
/// `data_decimal` field is informational; we never read it.
fn load_quat_array(path: &PathBuf) -> Vec<[f64; 4]> {
    let body = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&body)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    let bits = v["data_bits"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: missing `data_bits` array", path.display()));
    bits.iter()
        .map(|row| {
            let r = row.as_array().expect("row not an array");
            [
                f64::from_bits(r[0].as_u64().expect("q0 bits not u64")),
                f64::from_bits(r[1].as_u64().expect("q1 bits not u64")),
                f64::from_bits(r[2].as_u64().expect("q2 bits not u64")),
                f64::from_bits(r[3].as_u64().expect("q3 bits not u64")),
            ]
        })
        .collect()
}

/// TDD-HAL-II.2 — `DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616)`
/// reproduces the regression-sentinel gold quaternion array
/// byte-for-byte. Strict f64 equality (no tolerance) because the
/// fixture was harvested from this exact code path and any drift
/// in the RNG, the Marsaglia algorithm, or the per-edge draw order
/// must trip this gate.
#[test]
fn tdd_hal_ii_2_haar_buckyball_gigi_gold() {
    let gold = load_quat_array(&gold_path());
    assert_eq!(gold.len(), 90, "gold must have 90 edges");

    let buffer = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616)
        .expect("Haar SU(2) buffer must succeed");
    assert_eq!(buffer.group, Group::SU2);
    assert_eq!(buffer.n_edges, 90);
    assert_eq!(buffer.repr_dim, 4);
    assert_eq!(buffer.data.len(), 360);

    // Strict byte equality — flatten the gold and compare to
    // `buffer.data` row-major.
    let mut flat = Vec::with_capacity(360);
    for q in &gold {
        flat.push(q[0]);
        flat.push(q[1]);
        flat.push(q[2]);
        flat.push(q[3]);
    }
    assert_eq!(
        buffer.data, flat,
        "DenseLinkBuffer::new_haar drift vs gold (RNG / algorithm / draw order changed?)"
    );
}

/// Structural smoke: gold rows are unit-norm to f64 rounding. Guards
/// against fixture corruption breaking the gate silently with a
/// "different bytes but wrong distribution" failure mode.
#[test]
fn tdd_hal_ii_2_haar_gold_rows_unit_norm() {
    let gold = load_quat_array(&gold_path());
    for (i, q) in gold.iter().enumerate() {
        let n2 = q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3];
        assert!(
            (n2 - 1.0).abs() < 1e-12,
            "gold[{i}] not unit-norm: |q|^2 = {n2}"
        );
    }
}

/// Harvest helper — `#[ignore]`d so normal CI does not run it.
///
/// Rebuilds the regression-sentinel gold + provenance side-car from
/// the current `DenseLinkBuffer::new_haar` output. Run via:
///
/// ```text
/// cargo test --features halcyon --test halcyon_part_ii_haar_gold \
///     harvest_haar_gold -- --ignored --nocapture
/// ```
///
/// The provenance file documents that the gold is intra-GIGI (NOT
/// NumPy PCG64): if the RNG, algorithm, or draw order changes, the
/// gold becomes stale and this harvest must be re-run with the new
/// commit SHA pinned in the provenance.
#[test]
#[ignore]
fn harvest_haar_gold() {
    let buffer = DenseLinkBuffer::new_haar(Group::SU2, 90, 20260616)
        .expect("Haar SU(2) buffer must succeed");
    assert_eq!(buffer.data.len(), 360);

    // Emit row-major (90 × 4) in both bit and decimal forms.
    //
    // `data_bits` is the byte-equality oracle: IEEE-754 bit pattern
    // of every f64 stored as u64 → loaded via `f64::from_bits`,
    // guaranteed to round-trip exactly across JSON.
    //
    // `data_decimal` is the human-readable shadow (ryu shortest);
    // useful for diffing a fixture by eye but never read at gate
    // time.
    let bits: Vec<[u64; 4]> = (0..buffer.n_edges)
        .map(|e| {
            let b = 4 * e;
            [
                buffer.data[b].to_bits(),
                buffer.data[b + 1].to_bits(),
                buffer.data[b + 2].to_bits(),
                buffer.data[b + 3].to_bits(),
            ]
        })
        .collect();
    let decimal: Vec<[f64; 4]> = (0..buffer.n_edges)
        .map(|e| {
            let b = 4 * e;
            [
                buffer.data[b],
                buffer.data[b + 1],
                buffer.data[b + 2],
                buffer.data[b + 3],
            ]
        })
        .collect();
    let envelope = serde_json::json!({
        "n_edges": buffer.n_edges,
        "repr_dim": buffer.repr_dim,
        "group": "SU(2)",
        "data_bits": bits,
        "data_decimal": decimal,
    });
    let gold_json = serde_json::to_string_pretty(&envelope)
        .expect("serialize gold envelope");
    fs::write(gold_path(), gold_json).expect("write gold");

    // Provenance side-car. `harvest_commit` is the commit SHA of the
    // II.2 commit that lands this fixture — fill it in via a small
    // post-harvest edit once the commit exists, per the spec.
    let provenance = serde_json::json!({
        "source": "gigi::gauge::dense_link_buffer::DenseLinkBuffer::new_haar",
        "seed": 20260616,
        "rng": "gigi::gauge::marsaglia_haar::SmallRng (xorshift64*, mirrored from gigi::geometry::generative_flow::SmallRng)",
        "algorithm": "Marsaglia 4-uniforms-with-rejection",
        "n_edges": 90,
        "repr_dim": 4,
        "group": "SU(2)",
        "harvest_commit": "<fill in with the SHA of the TDD-HAL-II.2 commit once it lands>",
        "purpose": "GIGI-internal regression sentinel - flags if the RNG, the Marsaglia algorithm, or the per-edge draw order changes",
        "note": "Bee's locked decision 1: NumPy PCG64 mock-vs-live byte equality was dropped at the CSPRNG decision. This gold is intra-GIGI only."
    });
    let prov_json = serde_json::to_string_pretty(&provenance)
        .expect("serialize provenance");
    fs::write(provenance_path(), prov_json).expect("write provenance");

    println!(
        "harvested {} rows to {}",
        bits.len(),
        gold_path().display()
    );
    println!("provenance at {}", provenance_path().display());
}
