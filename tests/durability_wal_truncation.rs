//! TDD-DUR — the WAL-truncation data-loss class.
//!
//! On 2026-08-12 a production restart surfaced silent loss: 10 bundles and 180
//! `jg_kv` records gone, including a creator account and an approved
//! id_verification. A CRC-validated parse of all 2,341,314 WAL records found
//! zero create/insert ops for them — they were unrecoverable.
//!
//! The mechanism: a WAL compaction ran without first checkpointing mmap-backed
//! bundles. `snapshot_with_chunk_size_report` iterates only `self.bundles` and
//! never touches `self.mmap_bundles`, then compacts the WAL. Writes to an mmap
//! bundle live in a RAM overlay whose ONLY durable form is the post-checkpoint
//! WAL. Compaction deletes it while the `.dhoom` base stays at its old
//! contents. The call returns `{"status":"ok"}` because RAM is still intact.
//! The loss becomes visible one restart later.
//!
//! WHY THE EXISTING SUITE MISSED IT — and what these tests do differently.
//! Not one test in the repository calls a snapshot entry point on an engine
//! whose `mmap_bundles` map is non-empty; every one is preceded by
//! `Engine::open` (heap), which makes `self.bundles` accidentally total and
//! the buggy loop accidentally correct. The nearest regression test asserts
//! `Path::exists()` on the `.dhoom` files and never restarts, so it would pass
//! against a build that writes 0-byte snapshots.
//!
//! So these live in `tests/` on purpose. `mmap_bundles` is a private field, so
//! from here nothing can be asserted about RAM: every final assertion happens
//! after the engine has been dropped and re-opened from `data_dir` alone. That
//! is the only shape that distinguishes "in memory" from "on disk".

use gigi::engine::Engine;
use gigi::types::{BundleSchema, FieldDef, Record, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn test_dir(tag: &str) -> PathBuf {
    std::env::temp_dir().join(format!("gigi_dur_{tag}"))
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn schema(name: &str) -> BundleSchema {
    BundleSchema::new(name)
        .base(FieldDef::numeric("id"))
        .fiber(FieldDef::categorical("tag"))
}

fn rec(i: i64) -> Record {
    let mut r = Record::new();
    r.insert("id".into(), Value::Integer(i));
    r.insert("tag".into(), Value::Text(format!("tag_{i}")));
    r
}

fn key(i: i64) -> Record {
    let mut k = Record::new();
    k.insert("id".into(), Value::Integer(i));
    k
}

// ---------------------------------------------------------------- T1

/// T1 — the incident itself, as a full restart simulation.
///
/// A record written to a bundle that is mmap-backed must survive the admin
/// snapshot route. Today it does not: the snapshot never enumerates the
/// bundle, writes no `.dhoom` for it, and then compacts away the WAL that was
/// holding the overlay.
#[test]
fn admin_snapshot_does_not_erase_mmap_overlay_records() {
    let dir = test_dir("t1_admin_snapshot_overlay");
    cleanup(&dir);

    // P1 — create and snapshot so a .dhoom exists and the bundle becomes
    // mmap-backed on the next open.
    {
        let mut e = Engine::open(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema("b")).unwrap();
        for i in 0..3 {
            e.insert("b", &rec(i)).unwrap();
        }
        e.snapshot().unwrap();
    }

    // P2 — reopen in mmap mode. `b` now lives in `mmap_bundles`, so a write
    // to it lands in the RAM overlay plus the WAL, and nowhere else.
    //
    // `disabled = true` is load-bearing: without it the maybe_checkpoint ->
    // maybe_auto_compact chain can route to mmap_rebase_snapshot, which is
    // the one path that has always been correct, and the test would pass for
    // the wrong reason.
    {
        let mut e = Engine::open_mmap(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.insert("b", &rec(99)).unwrap();
        assert_eq!(e.total_records(), 4, "overlay write must be visible in RAM");

        // P3 — the exact call the admin HTTP route makes.
        e.snapshot_with_report().unwrap();
    }

    // P4 — restart from disk alone.
    {
        let e = Engine::open_mmap(&dir).unwrap();
        assert_eq!(
            e.total_records(),
            4,
            "record written to an mmap-backed bundle was erased by the admin \
             snapshot: it existed only in the WAL, and compaction dropped it"
        );
        assert!(
            e.point_query("b", &key(99)).unwrap().is_some(),
            "the overlay record must be readable after restart"
        );
    }

    cleanup(&dir);
}

// ---------------------------------------------------------------- T9

/// CRC-32C over `op || payload`, matching `wal.rs`'s `crc32` (Castagnoli,
/// reflected, poly 0x82F63B78). Used only to locate record boundaries.
fn crc32c(buf: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, e) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0x82F6_3B78 } else { c >> 1 };
        }
        *e = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in buf {
        crc = (crc >> 8) ^ table[((crc ^ b as u32) & 0xFF) as usize];
    }
    crc ^ 0xFFFF_FFFF
}

