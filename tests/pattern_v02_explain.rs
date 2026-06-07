//! Patterns v0.2 — Phase PE: PATTERN_EXPLAIN
//!
//! Per-WEIGHT-term contribution decomposition. The math claims here mirror
//! `theory/patterns/validation_tests.py` PE1–PE6 (which shipped 30/30 green
//! before this Rust port started).
//!
//! Invariant under test: `explain(expr, row).contribution() == eval_weight(expr, row)`
//! for every (expr, row) pair. The decomposition tree's root value MUST
//! equal the scalar score we'd get from straight evaluation.

#![cfg(feature = "patterns")]

use gigi::parser::{explain, eval_weight, parse_weight_expr_for_test, ExplainNode, WeightExpr};
use gigi::types::{Record, Value};

fn rec(pairs: &[(&str, f64)]) -> Record {
    let mut r = Record::new();
    for (k, v) in pairs {
        r.insert(k.to_string(), Value::Float(*v));
    }
    r
}

// ─── PE1: Lit leaf attribution ──────────────────────────────────────────────

#[test]
fn pe1_explain_lit_returns_literal_value() {
    let expr = WeightExpr::Lit(7.5);
    let row = Record::new();
    let node = explain(&expr, &row);
    match node {
        ExplainNode::Lit { value, contribution } => {
            assert_eq!(value, 7.5);
            assert_eq!(contribution, 7.5);
        }
        other => panic!("expected ExplainNode::Lit, got {other:?}"),
    }
}

// ─── PE2: Field leaf attribution ────────────────────────────────────────────

#[test]
fn pe2_explain_field_returns_row_value() {
    let expr = WeightExpr::Field("x".to_string());
    let row = rec(&[("x", 3.0)]);
    let node = explain(&expr, &row);
    match node {
        ExplainNode::Field { name, value, contribution } => {
            assert_eq!(name, "x");
            assert_eq!(value, 3.0);
            assert_eq!(contribution, 3.0);
        }
        other => panic!("expected ExplainNode::Field, got {other:?}"),
    }
}

// ─── PE3: Add root contribution = sum of children ──────────────────────────

#[test]
fn pe3_explain_add_root_equals_eval() {
    let expr = WeightExpr::Add(
        Box::new(WeightExpr::Field("x".to_string())),
        Box::new(WeightExpr::Field("y".to_string())),
    );
    let row = rec(&[("x", 2.0), ("y", 5.0)]);
    let node = explain(&expr, &row);
    assert_eq!(node.contribution(), 7.0);
    assert_eq!(node.contribution(), eval_weight(&expr, &row));
}

// ─── PE4: Mul root contribution = product of children ───────────────────────

#[test]
fn pe4_explain_mul_root_equals_product() {
    let expr = WeightExpr::Mul(
        Box::new(WeightExpr::Field("x".to_string())),
        Box::new(WeightExpr::Lit(4.0)),
    );
    let row = rec(&[("x", 3.0)]);
    let node = explain(&expr, &row);
    assert_eq!(node.contribution(), 12.0);
    match node {
        ExplainNode::Mul { left, right, contribution } => {
            assert_eq!(left.contribution(), 3.0);
            assert_eq!(right.contribution(), 4.0);
            assert_eq!(contribution, 12.0);
        }
        other => panic!("expected ExplainNode::Mul, got {other:?}"),
    }
}

// ─── PE5: Min chosen-branch + clipped flag ──────────────────────────────────

#[test]
fn pe5_explain_min_chosen_branch_and_clip_flag() {
    // min(sum=15, cap=10) → contribution=10, chosen=right, clipped=true
    let expr = WeightExpr::Min(
        Box::new(WeightExpr::Field("sum".to_string())),
        Box::new(WeightExpr::Lit(10.0)),
    );
    let row_over = rec(&[("sum", 15.0)]);
    let node = explain(&expr, &row_over);
    match &node {
        ExplainNode::Min { chosen, clipped, contribution, .. } => {
            assert_eq!(chosen, "right");
            assert!(*clipped, "raw sum 15 > cap 10 → clipped must be true");
            assert_eq!(*contribution, 10.0);
        }
        other => panic!("expected ExplainNode::Min, got {other:?}"),
    }

    // min(sum=5, cap=10) → contribution=5, chosen=left, clipped=false
    let row_under = rec(&[("sum", 5.0)]);
    let node = explain(&expr, &row_under);
    match &node {
        ExplainNode::Min { chosen, clipped, contribution, .. } => {
            assert_eq!(chosen, "left");
            assert!(!*clipped, "raw sum 5 < cap 10 → cap didn't fire");
            assert_eq!(*contribution, 5.0);
        }
        other => panic!("expected ExplainNode::Min, got {other:?}"),
    }
}

// ─── PE6: Full SCJ scorer — the invariant on real-shaped data ──────────────

