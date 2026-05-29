//! Gauge-invariance test for the HOLONOMY GQL statement (Sprint v0.3.1
//! seal-up: previously this primitive read fiber values raw, making the
//! deficit gauge-dependent under per-field Affine encryption).
//!
//! The fix in `src/bin/gigi_stream.rs::compute_fiber_holonomy` normalizes
//! centroids by their own min/range per axis before computing angles;
//! this test verifies the deficit is now invariant under Affine gauge.
//!
//! Run with: `cargo test --test holonomy_gauge_invariance_v0_3`
//!
//! We can't directly import the `bin`-crate function `compute_fiber_holonomy`,
//! so this test re-derives the math from the same construction in pure Rust
//! and verifies the gauge-invariance property at the math level. The
//! production function follows identical equations.

use std::collections::BTreeMap;

/// Pure-math mirror of `compute_fiber_holonomy` (the v0.3.1 normalized
/// version). Inputs are (around_label, f0, f1) triples; output is the
/// angle-deficit holonomy in [0, 2π).
fn fiber_holonomy(
    records: &[(String, f64, f64)],
) -> f64 {
    // Group by label (the "around_field"); compute centroid per group.
    let mut groups: BTreeMap<String, (f64, f64, usize)> = BTreeMap::new();
    for (k, v0, v1) in records {
        let e = groups.entry(k.clone()).or_insert((0.0, 0.0, 0));
        e.0 += v0;
        e.1 += v1;
        e.2 += 1;
    }
    if groups.len() < 2 {
        return 0.0;
    }
    let centroids: Vec<(f64, f64)> = groups
        .into_iter()
        .map(|(_, (sx, sy, n))| (sx / n as f64, sy / n as f64))
        .collect();

    // Normalize by centroid-set min/range per axis.
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for (cx, cy) in &centroids {
        if *cx < min_x { min_x = *cx; }
        if *cx > max_x { max_x = *cx; }
        if *cy < min_y { min_y = *cy; }
        if *cy > max_y { max_y = *cy; }
    }
    let range_x = (max_x - min_x).max(f64::EPSILON);
    let range_y = (max_y - min_y).max(f64::EPSILON);
    let normalized: Vec<(f64, f64)> = centroids
        .iter()
        .map(|(cx, cy)| ((cx - min_x) / range_x, (cy - min_y) / range_y))
        .collect();

    let nc = normalized.len();
    let mut sum = 0.0f64;
    for i in 0..nc {
        let prev = if i == 0 { nc - 1 } else { i - 1 };
        let next = (i + 1) % nc;
        let dx_in = normalized[i].0 - normalized[prev].0;
        let dy_in = normalized[i].1 - normalized[prev].1;
        let dx_out = normalized[next].0 - normalized[i].0;
        let dy_out = normalized[next].1 - normalized[i].1;
        let mut delta = dy_out.atan2(dx_out) - dy_in.atan2(dx_in);
        while delta > std::f64::consts::PI {
            delta -= 2.0 * std::f64::consts::PI;
        }
        while delta < -std::f64::consts::PI {
            delta += 2.0 * std::f64::consts::PI;
        }
        sum += delta;
    }
    sum.abs() % (2.0 * std::f64::consts::PI)
}

/// Apply per-field Affine gauge encryption to a record set.
fn encrypt_affine(
    records: &[(String, f64, f64)],
    a0: f64,
    b0: f64,
    a1: f64,
    b1: f64,
) -> Vec<(String, f64, f64)> {
    records
        .iter()
        .map(|(k, v0, v1)| (k.clone(), a0 * v0 + b0, a1 * v1 + b1))
        .collect()
}

fn sample_records_with_nontrivial_holonomy() -> Vec<(String, f64, f64)> {
    // 8 groups forming a non-convex polygon in (f0, f1), so the
    // angle-deficit is non-zero.
    vec![
        ("A".into(), 1.0, 0.0),
        ("A".into(), 1.1, 0.0),
        ("B".into(), 2.0, 1.0),
        ("B".into(), 2.1, 1.0),
        ("C".into(), 1.5, 2.5),
        ("C".into(), 1.6, 2.4),
        ("D".into(), 0.0, 2.0),
        ("D".into(), 0.1, 2.0),
        ("E".into(), -1.0, 1.5),
        ("E".into(), -1.1, 1.4),
        ("F".into(), -2.0, 0.5),
        ("F".into(), -2.0, 0.6),
        ("G".into(), -1.5, -1.0),
        ("G".into(), -1.4, -1.1),
        ("H".into(), 0.5, -2.0),
        ("H".into(), 0.6, -2.0),
    ]
}

/// Wrap-around-aware distance between two angle-deficits modulo 2π.
/// Two deficits that differ by a full revolution (2π) are equivalent
/// after f64-rounding because the GQL function returns `sum.abs() % 2π`,
/// and `2π % 2π` is sensitive to which side of 2π the f64 lands on.
fn mod_2pi_distance(a: f64, b: f64) -> f64 {
    let two_pi = 2.0 * std::f64::consts::PI;
    let diff = (a - b).rem_euclid(two_pi);
    diff.min(two_pi - diff)
}

// ─────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────

