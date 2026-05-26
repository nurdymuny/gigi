//! # SUDOKU — Constrained inference on a learned affordance manifold
//!
//! Implements the meta-primitive specified in
//! `theory/kahler_upgrade/SUDOKU_PRIMITIVE_SPEC.md` v0.3.
//!
//! ## The shape
//!
//! Given a problem expressed as a set of constraints over fiber
//! field values (plus optional context conditioning on base fields),
//! SUDOKU walks the bundle's records, filters by constraints, and
//! returns the matching options ranked by stated-norm prior mass
//! — the **load-bearing honest-coverage contract** is the API
//! choice that distinguishes "feasible region is empty" from
//! "I stopped exploring too early."
//!
//! ## Why this lives in the geometry module
//!
//! SUDOKU composes existing brain primitives (SAMPLE, ATTEND,
//! EPISODIC, SEMANTIC, INPAINT, EXPLAIN). Its value-add over
//! direct composition is:
//!
//!   1. **Coverage estimation** — fraction of prior mass actually
//!      explored, not just "I returned an answer."
//!   2. **Near-miss enumeration** — options that violate one
//!      constraint, ranked by which constraint to relax.
//!   3. **unsat tristate contract** — `Sat` / `Unsat` / `Unknown`.
//!      Most solvers shrug; SUDOKU refuses to shrug.
//!
//! ## What's in scope for S2 (this commit) vs deferred
//!
//! - **S2 (here):** core constraint vocabulary (field predicates),
//!   record-walk filtering, mass-based coverage estimator, near-miss
//!   enumeration, unsat tristate verdict, exhaustive O(N) candidate
//!   enumeration. All deterministic — no Langevin sampling.
//! - **S3 (HTTP endpoint):** wire `/v1/bundles/{name}/brain/sudoku`,
//!   content negotiation, request validation.
//! - **S3.5 (puzzle expansion):** constraint_relaxation +
//!   bundle_hop. Drops a constraint or queries a related bundle
//!   when the original puzzle is UNSAT.
//! - **S4 (calibration):** soft constraints + cross-field relations,
//!   manifold-distance constraints.
//! - **S5 (demos):** literal 9×9 sudoku puzzle + transport-norms
//!   school commute + 10×10 expansion demo.
//! - **S6 (docs):** consumer guide updates.
//!
//! ## v1 limitations explicit
//!
//! - **Constraints in v1:** only `Constraint::Field` (predicates over
//!   one fiber field). Manifold-distance and cross-field-relation
//!   constraint types defined in the spec are stubbed in the enum
//!   but route to a `TODO_S4` error if used. S4 fills them in.
//! - **Sampling in v1:** exhaustive record walk. The spec calls for
//!   SAMPLE/ATTEND-based exploration with prior-mass coverage; for
//!   small-N bundles (≤10k records — most of Marcella's) exhaustive
//!   is faster and gives exact coverage = 1.0 always. For large
//!   bundles (chembl at 9M), we'll add stratified sampling in S4.
//! - **Expansion in v1:** disabled. S3.5 ships the constraint_relaxation
//!   + bundle_hop layers.

use crate::types::{Record, Value};

// ─── Constraint vocabulary ──────────────────────────────────────

/// A single constraint on a fiber field value.
///
/// `is_hard = true` means a violation excludes the option; `false`
/// makes it a penalty for soft scoring (deferred to S4 — currently
/// treated as hard).
#[derive(Debug, Clone)]
pub enum Constraint {
    /// Predicate over one fiber field. The workhorse v1 constraint.
    Field {
        field: String,
        op: FieldOp,
        hard: bool,
    },

    /// Manifold-distance constraint (spec §2 Type 2). Stub for v1 —
    /// returns `SudokuError::NotYetSupported` until S4 lands it.
    /// Kept in the enum so consumers can type-stable construct it
    /// today; spec contracts what the request shape is.
    #[allow(dead_code)]
    Manifold {
        field: String,
        near_manifold: String,
        epsilon: f64,
        hard: bool,
    },

    /// Cross-field algebraic relation (spec §2 Type 3). Stub for
    /// v1 — S4 ships the parser + evaluator.
    #[allow(dead_code)]
    Relation {
        expr: String,
        vars: std::collections::HashMap<String, f64>,
        hard: bool,
    },
}

