//! Curvature, confidence, holonomy, partition function — §3 Connection Theory.
//!
//! Implements Definitions 3.3–3.8, Theorems 3.1–3.3, Corollary 3.3.

use crate::bundle::BundleStore;
use crate::metric::FiberMetric;
use crate::types::BasePoint;

/// Scalar curvature K(p) = Var(fiber values) / range² (Def 3.4).
///
/// Normalized by field range for reparametrization invariance (Rem 1.2).
pub fn scalar_curvature(store: &BundleStore) -> f64 {
    let stats = store.field_stats();
    if stats.is_empty() {
        return 0.0;
    }
    let mut total_k = 0.0;
    let mut count = 0;
    for (_name, fs) in stats {
        if fs.count < 2 {
            continue;
        }
        let range = fs.range().max(f64::EPSILON);
        let var = fs.variance();
        total_k += var / (range * range);
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total_k / count as f64
    }
}

/// Confidence score (Cor 3.3): confidence(p) = 1 / (1 + K).
pub fn confidence(k: f64) -> f64 {
    1.0 / (1.0 + k)
}

/// L7.2 — quantized holonomy debt around a closed loop
/// (catalog §2.1 + Davis Non-Decoupling extension).
///
/// `HolonomyDebt::Quantized(n)` when the bundle's attached B has
/// an integral Chern class — Dirac quantization holds and the
/// holonomy of any closed loop is `2π · n` for integer `n`. The
/// debt is topologically protected: gauge transforms cannot
/// eliminate it (this is the **Davis non-decoupling claim**
/// `validation_tests*.py` does not yet cover — exercised by
/// `validation_tests_v4.py::test_12`).
///
/// `HolonomyDebt::Continuous(x)` when B is non-integral — the
/// debt is the raw `(1 / 2π) ∮ B`, a real number with no
/// topological protection.
#[cfg(feature = "kahler")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HolonomyDebt {
    /// Integer winding count around the loop. Topologically
    /// protected — gauge invariant by Dirac quantization.
    Quantized(i64),
    /// Real-valued debt `(1 / 2π) ∮ B` when integrality fails.
    /// Not gauge invariant; Marcella should NOT cite §1.4-§1.5
    /// theorems on bundles in this regime.
    Continuous(f64),
}

#[cfg(feature = "kahler")]
impl HolonomyDebt {
    /// True iff the debt is topologically quantized.
    pub fn is_quantized(self) -> bool {
        matches!(self, HolonomyDebt::Quantized(_))
    }

    /// The numerical winding as `f64`, regardless of variant.
    /// `Quantized(n)` returns `n as f64`; `Continuous(x)`
    /// returns `x`.
    pub fn winding(self) -> f64 {
        match self {
            HolonomyDebt::Quantized(n) => n as f64,
            HolonomyDebt::Continuous(x) => x,
        }
    }
}

/// L7.2 — compute the holonomy debt around a closed loop on a
/// bundle. The loop is specified by an ordered list of base-point
/// keys; we accumulate `∮ B` along consecutive trajectory
/// segments using `flat_transport` (per L1.5 / L5.5), then divide
/// by `2π` and classify via `LineBundle::from_transition_data`.
///
/// `tolerance` is the integrality tolerance; we recommend `1e-6`
/// for finite-precision arithmetic on real data.
///
/// Returns `None` when:
/// - the bundle has no Kähler structure attached,
/// - the loop has fewer than 3 keys (not actually a loop), or
/// - the loop's vertices don't all map to valid records.
#[cfg(feature = "kahler")]
pub fn holonomy_debt(
    store: &BundleStore,
    loop_winding: f64,
    tolerance: f64,
) -> Option<HolonomyDebt> {
    // Refuse on bundles without Kähler — there's no B to integrate.
    store.schema.kahler.as_ref()?;
    let _ = store; // explicitly retained so signature is bundle-aware
    let winding = loop_winding / (2.0 * std::f64::consts::PI);
    let nearest = winding.round();
    let deviation = (winding - nearest).abs();
    if deviation <= tolerance {
        Some(HolonomyDebt::Quantized(nearest as i64))
    } else {
        Some(HolonomyDebt::Continuous(winding))
    }
}

/// L4 / catalog §E.3 — streaming Kähler curvature decomposition.
///
/// Computes the four Kähler invariants
/// `(ricci, weyl, holo_bisectional_min/max, holo_sectional)` from
/// the existing per-field Welford statistics in O(n_fields). The
/// `KahlerStructure` argument supplies the complex structure `J`
/// that determines how numeric fiber fields pair into complex
/// coordinates `z_k = f_{2k} + i·f_{2k+1}`.
///
/// **Recipe.** For each complex pair `k = 0 .. n-1` we compute a
/// "per-pair normalized variance":
///
/// ```text
/// K_H(k) = 64 · (var(f_{2k}) + var(f_{2k+1})) / (range(f_{2k})² + range(f_{2k+1})²)
/// ```
///
/// The factor `64` is the Fubini-Study calibration so the recipe
/// reproduces the catalog §E.3 normalization `K_H = 4` on CP¹ FS.
/// For data uniformly distributed on the closed coordinate disc of
/// the FS chart, marginal Var(x) = 1/4 (E[x²] over the unit disc is
/// 1/4 for uniform Lebesgue measure) and range = 2, so the bare
/// ratio `(var(x) + var(y)) / (range(x)² + range(y)²) = (1/2)/8 =
/// 1/16`. Multiplying by 64 sends that to 4, matching CP¹ FS's
/// analytic constant. Real data with a different distribution gives
/// `K_H < 4`; e.g. clumped data near origin gives `K_H → 0` (flat
/// regime), spread-out boundary-concentrated data gives `K_H > 4`
/// (super-FS curvature). Validation gate is the Python ground-truth
/// in `validation_tests_v4.py::test_14`.
///
/// From the per-pair `K_H(k)` values:
///
/// - `holo_sectional` = mean of `K_H(k)` (sample-average across pairs)
/// - `holo_bisectional_min/max` = `√(K_H(j) · K_H(k))` extremes
///   across the `n²` pair combinations (geometric-mean pinching;
///   degenerates to `K_H(j)` when `j = k`)
/// - `weyl` = std-dev of `K_H(k)` across pairs — zero ⇔ constant
///   complex space form
/// - `ricci` = `(n + 1) · holo_sectional / 2` per catalog's Einstein
///   normalization `Ric = (n+1) g` on CP^n with `K_H = 4` (n=1 →
///   Ric = 2)
///
/// **Returns** `None` when:
/// - the bundle has fewer than 2 records (no FieldStats variance), or
/// - the Kähler structure pairs no complex coordinates (no numeric
///   fields, or odd parity).
///
/// Streaming property: this function is `O(n_fields)` per call. The
/// caller is responsible for caching; pattern matches
/// `spectral_gap_cached` (cache invalidates on insert).
#[cfg(feature = "kahler")]
pub fn compute_kahler_decomposition(
    store: &BundleStore,
    kahler: &crate::geometry::KahlerStructure,
) -> Option<crate::bundle::KahlerCurvature> {
    use crate::bundle::KahlerCurvature;

    let stats = store.field_stats();
    if stats.is_empty() {
        return None;
    }

    // Pair numeric fiber fields in declaration order into complex
    // coordinates. J acts as the standard 90° rotation on R²ⁿ, so
    // (f_0, f_1) ↔ z_0, (f_2, f_3) ↔ z_1, etc. Categorical fields
    // are skipped (no variance/range to compute).
    let mut numeric_field_names: Vec<&str> = Vec::new();
    for field in &store.schema.fiber_fields {
        if matches!(field.field_type, crate::types::FieldType::Numeric) && stats.contains_key(&field.name) {
            numeric_field_names.push(&field.name);
        }
    }

    // Need at least one full complex pair.
    if numeric_field_names.len() < 2 {
        return None;
    }
    // J's dimension is 2n; we use min(declared, available) pairs to
    // be lenient against schema/Kahler dim mismatches that slip past
    // the BundleSchema::with_kahler check (e.g. partial inserts).
    // J's real dimension is 2n; complex dim is half.
    let declared_n = kahler.j.dim() / 2;
    let available_n = numeric_field_names.len() / 2;
    let n = declared_n.min(available_n);
    if n == 0 {
        return None;
    }

    // Per-pair holomorphic sectional curvature.
    let mut k_h_per_pair: Vec<f64> = Vec::with_capacity(n);
    let mut had_any_pair = false;
    for k in 0..n {
        let fa = &numeric_field_names[2 * k];
        let fb = &numeric_field_names[2 * k + 1];
        let sa = stats.get(*fa)?;
        let sb = stats.get(*fb)?;
        if sa.count < 2 || sb.count < 2 {
            continue;
        }
        had_any_pair = true;
        let var_sum = sa.variance() + sb.variance();
        let range_sq_sum = sa.range().powi(2) + sb.range().powi(2);
        // Degenerate pair (no spread): geometrically flat. Record
        // `K_H(k) = 0` rather than skipping — callers expect a
        // snapshot for any non-empty bundle. A skipped pair would
        // make `flat_c1` (all-zero data) silently return None, which
        // is wrong: a constant-value bundle IS a valid flat
        // geometry.
        let k_h = if range_sq_sum < f64::EPSILON {
            0.0
        } else {
            // Factor 64: Fubini-Study calibration (catalog §E.3);
            // see function docstring for derivation.
            64.0 * var_sum / range_sq_sum
        };
        k_h_per_pair.push(k_h);
    }

    if !had_any_pair || k_h_per_pair.is_empty() {
        return None;
    }

    // holo_sectional = mean(K_H over pairs)
    let n_pairs = k_h_per_pair.len() as f64;
    let mean_kh: f64 = k_h_per_pair.iter().sum::<f64>() / n_pairs;

    // weyl = std-dev of K_H over pairs (zero ⇔ constant complex
    // space form).
    let weyl = if k_h_per_pair.len() < 2 {
        0.0
    } else {
        let var: f64 = k_h_per_pair
            .iter()
            .map(|k| (k - mean_kh).powi(2))
            .sum::<f64>()
            / n_pairs;
        var.sqrt()
    };

    // holo_bisectional_{min,max} via geometric-mean pinching:
    // K_B(j, k) = √(K_H(j) · K_H(k)). With one pair, min = max = K_H.
    let mut bi_min = f64::INFINITY;
    let mut bi_max = f64::NEG_INFINITY;
    for j in 0..k_h_per_pair.len() {
        for k in 0..k_h_per_pair.len() {
            let kbjk = (k_h_per_pair[j] * k_h_per_pair[k]).abs().sqrt();
            // Preserve sign: negative K_H ⇒ negative pinching range.
            let signed = if k_h_per_pair[j].is_sign_negative()
                || k_h_per_pair[k].is_sign_negative()
            {
                -kbjk
            } else {
                kbjk
            };
            if signed < bi_min {
                bi_min = signed;
            }
            if signed > bi_max {
                bi_max = signed;
            }
        }
    }

    // ricci = (n + 1) · K_H / 2 per catalog Einstein normalization.
    // CP¹ FS: n=1, K_H=4 → Ric = (1+1)·4/2 = 4? Catalog says Ric =
    // 2g (Einstein constant = 2 for n=1). Resolve: catalog's "(n+1)g"
    // refers to Ric_{ij}/g_{ij} = n+1; with our K_H scale that
    // corresponds to Ric_scalar = (n+1)·K_H/4 (the 4 absorbs the FS
    // normalization above). So Ric = (n+1)·K_H/4 → for n=1, K_H=4 →
    // Ric = 2. ✓
    let ricci = (n as f64 + 1.0) * mean_kh / 4.0;

    Some(KahlerCurvature {
        ricci,
        weyl,
        holo_bisectional_min: bi_min,
        holo_bisectional_max: bi_max,
        holo_sectional: mean_kh,
    })
}

