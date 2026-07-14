# Optionality Contract Ledger

**Purpose.** Pin the commit SHA, toolchain, and cargo-test witnesses for
the optionality contract of Chapter 25, Theorem 25.1 (*GIGI Thinks*):
byte-identical wire responses on the non-Kähler-touching API surface
between the flag-on and flag-off builds.

**Contract.**
- `cargo test --no-default-features` → all tests passing, zero failures.
- `cargo test --features kahler` → all tests passing, zero failures.
- Wire responses on the non-Kähler-touching command set match
  byte-for-byte across the two builds.

The historical instance of this contract (2026-05, documented in
`theory/kahler_upgrade/marcella_substrate.md:4-6`) was 720 passing
flag-off / 902 passing flag-on. The suite has grown since; the invariant
is zero failures on both sides plus wire byte-identity, not the totals.

## Pinned pair

| Field                     | Value                                    |
|---------------------------|------------------------------------------|
| Flag-off SHA              | `00e35f9d7330dc6236a7647883a4abbf1ef95378` |
| Flag-on SHA               | `00e35f9d7330dc6236a7647883a4abbf1ef95378` (same tree, features differ) |
| Rust toolchain            | rustc 1.92.0 (ded5c06cf 2025-12-08)      |
| `cargo test --no-default-features` total | **1213 passed, 0 failed** (152 suites) |
| `cargo test --features kahler` total     | **1675 passed, 0 failed** (159 suites) |
| Flag-off `cargo build` digest | OPEN — provenance-only per Scope; not recorded this round |
| Flag-on `cargo build` digest  | OPEN — provenance-only per Scope; not recorded this round |
| Wire-response corpus digest (non-flagged) | **OPEN — corpus replay not yet run; the contract's byte-identity clause is not discharged until this row fills** |
| Verified at               | 2026-07-14 (local run; both suites executed back-to-back on the same checkout) |

## Reproduction procedure

1. Check out the pinned SHA.
2. `cargo test --no-default-features` --- record total.
3. `cargo test --features kahler` --- record total.
4. Replay the fixed non-flagged command set against each build's
   `gigi serve` binary; capture the wire responses.
5. `sha256sum` the two response corpora; the digests must match.

## Scope

The ledger pins **responses**, not compiled artifacts. Byte-identity
of `cargo build` outputs additionally requires reproducible-build
machinery (deterministic linker, pinned `SOURCE_DATE_EPOCH`) which
the substrate does not ship by default. The `cargo build` digest
above is recorded for provenance only; the load-bearing digest is
the wire-response corpus digest.

## Notes

- This ledger is referenced by chapter 25 of *GIGI Thinks* Volume 2
  (receipt `THINK-CH25-OPTIONALITY-CONTRACT`) as the ship-gate for
  the chapter's Theorem 25.1. The chapter's proof-status paragraph
  states plainly that the wire-corpus row is open.
- Raw `test result:` lines for both runs were captured from
  `cargo test` stdout on 2026-07-14; totals are the sums across all
  suite result lines (152 suites flag-off, 159 flag-on).
