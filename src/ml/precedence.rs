//! PRECEDENCE — which of two ordered signals moves first?
//!
//! Returns the normalised signed area enclosed by the 2-D path `(x, y)`:
//!
//! ```text
//! A = 1/2 * sum_i ( x_i * y_{i+1} - y_i * x_{i+1} ) (about the start)
//! normalised by sqrt( QV_x * QV_y )
//! ```
//!
//! This is the Levy area — the level-2 antisymmetric part of the path
//! signature (Chen 1957; Lyons rough-path theory) — and it is circulation
//! exactly. If `x` consistently moves before `y`, the joint path traces a
//! loop with a definite orientation, and the sign of that loop is the answer.
//!
//! WHAT MAKES IT WORTH HAVING. Every conventional lead-lag estimate resamples
//! both series onto a common grid whose bin width the caller picked. That
//! choice changes the answer: measured on live tape, 3 of 6 instrument pairs
//! reported a DIFFERENT lead-lag under a time warp that preserved event order,
//! and one pair reversed direction entirely. PRECEDENCE never reads a
//! timestamp — only the order of the records — so there is no bin width to get
//! wrong.
//!
//! NOT finance-specific. `x` and `y` are any two numeric fields: requests and
//! latency, temperature and pressure, dose and response.
//!
//! NORMALISATION IS NOT COSMETIC. The raw area scales as (size of x) x (size
//! of y). On small-magnitude fields it lands near 1e-6, and comparing two such
//! numbers is comparing noise. Dividing by sqrt(QV_x * QV_y) carries exactly
//! the same bilinear scaling, so the result is dimensionless and comparable
//! across pairs and across units.
//!
//! SIGN CONVENTION, MEASURED NOT ASSUMED:
//!   A > 0  =>  x leads y   (x moves first, y follows)
//!   A < 0  =>  y leads x
//!   A = 0  =>  simultaneous (exact at zero lag)
//! This sign has now been got wrong TWICE by assertion — once in the Python
//! validation and again in this port, in both cases by writing down what
//! "felt" right instead of running the planted-lag fixture first. The
//! estimator was correct both times. TXP-4 pins the convention against a
//! fixture with a known lead, and swapping the roles must negate it exactly.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Hard cap — TXP-15. Refuse rather than churn or return null.
pub const MAX_PRECEDENCE_N: usize = 2_000_000;
/// Below this a signed area is not meaningfully estimable.
pub const MIN_PRECEDENCE_N: usize = 32;
/// |A| below this reads as "neither leads" rather than a spurious direction.
/// A float-equality guard only — it fires on identical or exactly-zero series.
/// It is NOT a significance band; `p_value` below is the significance instrument.
pub const PRECEDENCE_DEADBAND: f64 = 1e-6;

/// Rotations used to build the null distribution of `area`.
///
/// 199 gives clean 5% and 1% quantiles for a permutation test (the p-value is
/// `(1 + #{|a_k| >= |a_obs|}) / (1 + K)`, so K+1 = 200 divides evenly).
pub const PRECEDENCE_NULL_ROTATIONS: usize = 199;

/// Cap on the sample the null is built from.
///
/// The null is computed on a CONTIGUOUS block so the increments keep their own
/// autocorrelation — which is the whole point, see [`precedence`]. Measured, the
/// null sd is n-independent for every increment structure except the very
/// roughest, where it narrows with n (sd 0.0119 at n=1024 down to 0.0034 at
/// n=65536 for MA(1) phi = -0.9). Capping therefore errs WIDE on rough data:
/// conservative, costing power rather than manufacturing false leads. Disclosed
/// in `notes` whenever it bites.
pub const PRECEDENCE_NULL_MAX_N: usize = 65_536;

