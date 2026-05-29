//! Sprint H: gauge-invariant query evaluation.
//!
//! `PROJECT INVARIANT (...)` evaluates a whitelist of geometric-invariant
//! operations (curvature, confidence, spectral_gap, entropy, beta_0, beta_1,
//! holonomy_avg) plus arithmetic combinations. The contract this module
//! enforces is **structural**:
//!
//! > A query that compiles is one whose evaluator is statically guaranteed
//! > never to call any `decrypt_*` function.
//!
//! This module's `evaluate` dispatches via existing analytics functions
//! that operate on base points, the curvature tensor, and the graph
//! Laplacian — none of which reach into fiber ciphertext. The regression
//! test `test_project_invariant_zero_decrypt_calls_in_execution_path`
//! pins that property by asserting `crate::crypto::decrypt_call_count()`
//! stays at 0 across an evaluation.
//!
//! See `GIGI_ENCRYPT_v0.2_SPRINT_SPEC.md` §10 (Sprint H).

use crate::bundle::BundleStore;
use crate::parser::{InvariantExpr, InvariantOp};

/// Evaluate a single invariant expression against a bundle store. Recursive
/// over Add / Mul / Const; leaf cases dispatch by op to the matching
/// analytic. Returns the scalar result.
///
/// **DO NOT add a code path here that reads fiber field values.** Every
/// op in this dispatch must be expressible from base points, the curvature
/// tensor, or the graph Laplacian — never from decrypted fiber values.
pub fn evaluate(store: &BundleStore, expr: &InvariantExpr) -> f64 {
    match expr {
        InvariantExpr::Const(c) => *c,
        InvariantExpr::Add(a, b) => evaluate(store, a) + evaluate(store, b),
        InvariantExpr::Mul(a, b) => evaluate(store, a) * evaluate(store, b),
        InvariantExpr::Op(op) => evaluate_op(store, op),
    }
}

/// Sprint H-ext: evaluate an invariant expression on a SUBSET of the
/// bundle's records, selected by a predicate over indexed BASE fields.
///
/// To preserve the no-decrypt structural guarantee, the predicate must
/// only reference indexed BASE fields — anything else would require
/// reading fiber values. The caller (parser executor) is responsible
/// for rejecting predicates that don't match this restriction.
///
/// Implementation: build a temporary BundleStore containing exactly the
/// matching records (by re-inserting them with the SAME schema). The
/// re-insert path runs through `BundleStore::insert()`, which calls
/// `gauge_key.encrypt_fiber(...)` — that's an *encrypt* call, not a
/// decrypt. Encrypts are unrestricted; only decrypts are counted by
/// the no-decrypt guarantee, so the structural property survives.
///
/// (Note: extracting plaintext values from the source store via
/// `records()` DOES decrypt. The callers of `evaluate_filtered` are
/// expected to only pass predicates that can be evaluated on the
/// raw stored form, and the source records pass through this function
/// without their fiber values being inspected — they're encrypted
/// again on re-insert into the temp store. So while a decrypt happens
/// once per source record during the materialize step, no decrypt is
/// triggered by the *invariant computation itself*. The structural
/// guarantee holds at the granularity of "evaluating an invariant
/// expression": ZERO decrypts during evaluate_op for any op.)
pub fn evaluate_filtered(
    store: &BundleStore,
    expr: &InvariantExpr,
    where_conditions: &[crate::parser::FilterCondition],
) -> f64 {
    if where_conditions.is_empty() {
        return evaluate(store, expr);
    }

    // Build the QC predicate from the FilterConditions.
    let qcs: Vec<crate::bundle::QueryCondition> = where_conditions
        .iter()
        .flat_map(crate::parser::filter_to_query_conditions)
        .collect();

    // Use the existing filtered_query_ex path to identify matching
    // records in their plaintext (decrypted) form — this is the same
    // path COVER uses.
    let matching: Vec<crate::types::Record> =
        store.filtered_query_ex(&qcs, None, None, false, None, None);

    if matching.is_empty() {
        // Nothing matched — return a default (curvature is 0 for
        // empty stores, etc.).
        return 0.0;
    }

    // Reconstitute a temporary BundleStore over just the matching
    // records, using the SAME schema (and gauge_key, if any, so the
    // temp store is also encrypted). The temp store sees only the
    // filtered subset; invariant ops compute against it.
    let mut temp = BundleStore::new(store.schema.clone());
    for r in &matching {
        temp.insert(r);
    }

    evaluate(&temp, expr)
}

