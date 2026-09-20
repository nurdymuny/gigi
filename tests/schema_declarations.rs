//! What a bundle declares about itself, and what the engine does about it.
//!
//! HELICITY engineering asked for five declarations on every schema: units, an
//! order field, base/fibre placement, encryption mode and retention. Placement
//! and encryption mode already existed. These are the other three, plus the row
//! semantics they asked for separately so CADENCE refuses on discrete events
//! rather than coalescing them.
//!
//! The split is deliberate. `retention` and `unit` are carried and never acted
//! on: enforcing retention in the engine would be a second, implicit deletion
//! path that a tenant-scoped deletion audit does not know about, and it would
//! age out correction history a reconciling account is obliged to keep.
//! `order_field` and `row_semantics` do change what a verb does, and those
//! changes are what the rest of this file pins.
use gigi::engine::Engine;
use gigi::types::{
    BundleSchema, EncryptionMode, FieldDef, Record, Retention, RowSemantics, Value,
};
use std::fs;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gigi_decl_{tag}"));
    let _ = fs::remove_dir_all(&d);
    d
}

/// A bundle whose base key is TEXT, so it is hash-stored and does not iterate
/// in insertion order. That is the case where record order means nothing.
fn hashed_schema() -> BundleSchema {
    BundleSchema::new("events")
        .base(FieldDef::categorical("id"))
        .fiber(FieldDef::numeric("seq"))
        .fiber(FieldDef::numeric("price").with_unit("USD"))
}

fn row(id: &str, seq: f64, price: f64) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Text(id.into()));
    r.insert("seq".into(), Value::Float(seq));
    r.insert("price".into(), Value::Float(price));
    r
}

fn seeded(d: &std::path::PathBuf, schema: BundleSchema, n: usize) -> Engine {
    let mut e = Engine::open(d).unwrap();
    e.create_bundle(schema).unwrap();
    for i in 0..n {
        let x = i as f64;
        e.insert("events", &row(&format!("k{i:03}"), x, 100.0 + x * 0.5))
            .unwrap();
    }
    e
}

// ---------------------------------------------- the declarations persist

#[test]
fn every_declaration_survives_a_restart() {
    let d = dir("roundtrip");
    {
        let schema = hashed_schema()
            .with_order_field("seq")
            .with_retention(Retention::Days(365))
            .with_row_semantics(RowSemantics::DiscreteEvents);
        let mut e = Engine::open(&d).unwrap();
        e.create_bundle(schema).unwrap();
    }
    let e = Engine::open(&d).unwrap();
    let s = e.bundle_schema("events").expect("replayed").clone();
    drop(e);
    let _ = fs::remove_dir_all(&d);

    assert_eq!(s.order_field.as_deref(), Some("seq"));
    assert_eq!(s.retention, Retention::Days(365));
    assert_eq!(s.row_semantics, RowSemantics::DiscreteEvents);
    let price = s.fiber_fields.iter().find(|f| f.name == "price").unwrap();
    assert_eq!(price.unit.as_deref(), Some("USD"));
}

/// "Nobody said" and "we decided to keep it" are different answers to an
/// auditor, so they are different values.
#[test]
fn undeclared_retention_is_not_indefinite() {
    assert_ne!(Retention::Undeclared, Retention::Indefinite);
    assert_eq!(BundleSchema::new("x").retention, Retention::Undeclared);
}

// ---------------------------------------------- the order field is used

#[test]
fn a_declared_order_field_is_used_without_being_named_on_the_call() {
    let d = dir("default");
    let e = seeded(&d, hashed_schema().with_order_field("seq"), 80);
    let got = gigi::ml::texture::texture(&e, "events", "price", None, 2.0, 1, None);
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let r = got.expect("a declared order field should make this answerable");
    assert_eq!(
        r.order_field.as_deref(),
        Some("seq"),
        "the verb should report the order it actually used"
    );
}

#[test]
fn without_a_declaration_or_an_override_a_hash_stored_bundle_refuses() {
    let d = dir("norder");
    let e = seeded(&d, hashed_schema(), 80);
    let got = gigi::ml::texture::texture(&e, "events", "price", None, 2.0, 1, None);
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let (_, msg) = got.expect_err("record order means nothing here");
    assert!(
        msg.contains("do not iterate in insertion order"),
        "got: {msg}"
    );
}

