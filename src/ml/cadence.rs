//! CADENCE — is this stream arriving steadily, or in bursts?
//!
//! The third member of the shape family. TEXTURE asks how rough a signal is,
//! PRECEDENCE asks which of two signals moves first — and both deliberately
//! read *record order*, discarding the timestamps entirely. CADENCE reads
//! exactly what they throw away: the spacing between events.
//!
//! Two numbers, and they are orthogonal by construction.
//!
//! # 1. `index` — how uneven the spacing is
//!
//! Let `tau_1..tau_n` be the gaps between consecutive events and
//!
//! ```text
//! R_n = sum(tau^2) / (sum tau)^2
//! index^2 = (n+1)/(n-1) * (n * R_n - 1)
//! ```
//!
//! `R_n` is the Greenwood statistic. For a memoryless (Poisson) stream the
//! normalised gaps `tau/sum(tau)` are Dirichlet(1,..,1) — *for every rate*,
//! exactly, at finite `n` — so
//!
//! ```text
//! E[R_n]   = 2/(n+1)
//! Var[R_n] = 4(n-1) / [(n+1)^2 (n+2) (n+3)]
//! ```
//!
//! are exact, and `E[index^2] = 1` identically. The null needs no simulation
//! and does not depend on how fast events arrive. `index = 1` is memoryless,
//! `> 1` is bursty, `< 1` is *more regular than random* — which for most real
//! sources means something is metering the stream.
//!
//! # 2. `memory` — whether the burstiness has any memory
//!
//! `R_n` is a **symmetric function of the gaps**: permuting them cannot change
//! it. Self-excitation — events triggering further events — is an *ordering*
//! property, so `index` has exactly zero power against it. Verified: a Hawkes
//! process and the same gaps shuffled both read 1.448.
//!
//! That is not a defect once you know it, because it makes the lag-1
//! autocorrelation of `ln tau` an exactly non-overlapping second measurement:
//!
//! ```text
//! memory = sum(l_i * l_{i+1}) / sum(l_i^2),   l = ln(tau) - mean(ln tau)
//! ```
//!
//! with null sd `1/sqrt(n)`. `memory > 0` means long gaps follow long gaps and
//! short follow short — the stream has a state it stays in for a while.
//!
//! Together: `index` says *how uneven*, `memory` says *whether the unevenness
//! persists*. An independent stream with a heavy-tailed gap distribution scores
//! high on the first and zero on the second; a self-exciting one scores on both.
//!
//! # What this is NOT
//!
//! Not a clustering or self-excitation detector on its own — see above, and see
//! `theory/gigi/TDD-CAD_cadence.md` for the measurements that establish it.
//! Neither number predicts anything. Both describe the stream in front of them.
//!
//! Greenwood (1946) for the statistic; the Dirichlet null is the standard
//! exponential-spacings result. No novelty is claimed for either.
//!
//! NOT finance-specific. Anything with arrival times: log lines, sensor pings,
//! request traces, seismic events, neuron spikes, trades.

use axum::http::StatusCode;
use serde::Deserialize;

use crate::engine::Engine;

/// Hard cap — refuse rather than churn (CAD-15).
pub const MAX_CADENCE_N: usize = 2_000_000;

/// Minimum usable **events**. Yields `MIN_CADENCE_N - 1` gaps; at 64 the null
/// sd of `index` is 0.121, which is wide but honest and reported.
pub const MIN_CADENCE_N: usize = 64;

/// Default gaps per block for the blocked form. The block length *is* the
/// filter corner (see [`cadence`] docs on `block`), so it is held fixed and the
/// block count floats with `n` — the opposite of fixing `K`, which would let
/// the corner drift with sample size and quietly change what is being measured.
pub const DEFAULT_CADENCE_BLOCK: usize = 64;

/// Smallest defensible block. Below this the per-block `index` is mostly noise.
pub const MIN_CADENCE_BLOCK: usize = 16;

/// Multiple of the inferred timestamp quantum the median gap must clear
/// (CAD-9). Calibrated against a bias budget of half a null sd; below ~10 the
/// discretisation bias in `index` exceeds that and, worse, turns *negative* —
/// a coarse feed reads as suspiciously regular rather than as unmeasurable.
pub const CADENCE_MIN_GRANULARITY: f64 = 10.0;

/// Maximum share of gaps that may be exactly equal before the stream is judged
/// quantised rather than clocked (CAD-16). A lattice feed measures 0.22..0.33
/// and is unmoved by injected off-grid records; continuous feeds measure
/// 0.000..0.016, where `1/n` is the floor at the minimum sample size. The
/// threshold sits in the empty decade between those two populations.
pub const CADENCE_MAX_LATTICE_FRAC: f64 = 0.10;

/// Request for `POST /v1/bundles/{name}/cadence`.
#[derive(Deserialize)]
pub struct CadenceRequest {
    /// Numeric timestamp field. **Required, and deliberately not named
    /// `order`.** TEXTURE and PRECEDENCE take an *ordinal* sort key whose
    /// values are ignored; here the values are the entire content. A caller who
    /// copies a working TEXTURE call and passes a row index gets perfectly
    /// uniform gaps, and CAD-10 refuses that by name rather than reporting a
    /// confident `index = 0` on a healthy stream.
    pub time: String,
    /// Gaps per block for the blocked form. Omitted → [`DEFAULT_CADENCE_BLOCK`].
    #[serde(default)]
    pub block: Option<usize>,
}

#[derive(Debug)]
pub struct CadenceResult {
    pub time_field: String,
    /// Usable events after coalescing.
    pub n_events: usize,
    /// Gaps measured — `n_events - 1`, and the `n` in every formula.
    pub n_gaps: usize,
    pub index: f64,
    /// Closed-form null sd of `index`: `n/sqrt((n-1)(n+2)(n+3))`.
    pub null_sd: f64,
    /// Standard deviations from the memoryless null.
    pub index_z: f64,
    /// Mean per-block `index`; structure longer than one block is filtered out.
    pub index_blocked: f64,
    pub block_len: usize,
    pub n_blocks: usize,
    pub memory: f64,
    pub memory_z: f64,
    pub verdict: String,
    pub memory_verdict: String,
    pub reads: Vec<String>,
    pub notes: Vec<String>,
}

