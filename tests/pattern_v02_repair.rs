//! Patterns v0.2 — Phase PR: PATTERN_REPAIR menu
//!
//! For each near-miss row, return the ordered minimum-cost sequence of
//! field flips that would make it satisfy the predicate.
//!
//! Math target: `theory/patterns/validation_tests.py` PR1-PR6 (30/30 green).

#![cfg(feature = "patterns")]

use gigi::parser::{
    repair_menu, FilterCondition, Literal, RepairMenu, RepairOption,
};
use gigi::types::{Record, Value};
use std::collections::HashMap;

fn rec_int(pairs: &[(&str, i64)]) -> Record {
    let mut r = Record::new();
    for (k, v) in pairs {
        r.insert(k.to_string(), Value::Integer(*v));
    }
    r
}

// ─── PR1: single-flip uniform cost ──────────────────────────────────────────

#[test]
fn pr1_repair_single_flip_uniform_cost() {
    let row = rec_int(&[("a", 1), ("b", 1)]);
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(1)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
    ];
    let menu = repair_menu(&pred, &row, 1, &HashMap::new(), 5);
    match menu {
        RepairMenu::Options(opts) => {
            assert_eq!(opts.len(), 1, "exactly one flip option");
            assert_eq!(opts[0].cost, 1.0);
            assert_eq!(opts[0].flips.len(), 1);
            assert_eq!(opts[0].flips[0].field, "b");
        }
        other => panic!("expected RepairMenu::Options, got {other:?}"),
    }
}

// ─── PR2: double flip ───────────────────────────────────────────────────────

#[test]
fn pr2_repair_double_flip() {
    let row = rec_int(&[("a", 1), ("b", 1), ("c", 1)]);
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
    ];
    let menu = repair_menu(&pred, &row, 2, &HashMap::new(), 5);
    match menu {
        RepairMenu::Options(opts) => {
            assert_eq!(opts.len(), 1);
            assert_eq!(opts[0].cost, 2.0);
            let fields: std::collections::HashSet<_> =
                opts[0].flips.iter().map(|f| f.field.clone()).collect();
            assert_eq!(fields, ["a", "b"].iter().map(|s| s.to_string()).collect());
        }
        other => panic!("expected RepairMenu::Options, got {other:?}"),
    }
}

// ─── PR3: custom relaxation costs reorder the menu ──────────────────────────

#[test]
fn pr3_repair_custom_costs_sort_correctly() {
    let row = rec_int(&[("cheap_field", 1), ("expensive_field", 1)]);
    let pred_cheap = vec![FilterCondition::Eq(
        "cheap_field".to_string(),
        Literal::Integer(0),
    )];
    let pred_expensive = vec![FilterCondition::Eq(
        "expensive_field".to_string(),
        Literal::Integer(0),
    )];

    let mut costs = HashMap::new();
    costs.insert("cheap_field".to_string(), 0.5);
    costs.insert("expensive_field".to_string(), 3.0);

    let menu_cheap = repair_menu(&pred_cheap, &row, 1, &costs, 5);
    let menu_exp = repair_menu(&pred_expensive, &row, 1, &costs, 5);

    if let (RepairMenu::Options(ops_c), RepairMenu::Options(ops_e)) =
        (menu_cheap, menu_exp)
    {
        assert_eq!(ops_c[0].cost, 0.5);
        assert_eq!(ops_e[0].cost, 3.0);
    } else {
        panic!("both menus should be Options");
    }
}

// ─── PR4: already-matches sentinel ──────────────────────────────────────────

#[test]
fn pr4_repair_already_matches_returns_sentinel() {
    let row = rec_int(&[("a", 1)]);
    let pred = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(1))];
    let menu = repair_menu(&pred, &row, 1, &HashMap::new(), 5);
    assert!(matches!(menu, RepairMenu::AlreadyMatches));
}

// ─── PR5: too-far sentinel ──────────────────────────────────────────────────

#[test]
fn pr5_repair_too_far_returns_sentinel() {
    let row = rec_int(&[("a", 1), ("b", 1), ("c", 1), ("d", 1)]);
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("c".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("d".to_string(), Literal::Integer(0)),
    ];
    let menu = repair_menu(&pred, &row, 1, &HashMap::new(), 5);
    assert!(matches!(menu, RepairMenu::TooFar { .. }));
}

// ─── PR6: min-cost is actually min — applying the flip satisfies the pred ──

#[test]
fn pr6_repair_min_cost_is_actually_minimum() {
    let row = rec_int(&[("a", 1), ("b", 1), ("c", 1)]);
    let pred = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(0))];
    let menu = repair_menu(&pred, &row, 1, &HashMap::new(), 5);
    match menu {
        RepairMenu::Options(opts) => {
            let cost = opts[0].cost;
            assert_eq!(cost, 1.0, "single violation, default cost");
            // Apply the flip and confirm the row would now match.
            let mut new_row = row.clone();
            for f in &opts[0].flips {
                new_row.insert(f.field.clone(), f.target.clone());
            }
            let v = new_row.get("a").cloned();
            assert_eq!(v, Some(Value::Integer(0)));
        }
        other => panic!("expected RepairMenu::Options, got {other:?}"),
    }
}

// ─── Top-K cap ──────────────────────────────────────────────────────────────

#[test]
fn pr_top_k_cap_truncates_menu() {
    // 3 violations, max_flips=3 — only one valid full-flip sequence.
    // With max_flips=2, no valid sequence (insufficient).
    let row = rec_int(&[("a", 1), ("b", 1), ("c", 1)]);
    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(0)),
        FilterCondition::Eq("c".to_string(), Literal::Integer(0)),
    ];
    let menu = repair_menu(&pred, &row, 3, &HashMap::new(), 5);
    match menu {
        RepairMenu::Options(opts) => {
            assert!(opts.len() <= 5, "top-5 cap respected");
            assert_eq!(opts[0].flips.len(), 3, "full flip count");
        }
        other => panic!("expected Options, got {other:?}"),
    }
}

// ─── Domain swap (§8): identical math across four field families ───────────

fn run_ds_repair(field_a: &str, field_b: &str) -> RepairMenu {
    let row = rec_int(&[(field_a, 1), (field_b, 1)]);
    let pred = vec![
        FilterCondition::Eq(field_a.to_string(), Literal::Integer(0)),
        FilterCondition::Eq(field_b.to_string(), Literal::Integer(0)),
    ];
    repair_menu(&pred, &row, 2, &HashMap::new(), 5)
}

fn menu_shape(m: &RepairMenu) -> Vec<(usize, f64)> {
    match m {
        RepairMenu::Options(opts) => opts.iter().map(|o| (o.flips.len(), o.cost)).collect(),
        _ => vec![],
    }
}

#[test]
fn ds_repair_vuln_hunt() {
    let m = run_ds_repair("cast_truncate_alloc", "has_probe_read");
    assert_eq!(menu_shape(&m), vec![(2, 2.0)]);
}

#[test]
fn ds_repair_fraud() {
    let m = run_ds_repair("amount_over_threshold", "same_origin_destination");
    assert_eq!(menu_shape(&m), vec![(2, 2.0)]);
}

#[test]
fn ds_repair_education() {
    let m = run_ds_repair("assignments_complete", "attendance_high");
    assert_eq!(menu_shape(&m), vec![(2, 2.0)]);
}

#[test]
fn ds_repair_discourse_flow() {
    let m = run_ds_repair("dialog_act_question", "boundary_turn_change");
    assert_eq!(menu_shape(&m), vec![(2, 2.0)]);
}
