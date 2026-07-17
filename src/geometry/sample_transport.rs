//! SAMPLE_TRANSPORT — curvature-bounded neighborhood sampling on the fiber.
//!
//! Implements the `SAMPLE_TRANSPORT` GQL verb from the sprint spec:
//! given a source fiber point `p_src`, return `k` draws from the
//! neighborhood
//!
//! ```text
//! N(p_src, tau) := { p in E : d^2(p_src, p) <= tau }
//! ```
//!
//! where `d^2` is the normalized half-chord distance from the
//! Double Cover (`S + d^2 = 1`, `S = cos^2(theta/2)`):
//!
//! ```text
//! d^2(p_src, p) = (1 - cos(theta)) / 2    in [0, 1]
//! ```
//!
//! Candidates are weighted by `exp(-beta * d^2)` (default `beta = 1`),
//! sampled without replacement via the Efraimidis-Spirakis priority
//! algorithm (`r^(1/w)` keys, take top-k).
//!
//! This module is the in-process Rust primitive. The HTTP surface
//! (`/v1/bundles/{name}/brain/sample_transport`) and GQL verb live in
//! `gigi_stream.rs` and `parser.rs` respectively.
//!
//! # Relation to the Davis Law budget
//!
//! The spec interprets `tau` as a maximum admissible `d^2` per
//! candidate under the Double Cover `S + d^2 = 1`. From
//! `K^2 = 4 sin^2(theta/2) = 4 d^2`, the budget is equivalent to a
//! maximum curvature-per-hop: `K <= 2 * sqrt(tau)`. `BUDGET 0.3`
//! admits candidates within `K <= 1.095`.

use crate::geometry::generative_flow::SmallRng;
use crate::types::{Record, Value};

// ── Public types ────────────────────────────────────────────────

/// Request parameters for a sample-transport query.
#[derive(Debug, Clone)]
pub struct SampleTransportRequest {
    /// Fields that define the fiber projection. Must be numeric.
    pub fiber_fields: Vec<String>,
    /// Maximum squared chord distance `d^2 in [0, 1]`. Budget = 0
    /// admits only records with identical fiber direction; budget = 1
    /// admits the whole fiber.
    pub budget: f64,
    /// Number of candidates to return (capped at `n_admissible`).
    pub k: usize,
    /// Temperature for the exp-kernel `exp(-beta * d^2)`.
    /// Default: 1.0 (monotonically decreasing with distance).
    pub beta: f64,
    /// Optional deterministic seed for reproducibility. `None`
    /// draws from a time-based entropy source.
    pub seed: Option<u64>,
}

impl Default for SampleTransportRequest {
    fn default() -> Self {
        Self {
            fiber_fields: Vec::new(),
            budget: 0.3,
            k: 16,
            beta: 1.0,
            seed: None,
        }
    }
}

/// A single candidate in the transport neighborhood.
#[derive(Debug, Clone)]
pub struct TransportCandidate {
    /// Full record (includes key fields and all base/fiber fields).
    pub record: Record,
    /// Fiber projection vector (one float per fiber field).
    pub fiber_projection: Vec<f64>,
    /// Squared chord distance `d^2 in [0, budget]`.
    pub d_sq: f64,
    /// Sameness `S = 1 - d^2` (Double Cover identity `S + d^2 = 1`).
    pub sameness: f64,
    /// Sampling weight `exp(-beta * d^2)`. Unnormalized.
    pub weight: f64,
    /// Local curvature proxy `K = 2 * sqrt(d^2)`.
    pub curvature_k: f64,
}

/// Result of a `sample_transport_neighborhood` call.
#[derive(Debug, Clone)]
pub struct SampleTransportResult {
    /// Up to `k` sampled candidates, in sampling-priority order.
    pub candidates: Vec<TransportCandidate>,
    /// Budget `tau` used for this query.
    pub budget: f64,
    /// Total records in `N(p_src, tau)` before sampling.
    pub n_admissible: usize,
    /// Actual number of candidates returned (`<= min(k, n_admissible)`).
    pub n_returned: usize,
    /// Bundle-level scalar curvature `kappa` (passed in by caller).
    pub kappa: f64,
    /// Confidence `= 1 / (1 + kappa)`.
    pub confidence: f64,
}

/// Failure modes for `sample_transport_neighborhood`.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SampleTransportError {
    /// `fiber_fields` is empty — cannot project onto a zero-dimensional fiber.
    #[error("no fiber fields specified")]
    EmptyFiber,
    /// Source fiber vector is degenerate (all-zero norm < 1e-12).
    #[error("source fiber point is degenerate (zero-norm vector)")]
    DegenerateSrc,
    /// Budget not in `[0, 1]`.
    #[error("budget {0} out of range; must be in [0.0, 1.0]")]
    BudgetOutOfRange(f64),
}

