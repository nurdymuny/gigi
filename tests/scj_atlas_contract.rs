//! Contract tests for the Shadow Clone Jutsu (SCJ) Windows Atlas on Gigi
//! v0.1 ingest target.
//!
//! Background. SCJ is Gigi's third major downstream consumer (after Marcella
//! and KRAKEN) and is about to ingest the full Windows binary corpus — every
//! function in ntoskrnl, win32k*, the Hyper-V family, system drivers, system
//! DLLs — as three bundles keyed by `(module, rva)` with the SCJ feature
//! ontology and the GASM map scalar fields as fiber data.  See:
//!
//!   `theory/scj/REPLY_TO_LETTER_2026-06-05.md`        — our reply to their letter
//!   `theory/scj/REPLY_TO_REPLY_2026-06-06.md`         — SCJ ack + contract close
//!   `theory/scj/REPLY_TO_REPLY_2_2026-06-07.md`       — Gigi 4-row partition reply
//!   `theory/scj/REPLY_TO_REPLY_3_2026-06-07_CLOSE.md` — Gigi round-6 close ack
//!   `theory/scj/REPLY_FROM_SCJ_2026-06-06.md`         — SCJ commitment letter
//!   `theory/scj/REPLY_FROM_SCJ_2026-06-07.md`         — SCJ counter-correction
//!   `examples/scj_atlas/README.md`                     — runnable BUNDLE DDLs live here
//!
//! What this file gates.  Five tests total, divided by what upstream gate
//! they're waiting on — named honestly per SCJ 2026-06-07 close drift #2:
//!
//!   3 tests gated on SCJ deliverable 2A (SCJ ships DDLs + smoke pipeline).
//!     Each is `#[ignore]`'d AND `#[cfg(feature = "sharded")]` because they
//!     wire through sharded code paths once live.
//!       (a) `ddls_parse_against_scj_v01_substrate`
//!       (b) `dhoom_roundtrip_is_byte_identical_on_synthetic_vid_sys`
//!       (c) `similar_top10_is_run_to_run_deterministic`
//!
//!   1 test gated on engine-side TAGSET ship (Ask A roadmap).  Also
//!     `#[ignore]`'d AND `#[cfg(feature = "sharded")]`.
//!       (d) `tagset_shadow_encoding_equivalent_to_eventual_contains_any`
//!
//!   1 test live now — the instant-distance version pin.  Intentionally
//!     NOT under any cfg guard: bumping `instant-distance` is a substrate
//!     contract change with SCJ, and we want that guard to trip on every
//!     `cargo test` invocation including the default-features no-sharded
//!     CI run.  Prior versions of this file gated the whole module on
//!     `#![cfg(feature = "sharded")]`, which silently disabled the pin
//!     guard on default CI.
//!       (e) `instant_distance_version_pin_is_stable`
//!
//! When deliverable 2A lands, flip the `#[ignore]` off tests (a)/(b)/(c),
//! point the path constants at the dropped files, and the 2A contract
//! turns live.  Test (d) flips when TAGSET ships engine-side.

use std::path::PathBuf;

// ─── Helpers used by the four `sharded`-gated tests below.  Per-item ────
// cfg-guarded to avoid `dead_code` warnings on default-features builds
// (where only the pin guard at the bottom of this file is live).

/// Where SCJ's three BUNDLE DDLs will live once they're dropped.
#[cfg(feature = "sharded")]
const DDL_DIR: &str = "examples/scj_atlas";

#[cfg(feature = "sharded")]
const WINDOWS_FNS_DDL: &str = "windows_fns.gql";
#[cfg(feature = "sharded")]
const WINDOWS_CALLS_DDL: &str = "windows_calls.gql";
#[cfg(feature = "sharded")]
const WINDOWS_SINKS_DDL: &str = "windows_sinks.gql";

#[cfg(feature = "sharded")]
fn ddl_path(name: &str) -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push(DDL_DIR);
    p.push(name);
    p
}

/// Returns true iff all three DDLs are present.  Used to keep the test
/// suite green before SCJ drops their files.
#[cfg(feature = "sharded")]
fn ddls_present() -> bool {
    ddl_path(WINDOWS_FNS_DDL).exists()
        && ddl_path(WINDOWS_CALLS_DDL).exists()
        && ddl_path(WINDOWS_SINKS_DDL).exists()
}

/// (a) — Parse-clean gate.  Each DDL must parse against the frozen
/// `scj-v0.1-substrate` grammar with no hand-edits.
#[cfg(feature = "sharded")]
#[test]
#[ignore = "SCJ ddls not yet dropped — gated on SCJ deliverable 2A (REPLY_TO_REPLY_2026-06-06.md §2A)"]
fn ddls_parse_against_scj_v01_substrate() {
    if !ddls_present() {
        eprintln!(
            "skipping: SCJ DDLs absent under {DDL_DIR}/. Deliverable 2A. \
             See theory/scj/REPLY_FROM_SCJ_2026-06-06.md §2A."
        );
        return;
    }
    // TODO(scj-2A): wire to crate::parser::parse_bundle_schema once the
    // DDLs land. Each DDL must parse, schema-validate, and produce a
    // BundleSchema with the expected fiber field set per the Atlas spec.
    panic!("scj-v0.1: DDLs present but parser-wiring not yet implemented.");
}