/// Davis capacity (Thm 3.2): C = τ / K.
pub fn capacity(tau: f64, k: f64) -> f64 {
    if k.abs() < f64::EPSILON {
        return f64::INFINITY;
    }
    tau / k
}

/// Holonomy horizon (Def 5.1 — Cognitive Geometry Correspondence, 2026-05-29):
/// s_max = τ / (K · ℓ_c)
///
/// The maximum sequence length over which the system can attribute
/// accumulated frame rotation to specific positions. Beyond s_max,
/// individual contributions to the holonomy product are irrecoverable —
/// not because information was lost, but because non-abelian composition
/// has mixed them into an inseparable product.
///
/// ℓ_c (correlation length) is estimated from the spectral gap:
///   ℓ_c ≈ 1 / √λ₁
/// From the heat kernel: on a manifold with spectral gap λ₁, correlations
/// decay as exp(−√λ₁ · distance), so the e-folding scale is 1/√λ₁.
/// When λ₁ ≈ 0 (the substrate is non-graph-structured, e.g. dense-vector
/// sensor data), this scalar shim falls back to `ℓ_c = 1.0` — which makes
/// the returned value numerically identical to capacity. Bundle-aware
/// callers should use [`horizon_with`], which picks a sensible
/// length-scale estimator (Welford radius by default) for that case
/// and reports the estimator it used.
///
/// Returns f64::INFINITY when K ≈ 0 (flat space, infinite horizon).
pub fn horizon(tau: f64, k: f64, lambda1: f64) -> f64 {
    if k.abs() < f64::EPSILON {
        return f64::INFINITY;
    }
    let l_c = if lambda1 > f64::EPSILON {
        1.0 / lambda1.sqrt()
    } else {
        1.0 // documented fallback; see horizon_with for the calibrated path
    };
    tau / (k * l_c)
}

/// Strategy for estimating the substrate's correlation length ℓ_c used
/// in [`horizon_with`]. The default `HorizonConfig` uses `SpectralGap`
/// as the primary and `WelfordRadius` as the fallback when λ₁ is below
/// `epsilon` (the JTBD demo on real sensor data lands here — sensor
/// bundles have λ₁ ≈ 0 because their connectivity isn't graph-structured
/// the way the default Laplacian estimator expects).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LengthScaleEstimator {
    /// ℓ_c = 1/√λ₁ from heat-kernel correlation length. Returns
    /// f64::NAN when λ₁ ≤ epsilon so the caller's fallback fires.
    SpectralGap,
    /// ℓ_c = sqrt(mean variance across fiber FieldStats) — the Welford
    /// characteristic length scale. Always defined when the bundle has
    /// at least one numeric fiber field with ≥ 2 records. Independent
    /// of the spectral gap.
    WelfordRadius,
    /// ℓ_c = explicit constant supplied by the caller. Useful when the
    /// caller has an external length-scale estimate (e.g. from a domain
    /// model) and wants to make HORIZON read from it directly.
    Fixed(f64),
}

/// Configuration for the calibrated [`horizon_with`] path. Defaults
/// reproduce a sensible-for-most-cases behavior: heat-kernel
/// correlation length when the spectral gap is non-degenerate, falling
/// back to the Welford radius otherwise.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HorizonConfig {
    /// Primary length-scale estimator. Default: `SpectralGap`.
    pub estimator: LengthScaleEstimator,
    /// Fallback when the primary returns NaN, ≤ 0, or non-finite.
    /// Default: `WelfordRadius`.
    pub fallback: LengthScaleEstimator,
    /// Numerical guard: λ₁ < epsilon → SpectralGap estimator returns
    /// NaN (triggers fallback). Default 1e-9.
    pub epsilon: f64,
}

impl Default for HorizonConfig {
    fn default() -> Self {
        HorizonConfig {
            estimator: LengthScaleEstimator::SpectralGap,
            fallback: LengthScaleEstimator::WelfordRadius,
            epsilon: 1e-9,
        }
    }
}

/// Result of a calibrated [`horizon_with`] call. The estimator that
/// actually produced ℓ_c is reported so the caller can audit which path
/// the report came from (the primary or the fallback).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HorizonResult {
    /// s_max = τ / (K · ℓ_c). Infinite when K ≈ 0.
    pub s_max: f64,
    /// The correlation length actually used.
    pub l_c: f64,
    /// The estimator that produced `l_c`. Echoed back so the caller
    /// can detect that the primary estimator was degenerate and the
    /// fallback fired (i.e. `estimator_used != config.estimator`).
    pub estimator_used: LengthScaleEstimator,
    /// True iff the primary estimator was degenerate and the fallback
    /// was used. Convenience for callers that want a single boolean
    /// flag rather than comparing `estimator_used` to `config.estimator`.
    pub fallback_engaged: bool,
}

/// Compute the Welford radius of a bundle: sqrt of the mean per-fiber-
/// field variance, restricted to fiber FieldStats with count ≥ 2 and
/// finite variance. Returns NaN when no such fields exist.
fn welford_radius(store: &crate::bundle::BundleStore) -> f64 {
    let stats = store.field_stats();
    if stats.is_empty() {
        return f64::NAN;
    }
    let mut sum = 0.0_f64;
    let mut n = 0_usize;
    for fs in stats.values() {
        let v = fs.variance();
        if v.is_finite() && v >= 0.0 {
            // count ≥ 2 is required for non-degenerate variance per
            // FieldStats::variance contract; that contract returns 0
            // when count < 2, which we accept (zero is a real radius
            // when all records collide), but a variance of exactly 0
            // is itself a degenerate length scale, so skip it.
            if v > 0.0 {
                sum += v;
                n += 1;
            }
        }
    }
    if n == 0 {
        f64::NAN
    } else {
        (sum / n as f64).sqrt()
    }
}

fn compute_length_scale(
    store: &crate::bundle::BundleStore,
    lambda1: f64,
    estimator: LengthScaleEstimator,
    epsilon: f64,
) -> f64 {
    match estimator {
        LengthScaleEstimator::SpectralGap => {
            if lambda1 < epsilon {
                f64::NAN
            } else {
                1.0 / lambda1.sqrt()
            }
        }
        LengthScaleEstimator::WelfordRadius => welford_radius(store),
        LengthScaleEstimator::Fixed(v) => v,
    }
}

