//! GIGI Encrypt v0.3.x — Gauge-equivariant aggregate inversion.
//!
//! Server-side aggregate computations (SUM, AVG, MIN, MAX, VARIANCE, ...)
//! produce *gauge-equivariant* scalars under deterministic-bijective
//! gauge modes (Affine, Probabilistic numeric). The client, who holds
//! the gauge key, applies a closed-form inverse to recover the
//! plaintext aggregate. The record set never leaves ciphertext form on
//! the server; only one scalar per query is post-processed at the
//! client.
//!
//! This module ships the closed-form inverses for the standard SQL
//! analytical aggregates. **No FHE-style ciphertext arithmetic is
//! required**; the server uses its native sum/min/max primitives and
//! the client applies a single affine inversion per query.
//!
//! ## Mathematical relationships (Affine mode)
//!
//! For a numeric fiber field with affine gauge $g(v) = a \cdot v + b$
//! ($a \neq 0$), the encrypted-side aggregate maps to the plaintext
//! aggregate as follows:
//!
//! | Aggregate | $f(\text{ciphertext})$ | $f(\text{plaintext})$ via |
//! |---|---|---|
//! | COUNT | $n$ | $n$ (gauge-invariant) |
//! | SUM | $a \cdot \mathrm{SUM}_{\text{plain}} + n b$ | $(f_{\text{cipher}} - n b) / a$ |
//! | AVG | $a \cdot \mathrm{AVG}_{\text{plain}} + b$ | $(f_{\text{cipher}} - b) / a$ |
//! | MIN | $a \cdot \mathrm{MIN}_{\text{plain}} + b$ if $a > 0$; $a \cdot \mathrm{MAX}_{\text{plain}} + b$ if $a < 0$ | sign-aware inverse |
//! | MAX | symmetric to MIN | sign-aware inverse |
//! | MEDIAN | $a \cdot \mathrm{MEDIAN}_{\text{plain}} + b$ (sign-symmetric: $q = 0.5 \Rightarrow 1-q = 0.5$) | $(f_{\text{cipher}} - b) / a$ |
//! | QUANTILE $q$ | $a \cdot \mathrm{Q}_q^{\text{plain}} + b$ if $a > 0$; $a \cdot \mathrm{Q}_{1-q}^{\text{plain}} + b$ if $a < 0$ | $(f_{\text{cipher}} - b) / a$ — caller flips $q \mapsto 1-q$ at the server query layer when $a < 0$ |
//! | ARGMIN, ARGMAX | record positions; preserved if $a > 0$, swapped if $a < 0$ | joint swap on $(\text{enc\_argmin}, \text{enc\_argmax})$ when $a < 0$ |
//! | VARIANCE | $a^2 \cdot \mathrm{Var}_{\text{plain}}$ | $f_{\text{cipher}} / a^2$ |
//! | STDDEV | $|a| \cdot \mathrm{StdDev}_{\text{plain}}$ | $f_{\text{cipher}} / |a|$ |
//! | RANGE | $|a| \cdot \mathrm{Range}_{\text{plain}}$ | $f_{\text{cipher}} / |a|$ |
//!
//! ## Probabilistic mode
//!
//! For $g(v) = a \cdot v + b + \varepsilon$ with $\varepsilon \sim
//! \mathcal{N}(0, \sigma^2)$ i.i.d. per encryption:
//! - **SUM, AVG**: same closed-form as Affine (noise terms average to
//!   zero in expectation; standard error is $\sigma \sqrt{n}$ for SUM
//!   and $\sigma / \sqrt{n}$ for AVG). Verified by
//!   `tests/fhe_pq_parity_rigor.rs::a2_probabilistic_*`.
//! - **VARIANCE**: the encrypted-side variance is $a^2 \cdot
//!   \mathrm{Var}_{\text{plain}} + \sigma^2$; recovery requires the
//!   schema-declared $\sigma$. The plaintext variance is
//!   $(f_{\text{cipher}} - \sigma^2) / a^2$ (bias-corrected; converges
//!   as $n \to \infty$).
//! - **STDDEV**: recovered via variance then sqrt (bias-corrected).
//! - **MIN, MAX, RANGE, MEDIAN, QUANTILE, ARGMIN, ARGMAX**: order
//!   statistics and order-statistic-derived positions do **not** commute
//!   with additive Gaussian noise. The naïve affine inverse
//!   $(f_{\text{cipher}} - b) / a$ produces a *biased* estimate (under
//!   $\sigma > 0$) for the value-recovery aggregates: recovered MIN <
//!   plain MIN, recovered MAX > plain MAX, recovered RANGE > plain RANGE,
//!   recovered QUANTILE biased toward the extremes. The position-recovery
//!   aggregates (ARGMIN, ARGMAX) suffer a different failure: noise can
//!   permute *which* record holds the extremum, so the recovered indices
//!   may point at the wrong records. The value bias magnitude is
//!   $\Theta(\sigma)$ and **does not vanish as $n \to \infty$** (it
//!   grows weakly with $n$, asymptotically like the Gumbel extreme-value
//!   formula $\sigma \sqrt{2 \log n}$ for sub-Gaussian noise).
//!
//!   Therefore `decrypt_min`, `decrypt_max`, `decrypt_range`,
//!   `decrypt_median`, `decrypt_quantile`, and `decrypt_argmin_argmax`
//!   **refuse** Probabilistic gauges with $\sigma > 0$ and return
//!   `AggregateError::BiasedUnderProbabilisticNoise`. Callers who want
//!   the (biased) estimate anyway — e.g.\ for a coarse upper/lower bound
//!   — must use the explicit `*_unchecked` variants and accept the
//!   bias. Rigor proofs of the bias for the MIN/MAX/RANGE cases are
//!   documented in `tests/fhe_pq_parity_rigor.rs::a4_*_unchecked_shows_bias`
//!   and `theory/encryption/validation/validation_tests_fhe_pq_rigor.py`
//!   §A.4; the same bias mechanism applies to MEDIAN / QUANTILE /
//!   ARGMIN / ARGMAX by direct extension of the order-statistic argument.
//!
//! ## Indexed / Opaque modes
//!
//! These are not numeric (PRF / AEAD outputs); aggregate inversion is
//! mode-incompatible. The helper functions return
//! `AggregateError::UnsupportedMode`.

