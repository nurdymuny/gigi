//! The GIGI Sheets edit and delete paths must be journalled.
//!
//! Found by TDD-IDX W-IDX-2's metadata-door audit and sized by reading the
//! client: `sheets/src/lib/gigi-client.ts` calls
//!
//!   * `POST /v1/bundles/{name}/update`  (line 462)  -> `update_records_v2`
//!   * `POST /v1/bundles/{name}/delete`  (line 730)  -> `delete_records_v2`
//!
//! Both handlers took `bundle_mut` and called a store method directly rather
//! than the journalling `Engine::update` / `Engine::delete`. The mutation
//! applied, the response said success, and **nothing reached the WAL** — so it
//! was durable only if a snapshot happened to run before the next restart.
//!
//! That is every edit and every delete made in the spreadsheet UI.
//!
//! These tests use the versioned and returning variants specifically, because
//! those are the store methods the two handlers call — testing plain
//! `Engine::update` would pass without touching the defect.
//!
//! Written before the fix. Observed red.

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::PathBuf;

fn dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_sheets_{tag}"))
}

fn schema() -> BundleSchema {
    BundleSchema::new("sheet")
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("cell"))
}

fn row(i: i64, cell: &str) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert("cell".into(), Value::Text(cell.into()));
    r
}

fn key(i: i64) -> Record {
    let mut k = Record::new();
    k.insert("id".into(), Value::Integer(i));
    k
}

/// Editing a cell must survive a restart with no snapshot in between.
///
/// No snapshot is the point: a snapshot writes live RAM to the `.dhoom` and
/// would mask a missing journal entry entirely. A crash does not run one first.
#[test]
fn a_sheet_edit_survives_restart_without_a_snapshot() {
    let d = dir("edit");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema()).unwrap();
        e.insert("sheet", &row(1, "original")).unwrap();

        let mut patch = Record::new();
        patch.insert("cell".into(), Value::Text("edited".into()));
        e.update_versioned("sheet", &key(1), &patch, 0)
            .expect("versioned update applies");

        assert_eq!(
            e.point_query("sheet", &key(1)).unwrap().unwrap().get("cell"),
            Some(&Value::Text("edited".into()))
        );
    }

    {
        let e = Engine::open(&d).unwrap();
        assert_eq!(
            e.point_query("sheet", &key(1)).unwrap().unwrap().get("cell"),
            Some(&Value::Text("edited".into())),
            "the edit was applied and acknowledged but never journalled — every \
             cell edit in the Sheets UI is durable only via a later snapshot"
        );
    }

    let _ = fs::remove_dir_all(&d);
}

/// Deleting a row must stay deleted. The mirror failure is worse than a lost
/// edit: the row comes back.
#[test]
fn a_sheet_delete_survives_restart_without_a_snapshot() {
    let d = dir("delete");
    let _ = fs::remove_dir_all(&d);

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema()).unwrap();
        e.insert("sheet", &row(1, "a")).unwrap();
        e.insert("sheet", &row(2, "b")).unwrap();

        let removed = e
            .delete_returning("sheet", &key(2))
            .expect("delete_returning succeeds")
            .expect("row 2 existed");
        assert_eq!(removed.get("cell"), Some(&Value::Text("b".into())));
        assert_eq!(e.total_records(), 1);
    }

    {
        let e = Engine::open(&d).unwrap();
        assert!(
            e.point_query("sheet", &key(2)).unwrap().is_none(),
            "the deleted row came back after restart — the delete was never journalled"
        );
        assert_eq!(e.total_records(), 1, "record count must reflect the delete");
    }

    let _ = fs::remove_dir_all(&d);
}

/// The versioned update's optimistic-concurrency contract must survive too:
/// the version it returned must be the version after a restart, or a client
/// that cached it will get a spurious conflict on its next write.
#[test]
fn the_version_returned_to_the_client_survives_restart() {
    let d = dir("version");
    let _ = fs::remove_dir_all(&d);

    let handed_to_client = {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema()).unwrap();
        e.insert("sheet", &row(1, "v0")).unwrap();
        let mut patch = Record::new();
        patch.insert("cell".into(), Value::Text("v1".into()));
        e.update_versioned("sheet", &key(1), &patch, 0).unwrap()
    };

    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        let mut patch = Record::new();
        patch.insert("cell".into(), Value::Text("v2".into()));
        // A client holding the version from before the restart must be able to
        // use it. If the update was not journalled, the record replays at its
        // pre-edit version and this is rejected as a conflict.
        e.update_versioned("sheet", &key(1), &patch, handed_to_client)
            .expect("the version handed to the client must still be current");
    }

    let _ = fs::remove_dir_all(&d);
}