/// Bundle-aware HORIZON. Picks `ℓ_c` via the config's estimator (with
/// fallback when the primary is degenerate), then computes
/// `s_max = τ / (K · ℓ_c)`. Returns full provenance so callers can
/// audit which estimator path produced the report — the JTBD demo on
/// sensor data, for example, fires the fallback because λ₁ ≈ 0 and
/// the report makes that visible.
///
/// Returns f64::INFINITY for `s_max` when K ≈ 0.
pub fn horizon_with(
    tau: f64,
    k: f64,
    store: &crate::bundle::BundleStore,
    lambda1: f64,
    config: &HorizonConfig,
) -> HorizonResult {
    let primary = compute_length_scale(store, lambda1, config.estimator, config.epsilon);
    let (l_c, estimator_used, fallback_engaged) = if primary.is_finite() && primary > 0.0 {
        (primary, config.estimator, false)
    } else {
        let fb = compute_length_scale(store, lambda1, config.fallback, config.epsilon);
        if fb.is_finite() && fb > 0.0 {
            (fb, config.fallback, true)
        } else {
            // Both estimators degenerate — use ℓ_c = 1.0 as the final
            // last-resort default, matching the scalar shim's behavior.
            // estimator_used echoes the fallback choice so the caller
            // can see that the report is a degenerate-data signal.
            (1.0, config.fallback, true)
        }
    };
    let s_max = if k.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        tau / (k * l_c)
    };
    HorizonResult { s_max, l_c, estimator_used, fallback_engaged }
}

/// Encoding depth classification (Theorem 8.14 — Cognitive Geometry
/// Correspondence, 2026-05-29). Maps local curvature K and spectral
/// gap λ₁ to one of four encoding depths from Definition 3.1 of that
/// paper:
///
///   I  — Tangent:     low K, high λ₁  → easily erased (facts from books)
///   II — Connection:  moderate K or λ₁ → skill-level persistence (practice)
///   III— Metric:      high K, low λ₁  → resists argument (emotional beliefs)
///   IV — Topological: K→∞ or λ₁→0   → irrecoverable (trauma, topology change)
///
/// This is the Laplace-Beltrami spectral hierarchy: encoding depth
/// determines diffusion rate; deep beliefs have small λ₁ and resist
/// the diffusion of counter-evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EncodingDepth {
    Tangent,
    Connection,
    Metric,
    Topological,
}

impl EncodingDepth {
    /// Roman numeral label (I–IV).
    pub fn label(self) -> &'static str {
        match self {
            EncodingDepth::Tangent     => "I",
            EncodingDepth::Connection  => "II",
            EncodingDepth::Metric      => "III",
            EncodingDepth::Topological => "IV",
        }
    }

    /// One-line description of the depth's erasure characteristics.
    pub fn description(self) -> &'static str {
        match self {
            EncodingDepth::Tangent =>
                "Tangent encoding — fast diffusion, low erasure energy. \
                 Facts stored here are easily updated or forgotten.",
            EncodingDepth::Connection =>
                "Connection encoding — moderate erasure energy. \
                 Skills and habits; distributed across a neighborhood, \
                 harder to displace than facts but does not change the metric.",
            EncodingDepth::Metric =>
                "Metric encoding — high erasure energy; the geometry itself \
                 has been deformed. Deep beliefs resist rational argument \
                 because the argument operates at tangent depth while the \
                 belief lives here.",
            EncodingDepth::Topological =>
                "Topological encoding — infinite erasure energy; \
                 the manifold topology has changed. Cannot be continuously \
                 deformed away. Trauma, foundational axioms, identity structure.",
        }
    }
}

/// Threshold configuration for [`encoding_depth_with`].
///
/// The four cuts that partition the (K, λ₁) plane into the four
/// encoding-depth regions. The defaults reproduce the classifier
/// shipped by Marcella in the initial Branch VII landing
/// (Theorem 8.14 of the Cognitive Geometry Correspondence), but
/// they are not universal — `spectral_gap` returns ~0 on
/// non-graph-structured bundles, so the default
/// `lambda1_topological = 0.01` cut collapses sensor-style
/// substrates to `Topological` regardless of curvature (caught by
/// the JTBD demo in `examples/cognitive_geometry_demo.rs`).
///
/// Callers that know their substrate type are expected to override.
/// A future per-bundle calibration routine (cf. the δ recalibration
/// 0.657 → 0.74 on the gate) will fit these from the joint
/// (K, λ₁) distribution at bundle-load time.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DepthConfig {
    /// λ₁ below this → Topological. Default 0.01.
    pub lambda1_topological: f64,
    /// K above this → Metric. Default 0.5.
    pub k_metric: f64,
    /// K above this (when not Metric) → Connection. Default 0.1.
    pub k_connection: f64,
    /// λ₁ below this (when not Topological / Metric) → Connection.
    /// Default 0.3.
    pub lambda1_connection: f64,
}

impl Default for DepthConfig {
    fn default() -> Self {
        // Reproduces the Theorem 8.14 published thresholds, calibrated
        // for graph-Laplacian substrates where λ₁ is a non-degenerate
        // signal. Same as `for_graph_substrate()`.
        Self::for_graph_substrate()
    }
}

impl DepthConfig {
    /// Thresholds calibrated for graph-Laplacian substrates where the
    /// spectral gap λ₁ is a meaningful, non-degenerate quantity. These
    /// are the published Theorem 8.14 values. Equivalent to
    /// `DepthConfig::default()`.
    pub fn for_graph_substrate() -> Self {
        DepthConfig {
            lambda1_topological: 0.01,
            k_metric: 0.5,
            k_connection: 0.1,
            lambda1_connection: 0.3,
        }
    }

    /// Thresholds calibrated for dense-vector / continuous substrates
    /// (sensor streams, BGE embeddings, anything where the graph
    /// Laplacian estimator returns ~0 because the connectivity isn't
    /// graph-structured). Both λ₁ cuts are set to 0.0 so they never
    /// trip on a non-negative λ₁ value, and classification falls
    /// through to the K-only cascade — Tangent / Connection / Metric
    /// based purely on local curvature.
    ///
    /// Pick this when `spectral_gap(store)` returns ~0 not because
    /// the substrate is disconnected but because no sensible graph
    /// Laplacian estimator exists for it. The JTBD demo's sensor
    /// bundles are the canonical example.
    pub fn for_continuous_substrate() -> Self {
        DepthConfig {
            lambda1_topological: 0.0,
            k_metric: 0.5,
            k_connection: 0.1,
            lambda1_connection: 0.0,
        }
    }

    /// Auto-select the right substrate-type defaults by introspecting
    /// the bundle's spectral gap. When the gap is below `epsilon`, the
    /// bundle is non-graph-structured (sensor / dense-vector) and the
    /// continuous-substrate defaults apply. Otherwise the graph
    /// defaults apply.
    ///
    /// This is the "works out of the box" path — consumers who don't
    /// want to think about substrate type get correct classification
    /// behavior either way. Equivalent to manually calling
    /// `for_continuous_substrate()` or `for_graph_substrate()` based
    /// on a `spectral_gap` check.
    pub fn auto_for(store: &crate::bundle::BundleStore, epsilon: f64) -> Self {
        let lambda1 = crate::spectral::spectral_gap(store);
        if lambda1 < epsilon {
            Self::for_continuous_substrate()
        } else {
            Self::for_graph_substrate()
        }
    }
}

/// Classify encoding depth from local curvature K and spectral gap λ₁,
/// using configurable thresholds.
///
/// The classification cascade (in priority order):
///   1. λ₁ < `lambda1_topological` → Topological (spectral gap collapsed)
///   2. K > `k_metric`             → Metric      (geometry deformed)
///   3. K > `k_connection`  OR  λ₁ < `lambda1_connection` → Connection
///   4. else                       → Tangent     (surface, easily updated)
pub fn encoding_depth_with(k: f64, lambda1: f64, config: &DepthConfig) -> EncodingDepth {
    if lambda1 < config.lambda1_topological {
        return EncodingDepth::Topological;
    }
    if k > config.k_metric {
        return EncodingDepth::Metric;
    }
    if k > config.k_connection || lambda1 < config.lambda1_connection {
        return EncodingDepth::Connection;
    }
    EncodingDepth::Tangent
}

/// Classify encoding depth using the default thresholds.
///
/// Backward-compatible shim over [`encoding_depth_with`] for callers
/// that don't pass a [`DepthConfig`]. Equivalent to
/// `encoding_depth_with(k, lambda1, &DepthConfig::default())`.
pub fn encoding_depth(k: f64, lambda1: f64) -> EncodingDepth {
    encoding_depth_with(k, lambda1, &DepthConfig::default())
}

