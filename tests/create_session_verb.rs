//! CREATE SESSION first-class verb (personal-list #2, 2026-06-22).
//!
//! Stamps the canonical session-bundle schema in ONE place so consumers
//! (marcella_persistent_memory, claude_substrate_v0, every future fiber-
//! LM session bundle) stop reinventing the (thought_id, ts, session,
//! topic, content, refs) shape by hand.
//!
//! Behaviors verified:
//!
//!   1. CREATE SESSION <name>; emits a bundle with the canonical 6-field
//!      schema: thought_id BASE TEXT + ts FIBER TIMESTAMP + session/
//!      topic/content/refs FIBER TEXT.
//!   2. WITH SCHEMA (extra FIBER TYPE [INDEX], ...) appends extra fields
//!      AFTER the canonical 5 fibers; INDEX modifiers register.
//!   3. INSERT INTO <session> works against the freshly-stamped schema
//!      (thought rows land + read back).
//!   4. COVER <session> RANK BY thought_id returns rows in lexicographic
//!      thought_id order (cold-start protocol shape).
//!   5. Extras that collide with a canonical name are rejected at parse
//!      time with a clear error.
//!   6. BASE-typed extras are rejected (extras must be FIBER).

use gigi::engine::Engine;
use gigi::parser::{execute, parse, ExecResult, Statement};
use gigi::types::{FieldType, Value};

fn fresh_engine() -> (Engine, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path()).expect("engine open");
    (engine, dir)
}

#[test]
fn test_create_session_canonical_schema() {
    // GQL parses to Statement::CreateSession with no extras.
    let stmt = parse("CREATE SESSION my_thoughts;").expect("parse CREATE SESSION");
    match &stmt {
        Statement::CreateSession {
            session_name,
            extra_schema,
        } => {
            assert_eq!(session_name, "my_thoughts");
            assert!(extra_schema.is_none(), "no WITH SCHEMA -> None");
        }
        other => panic!("expected CreateSession, got {other:?}"),
    }

    // Executor stamps the canonical 6-field schema on a real engine.
    let (mut engine, _dir) = fresh_engine();
    let result = execute(&mut engine, &stmt).expect("execute CREATE SESSION");
    assert_eq!(result, ExecResult::Ok);

    let bundle = engine.bundle("my_thoughts").expect("bundle exists");
    let schema = bundle.schema();
    // BASE: thought_id, categorical (TEXT).
    assert_eq!(schema.base_fields.len(), 1);
    assert_eq!(schema.base_fields[0].name, "thought_id");
    assert_eq!(schema.base_fields[0].field_type, FieldType::Categorical);

    // FIBER: ts (Timestamp), session/topic/content/refs (Categorical),
    // in canonical order.
    assert_eq!(schema.fiber_fields.len(), 5);
    let names: Vec<&str> = schema.fiber_fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, vec!["ts", "session", "topic", "content", "refs"]);
    assert_eq!(schema.fiber_fields[0].field_type, FieldType::Timestamp);
    for f in &schema.fiber_fields[1..] {
        assert_eq!(
            f.field_type,
            FieldType::Categorical,
            "{} should be categorical",
            f.name
        );
    }

    // Default indices: ts (for ORDER BY) + topic (for filter).
    assert!(
        schema.indexed_fields.iter().any(|n| n == "ts"),
        "ts indexed"
    );
    assert!(
        schema.indexed_fields.iter().any(|n| n == "topic"),
        "topic indexed"
    );
}

#[test]
fn test_create_session_with_extra_schema() {
    let gql = "CREATE SESSION marcella_mem \
        WITH SCHEMA (embedding FIBER VECTOR INDEX, confidence FIBER FLOAT);";
    let stmt = parse(gql).expect("parse CREATE SESSION WITH SCHEMA");

    match &stmt {
        Statement::CreateSession {
            session_name,
            extra_schema,
        } => {
            assert_eq!(session_name, "marcella_mem");
            let extras = extra_schema.as_ref().expect("extras present");
            assert_eq!(extras.len(), 2);
            assert_eq!(extras[0].0, "embedding");
            assert!(extras[0].2, "embedding INDEX");
            assert_eq!(extras[1].0, "confidence");
            assert!(!extras[1].2, "confidence not indexed");
        }
        other => panic!("expected CreateSession, got {other:?}"),
    }

    let (mut engine, _dir) = fresh_engine();
    execute(&mut engine, &stmt).expect("execute");
    let bundle = engine.bundle("marcella_mem").expect("bundle exists");
    let schema = bundle.schema();

    // 5 canonical + 2 extras, extras AFTER the canonical fibers.
    assert_eq!(schema.fiber_fields.len(), 7);
    assert_eq!(schema.fiber_fields[5].name, "embedding");
    assert_eq!(schema.fiber_fields[6].name, "confidence");

    // INDEX modifier on `embedding` registered.
    assert!(
        schema.indexed_fields.iter().any(|n| n == "embedding"),
        "embedding INDEX modifier propagated"
    );
    // `confidence` has no INDEX modifier.
    assert!(
        !schema.indexed_fields.iter().any(|n| n == "confidence"),
        "confidence not indexed"
    );
}