/// Field predicate operators.
#[derive(Debug, Clone)]
pub enum FieldOp {
    /// Equality with a single value.
    Eq(Value),
    /// Inequality.
    Ne(Value),
    /// Strictly less than (numeric).
    Lt(f64),
    /// Less than or equal (numeric).
    Le(f64),
    /// Strictly greater than (numeric).
    Gt(f64),
    /// Greater than or equal (numeric).
    Ge(f64),
    /// Numeric in `[lo, hi]` inclusive.
    Between { lo: f64, hi: f64 },
    /// Categorical membership (the value must equal one of these).
    IsIn(Vec<Value>),
}

// ─── Request / response types ───────────────────────────────────

/// SUDOKU request. Mirrors the HTTP body shape from spec §7
/// minus the wire-only fields (mode, explore_budget_ms,
/// acknowledge_pathology — those live on the HTTP layer in S3).
#[derive(Debug, Clone)]
pub struct SudokuRequest {
    /// Constraints — all `Field` predicates must be satisfied by
    /// returned solutions (hard=true) or penalize them (hard=false,
    /// currently treated as hard until S4).
    pub constraints: Vec<Constraint>,
    /// Maximum number of solutions to return.
    pub max_options: usize,
    /// Maximum number of near-misses to return (options that violate
    /// exactly one hard constraint).
    pub max_near_misses: usize,
}

impl Default for SudokuRequest {
    fn default() -> Self {
        SudokuRequest {
            constraints: Vec::new(),
            max_options: 5,
            max_near_misses: 3,
        }
    }
}

/// SUDOKU response — the honest-coverage contract.
#[derive(Debug, Clone)]
pub struct SudokuResponse {
    /// Solutions that satisfy ALL hard constraints, ranked by
    /// stated-prior mass (descending — most-common options first).
    pub solutions: Vec<Solution>,
    /// Options that violate exactly one hard constraint. Each
    /// carries which constraint and the cheapest relaxation that
    /// would unlock it.
    pub near_misses: Vec<NearMiss>,
    /// Verdict tristate.
    pub verdict: SudokuVerdict,
    /// Fraction of stated-prior mass explored. Always 1.0 for v1
    /// exhaustive walk; <1.0 once stratified sampling lands in S4.
    pub coverage: f64,
    /// Count of records considered (post-context filter).
    pub n_records_considered: usize,
}

/// A satisfying option.
#[derive(Debug, Clone)]
pub struct Solution {
    /// The record itself.
    pub record: Record,
    /// Stated-prior mass = `count_at_this_record_signature /
    /// total_records_considered`. In v1 records are weighted
    /// uniformly; once SUDOKU consumes brain/attend the mass will
    /// be the softmax weight.
    pub stated_prior_mass: f64,
}

/// An option that violates exactly one hard constraint, with the
/// minimal relaxation needed to unlock it.
#[derive(Debug, Clone)]
pub struct NearMiss {
    pub record: Record,
    pub stated_prior_mass: f64,
    /// Which constraint(s) this option violates.
    pub violations: Vec<ViolationDetail>,
    /// Indices into the request's `constraints` array that, if
    /// relaxed to the values shown in `violations[i].relax_to`,
    /// would make this option feasible.
    pub would_unlock_if_relaxed: Vec<usize>,
}

/// Detail of a single constraint violation on a near-miss option.
#[derive(Debug, Clone)]
pub struct ViolationDetail {
    /// Index into the request's `constraints` array.
    pub constraint_idx: usize,
    /// Human-readable field name (echoed from the constraint).
    pub field: String,
    /// Description of the violation (e.g. "08:45 > 08:30").
    pub violation: String,
    /// Smallest relaxation that would make this option satisfy
    /// the constraint (e.g. the option's actual value).
    pub relax_to: Value,
}

/// Honest tristate verdict. The whole point of SUDOKU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SudokuVerdict {
    /// Feasible region non-empty; ≥1 solution returned.
    Sat,
    /// Feasible region empty AND coverage ≥ 0.80 — we genuinely
    /// looked. Consumer can trust this as "no solution exists."
    Unsat,
    /// Coverage too low to make a claim either way. Consumer
    /// should retry with a larger budget OR accept the partial
    /// view. **Most solvers shrug here; SUDOKU refuses.**
    Unknown,
}