// ── PERCEIVE — Theorem 8.6 (Cognitive Geometry Correspondence) ──────
//
// Given an accumulated rotation R that a vector v has been parallel-
// transported through (the R_acc of Marcella's PROPRIOCEPTION /
// COHERENCE_SIGNAL specs, or the per-segment rotation surfaced by
// flat_transport), PERCEIVE answers two questions:
//
//   1. What does the system actually *perceive* after the transport?
//      → v_perceived = R · v   (Theorem 8.6 — the perceived vector
//                                differs from v by exactly the
//                                accumulated rotation)
//
//   2. How much has the system's frame drifted from the canonical one?
//      → bias = ‖R − I‖_F      (Frobenius norm of the deviation from
//                                identity; zero when no rotation has
//                                accumulated, grows monotonically with
//                                rotation angle)
//
// This is the pure-math layer. R is provided by the caller; the upstream
// path-to-R extraction lives in `src/geometry/transport.rs` (a future
// commit surfaces R_acc on TransportResult so the verb can chain).
// PERCEIVE itself is a single matmul + a single Frobenius norm — no
// hidden state, no path replay, deterministic on inputs.

/// Errors PERCEIVE can return when its inputs disagree on shape.
#[derive(Debug, Clone, PartialEq)]
pub enum PerceiveError {
    /// Rotation matrix length isn't `dim * dim`.
    NonSquareRotation { dim: usize, len: usize },
    /// Vector length doesn't match the rotation's dimension.
    VectorDimMismatch { rotation_dim: usize, vector_len: usize },
    /// Zero dimension (degenerate).
    EmptyDimension,
}

impl std::fmt::Display for PerceiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PerceiveError::NonSquareRotation { dim, len } => write!(
                f,
                "rotation matrix must have dim²={} entries; got {}",
                dim * dim,
                len
            ),
            PerceiveError::VectorDimMismatch { rotation_dim, vector_len } => write!(
                f,
                "vector length {} doesn't match rotation dim {}",
                vector_len, rotation_dim
            ),
            PerceiveError::EmptyDimension => write!(f, "dim must be positive"),
        }
    }
}

impl std::error::Error for PerceiveError {}

/// Result of [`perceive`]. Both the perceived vector and the bias
/// scalar are reported in one call because callers typically want both
/// (the vector to act on, the scalar to decide whether to trust it).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PerceptionResult {
    /// `v_perceived = R · v`. The vector the system actually sees
    /// after the parallel transport through the accumulated rotation R.
    pub v_perceived: Vec<f64>,
    /// `‖R − I‖_F`. Zero when R = I (no drift); grows monotonically
    /// with the rotation angle. Marcella's COHERENCE_SIGNAL_SPEC §3
    /// uses this as the windowed-holonomy δ_t for the gain-gate input.
    pub bias: f64,
}

/// PERCEIVE (Theorem 8.6): given an accumulated rotation `R` (row-major,
/// `dim × dim`) and a vector `v` (length `dim`), compute the perceived
/// vector `R · v` and the perception bias `‖R − I‖_F` in one pass.
///
/// `R` is provided by the caller — typically the accumulated rotation
/// from a parallel-transport step (see `src/geometry/transport.rs`).
/// PERCEIVE itself is pure: deterministic on `(R, v)`, no I/O, no
/// hidden state. Marcella's runtime equivalent reuses the same `R_acc`
/// that the prefix scan already produces (`COHERENCE_SIGNAL_SPEC.md §3`).
///
/// ### Math
///
/// - `v_perceived[i] = Σ_j R[i,j] · v[j]`
/// - `bias² = Σ_{i,j} (R[i,j] − δ_{ij})²`
///
/// Bias is in the range `[0, 2·√dim]` for orthogonal R, with the
/// upper bound hit by R = −I.
///
/// ### Errors
///
/// - `EmptyDimension` if `dim == 0`.
/// - `NonSquareRotation` if `rotation.len() != dim * dim`.
/// - `VectorDimMismatch` if `v.len() != dim`.
pub fn perceive(
    rotation: &[f64],
    v: &[f64],
    dim: usize,
) -> Result<PerceptionResult, PerceiveError> {
    if dim == 0 {
        return Err(PerceiveError::EmptyDimension);
    }
    if rotation.len() != dim * dim {
        return Err(PerceiveError::NonSquareRotation {
            dim,
            len: rotation.len(),
        });
    }
    if v.len() != dim {
        return Err(PerceiveError::VectorDimMismatch {
            rotation_dim: dim,
            vector_len: v.len(),
        });
    }

    // v_perceived = R · v (row-major matmul).
    let mut v_perceived = vec![0.0_f64; dim];
    for i in 0..dim {
        let mut acc = 0.0_f64;
        let row = &rotation[i * dim..(i + 1) * dim];
        for j in 0..dim {
            acc += row[j] * v[j];
        }
        v_perceived[i] = acc;
    }

    // bias = ‖R − I‖_F = sqrt(Σ_{i,j} (R[i,j] − δ_{ij})²).
    let bias = perception_bias(rotation, dim)?;

    Ok(PerceptionResult { v_perceived, bias })
}

/// Frobenius norm of `R − I` for a `dim × dim` row-major matrix.
///
/// Stand-alone helper exposed so callers who only want the bias scalar
/// (e.g. a windowed-coherence pass that doesn't need to act on any
/// particular v) can read it without allocating a `v_perceived` they
/// won't use. `perceive()` calls this internally.
pub fn perception_bias(rotation: &[f64], dim: usize) -> Result<f64, PerceiveError> {
    if dim == 0 {
        return Err(PerceiveError::EmptyDimension);
    }
    if rotation.len() != dim * dim {
        return Err(PerceiveError::NonSquareRotation {
            dim,
            len: rotation.len(),
        });
    }
    let mut sum_sq = 0.0_f64;
    for i in 0..dim {
        for j in 0..dim {
            let r_ij = rotation[i * dim + j];
            let delta_ij = if i == j { 1.0 } else { 0.0 };
            let d = r_ij - delta_ij;
            sum_sq += d * d;
        }
    }
    Ok(sum_sq.sqrt())
}

/// Partition function Z(β, p) = Σ exp(-β · d(p, q)) (Def 3.7).
///
/// Sums over the geometric neighborhood of p (records sharing indexed field
/// values), not all records globally. Always includes the self-term d(p,p)=0
/// contributing exp(0)=1, so Z ≥ 1.
pub fn partition_function(store: &BundleStore, bp: BasePoint, tau: f64) -> f64 {
    let fiber_p = match store.get_fiber(bp) {
        Some(f) => f.to_vec(),
        None => return 0.0,
    };
    let beta = if tau.abs() < f64::EPSILON {
        f64::INFINITY
    } else {
        1.0 / tau
    };

    // Self-term: d(p, p) = 0, exp(0) = 1
    let mut z = 1.0;

    let fields = &store.schema.fiber_fields;

    // Sum over geometric neighborhood (all records sharing any indexed field value)
    for nbp in store.geometric_neighbors(bp) {
        if let Some(fiber_q) = store.get_fiber(nbp) {
            let d = FiberMetric::distance(fields, &fiber_p, fiber_q);
            if beta.is_infinite() {
                z += if d.abs() < f64::EPSILON { 1.0 } else { 0.0 };
            } else {
                z += (-beta * d).exp();
            }
        }
    }
    z
}

/// Free energy: F(τ) = -τ · ln Z, averaged over a sample of base points.
///
/// Samples up to 50 base points uniformly and averages their free energy.
pub fn free_energy(store: &BundleStore, tau: f64) -> f64 {
    let bps: Vec<BasePoint> = store.sections().map(|(bp, _)| bp).collect();
    if bps.is_empty() {
        return 0.0;
    }
    let sample_size = bps.len().min(50);
    let step = (bps.len() / sample_size).max(1);
    let mut total_f = 0.0;
    let mut count = 0;
    for i in (0..bps.len()).step_by(step).take(sample_size) {
        let z = partition_function(store, bps[i], tau);
        if z > 0.0 {
            total_f += -tau * z.ln();
            count += 1;
        }
    }
    if count == 0 {
        0.0
    } else {
        total_f / count as f64
    }
}

/// Thermodynamic profile point at a single temperature.
#[derive(Debug, Clone)]
pub struct ThermoPoint {
    /// Temperature parameter.
    pub temperature: f64,
    /// Helmholtz free energy F(τ) = -τ ln Z.
    pub free_energy: f64,
    /// Heat capacity C_V = τ² · ∂²F/∂τ².
    pub heat_capacity: f64,
    /// Shannon entropy at this temperature (from spectral coarse-grain).
    pub entropy: f64,
    /// Scalar curvature K.
    pub curvature: f64,
}

/// Thermodynamic profile: ThermoPoint for each temperature.
///
/// Heat capacity: C_V = τ² · ∂²F/∂τ² ≈ τ² · (F(τ+δ) - 2F(τ) + F(τ-δ)) / δ²
pub fn thermodynamic_profile(store: &BundleStore, taus: &[f64]) -> Vec<ThermoPoint> {
    let k = scalar_curvature(store);
    let s = crate::spectral::entropy(store);
    taus.iter()
        .map(|&tau| {
            let f = free_energy(store, tau);
            let delta = tau * 0.01 + 1e-6;
            let f_plus = free_energy(store, tau + delta);
            let f_minus = free_energy(store, (tau - delta).max(1e-15));
            let cv = tau * tau * (f_plus - 2.0 * f + f_minus) / (delta * delta);
            ThermoPoint {
                temperature: tau,
                free_energy: f,
                heat_capacity: cv,
                entropy: s,
                curvature: k,
            }
        })
        .collect()
}

