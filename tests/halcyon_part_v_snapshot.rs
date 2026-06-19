//! TDD-HAL-V.4 — the load-bearing smoke gate for Part V snapshots.
//!
//! Spec: `HALCYON_PART_V_SNAPSHOT_GATES.md` §P2.1.
//!
//! Bee's locked decisions (ratified 2026-06-19) wired through:
//!
//! - **D-V-A** — WAL op encoding is explicit little-endian
//!   (`f64::to_le_bytes` write / `f64::from_le_bytes` read at the
//!   `OP_GAUGE_FIELD_SNAPSHOT` (0x0B) site in `src/wal.rs`).
//! - **D-V-B** — HTTP surface is `/v1/gql` only. There is no
//!   `POST /v1/gauge_field/{name}/snapshot` route. The integration
//!   path this test drives is `parser::execute` (the same path the
//!   binary's `gql_query` consults via `try_dispatch_gauge_statement`).
//! - **D-V-C** — SHA-256 over the LE-encoded buffer bytes is the
//!   canonical citation handle; the same SHA-256 lands in the WAL
//!   entry AND in the Rows envelope returned to the caller.
//! - **D-V-D** — `PERSIST` is REQUIRED on `SNAPSHOT GAUGE_FIELD` (bare
//!   `SNAPSHOT GAUGE_FIELD U;` parse-errors). Every existing call site
//!   is already explicit so the future `TRANSIENT` variant ships
//!   without flipping any caller.
//!
//! This gate is the load-bearing one because it exercises the full
//! chain end-to-end through the public GQL surface:
//!
//!     LATTICE buckyball …
//!     GAUGE_FIELD U … PERSIST
//!     GIBBS_SAMPLE U …                   ← thermalizes off identity
//!     SNAPSHOT GAUGE_FIELD U PERSIST     ← writes WAL OP 0x0B + Rows
//!     drop(engine)                       ← close
//!     Engine::open(same data dir)        ← triggers WAL replay
//!     gauge_registry::get("U").as_dense_buffer().data
//!         == pre-close buffer            ← byte-identical Vec<f64>
//!     SHA-256(post-reopen LE bytes)
//!         == pre-close SHA-256           ← citation handle is stable
//!
//! If this gate is green, the spec's primary use case — Halcyon's
//! `verify_canonical_receipt.py --snapshot` flag promoting from a
//! 30-second thermalization to a sub-100ms cached read — works.
//!
//! Optionality contract: gated on `halcyon` (composite feature pulling
//! in `lattice + gauge`) so the no-default-features build stays
//! byte-identical at 852/0.

#![cfg(feature = "halcyon")]

use gigi::engine::Engine;
use gigi::gauge::registry as gauge_registry;
use gigi::lattice::registry as lattice_registry;
use gigi::parser::{execute, parse, ExecResult};
use gigi::types::Value;

