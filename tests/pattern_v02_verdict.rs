//! Patterns v0.2 — Phase VT: verdict trichotomy
//!
//! Every (pattern, bundle, near_miss_budget) tuple lands in exactly one of
//! { sat, unsat, near_miss }. Order: internal preflight → (statistic
//! preflight if budget=0) → sat scan → near-miss scan → unsat.
//!
//! Math target: `theory/patterns/validation_tests.py` VT1-VT5 (shipped 30/30).

#![cfg(feature = "patterns")]

use gigi::parser::{
    compute_verdict, FilterCondition, Literal, Verdict,
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

// ─── VT1: sat when ≥1 row matches ───────────────────────────────────────────

#[test]
fn vt1_verdict_sat_when_rows_match() {
    let records: Vec<Record> = (0..10)
        .map(|i| rec_int(&[("a", if i < 3 { 1 } else { 0 })]))
        .collect();
    let pred = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(1))];

    let v = compute_verdict(&pred, &records, 1);
    match v {
        Verdict::Sat { n_matches } => assert_eq!(n_matches, 3),
        other => panic!("expected Sat, got {other:?}"),
    }
}

// ─── VT2: unsat-by-preflight when budget=0 ─────────────────────────────────

#[test]
fn vt2_verdict_unsat_by_preflight_at_budget_zero() {
    let records: Vec<Record> = (0..10).map(|i| rec_int(&[("x", i)])).collect();
    let pred = vec![FilterCondition::Gte("x".to_string(), Literal::Integer(999))];

    let v = compute_verdict(&pred, &records, 0);
    match v {
        Verdict::Unsat { preflight_caught, .. } => {
            assert!(preflight_caught, "preflight should catch x >= 999 against max=9");
        }
        other => panic!("expected Unsat (preflight_caught), got {other:?}"),
    }
}

// ─── VT2b: same predicate, budget=1 → near_miss (preflight doesn't gate) ────

#[test]
fn vt2b_verdict_with_budget_skips_statistic_preflight() {
    // Bundle has b=1, predicate wants b=0. Statistic preflight would say
    // unsat (no row has b=0). But budget=1 means single-flip near-miss may
    // repair it — so we should NOT hit unsat here.
    let records: Vec<Record> = (0..5).map(|_| rec_int(&[("a", 1), ("b", 1)])).collect();
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(1)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
    ];

    // budget=0 should hit preflight unsat
    let v0 = compute_verdict(&pred, &records, 0);
    assert!(matches!(v0, Verdict::Unsat { preflight_caught: true, .. }));

    // budget=1 should land in near_miss
    let v1 = compute_verdict(&pred, &records, 1);
    match v1 {
        Verdict::NearMiss { near_miss_count, .. } => assert_eq!(near_miss_count, 5),
        other => panic!("expected NearMiss, got {other:?}"),
    }
}

// ─── VT3: pure unsat by scan — no match, no near-miss within budget ─────────

#[test]
fn vt3_verdict_unsat_by_scan_when_no_match_and_no_near_miss() {
    // All rows have a=1, b=1, c=1. Predicate wants a=0, b=0, c=0 (3 violations).
    // With budget=1, no row is within reach.
    let records: Vec<Record> = (0..5)
        .map(|_| rec_int(&[("a", 1), ("b", 1), ("c", 1)]))
        .collect();
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("c".to_string(), Literal::Integer(0)),
    ];

    let v = compute_verdict(&pred, &records, 1);
    assert!(matches!(v, Verdict::Unsat { .. }), "3 violations > budget 1 ⇒ unsat");
}

// ─── VT4: near-miss at distance 1 — single-flip away ────────────────────────

#[test]
fn vt4_verdict_near_miss_at_distance_1() {
    // All rows have (a=1, b=1); predicate wants (a=1, b=0). 5 rows
    // each one flip away.
    let records: Vec<Record> = (0..5)
        .map(|_| rec_int(&[("a", 1), ("b", 1)]))
        .collect();
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(1)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
    ];

    let v = compute_verdict(&pred, &records, 1);
    match v {
        Verdict::NearMiss { near_miss_count, budget } => {
            assert_eq!(near_miss_count, 5);
            assert_eq!(budget, 1);
        }
        other => panic!("expected NearMiss, got {other:?}"),
    }
}

// ─── VT5: trichotomy exhaustive — three patterns, three verdicts ───────────

#[test]
fn vt5_verdict_trichotomy_exhaustive() {
    let records = vec![
        rec_int(&[("a", 1), ("b", 1)]),
        rec_int(&[("a", 1), ("b", 0)]),
        rec_int(&[("a", 0), ("b", 0)]),
    ];

    let sat_pred = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(1))];
    let near_pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(1)),
    ]; // 1 flip from row 0
    let unsat_pred = vec![FilterCondition::Gte("a".to_string(), Literal::Integer(100))];

    let v_sat = compute_verdict(&sat_pred, &records, 1);
    let v_near = compute_verdict(&near_pred, &records, 1);
    // budget=0 so the bundle-statistic preflight catches the unreachable
    let v_unsat = compute_verdict(&unsat_pred, &records, 0);

    assert!(matches!(v_sat, Verdict::Sat { .. }));
    assert!(matches!(v_near, Verdict::NearMiss { .. }));
    assert!(matches!(v_unsat, Verdict::Unsat { .. }));
}

// ─── Domain swap (§8): identical math across four field families ───────────

fn run_domain_swap_verdict_sat(field: &str) -> Verdict {
    let records: Vec<Record> = (0..10)
        .map(|i| r(&[(field, Value::Integer(if i < 3 { 1 } else { 0 }))]))
        .collect();
    let pred = vec![FilterCondition::Eq(field.to_string(), Literal::Integer(1))];
    compute_verdict(&pred, &records, 1)
}

#[test]
fn ds_verdict_vuln_hunt_sat() {
    let v = run_domain_swap_verdict_sat("cast_truncate_alloc");
    assert!(matches!(v, Verdict::Sat { n_matches: 3 }));
}

#[test]
fn ds_verdict_fraud_sat() {
    let v = run_domain_swap_verdict_sat("amount_over_threshold");
    assert!(matches!(v, Verdict::Sat { n_matches: 3 }));
}

#[test]
fn ds_verdict_education_sat() {
    let v = run_domain_swap_verdict_sat("assignments_complete");
    assert!(matches!(v, Verdict::Sat { n_matches: 3 }));
}

#[test]
fn ds_verdict_discourse_flow_sat() {
    let v = run_domain_swap_verdict_sat("dialog_act_question");
    assert!(matches!(v, Verdict::Sat { n_matches: 3 }));
}

// ─── Internal contradiction wins regardless of budget ──────────────────────

#[test]
fn vt_internal_contradiction_unsat_at_any_budget() {
    let records: Vec<Record> = (0..5).map(|i| rec_int(&[("x", i)])).collect();
    let pred = vec![
        FilterCondition::Gte("x".to_string(), Literal::Integer(5)),
        FilterCondition::Lt("x".to_string(), Literal::Integer(3)),
    ];
    for budget in [0, 1, 2, 5] {
        let v = compute_verdict(&pred, &records, budget);
        assert!(
            matches!(v, Verdict::Unsat { preflight_caught: true, .. }),
            "internal contradiction unsat at budget {budget}, got {v:?}"
        );
    }
}