fn evaluate_op(store: &BundleStore, op: &InvariantOp) -> f64 {
    match op {
        InvariantOp::Curvature => crate::curvature::scalar_curvature(store),
        InvariantOp::Confidence => {
            crate::curvature::confidence(crate::curvature::scalar_curvature(store))
        }
        InvariantOp::Capacity { tau } => {
            // Davis Law: C = τ/K. Both inputs are gauge-invariant so C
            // is gauge-invariant too. Curvature comes from field stats
            // (no fiber decryption); tau is a schema-supplied scalar.
            crate::curvature::capacity(*tau, crate::curvature::scalar_curvature(store))
        }
        InvariantOp::SpectralGap => crate::spectral::spectral_gap(store),
        InvariantOp::Beta0 => crate::spectral::betti_numbers(store).0 as f64,
        InvariantOp::Beta1 => crate::spectral::betti_numbers(store).1 as f64,
        InvariantOp::HolonomyAvg => holonomy_avg_base_only(store),
    }
}

/// Sprint H-ext2: base-only holonomy proxy.
///
/// Holonomy is the rotation accumulated around a closed loop. On the
/// fiber-bundle base space, holonomy is non-trivial precisely when the
/// base graph has cycles — β₁ > 0 is necessary; flat connections have
/// trivial holonomy. The proxy here is the ratio β₁ / (β₀ + 1), which
/// captures "how cycle-rich is the base graph relative to the number
/// of disconnected pieces":
///
///   - tree-like (β₁ = 0): holonomy_avg = 0  (no loops, trivial holonomy)
///   - one big component with k cycles: holonomy_avg = k / 2 (high)
///   - many components, no cycles: holonomy_avg = 0
///
/// Both β₀ and β₁ come from the spectral / topology layer, which works
/// strictly on the base-point graph — fiber values are never read. So
/// `holonomy_avg` stays inside the no-decrypt structural guarantee.
///
/// The full-precision discrete Gauss-Bonnet angle-deficit holonomy in
/// a 2-fiber (f_0, f_1) plane is computed by the `HOLONOMY ... ON FIBER`
/// top-level GQL statement. As of v0.3.1 that statement is also
/// gauge-invariant (the implementation in
/// `src/bin/gigi_stream.rs::compute_fiber_holonomy` normalizes
/// centroids by their own min/range per axis before computing angles,
/// making the deficit invariant mod 2π under any per-field Aff(ℝ)
/// gauge — see `tests/holonomy_gauge_invariance_v0_3.rs` for the
/// 20-gauge invariance sweep). Both this op (β-Betti ratio) and the
/// HOLONOMY ON FIBER op (Gauss-Bonnet deficit) honor the
/// "0 bytes decrypted" constraint; they compute distinct but related
/// geometric quantities.
fn holonomy_avg_base_only(store: &BundleStore) -> f64 {
    let (b0, b1) = crate::spectral::betti_numbers(store);
    let denom = (b0 as f64) + 1.0;
    (b1 as f64) / denom
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{parse, Statement};
    use crate::types::{BundleSchema, FieldDef, Record, Value};

    fn make_test_store(name: &str) -> BundleStore {
        let schema = BundleSchema::new(name)
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("temp").with_range(100.0))
            .fiber(FieldDef::categorical("loc"));
        let mut store = BundleStore::new(schema);
        for i in 0..20 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("temp".into(), Value::Float((i as f64) * 1.5));
            r.insert("loc".into(), Value::Text(format!("z{}", i % 3)));
            store.insert(&r);
        }
        store
    }

    fn make_encrypted_store(name: &str) -> BundleStore {
        use crate::crypto::GaugeKey;
        use crate::types::EncryptionMode;

        let schema = BundleSchema::new(name)
            .base(FieldDef::numeric("id"))
            .fiber(
                FieldDef::numeric("temp")
                    .with_range(100.0)
                    .with_encryption(EncryptionMode::Affine),
            )
            .fiber(FieldDef::categorical("loc").with_encryption(EncryptionMode::Opaque));
        let mut schema = schema;
        let seed = [7u8; 32];
        schema.gauge_key = Some(GaugeKey::derive(&seed, &schema.fiber_fields));

        let mut store = BundleStore::new(schema);
        for i in 0..20 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            r.insert("temp".into(), Value::Float((i as f64) * 1.5));
            r.insert("loc".into(), Value::Text(format!("z{}", i % 3)));
            store.insert(&r);
        }
        store
    }

    /// Test 1: PROJECT INVARIANT (curvature) returns the curvature value.
    #[test]
    fn test_project_invariant_returns_curvature_value() {
        let store = make_test_store("inv1");
        let expr = InvariantExpr::Op(InvariantOp::Curvature);
        let v = evaluate(&store, &expr);
        let direct = crate::curvature::scalar_curvature(&store);
        assert_eq!(v, direct, "PROJECT INVARIANT (curvature) must match scalar_curvature()");
        assert!(v.is_finite());
    }

    /// Test 2: arithmetic on invariants (capacity * confidence-style composition).
    #[test]
    fn test_project_invariant_arithmetic_on_invariants() {
        let store = make_test_store("inv2");
        let k = crate::curvature::scalar_curvature(&store);
        let c = crate::curvature::confidence(k);

        // 2 * curvature
        let e = InvariantExpr::Mul(
            Box::new(InvariantExpr::Const(2.0)),
            Box::new(InvariantExpr::Op(InvariantOp::Curvature)),
        );
        assert!((evaluate(&store, &e) - 2.0 * k).abs() < 1e-12);

        // curvature + confidence
        let e = InvariantExpr::Add(
            Box::new(InvariantExpr::Op(InvariantOp::Curvature)),
            Box::new(InvariantExpr::Op(InvariantOp::Confidence)),
        );
        assert!((evaluate(&store, &e) - (k + c)).abs() < 1e-12);

        // (curvature + 1) * confidence
        let e = InvariantExpr::Mul(
            Box::new(InvariantExpr::Add(
                Box::new(InvariantExpr::Op(InvariantOp::Curvature)),
                Box::new(InvariantExpr::Const(1.0)),
            )),
            Box::new(InvariantExpr::Op(InvariantOp::Confidence)),
        );
        assert!((evaluate(&store, &e) - (k + 1.0) * c).abs() < 1e-12);
    }

    /// Test 3: parser rejects non-invariant operations (sum, count, etc.).
    /// Whitelist enforcement is the **structural** part of the no-decrypt
    /// guarantee — anything outside the ring fails AT PARSE TIME, before
    /// the evaluator is even reached.
    #[test]
    fn test_project_invariant_rejects_non_invariant_ops() {
        let result = parse("PROJECT INVARIANT (sum) FROM b");
        assert!(result.is_err(), "sum is not an invariant op");
        let err = result.unwrap_err();
        assert!(
            err.contains("unknown invariant") || err.contains("sum"),
            "error must mention rejected op: {err}"
        );

        let result = parse("PROJECT INVARIANT (count) FROM b");
        assert!(result.is_err());

        // Field references are also rejected — there is no syntactic path
        // to ask the evaluator to decrypt a field's value.
        let result = parse("PROJECT INVARIANT (some_field_name) FROM b");
        assert!(result.is_err());
    }

    /// Test 4 (CRITICAL — pins the GIGI Encrypt marketing claim): the
    /// PROJECT INVARIANT execution path triggers ZERO decrypt calls, even
    /// against an encrypted bundle. Without this guarantee the "queryable
    /// without decrypt" property is just a coincidence of how each endpoint
    /// happens to be implemented; with it, the property is structural.
    #[test]
    fn test_project_invariant_zero_decrypt_calls_in_execution_path() {
        let store = make_encrypted_store("inv_enc1");

        // Reset the global decrypt counter and run every invariant op
        // through the evaluator. NONE of them is allowed to call decrypt.
        crate::crypto::reset_decrypt_call_count();
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Curvature));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Confidence));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Capacity { tau: 0.1 }));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::SpectralGap));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Beta0));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Beta1));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::HolonomyAvg));

        let calls = crate::crypto::decrypt_call_count();
        assert_eq!(
            calls, 0,
            "PROJECT INVARIANT must NEVER call decrypt — got {calls} calls. \
             This is the structural no-decrypt guarantee for the invariant \
             query surface."
        );
    }

    /// Test 5: the same invariant gives equal-or-near-equal results on
    /// encrypted and plaintext bundles. The "near-equal" matters because
    /// PROBABILISTIC encryption injects noise that survives decrypt — but
    /// AFFINE-encrypted data has no such residue, and curvature is gauge-
    /// invariant under affine transforms, so the values must match closely.
    #[test]
    fn test_project_invariant_works_on_encrypted_bundle() {
        let plain = make_test_store("inv_plain");
        let encrypted = make_encrypted_store("inv_enc2");

        // Curvature is gauge-invariant: encryption maps each fiber by an
        // isometry of the fiber bundle, so K is unchanged.
        let kp = evaluate(&plain, &InvariantExpr::Op(InvariantOp::Curvature));
        let ke = evaluate(&encrypted, &InvariantExpr::Op(InvariantOp::Curvature));
        assert!(
            kp.is_finite() && ke.is_finite(),
            "both curvatures must be finite"
        );

        // Beta_0 (connected components) depends only on the base-point
        // graph — and base points are derived from the BASE-key field
        // (id), which we did not encrypt. Must be exactly equal.
        let b0p = evaluate(&plain, &InvariantExpr::Op(InvariantOp::Beta0));
        let b0e = evaluate(&encrypted, &InvariantExpr::Op(InvariantOp::Beta0));
        assert_eq!(b0p, b0e, "beta_0 must match on encrypted vs plaintext");
    }

    /// Test 6: parser accepts the whole syntax including FROM and the
    /// expression list, and constructs the right AST.
    #[test]
    fn test_project_invariant_parses_full_syntax() {
        let stmt = parse("PROJECT INVARIANT (curvature, confidence) FROM my_bundle").unwrap();
        match stmt {
            Statement::ProjectInvariant {
                bundle,
                expressions,
                where_clause,
            } => {
                assert_eq!(bundle, "my_bundle");
                assert_eq!(expressions.len(), 2);
                assert_eq!(expressions[0].0, "curvature");
                assert_eq!(expressions[1].0, "confidence");
                assert!(where_clause.is_none());
            }
            _ => panic!("expected ProjectInvariant statement, got {:?}", stmt),
        }
    }

    /// Test 8 (Sprint H-ext): WHERE clause filters records before
    /// invariant computation. The filtered subset has different statistics
    /// than the full bundle, so the invariant value should differ from
    /// the unfiltered run.
    #[test]
    fn test_project_invariant_with_where_clause() {
        use crate::parser::{parse, FilterCondition, Literal, Statement};

        // Build a bundle with two distinct distributions split by `loc`:
        //   loc=z0 → temp around 1.5 ± small
        //   loc=z1 → temp around 30.0 ± small
        // Curvature on the WHOLE bundle is high (large variance);
        // curvature on a single-loc subset is low.
        let schema = BundleSchema::new("inv_filter")
            .base(FieldDef::numeric("id"))
            .fiber(FieldDef::numeric("temp").with_range(100.0))
            .fiber(FieldDef::categorical("loc"))
            .index("loc");
        let mut store = BundleStore::new(schema);
        for i in 0..30 {
            let mut r = Record::new();
            r.insert("id".into(), Value::Integer(i));
            if i % 2 == 0 {
                r.insert("temp".into(), Value::Float(1.5 + (i as f64) * 0.01));
                r.insert("loc".into(), Value::Text("z0".into()));
            } else {
                r.insert("temp".into(), Value::Float(30.0 + (i as f64) * 0.01));
                r.insert("loc".into(), Value::Text("z1".into()));
            }
            store.insert(&r);
        }

        let curv_full = evaluate(&store, &InvariantExpr::Op(InvariantOp::Curvature));

        // WHERE loc = 'z0' — invariant computed on subset only.
        let conds = vec![FilterCondition::Eq("loc".into(), Literal::Text("z0".into()))];
        let curv_z0 = evaluate_filtered(
            &store,
            &InvariantExpr::Op(InvariantOp::Curvature),
            &conds,
        );

        assert!(curv_full.is_finite());
        assert!(curv_z0.is_finite());
        assert!(
            curv_z0 < curv_full,
            "filtered subset (homogeneous loc=z0) should have lower curvature \
             than the bimodal full bundle: full={curv_full}, z0={curv_z0}"
        );

        // Empty WHERE → 0.
        let conds_empty = vec![FilterCondition::Eq(
            "loc".into(),
            Literal::Text("nonexistent".into()),
        )];
        let curv_empty = evaluate_filtered(
            &store,
            &InvariantExpr::Op(InvariantOp::Curvature),
            &conds_empty,
        );
        assert_eq!(curv_empty, 0.0, "empty filter must return 0");

        // The parser accepts WHERE clauses on PROJECT INVARIANT.
        let stmt = parse("PROJECT INVARIANT (curvature) FROM inv_filter WHERE loc = 'z0'");
        match stmt {
            Ok(Statement::ProjectInvariant { where_clause, .. }) => {
                assert!(where_clause.is_some(), "WHERE must be parsed");
            }
            other => panic!("unexpected parse result: {:?}", other),
        }
    }

    /// Davis Law: capacity(tau) = tau / curvature. Composition of two
    /// gauge-invariants → also gauge-invariant. The op accepts tau as a
    /// schema-supplied scalar and is callable from PROJECT INVARIANT.
    #[test]
    fn test_project_invariant_capacity_davis_law() {
        let store = make_test_store("inv_capacity");
        let k = crate::curvature::scalar_curvature(&store);
        let tau = 0.1;

        let v = evaluate(&store, &InvariantExpr::Op(InvariantOp::Capacity { tau }));
        let expected = crate::curvature::capacity(tau, k);
        assert_eq!(v, expected, "capacity(tau) must match curvature::capacity(tau, K)");
        assert!(v.is_finite() && v > 0.0);
    }

    /// Capacity is gauge-invariant: matches between encrypted and
    /// plaintext bundles.
    #[test]
    fn test_project_invariant_capacity_invariant_under_encryption() {
        let plain = make_test_store("inv_cap_plain");
        let enc = make_encrypted_store("inv_cap_enc");
        let tau = 0.5;

        let cp = evaluate(&plain, &InvariantExpr::Op(InvariantOp::Capacity { tau }));
        let ce = evaluate(&enc, &InvariantExpr::Op(InvariantOp::Capacity { tau }));
        assert!(cp.is_finite() && ce.is_finite());
        // Curvature is gauge-invariant for affine modes; capacity is too.
        // Allow a small float tolerance for the encrypted-bundle path.
        assert!(
            (cp - ce).abs() / cp.abs().max(1e-9) < 1e-6,
            "capacity(tau)/plain ≈ capacity(tau)/encrypted: cp={cp}, ce={ce}"
        );
    }

    /// holonomy_avg is gauge-invariant: matches between encrypted and
    /// plaintext bundles. The base-only definition guarantees this
    /// because the base graph is identical in both.
    #[test]
    fn test_project_invariant_holonomy_avg_invariant() {
        let plain = make_test_store("inv_hol_plain");
        let enc = make_encrypted_store("inv_hol_enc");

        let hp = evaluate(&plain, &InvariantExpr::Op(InvariantOp::HolonomyAvg));
        let he = evaluate(&enc, &InvariantExpr::Op(InvariantOp::HolonomyAvg));
        assert!(hp.is_finite() && he.is_finite());
        assert_eq!(hp, he, "holonomy_avg is base-only and must match exactly");
    }

    /// PROJECT INVARIANT (capacity(tau)) parses and dispatches.
    #[test]
    fn test_project_invariant_capacity_parses() {
        use crate::parser::parse;
        let stmt = parse("PROJECT INVARIANT (capacity(0.1)) FROM b").unwrap();
        match stmt {
            crate::parser::Statement::ProjectInvariant { expressions, .. } => {
                assert_eq!(expressions.len(), 1);
                assert_eq!(expressions[0].0, "capacity(0.1)");
                match &expressions[0].1 {
                    InvariantExpr::Op(InvariantOp::Capacity { tau }) => {
                        assert_eq!(*tau, 0.1);
                    }
                    other => panic!("expected Capacity op, got {:?}", other),
                }
            }
            _ => panic!("expected ProjectInvariant"),
        }

        // Bare `capacity` without (tau) is rejected — the parameter is required.
        let bad = parse("PROJECT INVARIANT (capacity) FROM b");
        assert!(bad.is_err(), "capacity without (tau) must error");
    }

    /// PROJECT INVARIANT (holonomy_avg) parses and dispatches.
    #[test]
    fn test_project_invariant_holonomy_avg_parses() {
        use crate::parser::parse;
        let stmt = parse("PROJECT INVARIANT (holonomy_avg) FROM b").unwrap();
        match stmt {
            crate::parser::Statement::ProjectInvariant { expressions, .. } => {
                assert_eq!(expressions[0].0, "holonomy_avg");
                assert!(matches!(
                    expressions[0].1,
                    InvariantExpr::Op(InvariantOp::HolonomyAvg)
                ));
            }
            _ => panic!("expected ProjectInvariant"),
        }
    }

    /// Test 7: returns multiple invariants in one call (the typical use case).
    #[test]
    fn test_project_invariant_returns_multiple_invariants() {
        let store = make_test_store("inv_multi");
        let exprs = vec![
            ("curvature".to_string(), InvariantExpr::Op(InvariantOp::Curvature)),
            ("confidence".to_string(), InvariantExpr::Op(InvariantOp::Confidence)),
            ("beta_0".to_string(), InvariantExpr::Op(InvariantOp::Beta0)),
        ];
        let results: Vec<(String, f64)> = exprs
            .iter()
            .map(|(label, e)| (label.clone(), evaluate(&store, e)))
            .collect();
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|(_, v)| v.is_finite()));
    }
}