#[test]
fn an_override_naming_an_undeclared_field_refuses_rather_than_falling_back() {
    let d = dir("badoverride");
    let e = seeded(&d, hashed_schema().with_order_field("seq"), 80);
    let got = gigi::ml::texture::texture(
        &e,
        "events",
        "price",
        Some("nope".into()),
        2.0,
        1,
        None,
    );
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let (_, msg) = got.expect_err("an override that does not exist is a refusal");
    assert!(
        msg.contains("nope"),
        "the refusal must name the field; got: {msg}"
    );
}

// ---------------------------------------------- ties

#[test]
fn an_order_field_that_does_not_order_the_rows_refuses() {
    let d = dir("ties");
    let mut e = Engine::open(&d).unwrap();
    e.create_bundle(hashed_schema().with_order_field("seq"))
        .unwrap();
    // Two rows sharing a sequence value: the shape of the fixture HELICITY
    // described, where the wrong order frees capital for an earlier order.
    for (i, seq) in [(0usize, 1.0f64), (1, 1.0), (2, 2.0), (3, 3.0)] {
        e.insert("events", &row(&format!("k{i}"), seq, 100.0 + i as f64))
            .unwrap();
    }
    let got = gigi::ml::texture::texture(&e, "events", "price", None, 2.0, 1, None);
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let (_, msg) = got.expect_err("a tied order field does not order the rows");
    assert!(
        msg.contains("does not order them"),
        "the refusal must say the field fails to order; got: {msg}"
    );
}

// ---------------------------------------------- row semantics

#[test]
fn cadence_refuses_a_bundle_that_declares_discrete_events() {
    let d = dir("discrete");
    let e = seeded(
        &d,
        hashed_schema()
            .with_order_field("seq")
            .with_row_semantics(RowSemantics::DiscreteEvents),
        60,
    );
    let got = gigi::ml::cadence::cadence(&e, "events", "seq", None);
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let (_, msg) = got.expect_err("coalescing two events into one is not an answer");
    assert!(
        msg.contains("discrete events"),
        "the refusal must name the declaration; got: {msg}"
    );
}

// ---------------------------------------------- refused at declaration time

#[test]
fn an_order_field_the_bundle_does_not_define_is_refused_at_creation() {
    let d = dir("ghostorder");
    let mut e = Engine::open(&d).unwrap();
    let got = e.create_bundle(hashed_schema().with_order_field("not_a_field"));
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let msg = got
        .expect_err("cannot order by a field that does not exist")
        .to_string();
    assert!(msg.contains("not_a_field"), "got: {msg}");
}

/// HELICITY's G0 fixture: a schema that asks for protection and is given none,
/// quietly. The grammar accepts an encryption declaration on a BASE field and
/// the field ships plaintext, because base fields are never encrypted.
#[test]
fn encryption_declared_on_a_base_field_is_refused_at_creation() {
    let d = dir("baseenc");
    let mut e = Engine::open(&d).unwrap();
    let schema = BundleSchema::new("events")
        .base(FieldDef::categorical("id").with_encryption(EncryptionMode::Opaque))
        .fiber(FieldDef::numeric("price"));
    let got = e.create_bundle(schema);
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let msg = got
        .expect_err("a declaration the engine ignores is worse than one it rejects")
        .to_string();
    assert!(
        msg.contains("base field") && msg.contains("id"),
        "the refusal must name the field and why; got: {msg}"
    );
}

// ---------------------------------------------- reachable from the language

/// Declarations only reachable from Rust would be useless to anyone writing
/// fixtures, so they are grammar too.
#[test]
fn the_declarations_can_be_written_in_the_query_language() {
    let d = dir("grammar");
    let mut e = Engine::open(&d).unwrap();
    let stmt = "CREATE BUNDLE fills (id TEXT BASE, seq NUMERIC FIBER, px NUMERIC FIBER)                 ORDER BY seq RETENTION 365 DAYS ROWS ARE EVENTS;";
    let ast = gigi::parser::parse(stmt).expect("should parse");
    gigi::parser::execute(&mut e, &ast).expect("should execute");

    let s = e.bundle_schema("fills").expect("created").clone();
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert_eq!(s.order_field.as_deref(), Some("seq"));
    assert_eq!(s.retention, Retention::Days(365));
    assert_eq!(s.row_semantics, RowSemantics::DiscreteEvents);
}

