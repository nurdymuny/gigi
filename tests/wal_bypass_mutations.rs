//! TDD-IDX W-IDX-2 — the metadata-door audit's headline finding.
//!
//! Nine live HTTP mutation routes apply their change to the store and write
//! NOTHING to the WAL. They return success. The change is real in RAM and is
//! durable ONLY if a snapshot runs before the next restart — there is no
//! journal entry to replay.
//!
//! This is TDD_DUR section 5's "WAL-bypass mutations" item, which named
//! truncate_bundle and ttl_eviction_task and said ~15 other bundle_mut sites
//! wanted the same audit. This is that audit, and the class is larger than the
//! two it named: it covers the primary update and delete routes.
//!
//! **UPDATE 2026-08-16 — two of the nine are fixed.** The class was sized by
//! reading the clients rather than guessing: `sheets/src/lib/gigi-client.ts`
//! calls `POST .../update` (line 462) and `POST .../delete` (line 730), so the
//! GIGI Sheets UI's edit and delete paths were both in it. Those two handlers
//! now route through `Engine::update_versioned` / `Engine::delete_returning`,
//! which journal before applying, and are gated by
//! `tests/sheets_write_durability.rs`.
//!
//! **Seven remain**: PATCH and DELETE `{name}/{path}`, PATCH `{name}/records`,
//! and POST `{name}/upsert`, `/bulk-delete`, `/truncate`, `/increment`. No
//! client on disk references them, which is why they were not done first — but
//! "no client on disk" is not "no caller", and this test stays as their gate.
//!
//! Belongs to its own spec, not TDD-IDX. Recorded here so the finding carries a
//! reproduction rather than a claim.
use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;

fn r(i: i64, tag: &str) -> Record {
    let mut m = Record::new();
    m.insert("id".into(), Value::Integer(i));
    m.insert("tag".into(), Value::Text(tag.into()));
    m
}

#[ignore = "OPEN BUG (found 2026-08-15 by TDD-IDX W-IDX-2). Nine live HTTP mutation routes bypass the WAL: PATCH/DELETE {name}/{path}, PATCH {name}/records, POST {name}/upsert, /bulk-delete, /truncate, /increment, /update, /delete. Each takes bundle_mut and calls a store method directly instead of the journalling Engine::update / Engine::delete, so the mutation is durable only via a subsequent snapshot. Verified by execution: the value reads `after` live and `before` after a restart with no snapshot in between. Invalidation is fine on all of them; journalling is what is missing. Wants its own spec - this is TDD_DUR section 5 WAL-bypass, now sized. Run with `cargo test -- --ignored`."]
#[test]
fn store_level_update_survives_restart() {
    let d = std::env::temp_dir().join("gigi_walbypass");
    let _ = fs::remove_dir_all(&d);
    {
        let mut e = Engine::open(&d).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(
            BundleSchema::new("b")
                .base(FieldDef::numeric("id"))
                .fiber(FieldDef::categorical("tag")),
        )
        .unwrap();
        e.insert("b", &r(1, "before")).unwrap();

        // exactly what patch_by_path does: bundle_mut -> store.update
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let mut patch = Record::new();
        patch.insert("tag".into(), Value::Text("after".into()));
        assert!(e.bundle_mut("b").unwrap().update(&key, &patch), "update applied");

        let got = e.point_query("b", &key).unwrap().unwrap();
        println!("  live      : tag = {:?}", got.get("tag"));
        assert_eq!(got.get("tag"), Some(&Value::Text("after".into())));
    }
    {
        let e = Engine::open(&d).unwrap();
        let mut key = Record::new();
        key.insert("id".into(), Value::Integer(1));
        let got = e.point_query("b", &key).unwrap().unwrap();
        println!("  restarted : tag = {:?}", got.get("tag"));
        assert_eq!(
            got.get("tag"),
            Some(&Value::Text("after".into())),
            "PATCH-shaped mutation lost on restart — the handler bypasses the WAL"
        );
    }
    let _ = fs::remove_dir_all(&d);
}