#[test]
fn pe6_explain_full_scj_scorer_invariant() {
    // Build the actual SCJ 10-weight scorer: min(sum_of_weighted_terms, 10).
    let scj_sql = "DEFINE PATTERN scj_v01 AS cast_truncate_alloc >= 0 \
        WEIGHT (min(\
            cast_truncate_alloc * 3 \
          + multiply_before_alloc * 3 \
          + shift_before_alloc * 3 \
          + param_times_const * 2 \
          + unchecked_param_to_size * 2 \
          + mdl_shift_size * 2 \
          + reaches_ExAllocatePool2 * 1 \
          + reaches_MmBuildMdlForNonPagedPool * 1 \
          + has_probe_read * 1 \
          + has_probe_write * 1, \
          10))";
    let expr = parse_weight_expr_for_test(scj_sql).expect("parse SCJ WEIGHT");

    // All ten bits set → raw sum = 3+3+3+2+2+2+1+1+1+1 = 19, clipped to 10.
    let row = rec(&[
        ("cast_truncate_alloc", 1.0),
        ("multiply_before_alloc", 1.0),
        ("shift_before_alloc", 1.0),
        ("param_times_const", 1.0),
        ("unchecked_param_to_size", 1.0),
        ("mdl_shift_size", 1.0),
        ("reaches_ExAllocatePool2", 1.0),
        ("reaches_MmBuildMdlForNonPagedPool", 1.0),
        ("has_probe_read", 1.0),
        ("has_probe_write", 1.0),
    ]);

    let score = eval_weight(&expr, &row);
    let node = explain(&expr, &row);

    assert_eq!(score, 10.0, "raw sum 19 must clip to 10");
    assert_eq!(node.contribution(), score, "explain root = eval — load-bearing invariant");

    match &node {
        ExplainNode::Min { chosen, clipped, .. } => {
            assert_eq!(chosen, "right");
            assert!(*clipped);
        }
        other => panic!("root must be Min, got {other:?}"),
    }
}

// ─── Domain-swap (§8 discipline) — identical math, different field names ───
//
// Every test above is "vuln-hunt-flavored." These four prove the substrate
// produces bit-identical contribution numbers for isomorphic data,
// regardless of which domain's field names we use.

fn run_domain_swap_scorer(field_names: &[&str]) -> f64 {
    assert_eq!(field_names.len(), 3, "scorer expects 3 fields");
    // Build: min(f0 * 3 + f1 * 2 + f2 * 1, 5)
    let f0 = WeightExpr::Field(field_names[0].to_string());
    let f1 = WeightExpr::Field(field_names[1].to_string());
    let f2 = WeightExpr::Field(field_names[2].to_string());
    let sum = WeightExpr::Add(
        Box::new(WeightExpr::Add(
            Box::new(WeightExpr::Mul(Box::new(f0), Box::new(WeightExpr::Lit(3.0)))),
            Box::new(WeightExpr::Mul(Box::new(f1), Box::new(WeightExpr::Lit(2.0)))),
        )),
        Box::new(WeightExpr::Mul(Box::new(f2), Box::new(WeightExpr::Lit(1.0)))),
    );
    let expr = WeightExpr::Min(Box::new(sum), Box::new(WeightExpr::Lit(5.0)));
    let row = rec(&[
        (field_names[0], 1.0),
        (field_names[1], 1.0),
        (field_names[2], 1.0),
    ]);
    let node = explain(&expr, &row);
    node.contribution()
}

#[test]
fn ds_explain_vuln_hunt_domain() {
    let score = run_domain_swap_scorer(&["cast_truncate_alloc", "multiply_before_alloc", "has_probe_read"]);
    // 1*3 + 1*2 + 1*1 = 6, clipped to 5
    assert_eq!(score, 5.0);
}

#[test]
fn ds_explain_fraud_domain() {
    let score = run_domain_swap_scorer(&["amount_over_threshold", "same_origin_destination", "high_velocity"]);
    assert_eq!(score, 5.0);
}

#[test]
fn ds_explain_education_domain() {
    let score = run_domain_swap_scorer(&["assignments_complete", "attendance_high", "passing_grade"]);
    assert_eq!(score, 5.0);
}

#[test]
fn ds_explain_discourse_flow_domain() {
    let score = run_domain_swap_scorer(&["dialog_act_question", "boundary_turn_change", "topic_shift"]);
    assert_eq!(score, 5.0);
}

// ─── Integer + Bool field coercion ──────────────────────────────────────────

#[test]
fn pe_field_coerces_integer_and_bool_per_spec() {
    let expr = WeightExpr::Add(
        Box::new(WeightExpr::Field("int_field".to_string())),
        Box::new(WeightExpr::Field("bool_field".to_string())),
    );
    let mut row = Record::new();
    row.insert("int_field".to_string(), Value::Integer(7));
    row.insert("bool_field".to_string(), Value::Bool(true));
    let node = explain(&expr, &row);
    assert_eq!(node.contribution(), 8.0, "int 7 + bool true → 7 + 1 = 8");
}

#[test]
fn pe_field_missing_coerces_to_zero() {
    let expr = WeightExpr::Field("absent".to_string());
    let row = Record::new();
    let node = explain(&expr, &row);
    assert_eq!(node.contribution(), 0.0);
}