/// Request for `POST /v1/bundles/{name}/precedence`.
// `deny_unknown_fields`: a misspelled key used to be IGNORED, so the verb
// silently fell back to the default and returned a different number with
// HTTP 200. Measured: `maxLag` gave exponent 0.505802 where `max_lag` gave
// 0.495073. A typo must be an error, not a quiet change of answer.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrecedenceRequest {
    /// First numeric field. Base or fiber.
    pub x: String,
    /// Second numeric field.
    pub y: String,
    /// Field to order the records by. Omitted → record order.
    #[serde(default)]
    pub order: Option<String>,
}

#[derive(Debug)]
pub struct PrecedenceResult {
    pub x: String,
    pub y: String,
    pub n: usize,
    pub area: f64,
    pub leads: String,
    pub magnitude: f64,
    /// Permutation p-value against a rotation null — the probability of seeing
    /// an area at least this large with NO lead relationship, given these two
    /// series' own increment structure.
    pub p_value: f64,
    /// Spread of that null. This is what makes the reading interpretable: it
    /// swings by a factor of ~160 with how rough the data is.
    pub null_sd: f64,
    /// `p_value <= 0.05`. False means the direction below is not distinguishable
    /// from no relationship *in this window*.
    pub significant: bool,
    /// Records the null was built from (capped at `PRECEDENCE_NULL_MAX_N`).
    pub null_n: usize,
    pub order_field: Option<String>,
    pub reads: Vec<String>,
    pub notes: Vec<String>,
}

/// Normalised signed area from two increment series, about the path start.
fn area_from_increments(dx: &[f64], dy: &[f64]) -> Option<f64> {
    let n = dx.len();
    if n == 0 || dy.len() != n { return None; }
    let (mut xi, mut yi) = (0.0f64, 0.0f64);
    let (mut acc, mut qx, mut qy) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let (xn, yn) = (xi + dx[i], yi + dy[i]);
        acc += xi * yn - yi * xn;
        qx += dx[i] * dx[i];
        qy += dy[i] * dy[i];
        xi = xn;
        yi = yn;
    }
    let qv = (qx * qy).sqrt();
    if !(qv > 0.0) || !qv.is_finite() { return None; }
    let a = 0.5 * acc / qv;
    if a.is_finite() { Some(a) } else { None }
}

/// The null distribution of `area` for these two series, by circular rotation.
///
/// Returns `(p_value, null_sd)`.
///
/// **Why a rotation surrogate and not a formula.** For INDEPENDENT paths with
/// iid increments the normalised area is Levy's stochastic area, whose
/// characteristic function `sech(l/2)` inverts to density `sech(pi*a)` — so
/// `sd = 1/2` exactly, at every `n`. Verified: 0.494 / 0.510 / 0.488 at
/// n = 512 / 4096 / 32768.
///
/// That constant is useless on real data. With MA(1) increments the true null
/// sd runs 0.006 (rough) → 0.458 (walk) → 1.046 (smooth) — a factor of 160,
/// tracking exactly what TEXTURE measures. Shipping `1/2` would be ~80x too
/// WIDE on rough tape (never detects a real lead) and ~2x too NARROW on smooth
/// tape (calls noise a lead). Real books are never a pure random walk.
///
/// Rotating ONE channel's increments destroys any lead relationship while
/// preserving that channel's own autocorrelation exactly, so the null adapts to
/// the data instead of assuming its shape. Increments are rotated rather than
/// levels: rotating levels would splice in an artificial jump at the wrap.
/// Measured false-positive rate at the 5% level, independent pairs:
/// 4.3% / 4.0% / 3.3% / 6.3% / 6.7% across MA(1) phi = -0.9 .. +0.9 — nominal
/// everywhere the closed form is wrong.
///
/// Deterministic: the offsets come from a fixed-seed xorshift, so the same
/// bundle and parameters return the same p-value every time (TXP-14).
fn rotation_null(dx: &[f64], dy: &[f64], observed: f64) -> (f64, f64, usize) {
    let n = dx.len().min(PRECEDENCE_NULL_MAX_N);
    if n < 8 { return (1.0, f64::NAN, n); }
    let (dxs, dys) = (&dx[..n], &dy[..n]);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15 ^ (n as u64);
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    let mut rotated = vec![0.0f64; n];
    let mut at_least = 1usize; // the observed value counts itself
    let mut draws: Vec<f64> = Vec::with_capacity(PRECEDENCE_NULL_ROTATIONS);
    for _ in 0..PRECEDENCE_NULL_ROTATIONS {
        // Offset in 1..n-1: 0 would reproduce the observed pairing exactly.
        let k = 1 + (next() as usize) % (n - 1);
        for i in 0..n {
            rotated[i] = dys[(i + k) % n];
        }
        if let Some(a) = area_from_increments(dxs, &rotated) {
            if a.abs() >= observed.abs() { at_least += 1; }
            draws.push(a);
        }
    }
    let p = at_least as f64 / (PRECEDENCE_NULL_ROTATIONS + 1) as f64;
    let sd = if draws.len() > 1 {
        let m = draws.iter().sum::<f64>() / draws.len() as f64;
        (draws.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / (draws.len() - 1) as f64).sqrt()
    } else {
        f64::NAN
    };
    (p, sd, n)
}