use crate::crypto::{FieldTransform, GaugeKey};

#[derive(Debug, thiserror::Error)]
pub enum AggregateError {
    #[error("field at index {field_idx} has mode {mode}; aggregate inversion supports Affine, Probabilistic, and Identity numeric fields only")]
    UnsupportedMode {
        field_idx: usize,
        mode: &'static str,
    },
    #[error("field index {field_idx} out of bounds (gauge has {field_count} fields)")]
    FieldIndexOutOfBounds {
        field_idx: usize,
        field_count: usize,
    },
    #[error("gauge scale is zero — affine transform is singular")]
    ZeroScale,
    /// MIN / MAX / RANGE under a Probabilistic gauge with $\sigma > 0$
    /// cannot be exactly recovered — order statistics do not commute
    /// with additive Gaussian noise. Use the `*_unchecked` variant if
    /// you accept the bias, or use VARIANCE / STDDEV which *are*
    /// bias-correctable under Probabilistic.
    #[error(
        "aggregate {aggregate} on field {field_idx} is biased under Probabilistic gauge with σ={sigma}: \
         order statistics do not commute with additive noise. Use {aggregate}_unchecked if you accept the bias, \
         or switch the field to Affine mode (σ=0) for exact recovery."
    )]
    BiasedUnderProbabilisticNoise {
        field_idx: usize,
        aggregate: &'static str,
        sigma: f64,
    },
}

fn mode_name(ft: &FieldTransform) -> &'static str {
    match ft {
        FieldTransform::Identity => "Identity",
        FieldTransform::Affine { .. } => "Affine",
        FieldTransform::Opaque { .. } => "Opaque",
        FieldTransform::Indexed { .. } => "Indexed",
        FieldTransform::Probabilistic { .. } => "Probabilistic",
        FieldTransform::Isometric { .. } => "Isometric",
    }
}

fn field_transform<'g>(
    gauge: &'g GaugeKey,
    field_idx: usize,
) -> Result<&'g FieldTransform, AggregateError> {
    gauge
        .transforms
        .get(field_idx)
        .ok_or(AggregateError::FieldIndexOutOfBounds {
            field_idx,
            field_count: gauge.transforms.len(),
        })
}

fn affine_params(
    gauge: &GaugeKey,
    field_idx: usize,
) -> Result<(f64, f64, Option<f64>), AggregateError> {
    let ft = field_transform(gauge, field_idx)?;
    match ft {
        FieldTransform::Identity => Ok((1.0, 0.0, None)),
        FieldTransform::Affine { scale, offset } => {
            if *scale == 0.0 {
                return Err(AggregateError::ZeroScale);
            }
            Ok((*scale, *offset, None))
        }
        FieldTransform::Probabilistic {
            scale,
            offset,
            sigma,
            ..
        } => {
            if *scale == 0.0 {
                return Err(AggregateError::ZeroScale);
            }
            Ok((*scale, *offset, Some(*sigma)))
        }
        other => Err(AggregateError::UnsupportedMode {
            field_idx,
            mode: mode_name(other),
        }),
    }
}

// ───────────────────────────────────────────────────────────────────
// SUM
// ───────────────────────────────────────────────────────────────────

/// Recover plaintext SUM from the encrypted-side SUM and the record
/// count $n$. Server returns $\Sigma w_i = a \cdot \Sigma v_i + n b$;
/// this function applies $(f_{\text{cipher}} - n b) / a$.
pub fn decrypt_sum(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_sum: f64,
    n: u64,
) -> Result<f64, AggregateError> {
    let (a, b, _sigma) = affine_params(gauge, field_idx)?;
    Ok((encrypted_sum - (n as f64) * b) / a)
}

// ───────────────────────────────────────────────────────────────────
// AVG
// ───────────────────────────────────────────────────────────────────

/// Recover plaintext AVG from the encrypted-side AVG. AVG = SUM / n;
/// the gauge transform on AVG is the same affine map as on individual
/// values: $\text{AVG}_{\text{cipher}} = a \cdot \text{AVG}_{\text{plain}} + b$.
pub fn decrypt_avg(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_avg: f64,
) -> Result<f64, AggregateError> {
    let (a, b, _sigma) = affine_params(gauge, field_idx)?;
    Ok((encrypted_avg - b) / a)
}

