//! PRECEDENCE — which of two ordered signals moves first?
//!
//! Returns the normalised signed area enclosed by the 2-D path `(x, y)`:
//!
//! ```text
//! A = 1/2 * sum_i ( x_i * y_{i+1} - y_i * x_{i+1} )        (about the start)
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
pub const PRECEDENCE_DEADBAND: f64 = 1e-6;

/// Request for `POST /v1/bundles/{name}/precedence`.
#[derive(Deserialize)]
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
    pub order_field: Option<String>,
    pub reads: Vec<String>,
    pub notes: Vec<String>,
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

    let leads = if a.abs() < PRECEDENCE_DEADBAND { "neither" }
                else if a > 0.0 { "x" } else { "y" };
    let reads = vec![
        match leads {
            "x" => format!("'{}' leads '{}' (area {:+.4}).", x, y, a),
            "y" => format!("'{}' leads '{}' (area {:+.4}).", y, x, a),
            _ => format!("Neither leads (area {:+.4}, inside the {:.0e} deadband): \
                          these two are identical at zero lag.", a, PRECEDENCE_DEADBAND),
        },
        // The honest health warning. PRECEDENCE ships NO significance measure,
        // and the reader cannot supply one by intuition because the Levy area
        // does not behave like a correlation: it does NOT concentrate with more
        // data. Measured on independent random walks, sd(area) is 0.48-0.51 and
        // is the SAME at n = 512 and n = 32768. Twenty independent pairs all
        // returned a confident direction and `neither` never fired.
        format!(
            "CONFIDENCE: this verb returns no significance figure, and |area| = \
             {:.4} on its own does NOT establish a lead. On INDEPENDENT series \
             with no relationship, |area| has a standard deviation near 0.5 and \
             that does not shrink as you add data — so a single pair reading \
             under about 1.0 is inside the range pure noise produces. Treat one \
             reading as a hypothesis: confirm it against a null you build \
             yourself (shuffle or rotate one channel and re-measure), or across \
             independent sessions. Do not rank instruments by |area| alone.",
            a.abs()),
        "Read the SIGN as direction. Magnitude peaks at a moderate lead and \
         decays as the two series decorrelate, so it is a strength-within-band \
         reading, not a lag in units of records.".to_string(),
    ];
    let mut notes = vec![
        match &order {
            Some(o) if lexicographic_order => format!(
                "ordered by '{}' LEXICOGRAPHICALLY — its values are not all                  numeric, so '10' sorts before '9'. Correct for ISO-8601                  timestamps; WRONG for unpadded numeric ids, where it scrambles                  the record order this verb reads. Zero-pad them, or order by a                  numeric field.", o),
            Some(o) => format!("ordered by '{}'", o),
            // NOT a guarantee of insertion order. Record iteration follows the
            // bundle's STORAGE MODE: sequential bundles preserve insertion
            // order, hashed ones (any TEXT base field) do not. Measured on a
            // hashed bundle, omitting the order field gave area +0.0017 where
            // the same data ordered by its sequence field gave +0.7536 — a real
            // signal flattened to nothing, with the old note asserting the
            // opposite. These verbs read record order, so on a hashed bundle
            // the caller MUST name an ordering field.
            None => "no order field given — records read in the bundle's STORAGE                      order. That equals insertion order only on sequentially                      stored bundles; a bundle with a TEXT base field is stored                      hashed and will iterate in an arbitrary order, which                      silently scrambles what this verb measures. If in doubt,                      name an ordering field."
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

    Ok(PrecedenceResult {
        x: x.to_string(), y: y.to_string(), n, area: a,
        leads: leads.to_string(), magnitude: a.abs(),
        order_field: order, reads, notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Record, Value as V};

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