#[test]
fn test_insert_into_session_works() {
    let (mut engine, _dir) = fresh_engine();
    execute(
        &mut engine,
        &parse("CREATE SESSION s;").expect("parse"),
    )
    .expect("create session");

    let insert = "INSERT INTO s (thought_id, ts, session, topic, content, refs) \
        VALUES ('01HXY001', 1718956800000, 'design', 'wish_bundle', \
                'Connection IS load-bearing', '');";
    execute(&mut engine, &parse(insert).expect("parse INSERT")).expect("insert thought");

    let bundle = engine.bundle("s").expect("bundle exists");
    assert_eq!(bundle.len(), 1, "one thought landed");
}

#[test]
fn test_cover_all_order_by_thought_id_works() {
    let (mut engine, _dir) = fresh_engine();
    execute(
        &mut engine,
        &parse("CREATE SESSION s;").expect("parse"),
    )
    .expect("create session");

    // Insert thoughts out-of-order. Cold-start protocol's invariant is
    // that lexicographic thought_id order = wall-clock order (UUIDv7 /
    // monotonic id schemes).
    let inserts = [
        ("01HXY003", 1718956800003i64, "third"),
        ("01HXY001", 1718956800001i64, "first"),
        ("01HXY002", 1718956800002i64, "second"),
    ];
    for (tid, ts, content) in &inserts {
        let gql = format!(
            "INSERT INTO s (thought_id, ts, session, topic, content, refs) \
             VALUES ('{tid}', {ts}, 'design', 'topic_a', '{content}', '');"
        );
        execute(&mut engine, &parse(&gql).expect("parse INSERT"))
            .expect("insert thought");
    }

    // COVER ALL RANK BY thought_id returns rows ascending — the cold-
    // start protocol's shape (RANK BY is COVER's ORDER BY surface).
    let rows = match execute(
        &mut engine,
        &parse("COVER s ALL RANK BY thought_id;").expect("parse COVER"),
    )
    .expect("execute COVER")
    {
        ExecResult::Rows(r) => r,
        other => panic!("expected Rows, got {other:?}"),
    };

    assert_eq!(rows.len(), 3, "all three thoughts returned");
    // Extract thought_ids in returned order. thought_id is a BASE field,
    // so it lives on the record's base section.
    let ids: Vec<String> = rows
        .iter()
        .map(|r| match r.get("thought_id") {
            Some(Value::Text(s)) => s.clone(),
            other => panic!("thought_id field missing/wrong type: {other:?}"),
        })
        .collect();

    assert_eq!(
        ids,
        vec![
            "01HXY001".to_string(),
            "01HXY002".to_string(),
            "01HXY003".to_string(),
        ],
        "rows in ascending thought_id order (cold-start protocol)"
    );
}

#[test]
fn test_create_session_collision_rejected() {
    // `content` collides with a canonical fiber name.
    let err = parse("CREATE SESSION bad WITH SCHEMA (content FIBER TEXT);")
        .expect_err("expected parse error");
    assert!(
        err.contains("collides with canonical session schema"),
        "error message mentions collision; got: {err}"
    );

    // `thought_id` (the BASE key) is also locked.
    let err = parse("CREATE SESSION bad WITH SCHEMA (thought_id FIBER TEXT);")
        .expect_err("expected parse error");
    assert!(
        err.contains("collides with canonical session schema"),
        "thought_id collision rejected; got: {err}"
    );
}

#[test]
fn test_create_session_base_extra_rejected() {
    // Extras must be FIBER — BASE is locked to thought_id.
    let err = parse("CREATE SESSION bad WITH SCHEMA (extra BASE TEXT);")
        .expect_err("expected parse error");
    assert!(
        err.contains("must be") && err.contains("FIBER"),
        "BASE-typed extra rejected; got: {err}"
    );
}