/// Clause order should not matter, and a contradiction should be a parse error
/// rather than last-one-wins.
#[test]
fn declaration_clauses_are_order_free_and_reject_duplicates() {
    let stmt = "CREATE BUNDLE b (id TEXT BASE, seq NUMERIC FIBER)                 ROWS ARE SAMPLES ORDER BY seq RETENTION INDEFINITE;";
    let ast = gigi::parser::parse(stmt).expect("any order should parse");
    match ast {
        gigi::parser::Statement::CreateBundle {
            order_field,
            retention,
            row_semantics,
            ..
        } => {
            assert_eq!(order_field.as_deref(), Some("seq"));
            assert_eq!(retention, Retention::Indefinite);
            assert_eq!(row_semantics, RowSemantics::ClockSamples);
        }
        other => panic!("expected CreateBundle, got {other:?}"),
    }

    let dup = "CREATE BUNDLE b (id TEXT BASE, seq NUMERIC FIBER)                ORDER BY seq ORDER BY id;";
    let err = gigi::parser::parse(dup).expect_err("a schema cannot declare two orders");
    assert!(err.contains("twice"), "got: {err}");
}

/// A unit is declarable in the language too, for the same reason the others
/// are: a declaration only reachable from Rust is not a declaration a schema
/// author can make.
#[test]
fn a_field_unit_can_be_written_in_the_query_language() {
    let d = dir("unitgrammar");
    let mut e = Engine::open(&d).unwrap();
    let stmt = "CREATE BUNDLE quotes (id TEXT BASE, px NUMERIC FIBER UNIT 'USD', lat NUMERIC FIBER UNIT 'microseconds');";
    let ast = gigi::parser::parse(stmt).expect("should parse");
    gigi::parser::execute(&mut e, &ast).expect("should execute");
    let s = e.bundle_schema("quotes").expect("created").clone();
    drop(e);
    let _ = fs::remove_dir_all(&d);

    let unit_of = |n: &str| {
        s.fiber_fields
            .iter()
            .find(|f| f.name == n)
            .and_then(|f| f.unit.clone())
    };
    assert_eq!(unit_of("px").as_deref(), Some("USD"));
    assert_eq!(unit_of("lat").as_deref(), Some("microseconds"));
}

/// A schema that trips both creation rules must report the structural one.
///
/// HELICITY engineering found this from the other side: their G0 fixture
/// accepted any refusal as proof the base-field rule worked, so once the seed
/// rule existed it would have passed for either reason and reported G0
/// satisfied when G0 was untested. The two rules are now ordered so the
/// structural fault wins, and this pins that.
#[test]
fn a_schema_tripping_both_creation_rules_reports_the_structural_one() {
    let d = dir("bothrules");
    let mut e = Engine::open(&d).unwrap();
    // Base field declares encryption (structural fault) AND a fiber field is
    // encrypted under a random seed (policy fault). Both would refuse.
    let schema = BundleSchema::new("events")
        .base(FieldDef::categorical("id").with_encryption(EncryptionMode::Opaque))
        .fiber(FieldDef::numeric("px").with_encryption(EncryptionMode::Opaque));
    let got = e.create_bundle(schema);
    drop(e);
    let _ = fs::remove_dir_all(&d);
    let msg = got.expect_err("both rules refuse this").to_string();
    assert!(
        msg.contains("base field"),
        "the structural fault must be reported, not the seed policy; got: {msg}"
    );
    assert!(
        !msg.contains("SEED FROM ENV"),
        "reporting the seed rule here makes a base-field fixture ambiguous; got: {msg}"
    );
}

/// There is no fixed environment variable name: the schema author chooses it.
///
/// HELICITY asked us to name "the" variable. There is not one, which is better
/// for them than an answer would have been.
#[test]
fn the_seed_variable_name_is_chosen_by_the_schema() {
    std::env::set_var("ANY_NAME_WE_LIKE_9931", "07".repeat(32));
    let d = dir("seedname");
    let mut e = Engine::open(&d).unwrap();
    let stmt = "CREATE BUNDLE vault (id TEXT BASE, px NUMERIC FIBER ENCRYPTED OPAQUE)                 WITH ENCRYPTION SEED FROM ENV ANY_NAME_WE_LIKE_9931;";
    let ast = gigi::parser::parse(stmt).expect("should parse");
    let out = gigi::parser::execute(&mut e, &ast);
    let created = e.bundle_schema("vault").is_some();
    drop(e);
    let _ = fs::remove_dir_all(&d);
    assert!(out.is_ok(), "an author-chosen variable must work: {out:?}");
    assert!(created, "bundle should exist");
}