/// (b) — Round-trip byte-identical gate.  schema → DHOOM emit → re-ingest
/// must produce the same DHOOM bytes on vid.sys-scale synthetic data.
/// This is the safety gate for SCJ's per-shard rebuild discipline.
#[cfg(feature = "sharded")]
#[test]
#[ignore = "SCJ ddls not yet dropped — gated on SCJ deliverable 2A (REPLY_TO_REPLY_2026-06-06.md §2A)"]
fn dhoom_roundtrip_is_byte_identical_on_synthetic_vid_sys() {
    if !ddls_present() {
        return;
    }
    // TODO(scj-2A): generate ~1850-record synthetic at vid.sys scale,
    // emit DHOOM, re-ingest into a fresh BundleStore, compare. Match
    // determinism harness for the existing kahler tour.
    panic!("scj-v0.1: round-trip harness not yet implemented.");
}

/// (c) — SIMILAR determinism gate.  Three identical runs of
/// `SIMILAR windows_fns TO ... ON embedding TOP 10` must return the same
/// 10 records in the same order, against 2K × 128-d synthetic embeddings.
/// This is what SCJ's SUSANOO top-10 reproducibility depends on.
#[cfg(feature = "sharded")]
#[test]
#[ignore = "SCJ ddls not yet dropped — gated on SCJ deliverable 2A (REPLY_TO_REPLY_2026-06-06.md §2A)"]
fn similar_top10_is_run_to_run_deterministic() {
    if !ddls_present() {
        return;
    }
    // TODO(scj-2A): build a deterministic 2K × 128-d synthetic embedding
    // corpus with a planted "SUSANOO" anchor + 9 known-near "TSUKUYOMI"-class
    // neighbors. Issue SIMILAR TOP 10 three times against a freshly-built
    // HNSW; assert identical result sets and identical ordering.
    //
    // This wires through pre-cluster + HNSW recall gates per Ask C in
    // the SCJ correspondence. instant-distance v0.6 is the pinned backend.
    panic!("scj-v0.1: SIMILAR determinism harness not yet implemented.");
}

/// (d) — TAGSET shadow-encoding equivalence gate.  Per SCJ's 17-boolean
/// shadow encoding for v0.1 (Ask A), `COVER WHERE reaches_<sink> = true`
/// must return the same record set as the eventual
/// `COVER WHERE sinks_reached CONTAINS_ANY [<sink>]` v0.2 form.
///
/// Lands when TAGSET ships engine-side; until then, this is the
/// safety-net that lets SCJ migrate without re-validating their query
/// surface byte-by-byte.
#[cfg(feature = "sharded")]
#[test]
#[ignore = "TAGSET type not yet shipped engine-side — gated on Ask A engine-side roadmap, NOT on SCJ 2A"]
fn tagset_shadow_encoding_equivalent_to_eventual_contains_any() {
    eprintln!("skipping: TAGSET type not yet shipped — see Ask A in correspondence.");
}

/// (e) — `instant-distance` version pin assertion.  SCJ pins the exact
/// instant-distance major.minor as part of their requirements freeze.
/// If this assertion ever fails, our Cargo.toml moved and SCJ needs to
/// be notified before they rebuild HNSWs against a new graph version.
///
/// **Intentionally NOT under `#[cfg(feature = "sharded")]`.**  The pin
/// is a substrate contract with SCJ, and we want this guard to trip on
/// every `cargo test` — including the default-features no-sharded CI
/// run.  Prior versions of this file gated the entire module on
/// `#![cfg(feature = "sharded")]`, which silently disabled this guard.
/// SCJ caught the silent-disable in their 2026-06-07 verification pass;
/// drift #2 in `theory/scj/REPLY_TO_REPLY_3_2026-06-07_CLOSE.md`.
#[test]
fn instant_distance_version_pin_is_stable() {
    // Hard-coded match against Cargo.toml. Bumping instant-distance is
    // a substrate contract change; touching this test forces a deliberate
    // SCJ-notification step before the bump lands.
    let pinned = "0.6";
    let cargo_toml = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("Cargo.toml readable");
    let line = cargo_toml
        .lines()
        .find(|l| l.trim_start().starts_with("instant-distance"))
        .expect("instant-distance dependency present in Cargo.toml");
    assert!(
        line.contains(&format!("\"{pinned}\"")),
        "instant-distance version drift detected. Was pinned at {pinned}, \
         now reads: `{line}`. Bumping this is a substrate contract change — \
         see theory/scj/REPLY_TO_REPLY_2026-06-06.md §1A before changing."
    );
}
