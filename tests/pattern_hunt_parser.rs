//! Phase 1 of the Pattern Hunt spec (Ask G — Patterns):
//! parser-only surface for DEFINE PATTERN / HUNT / DROP PATTERN /
//! SHOW PATTERNS / EXCLUDING IN.
//!
//! Gates PH1–PH4 from `theory/scj/PATTERN_HUNT_SPEC_v0.1.md` §3.5.
//!
//! ### Domain-neutrality is load-bearing.
//!
//! These tests exercise the GRAMMAR, not any single consumer's domain.
//! Every field name is generic (`field_a`, `score`, `category`,
//! `recent_absence_count`). The grammar must work for any consumer —
//! SCJ binary-vuln hunting, hypothetical PRISM fraud detection,
//! hypothetical DGSA at-risk-student identification, Marcella
//! discourse-flow analysis — and the test surface treats this as
//! load-bearing rather than incidental.
//!
//! The substrate doesn't know what a "bug" is or what a "fraudulent
//! transaction" is. It knows how to parse a named, weighted, predicate-
//! filtered ranked query against a bundle. That's the level of the
//! abstraction this test file pins.
//!
//! ### Scope.
//!
//! Parser only. No registry, no executor, no `_score` evaluation, no
//! sharding. Phase 2 onward adds those.
//!
//! ### Behind `patterns` feature flag.
//!
//! `#![cfg(feature = "patterns")]` at the top. Default build skips
//! this file entirely; `--features patterns` exercises it. PH4 is
//! enforced by Cargo's feature isolation, not by an in-file assert.

#![cfg(feature = "patterns")]

use gigi::parser::{parse, Statement};

// ─── PH1 — DEFINE PATTERN parses and round-trips through AST ────────────────

#[test]
fn ph1_define_pattern_minimal_parses() {
    let src = "DEFINE PATTERN p AS field_a = 1";
    let parsed = parse(src).expect("DEFINE PATTERN with minimal body must parse");
    match parsed {
        Statement::DefinePattern { name, pred, .. } => {
            assert_eq!(name, "p", "pattern name");
            assert_eq!(pred.len(), 1, "one predicate clause");
        }
        other => panic!("expected Statement::DefinePattern, got {other:?}"),
    }
}

#[test]
fn ph1_define_pattern_with_weight_and_using_parses() {
    let src = "DEFINE PATTERN composite AS \
               field_a = 1 AND field_b > 5 \
               WEIGHT (field_a * 2.0 + field_b) \
               USING (field_a, field_b)";
    let parsed = parse(src).expect("full DEFINE PATTERN must parse");
    match parsed {
        Statement::DefinePattern {
            name,
            pred,
            weight,
            using_fields,
            ..
        } => {
            assert_eq!(name, "composite");
            assert_eq!(pred.len(), 2, "two AND'd predicate clauses");
            assert!(
                weight.is_some(),
                "WEIGHT clause must produce a populated weight field"
            );
            assert_eq!(
                using_fields,
                vec!["field_a".to_string(), "field_b".to_string()],
                "USING clause field list"
            );
        }
        other => panic!("expected DefinePattern, got {other:?}"),
    }
}

#[test]
fn ph1_define_pattern_without_optional_clauses_succeeds() {
    // The minimal form leaves WEIGHT and USING absent. Verify the
    // Optional/Vec-empty defaults match.
    let parsed = parse("DEFINE PATTERN bare AS x = 0").expect("must parse");
    match parsed {
        Statement::DefinePattern {
            weight,
            using_fields,
            ..
        } => {
            assert!(weight.is_none(), "WEIGHT absent → None");
            assert!(using_fields.is_empty(), "USING absent → empty Vec");
        }
        _ => panic!("wrong variant"),
    }
}

// ─── PH2 — HUNT parses with optional clauses in flexible order ──────────────

#[test]
fn ph2_hunt_minimal_parses() {
    let parsed = parse("HUNT p IN bundle_a").expect("minimal HUNT must parse");
    match parsed {
        Statement::Hunt {
            pattern,
            bundle,
            excluding,
            top,
            project,
            ..
        } => {
            assert_eq!(pattern, "p");
            assert_eq!(bundle, "bundle_a");
            assert!(excluding.is_empty(), "no EXCLUDING IN → empty Vec");
            assert!(top.is_none(), "no TOP → None");
            assert!(project.is_none(), "no PROJECT → None");
        }
        other => panic!("expected Statement::Hunt, got {other:?}"),
    }
}

#[test]
fn ph2_hunt_with_excluding_top_project_parses() {
    let src = "HUNT p IN bundle_a \
               EXCLUDING IN bundle_b \
               EXCLUDING IN bundle_c \
               TOP 50 \
               PROJECT (name, _score)";
    let parsed = parse(src).expect("full HUNT must parse");
    match parsed {
        Statement::Hunt {
            pattern,
            bundle,
            excluding,
            top,
            project,
            ..
        } => {
            assert_eq!(pattern, "p");
            assert_eq!(bundle, "bundle_a");
            assert_eq!(
                excluding,
                vec!["bundle_b".to_string(), "bundle_c".to_string()],
                "EXCLUDING IN clauses preserve order"
            );
            assert_eq!(top, Some(50));
            assert!(project.is_some(), "PROJECT clause populated");
        }
        other => panic!("expected Hunt, got {other:?}"),
    }
}

