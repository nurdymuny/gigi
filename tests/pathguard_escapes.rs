//! pathguard::contain — the escape attack matrix.
//!
//! The helper is the single containment story for every env-rooted
//! file touch (GIGI_EMIT_DIR writes today, GIGI_INGEST_DIR reads):
//! component-level lexical rejection BEFORE joining (Prefix / RootDir /
//! ParentDir), then canonical-to-canonical verification so symlinks and
//! junctions cannot tunnel out of the root.
//!
//! Every test uses its OWN env-var name as the containment root, so the
//! matrix runs on parallel test threads with no set_var races — the
//! `root_env` parameter exists exactly for this.

use std::path::PathBuf;

use gigi::pathguard::{contain, PathGuardError};

/// Fresh tempdir exported under `var`; returns the guard so the dir
/// lives for the test's duration.
fn root_at(var: &str) -> tempfile::TempDir {
    let d = tempfile::tempdir().expect("tempdir");
    std::env::set_var(var, d.path());
    d
}

fn write_under(dir: &std::path::Path, rel: &str, body: &str) -> PathBuf {
    let p = dir.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, body).unwrap();
    p
}

// ── Gate: unset root fails closed ───────────────────────────────────

#[test]
fn root_unset_fails_closed_both_modes() {
    let var = "GIGI_PATHGUARD_TEST_UNSET";
    std::env::remove_var(var);
    for must_exist in [true, false] {
        match contain(var, "x.csv", must_exist) {
            Err(PathGuardError::RootUnset { var: v }) => {
                assert_eq!(v, var);
            }
            other => panic!("unset root must fail closed, got {other:?}"),
        }
    }
    // The Display names the env var so the remedy is one export away.
    let err = contain(var, "x.csv", true).unwrap_err();
    assert!(err.to_string().contains(var), "{err}");
}

#[test]
fn root_set_but_empty_fails_closed() {
    let var = "GIGI_PATHGUARD_TEST_EMPTY_ROOT";
    std::env::set_var(var, "");
    assert!(
        matches!(contain(var, "x.csv", false), Err(PathGuardError::RootUnset { .. })),
        "empty-string root must count as unset"
    );
    std::env::remove_var(var);
}

// ── Lexical screen: ParentDir ───────────────────────────────────────

#[test]
fn parent_traversal_rejected() {
    let var = "GIGI_PATHGUARD_TEST_DOTDOT";
    let _root = root_at(var);
    for path in ["../x", "a/../../x", "a/../b", ".."] {
        match contain(var, path, false) {
            Err(PathGuardError::Escape { path: p, .. }) => assert_eq!(p, path),
            other => panic!("'{path}' must be rejected lexically, got {other:?}"),
        }
        // read mode screens identically — before any filesystem access
        assert!(
            matches!(contain(var, path, true), Err(PathGuardError::Escape { .. })),
            "'{path}' must be rejected in read mode too"
        );
    }
    // the message names the offense so the caller can fix the path
    let err = contain(var, "../x", false).unwrap_err();
    assert!(err.to_string().contains(".."), "{err}");
}

// ── Lexical screen: rooted / absolute ───────────────────────────────

#[test]
fn rooted_paths_rejected() {
    let var = "GIGI_PATHGUARD_TEST_ROOTED";
    let _root = root_at(var);
    // '/abs' carries RootDir on unix AND windows
    assert!(
        matches!(contain(var, "/abs", false), Err(PathGuardError::Escape { .. })),
        "'/abs' must be rejected"
    );
    assert!(
        matches!(contain(var, "/tmp/nonexistent.npz", true), Err(PathGuardError::Escape { .. })),
        "absolute read paths are rejected before any filesystem access"
    );
}

#[cfg(windows)]
#[test]
fn windows_rooted_backslash_rejected() {
    // '\rooted' is NOT is_absolute() on Windows (no drive prefix) — the
    // legacy emit_target check missed it; join() would have replaced the
    // root and written to the current drive's root. The component screen
    // sees RootDir and kills it.
    let var = "GIGI_PATHGUARD_TEST_BSLASH_ROOT";
    let _root = root_at(var);
    assert!(
        matches!(contain(var, r"\rooted", false), Err(PathGuardError::Escape { .. })),
        r"'\rooted' must be rejected"
    );
}