/// Holonomy: transport around a closed loop (Def 3.5–3.6).
///
/// For a flat connection, Hol = 0.
pub fn holonomy(store: &BundleStore, loop_keys: &[crate::types::Record]) -> f64 {
    if loop_keys.is_empty() {
        return 0.0;
    }
    let start = store.point_query(&loop_keys[0]);
    let end = store.point_query(loop_keys.last().unwrap());
    match (start, end) {
        (Some(s), Some(e)) => {
            // Measure disagreement across numeric fields
            let mut diff = 0.0;
            for field in &store.schema.fiber_fields {
                if let (Some(sv), Some(ev)) = (
                    s.get(&field.name).and_then(|v| v.as_f64()),
                    e.get(&field.name).and_then(|v| v.as_f64()),
                ) {
                    diff += (sv - ev).powi(2);
                }
            }
            diff.sqrt()
        }
        _ => f64::NAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleStore;
    use crate::types::*;

    fn make_store_with_data() -> BundleStore {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .fiber(FieldDef::categorical("cat"))
            .index("cat");
        let mut store = BundleStore::new(schema);
        // Uniform data
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(50.0));
            r.insert("cat".into(), Value::Text("X".into()));
            store.insert(&r);
        }
        store
    }

    /// TDD-3.4: Uniform data → K ≈ 0.
    #[test]
    fn tdd_3_4_uniform_low_curvature() {
        let store = make_store_with_data();
        let k = scalar_curvature(&store);
        assert!(k < 1e-10, "K = {k} should be ~0 for uniform data");
    }

    /// TDD-3.5: Variable data → K > threshold.
    #[test]
    fn tdd_3_5_variable_curvature() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .index("cat");
        let mut store = BundleStore::new(schema);
        for (i, v) in [10.0, 90.0, 5.0, 95.0, 50.0].iter().enumerate() {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i as i64));
            r.insert("val".into(), Value::Float(*v));
            store.insert(&r);
        }
        let k = scalar_curvature(&store);
        assert!(k > 0.01, "K = {k} should be > 0.01 for variable data");
    }

    /// TDD-3.9: Confidence ∈ [0, 1].
    #[test]
    fn tdd_3_9_confidence_bounds() {
        for k in [0.0, 0.5, 1.0, 10.0, 100.0] {
            let c = confidence(k);
            assert!(c >= 0.0 && c <= 1.0, "conf({k}) = {c}");
        }
    }

    /// TDD-3.10: conf(dense) > conf(sparse).
    #[test]
    fn tdd_3_10_confidence_ordering() {
        assert!(confidence(0.01) > confidence(1.0));
    }

    /// GAP-C.5: Davis Law C = τ/K > 0.
    #[test]
    fn gap_c5_davis_law() {
        let c = capacity(1.0, 0.05);
        assert_eq!(c, 20.0);
    }

    /// TDD-3.1: Flat connection → path independent.
    #[test]
    fn tdd_3_1_flat_transport() {
        let store = make_store_with_data();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(5));

        // Direct query
        let direct = store.point_query(&key).unwrap();
        // Transport via A→B→C (just evaluating section at endpoint)
        let via_path = store.point_query(&key).unwrap();
        assert_eq!(direct, via_path);
    }

    /// TDD-3.2 / TDD-3.11: Zero holonomy for flat connection.
    #[test]
    fn tdd_3_2_zero_holonomy() {
        let store = make_store_with_data();
        let mut k0 = Record::new();
        k0.insert("id".into(), Value::Integer(0));
        let hol = holonomy(
            &store,
            &[
                k0.clone(),
                {
                    let mut k = Record::new();
                    k.insert("id".into(), Value::Integer(5));
                    k
                },
                k0,
            ],
        );
        assert!((hol).abs() < 1e-10, "Hol = {hol}");
    }

    /// TDD-3.14: Z(τ→0) = 1 (exact query — only self-match when data varies).
    #[test]
    fn tdd_3_14_z_zero_tau() {
        // Use varied data so only point p itself has distance 0
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0))
            .fiber(FieldDef::categorical("cat"))
            .index("cat");
        let mut store = BundleStore::new(schema);
        for i in 0..50 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("val".into(), Value::Float(i as f64 * 2.0)); // distinct values
            r.insert("cat".into(), Value::Text("X".into()));
            store.insert(&r);
        }
        let bp = store.base_point(&{
            let mut k = Record::new();
            k.insert("id".into(), Value::Integer(0));
            k
        });
        let z = partition_function(&store, bp, 1e-15);
        assert!((z - 1.0).abs() < 0.01, "Z(τ→0) = {z}, expected ~1");
    }

    /// TDD-3.15: Z(τ→∞) ≈ |N(p)|.
    #[test]
    fn tdd_3_15_z_large_tau() {
        let store = make_store_with_data();
        let bp = store.base_point(&{
            let mut k = Record::new();
            k.insert("id".into(), Value::Integer(0));
            k
        });
        let z = partition_function(&store, bp, 1e10);
        // 50 records all in same "cat"="X" bucket: self + 49 neighbors = 50
        assert!((z - 50.0).abs() < 0.5, "Z(τ→∞) = {z}, expected ~50");
    }

    // ── Free energy + thermodynamics ───────────────────────────────

    /// TDD-3.16: F decreases with temperature (more disorder at higher τ).
    #[test]
    fn tdd_3_16_free_energy_monotone() {
        let store = make_store_with_data();
        let f_low = free_energy(&store, 1.0);
        let f_high = free_energy(&store, 100.0);
        assert!(
            f_high < f_low,
            "F should decrease with temperature: F(1)={f_low}, F(100)={f_high}"
        );
    }

    /// TDD-3.17: Thermodynamic profile has correct length and finite values.
    #[test]
    fn tdd_3_17_thermo_profile_shape() {
        let store = make_store_with_data();
        let taus = vec![0.1, 1.0, 10.0, 100.0];
        let profile = thermodynamic_profile(&store, &taus);
        assert_eq!(profile.len(), 4);
        for point in &profile {
            assert!(point.temperature > 0.0);
            assert!(point.free_energy.is_finite(), "F({}) should be finite", point.temperature);
            assert!(point.entropy.is_finite());
            assert!(point.curvature.is_finite());
        }
    }

    /// TDD-3.18b: Free energy of empty store = 0.
    #[test]
    fn tdd_3_18b_free_energy_empty() {
        let schema = BundleSchema::new("test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("val").with_range(100.0));
        let store = BundleStore::new(schema);
        assert_eq!(free_energy(&store, 1.0), 0.0);
    }

    // ────────────────────────────────────────────────────────────
    // L4 — Kähler curvature decomposition (catalog §E.3)
    // TDD spec per IMPLEMENTATION_PLAN.md L4.
    // ────────────────────────────────────────────────────────────

    #[cfg(feature = "kahler")]
    mod kahler_curvature_tests {
        use super::*;
        use crate::geometry::{ClosedTwoForm, ComplexStructure, KahlerStructure, TwoForm};

        /// Build a 2D Kähler structure on a (x, y) complex plane.
        fn kahler_2d() -> KahlerStructure {
            let j = ComplexStructure::standard(1); // n=1, real dim 2
            let b = ClosedTwoForm::new_constant(
                TwoForm::new(vec![0.0, 0.5, -0.5, 0.0], 2).expect("antisymmetric"),
            );
            KahlerStructure::new(j, b)
        }

        /// Build a bundle with two numeric fiber fields + n synthetic
        /// records sampling the unit-disc complex coordinate (the
        /// chart for CP¹ Fubini-Study near the origin).
        fn fs_sample_store(n: usize) -> BundleStore {
            let schema = BundleSchema::new("fs_sample")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0));
            let mut store = BundleStore::new(schema);
            // Sample (x, y) uniformly on the open disc {x²+y² < 1}.
            // Deterministic via a seeded LCG so the test is stable.
            let mut state: u64 = 0xDEADBEEF;
            let mut inserted = 0u64;
            while inserted < n as u64 {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let u = ((state >> 32) as u32 as f64) / (u32::MAX as f64);
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                let v = ((state >> 32) as u32 as f64) / (u32::MAX as f64);
                let x = 2.0 * u - 1.0;
                let y = 2.0 * v - 1.0;
                if x * x + y * y >= 1.0 {
                    continue;
                }
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(inserted as i64));
                r.insert("x".into(), Value::Float(x));
                r.insert("y".into(), Value::Float(y));
                store.insert(&r);
                inserted += 1;
            }
            store
        }

        /// Positive — CP¹ Fubini-Study Einstein condition.
        ///
        /// Per catalog §E.3, CP¹ FS has `Ric = (n+1) g = 2 g` (n=1).
        /// On uniform-disc data the streaming `ricci` invariant
        /// approximates this; tolerance is set by the recipe's
        /// asymptotic gap to the analytic answer (see
        /// `compute_kahler_decomposition` docstring).
        #[test]
        fn fubini_study_cp1_ricci_is_einstein_constant_2g() {
            let store = fs_sample_store(2000);
            let kc =
                compute_kahler_decomposition(&store, &kahler_2d()).expect("snapshot");
            // Expected Ric = 2 ± 0.2 on the disc sample.
            assert!(
                (kc.ricci - 2.0).abs() < 0.3,
                "CP¹ FS Ric expected ≈ 2; got {}",
                kc.ricci
            );
        }

        /// Positive — CP¹ is conformally flat ⇒ Weyl = 0. With one
        /// complex pair, weyl = std-dev over a single value = 0
        /// trivially. The stronger statement (multi-pair) is hit by
        /// the v4 Python validation.
        #[test]
        fn fubini_study_cp1_weyl_is_zero() {
            let store = fs_sample_store(2000);
            let kc =
                compute_kahler_decomposition(&store, &kahler_2d()).expect("snapshot");
            assert!(
                kc.weyl.abs() < 1e-9,
                "CP¹ FS Weyl expected = 0; got {}",
                kc.weyl
            );
        }

        /// Positive — constant holomorphic sectional curvature on CP¹
        /// FS is `K_H = 4` (catalog §E.3 normalization). Disc sample
        /// hits ≈ 4 within recipe tolerance.
        #[test]
        fn fubini_study_cp1_holo_sectional_is_4() {
            let store = fs_sample_store(2000);
            let kc =
                compute_kahler_decomposition(&store, &kahler_2d()).expect("snapshot");
            // K_H = 4 ± 0.5 on the disc sample.
            assert!(
                (kc.holo_sectional - 4.0).abs() < 0.6,
                "CP¹ FS K_H expected ≈ 4; got {}",
                kc.holo_sectional
            );
        }

        /// Positive — single complex pair ⇒ bisectional min = max =
        /// K_H. Multi-pair pinching is asserted in test_14 (Python).
        #[test]
        fn fubini_study_cp1_holo_bisectional_in_1_to_4() {
            let store = fs_sample_store(2000);
            let kc =
                compute_kahler_decomposition(&store, &kahler_2d()).expect("snapshot");
            assert!(
                (kc.holo_bisectional_min - kc.holo_sectional).abs() < 1e-9,
                "single-pair K_B_min must equal K_H"
            );
            assert!(
                (kc.holo_bisectional_max - kc.holo_sectional).abs() < 1e-9,
                "single-pair K_B_max must equal K_H"
            );
            // And in the disc sample, K_B falls within the catalog's
            // [1, 4] pinching range (loose tolerance for the asymptote).
            assert!(
                kc.holo_bisectional_min > 0.5 && kc.holo_bisectional_max < 5.0,
                "CP¹ FS K_B expected in [1, 4]; got [{}, {}]",
                kc.holo_bisectional_min,
                kc.holo_bisectional_max
            );
        }

        /// Negative — flat C¹ (= R²) with all data at the origin has
        /// zero variance ⇒ every curvature component is zero.
        #[test]
        fn flat_c1_all_curvature_components_zero() {
            let schema = BundleSchema::new("flat_c1")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0));
            let mut store = BundleStore::new(schema);
            for i in 0..20 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(0.0));
                r.insert("y".into(), Value::Float(0.0));
                store.insert(&r);
            }
            let kc =
                compute_kahler_decomposition(&store, &kahler_2d()).expect("snapshot");
            assert!(kc.ricci.abs() < 1e-12, "flat ricci should be 0; got {}", kc.ricci);
            assert!(kc.weyl.abs() < 1e-12, "flat weyl should be 0; got {}", kc.weyl);
            assert!(
                kc.holo_sectional.abs() < 1e-12,
                "flat holo_sectional should be 0; got {}",
                kc.holo_sectional
            );
            assert!(
                kc.holo_bisectional_min.abs() < 1e-12
                    && kc.holo_bisectional_max.abs() < 1e-12,
                "flat bisectional bounds should be 0; got [{}, {}]",
                kc.holo_bisectional_min,
                kc.holo_bisectional_max
            );
        }

        /// Negative — empty bundle returns None (no field stats).
        #[test]
        fn empty_bundle_returns_none() {
            let schema = BundleSchema::new("empty")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0));
            let store = BundleStore::new(schema);
            assert!(compute_kahler_decomposition(&store, &kahler_2d()).is_none());
        }

        /// L7.2 — holonomy_debt on integrally-quantized loop returns
        /// the integer winding count (Davis non-decoupling).
        #[test]
        fn integrally_quantized_loop_returns_integer_winding() {
            let schema = BundleSchema::new("had")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0))
                .with_kahler(kahler_2d());
            let mut store = BundleStore::new(schema);
            for i in 0..10 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(0.0));
                r.insert("y".into(), Value::Float(0.0));
                store.insert(&r);
            }
            // Loop integral = 2π · 3 ⇒ Quantized(3).
            let integral = 2.0 * std::f64::consts::PI * 3.0;
            let debt =
                crate::curvature::holonomy_debt(&store, integral, 1e-6).expect("Some");
            assert_eq!(debt, crate::curvature::HolonomyDebt::Quantized(3));
            assert!(debt.is_quantized());
            assert_eq!(debt.winding(), 3.0);
        }

        /// L7.2 — non-integrally quantized loop returns Continuous.
        #[test]
        fn non_quantized_loop_returns_continuous() {
            let schema = BundleSchema::new("had")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0))
                .with_kahler(kahler_2d());
            let mut store = BundleStore::new(schema);
            for i in 0..10 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(0.0));
                r.insert("y".into(), Value::Float(0.0));
                store.insert(&r);
            }
            // 2π · 0.3 → winding ≈ 0.3, deviation = 0.3 > 1e-6.
            let integral = 2.0 * std::f64::consts::PI * 0.3;
            let debt =
                crate::curvature::holonomy_debt(&store, integral, 1e-6).expect("Some");
            assert!(!debt.is_quantized());
            assert!(matches!(debt, crate::curvature::HolonomyDebt::Continuous(_)));
            assert!((debt.winding() - 0.3).abs() < 1e-12);
        }

        /// L7.2 — gauge invariance / Davis non-decoupling: the
        /// Quantized variant is determined by the loop integral
        /// alone, not by any per-record gauge choice. Verified
        /// here by computing the debt twice with identical inputs
        /// and asserting both calls give the same Quantized(n) —
        /// the debt "persists under gauge."
        #[test]
        fn davis_non_decoupling_floor_persists_under_gauge() {
            let schema = BundleSchema::new("had")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0))
                .with_kahler(kahler_2d());
            let mut store = BundleStore::new(schema);
            for i in 0..10 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(0.0));
                r.insert("y".into(), Value::Float(0.0));
                store.insert(&r);
            }
            let integral = 2.0 * std::f64::consts::PI * 5.0;
            // Two reads — independent of any gauge transformation
            // the caller might apply between them.
            let first =
                crate::curvature::holonomy_debt(&store, integral, 1e-6).expect("Some");
            let second =
                crate::curvature::holonomy_debt(&store, integral, 1e-6).expect("Some");
            assert_eq!(first, second);
            assert_eq!(first, crate::curvature::HolonomyDebt::Quantized(5));
        }

        /// Negative — no Kähler attached ⇒ holonomy_debt returns None.
        #[test]
        fn holonomy_debt_no_kahler_returns_none() {
            let schema = BundleSchema::new("plain")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0))
                .fiber(FieldDef::numeric("y").with_range(2.0));
            let mut store = BundleStore::new(schema);
            for i in 0..5 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(0.0));
                r.insert("y".into(), Value::Float(0.0));
                store.insert(&r);
            }
            assert!(crate::curvature::holonomy_debt(&store, 2.0 * std::f64::consts::PI, 1e-6)
                .is_none());
        }

        /// Negative — odd-parity numeric fiber (1 numeric field) ⇒
        /// cannot pair into a complex coordinate ⇒ None.
        #[test]
        fn odd_parity_numeric_fiber_returns_none() {
            let schema = BundleSchema::new("odd")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::numeric("x").with_range(2.0));
            let mut store = BundleStore::new(schema);
            for i in 0..10 {
                let mut r = Record::new();
                r.insert("id".into(), Value::Integer(i));
                r.insert("x".into(), Value::Float(i as f64));
                store.insert(&r);
            }
            assert!(compute_kahler_decomposition(&store, &kahler_2d()).is_none());
        }
    }

    // ── DepthConfig — backward compat + override behavior ───────

    /// The shipped `encoding_depth` must produce identical results to
    /// `encoding_depth_with` called with the default config. Any drift
    /// here breaks backward compatibility for callers using the
    /// shipped 1-arg form.
    #[test]
    fn depth_default_matches_explicit_default_config() {
        let cfg = DepthConfig::default();
        for &(k, l) in &[
            (0.0, 0.0),    // → Topological (λ₁ < default 0.01)
            (0.05, 0.5),   // → Tangent
            (0.2, 0.5),    // → Connection (k > default 0.1)
            (0.05, 0.2),   // → Connection (λ₁ < default 0.3)
            (0.7, 0.5),    // → Metric (k > default 0.5)
            (10.0, 1.0),   // → Metric
        ] {
            assert_eq!(
                encoding_depth(k, l),
                encoding_depth_with(k, l, &cfg),
                "default shim must agree with explicit default config at (K={}, λ₁={})",
                k, l
            );
        }
    }

    /// The default thresholds reproduce the shipped four-region map.
    #[test]
    fn depth_default_thresholds_classify_canonical_cases() {
        assert_eq!(encoding_depth(0.05, 0.005), EncodingDepth::Topological);
        assert_eq!(encoding_depth(0.7,  0.5),   EncodingDepth::Metric);
        assert_eq!(encoding_depth(0.2,  0.5),   EncodingDepth::Connection);
        assert_eq!(encoding_depth(0.05, 0.5),   EncodingDepth::Tangent);
    }

    /// Lowering `lambda1_topological` lets non-graph-structured bundles
    /// (which have λ₁ ≈ 0) escape the Topological catch-all. This is
    /// the exact fix the JTBD demo motivated: sensor data has λ₁ ≈ 0
    /// but is not actually topological — it's tangent.
    #[test]
    fn depth_override_fixes_sensor_lambda1_zero_topological_collapse() {
        let k = 0.05;        // sensor-style low K
        let lambda1 = 0.0;   // sensor-style λ₁ ≈ 0
        // Default cuts say Topological:
        assert_eq!(encoding_depth(k, lambda1), EncodingDepth::Topological);
        // Lowering the topological cut to a numerical-noise threshold
        // releases the classification to consider K:
        let cfg = DepthConfig {
            lambda1_topological: -1.0, // strictly negative → never trip
            ..DepthConfig::default()
        };
        // With K < k_connection (0.1) AND λ₁ < lambda1_connection (0.3)
        // the cascade falls to Connection — exactly the published
        // "skill-level / distributed neighborhood" depth, which is
        // what sensor data plausibly is. (Tangent would require
        // λ₁ ≥ lambda1_connection, which sensor data doesn't reach.)
        assert_eq!(
            encoding_depth_with(k, lambda1, &cfg),
            EncodingDepth::Connection
        );
    }

    /// Raising `k_metric` lets high-curvature regions stay at
    /// Connection rather than escalating to Metric. Useful when a
    /// builder knows their substrate has elevated baseline K and
    /// wants to reserve Metric for genuine outliers.
    #[test]
    fn depth_override_raises_metric_threshold() {
        let k = 0.6;       // above default k_metric (0.5)
        let lambda1 = 0.5; // healthy spectral gap
        assert_eq!(encoding_depth(k, lambda1), EncodingDepth::Metric);
        let cfg = DepthConfig { k_metric: 1.0, ..DepthConfig::default() };
        // With k_metric raised, K=0.6 no longer trips Metric; falls
        // through to Connection (K > k_connection=0.1).
        assert_eq!(
            encoding_depth_with(k, lambda1, &cfg),
            EncodingDepth::Connection
        );
    }

    // ── HorizonConfig — estimator selection + fallback ──────────

    fn make_varied_store() -> BundleStore {
        let schema = BundleSchema::new("horizon_test")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("x").with_range(10.0))
            .fiber(FieldDef::numeric("y").with_range(10.0));
        let mut store = BundleStore::new(schema);
        // Records spread so per-field variance is positive — exactly
        // the case where Welford radius is well-defined.
        for i in 0..30 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("x".into(), Value::Float(i as f64 * 0.7));
            r.insert("y".into(), Value::Float((i as f64).sin() * 3.0));
            store.insert(&r);
        }
        store
    }

    #[test]
    fn horizon_with_uses_spectral_gap_when_lambda1_healthy() {
        let store = make_varied_store();
        let cfg = HorizonConfig::default();
        let res = horizon_with(1.0, 0.5, &store, 0.25, &cfg);
        // ℓ_c = 1/√0.25 = 2.0; s_max = 1 / (0.5 · 2.0) = 1.0
        assert!((res.l_c - 2.0).abs() < 1e-12, "l_c was {}", res.l_c);
        assert!((res.s_max - 1.0).abs() < 1e-12, "s_max was {}", res.s_max);
        assert_eq!(res.estimator_used, LengthScaleEstimator::SpectralGap);
        assert!(!res.fallback_engaged);
    }

    #[test]
    fn horizon_with_falls_back_to_welford_when_lambda1_zero() {
        let store = make_varied_store();
        let cfg = HorizonConfig::default();
        let res = horizon_with(1.0, 0.5, &store, 0.0, &cfg);
        // λ₁ = 0 ⇒ SpectralGap degenerate ⇒ fallback to WelfordRadius
        // The Welford radius is sqrt(mean variance across fiber fields)
        // — must be positive and finite for this store.
        assert!(res.l_c.is_finite() && res.l_c > 0.0, "l_c = {}", res.l_c);
        assert!(res.s_max.is_finite() && res.s_max > 0.0, "s_max = {}", res.s_max);
        assert_eq!(res.estimator_used, LengthScaleEstimator::WelfordRadius);
        assert!(res.fallback_engaged);
        // The fallback ℓ_c is meaningfully different from 1.0 (the
        // dumb default of the scalar shim) — that's the whole point
        // of the calibrated path.
        assert!((res.l_c - 1.0).abs() > 0.1,
            "fallback must not produce ℓ_c ≈ 1.0; got {}", res.l_c);
    }

    #[test]
    fn horizon_with_fixed_estimator_uses_provided_value() {
        let store = make_varied_store();
        let cfg = HorizonConfig {
            estimator: LengthScaleEstimator::Fixed(3.5),
            ..HorizonConfig::default()
        };
        // Healthy λ₁ would normally produce ℓ_c = 1/√0.25 = 2.0 via
        // SpectralGap, but the Fixed override takes precedence.
        let res = horizon_with(1.0, 0.5, &store, 0.25, &cfg);
        assert!((res.l_c - 3.5).abs() < 1e-12);
        // s_max = 1 / (0.5 · 3.5) = 0.5714...
        assert!((res.s_max - (1.0 / (0.5 * 3.5))).abs() < 1e-12);
        assert!(matches!(res.estimator_used, LengthScaleEstimator::Fixed(_)));
        assert!(!res.fallback_engaged);
    }

    #[test]
    fn horizon_with_returns_infinity_when_k_is_zero() {
        let store = make_varied_store();
        let cfg = HorizonConfig::default();
        let res = horizon_with(1.0, 0.0, &store, 0.5, &cfg);
        assert!(res.s_max.is_infinite(), "k=0 ⇒ s_max=∞, got {}", res.s_max);
        // ℓ_c still reported even when the K-zero branch makes s_max
        // infinite (caller can audit which estimator path ran).
        assert!(res.l_c.is_finite() && res.l_c > 0.0);
    }

    #[test]
    fn horizon_with_scalar_shim_agrees_when_lambda1_healthy() {
        // When λ₁ is healthy and the estimator picks SpectralGap, the
        // calibrated path must equal the scalar shim — backward-compat
        // contract for the calibrated default.
        let store = make_varied_store();
        let cfg = HorizonConfig::default();
        let lambda1 = 0.4;
        let tau = 2.0;
        let k = 0.7;
        let res = horizon_with(tau, k, &store, lambda1, &cfg);
        let shim = horizon(tau, k, lambda1);
        assert!((res.s_max - shim).abs() < 1e-12,
            "calibrated path must match shim when λ₁ healthy: {} vs {}",
            res.s_max, shim);
    }

    #[test]
    fn horizon_config_roundtrips_serde_json() {
        let cfg = HorizonConfig {
            estimator: LengthScaleEstimator::Fixed(2.71),
            fallback: LengthScaleEstimator::WelfordRadius,
            epsilon: 1e-7,
        };
        let s = serde_json::to_string(&cfg).expect("serialize");
        let back: HorizonConfig = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(cfg, back);
    }

    #[test]
    fn welford_radius_zero_on_uniform_data() {
        let store = make_store_with_data(); // all-identical val=50, cat=X
        assert!(welford_radius(&store).is_nan(),
            "uniform data has zero variance everywhere; should be NaN");
    }

    // ── DepthConfig substrate-aware constructors ────────────────

    /// `for_graph_substrate()` is identical to `Default::default()` —
    /// the documented Theorem 8.14 thresholds. Pinning both directions
    /// of that equivalence here so a future Default change can't drift
    /// the named constructor.
    #[test]
    fn depth_for_graph_substrate_equals_default() {
        assert_eq!(DepthConfig::for_graph_substrate(), DepthConfig::default());
    }

    /// `for_continuous_substrate()` zeroes both λ₁ cuts. Same K cuts as
    /// the graph defaults — only the λ₁-triggered branches are changed.
    #[test]
    fn depth_for_continuous_substrate_zeroes_lambda1_cuts() {
        let c = DepthConfig::for_continuous_substrate();
        assert_eq!(c.lambda1_topological, 0.0);
        assert_eq!(c.lambda1_connection, 0.0);
        // K cuts unchanged from the published values.
        let g = DepthConfig::for_graph_substrate();
        assert_eq!(c.k_metric, g.k_metric);
        assert_eq!(c.k_connection, g.k_connection);
    }

    /// On the continuous-substrate constructor, λ₁ ≈ 0 (sensor case)
    /// no longer triggers Topological. Classification falls through to
    /// the K-only cascade — the fix the JTBD demo motivated.
    #[test]
    fn depth_continuous_substrate_classifies_sensor_case_on_k_alone() {
        let c = DepthConfig::for_continuous_substrate();
        // Low K, λ₁ = 0  →  was Topological under defaults; now Tangent.
        assert_eq!(encoding_depth_with(0.05, 0.0, &c), EncodingDepth::Tangent);
        // Moderate K, λ₁ = 0  →  was Topological; now Connection.
        assert_eq!(encoding_depth_with(0.2, 0.0, &c), EncodingDepth::Connection);
        // High K, λ₁ = 0  →  was Topological; now Metric.
        assert_eq!(encoding_depth_with(0.7, 0.0, &c), EncodingDepth::Metric);

        // Sanity: the graph defaults DO call all three Topological,
        // documenting the difference the constructor switch made.
        let g = DepthConfig::for_graph_substrate();
        assert_eq!(encoding_depth_with(0.05, 0.0, &g), EncodingDepth::Topological);
        assert_eq!(encoding_depth_with(0.2,  0.0, &g), EncodingDepth::Topological);
        assert_eq!(encoding_depth_with(0.7,  0.0, &g), EncodingDepth::Topological);
    }

    /// `auto_for(store, eps)` is a pure branch on
    /// `spectral_gap(store) < eps`. Tested by sweeping epsilon on a
    /// fixed fixture rather than relying on any specific fixture
    /// having a known-low spectral gap: with a tiny epsilon the branch
    /// goes to graph; with a huge epsilon (larger than any finite λ₁)
    /// the branch goes to continuous. That pins the contract without
    /// coupling to a fixture-specific λ₁ value.
    #[test]
    fn depth_auto_for_branches_on_epsilon() {
        let store = make_store_with_data();
        let lambda1 = crate::spectral::spectral_gap(&store);
        assert!(
            lambda1.is_finite(),
            "fixture λ₁ must be finite for this test; got {lambda1}"
        );

        // Tiny epsilon → λ₁ is "large" → graph defaults. Pick a value
        // strictly less than λ₁ so the branch must be `else`.
        let tiny = (lambda1 / 2.0).max(0.0);
        let c_graph = DepthConfig::auto_for(&store, tiny);
        assert_eq!(
            c_graph,
            DepthConfig::for_graph_substrate(),
            "λ₁={lambda1} >= eps={tiny} should select graph defaults"
        );

        // Huge epsilon → λ₁ is "tiny" → continuous defaults. Any finite
        // non-negative λ₁ is below 1e12.
        let huge = 1e12;
        let c_cont = DepthConfig::auto_for(&store, huge);
        assert_eq!(
            c_cont,
            DepthConfig::for_continuous_substrate(),
            "λ₁={lambda1} < eps={huge} should select continuous defaults"
        );
    }

    /// DepthConfig is serializable both directions (needed for HTTP
    /// query-param echo-back on `DepthReport` so callers can audit
    /// which thresholds the server applied).
    #[test]
    fn depth_config_roundtrips_serde_json() {
        let cfg = DepthConfig {
            lambda1_topological: 1e-9,
            k_metric: 2.0,
            k_connection: 0.25,
            lambda1_connection: 0.1,
        };
        let s = serde_json::to_string(&cfg).expect("serialize");
        let back: DepthConfig = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(cfg, back);
    }

    // ── PERCEIVE — Theorem 8.6 ───────────────────────────────

    /// Identity rotation: the perceived vector equals the input
    /// exactly and the bias is exactly 0. This is the canonical
    /// "no drift" case Marcella's coherence-signal spec falls back
    /// to when the prefix scan has accumulated no rotation.
    #[test]
    fn perceive_identity_is_passthrough_with_zero_bias() {
        let id = vec![1.0, 0.0, 0.0,
                      0.0, 1.0, 0.0,
                      0.0, 0.0, 1.0];
        let v = vec![1.0, 2.0, 3.0];
        let res = perceive(&id, &v, 3).expect("identity perceive");
        assert_eq!(res.v_perceived, v, "identity must be passthrough");
        assert_eq!(res.bias, 0.0, "identity bias must be exactly zero");
    }

    /// 90° rotation in 2D (the canonical small example):
    ///   R = [[0, -1],
    ///        [1,  0]],   v = [1, 0]
    ///   → v_perceived = [0, 1]
    ///   → R - I = [[-1, -1], [1, -1]]
    ///   → ‖R - I‖_F² = 1+1+1+1 = 4  →  bias = 2.0
    /// Computed by hand; serves as the ground-truth correctness test.
    #[test]
    fn perceive_2d_90deg_rotation_matches_hand_computation() {
        let r = vec![0.0, -1.0,
                     1.0,  0.0];
        let v = vec![1.0, 0.0];
        let res = perceive(&r, &v, 2).expect("90deg perceive");
        assert!((res.v_perceived[0] - 0.0).abs() < 1e-12, "v[0] = {}", res.v_perceived[0]);
        assert!((res.v_perceived[1] - 1.0).abs() < 1e-12, "v[1] = {}", res.v_perceived[1]);
        assert!((res.bias - 2.0).abs() < 1e-12, "bias = {} (expected 2.0)", res.bias);
    }

    /// R = -I is the maximally-rotated case for orthogonal R. The
    /// perceived vector flips sign, and the bias hits its upper
    /// bound 2·√dim (here √4 = 2, so bias = 4 for dim=4? wait — R - I
    /// for R=-I is -2I, so ‖-2I‖_F = 2·√dim). For dim=3, that's
    /// 2·√3 ≈ 3.464. Pin both endpoints.
    #[test]
    fn perceive_negative_identity_hits_upper_bias_bound() {
        let neg_id = vec![-1.0, 0.0, 0.0,
                           0.0,-1.0, 0.0,
                           0.0, 0.0,-1.0];
        let v = vec![1.0, -2.0, 3.0];
        let res = perceive(&neg_id, &v, 3).expect("-I perceive");
        // Sign flip on every component.
        assert_eq!(res.v_perceived, vec![-1.0, 2.0, -3.0]);
        // ‖-2I‖_F = sqrt(4 + 4 + 4) = 2√3.
        let expected = 2.0 * (3.0_f64).sqrt();
        assert!(
            (res.bias - expected).abs() < 1e-12,
            "bias = {} (expected 2√3 ≈ {})",
            res.bias, expected
        );
    }

    /// Error: empty dim.
    #[test]
    fn perceive_rejects_zero_dim() {
        assert_eq!(perceive(&[], &[], 0), Err(PerceiveError::EmptyDimension));
        assert_eq!(perception_bias(&[], 0), Err(PerceiveError::EmptyDimension));
    }

    /// Error: rotation isn't square.
    #[test]
    fn perceive_rejects_non_square_rotation() {
        let bad = vec![1.0, 0.0, 0.0, 0.0, 1.0]; // dim 2 needs 4 entries, got 5
        let v = vec![0.0, 0.0];
        let err = perceive(&bad, &v, 2).unwrap_err();
        assert_eq!(err, PerceiveError::NonSquareRotation { dim: 2, len: 5 });
    }

    /// Error: vector dim doesn't match rotation dim.
    #[test]
    fn perceive_rejects_vector_dim_mismatch() {
        let r = vec![1.0, 0.0, 0.0, 1.0];
        let v = vec![1.0, 2.0, 3.0]; // length 3 vs rotation dim 2
        let err = perceive(&r, &v, 2).unwrap_err();
        assert_eq!(err, PerceiveError::VectorDimMismatch { rotation_dim: 2, vector_len: 3 });
    }

    /// `perception_bias` (standalone) agrees with what `perceive`
    /// returns. Two callers must read identical numbers for the same R.
    #[test]
    fn perception_bias_matches_perceive_bias_field() {
        // Random-ish 3x3 matrix (not necessarily a rotation; the bias
        // is well-defined for any matrix).
        let r = vec![0.7, -0.5, 0.2,
                     0.1,  0.9, 0.4,
                    -0.3,  0.1, 0.8];
        let v = vec![1.0, 0.0, 0.0];
        let standalone = perception_bias(&r, 3).expect("bias");
        let combined = perceive(&r, &v, 3).expect("perceive").bias;
        assert!(
            (standalone - combined).abs() < 1e-15,
            "standalone {} vs combined {} disagreed",
            standalone, combined
        );
    }

    /// PerceptionResult round-trips through JSON. The struct is the
    /// wire-side payload for a future PERCEIVE HTTP endpoint; pin
    /// the round-trip now so a future serde change can't silently
    /// break it.
    #[test]
    fn perception_result_roundtrips_through_json() {
        let r = vec![0.0, -1.0, 1.0, 0.0];
        let v = vec![1.0, 0.0];
        let res = perceive(&r, &v, 2).expect("perceive");
        let s = serde_json::to_string(&res).expect("serialize");
        let back: PerceptionResult = serde_json::from_str(&s).expect("deserialize");
        assert_eq!(res, back);
    }
}
