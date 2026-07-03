//! EMIT CSV TO — the export half of the CSV round trip.
//!
//! Contract: EMIT executes only when GIGI_EMIT_DIR is set, and paths
//! resolve strictly inside that directory (relative, no '..'). All
//! phases live in ONE test because the gate is a process-global env
//! var and cargo runs tests in threads.

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

fn seeded(dir: &std::path::Path) -> Engine {
    let mut e = Engine::open(dir).unwrap();
    for stmt in [
        "BUNDLE sensors BASE (id TEXT) FIBER (city TEXT INDEX, temp NUMERIC);",
        "SECTION sensors (id='s1', city='Moscow', temp=-3.0);",
        "SECTION sensors (id='s2', city='Moscow', temp=-25.5);",
        "SECTION sensors (id='s3', city='Lagos, Island', temp=31.0);",
        "SECTION sensors (id='s4', city='Lagos', temp=29.5);",
    ] {
        run(&mut e, stmt).unwrap();
    }
    e
}

#[test]
fn emit_csv_gate_export_and_traversal() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = seeded(dir.path());

    // Phase 1 — gate closed: refused loudly, names the knob.
    std::env::remove_var("GIGI_EMIT_DIR");
    let err = run(&mut e, "COVER sensors ALL EMIT CSV TO 'rows.csv';").unwrap_err();
    assert!(err.contains("GIGI_EMIT_DIR"), "gate error must name the knob: {err}");

    // Phase 2 — gate open: rows land as CSV, quoted comma survives.
    let out = tempfile::tempdir().unwrap();
    std::env::set_var("GIGI_EMIT_DIR", out.path());
    match run(&mut e, "COVER sensors ALL EMIT CSV TO 'exports/rows.csv';").unwrap() {
        ExecResult::Notice(msg) => assert!(msg.contains("wrote 4 rows"), "{msg}"),
        other => panic!("expected Notice receipt, got {other:?}"),
    }
    let csv = std::fs::read_to_string(out.path().join("exports/rows.csv")).unwrap();
    let mut lines = csv.lines();
    assert_eq!(lines.next().unwrap(), "city,id,temp", "sorted header");
    assert_eq!(csv.lines().count(), 5, "header + 4 rows");
    assert!(
        csv.contains("\"Lagos, Island\""),
        "embedded comma must be quoted: {csv}"
    );

    // Aggregations export too.
    match run(
        &mut e,
        "INTEGRATE sensors OVER city MEASURE count(*), avg(temp) EMIT CSV TO 'agg.csv';",
    )
    .unwrap()
    {
        ExecResult::Notice(msg) => assert!(msg.contains("wrote 3 rows"), "{msg}"),
        other => panic!("expected Notice, got {other:?}"),
    }

    // Phase 3 — traversal and absolute paths are refused.
    let err = run(&mut e, "COVER sensors ALL EMIT CSV TO '../escape.csv';").unwrap_err();
    assert!(err.contains("relative"), "{err}");
    // The fixture must be absolute ON THE RUNNING PLATFORM: '/tmp/x'
    // has no drive prefix, so Windows' Path::is_absolute() says false,
    // the refusal never fires, and join() escapes the emit dir — the
    // phase was vacuous-and-escaping on Windows. (emit_target's
    // treatment of rootless/drive-relative paths on Windows is a
    // separate, deferred finding; prod is Linux.)
    let abs = if cfg!(windows) { "C:/tmp/abs.csv" } else { "/tmp/abs.csv" };
    let err = run(&mut e, &format!("COVER sensors ALL EMIT CSV TO '{abs}';")).unwrap_err();
    assert!(err.contains("relative"), "{err}");

    // Phase 4 — non-rows statements refuse EMIT with an explanation.
    let err = run(&mut e, "HEALTH sensors EMIT CSV TO 'h.csv';").unwrap_err();
    assert!(err.contains("returns rows"), "{err}");

    // Phase 5 — only CSV is a valid format, at parse time.
    let err = parser::parse("COVER sensors ALL EMIT PARQUET TO 'x.parquet';").unwrap_err();
    assert!(err.contains("CSV"), "{err}");

    std::env::remove_var("GIGI_EMIT_DIR");
}
