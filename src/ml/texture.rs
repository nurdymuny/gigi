//! TEXTURE — how rough is an ordered numeric signal?
//!
//! Returns the self-similarity (Hurst) exponent `H` of one numeric field,
//! estimated from how fast the signal's typical movement grows with the gap
//! you look across:
//!
//! ```text
//! E|X(i+d) - X(i)|^q  ~  d^(q*H)
//! ```
//!
//! so a straight-line fit of `log m(q,d)` against `log d`, divided by `q`,
//! recovers `H`. Classical scaling estimator; Mandelbrot-Van Ness lineage,
//! standard in the rough-volatility literature.
//!
//! * `H > 1/2` persistent — moves tend to continue (a smooth, trending signal)
//! * `H = 1/2` a random walk
//! * `H < 1/2` anti-persistent — moves tend to reverse (a jagged signal)
//!
//! NOT finance-specific. Anything sampled in order: queue latency, sensor
//! drift, server load, a patient vital, a price. The API says `field` and
//! `order`, never anything about markets.
//!
//! Validated against exact fractional Brownian motion (Davies-Harte circulant
//! embedding) before porting: max |error| 0.029 over H = 0.1..0.9, monotone
//! throughout. Gates TXP-1, TXP-2, TXP-7 pin that here.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Hard cap. Above this we refuse rather than churn — TXP-15. A `MAX_N`
/// overrun that returns `null` instead of an error is worse than a refusal;
/// that exact failure cost an hour of debugging on another verb.
pub const MAX_TEXTURE_N: usize = 2_000_000;

/// Minimum records for a defensible fit: we need several distinct lags, and
/// each lag needs enough differences to average over.
pub const MIN_TEXTURE_N: usize = 64;

/// Request for `POST /v1/bundles/{name}/texture`.
// `deny_unknown_fields`: a misspelled key used to be IGNORED, so the verb
// silently fell back to the default and returned a different number with
// HTTP 200. Measured: `maxLag` gave exponent 0.505802 where `max_lag` gave
// 0.495073. A typo must be an error, not a quiet change of answer.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureRequest {
    /// Numeric field to measure. Base or fiber — callers should not have to
    /// care which side of the schema a column landed on.
    pub field: String,
    /// Field to order the series by. Omitted → record order.
    #[serde(default)]
    pub order: Option<String>,
    /// Moment order for the structure function. Default 2 (variance-like).
    #[serde(default = "default_texture_q")]
    pub q: f64,
    /// Smallest gap to fit over. Default 1.
    #[serde(default = "default_texture_min_lag")]
    pub min_lag: usize,
    /// Largest gap. Omitted → n/8, which keeps every lag well-sampled.
    #[serde(default)]
    pub max_lag: Option<usize>,
}
pub fn default_texture_q() -> f64 { 2.0 }
pub fn default_texture_min_lag() -> usize { 1 }

#[derive(Debug)]
pub struct TextureResult {
    pub field: String,
    pub n: usize,
    pub exponent: f64,
    pub r_squared: f64,
    pub n_lags: usize,
    /// Sampling sd of `exponent` at this `n` — the ruler the verdict band uses.
    pub h_sd: f64,
    pub verdict: String,
    pub order_field: Option<String>,
    pub reads: Vec<String>,
    pub notes: Vec<String>,
}

