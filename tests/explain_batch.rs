//! EXPLAIN SECTION … AT <field> IN (…) — batch form (Marcella
//! EXPLAIN-family ask 2).
//!
//! Contract under test:
//!   - grouped rows: each group is one record's FULL EXPLAIN output
//!     (scalar rows + record_kappa on every row + optional vector
//!     row), stamped with a group discriminator column = the key value
//!   - groups come back in INPUT order (the caller's list is the
//!     contract, unlike PER-grouping which sorts ascending)
//!   - missing individual keys do NOT fail the batch and are NOT
//!     silently skipped: each miss emits one typed row
//!     (kind='miss') naming the key and bundle
//!   - the batch runs under one engine read-lock (one store
//!     resolution — structural, exercised implicitly)
//!   - the invariant holds PER GROUP: mean(scalar kappa) ==
//!     record_kappa to 1e-9, vector rows excluded

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};
use gigi::types::{Record, Value};

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

fn rows(e: &mut Engine, stmt: &str) -> Vec<Record> {
    match run(e, stmt).unwrap() {
        ExecResult::Rows(rows) => rows,
        other => panic!("expected Rows from `{stmt}`, got {other:?}"),
    }
}

/// id TEXT base; a, b numeric fibers; three records.
fn engine3() -> (tempfile::TempDir, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (a NUMERIC, b NUMERIC);").unwrap();
    run(&mut e, "SECTION st (id='r1', a=1.0, b=10.0);").unwrap();
    run(&mut e, "SECTION st (id='r2', a=2.0, b=30.0);").unwrap();
    run(&mut e, "SECTION st (id='r3', a=4.0, b=20.0);").unwrap();
    (dir, e)
}

fn id_of(r: &Record) -> &str {
    r.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("every batch row carries the discriminator column: {r:?}"))
}

fn kind_of(r: &Record) -> Option<&str> {
    r.get("kind").and_then(|v| v.as_str())
}

/// Split batch rows into (discriminator, group-rows) runs, input order.
fn groups(rows: &[Record]) -> Vec<(String, Vec<&Record>)> {
    let mut out: Vec<(String, Vec<&Record>)> = Vec::new();
    for r in rows {
        let id = id_of(r).to_string();
        match out.last_mut() {
            Some((last_id, grp)) if *last_id == id => grp.push(r),
            _ => out.push((id, vec![r])),
        }
    }
    out
}

fn assert_group_invariant(grp: &[&Record]) {
    let scal: Vec<&&Record> = grp
        .iter()
        .filter(|r| kind_of(r) != Some("vector") && kind_of(r) != Some("miss"))
        .collect();
    assert!(!scal.is_empty());
    let record_kappa = scal[0]["record_kappa"].as_f64().unwrap();
    for r in &scal {
        assert!(
            (r["record_kappa"].as_f64().unwrap() - record_kappa).abs() < 1e-12,
            "record_kappa constant within a group"
        );
    }
    let mean = scal
        .iter()
        .map(|r| r["kappa"].as_f64().unwrap())
        .sum::<f64>()
        / scal.len() as f64;
    assert!(
        (mean - record_kappa).abs() < 1e-9,
        "per-group invariant: mean(scalar kappa) {mean} == record_kappa {record_kappa}"
    );
}

#[test]
fn batch_groups_in_input_order_with_discriminator_on_every_row() {
    let (_d, mut e) = engine3();
    let all = rows(&mut e, "EXPLAIN SECTION st AT id IN ('r2', 'r1');");
    // two fibers → two rows per group
    assert_eq!(all.len(), 4);
    let gs = groups(&all);
    assert_eq!(gs.len(), 2);
    assert_eq!(gs[0].0, "r2", "input order, not sorted");
    assert_eq!(gs[1].0, "r1");
    for (_, grp) in &gs {
        assert_eq!(grp.len(), 2);
        // full EXPLAIN output per group: field/kappa columns, sorted
        // loudest-first within the group
        let k0 = grp[0]["kappa"].as_f64().unwrap();
        let k1 = grp[1]["kappa"].as_f64().unwrap();
        assert!(k0 >= k1, "rows sorted kappa-desc within each group");
        assert_group_invariant(grp);
    }
    // Groups genuinely differ (different records).
    let rk_r2 = gs[0].1[0]["record_kappa"].as_f64().unwrap();
    let rk_r1 = gs[1].1[0]["record_kappa"].as_f64().unwrap();
    assert!((rk_r2 - rk_r1).abs() > 1e-12);
}

