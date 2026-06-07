//! Patterns v0.2 — Phase PP: pattern_preflight
//!
//! Three-layer preflight per `theory/patterns/SPEC_v0.2_VERDICT.md` §3.1.5:
//!
//!   1. Internal contradiction — always a verdict gate. No bundle needed.
//!   2. Bundle-statistic — verdict gate ONLY when near_miss_budget == 0.
//!   3. Holonomy — informational. Fires only in the unsat branch.
//!
//! Math target: `theory/patterns/validation_tests.py` PP1–PP5 (shipped
//! 30/30 last commit). Rust port matches.

#![cfg(feature = "patterns")]

use gigi::parser::{
    preflight_holonomy, preflight_internal, preflight_statistic,
    FilterCondition, Literal, PreflightVerdict,
};
use gigi::types::{Record, Value};

fn r(pairs: &[(&str, Value)]) -> Record {
    let mut r = Record::new();
    for (k, v) in pairs {
        r.insert(k.to_string(), v.clone());
    }
    r
}

fn rec_int(pairs: &[(&str, i64)]) -> Record {
    r(&pairs.iter().map(|(k, v)| (*k, Value::Integer(*v))).collect::<Vec<_>>())
}

fn rec_text(pairs: &[(&str, &str)]) -> Record {
    r(&pairs.iter()
        .map(|(k, v)| (*k, Value::Text(v.to_string())))
        .collect::<Vec<_>>())
}

// ─── PP1: impossible numeric range — catches "x >= 100" against max=9 ───────

#[test]
fn pp1_preflight_statistic_catches_impossible_numeric_range() {
    // Bundle: x ∈ [0, 9]
    let records: Vec<Record> = (0..10).map(|i| rec_int(&[("x", i)])).collect();
    let pred = vec![FilterCondition::Gte("x".to_string(), Literal::Integer(100))];

    let v = preflight_statistic(&pred, &records);
    match v {
        PreflightVerdict::UnsatStatistic(reason) => {
            assert!(
                reason.contains("100") || reason.contains("max") || reason.contains("x"),
                "reason should name the field/value: {reason}"
            );
        }
        other => panic!("expected UnsatStatistic, got {other:?}"),
    }
}

// ─── PP2: missing categorical — catches color=purple against {red, blue} ────

#[test]
fn pp2_preflight_statistic_catches_missing_categorical() {
    let records = vec![
        rec_text(&[("color", "red")]),
        rec_text(&[("color", "blue")]),
        rec_text(&[("color", "red")]),
    ];
    let pred = vec![FilterCondition::Eq(
        "color".to_string(),
        Literal::Text("purple".to_string()),
    )];

    let v = preflight_statistic(&pred, &records);
    match v {
        PreflightVerdict::UnsatStatistic(reason) => {
            assert!(reason.contains("purple") || reason.contains("color"),
                    "reason should name purple/color: {reason}");
        }
        other => panic!("expected UnsatStatistic, got {other:?}"),
    }
}

// ─── PP3: internal contradiction — caught regardless of bundle ──────────────

#[test]
fn pp3_preflight_internal_catches_self_contradiction() {
    let pred = vec![
        FilterCondition::Gte("x".to_string(), Literal::Integer(5)),
        FilterCondition::Lt("x".to_string(), Literal::Integer(3)),
    ];

    let v = preflight_internal(&pred);
    match v {
        PreflightVerdict::UnsatInternal(reason) => {
            assert!(reason.to_lowercase().contains("contradiction")
                    || reason.contains("x"),
                    "reason should name the field/contradiction: {reason}");
        }
        other => panic!("expected UnsatInternal, got {other:?}"),
    }
}

#[test]
fn pp3b_preflight_internal_catches_equality_contradiction() {
    let pred = vec![
        FilterCondition::Eq("color".to_string(), Literal::Text("red".to_string())),
        FilterCondition::Eq("color".to_string(), Literal::Text("blue".to_string())),
    ];
    let v = preflight_internal(&pred);
    assert!(!v.is_ok(), "two conflicting equalities on same field must be unsat");
}

// ─── PP4: holonomy — joint distribution forbids the conjunction ─────────────

