//! INGEST source gate — GIGI_INGEST_DIR fail-closed containment.
//!
//! Server-side file reads triggered by statements are constrained to an
//! explicit allowlisted root (cf. Postgres pg_read_server_files, MySQL
//! secure_file_priv): unset ⇒ INGEST from files is disabled; set ⇒
//! source paths are relative to the root, component-screened, and
//! canonically verified via `gigi::pathguard::contain`.
//!
//! Contract change pinned here (2026-07-03 hardening): an absolute
//! source like '/tmp/nonexistent.npz' now returns the CONTAINMENT
//! error — before any filesystem access — instead of file-not-found.
//!
//! All phases live in ONE test because the gate is a process-global
//! env var and cargo runs tests in threads (same pattern as
//! tests/emit_csv.rs).

use gigi::engine::Engine;
use gigi::parser::{self, ExecResult};

fn run(e: &mut Engine, stmt: &str) -> Result<ExecResult, String> {
    let ast = parser::parse(stmt)?;
    parser::execute(e, &ast)
}

#[test]
fn ingest_dir_gate_unset_containment_traversal_and_links() {
    let dir = tempfile::tempdir().unwrap();
    let mut e = Engine::open(dir.path()).unwrap();

    // The allowlisted root, with a real CSV inside it…
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("inside")).unwrap();
    std::fs::write(root.path().join("inside/rows.csv"), "id,v\nr1,1\nr2,2\n").unwrap();
    // …and a sibling directory OUTSIDE it holding a file that must
    // never be readable through INGEST.
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("leak.csv"), "id,v\nx,9\n").unwrap();

    // Phase 1 — gate closed: refused loudly, names the knob, reads nothing.
    std::env::remove_var("GIGI_INGEST_DIR");
    let err = run(&mut e, "INGEST g1 FROM 'inside/rows.csv' FORMAT CSV;").unwrap_err();
    assert!(
        err.contains("GIGI_INGEST_DIR"),
        "gate error must name the env var: {err}"
    );

    // Phase 2 — gate open, absolute path outside the root: the
    // containment error names both the path and the root, and it is
    // NOT a file-not-found (the screen fires before any file access).
    std::env::set_var("GIGI_INGEST_DIR", root.path());
    let leak_abs = outside
        .path()
        .join("leak.csv")
        .to_string_lossy()
        .replace('\\', "/");
    let err = run(&mut e, &format!("INGEST g2 FROM '{leak_abs}' FORMAT CSV;")).unwrap_err();
    assert!(
        err.contains("escapes containment root"),
        "outside-root path must be a containment refusal: {err}"
    );
    assert!(err.contains("leak.csv"), "error names the offending path: {err}");
    let root_name = root
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string();
    assert!(err.contains(&root_name), "error names the root: {err}");
    assert!(!err.contains("not found"), "containment, not existence: {err}");

    // Same shape for the probe path the production checks fire.
    let err = run(&mut e, "INGEST g2 FROM '/tmp/nonexistent.npz' FORMAT NPZ;").unwrap_err();
    assert!(
        err.contains("escapes containment root"),
        "absolute probe path returns the containment error now: {err}"
    );

    // Phase 3 — relative path inside the root: succeeds, records land.
    run(&mut e, "INGEST g3 FROM 'inside/rows.csv' FORMAT CSV;").unwrap();
    match run(&mut e, "COVER g3 ALL;").unwrap() {
        ExecResult::Rows(rows) => assert_eq!(rows.len(), 2, "both CSV rows ingested"),
        other => panic!("expected rows, got {other:?}"),
    }

    // Phase 4 — '..'-traversal from inside the root: rejected lexically.
    let err = run(
        &mut e,
        "INGEST g4 FROM 'inside/../../leak.csv' FORMAT CSV;",
    )
    .unwrap_err();
    assert!(
        err.contains("'..'"),
        "parent traversal must be rejected by the component screen: {err}"
    );

    // Phase 5 — missing file INSIDE the root keeps the existing
    // file-not-found contract (the gate only changes what is reachable,
    // not how absence reads).
    let err = run(&mut e, "INGEST g5 FROM 'inside/ghost.csv' FORMAT CSV;").unwrap_err();
    assert!(err.contains("not found"), "in-root absence stays loud: {err}");

    // Phase 6 — a link INSIDE the root pointing OUTSIDE it: lexically
    // clean, canonically outside — refused by the canonical check.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), root.path().join("sneak")).unwrap();
        let err = run(&mut e, "INGEST g6 FROM 'sneak/leak.csv' FORMAT CSV;").unwrap_err();
        assert!(
            err.contains("not under containment root"),
            "symlink escape must be refused: {err}"
        );
    }
    #[cfg(windows)]
    {
        // Junctions need no privilege; skip gracefully if the temp
        // filesystem refuses (the unix twin pins the same logic).
        let link = root.path().join("sneak");
        let status = std::process::Command::new("cmd")
            .args([
                "/C",
                "mklink",
                "/J",
                &link.to_string_lossy(),
                &outside.path().to_string_lossy(),
            ])
            .status();
        match status {
            Ok(s) if s.success() => {
                let err =
                    run(&mut e, "INGEST g6 FROM 'sneak/leak.csv' FORMAT CSV;").unwrap_err();
                assert!(
                    err.contains("not under containment root"),
                    "junction escape must be refused: {err}"
                );
            }
            other => eprintln!("skipping junction phase: mklink /J unavailable ({other:?})"),
        }
    }

    std::env::remove_var("GIGI_INGEST_DIR");
}