#[test]
fn batch_missing_key_emits_typed_miss_row_not_failure() {
    let (_d, mut e) = engine3();
    let all = rows(
        &mut e,
        "EXPLAIN SECTION st AT id IN ('r1', 'ghost', 'r3');",
    );
    // r1: 2 rows, ghost: 1 miss row, r3: 2 rows — input order.
    assert_eq!(all.len(), 5);
    let gs = groups(&all);
    assert_eq!(gs.len(), 3);
    assert_eq!(gs[0].0, "r1");
    assert_eq!(gs[1].0, "ghost");
    assert_eq!(gs[2].0, "r3");

    let miss = &gs[1].1;
    assert_eq!(miss.len(), 1, "one typed miss entry per missing key");
    assert_eq!(kind_of(miss[0]), Some("miss"));
    let msg = miss[0]["miss"].as_str().unwrap();
    assert!(msg.contains("no section"), "{msg}");
    assert!(msg.contains("'st'"), "miss names the bundle: {msg}");
    assert!(msg.contains("ghost"), "miss names the key value: {msg}");

    // The found groups still carry the full invariant.
    assert_group_invariant(&gs[0].1);
    assert_group_invariant(&gs[2].1);
}

#[test]
fn batch_all_missing_is_still_ok_rows_of_misses() {
    let (_d, mut e) = engine3();
    let all = rows(&mut e, "EXPLAIN SECTION st AT id IN ('x', 'y');");
    assert_eq!(all.len(), 2);
    assert!(all.iter().all(|r| kind_of(r) == Some("miss")));
    assert_eq!(id_of(&all[0]), "x");
    assert_eq!(id_of(&all[1]), "y");
}

#[test]
fn batch_with_vector_clause_adds_kappa_v_row_per_group() {
    // Same fixture as the vector suite: a=(2,0), b=(0,1) over v0,v1.
    // kappa_v(a) = √5−2, kappa_v(b) = √5−1 — vector context (mu, R_cos)
    // is bundle-level and shared across the batch.
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE st BASE (id TEXT) FIBER (v0 NUMERIC, v1 NUMERIC);").unwrap();
    run(&mut e, "SECTION st (id='a', v0=2.0, v1=0.0);").unwrap();
    run(&mut e, "SECTION st (id='b', v0=0.0, v1=1.0);").unwrap();

    let s5 = 5f64.sqrt();
    let all = rows(
        &mut e,
        "EXPLAIN SECTION st AT id IN ('a', 'ghost', 'b') VECTOR (v0..v1);",
    );
    let gs = groups(&all);
    assert_eq!(gs.len(), 3);

    // group a: 2 scalar rows + 1 vector row
    assert_eq!(gs[0].0, "a");
    assert_eq!(gs[0].1.len(), 3);
    let va = gs[0]
        .1
        .iter()
        .find(|r| kind_of(r) == Some("vector"))
        .expect("vector row in group a");
    assert_eq!(va["field"].as_str().unwrap(), "vector(v0..v1)");
    assert!((va["kappa"].as_f64().unwrap() - (s5 - 2.0)).abs() < 1e-9);
    assert_group_invariant(&gs[0].1);

    // group ghost: exactly the typed miss, no vector row fabricated
    assert_eq!(gs[1].1.len(), 1);
    assert_eq!(kind_of(gs[1].1[0]), Some("miss"));

    // group b
    assert_eq!(gs[2].0, "b");
    let vb = gs[2]
        .1
        .iter()
        .find(|r| kind_of(r) == Some("vector"))
        .expect("vector row in group b");
    assert!((vb["kappa"].as_f64().unwrap() - (s5 - 1.0)).abs() < 1e-9);
    assert_group_invariant(&gs[2].1);
}

#[test]
fn batch_single_key_form_unchanged() {
    // No IN — the single-key grammar and row shape stay exactly as
    // before (no discriminator column stamped).
    let (_d, mut e) = engine3();
    let all = rows(&mut e, "EXPLAIN SECTION st AT id='r1';");
    assert_eq!(all.len(), 2);
    assert!(
        all.iter().all(|r| r.get("id").is_none()),
        "single-key rows carry no discriminator column"
    );
}

#[test]
fn batch_integer_keys_work() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();
    run(&mut e, "BUNDLE nums BASE (id INT) FIBER (x NUMERIC);").unwrap();
    run(&mut e, "SECTION nums (id=1, x=0.5);").unwrap();
    run(&mut e, "SECTION nums (id=2, x=1.5);").unwrap();
    let all = rows(&mut e, "EXPLAIN SECTION nums AT id IN (1, 7);");
    let g1: Vec<&Record> = all
        .iter()
        .filter(|r| r.get("id") == Some(&Value::Integer(1)))
        .collect();
    assert_eq!(g1.len(), 1, "x is the only fiber");
    assert!(kind_of(g1[0]).is_none());
    let g7: Vec<&Record> = all
        .iter()
        .filter(|r| r.get("id") == Some(&Value::Integer(7)))
        .collect();
    assert_eq!(g7.len(), 1);
    assert_eq!(kind_of(g7[0]), Some("miss"));
}