/// Closed-form null sd of `index` at `n` gaps, by the delta method on
/// `Var[index^2] = 4n^2/[(n-1)(n+2)(n+3)]`. Checked against simulation at
/// n = 64/128/512/2048/8192 (max relative error 4% at n = 64, under 1% above).
fn null_sd_index(n: usize) -> f64 {
    let nf = n as f64;
    nf / ((nf - 1.0) * (nf + 2.0) * (nf + 3.0)).sqrt()
}

/// `index` from a slice of gaps. Returns `None` if the slice is too short or
/// sums to nothing.
fn index_of(gaps: &[f64]) -> Option<f64> {
    let n = gaps.len();
    if n < 3 { return None; }
    let s: f64 = gaps.iter().sum();
    if !(s > 0.0) || !s.is_finite() { return None; }
    let sq: f64 = gaps.iter().map(|t| t * t).sum();
    let r = sq / (s * s);
    let nf = n as f64;
    let g2 = (nf + 1.0) / (nf - 1.0) * (nf * r - 1.0);
    Some(g2.max(0.0).sqrt())
}

/// Measure the arrival cadence of a bundle's `time` field.
pub fn cadence(
    engine: &Engine,
    name: &str,
    time: &str,
    block: Option<usize>,
) -> Result<CadenceResult, (StatusCode, String)> {
    let store = engine.bundle(name).ok_or_else(|| (
        StatusCode::NOT_FOUND, format!("Bundle '{}' not found", name)))?;
    let schema = store.schema();
    let has_field = |f: &str| schema.base_fields.iter()
        .chain(schema.fiber_fields.iter()).any(|d| d.name == f);
    if !has_field(time) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY,
                    format!("time field '{}' not found", time)));
    }
    let block_len = block.unwrap_or(DEFAULT_CADENCE_BLOCK);
    if block_len < MIN_CADENCE_BLOCK {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "block must be at least {} gaps (got {}); below that the per-block \
             index is mostly noise", MIN_CADENCE_BLOCK, block_len)));
    }

    let records: Vec<crate::types::Record> = store.records().collect();
    let total = records.len();
    if total > MAX_CADENCE_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "cadence caps at {} records (got {}); filter the bundle first",
            MAX_CADENCE_N, total)));
    }

    // CAD-13: non-numeric and non-finite stamps are SKIPPED and counted, never
    // coerced to 0.0 — a coerced stamp lands at the epoch and manufactures one
    // enormous gap, which is exactly the kind of confident wrong answer this
    // family exists to refuse.
    // Exact-integer stamps are rebased in i64 space BEFORE the f64 cast.
    // `Value::Timestamp` is a nanosecond epoch in this engine, and a 2026 value
    // (~1.75e18) sits in [2^60, 2^61) where the f64 ULP is exactly 256 — so the
    // cast alone merges every pair of events closer than 256 ns, and the
    // `dedup()` below would then report that merge as "shared a timestamp",
    // which is a statement about the caller's data that would not be true.
    // Sub-microsecond bursts are exactly what this verb is for, so losing them
    // silently on the engine's highest-resolution stamp type is not acceptable.
    //
    // Rebasing is the origin half of the affine map CAD-3 already proves cannot
    // move anything measured, and `t - t_min` is exact in f64 for any span under
    // 2^53 units. Float feeds fall through unchanged — `as_i64()` returns None
    // for `Value::Float`, so every existing fixture takes the original path.
    let ints: Vec<i64> = records.iter()
        .filter_map(|r| r.get(time).and_then(|v| v.as_i64()))
        .collect();
    let n_float_usable = records.iter()
        .filter_map(|r| r.get(time).and_then(|v| v.as_f64()))
        .filter(|v| v.is_finite())
        .count();
    let mut stamps: Vec<f64> = if !ints.is_empty() && ints.len() == n_float_usable {
        let base = *ints.iter().min().unwrap() as i128;
        ints.iter().map(|v| (*v as i128 - base) as f64).collect()
    } else {
        records.iter()
            .filter_map(|r| r.get(time).and_then(|v| v.as_f64()))
            .filter(|v| v.is_finite())
            .collect()
    };
    let skipped = total - stamps.len();
    stamps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Coalescing is mandatory, not an option. Events sharing a stamp are one
    // observation of the clock; treating them as separate manufactures zero
    // gaps that drag `index` upward without any burst having occurred.
    let before = stamps.len();
    stamps.dedup();
    let coalesced = before - stamps.len();
    let n_events = stamps.len();

    if n_events < MIN_CADENCE_N {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "cadence needs at least {} distinct usable timestamps in '{}' \
             (got {} from {} records)",
            MIN_CADENCE_N, time, n_events, total)));
    }

    let gaps: Vec<f64> = stamps.windows(2).map(|w| w[1] - w[0]).collect();
    let n = gaps.len();

    // Quantum: the smallest positive gap. Deliberately NOT a gcd — a float gcd
    // collapses toward zero the moment one stamp is not an exact multiple of
    // the quantum, which makes the granularity gate pass everything. That is a
    // silent failure in the permissive direction.
    let q = gaps.iter().cloned().filter(|g| *g > 0.0)
        .fold(f64::INFINITY, f64::min);
    let mut sorted = gaps.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = sorted[n / 2];

    // CAD-10: perfectly uniform spacing. Almost always a row index passed where
    // a clock belongs. `index` would be exactly 0 — a maximally damning verdict
    // on a stream that was never measured at all.
    if !(median > 0.0) || (sorted[n - 1] - sorted[0]).abs() <= 0.0 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "every gap in '{}' is identical ({}). That is a counter, not a \
             clock — cadence needs the real arrival times. If you meant record \
             order, you want TEXTURE or PRECEDENCE, which read order by design.",
            time, median)));
    }
    // Is `q` actually a clock QUANTUM, or just the smallest draw from a gap law
    // that happens to be bounded away from zero? Only the former justifies
    // refusing. If the stamps really are quantised, every gap is an exact
    // multiple of the quantum; if they are not, they are not.
    let off_lattice = gaps.iter()
        .map(|g| { let r = g / q; (r - r.round()).abs() })
        .fold(0.0f64, f64::max);
    let quantised = off_lattice <= 1e-6;

    // CAD-9: too coarse to measure. Refuse rather than report the bias.
    //
    // The `quantised` guard is load-bearing and was missing in the first draft.
    // `median/q` is a property of the gap DISTRIBUTION, not of clock
    // resolution: for any metered stream — gaps bounded away from zero, which
    // is what a poll interval or a rate limiter produces — the ratio is a small
    // constant no matter how fine the clock is. A 100 ms poller with +-5%
    // jitter, stamped in f64 seconds with a 1e-17 ULP, has median/q = 1.05 and
    // was being refused as "too coarsely stamped", which is a false statement
    // about its clock. Worse, its true reading is index = 0.029 — deeply
    // THROTTLED — so the unguarded gate suppressed precisely the feed-quality
    // alarm this verb exists to raise. Measured: refused at every jitter level
    // from 5% to 90%. The guard passes all of them (off-lattice 0.105..0.500)
    // and still refuses a genuine 1 s grid (off-lattice 0.000000).
    if quantised && median < CADENCE_MIN_GRANULARITY * q {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "'{}' is too coarsely stamped to measure: median gap {:.6} is only \
             {:.1}x the smallest gap {:.6}, and cadence needs {:.0}x. Below \
             that the discretisation bias exceeds the measurement and flips \
             sign, reporting a coarse feed as an unusually regular one.",
            time, median, median / q, q, CADENCE_MIN_GRANULARITY)));
    }
    // CAD-16: the gate above keys on the SMALLEST gap, which is one record's
    // opinion — a single off-grid stamp collapses `q` and opens the gate on a
    // feed that is otherwise entirely quantised. Measured: a bursty stream with
    // a true index of 3.611, rounded onto a 1 s grid, reads 0.912 (STEADY) — the
    // sign of the answer inverted — and one injected record at 0.001 takes
    // median/q from 2.0 to 2000 and lets it through.
    //
    // So key the second check on something no single record can move: a
    // quantised clock puts its gaps on a LATTICE, and lattice gaps repeat
    // exactly. Floats off a real clock do not. Measured modal fraction — the
    // share of gaps exactly equal to the commonest gap — is 0.22..0.33 on a
    // coarse feed regardless of how many off-grid records are injected (still
    // 0.221 at +100), against 0.000..0.016 on Poisson and lognormal at
    // sigma = 1.0 and 2.0, where 1/n is the arithmetic floor.
    //
    // Note this is NOT the tie-fraction gate rejected upstream: that one keyed
    // on duplicate TIMESTAMPS and was rate-dependent, and its counterexample —
    // a slow book on a 1 s feed passing at a duplicate fraction of 0.397 and
    // then reporting the opposite sign — is refused here, because its gaps are
    // lattice-valued whatever its duplicate fraction is.
    let mut counts: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    for g in &gaps {
        *counts.entry(g.to_bits()).or_insert(0) += 1;
    }
    let modal = counts.values().copied().max().unwrap_or(0);
    let modal_frac = modal as f64 / n as f64;
    if modal_frac > CADENCE_MAX_LATTICE_FRAC {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, format!(
            "'{}' is quantised: {:.1}% of gaps are exactly equal to one another, \
             so the arrival times sit on a lattice rather than on a clock \
             cadence can measure. A real clock does not repeat a gap to the bit. \
             Measured on a rounded feed this reads as unusually REGULAR when the \
             underlying stream is in fact bursty, so it is refused rather than \
             reported.",
            time, 100.0 * modal_frac)));
    }

    let index = index_of(&gaps).ok_or_else(|| (
        StatusCode::UNPROCESSABLE_ENTITY,
        format!("gaps in '{}' sum to zero; nothing to measure", time)))?;
    let null_sd = null_sd_index(n);
    let index_z = (index - 1.0) / null_sd;

    // Blocked form: mean per-block index over contiguous blocks. This removes
    // drift in the arrival rate, and by the same mechanism removes any
    // structure longer than one block — it is a high-pass filter with its
    // corner at `block_len`, not a neutral cleanup. Measured attenuation for
    // bursts of length L at block 64: L=4 keeps 98%, L=64 keeps 63%,
    // L=256 keeps 32%.
    let n_blocks = n / block_len;
    let (index_blocked, block_note) = if n_blocks >= 2 {
        let mut acc = 0.0;
        let mut k = 0usize;
        for b in 0..n_blocks {
            if let Some(g) = index_of(&gaps[b * block_len..(b + 1) * block_len]) {
                acc += g; k += 1;
            }
        }
        if k > 0 {
            (acc / k as f64, format!(
                "blocked index over {} blocks of {} gaps ({} gaps dropped as \
                 remainder); filters out structure longer than one block",
                k, block_len, n - n_blocks * block_len))
        } else {
            (f64::NAN, "blocked index unavailable".to_string())
        }
    } else {
        (f64::NAN, format!(
            "blocked index needs 2 blocks of {} gaps; only {} gaps available, \
             so rate drift cannot be separated here", block_len, n))
    };

    // Memory: lag-1 autocorrelation of log gaps. Orthogonal to `index` by
    // construction — `index` is symmetric in the gaps, this is not.
    let logs: Vec<f64> = gaps.iter().map(|g| g.max(f64::MIN_POSITIVE).ln()).collect();
    let lbar = logs.iter().sum::<f64>() / n as f64;
    let c: Vec<f64> = logs.iter().map(|l| l - lbar).collect();
    let denom: f64 = c.iter().map(|x| x * x).sum();
    let memory = if denom > 0.0 {
        c.windows(2).map(|w| w[0] * w[1]).sum::<f64>() / denom
    } else { 0.0 };
    let memory_z = memory * (n as f64).sqrt();

    let verdict = if index_z > 2.0 { "BURSTY" }
                  else if index_z < -2.0 { "THROTTLED" }
                  else { "STEADY" };
    let memory_verdict = if memory_z > 2.0 { "PERSISTENT" }
                         else if memory_z < -2.0 { "ALTERNATING" }
                         else { "MEMORYLESS" };

    let reads = vec![
        match verdict {
            "BURSTY" => format!(
                "index = {:.3} ({:+.1} sd): arrivals are UNEVEN — the stream \
                 comes in gulps rather than a steady drip.", index, index_z),
            "THROTTLED" => format!(
                "index = {:.3} ({:+.1} sd): arrivals are MORE REGULAR than \
                 random. Real sources rarely are; suspect something metering \
                 the stream — a poll interval, a rate limiter, a fixed flush.",
                index, index_z),
            _ => format!(
                "index = {:.3} ({:+.1} sd): indistinguishable from memoryless \
                 arrivals. This is the ordinary state.", index, index_z),
        },
        match memory_verdict {
            "PERSISTENT" => format!(
                "memory = {:+.3} ({:+.1} sd): the spacing has STATE — quiet \
                 follows quiet, busy follows busy. The stream stays in a mode \
                 rather than resampling it each time.", memory, memory_z),
            "ALTERNATING" => format!(
                "memory = {:+.3} ({:+.1} sd): the spacing ALTERNATES — a long \
                 gap tends to be followed by a short one. Often a queue \
                 draining in batches.", memory, memory_z),
            _ => format!(
                "memory = {:+.3} ({:+.1} sd): no detectable memory in the \
                 spacing. Any unevenness above is in the distribution of gaps, \
                 not in their order.", memory, memory_z),
        },
        format!(
            "These two are independent by construction: `index` cannot see the \
             ORDER of the gaps and `memory` cannot see their SPREAD. High index \
             with no memory is a heavy-tailed but forgetful stream; both \
             together is one that clusters."),
    ];

    let mut notes = vec![
        format!("{} gaps from {} distinct timestamps in '{}'", n, n_events, time),
        format!("null sd of index at n = {} is {:.4}; verdict bands are the \
                 memoryless null +-2 sd, derived not chosen", n, null_sd),
        block_note,
        format!("median gap {:.6} is {:.0}x the smallest gap {:.6} (needs {:.0}x)",
                median, median / q, q, CADENCE_MIN_GRANULARITY),
    ];
    if coalesced > 0 {
        notes.push(format!(
            "{} of {} events shared a timestamp with another and were coalesced \
             ({:.1}%); duplicate stamps are one observation of the clock, not a \
             zero-length gap", coalesced, before, 100.0 * coalesced as f64 / before as f64));
    }
    if skipped > 0 {
        notes.push(format!(
            "{} of {} records skipped: '{}' missing, non-numeric or non-finite \
             (skipped, never coerced to 0)", skipped, total, time));
    }
    if !index_blocked.is_nan() && index > 0.0 {
        notes.push(format!(
            "index/index_blocked = {:.2}; above 1 means some of the unevenness \
             lives at a scale LONGER than {} gaps. That is drift and slow \
             structure together — this ratio cannot separate them.",
            index / index_blocked, block_len));
    }

    Ok(CadenceResult {
        time_field: time.to_string(), n_events, n_gaps: n,
        index, null_sd, index_z, index_blocked, block_len,
        n_blocks: if index_blocked.is_nan() { 0 } else { n_blocks },
        memory, memory_z,
        verdict: verdict.to_string(),
        memory_verdict: memory_verdict.to_string(),
        reads, notes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ml::test_support::{cleanup, scan_env, scan_rec};
    use crate::types::{BundleSchema, FieldDef, Record, Value as V};

    /// Deterministic uniform stream — reproducible byte-for-byte, no dep.
    fn unif_stream(seed: u64) -> impl FnMut() -> f64 {
        let mut s = seed | 1;
        move || {
            s ^= s << 13; s ^= s >> 7; s ^= s << 17;
            ((s >> 11) as f64 / (1u64 << 53) as f64).clamp(1e-12, 1.0 - 1e-12)
        }
    }

    /// Memoryless arrivals at rate `lam` — the exact null.
    fn poisson_stamps(n: usize, lam: f64, seed: u64) -> Vec<f64> {
        let mut u = unif_stream(seed);
        let mut t = 0.0f64;
        (0..n).map(|_| { t += -u().ln() / lam; t }).collect()
    }

    /// Independent gaps with a heavy-tailed marginal and NO memory: bursty by
    /// spread alone. This is the fixture that separates the two numbers.
    fn lognormal_stamps(n: usize, sigma: f64, seed: u64) -> Vec<f64> {
        let mut u = unif_stream(seed);
        let mut t = 0.0f64;
        (0..n).map(|_| {
            let (u1, u2) = (u(), u());
            let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            t += (sigma * z).exp(); t
        }).collect()
    }

    /// Strictly PERIODIC bursts: `burst` fast events, then one long quiet gap,
    /// repeating. Bursty, and — counter-intuitively — *anti*-correlated at lag
    /// 1, because each long gap is bracketed by short ones and dominates the
    /// log-space sum. The first draft of these gates asserted this fixture was
    /// persistent; it is not, and CAD-6 caught it (z = -6.8). Kept deliberately
    /// as the ALTERNATING fixture, because a regular burst rhythm is a real
    /// stream shape and `memory` should report its sign correctly.
    fn clustered_stamps(n: usize, burst: usize, seed: u64) -> Vec<f64> {
        let mut u = unif_stream(seed);
        let mut t = 0.0f64;
        let mut out = Vec::with_capacity(n);
        while out.len() < n {
            for _ in 0..burst { t += -u().ln() * 0.02; out.push(t); }
            t += -u().ln() * 4.0;
        }
        out.truncate(n); out
    }

    /// Markov-modulated arrivals: the stream switches between a fast mode and a
    /// slow one and STAYS in it for ~50 events. Long gaps genuinely follow long
    /// gaps. This is what `memory > 0` actually means — a stream with a state,
    /// as opposed to a periodic rhythm (above) or a heavy tail (below).
    fn modulated_stamps(n: usize, seed: u64) -> Vec<f64> {
        let mut u = unif_stream(seed);
        let mut t = 0.0f64;
        let mut fast = true;
        (0..n).map(|_| {
            if u() < 0.02 { fast = !fast; }
            t += -u().ln() / if fast { 10.0 } else { 0.5 };
            t
        }).collect()
    }

    fn schema_for(name: &str) -> BundleSchema {
        BundleSchema::new(name)
            .base(FieldDef::categorical("id"))
            .fiber(FieldDef::numeric("ts"))
            .fiber(FieldDef::categorical("label"))
    }

    fn rows(stamps: &[f64]) -> Vec<Record> {
        stamps.iter().enumerate().map(|(i, t)| scan_rec(&[
            ("id", V::Text(format!("r{i:06}"))),
            ("ts", V::Float(*t)),
            ("label", V::Text("x".into())),
        ])).collect()
    }

    fn run(tag: &str, stamps: &[f64]) -> (std::path::PathBuf, CadenceResult) {
        let (dir, engine) = scan_env(tag, "s", schema_for("s"), rows(stamps));
        let r = cadence(&engine, "s", "ts", None).expect("cadence");
        (dir, r)
    }

    /// CAD-1: the null is calibrated. Memoryless arrivals must read index ~ 1.
    #[test]
    fn cad_1_null_is_calibrated() {
        let (dir, r) = run("cad_null", &poisson_stamps(4096, 1.0, 7));
        assert!(r.index_z.abs() < 3.0,
                "memoryless stream must read near 1, got index={} z={}",
                r.index, r.index_z);
        assert_eq!(r.verdict, "STEADY", "index={} z={}", r.index, r.index_z);
        cleanup(&dir);
    }

    /// CAD-2: rate invariance — the whole product rests on this. Identical
    /// arrivals at 1000x the rate must give the SAME index, not merely a close
    /// one, because the Dirichlet null is exactly free of the rate.
    #[test]
    fn cad_2_rate_invariant() {
        let slow = poisson_stamps(2048, 1.0, 11);
        let fast: Vec<f64> = slow.iter().map(|t| t / 1000.0).collect();
        let (d1, a) = run("cad_rate_a", &slow);
        let (d2, b) = run("cad_rate_b", &fast);
        assert!((a.index - b.index).abs() < 1e-9,
                "rate changed the index: {} vs {}", a.index, b.index);
        assert!((a.memory - b.memory).abs() < 1e-9,
                "rate changed memory: {} vs {}", a.memory, b.memory);
        cleanup(&d1); cleanup(&d2);
    }

    /// CAD-3: affine invariance. `t -> a*t + b` is a change of clock units and
    /// origin; nothing measured here may move.
    #[test]
    fn cad_3_affine_invariant() {
        let base = poisson_stamps(2048, 1.0, 13);
        let shifted: Vec<f64> = base.iter().map(|t| t * 7.5 + 1_600_000_000.0).collect();
        let (d1, a) = run("cad_aff_a", &base);
        let (d2, b) = run("cad_aff_b", &shifted);
        assert!((a.index - b.index).abs() < 1e-9,
                "affine moved index: {} vs {}", a.index, b.index);
        cleanup(&d1); cleanup(&d2);
    }

    /// CAD-4: bursty streams read bursty, and the ordering is recovered.
    #[test]
    fn cad_4_detects_burstiness() {
        let (d1, steady) = run("cad_b_steady", &poisson_stamps(4096, 1.0, 17));
        let (d2, bursty) = run("cad_b_burst", &clustered_stamps(4096, 8, 19));
        assert!(bursty.index > steady.index,
                "burst must exceed steady: {} vs {}", bursty.index, steady.index);
        assert_eq!(bursty.verdict, "BURSTY", "index={}", bursty.index);
        cleanup(&d1); cleanup(&d2);
    }

    /// CAD-5 — THE GATE THIS VERB EXISTS FOR.
    ///
    /// `index` is a symmetric function of the gaps, so it cannot distinguish a
    /// clustered stream from one with the same gaps in a scrambled order. If
    /// only `index` shipped, "detects clustering" would be a claim no fixture
    /// could support. `memory` must separate them, and `index` must NOT.
    #[test]
    fn cad_5_memory_separates_what_index_cannot() {
        // Three streams that `index` sees as the same KIND of thing — all
        // bursty — and that `memory` must tell apart three ways.
        //   heavy: independent gaps, heavy-tailed marginal  -> no memory
        //   burst: strictly periodic burst rhythm -> alternating
        //   mod:   mode-switching, stays in a mode -> persistent
        let (d1, heavy) = run("cad_m_heavy", &lognormal_stamps(4096, 1.0, 23));
        let (d2, burst) = run("cad_m_burst", &clustered_stamps(4096, 8, 29));
        let (d3, moded) = run("cad_m_mod", &modulated_stamps(4096, 83));

        // Bursty by the DERIVED band, not by a number anyone chose.
        for (tag, r) in [("heavy", &heavy), ("burst", &burst), ("mod", &moded)] {
            assert_eq!(r.verdict, "BURSTY",
                       "{tag} must read bursty: index={} z={}", r.index, r.index_z);
        }

        // `index` cannot separate them. `memory` must, in all three directions.
        assert_eq!(heavy.memory_verdict, "MEMORYLESS",
                   "independent gaps must show no memory: {} (z={})",
                   heavy.memory, heavy.memory_z);
        assert_eq!(burst.memory_verdict, "ALTERNATING",
                   "a periodic burst rhythm alternates at lag 1: {} (z={})",
                   burst.memory, burst.memory_z);
        assert_eq!(moded.memory_verdict, "PERSISTENT",
                   "a mode-switching stream must show memory: {} (z={})",
                   moded.memory, moded.memory_z);
        cleanup(&d1); cleanup(&d2); cleanup(&d3);
    }

    /// CAD-6: and the converse — `index` is provably blind to order. Shuffling
    /// the gaps must leave it BIT-IDENTICAL while destroying `memory`. This
    /// pins the orthogonality claim rather than asserting it.
    #[test]
    fn cad_6_index_is_blind_to_order_memory_is_not() {
        let stamps = modulated_stamps(4096, 31);
        let gaps: Vec<f64> = stamps.windows(2).map(|w| w[1] - w[0]).collect();

        // Deterministic Fisher-Yates on the gaps, then rebuild the stamps.
        let mut u = unif_stream(37);
        let mut sh = gaps.clone();
        for i in (1..sh.len()).rev() {
            let j = (u() * (i + 1) as f64) as usize;
            sh.swap(i, j.min(i));
        }
        let mut t = stamps[0];
        let mut shuffled = vec![t];
        for g in &sh { t += *g; shuffled.push(t); }

        let (d1, a) = run("cad_ord_a", &stamps);
        let (d2, b) = run("cad_ord_b", &shuffled);

        assert!((a.index - b.index).abs() < 1e-9,
                "index is symmetric in the gaps and must not move under a \
                 shuffle: {} vs {}", a.index, b.index);
        assert!(a.memory_z > 2.0, "ordered stream should have memory: {}", a.memory_z);
        assert!(b.memory_z.abs() < 2.0,
                "shuffling must destroy memory: {} (z={})", b.memory, b.memory_z);
        cleanup(&d1); cleanup(&d2);
    }

    /// CAD-7: a metered stream reads THROTTLED. This is the feed-quality alarm
    /// — a fixed release grid is the failure that a bar-count statistic hides.
    #[test]
    fn cad_7_detects_a_metered_stream() {
        // Events generated irregularly but released on a fixed 1.0 grid.
        let raw = poisson_stamps(4096, 5.0, 41);
        let gridded: Vec<f64> = raw.iter().map(|t| t.ceil()).collect();
        let (dir, engine) = scan_env("cad_grid", "s", schema_for("s"), rows(&gridded));
        // This fixture is QUANTISED, so the required outcome is a refusal that
        // names quantisation. The first draft of this gate accepted *either* a
        // refusal or a THROTTLED verdict, and that escape hatch is exactly what
        // hid CAD-17: with both branches acceptable, the suite could not tell
        // "refused because the clock is coarse" from "reported because the
        // stream is regular", and the latter had become unreachable.
        let err = cadence(&engine, "s", "ts", None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("quantised") || err.1.contains("coarsely stamped"),
                "a release grid must be refused by name: {}", err.1);
        cleanup(&dir);
    }

    /// CAD-8: coalescing. Duplicating every timestamp must leave the answer
    /// unchanged, because duplicates are one observation of the clock.
    #[test]
    fn cad_8_coalesces_duplicate_stamps() {
        let stamps = poisson_stamps(2048, 1.0, 43);
        let mut doubled: Vec<f64> = Vec::with_capacity(stamps.len() * 2);
        for t in &stamps { doubled.push(*t); doubled.push(*t); }
        let (d1, a) = run("cad_dup_a", &stamps);
        let (d2, b) = run("cad_dup_b", &doubled);
        assert!((a.index - b.index).abs() < 1e-9,
                "duplicate stamps changed the index: {} vs {}", a.index, b.index);
        assert!(b.notes.iter().any(|s| s.contains("coalesced")),
                "the coalescing must be disclosed: {:?}", b.notes);
        cleanup(&d1); cleanup(&d2);
    }

    /// CAD-9: too coarse to measure → refuse, and say by how much.
    #[test]
    fn cad_9_refuses_a_coarse_clock() {
        // Median gap ~2 quanta: far under the 10x the estimator needs.
        let raw = poisson_stamps(2048, 0.5, 47);
        let coarse: Vec<f64> = raw.iter().map(|t| t.round()).collect();
        let (dir, engine) = scan_env("cad_coarse", "s", schema_for("s"), rows(&coarse));
        let err = cadence(&engine, "s", "ts", None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("coarsely stamped"), "{}", err.1);
        cleanup(&dir);
    }

    /// CAD-10: a row index passed where a clock belongs. The trap this verb's
    /// parameter name exists to avoid — must refuse BY NAME, never report the
    /// index of 0 that a uniform grid mathematically produces.
    #[test]
    fn cad_10_refuses_a_counter_masquerading_as_a_clock() {
        let counter: Vec<f64> = (0..2048).map(|i| i as f64).collect();
        let (dir, engine) = scan_env("cad_counter", "s", schema_for("s"), rows(&counter));
        let err = cadence(&engine, "s", "ts", None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("counter, not a clock"), "{}", err.1);
        assert!(err.1.contains("TEXTURE"), "must point at the right verb: {}", err.1);
        cleanup(&dir);
    }

    /// CAD-16: one off-grid record must not open a coarse feed.
    ///
    /// The granularity gate keys on the smallest gap, which is a single
    /// record's opinion. Inject one stamp 0.001 off a 1 s grid and `median/q`
    /// goes from 2.0 to 2000, sailing through CAD-9 — and the verb then reports
    /// a *bursty* stream (true index 3.611) as index 0.912, which lands in the
    /// THROTTLED band. The sign of the answer inverts. Found by adversarial
    /// review of the first 15 gates, which did not cover it.
    #[test]
    fn cad_16_one_fine_record_cannot_open_a_coarse_feed() {
        let coarse: Vec<f64> = clustered_stamps(4096, 8, 19)
            .iter().map(|t| t.ceil()).collect();

        // Clean coarse feed: CAD-9 already refuses this one.
        let (d1, e1) = scan_env("cad_c_clean", "s", schema_for("s"), rows(&coarse));
        let clean = cadence(&e1, "s", "ts", None).unwrap_err();
        assert_eq!(clean.0, StatusCode::UNPROCESSABLE_ENTITY);
        cleanup(&d1);

        // Same feed plus ONE record off the grid. Must still refuse.
        let mut uniq = coarse.clone();
        uniq.sort_by(|a, b| a.partial_cmp(b).unwrap());
        uniq.dedup();
        let mut injected = coarse.clone();
        injected.push(uniq[0] + 0.001);
        let (d2, e2) = scan_env("cad_c_inj", "s", schema_for("s"), rows(&injected));
        let err = cadence(&e2, "s", "ts", None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY,
                   "one off-grid record must not open a quantised feed");
        assert!(err.1.contains("quantised") || err.1.contains("coarsely stamped"),
                "refusal must name the reason: {}", err.1);
        cleanup(&d2);

        // And the lattice check must not fire on a legitimate continuous feed.
        let (d3, ok) = run("cad_c_cont", &poisson_stamps(4096, 1.0, 97));
        assert!(ok.index.is_finite());
        cleanup(&d3);
    }

    /// CAD-17 — THE THROTTLED VERDICT MUST BE REACHABLE.
    ///
    /// A metered stream on a perfectly fine clock: a 100 ms poller with jitter,
    /// stamped in f64 seconds. There is no quantisation whatsoever. Its true
    /// reading is deeply sub-Poisson, and reporting that is the single most
    /// commercially useful thing this verb does — it is the feed-quality alarm.
    ///
    /// This gate exists because the granularity refusal, written without a
    /// quantisation guard, made `THROTTLED` **structurally unreachable**:
    /// `median/q` is small for ANY gap law bounded away from zero, so every
    /// metered feed was refused as "too coarsely stamped" — a false statement
    /// about a clock with a 1e-17 ULP. Measured refusal at 5%, 30%, 70% and 90%
    /// jitter before the fix.
    #[test]
    fn cad_17_a_metered_stream_on_a_fine_clock_reads_throttled() {
        for (label, jitter) in [("tight", 0.05f64), ("loose", 0.30)] {
            let mut u = unif_stream(101);
            let mut t = 0.0f64;
            let stamps: Vec<f64> = (0..4096).map(|_| {
                t += 0.1 * (1.0 - jitter + 2.0 * jitter * u());
                t
            }).collect();
            let (dir, engine) = scan_env(
                &format!("cad_poll_{label}"), "s", schema_for("s"), rows(&stamps));
            let r = cadence(&engine, "s", "ts", None).unwrap_or_else(|e| panic!(
                "a {label} poller on an f64-seconds clock must be MEASURED, not \
                 refused — the clock is not coarse, the stream is regular: {}", e.1));
            assert_eq!(r.verdict, "THROTTLED",
                       "{label} poller must read THROTTLED: index={} z={}",
                       r.index, r.index_z);
            assert!(r.index < 1.0, "index={}", r.index);
            cleanup(&dir);
        }
    }

    /// CAD-18: nanosecond epoch stamps must keep their structure.
    ///
    /// `Value::Timestamp` is a nanosecond epoch integer here. A 2026 value
    /// (~1.75e18) lies in [2^60, 2^61) where the f64 ULP is 256, so a naive
    /// cast merges every pair of events closer than 256 ns — and the coalescing
    /// step would then report that as the caller's duplicate stamps. Bursts at
    /// sub-microsecond spacing are exactly what this verb is for.
    #[test]
    fn cad_18_nanosecond_epoch_stamps_survive_the_f64_cast() {
        const BASE: i64 = 1_750_000_000_000_000_000;
        let mut u = unif_stream(103);
        let mut t_ns: i64 = 0;
        let mut ns: Vec<i64> = Vec::new();
        for i in 0..3000 {
            t_ns += (-u().ln() * 300_000.0) as i64; // ~300 us mean
            ns.push(BASE + t_ns);
            // A sweep at ~50 ns spacing, JITTERED. The first draft used exactly
            // 50 ns and CAD-16 refused it at an 11.1% lattice fraction — the
            // fixture, not the gate, was wrong: a real venue does not emit
            // trades at a bit-identical spacing, and a fixture that does is
            // testing the lattice check rather than the precision it claims to.
            if i % 40 == 0 {
                for _ in 0..5 {
                    t_ns += 40 + (u() * 20.0) as i64;
                    ns.push(BASE + t_ns);
                }
            }
        }
        let recs: Vec<Record> = ns.iter().enumerate().map(|(i, t)| scan_rec(&[
            ("id", V::Text(format!("r{i:06}"))),
            ("ts", V::Timestamp(*t)),
            ("label", V::Text("x".into())),
        ])).collect();
        let total = recs.len();
        let (dir, engine) = scan_env("cad_nanos", "s", schema_for("s"), recs);
        let r = cadence(&engine, "s", "ts", None).expect("cadence on nanos");
        assert_eq!(r.n_events, total,
                   "no event may be lost to f64 precision: {} of {}", r.n_events, total);
        assert!(!r.notes.iter().any(|s| s.contains("coalesced")),
                "nothing shared a stamp; coalescing must not be reported: {:?}", r.notes);
        assert_eq!(r.verdict, "BURSTY",
                   "the 50 ns sweeps must survive: index={}", r.index);
        cleanup(&dir);
    }

    /// CAD-19: the GQL surface parses to the same request the REST surface
    /// takes, and produces the SAME numbers. Both call this kernel, so what is
    /// gated here is the parse and the plumbing, not the maths twice.
    #[test]
    fn cad_19_gql_surface_matches_rest() {
        use crate::parser::{execute, parse, ExecResult, Statement};

        let stamps = modulated_stamps(4096, 107);
        let (dir, mut engine) = scan_env(
            "cad_gql", "s", schema_for("s"), rows(&stamps));

        // Parse: the shape the grammar produces.
        let stmt = parse("CADENCE s ON ts;").expect("parse CADENCE");
        match &stmt {
            Statement::Cadence { bundle, time, block } => {
                assert_eq!(bundle, "s");
                assert_eq!(time, "ts");
                assert_eq!(*block, None, "block must default, not be invented");
            }
            other => panic!("CADENCE parsed to the wrong statement: {other:?}"),
        }

        // Execute: identical numbers to calling the kernel directly.
        let direct = cadence(&engine, "s", "ts", None).expect("direct");
        let rows = match execute(&mut engine, &stmt).expect("execute CADENCE") {
            ExecResult::Rows(r) => r,
            other => panic!("CADENCE must return rows, got {other:?}"),
        };
        assert_eq!(rows.len(), 1, "one-row envelope");
        let got = |k: &str| rows[0].get(k).and_then(|v| v.as_f64()).unwrap();
        assert_eq!(got("index").to_bits(), direct.index.to_bits(),
                   "GQL and the kernel must not differ by a bit");
        assert_eq!(got("memory").to_bits(), direct.memory.to_bits());
        assert_eq!(
            rows[0].get("verdict").map(|v| format!("{v}")),
            Some(direct.verdict.clone()),
            "verdict must match the REST surface");

        // WITH BLOCK is honoured, not parsed and dropped.
        let blocked = parse("CADENCE s ON ts WITH BLOCK = 128;").expect("parse WITH");
        match &blocked {
            Statement::Cadence { block, .. } => assert_eq!(*block, Some(128)),
            other => panic!("{other:?}"),
        }

        // ALONG is refused by name — CADENCE has no ordering field.
        let err = parse("CADENCE s ON ts ALONG seq;").unwrap_err();
        assert!(err.contains("no ALONG"), "must explain why: {err}");
        assert!(err.contains("TEXTURE"), "must point at the right verb: {err}");

        // Refusals survive the GQL surface rather than becoming empty rows.
        let ghost = parse("CADENCE s ON not_a_column;").expect("parses fine");
        let e = execute(&mut engine, &ghost).unwrap_err();
        assert!(e.contains("not_a_column"), "must name the field: {e}");

        cleanup(&dir);
    }

    /// CAD-11 / CAD-12: refusals name what is wrong.
    #[test]
    fn cad_11_12_refusals_name_the_problem() {
        let (dir, engine) = scan_env(
            "cad_ref", "s", schema_for("s"), rows(&poisson_stamps(512, 1.0, 53)));

        let miss = cadence(&engine, "no_such", "ts", None).unwrap_err();
        assert_eq!(miss.0, StatusCode::NOT_FOUND);
        assert!(miss.1.contains("no_such"), "must name the bundle: {}", miss.1);

        let f = cadence(&engine, "s", "ghost", None).unwrap_err();
        assert_eq!(f.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(f.1.contains("ghost"), "must name the field: {}", f.1);

        // Non-numeric: yields no usable stamps at all.
        let t = cadence(&engine, "s", "label", None).unwrap_err();
        assert_eq!(t.0, StatusCode::UNPROCESSABLE_ENTITY);

        let b = cadence(&engine, "s", "ts", Some(4)).unwrap_err();
        assert_eq!(b.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(b.1.contains("block must be at least"), "{}", b.1);
        cleanup(&dir);
    }

    /// CAD-13: too few events → refuse, reporting the actual count.
    #[test]
    fn cad_13_refuses_thin_data() {
        let (dir, engine) = scan_env(
            "cad_thin", "s", schema_for("s"), rows(&poisson_stamps(20, 1.0, 59)));
        let err = cadence(&engine, "s", "ts", None).unwrap_err();
        assert_eq!(err.0, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(err.1.contains("20"), "must report the actual count: {}", err.1);
        cleanup(&dir);
    }

    /// CAD-14: same input, same bits.
    #[test]
    fn cad_14_deterministic() {
        let stamps = poisson_stamps(2048, 1.0, 61);
        let (dir, engine) = scan_env("cad_det", "s", schema_for("s"), rows(&stamps));
        let a = cadence(&engine, "s", "ts", None).unwrap();
        let b = cadence(&engine, "s", "ts", None).unwrap();
        assert_eq!(a.index.to_bits(), b.index.to_bits());
        assert_eq!(a.memory.to_bits(), b.memory.to_bits());
        assert_eq!(a.verdict, b.verdict);
        cleanup(&dir);
    }

    /// CAD-15: the blocked form is a high-pass filter, and must be described as
    /// one. Bursts SHORTER than a block survive it; bursts LONGER are
    /// attenuated. Asserting the direction stops anyone reading
    /// `index/index_blocked` as a clean drift-vs-clustering split.
    #[test]
    fn cad_15_blocking_attenuates_long_structure() {
        let (d1, short) = run("cad_blk_s", &clustered_stamps(4096, 4, 67));
        let (d2, long) = run("cad_blk_l", &clustered_stamps(4096, 256, 71));
        let keep_short = short.index_blocked / short.index;
        let keep_long = long.index_blocked / long.index;
        assert!(keep_short > keep_long,
                "blocking must cost long structure more than short: {} vs {}",
                keep_short, keep_long);
        assert!(short.n_blocks > 0 && long.n_blocks > 0);
        cleanup(&d1); cleanup(&d2);
    }

    /// Every answer carries a plain-English reading of BOTH numbers, and the
    /// note that they are independent. A caller must not have to read the paper
    /// to know that a high index with no memory is not clustering.
    #[test]
    fn cad_reads_are_populated_and_explain_orthogonality() {
        let (dir, r) = run("cad_reads", &clustered_stamps(2048, 8, 73));
        assert!(r.reads.len() >= 3, "{:?}", r.reads);
        assert!(r.reads[0].contains("index ="), "{:?}", r.reads);
        assert!(r.reads[1].contains("memory ="), "{:?}", r.reads);
        assert!(r.reads[2].contains("independent"), "{:?}", r.reads);
        assert!(["BURSTY", "STEADY", "THROTTLED"].contains(&r.verdict.as_str()));
        assert!(["PERSISTENT", "MEMORYLESS", "ALTERNATING"]
                .contains(&r.memory_verdict.as_str()));
        assert!(r.notes.iter().any(|s| s.contains("derived not chosen")));
        cleanup(&dir);
    }

}