// ── Core algorithm ──────────────────────────────────────────────

/// Squared chord distance between two fiber projections on the
/// unit sphere (half-angle formula): `d^2 = (1 - cos theta) / 2`.
///
/// Returns `1.0` (maximum distance) when either vector is degenerate
/// so degenerate records are never admitted unless budget = 1.
pub(crate) fn fiber_d_sq(p_src: &[f64], p: &[f64]) -> f64 {
    let n = p_src.len().min(p.len());
    let dot: f64 = p_src[..n].iter().zip(p[..n].iter()).map(|(a, b)| a * b).sum();
    let norm_s = p_src[..n].iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_p = p[..n].iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_s < 1e-12 || norm_p < 1e-12 {
        return 1.0;
    }
    let cos_t = (dot / (norm_s * norm_p)).clamp(-1.0, 1.0);
    (1.0 - cos_t) / 2.0
}

/// Extract the fiber projection of a record as a `Vec<f64>`.
///
/// Scalar numeric fields contribute one element; `Vector` fields
/// contribute all of their components in order. Missing or
/// non-numeric scalar fields project to `0.0`.
///
/// **Surfaced live 2026-06-04**: the previous implementation called
/// `v.as_f64()` blindly, which silently returned `None` for
/// `Value::Vector` and collapsed a 4-d emb into a single `0.0`.
/// That triggered `SampleTransportError::DegenerateSrc` on every
/// vector-fiber bundle's SAMPLE_TRANSPORT request.
pub fn extract_fiber(record: &Record, fiber_fields: &[String]) -> Vec<f64> {
    let mut out: Vec<f64> = Vec::with_capacity(fiber_fields.len());
    for f in fiber_fields {
        match record.get(f.as_str()) {
            Some(crate::types::Value::Vector(v)) => out.extend(v.iter().copied()),
            Some(v) => out.push(v.as_f64().unwrap_or(0.0)),
            None => out.push(0.0),
        }
    }
    out
}

/// Weighted sampling without replacement — Efraimidis-Spirakis
/// priority keys: `r^(1/w)` for uniform `r in (0, 1)`. Take top-k.
///
/// Returns all `candidates` when `k >= candidates.len()`.
fn weighted_sample_k(
    candidates: &[TransportCandidate],
    k: usize,
    rng: &mut SmallRng,
) -> Vec<TransportCandidate> {
    if k == 0 || candidates.is_empty() {
        return Vec::new();
    }
    if k >= candidates.len() {
        return candidates.to_vec();
    }
    let mut priorities: Vec<(f64, usize)> = candidates
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let u = rng.uniform().max(1e-300);
            let w = c.weight.max(1e-300);
            (u.powf(1.0 / w), i)
        })
        .collect();
    // Sort descending by priority — highest priority is drawn first.
    priorities.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    priorities.truncate(k);
    priorities
        .iter()
        .map(|(_, i)| candidates[*i].clone())
        .collect()
}

