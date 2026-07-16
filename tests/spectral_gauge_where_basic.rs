//! Halcyon SPECTRAL_GAUGE Phase 1 — WHERE-clause sectoral filtering (Ask 4).
//!
//! RED phase: these tests pin the target grammar and semantics of
//! `SPECTRAL_GAUGE <bundle> WHERE <predicate> ON FIBER (...) [GROUP g]`
//! before the parser/kernel patches land. They must all fail against
//! main (8b30221) — the current AST variant has no `where_conditions`
//! field, the current kernel signature has no filter argument, and
//! there is no `EmptySubgraph` error variant yet.
//!
//! The WHERE clause is what unblocks Hallie's L=24 β=2.3 SU(2) OBC
//! sectoral workflow:
//!
//!   SPECTRAL_GAUGE su2_L24 WHERE q_rounded = 0 ON FIBER (q0, q1, q2, q3) GROUP SU(2);
//!
//! Records are pre-filtered by the predicate, the adjacency graph is
//! built from the filtered subset only (edges retained iff every
//! condition matches the record), then the Laplacian is constructed +
//! eigendecomposed on the reduced graph. n_records_used reports the
//! filtered count so callers observe the filter effect.
//!
//! Grammar (Phase 1):
//!   SPECTRAL_GAUGE <bundle>
//!     [WHERE <field> <op> <literal> [AND <field> <op> <literal>]*]
//!     ON FIBER (f1, f2, ...)
//!     [GROUP <g>]
//!     [FULL [LIMIT k]];
//!
//! ops: =  !=  <  <=  >  >=   (Phase 1; OR / parens deferred to Phase 2)
//!
//! Run with:
//!   `cargo test --features halcyon --test spectral_gauge_where_basic`

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::Group;
use gigi::parser::{parse, FilterCondition, Literal, Statement};
use gigi::spectral::{spectral_gauge_gap, SpectralGaugeError};
use gigi::types::{BundleSchema, FieldDef, Record, Value};

fn su2_field_names() -> &'static [&'static str] {
    &["q0", "q1", "q2", "q3"]
}

/// Edge bundle with vertex_a/vertex_b + fiber columns + an integer
/// `q_rounded` base column so we can sector-filter on it (mirrors
/// Hallie's per-config sector index).
fn make_sector_bundle(engine: &mut Engine, name: &str) {
    let mut schema = BundleSchema::new(name)
        .base(FieldDef::numeric("vertex_a"))
        .base(FieldDef::numeric("vertex_b"))
        .base(FieldDef::numeric("q_rounded"));
    for f in su2_field_names() {
        schema = schema.fiber(FieldDef::numeric(f));
    }
    engine
        .create_bundle(schema)
        .expect("create_bundle should succeed");
}

fn insert_sector_edge(
    engine: &mut Engine,
    name: &str,
    va: i64,
    vb: i64,
    q_rounded: i64,
    fiber_vals: &[f64],
) {
    let mut rec = Record::new();
    rec.insert("vertex_a".to_string(), Value::Integer(va));
    rec.insert("vertex_b".to_string(), Value::Integer(vb));
    rec.insert("q_rounded".to_string(), Value::Integer(q_rounded));
    for (f, v) in su2_field_names().iter().zip(fiber_vals.iter()) {
        rec.insert(f.to_string(), Value::Float(*v));
    }
    engine.insert(name, &rec).expect("insert should succeed");
}

fn identity_fiber() -> [f64; 4] {
    [1.0, 0.0, 0.0, 0.0]
}