/// Signed, normalised area enclosed by the joint path of `x` and `y`.
pub fn precedence(
    engine: &Engine,
    name: &str,
    x: &str,
    y: &str,
    order: Option<String>,
) -> Result<PrecedenceResult, (StatusCode, String)> {
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    let has_field = |f: &str| schema.base_fields.iter()
        .chain(schema.fiber_fields.iter()).any(|d| d.name == f);
    if !has_field(x) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("x field '{}' not found", x)));
    }
    if !has_field(y) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!("y field '{}' not found", y)));
    }
    if x == y {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "x and y are the same field ('{}'); a signal cannot precede itself", x)));
    }
    if let Some(o) = &order {
        if !has_field(o) {
            return Err((StatusCode::UNPROCESSABLE_ENTITY,
                        format!("order field '{}' not found", o)));
        }
    }

    // TXP-17: these verbs read RECORD ORDER. Iteration equals insertion order
    // only for sequentially stored bundles; a bundle with a TEXT base field is
    // hash-stored and iterates arbitrarily. Measured on one hashed bundle:
    // area +0.7536 ordered by its sequence field, +0.0017 with `order` omitted
    // — a real signal flattened to nothing, HTTP 200, no warning. Refuse rather
    // than answer from an order that means nothing.
    if order.is_none() {
        let mode = store.storage_mode();
        if mode == "hashed" || mode == "hybrid" {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
                "bundle '{}' is {}-stored, so its records do not iterate in insertion order — and this verb reads record order. Name an ordering field. (A bundle gets this storage from a TEXT base field; there is nothing wrong with the bundle, but the order you inserted rows in is not recoverable from it.)",
                name, mode)));
        }
    }

    let mut records: Vec<crate::types::Record> = store.records().collect();
    let mut lexicographic_order = false;
    if records.len() > MAX_PRECEDENCE_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "precedence caps at {} records (got {}); filter the bundle first",
            MAX_PRECEDENCE_N, records.len())));
    }
    if let Some(o) = &order {
        lexicographic_order = crate::ml::sort_by_order(&mut records, o);
    }

    // TXP-13: a record contributes only if BOTH channels are numeric and
    // finite. Coercing a missing value to 0.0 would inject a fake excursion
    // into the path and manufacture area that never happened.
    let total = records.len();
    let mut xs: Vec<f64> = Vec::with_capacity(total);
    let mut ys: Vec<f64> = Vec::with_capacity(total);
    for r in &records {
        match (r.get(x).and_then(|v| v.as_f64()), r.get(y).and_then(|v| v.as_f64())) {
            (Some(a), Some(b)) if a.is_finite() && b.is_finite() => { xs.push(a); ys.push(b); }
            _ => {}
        }
    }
    let skipped = total - xs.len();
    let n = xs.len();
    if n < MIN_PRECEDENCE_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "precedence needs at least {} records where both '{}' and '{}' are \
             numeric and finite (got {} of {})",
            MIN_PRECEDENCE_N, x, y, n, total)));
    }

    // Translate to the path start: the enclosed area is about the origin of
    // the path, not the origin of the coordinate system.
    let (x0, y0) = (xs[0], ys[0]);
    for v in xs.iter_mut() { *v -= x0; }
    for v in ys.iter_mut() { *v -= y0; }

    let mut area = 0.0f64;
    let (mut qx, mut qy) = (0.0f64, 0.0f64);
    for i in 0..(n - 1) {
        area += xs[i] * ys[i + 1] - ys[i] * xs[i + 1];
        let (dx, dy) = (xs[i + 1] - xs[i], ys[i + 1] - ys[i]);
        qx += dx * dx;
        qy += dy * dy;
    }
    area *= 0.5;

    let qv = (qx * qy).sqrt();
    if !(qv > 0.0) || !qv.is_finite() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "one of '{}' or '{}' does not move over these {} records; \
             with no variation there is no area to measure", x, y, n)));
    }
    let a = area / qv;
    if !a.is_finite() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
                    "area is not finite; the path is degenerate".to_string()));
    }

    // The significance instrument. `area` alone cannot separate a real lead
    // from noise — see `rotation_null`.
    let dx: Vec<f64> = xs.windows(2).map(|w| w[1] - w[0]).collect();
    let dy: Vec<f64> = ys.windows(2).map(|w| w[1] - w[0]).collect();
    let (p_value, null_sd, null_n) = rotation_null(&dx, &dy, a);
    let significant = p_value <= 0.05;

    let leads = if a.abs() < PRECEDENCE_DEADBAND { "neither" }
                else if a > 0.0 { "x" } else { "y" };
    let reads = vec![
        match leads {
            "x" => format!("'{}' leads '{}' (area {:+.4}).", x, y, a),
            "y" => format!("'{}' leads '{}' (area {:+.4}).", y, x, a),
            _ => format!("Neither leads (area {:+.4}, inside the {:.0e} deadband): \
                          these two are identical at zero lag.", a, PRECEDENCE_DEADBAND),
        },
        // Significance, from the measured null rather than from prose.
        if significant {
            format!(
                "SIGNIFICANT: p = {:.3} against a rotation null (sd {:.3}). An area this large is unlikely with no lead relationship, given these two series' own increment structure.", p_value, null_sd)
        } else {
            format!(
                "NOT SIGNIFICANT: p = {:.3} against a rotation null (sd {:.3}). The direction above is the best estimate, but in THIS window it is not distinguishable from no relationship. Do not rank pairs on it. Lead-lag is a persistent property — aggregate the signed area across independent windows and test the mean, which is where this verb has its power (measured: a planted lead gives the correct sign 99.5% of the time and 20 windows separate from the null at z = 14.5, while a single window detects only about 20% of the time).", p_value, null_sd)
        },
        "Read the SIGN as direction. Magnitude peaks at a moderate lead and \
         decays as the two series decorrelate, so it is a strength-within-band \
         reading, not a lag in units of records.".to_string(),
    ];
    let mut notes = vec![
        match &order {
            Some(o) if lexicographic_order => format!(
                "ordered by '{}' LEXICOGRAPHICALLY — its values are not all numeric, so '10' sorts before '9'. Correct for ISO-8601 timestamps; WRONG for unpadded numeric ids, where it scrambles the record order this verb reads. Zero-pad them, or order by a numeric field.", o),
            Some(o) => format!("ordered by '{}'", o),
            // NOT a guarantee of insertion order. Record iteration follows the
            // bundle's STORAGE MODE: sequential bundles preserve insertion
            // order, hashed ones (any TEXT base field) do not. Measured on a
            // hashed bundle, omitting the order field gave area +0.0017 where
            // the same data ordered by its sequence field gave +0.7536 — a real
            // signal flattened to nothing, with the old note asserting the
            // opposite. These verbs read record order, so on a hashed bundle
            // the caller MUST name an ordering field.
            None => "no order field given — records read in the bundle's STORAGE order. That equals insertion order only on sequentially stored bundles; a bundle with a TEXT base field is stored hashed and will iterate in an arbitrary order, which silently scrambles what this verb measures. If in doubt, name an ordering field."
                .to_string(),
        },
        "area normalised by sqrt(QV_x * QV_y): dimensionless, so units and \
         scaling of either channel do not affect it".to_string(),
        "reads record ORDER only — no timestamp, no bin width, nothing to tune"
            .to_string(),
    ];
    if skipped > 0 {
        notes.push(format!(
            "{} of {} records skipped: '{}' or '{}' missing, non-numeric or \
             non-finite (skipped, never coerced to 0)", skipped, total, x, y));
    }

    if null_n < dx.len() {
        notes.push(format!(
            "null built from the first {} of {} gaps (cap {}); on very rough data the null narrows with sample size, so a capped null errs WIDE — conservative, costing power rather than inventing leads",
            null_n, dx.len(), PRECEDENCE_NULL_MAX_N));
    }
    Ok(PrecedenceResult {
        x: x.to_string(), y: y.to_string(), n, area: a,
        leads: leads.to_string(), magnitude: a.abs(),
        p_value, null_sd, significant, null_n,
        order_field: order, reads, notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Record, Value as V};

    /// TXP-18 — THE NULL MUST TRACK THE DATA'S OWN ROUGHNESS.
    ///
    /// `area` alone cannot separate a real lead from noise, and the noise floor
    /// is not a constant: with MA(1) increments the true null sd runs 0.006
    /// (rough) through 0.458 (random walk) to 1.046 (smooth) — a factor of 160,
    /// tracking exactly what TEXTURE measures.
    ///
    /// This gate is the reason a closed form was NOT shipped. For independent
    /// paths with iid increments the normalised area is Levy's stochastic area
    /// and `sd = 1/2` exactly, which is seductive and verified — and wrong on
    /// real tape in BOTH directions: ~80x too wide on rough data (never fires)
    /// and ~2x too narrow on smooth data (fires on noise). Replace
    /// `rotation_null` with the constant 1/2 and the rough and smooth rows below
    /// both fail.
    #[test]
    fn txp_18_null_tracks_roughness_not_a_constant() {
        // MA(1) increments: phi < 0 rough, phi > 0 smooth.
        fn ma1_inc(n: usize, phi: f64, seed: u64) -> Vec<f64> {
            let mut g = gauss_stream(seed);
            let mut prev = g();
            (0..n).map(|_| { let e = g(); let v = e + phi * prev; prev = e; v }).collect()
        }
        let n = 2048;
        let mut sds = Vec::new();
        for (k, phi) in [(-0.9f64), 0.0, 0.9].iter().enumerate() {
            let dx = ma1_inc(n, *phi, 11 + k as u64);
            let dy = ma1_inc(n, *phi, 97 + k as u64);
            let obs = area_from_increments(&dx, &dy).expect("area");
            let (p, sd, _) = rotation_null(&dx, &dy, obs);
            assert!(sd.is_finite() && sd > 0.0, "null sd must exist: {sd}");
            assert!((0.0..=1.0).contains(&p), "p must be a probability: {p}");
            sds.push(sd);
        }
        // The whole point: the null is NOT the same width at every roughness.
        assert!(sds[0] < sds[1] && sds[1] < sds[2],
                "null must widen from rough to smooth, got {sds:?}");
        assert!(sds[2] / sds[0] > 5.0,
                "rough and smooth nulls must differ by a large factor, got {sds:?} \
                 (a constant null would give a ratio of exactly 1)");
    }

    /// TXP-19: the significance instrument is wired into the response, is
    /// deterministic, and a planted lead is not called insignificant *for the
    /// wrong reason* — the p-value must be a real probability tied to the data.
    #[test]
    fn txp_19_p_value_is_reported_and_deterministic() {
        let lag = 6usize;
        let mut g = gauss_stream(31);
        let src: Vec<f64> = (0..(2048 + lag + 2)).map(|_| g()).collect();
        let (mut a, mut b) = (0.0f64, 0.0f64);
        let rows: Vec<Record> = (0..2048).map(|i| {
            a += src[i + lag];
            b += src[i];
            scan_rec(&[
                ("id", V::Text(format!("r{i:05}"))),
                ("i", V::Float(i as f64)),
                ("x", V::Float(a)),
                ("y", V::Float(b)),
                ("label", V::Text("p".into())),
            ])
        }).collect();
        let schema = BundleSchema::new("s")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("i"))
            .fiber(FieldDef::numeric("x"))
            .fiber(FieldDef::numeric("y"))
            .fiber(FieldDef::categorical("label"));
        let (dir, engine) = scan_env("txp_pval", "s", schema, rows);

        let r1 = precedence(&engine, "s", "x", "y", Some("i".into())).expect("p1");
        let r2 = precedence(&engine, "s", "x", "y", Some("i".into())).expect("p2");
        assert_eq!(r1.p_value.to_bits(), r2.p_value.to_bits(),
                   "p-value must be deterministic (TXP-14)");
        assert_eq!(r1.null_sd.to_bits(), r2.null_sd.to_bits());
        assert!((0.0..=1.0).contains(&r1.p_value), "p={}", r1.p_value);
        assert!(r1.null_sd.is_finite() && r1.null_sd > 0.0, "sd={}", r1.null_sd);
        assert_eq!(r1.significant, r1.p_value <= 0.05);
        // Every response must carry the significance reading, whichever way it
        // came out — this is the field whose absence was the blocker.
        assert!(r1.reads.iter().any(|s| s.contains("SIGNIFICANT")),
                "significance must be in reads: {:?}", r1.reads);
        cleanup(&dir);
    }

    fn gauss_stream(seed: u64) -> impl FnMut() -> f64 {
        let mut s = seed | 1;
        move || {
            let mut rnd = || {
                s ^= s << 13; s ^= s >> 7; s ^= s << 17;
                (s >> 11) as f64 / (1u64 << 53) as f64
            };
            let (u1, u2) = (rnd().max(1e-9), rnd());
            (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
        }
    }

    /// `y` follows `x` by `lag` records: a planted lead with a known direction.
    fn planted(n: usize, lag: usize, seed: u64) -> (Vec<f64>, Vec<f64>) {
        let mut g = gauss_stream(seed);
        let src: Vec<f64> = (0..n + lag + 2).map(|_| g()).collect();
        let cum = |v: Vec<f64>| {
            let mut a = 0.0;
            v.into_iter().map(|z| { a += z; a }).collect::<Vec<f64>>()
        };
        let xs = cum((0..n).map(|i| src[i + lag]).collect());
        let ys = cum((0..n).map(|i| src[i]).collect());
        (xs, ys)
    }

    fn schema_for(name: &str) -> BundleSchema {
        BundleSchema::new(name)
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("i"))
            .fiber(FieldDef::numeric("a"))
            .fiber(FieldDef::numeric("b"))
    }

    fn rows_from(xs: &[f64], ys: &[f64], order: impl Fn(usize) -> f64) -> Vec<Record> {
        (0..xs.len()).map(|i| scan_rec(&[
            ("id", V::Text(format!("r{i:05}"))),
            ("i", V::Float(order(i))),
            ("a", V::Float(xs[i])),
            ("b", V::Float(ys[i])),
        ])).collect()
    }

    /// TXP-3: identical series enclose no area. Exact, not approximate.
    #[test]
    fn txp_3_identical_series_give_zero_area() {
        let (xs, _) = planted(512, 0, 5);
        let (dir, engine) = scan_env("txp_zero", "s", schema_for("s"),
                                     rows_from(&xs, &xs, |i| i as f64));
        let r = precedence(&engine, "s", "a", "b", Some("i".into())).expect("precedence");
        assert!(r.area.abs() < 1e-9, "identical series gave area {}", r.area);
        assert_eq!(r.leads, "neither");
        cleanup(&dir);
    }

    /// TXP-4: correct sign, and swapping the roles must exactly negate it.
    #[test]
    fn txp_4_sign_correct_and_antisymmetric() {
        let (xs, ys) = planted(1024, 6, 11);
        let (dir, engine) = scan_env("txp_sign", "s", schema_for("s"),
                                     rows_from(&xs, &ys, |i| i as f64));
        let fwd = precedence(&engine, "s", "a", "b", Some("i".into())).expect("fwd");
        let rev = precedence(&engine, "s", "b", "a", Some("i".into())).expect("rev");
        assert!(fwd.area > 0.0, "x leads => positive area, got {:+.6}", fwd.area);
        assert_eq!(fwd.leads, "x");
        assert_eq!(rev.leads, "y");
        assert!((fwd.area + rev.area).abs() < 1e-9,
                "swapping roles must negate exactly: {} vs {}", fwd.area, rev.area);
        cleanup(&dir);
    }

    /// TXP-5: units must not matter. This is the gauge property.
    #[test]
    fn txp_5_invariant_to_rescaling_either_channel() {
        let (xs, ys) = planted(1024, 5, 13);
        let (d1, e1) = scan_env("txp_sc_a", "s", schema_for("s"),
                                rows_from(&xs, &ys, |i| i as f64));
        let base = precedence(&e1, "s", "a", "b", Some("i".into())).unwrap();

        let bx: Vec<f64> = xs.iter().map(|v| v * 1000.0).collect();
        let (d2, e2) = scan_env("txp_sc_b", "s", schema_for("s"),
                                rows_from(&bx, &ys, |i| i as f64));
        let sx = precedence(&e2, "s", "a", "b", Some("i".into())).unwrap();

        let by: Vec<f64> = ys.iter().map(|v| v * 500.0).collect();
        let (d3, e3) = scan_env("txp_sc_c", "s", schema_for("s"),
                                rows_from(&xs, &by, |i| i as f64));
        let sy = precedence(&e3, "s", "a", "b", Some("i".into())).unwrap();

        assert!((base.area - sx.area).abs() < 1e-9, "x rescale moved it");
        assert!((base.area - sy.area).abs() < 1e-9, "y rescale moved it");
        cleanup(&d1); cleanup(&d2); cleanup(&d3);
    }

    /// TXP-6: THE CLAIM. Same record ORDER, wildly different order-field
    /// VALUES, identical answer. The verb never reads a clock, so this is
    /// exact rather than approximate.
    #[test]
    fn txp_6_invariant_to_order_field_spacing() {
        let (xs, ys) = planted(1024, 7, 17);
        let (d1, e1) = scan_env("txp_clk_a", "s", schema_for("s"),
                                rows_from(&xs, &ys, |i| i as f64));
        // cubic clock: same sequence, utterly different spacing
        let (d2, e2) = scan_env("txp_clk_b", "s", schema_for("s"),
                                rows_from(&xs, &ys, |i| {
                                    let f = i as f64; f * f * f + f
                                }));
        let a = precedence(&e1, "s", "a", "b", Some("i".into())).unwrap();
        let b = precedence(&e2, "s", "a", "b", Some("i".into())).unwrap();
        assert_eq!(a.area.to_bits(), b.area.to_bits(),
                   "clock warp changed the answer: {} vs {}", a.area, b.area);
        cleanup(&d1); cleanup(&d2);
    }

    /// TXP-8 / TXP-9: refuse and name the problem.
    #[test]
    fn txp_8_9_refusals_name_the_problem() {
        let (xs, ys) = planted(256, 4, 2);
        let (dir, engine) = scan_env("txp_pref", "s", schema_for("s"),
                                     rows_from(&xs, &ys, |i| i as f64));

        let miss = precedence(&engine, "nope", "a", "b", None).unwrap_err();
        assert_eq!(miss.0, StatusCode::NOT_FOUND);
        assert!(miss.1.contains("nope"), "{}", miss.1);

        let bad = precedence(&engine, "s", "a", "ghost", Some("i".into())).unwrap_err();
        assert_eq!(bad.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(bad.1.contains("ghost"), "must name the field: {}", bad.1);

        let same = precedence(&engine, "s", "a", "a", Some("i".into())).unwrap_err();
        assert_eq!(same.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(same.1.contains("cannot precede itself"), "{}", same.1);
        cleanup(&dir);
    }

    /// TXP-11 / TXP-12: too thin, or a channel that never moves.
    #[test]
    fn txp_11_12_thin_and_static_refuse() {
        let (xs, ys) = planted(10, 2, 1);
        let (d1, e1) = scan_env("txp_thin_p", "s", schema_for("s"),
                                rows_from(&xs, &ys, |i| i as f64));
        let thin = precedence(&e1, "s", "a", "b", Some("i".into())).unwrap_err();
        assert_eq!(thin.0, StatusCode::UNPROCESSABLE_ENTITY);
        cleanup(&d1);

        let (xs2, _) = planted(256, 3, 6);
        let flat = vec![7.0f64; 256];
        let (d2, e2) = scan_env("txp_static_p", "s", schema_for("s"),
                                rows_from(&xs2, &flat, |i| i as f64));
        let err = precedence(&e2, "s", "a", "b", Some("i".into())).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("does not move"), "{}", err.1);
        cleanup(&d2);
    }

    /// TXP-13: rows where either channel is unusable are skipped and disclosed.
    #[test]
    fn txp_13_skips_and_discloses() {
        let (xs, ys) = planted(600, 5, 8);
        let rows: Vec<Record> = (0..xs.len()).map(|i| {
            let bv = if i % 40 == 3 { V::Null } else { V::Float(ys[i]) };
            scan_rec(&[
                ("id", V::Text(format!("r{i:05}"))),
                ("i", V::Float(i as f64)),
                ("a", V::Float(xs[i])),
                ("b", bv),
            ])
        }).collect();
        let (dir, engine) = scan_env("txp_skip_p", "s", schema_for("s"), rows);
        let r = precedence(&engine, "s", "a", "b", Some("i".into())).expect("precedence");
        assert!(r.n < 600, "skipped rows counted as data (n={})", r.n);
        assert!(r.notes.iter().any(|s| s.contains("skipped")), "{:?}", r.notes);
        cleanup(&dir);
    }

    /// TXP-14: deterministic.
    #[test]
    fn txp_14_deterministic() {
        let (xs, ys) = planted(512, 5, 21);
        let (dir, engine) = scan_env("txp_pdet", "s", schema_for("s"),
                                     rows_from(&xs, &ys, |i| i as f64));
        let a = precedence(&engine, "s", "a", "b", Some("i".into())).unwrap();
        let b = precedence(&engine, "s", "a", "b", Some("i".into())).unwrap();
        assert_eq!(a.area.to_bits(), b.area.to_bits());
        cleanup(&dir);
    }

    /// Every successful answer carries a plain-English reading.
    #[test]
    fn txp_reads_are_populated() {
        let (xs, ys) = planted(512, 6, 31);
        let (dir, engine) = scan_env("txp_reads_p", "s", schema_for("s"),
                                     rows_from(&xs, &ys, |i| i as f64));
        let r = precedence(&engine, "s", "a", "b", Some("i".into())).unwrap();
        assert!(!r.reads.is_empty());
        assert!(r.reads[0].contains("leads"), "{:?}", r.reads);
        assert!(r.notes.iter().any(|n| n.contains("record ORDER")),
                "the no-clock property should be stated in the notes: {:?}", r.notes);
        cleanup(&dir);
    }
}