/// Sample from the transport neighborhood of `src_fiber`.
///
/// ## Arguments
///
/// - `records` — full set of records in the bundle (or any slice the
///   caller wants to search; typically `bundle.records().collect()`).
/// - `src_fiber` — fiber projection of the source point, extracted
///   from the source record by the caller via `extract_fiber`.
/// - `req` — query parameters (budget, k, beta, seed, fiber_fields).
/// - `kappa` — bundle-level scalar curvature; attached to the response
///   for the caller; not used by the sampling algorithm itself.
///
/// ## Algorithm
///
/// 1. For each record, extract fiber projection and compute `d_sq`.
/// 2. Admit records with `d_sq <= budget`.
/// 3. Weight each by `exp(-beta * d_sq)`.
/// 4. Sample `k` without replacement (Efraimidis-Spirakis).
/// 5. Return result with analytics.
pub fn sample_transport_neighborhood(
    records: &[Record],
    src_fiber: &[f64],
    req: &SampleTransportRequest,
    kappa: f64,
) -> Result<SampleTransportResult, SampleTransportError> {
    if req.fiber_fields.is_empty() {
        return Err(SampleTransportError::EmptyFiber);
    }
    if req.budget < 0.0 || req.budget > 1.0 {
        return Err(SampleTransportError::BudgetOutOfRange(req.budget));
    }
    let src_norm: f64 = src_fiber.iter().map(|x| x * x).sum::<f64>().sqrt();
    if src_norm < 1e-12 {
        return Err(SampleTransportError::DegenerateSrc);
    }

    let admissible: Vec<TransportCandidate> = records
        .iter()
        .filter_map(|rec| {
            let proj = extract_fiber(rec, &req.fiber_fields);
            let d_sq = fiber_d_sq(src_fiber, &proj);
            if d_sq > req.budget {
                return None;
            }
            let sameness = 1.0 - d_sq;
            let weight = (-req.beta * d_sq).exp();
            let curvature_k = 2.0 * d_sq.sqrt();
            Some(TransportCandidate {
                record: rec.clone(),
                fiber_projection: proj,
                d_sq,
                sameness,
                weight,
                curvature_k,
            })
        })
        .collect();

    let n_admissible = admissible.len();
    let mut rng = SmallRng::seed_or_entropy(req.seed);
    let sampled = weighted_sample_k(&admissible, req.k, &mut rng);
    let n_returned = sampled.len();
    let confidence = 1.0 / (1.0 + kappa);

    Ok(SampleTransportResult {
        candidates: sampled,
        budget: req.budget,
        n_admissible,
        n_returned,
        kappa,
        confidence,
    })
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;
    use std::collections::HashMap;

    // ── helpers ─────────────────────────────────────────────────

    fn rec(fields: &[(&str, f64)]) -> Record {
        let mut m: Record = HashMap::new();
        for (k, v) in fields {
            m.insert(k.to_string(), Value::Float(*v));
        }
        m
    }

    fn fiber_fields(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    /// **Regression — surfaced live 2026-06-04.** `extract_fiber` used
    /// to call `v.as_f64()` blindly, collapsing a `Value::Vector`
    /// into `[0.0]` and breaking SAMPLE_TRANSPORT on every vector-
    /// fiber bundle. This test pins the unpacking behavior.
    #[test]
    fn extract_fiber_unpacks_vector_components() {
        let mut r: Record = HashMap::new();
        r.insert("emb".to_string(), Value::Vector(vec![2.0, 3.0, 4.0, 5.0]));
        let f = fiber_fields(&["emb"]);
        let out = extract_fiber(&r, &f);
        assert_eq!(out, vec![2.0, 3.0, 4.0, 5.0]);
    }

    #[test]
    fn extract_fiber_concatenates_scalar_and_vector_fields() {
        let mut r: Record = HashMap::new();
        r.insert("price".to_string(), Value::Float(9.5));
        r.insert("emb".to_string(), Value::Vector(vec![1.0, 2.0, 3.0]));
        r.insert("count".to_string(), Value::Integer(7));
        let f = fiber_fields(&["price", "emb", "count"]);
        let out = extract_fiber(&r, &f);
        assert_eq!(out, vec![9.5, 1.0, 2.0, 3.0, 7.0]);
    }

    #[test]
    fn extract_fiber_missing_field_defaults_to_zero_scalar() {
        let r: Record = HashMap::new();
        let f = fiber_fields(&["nope"]);
        assert_eq!(extract_fiber(&r, &f), vec![0.0]);
    }

    // Corpus spread across the unit circle in 2D fiber.
    // Angles: 0, pi/4, pi/2, 3pi/4, pi (5 distinct points).
    fn make_circle_corpus() -> Vec<Record> {
        use std::f64::consts::PI;
        (0..5)
            .map(|i| {
                let theta = i as f64 * PI / 4.0;
                rec(&[("f0", theta.cos()), ("f1", theta.sin())])
            })
            .collect()
    }

    // ── T1: empty budget ────────────────────────────────────────

    /// **T1.** `sample_transport_empty_budget` — BUDGET 0.0 with a
    /// corpus where no record has an identical fiber direction to src.
    ///
    /// Corpus: 4 records at angles pi/4, pi/2, 3pi/4, pi.
    /// src_fiber: (1, 0) (angle 0, NOT in corpus).
    /// d_sq((1,0), each) > 0, so n_admissible = 0 and n_returned = 0.
    #[test]
    fn sample_transport_empty_budget() {
        use std::f64::consts::PI;
        let corpus: Vec<Record> = (1..5)
            .map(|i| {
                let theta = i as f64 * PI / 4.0;
                rec(&[("f0", theta.cos()), ("f1", theta.sin())])
            })
            .collect();
        let src = vec![1.0_f64, 0.0_f64]; // angle 0, not in corpus
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 0.0,
            k: 16,
            beta: 1.0,
            seed: Some(1),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.1).unwrap();
        assert_eq!(res.n_admissible, 0, "budget=0 with no identical fiber => 0 admissible");
        assert_eq!(res.n_returned, 0, "budget=0 with no identical fiber => 0 returned");
    }

    // ── T2: full budget ─────────────────────────────────────────

    /// **T2.** `sample_transport_full_budget` — BUDGET 1.0 admits
    /// every record regardless of direction.
    ///
    /// All d_sq <= 1.0 always (since d_sq in [0, 1]).
    #[test]
    fn sample_transport_full_budget() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 1.0,
            k: 100,
            beta: 1.0,
            seed: Some(2),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        assert_eq!(
            res.n_admissible,
            corpus.len(),
            "budget=1.0 must admit entire corpus"
        );
        assert_eq!(
            res.n_returned,
            corpus.len(),
            "k > n => n_returned = n_admissible"
        );
    }

    // ── T3: seed reproducibility ─────────────────────────────────

    /// **T3.** `sample_transport_seed_reproducibility` — identical
    /// seeds produce identical candidate ordering.
    ///
    /// Uses a corpus of 20 records with varying d_sq, k=5, budget=0.8.
    /// Two calls with SEED 42 must return the same candidate sequence.
    #[test]
    fn sample_transport_seed_reproducibility() {
        use std::f64::consts::PI;
        let corpus: Vec<Record> = (0..20)
            .map(|i| {
                let theta = i as f64 * PI / 10.0;
                rec(&[("x", theta.cos()), ("y", theta.sin())])
            })
            .collect();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["x", "y"]),
            budget: 0.8,
            k: 5,
            beta: 1.0,
            seed: Some(42),
        };
        let r1 = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        let r2 = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        assert_eq!(r1.n_returned, r2.n_returned, "same seed => same n_returned");
        for (a, b) in r1.candidates.iter().zip(r2.candidates.iter()) {
            assert!(
                (a.d_sq - b.d_sq).abs() < 1e-12,
                "same seed => same d_sq: {} vs {}",
                a.d_sq,
                b.d_sq
            );
        }
    }

    // ── T4: default weight kernel ────────────────────────────────

    /// **T4.** `sample_transport_weight_default` — beta=1.0 kernel
    /// gives weight = exp(-1 * d_sq) for every candidate.
    #[test]
    fn sample_transport_weight_default() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 1.0,
            k: 100,
            beta: 1.0,
            seed: Some(3),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        for c in &res.candidates {
            let expected = (-1.0 * c.d_sq).exp();
            assert!(
                (c.weight - expected).abs() < 1e-12,
                "weight {} != exp(-d_sq) = {} for d_sq={}",
                c.weight,
                expected,
                c.d_sq
            );
        }
    }

    // ── T5: custom weight kernel (via beta) ───────────────────────

    /// **T5.** `sample_transport_weight_custom` — beta=3.0 gives
    /// weight = exp(-3 * d_sq). Verifies the beta parameter is
    /// threaded into the kernel.
    #[test]
    fn sample_transport_weight_custom_beta() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 1.0,
            k: 100,
            beta: 3.0,
            seed: Some(4),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        for c in &res.candidates {
            let expected = (-3.0 * c.d_sq).exp();
            assert!(
                (c.weight - expected).abs() < 1e-12,
                "weight {} != exp(-3*d_sq) = {} for d_sq={}",
                c.weight,
                expected,
                c.d_sq
            );
        }
    }

    // ── T6: n_admissible vs n_returned ───────────────────────────

    /// **T6.** `sample_transport_n_admissible_vs_returned` — when the
    /// admissible set is smaller than k, n_returned = n_admissible.
    ///
    /// Strategy: corpus of 50 records spread around the unit circle in
    /// 2D. src at angle 0. Budget = 0.04 (admits ~4 records within
    /// cos(theta) > 0.92). k = 100 (much larger). Verify
    /// n_returned = n_admissible < k.
    #[test]
    fn sample_transport_n_admissible_vs_returned() {
        use std::f64::consts::PI;
        let corpus: Vec<Record> = (0..50)
            .map(|i| {
                let theta = i as f64 * 2.0 * PI / 50.0;
                rec(&[("a", theta.cos()), ("b", theta.sin())])
            })
            .collect();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["a", "b"]),
            budget: 0.04,
            k: 100,
            beta: 1.0,
            seed: Some(5),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        assert!(
            res.n_admissible < req.k,
            "expected admissible < k, got {} vs {}",
            res.n_admissible,
            req.k
        );
        assert_eq!(
            res.n_returned, res.n_admissible,
            "n_returned must equal n_admissible when admissible < k"
        );
    }

    // ── T7: budget monotonic ─────────────────────────────────────

    /// **T7.** `sample_transport_budget_monotonic` — increasing the
    /// budget never decreases n_admissible.
    #[test]
    fn sample_transport_budget_monotonic() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let budgets = [0.0, 0.1, 0.2, 0.4, 0.7, 1.0];
        let mut prev_admissible = 0_usize;
        for &b in &budgets {
            let req = SampleTransportRequest {
                fiber_fields: fiber_fields(&["f0", "f1"]),
                budget: b,
                k: 100,
                beta: 1.0,
                seed: Some(6),
            };
            let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
            assert!(
                res.n_admissible >= prev_admissible,
                "n_admissible({}) = {} < n_admissible({}) = {}",
                b,
                res.n_admissible,
                budgets[budgets.iter().position(|x| *x == b).unwrap().saturating_sub(1)],
                prev_admissible
            );
            prev_admissible = res.n_admissible;
        }
    }

    // ── T8: d_sq bounded ─────────────────────────────────────────

    /// **T8.** `sample_transport_d_sq_bounded` — every returned
    /// candidate has `d_sq <= budget` exactly.
    #[test]
    fn sample_transport_d_sq_bounded() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let budget = 0.3;
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget,
            k: 100,
            beta: 1.0,
            seed: Some(7),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        for c in &res.candidates {
            assert!(
                c.d_sq <= budget + 1e-12,
                "candidate d_sq {} exceeds budget {}",
                c.d_sq,
                budget
            );
        }
    }

    // ── T9: kappa passthrough ────────────────────────────────────

    /// **T9.** `sample_transport_kappa_passthrough` — the `kappa`
    /// parameter is echoed in the result; confidence = 1/(1+kappa).
    #[test]
    fn sample_transport_kappa_passthrough() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 1.0,
            k: 10,
            beta: 1.0,
            seed: Some(8),
        };
        let kappa = 0.7_f64;
        let res = sample_transport_neighborhood(&corpus, &src, &req, kappa).unwrap();
        assert!(
            (res.kappa - kappa).abs() < 1e-12,
            "kappa not echoed: {} vs {}",
            res.kappa,
            kappa
        );
        let expected_conf = 1.0 / (1.0 + kappa);
        assert!(
            (res.confidence - expected_conf).abs() < 1e-12,
            "confidence {} != 1/(1+kappa) = {}",
            res.confidence,
            expected_conf
        );
    }

    // ── T10: error cases ─────────────────────────────────────────

    /// **T10a.** EmptyFiber error when fiber_fields is empty.
    #[test]
    fn sample_transport_error_empty_fiber() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: vec![],
            budget: 0.5,
            ..Default::default()
        };
        assert_eq!(
            sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap_err(),
            SampleTransportError::EmptyFiber
        );
    }

    /// **T10b.** BudgetOutOfRange when budget > 1.0.
    #[test]
    fn sample_transport_error_budget_out_of_range() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 1.5,
            ..Default::default()
        };
        assert!(matches!(
            sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap_err(),
            SampleTransportError::BudgetOutOfRange(_)
        ));
    }

    /// **T10c.** DegenerateSrc when src_fiber is all zeros.
    #[test]
    fn sample_transport_error_degenerate_src() {
        let corpus = make_circle_corpus();
        let src = vec![0.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 0.5,
            ..Default::default()
        };
        assert_eq!(
            sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap_err(),
            SampleTransportError::DegenerateSrc
        );
    }

    // ── T11: sameness invariant ──────────────────────────────────

    /// **T11.** `sameness = 1 - d_sq` and `curvature_k = 2 * sqrt(d_sq)`
    /// hold exactly for every candidate.
    #[test]
    fn sample_transport_sameness_and_curvature_k_invariants() {
        let corpus = make_circle_corpus();
        let src = vec![1.0_f64, 0.0_f64];
        let req = SampleTransportRequest {
            fiber_fields: fiber_fields(&["f0", "f1"]),
            budget: 1.0,
            k: 100,
            beta: 1.0,
            seed: Some(9),
        };
        let res = sample_transport_neighborhood(&corpus, &src, &req, 0.0).unwrap();
        for c in &res.candidates {
            assert!(
                (c.sameness - (1.0 - c.d_sq)).abs() < 1e-12,
                "sameness={} != 1-d_sq={}",
                c.sameness,
                1.0 - c.d_sq
            );
            assert!(
                (c.curvature_k - 2.0 * c.d_sq.sqrt()).abs() < 1e-12,
                "curvature_k={} != 2*sqrt(d_sq)={}",
                c.curvature_k,
                2.0 * c.d_sq.sqrt()
            );
        }
    }
}