// ────────────────────────────────────────────────────────────────────
// (1) Equality filter selects only matching records.
// ────────────────────────────────────────────────────────────────────
/// Two disjoint rings labelled q_rounded=0 (6 edges) and q_rounded=1
/// (4 edges). `WHERE q_rounded = 0` must build the graph from the
/// 6-edge ring only; the resulting gap must equal the ring6 algebraic
/// connectivity (2·(1 − cos(2π/6)) = 1.0), and n_records_used == 6.
#[test]
fn test_spectral_gauge_where_equality_filters_correctly() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_sector_bundle(&mut engine, "sectors");
    let id = identity_fiber();

    // Sector 0: 6-ring on vertices 0..5.
    for i in 0..6i64 {
        let j = (i + 1) % 6;
        insert_sector_edge(&mut engine, "sectors", i, j, 0, &id);
    }
    // Sector 1: 4-ring on vertices 100..103 (disjoint from sector 0).
    for i in 0..4i64 {
        let j = (i + 1) % 4;
        insert_sector_edge(&mut engine, "sectors", 100 + i, 100 + j, 1, &id);
    }

    let fiber: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let filter = vec![gigi::bundle::QueryCondition::Eq(
        "q_rounded".to_string(),
        gigi::types::Value::Integer(0),
    )];
    let result = spectral_gauge_gap(
        &engine,
        "sectors",
        &fiber,
        Group::SU2,
        false,
        None,
        Some(&filter),
    )
    .expect("sector 0 subgraph should give a gap");

    let expected_ring6 = 2.0 * (1.0 - (std::f64::consts::PI / 3.0).cos());
    assert!(
        (result.gap - expected_ring6).abs() < 1e-9,
        "sector-0 λ₁: got {}, expected {} (ring6 conn.)",
        result.gap,
        expected_ring6
    );
    assert_eq!(result.n_records_used, 6, "only sector-0 edges must survive");
}

// ────────────────────────────────────────────────────────────────────
// (2) Comparison filter (<) filters correctly.
// ────────────────────────────────────────────────────────────────────
/// Sectors 0, 1, 2. `WHERE q_rounded < 1` keeps only sector 0.
/// Same expected gap and record count as test (1).
#[test]
fn test_spectral_gauge_where_comparison_filters_correctly() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_sector_bundle(&mut engine, "cmp_sectors");
    let id = identity_fiber();

    // Sector 0: 6-ring.
    for i in 0..6i64 {
        let j = (i + 1) % 6;
        insert_sector_edge(&mut engine, "cmp_sectors", i, j, 0, &id);
    }
    // Sector 1: 4-ring on 100..103.
    for i in 0..4i64 {
        let j = (i + 1) % 4;
        insert_sector_edge(&mut engine, "cmp_sectors", 100 + i, 100 + j, 1, &id);
    }
    // Sector 2: 3-ring on 200..202.
    for i in 0..3i64 {
        let j = (i + 1) % 3;
        insert_sector_edge(&mut engine, "cmp_sectors", 200 + i, 200 + j, 2, &id);
    }

    let fiber: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let filter = vec![gigi::bundle::QueryCondition::Lt(
        "q_rounded".to_string(),
        gigi::types::Value::Integer(1),
    )];
    let result = spectral_gauge_gap(
        &engine,
        "cmp_sectors",
        &fiber,
        Group::SU2,
        false,
        None,
        Some(&filter),
    )
    .expect("< 1 filter should retain sector 0 only");

    let expected_ring6 = 2.0 * (1.0 - (std::f64::consts::PI / 3.0).cos());
    assert!(
        (result.gap - expected_ring6).abs() < 1e-9,
        "< 1 λ₁ mismatch: got {}, expected {}",
        result.gap,
        expected_ring6
    );
    assert_eq!(result.n_records_used, 6);
}

// ────────────────────────────────────────────────────────────────────
// (3) Missing / non-matching field yields typed EmptySubgraph error.
// ────────────────────────────────────────────────────────────────────
/// A WHERE clause referencing a nonexistent field filters every
/// record out (QueryCondition::matches returns false for missing
/// fields). The kernel must surface this as SpectralGaugeError::
/// EmptySubgraph so callers can distinguish "filter matched nothing"
/// from "bundle is empty".
#[test]
fn test_spectral_gauge_where_missing_field_errors_clearly() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_sector_bundle(&mut engine, "missing_field");
    let id = identity_fiber();
    for i in 0..4i64 {
        let j = (i + 1) % 4;
        insert_sector_edge(&mut engine, "missing_field", i, j, 0, &id);
    }

    let fiber: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let filter = vec![gigi::bundle::QueryCondition::Eq(
        "no_such_field".to_string(),
        gigi::types::Value::Integer(42),
    )];
    let err = spectral_gauge_gap(
        &engine,
        "missing_field",
        &fiber,
        Group::SU2,
        false,
        None,
        Some(&filter),
    )
    .expect_err("filter on missing field should surface EmptySubgraph");

    match err {
        SpectralGaugeError::EmptySubgraph { ref where_clause, ref message } => {
            assert!(
                where_clause.contains("no_such_field"),
                "where_clause should name the field: {where_clause}"
            );
            let _ = message;
        }
        other => panic!("expected EmptySubgraph, got {other:?}"),
    }
    let msg = err.to_string();
    assert!(
        msg.to_lowercase().contains("filter")
            || msg.to_lowercase().contains("empty")
            || msg.to_lowercase().contains("subgraph"),
        "message should reference filter/empty/subgraph: {msg}"
    );
}

