//! Patterns v0.2 — Phase K_P: pattern curvature
//!
//! Per-bundle scalar measuring how concentrated the pattern's matching
//! neighborhood is in the kNN graph induced by the predicate's fields.
//!
//! K_P = Var_i[ |{j ∈ N_k(i) : pred matches j}| / k ]
//!
//! Math target: `theory/patterns/validation_tests.py` K1-K4 (shipped 30/30).

#![cfg(feature = "patterns")]

use gigi::parser::{
    pattern_curvature, FilterCondition, Literal, PatternCurvature,
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

// ─── K1: concentrated pattern has strictly positive K_P ────────────────────

#[test]
fn k1_kp_concentrated_pattern_is_strictly_positive() {
    let mut records: Vec<Record> = Vec::new();
    // 20 matching rows clustered at (a=1, b=1, c=1)
    for _ in 0..20 {
        records.push(rec_int(&[("a", 1), ("b", 1), ("c", 1)]));
    }
    // 80 non-matching rows scattered through the (a,b,c) cube (avoid 1,1,1)
    let combos = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1),
                  (1, 1, 0), (1, 0, 1), (0, 1, 1)];
    for pk in 0..80 {
        let (a, b, c) = combos[pk % combos.len()];
        records.push(rec_int(&[("a", a), ("b", b), ("c", c)]));
    }

    let pred = vec![
        FilterCondition::Eq("a".to_string(), Literal::Integer(1)),
        FilterCondition::Eq("b".to_string(), Literal::Integer(1)),
        FilterCondition::Eq("c".to_string(), Literal::Integer(1)),
    ];
    let fields = vec!["a".to_string(), "b".to_string(), "c".to_string()];

    let pc = pattern_curvature(&pred, &records, &fields, 8);
    assert_eq!(pc.n_matches, 20);
    assert_eq!(pc.k, 8);
    assert!(pc.k_p > 0.0, "concentrated pattern K_P should be > 0, got {}", pc.k_p);
}

// ─── K2: same pattern, two bundles — clustered placement exceeds scattered ──

#[test]
fn k2_kp_responds_to_match_concentration() {
    // Bundle A — clustered: matching rows share noise fingerprint with each other
    let mut a_records = Vec::new();
    for _ in 0..20 {
        // Matching cluster: (flag=1, n0=0, n1=0, n2=0)
        a_records.push(rec_int(&[("flag", 1), ("n0", 0), ("n1", 0), ("n2", 0)]));
    }
    for _ in 0..20 {
        // Non-matching: (flag=0, n0=1, n1=1, n2=1)
        a_records.push(rec_int(&[("flag", 0), ("n0", 1), ("n1", 1), ("n2", 1)]));
    }

    // Bundle B — scattered: matches have iid-ish noise fingerprints
    let mut b_records = Vec::new();
    for pk in 0..40 {
        let flag = if pk < 20 { 1 } else { 0 };
        // Use pk-derived deterministic noise so the test is reproducible
        // and noise patterns are spread across rows.
        let n0 = (pk % 2) as i64;
        let n1 = ((pk / 2) % 2) as i64;
        let n2 = ((pk / 4) % 2) as i64;
        b_records.push(rec_int(&[("flag", flag), ("n0", n0), ("n1", n1), ("n2", n2)]));
    }

    let pred = vec![FilterCondition::Eq("flag".to_string(), Literal::Integer(1))];
    let fields = vec!["flag".to_string(), "n0".to_string(),
                      "n1".to_string(), "n2".to_string()];

    let kp_a = pattern_curvature(&pred, &a_records, &fields, 5).k_p;
    let kp_b = pattern_curvature(&pred, &b_records, &fields, 5).k_p;
    assert!(
        kp_a > kp_b,
        "clustered K_P {kp_a:.4} should exceed scattered K_P {kp_b:.4}"
    );
}

// ─── K3: empty match → K_P = 0 by convention ───────────────────────────────

#[test]
fn k3_kp_empty_match_is_zero() {
    let records: Vec<Record> = (0..10).map(|_| rec_int(&[("a", 0)])).collect();
    let pred = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(1))];
    let fields = vec!["a".to_string()];
    let pc = pattern_curvature(&pred, &records, &fields, 3);
    assert_eq!(pc.n_matches, 0);
    assert_eq!(pc.k_p, 0.0);
}

// ─── K4: tautology K_P = 0; concentrated K_P > tautology K_P ────────────────

#[test]
fn k4_kp_concentrated_exceeds_tautology() {
    let records: Vec<Record> = (0..100)
        .map(|i| {
            let mut r = Record::new();
            r.insert("a".to_string(), Value::Integer(if i < 20 { 1 } else { 0 }));
            r.insert("always".to_string(), Value::Integer(1));
            r
        })
        .collect();

    let pred_conc = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(1))];
    let pred_taut = vec![FilterCondition::Eq("always".to_string(), Literal::Integer(1))];
    let fields = vec!["a".to_string(), "always".to_string()];

    let pc_conc = pattern_curvature(&pred_conc, &records, &fields, 8);
    let pc_taut = pattern_curvature(&pred_taut, &records, &fields, 8);

    assert_eq!(pc_conc.n_matches, 20);
    assert_eq!(pc_taut.n_matches, 100);
    assert_eq!(pc_taut.k_p, 0.0, "tautology must have K_P=0");
    assert!(pc_conc.k_p > pc_taut.k_p);
}

// ─── Domain swap (§8): identical math across four field families ───────────

fn run_ds_kp(field: &str) -> f64 {
    let records: Vec<Record> = (0..40)
        .map(|i| {
            let mut r = Record::new();
            r.insert(field.to_string(), Value::Integer(if i < 20 { 1 } else { 0 }));
            r.insert("noise".to_string(), Value::Integer(i as i64));
            r
        })
        .collect();
    let pred = vec![FilterCondition::Eq(field.to_string(), Literal::Integer(1))];
    let fields = vec![field.to_string(), "noise".to_string()];
    pattern_curvature(&pred, &records, &fields, 5).k_p
}

#[test]
fn ds_kp_domain_swap_identical_numerical_output() {
    // Same data shape, four domain field names — bit-identical K_P.
    let vuln = run_ds_kp("cast_truncate_alloc");
    let fraud = run_ds_kp("amount_over_threshold");
    let edu = run_ds_kp("assignments_complete");
    let disc = run_ds_kp("dialog_act_question");
    assert_eq!(vuln, fraud);
    assert_eq!(fraud, edu);
    assert_eq!(edu, disc);
}

// ─── PatternCurvature shape contract ───────────────────────────────────────

#[test]
fn kp_struct_carries_k_p_n_matches_k_n_rows() {
    let records: Vec<Record> = (0..20)
        .map(|i| rec_int(&[("a", if i < 5 { 1 } else { 0 })]))
        .collect();
    let pred = vec![FilterCondition::Eq("a".to_string(), Literal::Integer(1))];
    let fields = vec!["a".to_string()];
    let pc: PatternCurvature = pattern_curvature(&pred, &records, &fields, 4);
    assert_eq!(pc.k, 4);
    assert_eq!(pc.n_matches, 5);
    assert_eq!(pc.n_rows, 20);
    assert!(pc.k_p >= 0.0);
}
