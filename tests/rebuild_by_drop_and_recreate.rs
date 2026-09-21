//! Rebuilding a bundle by dropping and recreating it under the same name.
//!
//! This is the route left open to the Marcella corpus repair once in-place
//! repair proved impossible: the damaged rows have no readable key, so they
//! cannot be deleted, but the whole bundle can be replaced.
//!
//! It did not work, and it failed in the worst possible shape. Boot walks the
//! replayed schemas and opens `snapshots/{name}.dhoom` for each, and dropping a
//! bundle left that file in place. So a rebuild looked correct, served
//! correctly, and then resurrected the old rows as its base at the next
//! restart — with the repaired rows replaying on top, which is precisely the
//! both-copies state the repair had just rolled back from.
use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;

fn dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("gigi_rb_{tag}"));
    let _ = fs::remove_dir_all(&d);
    d
}

fn schema() -> BundleSchema {
    BundleSchema::new("sections")
        .base(FieldDef::categorical("section_id"))
        .fiber(FieldDef::categorical("content"))
}

fn rec(id: &str, content: &str) -> Record {
    let mut r = Record::new();
    r.insert("section_id".into(), Value::Text(id.into()));
    r.insert("content".into(), Value::Text(content.into()));
    r
}

fn contents(e: &Engine) -> Vec<String> {
    let mut v: Vec<String> = e
        .bundle("sections")
        .unwrap()
        .records()
        .filter_map(|r| r.get("content").and_then(|c| c.as_str().map(String::from)))
        .collect();
    v.sort();
    v
}

/// NOT LOAD-BEARING, and kept anyway with that said out loud.
///
/// This passes with the fix removed. A checkpoint fires during the rebuild and
/// overwrites the stale snapshot, so this fixture never reaches the state it
/// is describing. It documents the intended end-to-end behaviour and proves
/// nothing about the mechanism.
///
/// The test that does prove it is below: it asserts boot can no longer find
/// the dropped file, which is the whole of the fix. Removing the mechanism
/// turns that one red and leaves this one green — which is how this note came
/// to be written.
#[test]
fn a_rebuilt_bundle_does_not_resurrect_its_old_rows_on_restart() {
    let d = dir("resurrect");

    // The damaged original, snapshotted so it has a .dhoom on disk.
    {
        let mut e = Engine::open(&d).unwrap();
        e.create_bundle(schema()).unwrap();
        for i in 0..4 {
            e.insert("sections", &rec(&format!("s{i}"), "DAMAGED")).unwrap();
        }
        e.snapshot_with_chunk_size_report(50_000, None).expect("snapshot");
    }

    // The rebuild: drop, recreate under the same name, insert repaired rows.
    {
        let mut e = Engine::open_mmap(&d).expect("reopen");
        assert!(e.drop_bundle("sections").unwrap(), "bundle existed");
        e.create_bundle(schema()).unwrap();
        for i in 0..4 {
            e.insert("sections", &rec(&format!("s{i}"), "REPAIRED")).unwrap();
        }
        assert_eq!(
            contents(&e),
            vec!["REPAIRED"; 4],
            "the rebuild must look right immediately -- it always did"
        );
    }

    // The restart, which is where it used to come apart.
    let e = Engine::open_mmap(&d).expect("reopen after rebuild");
    let after = contents(&e);
    drop(e);
    let _ = fs::remove_dir_all(&d);

    assert!(
        !after.iter().any(|c| c == "DAMAGED"),
        "the dropped bundle's snapshot was loaded as the new one's base: {after:?}"
    );
    assert_eq!(after, vec!["REPAIRED"; 4], "only the repaired rows survive");
}

/// THE LOAD-BEARING ONE. The retired file is moved aside, not destroyed.
///
/// A dropped bundle's last snapshot is the only copy of whatever was in it, so
/// the fix renames rather than deletes. Boot looks for an exact name and stops
/// finding it; a human looking for the data still can.
#[test]
fn the_dropped_snapshot_is_kept_aside_rather_than_deleted() {
    let d = dir("kept");
    {
        let mut e = Engine::open(&d).unwrap();
        e.create_bundle(schema()).unwrap();
        e.insert("sections", &rec("s1", "x")).unwrap();
        e.snapshot_with_chunk_size_report(50_000, None).expect("snapshot");
    }
    let mut e = Engine::open_mmap(&d).expect("reopen");
    e.drop_bundle("sections").unwrap();
    drop(e);

    let snaps = d.join("snapshots");
    let live = snaps.join("sections.dhoom").exists();
    let aside = snaps.join("sections.dhoom.dropped").exists();
    let _ = fs::remove_dir_all(&d);

    assert!(!live, "boot must not be able to find it");
    assert!(aside, "but it must still exist");
}