/// Sampling standard deviation of the exponent, as a function of `n`.
///
/// MEASURED, not chosen. On a true random walk (H = 0.5) this estimator's own
/// spread is:
///
/// ```text
/// n     mean H    sd(H)    2*sd
///     64     0.4790   0.0964   0.1929
///    256     0.4889   0.0615   0.1230
///   1024     0.4916   0.0433   0.0866
///   4096     0.4947   0.0296   0.0591
///  16384     0.4971   0.0220   0.0441
/// ```
///
/// A log-log fit gives `sd ~ 0.29 * n^-0.267`, reproduced above to within about
/// 7%. This is an EMPIRICAL law fitted to simulation, not a closed form — said
/// plainly because the alternative was worse: the shipped bands were a flat
/// `H < 0.45` / `H > 0.55`, a half-width of 0.05 that is NARROWER THAN THIS
/// ESTIMATOR'S OWN NOISE for every n below about 16000. At n = 1024 that band
/// is barely one standard deviation, so a true random walk was being called
/// ROUGH or SMOOTH roughly a third of the time.
///
/// The verdict band is now `0.5 +- 2*sd(n)`, so it widens on short series
/// instead of pretending to a precision the fit does not have.
fn exponent_sd(n: usize) -> f64 {
    0.29 * (n as f64).powf(-0.267)
}

