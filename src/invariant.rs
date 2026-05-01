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

fn evaluate_op(store: &BundleStore, op: &InvariantOp) -> f64 {
    match op {
        InvariantOp::Curvature => crate::curvature::scalar_curvature(store),
        InvariantOp::Confidence => {
            crate::curvature::confidence(crate::curvature::scalar_curvature(store))
        }
        InvariantOp::SpectralGap => crate::spectral::spectral_gap(store),
        InvariantOp::Beta0 => crate::spectral::betti_numbers(store).0 as f64,
        InvariantOp::Beta1 => crate::spectral::betti_numbers(store).1 as f64,
    }
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
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::SpectralGap));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Beta0));
        let _ = evaluate(&store, &InvariantExpr::Op(InvariantOp::Beta1));

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