// ───────────────────────────────────────────────────────────────────
// MIN, MAX (sign-aware) — refuse Probabilistic with σ > 0
// ───────────────────────────────────────────────────────────────────
//
// Order statistics do not commute with additive noise. For Affine
// (σ = 0) recovery is exact; for Probabilistic (σ > 0) the naïve
// affine inverse is biased. The `_unchecked` variants apply the
// inverse anyway for callers who accept the bias (coarse bounds,
// debugging, etc.); the safe variants refuse with a typed error.

/// Recover plaintext MIN from the encrypted-side MIN. When the affine
/// scale $a > 0$, MIN maps to MIN (order preserved). When $a < 0$,
/// MIN on ciphertext corresponds to MAX on plaintext (order reversed)
/// — the inversion still produces the correct plaintext MIN by
/// applying the affine inverse; the server's "MIN" is just on the
/// opposite end of the sorted list.
///
/// **Refuses Probabilistic with σ > 0** —
/// `AggregateError::BiasedUnderProbabilisticNoise` — because the naïve
/// inverse `(enc - b) / a` produces a value strictly less than the
/// plaintext MIN in expectation (the encrypted MIN includes a tail
/// draw from the Gaussian noise). Use `decrypt_min_unchecked` if you
/// accept the bias.
pub fn decrypt_min(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_min: f64,
) -> Result<f64, AggregateError> {
    let (a, b, sigma) = affine_params(gauge, field_idx)?;
    if let Some(s) = sigma {
        if s > 0.0 {
            return Err(AggregateError::BiasedUnderProbabilisticNoise {
                field_idx,
                aggregate: "MIN",
                sigma: s,
            });
        }
    }
    Ok((encrypted_min - b) / a)
}

/// Recover plaintext MIN by applying the affine inverse unconditionally.
/// Under Affine (σ = 0) this is exact. Under Probabilistic (σ > 0) the
/// result is **biased**: in expectation
/// $E[\hat{\text{MIN}}_{\text{plain}}] \lesssim \text{MIN}_{\text{plain}} -
/// \sigma \sqrt{2 \log n} / |a|$ (sub-Gaussian extreme-value bound).
/// Use only when a coarse lower bound is acceptable.
pub fn decrypt_min_unchecked(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_min: f64,
) -> Result<f64, AggregateError> {
    let (a, b, _sigma) = affine_params(gauge, field_idx)?;
    Ok((encrypted_min - b) / a)
}

/// Recover plaintext MAX. **Refuses Probabilistic with σ > 0** — see
/// `decrypt_min` rationale. Use `decrypt_max_unchecked` for the biased
/// estimate.
pub fn decrypt_max(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_max: f64,
) -> Result<f64, AggregateError> {
    let (a, b, sigma) = affine_params(gauge, field_idx)?;
    if let Some(s) = sigma {
        if s > 0.0 {
            return Err(AggregateError::BiasedUnderProbabilisticNoise {
                field_idx,
                aggregate: "MAX",
                sigma: s,
            });
        }
    }
    Ok((encrypted_max - b) / a)
}

/// Recover plaintext MAX unconditionally. Under Probabilistic the
/// result is biased *above* plain MAX by approximately
/// $\sigma \sqrt{2 \log n} / |a|$.
pub fn decrypt_max_unchecked(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_max: f64,
) -> Result<f64, AggregateError> {
    let (a, b, _sigma) = affine_params(gauge, field_idx)?;
    Ok((encrypted_max - b) / a)
}

// ───────────────────────────────────────────────────────────────────
// VARIANCE, STDDEV, RANGE
// ───────────────────────────────────────────────────────────────────

/// Recover plaintext VARIANCE. For Affine mode, Var(ciphertext) =
/// a² · Var(plaintext). For Probabilistic mode, Var(ciphertext) =
/// a² · Var(plaintext) + σ²; we subtract σ² before the a² division.
pub fn decrypt_variance(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_variance: f64,
) -> Result<f64, AggregateError> {
    let (a, _b, sigma) = affine_params(gauge, field_idx)?;
    let sigma_sq = sigma.map(|s| s * s).unwrap_or(0.0);
    Ok((encrypted_variance - sigma_sq) / (a * a))
}

/// Recover plaintext STDDEV from the encrypted-side STDDEV.
/// StdDev(ciphertext) = |a| · StdDev(plaintext). For Probabilistic
/// mode, the relationship is more subtle (StdDev does not commute with
/// the additive noise the way variance does); use `decrypt_variance`
/// then sqrt for accurate recovery.
pub fn decrypt_stddev(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_stddev: f64,
) -> Result<f64, AggregateError> {
    let (a, _b, sigma) = affine_params(gauge, field_idx)?;
    if sigma.is_some() {
        // For Probabilistic, recover via variance to maintain precision.
        let enc_var = encrypted_stddev * encrypted_stddev;
        let sigma_sq = sigma.unwrap().powi(2);
        let plain_var = ((enc_var - sigma_sq) / (a * a)).max(0.0);
        Ok(plain_var.sqrt())
    } else {
        Ok(encrypted_stddev / a.abs())
    }
}

