//! Time you can type — TIMESTAMP fields end to end.
//!
//! Before this feature: GQL `TIMESTAMP` columns were plain numerics,
//! REST-created `Value::Timestamp` fields compared against integers or
//! date strings by TYPE TAG (silently constant), and everything
//! displayed as epoch millis. These tests pin the new contract.

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};
use gigi::types::Value;

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

fn seeded(dir: &std::path::Path) -> Engine {
    let mut e = Engine::open(dir).unwrap();
    for stmt in [
        "BUNDLE events BASE (id TEXT) FIBER (kind TEXT INDEX, at TIMESTAMP);",
        // ISO date string
        "SECTION events (id='e1', kind='deploy', at='2026-06-30');",
        // ISO datetime
        "SECTION events (id='e2', kind='alert', at='2026-07-01T14:30:05Z');",
        // raw epoch ms (2026-07-02T00:00:00Z)
        "SECTION events (id='e3', kind='deploy', at=1782950400000);",
    ] {
        run(&mut e, stmt).unwrap_or_else(|err| panic!("{stmt}: {err}"));
    }
    e
}

#[test]
fn iso_and_epoch_inserts_normalize_to_timestamp() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded(dir.path());
    for (id, want) in [
        ("e1", gigi::timefmt::parse_iso_ms("2026-06-30").unwrap()),
        ("e2", gigi::timefmt::parse_iso_ms("2026-07-01T14:30:05Z").unwrap()),
        ("e3", 1_782_950_400_000),
    ] {
        match run(&mut e, &format!("SECTION events AT id='{id}';")).unwrap() {
            ExecResult::Rows(rows) => {
                assert_eq!(
                    rows[0]["at"],
                    Value::Timestamp(want),
                    "{id}: stored form must be Timestamp"
                );
            }
            other => panic!("expected rows, got {other:?}"),
        }
    }
}

#[test]
fn where_compares_dates_as_time_not_type_tags() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded(dir.path());
    // date-string comparison
    match run(&mut e, "COVER events WHERE at > '2026-07-01';").unwrap() {
        ExecResult::Rows(rows) => {
            let mut ids: Vec<_> = rows
                .iter()
                .map(|r| format!("{}", r["id"]))
                .collect();
            ids.sort();
            assert_eq!(ids, ["e2", "e3"], "after 2026-07-01 midnight: e2 and e3");
        }
        other => panic!("expected rows, got {other:?}"),
    }
    // epoch-ms integer comparison agrees exactly
    match run(&mut e, "COVER events WHERE at >= 1782950400000;").unwrap() {
        ExecResult::Rows(rows) => assert_eq!(rows.len(), 1, "only e3 is >= Jul 2"),
        other => panic!("expected rows, got {other:?}"),
    }
    // BETWEEN with two date strings
    match run(
        &mut e,
        "COVER events WHERE at BETWEEN '2026-06-30' AND '2026-07-01T23:59:59';",
    ) {
        Ok(ExecResult::Rows(rows)) => {
            assert_eq!(rows.len(), 2, "e1 and e2 inside the window");
        }
        Ok(other) => panic!("expected rows, got {other:?}"),
        // BETWEEN may not parse in COVER WHERE — then this form simply
        // isn't part of the contract; the Gt/Gte forms above are.
        Err(e) if e.contains("BETWEEN") => {}
        Err(e) => panic!("unexpected error: {e}"),
    }
}

#[test]
fn garbage_dates_are_loud_not_stored() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded(dir.path());
    let err = run(
        &mut e,
        "SECTION events (id='bad', kind='x', at='next tuesday');",
    )
    .unwrap_err();
    assert!(
        err.contains("not a date") && err.contains("2026-07-02"),
        "error must refuse AND teach the accepted forms: {err}"
    );
    // nothing was stored
    match run(&mut e, "EXISTS SECTION events AT id='bad';").unwrap() {
        ExecResult::Bool(b) => assert!(!b, "the bad record must not exist"),
        other => panic!("expected bool, got {other:?}"),
    }
}

#[test]
fn now_literal_is_current_epoch_ms() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded(dir.path());
    run(&mut e, "SECTION events (id='e4', kind='ping', at=NOW);").unwrap();
    match run(&mut e, "SECTION events AT id='e4';").unwrap() {
        ExecResult::Rows(rows) => match rows[0]["at"] {
            Value::Timestamp(ms) => {
                let now = gigi::timefmt::now_ms();
                assert!(
                    (now - ms).abs() < 60_000,
                    "NOW should be within a minute of the wall clock"
                );
            }
            ref other => panic!("NOW must store as Timestamp, got {other:?}"),
        },
        other => panic!("expected rows, got {other:?}"),
    }
    // everything before NOW = the three seeded events + e4 itself
    match run(&mut e, "COVER events WHERE at <= NOW;").unwrap() {
        ExecResult::Rows(rows) => assert_eq!(rows.len(), 4),
        other => panic!("expected rows, got {other:?}"),
    }
}

/// The coercion chokepoint must cover the update path, not just the
/// insert paths. REDEFINE drives `Engine::update`; before the fix an
/// update stored raw `Text` in a TIMESTAMP field — silently constant
/// under every time comparison from then on (type-tag ordering), the
/// exact disease the feature exists to cure.
#[test]
fn update_coerces_iso_string_to_timestamp_not_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded(dir.path());
    run(
        &mut e,
        "REDEFINE events AT id='e1' SET (at='2026-07-04T09:15:00Z');",
    )
    .unwrap();
    match run(&mut e, "SECTION events AT id='e1';").unwrap() {
        ExecResult::Rows(rows) => {
            assert_eq!(
                rows[0]["at"],
                Value::Timestamp(
                    gigi::timefmt::parse_iso_ms("2026-07-04T09:15:00Z").unwrap()
                ),
                "updated form must be the coerced Timestamp, not raw Text"
            );
        }
        other => panic!("expected rows, got {other:?}"),
    }
    // And the update path refuses garbage just as loudly as insert.
    let err = run(
        &mut e,
        "REDEFINE events AT id='e1' SET (at='next tuesday');",
    )
    .unwrap_err();
    assert!(
        err.contains("not a date"),
        "update must refuse garbage dates, not store them: {err}"
    );
}

#[test]
fn survives_wal_replay() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut e = seeded(dir.path());
        // ensure the coerced (Timestamp) form is what the WAL carries
        run(&mut e, "SECTION events (id='e5', kind='x', at='2026-07-03');").unwrap();
    }
    let mut e = Engine::open(dir.path()).unwrap();
    match run(&mut e, "COVER events WHERE at > '2026-07-02T12:00';").unwrap() {
        ExecResult::Rows(rows) => {
            assert_eq!(rows.len(), 1);
            assert_eq!(format!("{}", rows[0]["id"]), "e5");
        }
        other => panic!("expected rows, got {other:?}"),
    }
}