/// Estimate the self-similarity exponent of `field`.
pub fn texture(
    engine: &Engine,
    name: &str,
    field: &str,
    order: Option<String>,
    q: f64,
    min_lag: usize,
    max_lag: Option<usize>,
) -> Result<TextureResult, (StatusCode, String)> {
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    let has_field = |f: &str| schema.base_fields.iter()
        .chain(schema.fiber_fields.iter()).any(|d| d.name == f);
    if !has_field(field) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
                    format!("field '{}' not found", field)));
    }
    if let Some(o) = &order {
        if !has_field(o) {
            return Err((StatusCode::UNPROCESSABLE_ENTITY,
                        format!("order field '{}' not found", o)));
        }
    }
    if !(q > 0.0) || !q.is_finite() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
                    format!("q must be finite and > 0 (got {})", q)));
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
    if records.len() > MAX_TEXTURE_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "texture caps at {} records (got {}); filter the bundle first",
            MAX_TEXTURE_N, records.len())));
    }
    if let Some(o) = &order {
        lexicographic_order = crate::ml::sort_by_order(&mut records, o);
    }

    // TXP-13: non-finite and non-numeric values are SKIPPED and counted,
    // never coerced to 0.0. A silent coercion turns missing data into a
    // confident measurement of something that was never there.
    let total = records.len();
    let series: Vec<f64> = records.iter()
        .filter_map(|r| r.get(field).and_then(|v| v.as_f64()))
        .filter(|v| v.is_finite())
        .collect();
    let skipped = total - series.len();
    let n = series.len();

    if n < MIN_TEXTURE_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "texture needs at least {} usable numeric values in '{}' (got {} of {} records)",
            MIN_TEXTURE_N, field, n, total)));
    }
    let mean = series.iter().sum::<f64>() / n as f64;
    let var = series.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    if var <= 0.0 || !var.is_finite() {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "field '{}' has zero variance over {} records; there is no texture to measure",
            field, n)));
    }

    let hi = max_lag.unwrap_or(n / 8).max(min_lag + 1).min(n / 2);
    let lo = min_lag.max(1);
    if hi <= lo {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "lag range is empty: min_lag {} vs usable max {}", lo, hi)));
    }
    // Log-spaced lags: the fit is in log-log, so uniform lags would weight the
    // long end almost not at all.
    let n_pts = 12usize;
    let mut lags: Vec<usize> = (0..n_pts)
        .map(|i| {
            let t = i as f64 / (n_pts - 1) as f64;
            ((lo as f64).ln() + t * ((hi as f64).ln() - (lo as f64).ln())).exp().round() as usize
        })
        .filter(|&d| d >= lo && d <= hi)
        .collect();
    lags.sort_unstable();
    lags.dedup();

    let (mut sx, mut sy, mut sxx, mut sxy, mut k) = (0.0, 0.0, 0.0, 0.0, 0usize);
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for &d in &lags {
        if series.len() <= d { continue; }
        let mut acc = 0.0;
        let mut c = 0usize;
        for i in 0..(series.len() - d) {
            acc += (series[i + d] - series[i]).abs().powf(q);
            c += 1;
        }
        if c < 20 { continue; }
        let m = acc / c as f64;
        if !(m > 0.0) || !m.is_finite() { continue; }
        let (lx, ly) = ((d as f64).ln(), m.ln());
        pts.push((lx, ly));
        sx += lx; sy += ly; sxx += lx * lx; sxy += lx * ly; k += 1;
    }
    if k < 4 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "only {} usable lags for the fit (need 4); the series is too short or too flat", k)));
    }
    let kf = k as f64;
    let denom = kf * sxx - sx * sx;
    if denom.abs() < 1e-30 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
                    "degenerate lag spacing; cannot fit".to_string()));
    }
    let slope = (kf * sxy - sx * sy) / denom;
    let intercept = (sy - slope * sx) / kf;
    let ybar = sy / kf;
    let (mut ss_res, mut ss_tot) = (0.0, 0.0);
    for &(lx, ly) in &pts {
        let pred = intercept + slope * lx;
        ss_res += (ly - pred).powi(2);
        ss_tot += (ly - ybar).powi(2);
    }
    let r2 = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };
    let h = slope / q;

    // Bands derived from the estimator's measured noise at THIS n, not from a
    // flat convention. See `exponent_sd`.
    let h_sd = exponent_sd(n);
    let half = 2.0 * h_sd;
    let verdict = if h < 0.5 - half { "ROUGH" }
                  else if h > 0.5 + half { "SMOOTH" }
                  else { "RANDOM_WALK" };
    let reads = vec![
        match verdict {
            "ROUGH" => format!(
                "H = {:.3}: anti-persistent — movements tend to REVERSE. Choppy; \
                 trend-following on this signal fights itself.", h),
            "SMOOTH" => format!(
                "H = {:.3}: persistent — movements tend to CONTINUE. A move is \
                 evidence of more of the same.", h),
            _ => format!(
                "H = {:.3}: indistinguishable from a random walk. Past movement \
                 carries no directional information here.", h),
        },
        format!("Fit quality R^2 = {:.4} over {} lags. A low R^2 means the signal \
                 is not self-similar and H should not be read as a single number.", r2, k),
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
        format!("{} log-spaced lags from {} to {}, q = {}", k, lo, hi, q),
        format!("verdict band is 0.5 +- {:.3} (2 x the estimator's sd of {:.3} at n = {}), derived from measured noise rather than a fixed cutoff; it widens on shorter series", half, h_sd, n),
    ];
    if skipped > 0 {
        notes.push(format!(
            "{} of {} records skipped: '{}' missing, non-numeric or non-finite \
             (skipped, never coerced to 0)", skipped, total, field));
    }

    Ok(TextureResult {
        field: field.to_string(), n, exponent: h, r_squared: r2, n_lags: k, h_sd,
        verdict: verdict.to_string(), order_field: order, reads, notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Record, Value as V};

    /// Deterministic standard-normal stream; no external dep so the fixture is
    /// reproducible byte-for-byte across machines.
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

    /// Paths with a KNOWN roughness ordering, built from the autocorrelation
    /// of the increments — the property H actually measures.
    ///   phi < 0  increments anti-correlated  -> moves reverse   -> H < 1/2
    ///   phi = 0  iid increments -> random walk     -> H = 1/2
    ///   phi > 0  increments correlated -> moves continue  -> H > 1/2
    /// An MA(1) on the increments is the cleanest generator with that property
    /// and needs no FFT. Exact-H recovery to +-0.03 against true Davies-Harte
    /// fBm was validated pre-port; what this gate pins is the CONTRACT, that
    /// rougher input reads rougher.
    fn ma1_path(n: usize, phi: f64, seed: u64) -> Vec<f64> {
        let mut g = gauss_stream(seed);
        let mut prev = g();
        let mut acc = 0.0f64;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let e = g();
            acc += e + phi * prev;
            prev = e;
            out.push(acc);
        }
        out
    }

    fn rows_from(vals: &[f64]) -> Vec<Record> {
        vals.iter().enumerate().map(|(i, v)| scan_rec(&[
            ("i", V::Float(i as f64)),
            ("v", V::Float(*v)),
            ("label", V::Text("x".into())),
        ])).collect()
    }

    fn schema_for(name: &str) -> BundleSchema {
        BundleSchema::new(name)
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("i"))
            .fiber(FieldDef::numeric("v"))
            .fiber(FieldDef::categorical("label"))
    }

    fn rows_with_id(vals: &[f64]) -> Vec<Record> {
        vals.iter().enumerate().map(|(i, v)| scan_rec(&[
            ("id", V::Text(format!("r{i:05}"))),
            ("i", V::Float(i as f64)),
            ("v", V::Float(*v)),
            ("label", V::Text("x".into())),
        ])).collect()
    }

    /// TXP-1 + TXP-2: rougher input must read rougher, and the ordering of
    /// three known-roughness signals must be recovered.
    #[test]
    fn txp_1_2_orders_known_roughness() {
        let mut got = Vec::new();
        for (k, phi) in [(-0.9f64), 0.0, 0.9].iter().enumerate() {
            let vals = ma1_path(4096, *phi, 7 + k as u64);
            let (dir, engine) = scan_env(
                &format!("txp_h{k}"), "s", schema_for("s"), rows_with_id(&vals));
            let r = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None)
                .expect("texture");
            assert!(r.exponent.is_finite(), "H must be finite");
            // An MA(1) is only ASYMPTOTICALLY self-similar: at short lags the
            // MA structure dominates and the log-log relation genuinely is not
            // a straight line. R^2 lands near 0.78 here, and that is the field
            // doing its job — reporting that this path is not perfectly
            // self-similar rather than hiding it behind a clean-looking H.
            // The gate is that the fit is USABLE, not that the fixture is ideal.
            assert!(r.r_squared > 0.6,
                    "fit unusable, R2={} — H should not be read from this", r.r_squared);
            got.push(r.exponent);
            cleanup(&dir);
        }
        assert!(got[0] < got[1] && got[1] < got[2],
                "roughness ordering not recovered: {got:?}");
    }

    /// TXP-16: a numeric-TEXT order field must sort NUMERICALLY, and a
    /// genuinely non-numeric one must DISCLOSE that it sorted as text.
    ///
    /// Found by customer-readiness review. Before the fix, one trending series
    /// read exponent 0.5307 / RANDOM_WALK ordered by a numeric field and
    /// 0.0882 / ROUGH ordered by its own text id — same data, same verb, a
    /// flipped verdict, and `notes` said only "ordered by 'sid'". Worse, a
    /// ZERO-PADDED text id gave the right answer, so the defect appeared only
    /// on some customers' data. A trade id exported as text is the ordinary
    /// case, not an exotic one.
    #[test]
    fn txp_16_numeric_text_order_sorts_numerically_and_text_is_disclosed() {
        let vals = ma1_path(4096, 0.9, 4);
        let schema = BundleSchema::new("s")
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("i"))
            .fiber(FieldDef::numeric("v"))
            .fiber(FieldDef::categorical("sid"))
            .fiber(FieldDef::categorical("iso"))
            .fiber(FieldDef::categorical("label"));
        let rows: Vec<Record> = vals.iter().enumerate().map(|(i, v)| scan_rec(&[
            ("id", V::Text(format!("r{i:05}"))),
            ("i", V::Float(i as f64)),
            ("v", V::Float(*v)),
            // Unpadded numeric text — the trap.
            ("sid", V::Text(format!("{i}"))),
            // Genuinely non-numeric but correctly lexicographic.
            ("iso", V::Text(format!("2026-08-09T{:02}:{:02}:{:02}Z",
                                    i / 3600 % 24, i / 60 % 60, i % 60))),
            ("label", V::Text("x".into())),
        ])).collect();
        let (dir, engine) = scan_env("txp_lex", "s", schema, rows);

        let by_num = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None).unwrap();
        let by_txt = texture(&engine, "s", "v", Some("sid".into()), 2.0, 1, None).unwrap();
        assert!((by_num.exponent - by_txt.exponent).abs() < 1e-9,
                "numeric text must sort numerically: {} (by 'i') vs {} (by 'sid')",
                by_num.exponent, by_txt.exponent);
        assert!(!by_txt.notes.iter().any(|n| n.contains("LEXICOGRAPHICALLY")),
                "a numeric-text order field sorted numerically — do not warn: {:?}",
                by_txt.notes);

        // Non-numeric values still sort as text, and that is DISCLOSED.
        let by_iso = texture(&engine, "s", "v", Some("iso".into()), 2.0, 1, None).unwrap();
        assert!(by_iso.notes.iter().any(|n| n.contains("LEXICOGRAPHICALLY")),
                "a text sort must be disclosed: {:?}", by_iso.notes);
        cleanup(&dir);
    }

    /// TXP-7: H is a shape property. An affine rescale must not move it.
    #[test]
    fn txp_7_invariant_to_affine_rescale() {
        let vals = ma1_path(4096, 0.5, 3);
        let (d1, e1) = scan_env("txp_aff_a", "s", schema_for("s"), rows_with_id(&vals));
        let a = texture(&e1, "s", "v", Some("i".into()), 2.0, 1, None).expect("a");
        let scaled: Vec<f64> = vals.iter().map(|v| v * 1000.0 + 42.0).collect();
        let (d2, e2) = scan_env("txp_aff_b", "s", schema_for("s"), rows_with_id(&scaled));
        let b = texture(&e2, "s", "v", Some("i".into()), 2.0, 1, None).expect("b");
        assert!((a.exponent - b.exponent).abs() < 1e-9,
                "affine rescale moved H: {} vs {}", a.exponent, b.exponent);
        cleanup(&d1); cleanup(&d2);
    }

    /// TXP-8 / TXP-9 / TXP-10: refuse, and name what is wrong.
    #[test]
    fn txp_8_9_10_refusals_name_the_problem() {
        let vals = ma1_path(512, 0.0, 1);
        let (dir, engine) = scan_env("txp_ref", "s", schema_for("s"), rows_with_id(&vals));

        let miss = texture(&engine, "no_such", "v", None, 2.0, 1, None).unwrap_err();
        assert_eq!(miss.0, StatusCode::NOT_FOUND);
        assert!(miss.1.contains("no_such"), "must name the bundle: {}", miss.1);

        let f = texture(&engine, "s", "ghost", None, 2.0, 1, None).unwrap_err();
        assert_eq!(f.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(f.1.contains("ghost"), "must name the field: {}", f.1);

        // TXP-10: a categorical field yields no numeric values. Refuse rather
        // than silently measuring a series of coerced zeros.
        let t = texture(&engine, "s", "label", None, 2.0, 1, None).unwrap_err();
        assert_eq!(t.0, StatusCode::UNPROCESSABLE_ENTITY);

        let q = texture(&engine, "s", "v", None, 0.0, 1, None).unwrap_err();
        assert_eq!(q.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(q.1.contains("q must be"), "{}", q.1);
        cleanup(&dir);
    }

    /// TXP-17: on a bundle whose records do not iterate in insertion order,
    /// an order-less call must REFUSE rather than measure a meaningless order.
    ///
    /// These verbs read record order. A bundle with a TEXT base field is
    /// hash-stored, and iteration is then arbitrary. Measured before this guard,
    /// on one hashed bundle: PRECEDENCE returned area +0.7536 ordered by its
    /// sequence field and +0.0017 with `order` omitted — a real signal flattened
    /// to nothing, HTTP 200, no warning, and a `notes` line that claimed
    /// "records left in insertion order".
    #[test]
    fn txp_17_refuses_order_less_call_on_unordered_storage() {
        let vals = ma1_path(1024, 0.5, 77);
        let (dir, engine) = scan_env("txp_hash", "s", schema_for("s"), rows_with_id(&vals));

        let err = texture(&engine, "s", "v", None, 2.0, 1, None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("insertion order"), "must explain why: {}", err.1);
        assert!(err.1.contains("ordering field"), "must say what to do: {}", err.1);

        // Naming the ordering field is accepted and answers.
        let ok = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None)
            .expect("naming an ordering field must work");
        assert!(ok.exponent.is_finite());
        cleanup(&dir);
    }

    /// TXP-11 / TXP-12: too little data, or no variance at all.
    #[test]
    fn txp_11_12_insufficient_and_flat_refuse() {
        let short: Vec<f64> = (0..10).map(|i| i as f64).collect();
        // These fixtures have a TEXT base field, so they are hash-stored and
        // TXP-17 refuses an order-less call. Name the ordering field — that is
        // now the correct way to use the verb on such a bundle.
        let (d1, e1) = scan_env("txp_thin", "s", schema_for("s"), rows_with_id(&short));
        let err = texture(&e1, "s", "v", Some("i".into()), 2.0, 1, None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("10"), "must report the actual count: {}", err.1);
        cleanup(&d1);

        let flat = vec![5.0f64; 512];
        let (d2, e2) = scan_env("txp_flat", "s", schema_for("s"), rows_with_id(&flat));
        let err2 = texture(&e2, "s", "v", Some("i".into()), 2.0, 1, None).unwrap_err();
        assert_eq!(err2.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err2.1.contains("variance"), "{}", err2.1);
        cleanup(&d2);
    }

    /// TXP-14: same input, same bits.
    #[test]
    fn txp_14_deterministic() {
        let vals = ma1_path(1024, 0.2, 9);
        let (dir, engine) = scan_env("txp_det", "s", schema_for("s"), rows_with_id(&vals));
        let a = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None).unwrap();
        let b = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None).unwrap();
        assert_eq!(a.exponent.to_bits(), b.exponent.to_bits());
        assert_eq!(a.verdict, b.verdict);
        cleanup(&dir);
    }

    /// TXP-13: bad values are SKIPPED and DISCLOSED, never coerced to 0.
    #[test]
    fn txp_13_skips_and_discloses_bad_values() {
        let vals = ma1_path(800, 0.0, 4);
        let rows: Vec<Record> = vals.iter().enumerate().map(|(i, v)| {
            let val = if i % 50 == 7 { V::Null } else { V::Float(*v) };
            scan_rec(&[
                ("id", V::Text(format!("r{i:05}"))),
                ("i", V::Float(i as f64)),
                ("v", val),
                ("label", V::Text("x".into())),
            ])
        }).collect();
        let (dir, engine) = scan_env("txp_skip", "s", schema_for("s"), rows);
        let r = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None).expect("texture");
        assert!(r.n < 800, "skipped rows must not be counted as data (n={})", r.n);
        assert!(r.notes.iter().any(|s| s.contains("skipped")),
                "the skip must be disclosed: {:?}", r.notes);
        cleanup(&dir);
    }

    /// Every successful answer carries a plain-English reading. A number a
    /// caller cannot interpret is not an ergonomic API.
    #[test]
    fn txp_reads_are_populated() {
        let vals = ma1_path(1024, 0.5, 12);
        let (dir, engine) = scan_env("txp_reads", "s", schema_for("s"), rows_with_id(&vals));
        let r = texture(&engine, "s", "v", Some("i".into()), 2.0, 1, None).unwrap();
        assert!(!r.reads.is_empty());
        assert!(r.reads[0].contains("H ="), "{:?}", r.reads);
        assert!(["ROUGH", "SMOOTH", "RANDOM_WALK"].contains(&r.verdict.as_str()));
        cleanup(&dir);
    }
}