#[cfg(windows)]
#[test]
fn windows_drive_prefix_rejected() {
    // 'C:file' is drive-RELATIVE: is_absolute() == false, no ParentDir —
    // the legacy screen accepted it and Path::join REPLACES the whole
    // path when the argument carries a Prefix component. That was the
    // escape. Component screen: Prefix → reject.
    let var = "GIGI_PATHGUARD_TEST_DRIVE";
    let _root = root_at(var);
    for path in ["C:file", r"C:\abs", "C:/abs"] {
        assert!(
            matches!(contain(var, path, false), Err(PathGuardError::Escape { .. })),
            "'{path}' must be rejected (drive prefix)"
        );
    }
}

#[cfg(windows)]
#[test]
fn windows_unc_rejected() {
    let var = "GIGI_PATHGUARD_TEST_UNC";
    let _root = root_at(var);
    assert!(
        matches!(contain(var, r"\\unc\share", false), Err(PathGuardError::Escape { .. })),
        r"'\\unc\share' must be rejected (UNC prefix)"
    );
}

#[cfg(unix)]
#[test]
fn unix_backslash_and_drive_are_plain_filenames() {
    // On unix these strings carry no Prefix/RootDir component — they are
    // ordinary (ugly) filenames and must be CONTAINED, not rejected: the
    // screen is component-semantic, not character-superstitious.
    let var = "GIGI_PATHGUARD_TEST_UNIX_NAMES";
    let root = root_at(var);
    for path in ["C:file", r"\rooted"] {
        let got = contain(var, path, false)
            .unwrap_or_else(|e| panic!("'{path}' is a plain filename on unix: {e}"));
        assert!(got.starts_with(root.path()), "{got:?} must sit under the root");
    }
}

// ── Accept: legit relative paths ────────────────────────────────────

#[test]
fn legit_relative_write_accepted_and_parent_created() {
    let var = "GIGI_PATHGUARD_TEST_WRITE_OK";
    let root = root_at(var);
    let got = contain(var, "legit/sub/file.csv", false).expect("contained write path");
    // returns the UNcanonicalized root.join(rel) — same PathBuf the
    // legacy emit code produced, so downstream receipts keep their text
    assert_eq!(got, root.path().join("legit").join("sub").join("file.csv"));
    // parent dirs exist afterward (write mode creates them, as the emit
    // executor always has)
    assert!(root.path().join("legit/sub").is_dir());
}

#[test]
fn legit_relative_read_accepted_canonical() {
    let var = "GIGI_PATHGUARD_TEST_READ_OK";
    let root = root_at(var);
    write_under(root.path(), "inside/rows.csv", "id,v\na,1\n");
    let got = contain(var, "inside/rows.csv", true).expect("contained read path");
    let canonical_root = std::fs::canonicalize(root.path()).unwrap();
    assert!(
        got.starts_with(&canonical_root),
        "read mode returns the canonical path under the canonical root: {got:?}"
    );
    assert!(got.ends_with("rows.csv") || got.to_string_lossy().ends_with("rows.csv"));
}

#[test]
fn curdir_stripped_empty_and_dot_rejected() {
    let var = "GIGI_PATHGUARD_TEST_CURDIR";
    let root = root_at(var);
    // './x' is 'x'
    let got = contain(var, "./x.csv", false).expect("curdir components are noise");
    assert_eq!(got, root.path().join("x.csv"));
    // '', '.', './' resolve to the root itself — not a file, rejected
    for path in ["", ".", "./"] {
        assert!(
            matches!(contain(var, path, false), Err(PathGuardError::Escape { .. })),
            "'{path}' must be rejected (no usable components)"
        );
    }
}

// ── Read mode: missing file under the root ──────────────────────────