/// Byte offsets of every record in a WAL, framing `[u32 len][u8 op][payload][u32 crc]`.
fn wal_record_offsets(path: &Path) -> Vec<usize> {
    let d = fs::read(path).unwrap();
    let mut offs = Vec::new();
    let mut off = 0usize;
    while off + 8 <= d.len() {
        let ln = u32::from_le_bytes(d[off..off + 4].try_into().unwrap()) as usize;
        if ln == 0 || off + 4 + ln + 4 > d.len() {
            break;
        }
        let want = u32::from_le_bytes(d[off + 4 + ln..off + 8 + ln].try_into().unwrap());
        if crc32c(&d[off + 4..off + 4 + ln]) != want {
            break;
        }
        offs.push(off);
        off += 4 + ln + 4;
    }
    offs
}

/// T9 — one corrupt record in the middle of the WAL must not discard every
/// record after it.
///
/// This is the defect that lost eleven live records during the 2026-08-12
/// remediation deploy — a force-verified id_verification, four Stripe
/// webhook-dedup rows, three chat conversations, an onboarding token and an
/// audit row. None had expired; four had no expiry at all. They sat after a
/// corrupt entry roughly 90% of the way through a 1.35 GB WAL, and every
/// restart discarded them again.
///
/// `finish_wal_replay_prefix` turns a CRC mismatch *anywhere* into `Ok(())`
/// and keeps only the prefix. That is correct for a torn final write from a
/// crash. It is wrong for corruption in the middle of a file that has
/// perfectly good records after it — and nothing distinguishes the two cases.
#[test]
fn wal_replay_recovers_records_after_a_mid_file_corruption() {
    let dir = test_dir("t9_wal_midfile_corruption");
    cleanup(&dir);

    // Eleven inserts straight into the WAL, no snapshot. The .dhoom load
    // path is deliberately kept out of this fixture: it has its own defect
    // (a snapshotted record is not reloaded when inserts follow in the same
    // session) and mixing the two would mean a failure here could not be
    // attributed to either.
    {
        let mut e = Engine::open(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema("b")).unwrap();
        for i in 0..11 {
            e.insert("b", &rec(i)).unwrap();
        }
        assert_eq!(e.total_records(), 11);
    }

    // CONTROL — reopen with the WAL untouched. If this does not see all 11,
    // the fixture is wrong and any conclusion drawn after corrupting it would
    // be measuring the wrong thing.
    {
        let e = Engine::open(&dir).unwrap();
        let present: Vec<i64> = (0..11)
            .filter(|i| e.point_query("b", &key(*i)).unwrap().is_some())
            .collect();
        assert_eq!(
            present.len(),
            11,
            "control failed before any corruption was applied: present={present:?}"
        );
    }

    // Corrupt one record in the middle, leaving several valid ones after it.
    let wal = dir.join("gigi.wal");
    let offs = wal_record_offsets(&wal);
    assert!(
        offs.len() >= 8,
        "need a WAL with several records to corrupt the middle of, got {}",
        offs.len()
    );
    let victim = offs[offs.len() - 4]; // 3 good records follow it
    let mut bytes = fs::read(&wal).unwrap();
    let ln = u32::from_le_bytes(bytes[victim..victim + 4].try_into().unwrap()) as usize;
    bytes[victim + 4 + ln] ^= 0xFF; // flip a CRC byte
    fs::write(&wal, &bytes).unwrap();

    // Reopen from disk alone.
    {
        let e = Engine::open(&dir).unwrap();
        let present: Vec<i64> = (0..11)
            .filter(|i| e.point_query("b", &key(*i)).unwrap().is_some())
            .collect();
        let missing: Vec<i64> = (0..11).filter(|i| !present.contains(i)).collect();

        // Exactly one record was damaged, so exactly one may be missing.
        // Asserting the whole surviving set — not a count — means a future
        // regression names which record it dropped.
        assert_eq!(
            missing.len(),
            1,
            "one record was corrupted, so exactly one should be missing. \
             missing={missing:?} present={present:?}. More than one means \
             replay is still discarding records after the damage; zero would \
             mean the corruption was not actually applied."
        );
    }

    cleanup(&dir);
}