/// SUDOKU configuration (defaults per spec v0.3 §3 + §4).
#[derive(Debug, Clone, Copy)]
pub struct SudokuConfig {
    /// Coverage ≥ this AND no solutions → `Unsat`.
    pub coverage_high_threshold: f64,
    /// Coverage < this AND no solutions → `Unknown`.
    /// Middle bucket (low ≤ cov < high) AND no solutions: `Unknown`.
    pub coverage_low_threshold: f64,
}

impl Default for SudokuConfig {
    fn default() -> Self {
        // Per spec §3 OPEN-2: fixed thresholds for v1, named
        // regimes in v1.1 once telemetry exists.
        SudokuConfig {
            coverage_high_threshold: 0.80,
            coverage_low_threshold: 0.50,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SudokuError {
    #[error("constraint type not yet supported (S4): {0}")]
    NotYetSupported(String),
    #[error("invalid request: {0}")]
    InvalidRequest(String),
}

// ─── The solver ─────────────────────────────────────────────────

/// SUDOKU core entry point. Iterates `records`, classifies each as
/// satisfying / near-miss / violating, returns the response per
/// the honest-coverage contract.
///
/// For v1 the iterator is exhaustive — every record is considered.
/// Coverage is 1.0 (we looked at everything). Stratified sampling
/// for large bundles is S4.
pub fn solve_constraints<I>(
    records: I,
    req: &SudokuRequest,
    config: &SudokuConfig,
) -> Result<SudokuResponse, SudokuError>
where
    I: IntoIterator<Item = Record>,
{
    // Validate request: no stub constraint types in v1.
    for (i, c) in req.constraints.iter().enumerate() {
        match c {
            Constraint::Field { .. } => {}
            Constraint::Manifold { .. } => {
                return Err(SudokuError::NotYetSupported(format!(
                    "constraint[{}]: Manifold-distance — lands in S4",
                    i
                )));
            }
            Constraint::Relation { .. } => {
                return Err(SudokuError::NotYetSupported(format!(
                    "constraint[{}]: cross-field Relation — lands in S4",
                    i
                )));
            }
        }
    }

    // Pass 1: collect, classify per-record into satisfying /
    // near-miss / violating buckets. Record signature for prior
    // mass is the JSON of the record's full field map; identical
    // signatures aggregate (e.g. duplicated rows in a bundle get
    // their mass summed onto a single solution entry).
    let mut satisfying: Vec<(Record, Vec<ViolationDetail>)> = Vec::new();
    let mut near_miss_candidates: Vec<(Record, Vec<ViolationDetail>)> = Vec::new();
    let mut n_considered = 0_usize;

    for record in records {
        n_considered += 1;
        let violations = classify_record(&record, &req.constraints);
        match violations.len() {
            0 => satisfying.push((record, violations)),
            1 => near_miss_candidates.push((record, violations)),
            _ => {} // violates ≥2 — not a near-miss
        }
    }

    if n_considered == 0 {
        return Ok(SudokuResponse {
            solutions: Vec::new(),
            near_misses: Vec::new(),
            verdict: SudokuVerdict::Unknown,
            coverage: 0.0,
            n_records_considered: 0,
        });
    }

    // Stated-prior mass: count records per record-signature. v1
    // uses the record itself as the signature (collapse exact
    // duplicates). Total mass denominator = n_considered.
    let satisfying_mass = compute_mass_per_signature(&satisfying, n_considered);
    let near_miss_mass = compute_mass_per_signature(&near_miss_candidates, n_considered);

    // Sort solutions by mass descending, truncate to max_options.
    let mut solutions: Vec<Solution> = satisfying_mass
        .into_iter()
        .map(|(record, stated_prior_mass)| Solution {
            record,
            stated_prior_mass,
        })
        .collect();
    solutions.sort_by(|a, b| {
        b.stated_prior_mass
            .partial_cmp(&a.stated_prior_mass)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    solutions.truncate(req.max_options);

    // Sort near-misses by mass descending, attach metadata.
    let mut near_misses: Vec<NearMiss> = near_miss_mass
        .into_iter()
        .filter_map(|(record, mass)| {
            // Find the violation detail for this record from the
            // original near_miss_candidates list (first match —
            // signatures aren't shared across distinct violations).
            let violation = near_miss_candidates
                .iter()
                .find(|(r, _)| records_match(r, &record))
                .map(|(_, v)| v.clone())?;
            let unlock_indices: Vec<usize> =
                violation.iter().map(|v| v.constraint_idx).collect();
            Some(NearMiss {
                record,
                stated_prior_mass: mass,
                violations: violation,
                would_unlock_if_relaxed: unlock_indices,
            })
        })
        .collect();
    near_misses.sort_by(|a, b| {
        b.stated_prior_mass
            .partial_cmp(&a.stated_prior_mass)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    near_misses.truncate(req.max_near_misses);

    // Coverage: exhaustive walk → 1.0. (S4 will introduce stratified
    // sampling and a real estimator.)
    let coverage = 1.0_f64;
    let verdict = decide_verdict(coverage, &solutions, config);

    Ok(SudokuResponse {
        solutions,
        near_misses,
        verdict,
        coverage,
        n_records_considered: n_considered,
    })
}

// ─── Internals ──────────────────────────────────────────────────

/// Classify a record against the constraints. Returns the list of
/// violated constraint details (empty if all satisfied).
fn classify_record(record: &Record, constraints: &[Constraint]) -> Vec<ViolationDetail> {
    let mut violations = Vec::new();
    for (idx, c) in constraints.iter().enumerate() {
        let Constraint::Field { field, op, hard: _ } = c else {
            // Stubs already errored at request validation.
            continue;
        };
        let val = match record.get(field) {
            Some(v) => v,
            None => {
                // Missing field → counts as a violation (cannot
                // verify); spec §3 treats this as exclusion.
                violations.push(ViolationDetail {
                    constraint_idx: idx,
                    field: field.clone(),
                    violation: format!("field '{}' missing from record", field),
                    relax_to: Value::Null,
                });
                continue;
            }
        };
        if let Some(detail) = check_field_op(val, op, idx, field) {
            violations.push(detail);
        }
    }
    violations
}

fn check_field_op(
    val: &Value,
    op: &FieldOp,
    constraint_idx: usize,
    field: &str,
) -> Option<ViolationDetail> {
    let satisfied = match op {
        FieldOp::Eq(target) => val == target,
        FieldOp::Ne(target) => val != target,
        FieldOp::Lt(t) => value_as_f64(val).map(|v| v < *t).unwrap_or(false),
        FieldOp::Le(t) => value_as_f64(val).map(|v| v <= *t).unwrap_or(false),
        FieldOp::Gt(t) => value_as_f64(val).map(|v| v > *t).unwrap_or(false),
        FieldOp::Ge(t) => value_as_f64(val).map(|v| v >= *t).unwrap_or(false),
        FieldOp::Between { lo, hi } => {
            value_as_f64(val).map(|v| v >= *lo && v <= *hi).unwrap_or(false)
        }
        FieldOp::IsIn(targets) => targets.iter().any(|t| t == val),
    };
    if satisfied {
        None
    } else {
        Some(ViolationDetail {
            constraint_idx,
            field: field.to_string(),
            violation: format!("{:?} fails {:?}", val, op),
            relax_to: val.clone(),
        })
    }
}

fn value_as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Integer(i) => Some(*i as f64),
        _ => None,
    }
}

/// Compute stated-prior mass per record signature. v1: signature
/// is the record itself (exact-match collapse). Returns a Vec of
/// `(canonical_record, mass)` deduplicated.
fn compute_mass_per_signature(
    records_and_violations: &[(Record, Vec<ViolationDetail>)],
    n_total: usize,
) -> Vec<(Record, f64)> {
    let mut counts: Vec<(Record, usize)> = Vec::new();
    for (r, _) in records_and_violations {
        if let Some(slot) = counts.iter_mut().find(|(existing, _)| records_match(existing, r)) {
            slot.1 += 1;
        } else {
            counts.push((r.clone(), 1));
        }
    }
    counts
        .into_iter()
        .map(|(r, c)| (r, c as f64 / n_total as f64))
        .collect()
}

/// Record equality by full field map. Uses the underlying Record
/// type's PartialEq if available; otherwise builds it from field
/// comparison.
fn records_match(a: &Record, b: &Record) -> bool {
    // Record is std::collections::HashMap<String, Value>; HashMap
    // PartialEq is structural — checks same keys and same values.
    a == b
}

fn decide_verdict(coverage: f64, solutions: &[Solution], config: &SudokuConfig) -> SudokuVerdict {
    match (coverage >= config.coverage_high_threshold, solutions.is_empty()) {
        (true, false) => SudokuVerdict::Sat,
        (true, true) => SudokuVerdict::Unsat,
        (false, false) => {
            // Mid-bucket OR low-coverage WITH solutions: report Sat
            // with the found solutions but mark coverage modest.
            // Per spec §3 mid-bucket policy (≥low ∧ <high ∧ ≥1 sol → Sat).
            if coverage >= config.coverage_low_threshold {
                SudokuVerdict::Sat
            } else {
                // Low coverage even with solutions — caller asked
                // for honest reporting; treat as Unknown so they
                // don't act on a stale partial view.
                SudokuVerdict::Unknown
            }
        }
        (false, true) => SudokuVerdict::Unknown,
    }
}

// ─── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Value;

    fn rec(fields: &[(&str, Value)]) -> Record {
        let mut r = Record::new();
        for (k, v) in fields {
            r.insert((*k).to_string(), v.clone());
        }
        r
    }

    /// Trivial: empty constraint set + non-empty records →
    /// every record is a solution.
    #[test]
    fn no_constraints_returns_all_records() {
        let records = vec![
            rec(&[("x", Value::Integer(1))]),
            rec(&[("x", Value::Integer(2))]),
            rec(&[("x", Value::Integer(3))]),
        ];
        let req = SudokuRequest {
            constraints: vec![],
            max_options: 10,
            max_near_misses: 5,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        assert_eq!(resp.solutions.len(), 3);
        assert_eq!(resp.coverage, 1.0);
        assert!(resp.near_misses.is_empty());
    }

    /// Field equality constraint filters to exactly the matching
    /// records.
    #[test]
    fn eq_constraint_filters_correctly() {
        let records = vec![
            rec(&[("mode", Value::Text("walk".into()))]),
            rec(&[("mode", Value::Text("bike".into()))]),
            rec(&[("mode", Value::Text("walk".into()))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "mode".into(),
                op: FieldOp::Eq(Value::Text("walk".into())),
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 5,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        // Two "walk" records collapse to one solution (exact-match
        // dedupe) with mass 2/3.
        assert_eq!(resp.solutions.len(), 1);
        assert!((resp.solutions[0].stated_prior_mass - 2.0 / 3.0).abs() < 1e-9);
    }

    /// THE honest-coverage test: empty feasible region with full
    /// coverage → Unsat (not Unknown).
    #[test]
    fn full_coverage_no_solutions_returns_unsat() {
        let records = vec![
            rec(&[("x", Value::Integer(1))]),
            rec(&[("x", Value::Integer(2))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Eq(Value::Integer(99)),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unsat);
        assert!(resp.solutions.is_empty());
    }

    /// Empty bundle → Unknown verdict, coverage 0.0. **The
    /// load-bearing differentiator** between "no records" and
    /// "no satisfying records." Most solvers conflate.
    #[test]
    fn empty_bundle_returns_unknown_not_unsat() {
        let records: Vec<Record> = vec![];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "x".into(),
                op: FieldOp::Eq(Value::Integer(1)),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Unknown);
        assert_eq!(resp.coverage, 0.0);
        assert_eq!(resp.n_records_considered, 0);
    }

    /// Near-miss: records that violate exactly ONE constraint
    /// are surfaced with the relaxation needed to unlock them.
    /// Records violating ≥2 are NOT near-misses.
    #[test]
    fn near_miss_enumeration_single_constraint_violation() {
        let records = vec![
            // Satisfies both: solution
            rec(&[
                ("mode", Value::Text("walk".into())),
                ("eta_min", Value::Integer(25)),
            ]),
            // Violates eta only (eta=45 > 30): near-miss
            rec(&[
                ("mode", Value::Text("walk".into())),
                ("eta_min", Value::Integer(45)),
            ]),
            // Violates mode only (bike): near-miss
            rec(&[
                ("mode", Value::Text("bike".into())),
                ("eta_min", Value::Integer(25)),
            ]),
            // Violates both: NOT a near-miss
            rec(&[
                ("mode", Value::Text("drive".into())),
                ("eta_min", Value::Integer(60)),
            ]),
        ];
        let req = SudokuRequest {
            constraints: vec![
                Constraint::Field {
                    field: "mode".into(),
                    op: FieldOp::Eq(Value::Text("walk".into())),
                    hard: true,
                },
                Constraint::Field {
                    field: "eta_min".into(),
                    op: FieldOp::Le(30.0),
                    hard: true,
                },
            ],
            max_options: 5,
            max_near_misses: 5,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        assert_eq!(resp.solutions.len(), 1, "exactly one satisfying record");
        assert_eq!(resp.near_misses.len(), 2, "exactly two near-misses");
        // The 2-constraint-violating record must NOT appear in
        // near_misses (which by definition are single-violation).
        for nm in &resp.near_misses {
            assert_eq!(nm.violations.len(), 1, "near-misses violate exactly one");
        }
    }

    /// Stub constraint types return NotYetSupported error.
    #[test]
    fn manifold_constraint_errors_with_s4_note() {
        let req = SudokuRequest {
            constraints: vec![Constraint::Manifold {
                field: "embedding".into(),
                near_manifold: "marcella_voice_anchors".into(),
                epsilon: 0.3,
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 5,
        };
        let result = solve_constraints(
            vec![rec(&[("embedding", Value::Float(0.5))])],
            &req,
            &SudokuConfig::default(),
        );
        match result {
            Err(SudokuError::NotYetSupported(msg)) => {
                assert!(msg.contains("S4"));
                assert!(msg.contains("Manifold"));
            }
            other => panic!("expected NotYetSupported, got {:?}", other),
        }
    }

    /// Numeric range filter works as expected.
    #[test]
    fn between_filter_inclusive() {
        let records = vec![
            rec(&[("price", Value::Float(10.0))]),
            rec(&[("price", Value::Float(20.0))]),
            rec(&[("price", Value::Float(30.0))]),
            rec(&[("price", Value::Float(40.0))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "price".into(),
                op: FieldOp::Between { lo: 15.0, hi: 35.0 },
                hard: true,
            }],
            max_options: 10,
            max_near_misses: 5,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.verdict, SudokuVerdict::Sat);
        assert_eq!(resp.solutions.len(), 2);
    }

    /// IsIn categorical membership.
    #[test]
    fn is_in_categorical_membership() {
        let records = vec![
            rec(&[("kind", Value::Text("alpha".into()))]),
            rec(&[("kind", Value::Text("beta".into()))]),
            rec(&[("kind", Value::Text("gamma".into()))]),
        ];
        let req = SudokuRequest {
            constraints: vec![Constraint::Field {
                field: "kind".into(),
                op: FieldOp::IsIn(vec![
                    Value::Text("alpha".into()),
                    Value::Text("gamma".into()),
                ]),
                hard: true,
            }],
            max_options: 5,
            max_near_misses: 3,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.solutions.len(), 2);
    }

    /// Mass aggregation: 5 identical records → one solution
    /// with mass 1.0.
    #[test]
    fn identical_records_collapse_to_single_solution_with_full_mass() {
        let r = rec(&[("x", Value::Integer(1))]);
        let records = vec![r.clone(), r.clone(), r.clone(), r.clone(), r.clone()];
        let req = SudokuRequest {
            constraints: vec![],
            max_options: 5,
            max_near_misses: 3,
        };
        let resp = solve_constraints(records, &req, &SudokuConfig::default()).unwrap();
        assert_eq!(resp.solutions.len(), 1);
        assert!((resp.solutions[0].stated_prior_mass - 1.0).abs() < 1e-9);
    }
}
