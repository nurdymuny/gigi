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