#[test]
fn missing_file_read_reports_notfound_io() {
    let var = "GIGI_PATHGUARD_TEST_GHOST";
    let _root = root_at(var);
    match contain(var, "ghost.csv", true) {
        Err(PathGuardError::Io { path, source }) => {
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            assert!(
                path.to_string_lossy().ends_with("ghost.csv"),
                "Io names the resolved candidate: {path:?}"
            );
        }
        other => panic!("missing in-root file is an Io/NotFound, got {other:?}"),
    }
}

#[test]
fn root_pointing_at_missing_dir_is_unresolvable_for_reads() {
    let var = "GIGI_PATHGUARD_TEST_NO_ROOT_DIR";
    let d = tempfile::tempdir().unwrap();
    let missing = d.path().join("never_created");
    std::env::set_var(var, &missing);
    assert!(
        matches!(contain(var, "x.csv", true), Err(PathGuardError::RootUnresolvable { .. })),
        "read mode requires the root to exist"
    );
    // write mode creates the chain (the emit executor's create_dir_all
    // has always implied the root) and then contains normally
    let got = contain(var, "x.csv", false).expect("write mode may create the root");
    assert_eq!(got, missing.join("x.csv"));
    assert!(missing.is_dir(), "root chain created for writes");
    std::env::remove_var(var);
}

// ── Canonical verification: symlinks / junctions ────────────────────

#[cfg(unix)]
#[test]
fn symlink_inside_root_pointing_outside_rejected() {
    let var = "GIGI_PATHGUARD_TEST_SYMLINK";
    let root = root_at(var);
    let outside = tempfile::tempdir().unwrap();
    write_under(outside.path(), "leak.txt", "secret");
    std::os::unix::fs::symlink(outside.path(), root.path().join("sneak")).unwrap();

    // read through the link: lexically clean, canonically outside
    match contain(var, "sneak/leak.txt", true) {
        Err(PathGuardError::NotContained { path, root: r }) => {
            let msg = PathGuardError::NotContained { path, root: r }.to_string();
            assert!(msg.contains("not under"), "{msg}");
        }
        other => panic!("symlink escape must be rejected, got {other:?}"),
    }
    // write through the link: the canonical PARENT is outside
    assert!(
        matches!(contain(var, "sneak/out.csv", false), Err(PathGuardError::NotContained { .. })),
        "write through an outward symlink must be rejected"
    );
}

#[cfg(windows)]
#[test]
fn junction_inside_root_pointing_outside_rejected() {
    // Junctions are the unprivileged Windows analogue of the symlink
    // escape. `mklink /J` needs no SeCreateSymbolicLinkPrivilege; if the
    // filesystem still refuses (non-NTFS temp), skip with a note rather
    // than fail — the unix symlink twin pins the same containment logic.
    let var = "GIGI_PATHGUARD_TEST_JUNCTION";
    let root = root_at(var);
    let outside = tempfile::tempdir().unwrap();
    write_under(outside.path(), "leak.txt", "secret");
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
        Ok(s) if s.success() => {}
        other => {
            eprintln!("skipping junction case: mklink /J unavailable ({other:?})");
            return;
        }
    }
    match contain(var, "sneak/leak.txt", true) {
        Err(PathGuardError::NotContained { .. }) => {}
        other => panic!("junction escape must be rejected, got {other:?}"),
    }
    assert!(
        matches!(contain(var, "sneak/out.csv", false), Err(PathGuardError::NotContained { .. })),
        "write through an outward junction must be rejected"
    );
}

// ── Error surfaces name both paths ──────────────────────────────────

#[test]
fn escape_error_names_path_and_root() {
    let var = "GIGI_PATHGUARD_TEST_NAMES";
    let root = root_at(var);
    let err = contain(var, "../x", false).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("../x"), "names the offending path: {msg}");
    let root_name = root.path().file_name().unwrap().to_string_lossy().to_string();
    assert!(msg.contains(&root_name), "names the containment root: {msg}");
}