/// Recover plaintext RANGE. Range(ciphertext) = |a| · Range(plaintext)
/// under Affine. **Refuses Probabilistic with σ > 0** — RANGE = MAX −
/// MIN inherits the order-statistic bias from both endpoints (≈ 2× the
/// MIN/MAX bias in magnitude). Use `decrypt_range_unchecked` for the
/// biased estimate.
pub fn decrypt_range(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_range: f64,
) -> Result<f64, AggregateError> {
    let (a, _b, sigma) = affine_params(gauge, field_idx)?;
    if let Some(s) = sigma {
        if s > 0.0 {
            return Err(AggregateError::BiasedUnderProbabilisticNoise {
                field_idx,
                aggregate: "RANGE",
                sigma: s,
            });
        }
    }
    Ok(encrypted_range / a.abs())
}

/// Recover plaintext RANGE unconditionally. Under Probabilistic the
/// result is biased *above* plain RANGE by approximately
/// $2 \sigma \sqrt{2 \log n} / |a|$.
pub fn decrypt_range_unchecked(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_range: f64,
) -> Result<f64, AggregateError> {
    let (a, _b, _sigma) = affine_params(gauge, field_idx)?;
    Ok(encrypted_range / a.abs())
}

// ───────────────────────────────────────────────────────────────────
// MEDIAN, QUANTILE (sign-aware) — refuse Probabilistic with σ > 0
// ───────────────────────────────────────────────────────────────────
//
// On an Affine-mode column the encrypted-side q-th quantile value maps
// via the affine inverse `(enc_q - b) / a` to the plaintext q-quantile
// (under a > 0) or to the plaintext (1−q)-quantile (under a < 0). The
// value-recovery formula is identical to MIN/MAX; what changes under
// sign is *which* quantile of the plaintext distribution the recovered
// value corresponds to. The caller is responsible for flipping
// `q ↦ 1 − q` at the server-side query layer when `a < 0` if they want
// the plaintext q-quantile; this function inverts the value, not the
// selection.

/// Recover plaintext q-quantile value from the encrypted-side q-quantile.
///
/// **Math.** `enc_q = a · plain_q + b`; the inverse `(enc_q - b) / a`
/// recovers the plaintext value. Under `a > 0` the server's encrypted-
/// side q-quantile is the plaintext q-quantile; under `a < 0` it is the
/// plaintext (1−q)-quantile (order reversed). The caller flips `q` at
/// the query layer when `a < 0` to align the selection.
///
/// **The `q` parameter is informational** — used only in the error
/// report so the BiasedUnderProbabilisticNoise variant can name the
/// requested quantile. The math does not depend on `q`.
///
/// **Refuses Probabilistic with σ > 0.**
pub fn decrypt_quantile(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_quantile: f64,
    q: f64,
) -> Result<f64, AggregateError> {
    let (a, b, sigma) = affine_params(gauge, field_idx)?;
    if let Some(s) = sigma {
        if s > 0.0 {
            return Err(AggregateError::BiasedUnderProbabilisticNoise {
                field_idx,
                aggregate: "QUANTILE",
                sigma: s,
            });
        }
    }
    let _ = q; // informational; reserved for future use (e.g., q-validation, telemetry)
    Ok((encrypted_quantile - b) / a)
}

/// Recover plaintext q-quantile unconditionally. Under Probabilistic
/// the result is biased; see `decrypt_min_unchecked` for the bias model.
pub fn decrypt_quantile_unchecked(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_quantile: f64,
    q: f64,
) -> Result<f64, AggregateError> {
    let (a, b, _sigma) = affine_params(gauge, field_idx)?;
    let _ = q;
    Ok((encrypted_quantile - b) / a)
}

/// Recover plaintext MEDIAN from the encrypted-side MEDIAN. Special case
/// of `decrypt_quantile` with `q = 0.5`; sign-symmetric (because
/// `1 - 0.5 = 0.5`, the server's encrypted-side MEDIAN is the plaintext
/// MEDIAN under either sign of `a`).
///
/// **Refuses Probabilistic with σ > 0.**
pub fn decrypt_median(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_median: f64,
) -> Result<f64, AggregateError> {
    let (a, b, sigma) = affine_params(gauge, field_idx)?;
    if let Some(s) = sigma {
        if s > 0.0 {
            return Err(AggregateError::BiasedUnderProbabilisticNoise {
                field_idx,
                aggregate: "MEDIAN",
                sigma: s,
            });
        }
    }
    Ok((encrypted_median - b) / a)
}

/// Recover plaintext MEDIAN unconditionally.
pub fn decrypt_median_unchecked(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_median: f64,
) -> Result<f64, AggregateError> {
    let (a, b, _sigma) = affine_params(gauge, field_idx)?;
    Ok((encrypted_median - b) / a)
}

// ───────────────────────────────────────────────────────────────────
// ARGMIN, ARGMAX — joint sign-safe recovery
// ───────────────────────────────────────────────────────────────────