// ─── PH3 — predicate operator surface = COVER's, no novel operators ─────────

#[test]
fn ph3_binary_comparison_operators_parse() {
    // Every comparison operator that COVER supports must also work
    // inside DEFINE PATTERN. Surface parity is the contract.
    for op in ["=", "!=", "<", ">", "<=", ">="] {
        let src = format!("DEFINE PATTERN p AS field_a {op} 5");
        parse(&src).unwrap_or_else(|e| {
            panic!("operator {op:?} failed to parse inside DEFINE PATTERN: {e}")
        });
    }
}

#[test]
fn ph3_set_membership_operators_parse() {
    for clause in ["field_a IN (1, 2, 3)", "field_a NOT IN (1, 2, 3)"] {
        let src = format!("DEFINE PATTERN p AS {clause}");
        parse(&src).unwrap_or_else(|e| panic!("`{clause}` failed: {e}"));
    }
}

#[test]
fn ph3_and_or_combinators_parse() {
    parse("DEFINE PATTERN p AS field_a = 1 AND field_b = 2")
        .expect("AND combinator must work");
    parse("DEFINE PATTERN q AS field_a = 1 OR field_b = 2")
        .expect("OR combinator must work");
}

// ─── PH4 — drop + show forms parse cleanly ──────────────────────────────────
// (The "no-feature build byte-identical" half of PH4 is enforced by Cargo
//  feature isolation. With `--features patterns` off, this whole file is
//  skipped at compile time; the dispatcher and AST stay unmodified.)

#[test]
fn ph4_drop_pattern_parses() {
    let parsed = parse("DROP PATTERN p").expect("DROP PATTERN must parse");
    match parsed {
        Statement::DropPattern { name } => assert_eq!(name, "p"),
        other => panic!("expected DropPattern, got {other:?}"),
    }
}

#[test]
fn ph4_show_patterns_parses() {
    let parsed = parse("SHOW PATTERNS").expect("SHOW PATTERNS must parse");
    matches!(parsed, Statement::ShowPatterns)
        .then_some(())
        .expect("expected Statement::ShowPatterns");
}

// ─── Domain-neutrality smoke ────────────────────────────────────────────────
//
// The same grammar must serve consumer styles the substrate can't
// inspect — vuln-hunt, fraud, education, discourse. If any of these
// fails to parse, the grammar has accidentally specialized to one
// consumer's domain. Treat regressions here as P0.

#[test]
fn general_purpose_vuln_hunt_style_parses() {
    // SCJ-shaped. Generic field names, no proprietary vocabulary.
    let src = "DEFINE PATTERN int_overflow_alloc AS \
               has_alloc = 1 AND has_arith = 1 AND uses_untrusted_size = 1 \
               WEIGHT (has_alloc * 3.0 + has_arith * 2.0 + uses_untrusted_size * 3.0) \
               USING (has_alloc, has_arith, uses_untrusted_size)";
    parse(src).expect("vuln-hunt-style pattern must parse");
}

#[test]
fn general_purpose_fraud_detection_style_parses() {
    // Hypothetical PRISM-shape. Bigger numeric ranges; merchant-age field.
    let src = "DEFINE PATTERN suspicious_txn AS \
               amount > 10000 AND merchant_age_days < 30 \
               WEIGHT (amount * 0.0001 + merchant_age_days * 0.05) \
               USING (amount, merchant_age_days)";
    parse(src).expect("fraud-detection-style pattern must parse");
}

#[test]
fn general_purpose_at_risk_student_style_parses() {
    // Hypothetical DGSA-shape. Counts + recent activity.
    let src = "DEFINE PATTERN attendance_concern AS \
               recent_absence_count > 3 \
               WEIGHT (recent_absence_count * 1.5)";
    parse(src).expect("at-risk-student-style pattern must parse");
}

#[test]
fn general_purpose_discourse_flow_style_parses() {
    // Marcella-shape. Transitions, dwell times, semantic markers.
    let src = "DEFINE PATTERN coherence_break AS \
               transition_count > 5 AND avg_dwell_ms < 200 \
               WEIGHT (transition_count + avg_dwell_ms)";
    parse(src).expect("discourse-flow-style pattern must parse");
}

// ─── HUNT composability across the same consumer styles ─────────────────────

#[test]
fn general_purpose_hunt_works_for_all_consumer_styles() {
    for src in [
        // Vuln hunt:
        "HUNT int_overflow_alloc IN drivers EXCLUDING IN confirmed_bugs TOP 50",
        // Fraud:
        "HUNT suspicious_txn IN transactions EXCLUDING IN cleared_merchants TOP 100",
        // Education:
        "HUNT attendance_concern IN students EXCLUDING IN already_in_intervention TOP 25",
        // Discourse:
        "HUNT coherence_break IN dialogues EXCLUDING IN curated_neutral TOP 20",
    ] {
        parse(src).unwrap_or_else(|e| panic!("HUNT must parse: `{src}` — error: {e}"));
    }
}