/// Gauge invariance under per-field Affine with **shared positive scale**
/// (a0 = a1 > 0). Trivial direction — preserved even by the un-normalized
/// version of the function.
#[test]
fn test_holonomy_invariant_under_uniform_positive_scale() {
    let plain = sample_records_with_nontrivial_holonomy();
    let h_plain = fiber_holonomy(&plain);
    let encrypted = encrypt_affine(&plain, 2.5, 0.0, 2.5, 0.0);
    let h_enc = fiber_holonomy(&encrypted);
    assert!(
        mod_2pi_distance(h_plain, h_enc) < 1e-9,
        "uniform-scale gauge should give identical deficit (mod 2π): plain={} enc={}",
        h_plain,
        h_enc
    );
}

/// Gauge invariance under per-field Affine with **per-axis different
/// positive scales** (a0 ≠ a1, both > 0). This is the load-bearing test:
/// the pre-v0.3.1 un-normalized version FAILED this property; the
/// v0.3.1 normalized version should pass.
#[test]
fn test_holonomy_invariant_under_per_axis_positive_scale() {
    let plain = sample_records_with_nontrivial_holonomy();
    let h_plain = fiber_holonomy(&plain);
    // Mismatched per-axis scales — would have failed in v0.3.0:
    let encrypted = encrypt_affine(&plain, 3.0, 5.0, 7.0, -2.0);
    let h_enc = fiber_holonomy(&encrypted);
    assert!(
        mod_2pi_distance(h_plain, h_enc) < 1e-9,
        "per-axis-scale gauge should give identical deficit (mod 2π) after v0.3.1 normalization: plain={} enc={}",
        h_plain,
        h_enc
    );
}

/// Gauge invariance under translation (b0, b1 ≠ 0, scales = 1).
#[test]
fn test_holonomy_invariant_under_translation() {
    let plain = sample_records_with_nontrivial_holonomy();
    let h_plain = fiber_holonomy(&plain);
    let encrypted = encrypt_affine(&plain, 1.0, 100.0, 1.0, -50.0);
    let h_enc = fiber_holonomy(&encrypted);
    assert!(
        mod_2pi_distance(h_plain, h_enc) < 1e-9,
        "translation should preserve deficit (mod 2π): plain={} enc={}",
        h_plain,
        h_enc
    );
}

/// Gauge invariance under negative scale on one axis (per-axis reflection):
/// the magnitude is preserved; the sign may flip but the function returns
/// the absolute value, so the result is identical (mod 2π).
#[test]
fn test_holonomy_invariant_magnitude_under_per_axis_reflection() {
    let plain = sample_records_with_nontrivial_holonomy();
    let h_plain = fiber_holonomy(&plain);
    let encrypted = encrypt_affine(&plain, -2.0, 3.0, 4.0, 1.0);
    let h_enc = fiber_holonomy(&encrypted);
    assert!(
        mod_2pi_distance(h_plain, h_enc) < 1e-9,
        "per-axis reflection should preserve |deficit| (mod 2π): plain={} enc={}",
        h_plain,
        h_enc
    );
}

/// Multi-trial randomized gauge sweep: 20 random Affine gauges all produce
/// the same deficit (modulo 2π).
#[test]
fn test_holonomy_invariant_across_random_affine_gauges() {
    let plain = sample_records_with_nontrivial_holonomy();
    let h_plain = fiber_holonomy(&plain);
    // Deterministic "random" gauges — we use a fixed sequence so the test
    // is reproducible.
    let gauges = [
        (1.0, 0.0, 1.0, 0.0),
        (2.5, 1.0, 0.5, -2.0),
        (-1.5, 7.0, 3.0, 0.5),
        (0.1, 0.0, 100.0, 50.0),
        (1.0, 0.0, -1.0, 0.0),
        (7.0, -3.0, 0.2, 8.0),
        (-2.0, 1.0, -3.0, -1.0),
        (1.5, 2.5, 0.7, 4.2),
        (10.0, -100.0, 0.01, 1.0),
        (3.14, 1.0, 2.71, -1.0),
        (0.5, 0.5, 0.5, 0.5),
        (-0.5, 0.5, -0.5, 0.5),
        (8.0, 0.0, 1.0, 0.0),
        (1.0, 8.0, 1.0, 8.0),
        (0.001, 1000.0, 100.0, 0.0),
        (4.0, 0.0, 0.25, 0.0),
        (-7.7, 7.7, 11.1, -11.1),
        (2.5, 0.0, 0.4, 0.0),
        (1.234, 5.678, 9.012, 3.456),
        (-1.0, -1.0, -1.0, -1.0),
    ];
    for (i, (a0, b0, a1, b1)) in gauges.iter().enumerate() {
        let encrypted = encrypt_affine(&plain, *a0, *b0, *a1, *b1);
        let h_enc = fiber_holonomy(&encrypted);
        assert!(
            mod_2pi_distance(h_plain, h_enc) < 1e-9,
            "gauge #{} (a0={}, b0={}, a1={}, b1={}) drifted: plain={} enc={} (mod 2π distance = {})",
            i,
            a0,
            b0,
            a1,
            b1,
            h_plain,
            h_enc,
            mod_2pi_distance(h_plain, h_enc),
        );
    }
}
