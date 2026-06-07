//! Patterns v0.2 — Wire layer: HUNT verdict envelope end-to-end.
//!
//! Tests `hunt_v2_orchestrate`, the helper that composes Phase PE / PP /
//! VT / PR / K_P into the wire-shaped HUNT response. Same Engine setup
//! the v0.1 tests use — DEFINE PATTERN → ingest → orchestrate → assert
//! envelope shape.

#![cfg(feature = "patterns")]

use gigi::engine::Engine;
use gigi::parser::{
    execute, hunt_v2_orchestrate, parse, ExecResult, HuntV2Args, HuntV2Envelope,
};
use gigi::types::Value;
use std::collections::HashMap;
use tempfile::tempdir;

fn fresh_engine() -> Engine {
    let dir = tempdir().expect("tempdir");
    let path = dir.into_path();
    Engine::open(&path).expect("engine open")
}

fn run(engine: &mut Engine, sql: &str) -> Result<ExecResult, String> {
    let stmt = parse(sql).map_err(|e| format!("parse `{sql}`: {e}"))?;
    execute(engine, &stmt)
}

fn args_for(pattern: &str, bundle: &str) -> HuntV2Args {
    HuntV2Args {
        pattern: pattern.to_string(),
        bundle: bundle.to_string(),
        excluding: Vec::new(),
        top: None,
        project: None,
        near_miss_budget: 1,
        explain: false,
        include_repair_menu: false,
        relaxation_costs: HashMap::new(),
    }
}

// ─── W1: Sat verdict envelope — at least one row matches strictly ──────────

#[test]
fn w1_envelope_sat_verdict_with_n_matches() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER)").expect("create");
    for (id, a) in [(1, 1), (2, 0), (3, 1)] {
        run(&mut e, &format!("INSERT INTO b (id, a) VALUES ({id}, {a})")).expect("ins");
    }
    run(&mut e, "DEFINE PATTERN p AS a = 1 WEIGHT (a) USING (a)").expect("define");

    let env = hunt_v2_orchestrate(&mut e, &args_for("p", "b")).expect("hunt v2");
    assert_eq!(env.verdict, "sat");
    assert_eq!(env.n_matches, Some(2));
    assert_eq!(env.rows.len(), 2);
}

// ─── W2: Near-miss envelope — zero strict matches, ≥1 within budget ────────

#[test]
fn w2_envelope_near_miss_verdict_with_count() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER, x INT FIBER)").expect("create");
    // All rows have (a=1, x=1). Pattern wants (a=1 AND x=0): all rows are
    // 1 flip away from matching.
    for id in 1..=4 {
        run(&mut e, &format!("INSERT INTO b (id, a, x) VALUES ({id}, 1, 1)")).expect("ins");
    }
    run(&mut e, "DEFINE PATTERN p AS a = 1 AND x = 0 WEIGHT (a) USING (a, x)").expect("define");

    let env = hunt_v2_orchestrate(&mut e, &args_for("p", "b")).expect("hunt v2");
    assert_eq!(env.verdict, "near_miss");
    assert_eq!(env.near_miss_count, Some(4));
    assert!(env.rows.is_empty(), "near-miss returns no sat rows");
}

// ─── W3: Unsat envelope at budget=0 — preflight catches ────────────────────

#[test]
fn w3_envelope_unsat_verdict_at_budget_zero() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, x INT FIBER)").expect("create");
    for id in 1..=5 {
        run(&mut e, &format!("INSERT INTO b (id, x) VALUES ({id}, {id})")).expect("ins");
    }
    run(&mut e, "DEFINE PATTERN p AS x >= 999 WEIGHT (x) USING (x)").expect("define");

    let mut a = args_for("p", "b");
    a.near_miss_budget = 0; // v0.1-compatible mode
    let env = hunt_v2_orchestrate(&mut e, &a).expect("hunt v2");
    assert_eq!(env.verdict, "unsat");
    assert_eq!(env.preflight_caught, Some(true));
    assert!(env.reason.is_some());
}

// ─── W4: EXCLUDING IN composes with verdict ────────────────────────────────

#[test]
fn w4_envelope_sat_with_excluding_filter_applied() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER)").expect("create");
    for (id, a) in [(1, 1), (2, 1), (3, 1), (4, 1), (5, 1)] {
        run(&mut e, &format!("INSERT INTO b (id, a) VALUES ({id}, {a})")).expect("ins");
    }
    run(&mut e, "CREATE BUNDLE conf (id INT BASE, x INT FIBER)").expect("create conf");
    for id in [2, 4] {
        run(&mut e, &format!("INSERT INTO conf (id, x) VALUES ({id}, 0)")).expect("ins conf");
    }
    run(&mut e, "DEFINE PATTERN p AS a = 1 WEIGHT (a) USING (a)").expect("define");

    let mut a = args_for("p", "b");
    a.excluding = vec!["conf".to_string()];
    let env = hunt_v2_orchestrate(&mut e, &a).expect("hunt v2");
    assert_eq!(env.verdict, "sat");
    assert_eq!(env.n_matches, Some(3), "5 candidates minus {{2,4}} = {{1,3,5}}");
    let ids: Vec<i64> = env
        .rows
        .iter()
        .filter_map(|r| match r.get("id") {
            Some(Value::Integer(i)) => Some(*i),
            _ => None,
        })
        .collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort();
    assert_eq!(sorted_ids, vec![1, 3, 5]);
}