/// Recover plaintext `(ARGMIN, ARGMAX)` record indices from the
/// encrypted-side indices, applying the sign-aware swap automatically.
///
/// **Math.** Under affine gauge `g(v) = a · v + b`:
/// - if `a > 0`: enc_argmin == plain_argmin and enc_argmax == plain_argmax
///   (order preserved bijectively).
/// - if `a < 0`: enc_argmin == plain_argmax and enc_argmax == plain_argmin
///   (order reversed bijectively).
///
/// Pass both encrypted-side indices; this function returns
/// `(plain_argmin_idx, plain_argmax_idx)` with the swap applied when
/// `a < 0`. The joint API removes the sign-flip burden from the caller.
///
/// **Refuses Probabilistic with σ > 0** — noise can permute *which*
/// record holds the encrypted extremum, so the recovered indices may
/// point at the wrong records. The `_unchecked` variant applies the
/// sign-aware swap anyway for callers who accept that risk.
pub fn decrypt_argmin_argmax(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_argmin_idx: usize,
    encrypted_argmax_idx: usize,
) -> Result<(usize, usize), AggregateError> {
    let (a, _b, sigma) = affine_params(gauge, field_idx)?;
    if let Some(s) = sigma {
        if s > 0.0 {
            return Err(AggregateError::BiasedUnderProbabilisticNoise {
                field_idx,
                aggregate: "ARGMIN/ARGMAX",
                sigma: s,
            });
        }
    }
    if a > 0.0 {
        Ok((encrypted_argmin_idx, encrypted_argmax_idx))
    } else {
        Ok((encrypted_argmax_idx, encrypted_argmin_idx))
    }
}

/// Recover plaintext `(ARGMIN, ARGMAX)` indices unconditionally. Under
/// Probabilistic the result is biased: the noise may have permuted which
/// record actually holds the encrypted extremum.
pub fn decrypt_argmin_argmax_unchecked(
    gauge: &GaugeKey,
    field_idx: usize,
    encrypted_argmin_idx: usize,
    encrypted_argmax_idx: usize,
) -> Result<(usize, usize), AggregateError> {
    let (a, _b, _sigma) = affine_params(gauge, field_idx)?;
    if a > 0.0 {
        Ok((encrypted_argmin_idx, encrypted_argmax_idx))
    } else {
        Ok((encrypted_argmax_idx, encrypted_argmin_idx))
    }
}

// ───────────────────────────────────────────────────────────────────
// COUNT (gauge-invariant, included for API completeness)
// ───────────────────────────────────────────────────────────────────

/// COUNT is gauge-invariant — record count is unchanged by any
/// deterministic encryption. This function is provided for API
/// symmetry; the caller should just use the server-returned count
/// directly.
pub fn decrypt_count(_gauge: &GaugeKey, _field_idx: usize, encrypted_count: u64) -> u64 {
    encrypted_count
}