// ---------------------------------------------------------------- T10

/// T10 — a snapshotted record must reload when inserts follow in the SAME
/// session.
///
/// Found by T9's control phase. `snapshot()` writes the `.dhoom` correctly —
/// verified on disk — but on reopen the snapshotted record is absent while
/// every post-snapshot WAL insert is present.
///
/// `snapshot_then_new_inserts_survive_reopen` (engine.rs:3845) covers the same
/// shape and passes, because it closes the engine and reopens between the
/// snapshot and the inserts. Staying in one session is the untested path — and
/// it is the one production takes, since the admin snapshot route runs against
/// a live engine that keeps serving writes afterwards.
#[test]
fn snapshotted_record_reloads_when_inserts_follow_in_same_session() {
    let dir = test_dir("t10_dhoom_reload_same_session");
    cleanup(&dir);

    {
        let mut e = Engine::open(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema("b")).unwrap();
        e.insert("b", &rec(100)).unwrap();
        e.snapshot().unwrap();
        // Same session — no close/reopen between the snapshot and these.
        for i in 1..=10 {
            e.insert("b", &rec(i)).unwrap();
        }
        assert_eq!(e.total_records(), 11, "all 11 visible in RAM before restart");
    }

    // The snapshot really did capture it.
    let dhoom = dir.join("snapshots").join("b.dhoom");
    assert!(dhoom.exists(), "snapshot file must exist");
    let body = fs::read_to_string(&dhoom).unwrap();
    assert!(
        body.contains("100"),
        "the .dhoom must contain the pre-snapshot record; got {body:?}"
    );

    {
        let e = Engine::open(&dir).unwrap();
        let present: Vec<i64> = (1..=10)
            .chain(std::iter::once(100))
            .filter(|i| e.point_query("b", &key(*i)).unwrap().is_some())
            .collect();
        assert!(
            present.contains(&100),
            "the snapshotted record was not reloaded from its .dhoom even \
             though the file on disk contains it. present={present:?}"
        );
        assert_eq!(present.len(), 11, "present={present:?}");
    }

    cleanup(&dir);
}

// ---------------------------------------------------------------- T7

/// T7 — the backstop. A compaction must not rename over the only copy of the
/// WAL; the outgoing generation has to be retained.
///
/// This requires no correctness reasoning about mmap vs heap and changes
/// nothing on the success path. It does not prevent a violation — it makes any
/// violation that slips through recoverable. That is the difference between
/// 2026-08-12 being a ten-minute restore and being permanent: the forensics
/// that read 2,341,314 records had to work from a WAL that no longer contained
/// the lost ops.
#[test]
fn wal_generation_is_retained_across_compaction() {
    let dir = test_dir("t7_wal_retention");
    cleanup(&dir);

    {
        let mut e = Engine::open(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema("b")).unwrap();
        for i in 0..200 {
            e.insert("b", &rec(i)).unwrap();
        }
        // Compaction happens inside snapshot(): the WAL is rewritten to
        // schemas only, so the 200 insert entries are dropped from it.
        e.snapshot().unwrap();
    }

    let live_wal = dir.join("gigi.wal").metadata().unwrap().len();

    let retained: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with("gigi.wal.") && s.ends_with(".compacted"))
        })
        .collect();

    assert!(
        !retained.is_empty(),
        "compaction renamed over the only copy of the WAL — nothing was \
         retained, so any data that lived only in it is gone permanently. \
         Expected a gigi.wal.<ts>.compacted alongside the live WAL."
    );

    let retained_len = retained[0].metadata().unwrap().len();
    assert!(
        retained_len > live_wal,
        "the retained generation ({retained_len} B) should be larger than the \
         schema-only WAL that replaced it ({live_wal} B) — it holds the insert \
         entries that were compacted away"
    );

    cleanup(&dir);
}