/// Re-derives the canonical SHA-256 citation handle over a buffer's
/// LE-encoded bytes (D-V-A + D-V-C). Mirrors
/// `wal::GaugeFieldSnapshotPayload::compute_buffer_sha256`; we inline
/// the derivation here so the test asserts the citation contract from
/// outside the WAL module rather than trusting it.
fn sha256_le_bytes_hex(buf: &[f64]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let mut le = [0u8; 8];
    for v in buf {
        le.copy_from_slice(&v.to_le_bytes());
        hasher.update(&le);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest.iter() {
        use std::fmt::Write;
        write!(&mut out, "{:02x}", byte).unwrap();
    }
    out
}

/// TDD-HAL-V.4: smoke gate — SNAPSHOT writes a WAL entry that survives
/// engine close and replays byte-identically on reopen.
#[test]
fn tdd_hal_v_1_snapshot_writes_and_replays() {
    // Serialize against every other Halcyon test in this crate — the
    // gauge + lattice registries are process singletons and the engine
    // owns the WAL file. Two tests racing the same data dir would
    // either step on each other's WAL or trip the orphan-snapshot
    // check on a partial replay.
    let _serial = gauge_registry::test_serial_lock();

    // 1. Temp data dir — the engine owns the WAL under this path; the
    //    reopen step below points at the same path so replay walks the
    //    WAL we just wrote.
    let dir = tempfile::tempdir().expect("tempdir");
    let data_path = dir.path().to_path_buf();

    // 2. Open the engine. `Engine::open` clears the lattice + gauge
    //    registries as part of its open path, so the registry resets
    //    below have to come AFTER open (otherwise open clobbers them).
    let mut engine = Engine::open(&data_path).expect("engine open");
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();

    // 3. Execute the full chain via `parser::execute` — mirrors the
    //    production `/v1/gql` path through `try_dispatch_gauge_statement`.
    let stmt = parse("LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';")
        .expect("parse LATTICE FROM TRUNCATED_ICOSAHEDRON");
    let _ = execute(&mut engine, &stmt).expect("exec LATTICE");

    let stmt = parse(
        "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY PERSIST;",
    )
    .expect("parse GAUGE_FIELD PERSIST");
    let _ = execute(&mut engine, &stmt).expect("exec GAUGE_FIELD PERSIST");

    let stmt = parse(
        "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 10 MEASURE_EVERY 1 \
         MEASURE (MEAN(PLAQUETTE)) SEED 20260616;",
    )
    .expect("parse GIBBS_SAMPLE");
    let _ = execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE");

    let stmt = parse("SNAPSHOT GAUGE_FIELD U PERSIST;")
        .expect("parse SNAPSHOT GAUGE_FIELD U PERSIST");
    let snapshot_result = execute(&mut engine, &stmt).expect("exec SNAPSHOT PERSIST");

    // 4. Capture pre-close buffer bytes (Vec<f64> from the dyn read
    //    registry — the same surface `SELECT PLAQUETTE` and the gauge
    //    HTTP read routes consume).
    let pre_close_buffer: Vec<f64> = gauge_registry::get("U")
        .expect("U must be in the dyn read registry after declare + thermalize")
        .as_dense_buffer()
        .data
        .clone();
    assert_eq!(
        pre_close_buffer.len(),
        90 * 4,
        "buckyball SU(2): 90 edges * 4 quaternion components = 360 f64"
    );

    // 5. Capture pre-close SHA-256 from the SNAPSHOT Rows envelope.
    //    This is the D-V-C citation contract: the same SHA-256 the
    //    WAL entry carries is the one we hand back to the caller.
    let rows = match snapshot_result {
        ExecResult::Rows(r) => r,
        other => panic!("SNAPSHOT GAUGE_FIELD must return Rows, got {other:?}"),
    };
    assert_eq!(rows.len(), 1, "SNAPSHOT returns a single citation row");
    let pre_close_sha = match rows[0].get("sha256") {
        Some(Value::Text(s)) => s.clone(),
        other => panic!("SNAPSHOT Rows envelope missing sha256: {other:?}"),
    };
    assert_eq!(
        pre_close_sha.len(),
        64,
        "SHA-256 is 64 lowercase hex chars, got {} chars",
        pre_close_sha.len()
    );
    // Sanity-check the citation contract from outside the WAL module:
    // re-derive SHA-256 over the dyn buffer's LE bytes and assert it
    // equals what the SNAPSHOT returned.
    let derived_pre = sha256_le_bytes_hex(&pre_close_buffer);
    assert_eq!(
        derived_pre, pre_close_sha,
        "SNAPSHOT sha256 must equal SHA-256 over the LE-encoded dyn buffer bytes \
         (D-V-A + D-V-C citation contract)"
    );

    // Receipt: GIBBS_SAMPLE must have drifted U off IDENTITY. If U were
    // still all (1,0,0,0) the snapshot would have nothing to test —
    // the replay would round-trip the identity buffer trivially. We
    // need a thermalized state so byte-identity is load-bearing.
    let drifted = pre_close_buffer.iter().enumerate().any(|(i, v)| {
        if i % 4 == 0 {
            (*v - 1.0).abs() > 1e-9
        } else {
            v.abs() > 1e-9
        }
    });
    assert!(
        drifted,
        "GIBBS_SAMPLE must have drifted U off IDENTITY before the snapshot; \
         buffer is all (1,0,0,0) which means the heatbath kernel never ran \
         and the V.4 byte-identity assertion is vacuous"
    );

    // 6. Drop the engine — closes the WAL file. The WAL on disk is the
    //    only state that survives.
    drop(engine);

    // Wipe in-process registries so the reopen path is forced to
    // reconstruct everything from the WAL, not from leftover state.
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();

    // 7. Reopen the engine on the same data dir. `Engine::open` runs
    //    the three-pass replay: lattice declares (Pass 1) → gauge
    //    field declares (Pass 2) → snapshot restorations (Pass 3,
    //    which is the V.3 install path this gate exercises end-to-end).
    let _reopened = Engine::open(&data_path).expect("engine reopen on same data dir");

    // 8. Read the post-reopen buffer through the dyn read registry —
    //    the same surface a fresh GQL caller would hit after restart.
    let post_reopen_buffer: Vec<f64> = gauge_registry::get("U")
        .expect(
            "U must be in the dyn read registry after WAL replay — the V.3 \
             snapshot-restoration pass should have republished the buffer",
        )
        .as_dense_buffer()
        .data
        .clone();

    // 9. Byte-identical Vec<f64> equality. This is the load-bearing
    //    assertion — if the snapshot path lost a single ULP somewhere
    //    (encoding asymmetry, replay reading from a different buffer,
    //    a register clearing the SU(2)-mut handle without republishing
    //    the dyn handle, …) this catches it.
    assert_eq!(
        post_reopen_buffer, pre_close_buffer,
        "post-reopen U buffer must be byte-identical to pre-close U buffer \
         (full chain: declare → thermalize → snapshot → close → reopen → \
         replay-restore on the LE wire); divergence means the snapshot path \
         is lossy somewhere"
    );

    // 10. SHA-256 derivation over the post-reopen buffer must match the
    //     pre-close SHA-256 from the SNAPSHOT Rows envelope. This is
    //     the citation-handle stability contract: a verifier holding
    //     only the SHA-256 from a prior run can confirm the post-
    //     replay state matches the originally-snapshotted state without
    //     ever seeing the buffer itself.
    let derived_post = sha256_le_bytes_hex(&post_reopen_buffer);
    assert_eq!(
        derived_post, pre_close_sha,
        "SHA-256 over post-reopen LE bytes must equal the pre-close SNAPSHOT \
         SHA-256 — the citation handle is the receipt that survives a restart"
    );

    // Cleanup — leave the singletons clean for the next test.
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();
}

// ───────────────────────── V.5 failure-mode helpers ─────────────────────────
//
// The two failure-mode gates below (V.2 checksum rejection, V.3 orphan
// rejection) drive the same end-to-end path as V.4 — `parser::execute`
// against a temp data dir — and then surgically rewrite the on-disk WAL
// bytes between engine close and reopen. The rejection has to fire from
// `Engine::open`'s `replay_gauge_substrate` pass, NOT from the WAL
// reader's CRC32 check, so every byte-surgery test that mutates payload
// content must recompute the CRC tail before re-running `Engine::open`.
//
// The unit-test cousins in `src/engine.rs` (`tdd_hal_v_3_replay_*`) cover
// the same matrix at the in-process level; this file's job is the same
// matrix at the integration-test boundary — the surface a separate crate
// would exercise. The redundancy is load-bearing: integration-level
// rejection proves the `From<WalError> for io::Error` lowering survives
// the lib → integration-test boundary.

/// CRC32 (Castagnoli) — mirror of `wal::crc32` for the byte-surgery
/// failure-mode gates. The implementation is identical to the one in
/// `src/wal.rs`; replicated here because `crc32` is a private function
/// in the WAL module and the failure-mode gates need to recompute it
/// after rewriting payload bytes so the WAL reader's CRC32 check
/// doesn't trip before the V.3 replay-pass checks we actually want to
/// exercise.
fn crc32_for_test(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0x82F6_3B78;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Locate the first WAL entry whose op byte equals `op_target`. Returns
/// `(entry_start, entry_end_excl, payload_start, payload_end_excl)` —
/// the layout the V.5 failure-mode gates need to either rewrite payload
/// bytes (then recompute CRC over `[entry_start..entry_end)` and write
/// it into `[entry_end..entry_end+4)`) or strike out the OP byte.
///
/// Mirrors the private helper of the same shape in `src/engine.rs`;
/// generalised over the op byte so V.5 can locate both the snapshot
/// entry (0x0B) and the declare entry (0x0A) without code duplication.
fn locate_wal_entry(bytes: &[u8], op_target: u8) -> Option<(usize, usize, usize, usize)> {
    let mut offset = 0usize;
    while offset + 4 <= bytes.len() {
        let total_len =
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize;
        let entry_start = offset + 4;
        let entry_end = entry_start + total_len; // op + payload (no CRC)
        if entry_end + 4 > bytes.len() {
            return None;
        }
        let op = bytes[entry_start];
        if op == op_target {
            return Some((entry_start, entry_end, entry_start + 1, entry_end));
        }
        offset = entry_end + 4; // skip CRC tail
    }
    None
}

/// Drive the V.4 four-statement block (LATTICE FROM TRUNCATED_ICOSAHEDRON
/// → GAUGE_FIELD U IDENTITY PERSIST → GIBBS_SAMPLE → SNAPSHOT PERSIST)
/// against the engine at `data_path`. Used by both failure-mode gates so
/// the WAL on disk is in the same shape V.4 produces before each gate
/// performs its byte-surgery step. Caller is responsible for the serial
/// lock and the registry resets.
fn run_v4_chain_then_close(data_path: &std::path::Path) {
    let mut engine = Engine::open(data_path).expect("engine open");
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();

    let stmt = parse("LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';")
        .expect("parse LATTICE FROM TRUNCATED_ICOSAHEDRON");
    let _ = execute(&mut engine, &stmt).expect("exec LATTICE");

    let stmt = parse(
        "GAUGE_FIELD U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY PERSIST;",
    )
    .expect("parse GAUGE_FIELD PERSIST");
    let _ = execute(&mut engine, &stmt).expect("exec GAUGE_FIELD PERSIST");

    let stmt = parse(
        "GIBBS_SAMPLE U BETA 2.5 N_SWEEPS 10 MEASURE_EVERY 1 \
         MEASURE (MEAN(PLAQUETTE)) SEED 20260616;",
    )
    .expect("parse GIBBS_SAMPLE");
    let _ = execute(&mut engine, &stmt).expect("exec GIBBS_SAMPLE");

    let stmt = parse("SNAPSHOT GAUGE_FIELD U PERSIST;")
        .expect("parse SNAPSHOT GAUGE_FIELD U PERSIST");
    let _ = execute(&mut engine, &stmt).expect("exec SNAPSHOT PERSIST");

    drop(engine);
}

/// TDD-HAL-V.5: checksum-rejection gate — corrupt one byte of the
/// snapshot payload's buffer portion in the on-disk WAL, recompute the
/// entry's CRC, reopen the engine; the replay pass must re-derive
/// SHA-256 over the corrupted buffer, see disagreement with the
/// payload's stored hash, and surface `WalError::SnapshotChecksumMismatch`
/// with `name = "U"`.
#[test]
fn tdd_hal_v_2_snapshot_checksum_rejection() {
    let _serial = gauge_registry::test_serial_lock();

    // 1. Temp data dir — same shape as V.4 so the resulting WAL is the
    //    one V.4 produces.
    let dir = tempfile::tempdir().expect("tempdir");
    let data_path = dir.path().to_path_buf();

    // 2. Drive the V.4 chain to populate the WAL with one LATTICE +
    //    one GAUGE_FIELD_DECLARE + GIBBS_SAMPLE residue + one
    //    GAUGE_FIELD_SNAPSHOT entry, then close the engine.
    run_v4_chain_then_close(&data_path);

    // 3. Wipe registries — the rejection has to come from replay
    //    against a corrupted WAL, not from leftover in-process state.
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();

    // 4. Locate the snapshot entry (op = 0x0B) and flip one byte of the
    //    buffer portion. Payload layout (D-V-A):
    //      [u32 name_len][name_bytes][u8 group_tag][32 sha256]
    //      [u32 buf_len][buf_len*8 buffer_bytes]
    //    so the buffer starts at:
    //      payload_start + 4 + name_len + 1 + 32 + 4
    //    For name = "U" (len 1) the buffer starts at payload_start + 42.
    //    We pick the LE byte at the middle of the buffer (offset
    //    180 * 8 = 1440 bytes in, which is the first byte of the 180th
    //    f64) and XOR it with 0xFF — a guaranteed-mutation that
    //    perturbs both the SHA-256 input and the f64's value.
    let wal_path = data_path.join("gigi.wal");
    let mut bytes = std::fs::read(&wal_path).expect("read WAL");
    let (entry_start, entry_end, payload_start, _payload_end) =
        locate_wal_entry(&bytes, 0x0B).expect("snapshot entry in WAL");
    let name_len = u32::from_le_bytes(
        bytes[payload_start..payload_start + 4].try_into().unwrap(),
    ) as usize;
    let buffer_start = payload_start + 4 + name_len + 1 + 32 + 4;
    let buf_byte_count = 90 * 4 * 8; // 90 edges * 4 quat components * 8 bytes/f64
    let corrupt_byte_idx = buffer_start + buf_byte_count / 2;
    assert!(
        corrupt_byte_idx < entry_end,
        "corruption index must land inside the payload range — \
         layout assumption broken (buffer too small?)"
    );
    bytes[corrupt_byte_idx] ^= 0xFF;
    // Recompute the entry's CRC so the WAL reader's CRC32 check passes
    // — the rejection we want to catch is the SHA-256 mismatch in the
    // V.3 replay pass, not the CRC32 in the WAL reader.
    let new_crc = crc32_for_test(&bytes[entry_start..entry_end]);
    bytes[entry_end..entry_end + 4].copy_from_slice(&new_crc.to_le_bytes());
    std::fs::write(&wal_path, &bytes).expect("write corrupted WAL");

    // 5. Reopen — must surface `WalError::SnapshotChecksumMismatch`
    //    through the `From<WalError> for io::Error` lowering. The
    //    Display impl carries the field name and the "SHA-256" tag,
    //    which is what an integration caller would match on.
    let err = match Engine::open(&data_path) {
        Ok(_) => panic!(
            "checksum mismatch on snapshot payload must surface \
             WalError::SnapshotChecksumMismatch — Engine::open returned Ok"
        ),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("'U'") && msg.contains("SHA-256"),
        "Engine::open error must name field 'U' and the SHA-256 \
         rejection category (D-V-C citation contract), got: {msg}"
    );

    // Cleanup — leave the singletons clean for the next test.
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();
}

/// TDD-HAL-V.5: orphan-rejection gate — zero the OP byte of the
/// `OP_GAUGE_FIELD_DECLARE` (0x0A) entry for `U` in the on-disk WAL
/// (effectively deleting it from replay; the reader sees an "Unknown
/// WAL op" on op = 0x00 and bails before the snapshot pass walks the
/// declare into the registry). On reopen, the V.3 snapshot pass must
/// find no registered field named `U` and surface
/// `WalError::OrphanedSnapshot("U")`.
///
/// Implementation note: we DON'T delete the entry (that would re-index
/// every downstream entry's length-prefix offset and we'd have to
/// re-stream the WAL). Instead we strike the OP byte and recompute the
/// CRC. The replay path consumes entries sequentially; an unknown OP
/// returns an `io::Error::Other`-shaped error from `WalReader::read_one`
/// which the engine's `replay_gauge_substrate` surfaces — but ONLY if
/// the snapshot pass is the one that hits it. The cleanest path is to
/// zero the declare's OP byte AND truncate the WAL right after the
/// declare's CRC tail, then write the snapshot back as the next entry.
/// That is too invasive for an integration test; the simpler approach
/// (still load-bearing) is to corrupt the declare's OP byte to an
/// unknown value (0x00) AND recompute its CRC. The WAL reader will
/// surface `Unknown WAL op: 0x0` before the snapshot pass runs at all.
/// That tests "WAL reader rejects unknown op", not "snapshot pass
/// rejects orphan". To exercise the orphan-pass path specifically, we
/// instead REPLACE the declare's OP byte with a SECOND `OP_LATTICE_DECLARE`
/// (0x09) targeted at a name the snapshot's declare looked-up would
/// miss — but the lattice decoder will reject the gauge-shaped payload.
/// Cleanest: truncate the file to the byte range covering only
/// (LATTICE + GIBBS_SAMPLE residue + SNAPSHOT) — i.e. skip the gauge
/// declare entirely. We compute the byte range by locating the gauge
/// declare entry's [4-byte length-prefix..entry_end + 4) span and
/// splicing it out of the byte vector, then writing the rest back.
#[test]
fn tdd_hal_v_3_snapshot_orphan_rejection() {
    let _serial = gauge_registry::test_serial_lock();

    // 1. Temp data dir + V.4 chain to populate the WAL.
    let dir = tempfile::tempdir().expect("tempdir");
    let data_path = dir.path().to_path_buf();
    run_v4_chain_then_close(&data_path);

    // 2. Wipe in-process registries — the rejection has to come from
    //    replay against a hand-edited WAL, not from in-process state.
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();

    // 3. Read the WAL and splice out the `OP_GAUGE_FIELD_DECLARE`
    //    (0x0A) entry — its length prefix, op + payload, and CRC tail
    //    all go. The remaining entries (LATTICE + GIBBS_SAMPLE residue
    //    + SNAPSHOT) replay against a registry that never received the
    //    GAUGE_FIELD declare; the snapshot pass then trips the orphan
    //    check because no handle is registered for name "U".
    let wal_path = data_path.join("gigi.wal");
    let bytes = std::fs::read(&wal_path).expect("read WAL");
    let (entry_start, entry_end, _payload_start, _payload_end) =
        locate_wal_entry(&bytes, 0x0A).expect("gauge declare entry in WAL");
    // The length-prefix sits at [entry_start - 4 .. entry_start); the
    // CRC tail sits at [entry_end .. entry_end + 4). Splice out the
    // whole [entry_start - 4 .. entry_end + 4) range.
    let splice_start = entry_start - 4;
    let splice_end = entry_end + 4;
    let mut spliced = Vec::with_capacity(bytes.len() - (splice_end - splice_start));
    spliced.extend_from_slice(&bytes[..splice_start]);
    spliced.extend_from_slice(&bytes[splice_end..]);
    std::fs::write(&wal_path, &spliced).expect("write spliced WAL");

    // 4. Reopen — the snapshot replay pass must find no registered
    //    handle for "U" and surface `WalError::OrphanedSnapshot("U")`
    //    through the `From<WalError> for io::Error` lowering. The
    //    Display impl carries the field name and the word "orphan",
    //    which is the integration-level match surface.
    let err = match Engine::open(&data_path) {
        Ok(_) => panic!(
            "missing gauge-field declare with a surviving snapshot must \
             surface WalError::OrphanedSnapshot — Engine::open returned Ok"
        ),
        Err(e) => e,
    };
    let msg = err.to_string();
    assert!(
        msg.contains("'U'") && msg.contains("orphan"),
        "Engine::open error must name field 'U' and the orphan \
         rejection category, got: {msg}"
    );

    // Cleanup.
    lattice_registry::clear();
    gauge_registry::clear();
    gauge_registry::clear_e_registry();
}