// ────────────────────────────────────────────────────────────────────
// (4) No WHERE clause preserves existing behaviour (backwards compat).
// ────────────────────────────────────────────────────────────────────
/// Calling with `filter = None` on a plain ring must return exactly
/// the same gap as the pre-WHERE spectral_gauge_gap signature would
/// have — this pins the backwards-compat guarantee for every existing
/// caller (halcyon_part_iv_gold, chern_class_basic delegation,
/// spectral_gauge_basic).
#[test]
fn test_spectral_gauge_no_where_still_works() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_sector_bundle(&mut engine, "no_where");
    let id = identity_fiber();
    let n = 6i64;
    for i in 0..n {
        let j = (i + 1) % n;
        insert_sector_edge(&mut engine, "no_where", i, j, 0, &id);
    }

    let fiber: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let result = spectral_gauge_gap(
        &engine,
        "no_where",
        &fiber,
        Group::SU2,
        false,
        None,
        None,
    )
    .expect("plain ring should give a gap");

    let expected = 2.0 * (1.0 - (std::f64::consts::PI / 3.0).cos());
    assert!(
        (result.gap - expected).abs() < 1e-9,
        "no-filter ring6 λ₁ mismatch: got {} expected {}",
        result.gap,
        expected
    );
    assert_eq!(result.n_records_used, n as usize);
}

// ────────────────────────────────────────────────────────────────────
// (5) Filter produces a strictly smaller subgraph than unfiltered.
// ────────────────────────────────────────────────────────────────────
/// With sectors 0 (6 edges) and 1 (4 edges), the unfiltered call sees
/// 10 records; filtering to sector 0 sees exactly 6. This asserts the
/// filter is applied at the record level, not the vertex level, so
/// n_records_used < unfiltered count and both graphs give distinct
/// gaps (ring6 vs ring6+ring4 as two components — the second gap can
/// be zero or small; the essential assertion is the record count).
#[test]
fn test_spectral_gauge_where_produces_smaller_subgraph() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_sector_bundle(&mut engine, "smaller_sub");
    let id = identity_fiber();

    for i in 0..6i64 {
        let j = (i + 1) % 6;
        insert_sector_edge(&mut engine, "smaller_sub", i, j, 0, &id);
    }
    for i in 0..4i64 {
        let j = (i + 1) % 4;
        insert_sector_edge(&mut engine, "smaller_sub", 100 + i, 100 + j, 1, &id);
    }

    let fiber: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();

    // Unfiltered: sees all 10 records.
    let unfiltered = spectral_gauge_gap(
        &engine,
        "smaller_sub",
        &fiber,
        Group::SU2,
        false,
        None,
        None,
    )
    .expect("unfiltered call should succeed");
    assert_eq!(unfiltered.n_records_used, 10);

    // Filtered to sector 0: 6 records.
    let filter = vec![gigi::bundle::QueryCondition::Eq(
        "q_rounded".to_string(),
        gigi::types::Value::Integer(0),
    )];
    let filtered = spectral_gauge_gap(
        &engine,
        "smaller_sub",
        &fiber,
        Group::SU2,
        false,
        None,
        Some(&filter),
    )
    .expect("sector-0 filter should succeed");
    assert_eq!(filtered.n_records_used, 6);
    assert!(
        filtered.n_records_used < unfiltered.n_records_used,
        "filter must strictly shrink the record set: {} vs {}",
        filtered.n_records_used,
        unfiltered.n_records_used
    );
}