#[test]
fn pp4_preflight_holonomy_catches_joint_contradiction() {
    // Bundle where x=1 NEVER co-occurs with y=1, but both x=1 and y=1
    // exist separately. Statistic preflight passes; holonomy catches.
    let records = vec![
        rec_int(&[("x", 1), ("y", 0)]),
        rec_int(&[("x", 1), ("y", 0)]),
        rec_int(&[("x", 0), ("y", 1)]),
        rec_int(&[("x", 0), ("y", 1)]),
        rec_int(&[("x", 0), ("y", 0)]),
    ];
    let pred = vec![
        FilterCondition::Eq("x".to_string(), Literal::Integer(1)),
        FilterCondition::Eq("y".to_string(), Literal::Integer(1)),
    ];

    // Layer 1: individual clauses both have support
    let v_stat = preflight_statistic(&pred, &records);
    assert!(v_stat.is_ok(),
            "x=1 and y=1 each individually have rows; statistic preflight should pass");

    // Layer 2: joint distribution forbids — holonomy catches
    let v_holo = preflight_holonomy(&pred, &records);
    match v_holo {
        PreflightVerdict::UnsatJoint(_) => {}
        other => panic!("expected UnsatJoint, got {other:?}"),
    }
}

// ─── PP5: satisfiable predicate passes preflight ───────────────────────────

#[test]
fn pp5_preflight_passes_satisfiable_predicate() {
    let records: Vec<Record> = (0..10)
        .map(|i| {
            let mut r = Record::new();
            r.insert("x".to_string(), Value::Integer(i));
            r.insert(
                "color".to_string(),
                Value::Text(if i < 3 { "red".to_string() } else { "blue".to_string() }),
            );
            r
        })
        .collect();
    let pred = vec![
        FilterCondition::Gte("x".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("color".to_string(), Literal::Text("red".to_string())),
    ];

    assert!(preflight_internal(&pred).is_ok());
    assert!(preflight_statistic(&pred, &records).is_ok());
    assert!(preflight_holonomy(&pred, &records).is_ok());
}

// ─── Empty bundle edge case ─────────────────────────────────────────────────

#[test]
fn pp_empty_bundle_with_nonempty_predicate_is_unsat_statistic() {
    let records: Vec<Record> = vec![];
    let pred = vec![FilterCondition::Eq("x".to_string(), Literal::Integer(1))];
    // Statistic preflight: no field stats → no clause can be satisfied
    let v = preflight_statistic(&pred, &records);
    assert!(!v.is_ok(), "empty bundle vs. concrete clause must be unsat");
}

// ─── Single-clause holonomy is a no-op (1-loop, no joint structure) ─────────

#[test]
fn pp_single_clause_holonomy_is_ok() {
    let records = vec![rec_int(&[("x", 1)]), rec_int(&[("x", 0)])];
    let pred = vec![FilterCondition::Eq("x".to_string(), Literal::Integer(1))];
    assert!(preflight_holonomy(&pred, &records).is_ok());
}

// ─── Domain-swap (§8): identical math across four domain field families ─────

fn run_domain_swap_preflight(field_a: &str, field_b: &str) -> (bool, bool) {
    // Bundle: x=1 NEVER co-occurs with y=1 (joint contradiction).
    let records = vec![
        r(&[(field_a, Value::Integer(1)), (field_b, Value::Integer(0))]),
        r(&[(field_a, Value::Integer(1)), (field_b, Value::Integer(0))]),
        r(&[(field_a, Value::Integer(0)), (field_b, Value::Integer(1))]),
        r(&[(field_a, Value::Integer(0)), (field_b, Value::Integer(0))]),
    ];
    let pred = vec![
        FilterCondition::Eq(field_a.to_string(), Literal::Integer(1)),
        FilterCondition::Eq(field_b.to_string(), Literal::Integer(1)),
    ];
    let stat_ok = preflight_statistic(&pred, &records).is_ok();
    let holo_ok = preflight_holonomy(&pred, &records).is_ok();
    (stat_ok, holo_ok)
}

#[test]
fn ds_preflight_vuln_hunt() {
    let (stat, holo) = run_domain_swap_preflight("cast_truncate_alloc", "has_probe_read");
    assert!(stat && !holo, "statistic should pass, holonomy should catch joint");
}

#[test]
fn ds_preflight_fraud() {
    let (stat, holo) = run_domain_swap_preflight("amount_over_threshold", "high_velocity");
    assert!(stat && !holo);
}

#[test]
fn ds_preflight_education() {
    let (stat, holo) = run_domain_swap_preflight("assignments_complete", "attendance_high");
    assert!(stat && !holo);
}

#[test]
fn ds_preflight_discourse_flow() {
    let (stat, holo) = run_domain_swap_preflight("dialog_act_question", "boundary_turn_change");
    assert!(stat && !holo);
}