// ───────────────────────────────────────────────────────────────────
// Tests
// ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn affine_gauge(scale: f64, offset: f64) -> GaugeKey {
        GaugeKey {
            transforms: vec![FieldTransform::Affine { scale, offset }],
        }
    }

    fn probabilistic_gauge(scale: f64, offset: f64, sigma: f64) -> GaugeKey {
        GaugeKey {
            transforms: vec![FieldTransform::Probabilistic {
                scale,
                offset,
                sigma,
                bucket_key: [0u8; 32],
            }],
        }
    }

    #[test]
    fn sum_roundtrip_under_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let n = v.len() as u64;
        let plaintext_sum: f64 = v.iter().sum();
        let gauge = affine_gauge(2.5, -3.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| 2.5 * x - 3.0).collect();
        let encrypted_sum: f64 = encrypted_v.iter().sum();
        let recovered = decrypt_sum(&gauge, 0, encrypted_sum, n).unwrap();
        assert!(
            (recovered - plaintext_sum).abs() < 1e-9,
            "expected {}, got {}",
            plaintext_sum,
            recovered
        );
    }

    #[test]
    fn avg_roundtrip_under_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let plaintext_avg: f64 = v.iter().sum::<f64>() / (v.len() as f64);
        let gauge = affine_gauge(2.5, -3.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| 2.5 * x - 3.0).collect();
        let encrypted_avg: f64 = encrypted_v.iter().sum::<f64>() / (encrypted_v.len() as f64);
        let recovered = decrypt_avg(&gauge, 0, encrypted_avg).unwrap();
        assert!(
            (recovered - plaintext_avg).abs() < 1e-9,
            "expected {}, got {}",
            plaintext_avg,
            recovered
        );
    }

    #[test]
    fn min_max_roundtrip_under_positive_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let plain_min = 10.0_f64;
        let plain_max = 40.0_f64;
        let gauge = affine_gauge(2.5, -3.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| 2.5 * x - 3.0).collect();
        let enc_min = encrypted_v.iter().cloned().fold(f64::INFINITY, f64::min);
        let enc_max = encrypted_v
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let recovered_min = decrypt_min(&gauge, 0, enc_min).unwrap();
        let recovered_max = decrypt_max(&gauge, 0, enc_max).unwrap();
        assert!((recovered_min - plain_min).abs() < 1e-9);
        assert!((recovered_max - plain_max).abs() < 1e-9);
    }

    #[test]
    fn variance_roundtrip_under_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let mean = v.iter().sum::<f64>() / (v.len() as f64);
        let plain_var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (v.len() as f64);
        let gauge = affine_gauge(2.5, -3.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| 2.5 * x - 3.0).collect();
        let enc_mean = encrypted_v.iter().sum::<f64>() / (encrypted_v.len() as f64);
        let enc_var = encrypted_v
            .iter()
            .map(|x| (x - enc_mean).powi(2))
            .sum::<f64>()
            / (encrypted_v.len() as f64);
        let recovered = decrypt_variance(&gauge, 0, enc_var).unwrap();
        assert!(
            (recovered - plain_var).abs() < 1e-9,
            "expected var {}, got {}",
            plain_var,
            recovered
        );
    }

    #[test]
    fn stddev_roundtrip_under_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let mean = v.iter().sum::<f64>() / (v.len() as f64);
        let plain_var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (v.len() as f64);
        let plain_stddev = plain_var.sqrt();
        let gauge = affine_gauge(2.5, -3.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| 2.5 * x - 3.0).collect();
        let enc_mean = encrypted_v.iter().sum::<f64>() / (encrypted_v.len() as f64);
        let enc_var = encrypted_v
            .iter()
            .map(|x| (x - enc_mean).powi(2))
            .sum::<f64>()
            / (encrypted_v.len() as f64);
        let enc_stddev = enc_var.sqrt();
        let recovered = decrypt_stddev(&gauge, 0, enc_stddev).unwrap();
        assert!(
            (recovered - plain_stddev).abs() < 1e-9,
            "expected stddev {}, got {}",
            plain_stddev,
            recovered
        );
    }

    #[test]
    fn range_roundtrip_under_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let plain_range = 30.0_f64;
        let gauge = affine_gauge(2.5, -3.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| 2.5 * x - 3.0).collect();
        let enc_range = encrypted_v.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
            - encrypted_v.iter().cloned().fold(f64::INFINITY, f64::min);
        let recovered = decrypt_range(&gauge, 0, enc_range).unwrap();
        assert!((recovered - plain_range).abs() < 1e-9);
    }

    #[test]
    fn aggregates_under_negative_scale() {
        // Negative scale flips the ordering of min/max but the affine inverse
        // still recovers the right plaintext values. Server's "MIN" is on
        // the opposite end of the sorted list, but the inverse maps it back.
        let v = [10.0_f64, 20.0, 30.0, 40.0];
        let plain_sum: f64 = v.iter().sum();
        let plain_min = 10.0_f64;
        let plain_max = 40.0_f64;
        let gauge = affine_gauge(-1.5, 5.0);
        let encrypted_v: Vec<f64> = v.iter().map(|x| -1.5 * x + 5.0).collect();
        // Note: server's MIN-by-value on encrypted list is encrypted_max (because a<0).
        let enc_min_value = encrypted_v.iter().cloned().fold(f64::INFINITY, f64::min);
        let enc_max_value = encrypted_v
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);
        let enc_sum: f64 = encrypted_v.iter().sum();
        let recovered_sum = decrypt_sum(&gauge, 0, enc_sum, v.len() as u64).unwrap();
        // For a<0: server-returned MIN-by-value corresponds to plaintext MAX.
        // The inversion gives MAX in plaintext.
        let recovered_from_enc_min = decrypt_min(&gauge, 0, enc_min_value).unwrap();
        let recovered_from_enc_max = decrypt_max(&gauge, 0, enc_max_value).unwrap();
        assert!((recovered_sum - plain_sum).abs() < 1e-9);
        // Under a<0, decrypt_min(enc_min_value) recovers plain MAX:
        assert!((recovered_from_enc_min - plain_max).abs() < 1e-9);
        assert!((recovered_from_enc_max - plain_min).abs() < 1e-9);
    }

    #[test]
    fn probabilistic_sum_within_noise_bound() {
        let v = [10.0_f64, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0];
        let plain_sum: f64 = v.iter().sum();
        let sigma = 0.5;
        let gauge = probabilistic_gauge(2.5, -3.0, sigma);
        // Simulate encryption with Gaussian noise (using a fixed seed for
        // reproducibility — pseudo-random with explicit values).
        let noise: [f64; 8] = [0.31, -0.24, 0.18, -0.42, 0.05, 0.27, -0.13, 0.08];
        let encrypted_v: Vec<f64> = v
            .iter()
            .zip(noise.iter())
            .map(|(x, e)| 2.5 * x - 3.0 + e)
            .collect();
        let encrypted_sum: f64 = encrypted_v.iter().sum();
        let recovered = decrypt_sum(&gauge, 0, encrypted_sum, v.len() as u64).unwrap();
        // Noise is i.i.d. with σ; SUM has standard error σ√n; for 8 records,
        // expected error magnitude ~0.5·√8 ≈ 1.4. The decryption recovers
        // to within ~σ√n/|a| absolute error.
        let expected_err = sigma * (v.len() as f64).sqrt() / 2.5;
        assert!(
            (recovered - plain_sum).abs() < 5.0 * expected_err,
            "recovered {} vs plain {}; bound = {}",
            recovered,
            plain_sum,
            expected_err
        );
    }

    #[test]
    fn count_is_gauge_invariant() {
        let gauge = affine_gauge(2.5, -3.0);
        assert_eq!(decrypt_count(&gauge, 0, 42), 42);
    }

    #[test]
    fn opaque_field_returns_unsupported_mode() {
        let gauge = GaugeKey {
            transforms: vec![FieldTransform::Opaque { key: [0u8; 32] }],
        };
        let err = decrypt_sum(&gauge, 0, 100.0, 10).unwrap_err();
        assert!(matches!(err, AggregateError::UnsupportedMode { .. }));
    }

    #[test]
    fn indexed_field_returns_unsupported_mode() {
        let gauge = GaugeKey {
            transforms: vec![FieldTransform::Indexed { key: [0u8; 32] }],
        };
        let err = decrypt_avg(&gauge, 0, 100.0).unwrap_err();
        assert!(matches!(err, AggregateError::UnsupportedMode { .. }));
    }

    #[test]
    fn identity_passes_aggregates_through() {
        let gauge = GaugeKey {
            transforms: vec![FieldTransform::Identity],
        };
        let recovered_sum = decrypt_sum(&gauge, 0, 100.0, 5).unwrap();
        let recovered_avg = decrypt_avg(&gauge, 0, 20.0).unwrap();
        assert_eq!(recovered_sum, 100.0);
        assert_eq!(recovered_avg, 20.0);
    }

    // ───────────────────────────────────────────────────────────
    // Bias-refusal tests for MIN/MAX/RANGE under Probabilistic σ > 0
    // (the integration test in tests/fhe_pq_parity_rigor.rs proves
    //  the bias is statistically significant — these unit tests
    //  prove the safe API refuses and the _unchecked API still works.)
    // ───────────────────────────────────────────────────────────

    #[test]
    fn min_max_range_refuse_probabilistic_with_positive_sigma() {
        let gauge = probabilistic_gauge(2.5, -3.0, 0.5);
        let e_min = decrypt_min(&gauge, 0, 10.0).unwrap_err();
        let e_max = decrypt_max(&gauge, 0, 50.0).unwrap_err();
        let e_range = decrypt_range(&gauge, 0, 40.0).unwrap_err();
        assert!(matches!(
            e_min,
            AggregateError::BiasedUnderProbabilisticNoise {
                aggregate: "MIN",
                ..
            }
        ));
        assert!(matches!(
            e_max,
            AggregateError::BiasedUnderProbabilisticNoise {
                aggregate: "MAX",
                ..
            }
        ));
        assert!(matches!(
            e_range,
            AggregateError::BiasedUnderProbabilisticNoise {
                aggregate: "RANGE",
                ..
            }
        ));
    }

    #[test]
    fn min_max_range_accept_probabilistic_with_zero_sigma() {
        // σ = 0 degenerates Probabilistic into Affine; recovery is exact.
        let gauge = probabilistic_gauge(2.5, -3.0, 0.0);
        // f(10) = 2.5*10 - 3 = 22; inverse: (22 - (-3))/2.5 = 10
        assert!((decrypt_min(&gauge, 0, 22.0).unwrap() - 10.0).abs() < 1e-12);
        assert!((decrypt_max(&gauge, 0, 22.0).unwrap() - 10.0).abs() < 1e-12);
        // Range = |a| * plain_range; (10) / 2.5 = 4.0
        assert!((decrypt_range(&gauge, 0, 10.0).unwrap() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn unchecked_min_max_range_bypass_refusal() {
        // The _unchecked variants apply the affine inverse anyway —
        // useful for coarse bounds. Caller accepts the bias.
        let gauge = probabilistic_gauge(2.5, -3.0, 0.5);
        // The math still runs; we just don't get exactness under σ > 0.
        let _ = decrypt_min_unchecked(&gauge, 0, 10.0).expect("unchecked must not refuse");
        let _ = decrypt_max_unchecked(&gauge, 0, 50.0).expect("unchecked must not refuse");
        let _ = decrypt_range_unchecked(&gauge, 0, 40.0).expect("unchecked must not refuse");
    }

    #[test]
    fn variance_and_stddev_still_supported_under_probabilistic() {
        // VARIANCE and STDDEV are bias-correctable (subtract σ²); these
        // continue to work under Probabilistic.
        let gauge = probabilistic_gauge(2.5, -3.0, 0.5);
        let recovered = decrypt_variance(&gauge, 0, 100.0).unwrap();
        // (100 - 0.25) / 6.25 ≈ 15.96
        assert!((recovered - 15.96).abs() < 0.01);
        let _ = decrypt_stddev(&gauge, 0, 10.0).unwrap();
    }

    // ───────────────────────────────────────────────────────────
    // MEDIAN / QUANTILE — exactness on Affine, refusal on Probabilistic
    // ───────────────────────────────────────────────────────────

    #[test]
    fn median_quantile_roundtrip_under_positive_affine() {
        let v = [10.0_f64, 20.0, 30.0, 40.0, 50.0];
        // sorted v ascending: [10, 20, 30, 40, 50]; median = 30; q=0.25 ≈ 20
        let plain_median = 30.0_f64;
        let plain_q25 = 20.0_f64;
        let gauge = affine_gauge(2.5, -3.0);
        // enc_v = [22, 47, 72, 97, 122]; sorted: [22, 47, 72, 97, 122]; median = 72; q=0.25 ≈ 47
        let enc_median = 72.0_f64;
        let enc_q25 = 47.0_f64;
        assert!((decrypt_median(&gauge, 0, enc_median).unwrap() - plain_median).abs() < 1e-9);
        assert!(
            (decrypt_quantile(&gauge, 0, enc_q25, 0.25).unwrap() - plain_q25).abs() < 1e-9
        );
    }

    #[test]
    fn median_is_sign_symmetric_under_negative_affine() {
        // Under a < 0 the encrypted MEDIAN is still the plaintext MEDIAN
        // because 1 - 0.5 = 0.5 — the middle of a reversed sort is the
        // same record as the middle of the forward sort.
        let v = [10.0_f64, 20.0, 30.0, 40.0, 50.0];
        let plain_median = 30.0_f64;
        let gauge = affine_gauge(-1.5, 5.0);
        // enc_v = [-1.5*10+5, ...] = [-10, -25, -40, -55, -70]
        // sorted enc ascending: [-70, -55, -40, -25, -10]; median = -40
        // -40 = -1.5*30 + 5 = enc(30) ✓
        let enc_median = -40.0_f64;
        assert!(
            (decrypt_median(&gauge, 0, enc_median).unwrap() - plain_median).abs() < 1e-9,
            "median sign-symmetry broken"
        );
    }

    #[test]
    fn median_quantile_refuse_probabilistic_with_positive_sigma() {
        let gauge = probabilistic_gauge(2.5, -3.0, 0.5);
        let e_med = decrypt_median(&gauge, 0, 30.0).unwrap_err();
        let e_q = decrypt_quantile(&gauge, 0, 30.0, 0.5).unwrap_err();
        assert!(matches!(
            e_med,
            AggregateError::BiasedUnderProbabilisticNoise {
                aggregate: "MEDIAN",
                ..
            }
        ));
        assert!(matches!(
            e_q,
            AggregateError::BiasedUnderProbabilisticNoise {
                aggregate: "QUANTILE",
                ..
            }
        ));
    }

    #[test]
    fn median_quantile_unchecked_bypass_refusal() {
        let gauge = probabilistic_gauge(2.5, -3.0, 0.5);
        let _ = decrypt_median_unchecked(&gauge, 0, 30.0).expect("unchecked must not refuse");
        let _ = decrypt_quantile_unchecked(&gauge, 0, 30.0, 0.5)
            .expect("unchecked must not refuse");
    }

    // ───────────────────────────────────────────────────────────
    // ARGMIN / ARGMAX — joint, sign-safe
    // ───────────────────────────────────────────────────────────

    #[test]
    fn argmin_argmax_roundtrip_under_positive_affine() {
        let v = [50.0_f64, 10.0, 30.0, 40.0, 20.0];
        // plain argmin = 1 (v[1]=10); plain argmax = 0 (v[0]=50)
        let plain_argmin = 1_usize;
        let plain_argmax = 0_usize;
        let gauge = affine_gauge(2.5, -3.0);
        // enc_v = [122, 22, 72, 97, 47]; enc argmin = 1; enc argmax = 0
        let (rec_argmin, rec_argmax) = decrypt_argmin_argmax(&gauge, 0, 1, 0).unwrap();
        let _ = v;
        assert_eq!(rec_argmin, plain_argmin);
        assert_eq!(rec_argmax, plain_argmax);
    }

    #[test]
    fn argmin_argmax_auto_swap_under_negative_affine() {
        let v = [50.0_f64, 10.0, 30.0, 40.0, 20.0];
        let plain_argmin = 1_usize;
        let plain_argmax = 0_usize;
        let gauge = affine_gauge(-1.5, 5.0);
        // enc_v = [-1.5*50+5, ...] = [-70, -10, -40, -55, -25]
        // sorted ascending: [-70 (idx 0), -55 (idx 3), -40 (idx 2), -25 (idx 4), -10 (idx 1)]
        // enc argmin (smallest enc value) = 0; enc argmax (largest enc value) = 1
        let enc_argmin = 0_usize;
        let enc_argmax = 1_usize;
        let (rec_argmin, rec_argmax) =
            decrypt_argmin_argmax(&gauge, 0, enc_argmin, enc_argmax).unwrap();
        let _ = v;
        // a < 0 swaps: plain argmin = enc argmax = 1; plain argmax = enc argmin = 0
        assert_eq!(rec_argmin, plain_argmin);
        assert_eq!(rec_argmax, plain_argmax);
    }

    #[test]
    fn argmin_argmax_refuses_probabilistic_with_positive_sigma() {
        let gauge = probabilistic_gauge(2.5, -3.0, 0.5);
        let err = decrypt_argmin_argmax(&gauge, 0, 1, 0).unwrap_err();
        assert!(matches!(
            err,
            AggregateError::BiasedUnderProbabilisticNoise {
                aggregate: "ARGMIN/ARGMAX",
                ..
            }
        ));
    }

    #[test]
    fn argmin_argmax_unchecked_bypass_refusal_and_applies_swap() {
        let gauge_pos = probabilistic_gauge(2.5, -3.0, 0.5);
        let gauge_neg = probabilistic_gauge(-1.5, 5.0, 0.5);
        // Unchecked still applies the sign-aware swap; just no refusal.
        let (mn, mx) = decrypt_argmin_argmax_unchecked(&gauge_pos, 0, 1, 0)
            .expect("unchecked must not refuse");
        assert_eq!((mn, mx), (1, 0));
        let (mn, mx) = decrypt_argmin_argmax_unchecked(&gauge_neg, 0, 1, 0)
            .expect("unchecked must not refuse");
        assert_eq!((mn, mx), (0, 1)); // swapped
    }
}