// ---------------------------------------------------------------- T2

/// T2 — deleted records must not come back.
///
/// `compact_wal_to_schemas` re-emits CreateBundle and friends but never a
/// Delete. A bundle emptied by deletes is then skipped by the snapshot loop's
/// `if count == 0 { continue; }`, leaving a stale `.dhoom` that still holds
/// the deleted rows — while the WAL entries that removed them are compacted
/// away. On the next boot the rows return.
///
/// For `id_verification`-shaped data a silent un-delete is worse than a loss.
#[test]
fn deleted_records_do_not_resurrect_across_restart() {
    let dir = test_dir("t2_delete_resurrect");
    cleanup(&dir);

    {
        let mut e = Engine::open(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema("b")).unwrap();
        for i in 0..3 {
            e.insert("b", &rec(i)).unwrap();
        }
        e.snapshot().unwrap(); // b.dhoom holds 3 records

        for i in 0..3 {
            assert!(e.delete("b", &key(i)).unwrap(), "delete {i} should report true");
        }
        assert_eq!(e.total_records(), 0, "all rows deleted in RAM");

        e.snapshot().unwrap(); // skips the now-empty bundle, then truncates
    }

    // Heap boot path.
    {
        let e = Engine::open(&dir).unwrap();
        assert_eq!(
            e.total_records(),
            0,
            "deleted records resurrected after restart: the stale .dhoom was \
             reloaded and the Deletes were compacted out of the WAL"
        );
        for i in 0..3 {
            assert!(
                e.point_query("b", &key(i)).unwrap().is_none(),
                "record {i} was deleted but came back"
            );
        }
    }

    // mmap boot path — pin both.
    {
        let e = Engine::open_mmap(&dir).unwrap();
        assert_eq!(e.total_records(), 0, "resurrection on the mmap boot path");
    }

    cleanup(&dir);
}

// ---------------------------------------------------------------- T3

/// T3 — a delete against an mmap-backed bundle must survive a plain restart,
/// with no compaction involved at all and the WAL fully intact.
///
/// Isolated from T2 deliberately, so a failure names one mechanism: the
/// tombstone is keyed one way when written live and another when replayed by
/// `open_mmap`'s Phase 3, and the replayed form matches nothing on any read
/// path.
#[test]
#[ignore = "OPEN BUG (TDD-DUR W7). A delete against an mmap-backed bundle is \
lost on restart even with an intact WAL. The tombstone is filed under \
Engine::pk_string -> `[(\"id\", Integer(1))]` but every reader looks it up by \
OverlayBundle::tombstone_key / base_pk_set -> `Integer(1)`, so it never \
matches. point_query only appears correct because it separately consults the \
overlay. Unifying the key touches every mmap read path; design is in \
theory/gigi/TDD_DUR_wal_truncation_invariant.md section 2.8. Run with \
`cargo test -- --ignored` to see it fail."]
fn mmap_delete_survives_restart_with_intact_wal() {
    let dir = test_dir("t3_mmap_delete");
    cleanup(&dir);

    {
        let mut e = Engine::open(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        e.create_bundle(schema("b")).unwrap();
        for i in 0..3 {
            e.insert("b", &rec(i)).unwrap();
        }
        e.snapshot().unwrap();
    }

    {
        let mut e = Engine::open_mmap(&dir).unwrap();
        e.compaction_policy_mut().disabled = true;
        assert!(e.delete("b", &key(1)).unwrap(), "delete should report true");
        assert!(
            e.point_query("b", &key(1)).unwrap().is_none(),
            "delete must be visible immediately"
        );
        assert_eq!(
            e.total_records(),
            2,
            "live count must reflect the tombstone"
        );
        // NO snapshot — the WAL still holds the Delete. A restart alone must
        // be enough.
    }

    {
        let e = Engine::open_mmap(&dir).unwrap();
        assert!(
            e.point_query("b", &key(1)).unwrap().is_none(),
            "deleted record returned after a restart with an intact WAL: the \
             replayed tombstone is encoded differently from the live one"
        );
        assert_eq!(e.total_records(), 2, "count must stay at 2 after restart");
    }

    cleanup(&dir);
}