// ────────────────────────────────────────────────────────────────────
// (6) AND-combinator filter narrows across two predicates.
// ────────────────────────────────────────────────────────────────────
/// Records tagged (q_rounded, mu-like second column) — with two
/// clauses `q_rounded = 0 AND vertex_a >= 3` we retain only sector-0
/// edges whose vertex_a is at least 3, i.e. edges (3,4), (4,5), (5,0).
/// Assert record count == 3 (kernel-level AND semantics).
#[test]
fn test_spectral_gauge_where_and_combinator_filters_correctly() {
    let mut engine = Engine::open_memory().expect("memory engine");
    make_sector_bundle(&mut engine, "and_combo");
    let id = identity_fiber();

    // Sector 0: 6-ring on 0..5.
    for i in 0..6i64 {
        let j = (i + 1) % 6;
        insert_sector_edge(&mut engine, "and_combo", i, j, 0, &id);
    }
    // Sector 1: 4-ring on 100..103.
    for i in 0..4i64 {
        let j = (i + 1) % 4;
        insert_sector_edge(&mut engine, "and_combo", 100 + i, 100 + j, 1, &id);
    }

    let fiber: Vec<String> = su2_field_names().iter().map(|s| s.to_string()).collect();
    let filter = vec![
        gigi::bundle::QueryCondition::Eq(
            "q_rounded".to_string(),
            gigi::types::Value::Integer(0),
        ),
        gigi::bundle::QueryCondition::Gte(
            "vertex_a".to_string(),
            gigi::types::Value::Integer(3),
        ),
    ];
    let result = spectral_gauge_gap(
        &engine,
        "and_combo",
        &fiber,
        Group::SU2,
        false,
        None,
        Some(&filter),
    )
    .expect("AND filter should give a gap on the 3-edge subgraph");

    // Sector-0 edges with vertex_a >= 3: (3,4), (4,5), (5,0). 3 records.
    assert_eq!(
        result.n_records_used, 3,
        "AND filter should retain 3 edges, got {}",
        result.n_records_used
    );
}

// ────────────────────────────────────────────────────────────────────
// (7) Parser: SPECTRAL_GAUGE WHERE q_rounded = 0 ... matches Hallie's
//     Halcyon sectoral workflow verbatim.
// ────────────────────────────────────────────────────────────────────
/// This is the acceptance test for the grammar Hallie will actually
/// type. Must produce Statement::SpectralGauge with a single
/// FilterCondition::Eq("q_rounded", Integer(0)) in where_conditions.
#[test]
fn test_spectral_gauge_where_q_rounded_sector_matches_halcyon_workflow() {
    let stmt = parse(
        "SPECTRAL_GAUGE su2_L24_seed702 WHERE q_rounded = 0 ON FIBER (q0, q1, q2, q3) GROUP SU(2)",
    )
    .expect("Halcyon sectoral SPECTRAL_GAUGE must parse");

    match stmt {
        Statement::SpectralGauge {
            bundle,
            fiber_fields,
            group,
            full,
            limit,
            magnetic,
            where_conditions,
        } => {
            assert_eq!(bundle, "su2_L24_seed702");
            assert_eq!(fiber_fields, vec!["q0", "q1", "q2", "q3"]);
            assert_eq!(group, Some(Group::SU2));
            assert!(!full);
            assert_eq!(limit, None);
            assert!(!magnetic, "no MODE clause parses with magnetic = false");
            assert_eq!(where_conditions.len(), 1, "one WHERE predicate expected");
            match where_conditions[0].clone() {
                FilterCondition::Eq(field, Literal::Integer(v)) => {
                    assert_eq!(field, "q_rounded");
                    assert_eq!(v, 0);
                }
                other => panic!("expected Eq(q_rounded, Integer(0)), got {other:?}"),
            }
        }
        other => panic!("expected SpectralGauge, got {other:?}"),
    }
}