// ─── W5: explain=true attaches _explain to each sat row ────────────────────

#[test]
fn w5_explain_flag_attaches_explain_tree_per_row() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER, x INT FIBER)").expect("create");
    for (id, a, x) in [(1, 1, 2), (2, 1, 5)] {
        run(&mut e, &format!("INSERT INTO b (id, a, x) VALUES ({id}, {a}, {x})")).expect("ins");
    }
    run(
        &mut e,
        "DEFINE PATTERN p AS a = 1 WEIGHT (a * 3 + x) USING (a, x)",
    )
    .expect("define");

    let mut args = args_for("p", "b");
    args.explain = true;
    let env = hunt_v2_orchestrate(&mut e, &args).expect("hunt v2");
    assert_eq!(env.verdict, "sat");
    assert_eq!(env.rows.len(), 2);
    // Every sat row should have an `_explain` JSON-encoded tree.
    for row in &env.rows {
        let explain_val = row.get("_explain");
        assert!(
            explain_val.is_some(),
            "row should carry _explain when explain=true: {row:?}"
        );
    }
}

// ─── W6: include_repair_menu=true attaches repair_menu to near-miss rows ───

#[test]
fn w6_repair_menu_flag_attaches_repair_menu_to_near_miss() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER, x INT FIBER)").expect("create");
    for id in 1..=3 {
        run(&mut e, &format!("INSERT INTO b (id, a, x) VALUES ({id}, 1, 1)")).expect("ins");
    }
    run(
        &mut e,
        "DEFINE PATTERN p AS a = 1 AND x = 0 WEIGHT (a) USING (a, x)",
    )
    .expect("define");

    let mut args = args_for("p", "b");
    args.include_repair_menu = true;
    let env = hunt_v2_orchestrate(&mut e, &args).expect("hunt v2");
    assert_eq!(env.verdict, "near_miss");
    assert_eq!(env.near_miss_rows.len(), 3);
    for nm in &env.near_miss_rows {
        let menu = nm.row.get("_repair_menu");
        assert!(menu.is_some(), "near-miss row should carry _repair_menu");
    }
}

// ─── W7: Backwards-compat — v0.1 args (budget=0 with no flags) returns
//        the v0.1 shape (envelope with verdict=sat, rows present, no extras) ─

#[test]
fn w7_v0_1_compat_default_when_no_v2_flags() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER)").expect("create");
    run(&mut e, "INSERT INTO b (id, a) VALUES (1, 1)").expect("ins");
    run(&mut e, "DEFINE PATTERN p AS a = 1 WEIGHT (a) USING (a)").expect("define");

    let mut args = args_for("p", "b");
    args.near_miss_budget = 0;
    args.explain = false;
    args.include_repair_menu = false;
    let env = hunt_v2_orchestrate(&mut e, &args).expect("hunt v2");
    assert_eq!(env.verdict, "sat");
    assert_eq!(env.rows.len(), 1);
    // _explain MUST NOT be present
    assert!(env.rows[0].get("_explain").is_none());
}

// ─── W8: TOP + PROJECT compose with verdict envelope ───────────────────────

#[test]
fn w8_top_and_project_compose() {
    let mut e = fresh_engine();
    run(&mut e, "CREATE BUNDLE b (id INT BASE, a INT FIBER)").expect("create");
    for id in 1..=5 {
        run(&mut e, &format!("INSERT INTO b (id, a) VALUES ({id}, 1)")).expect("ins");
    }
    run(&mut e, "DEFINE PATTERN p AS a = 1 WEIGHT (a) USING (a)").expect("define");

    let mut args = args_for("p", "b");
    args.top = Some(2);
    args.project = Some(vec!["id".to_string(), "_score".to_string()]);
    let env = hunt_v2_orchestrate(&mut e, &args).expect("hunt v2");
    assert_eq!(env.verdict, "sat");
    assert_eq!(env.rows.len(), 2);
    for row in &env.rows {
        let keys: std::collections::HashSet<_> = row.keys().cloned().collect();
        assert_eq!(keys, ["id", "_score"].iter().map(|s| s.to_string()).collect());
    }
}
