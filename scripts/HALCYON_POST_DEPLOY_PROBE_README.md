# Halcyon post-deploy probe

Live verification probe for the deployed `gigi-stream.fly.dev` Halcyon
substrate. Fires the canonical Halcyon 200-sweep chain at fixed seed and
checks three receipts against locked targets.

## History

Originally staged as the post-Sprint-B deploy gate. Sprint B (per-face
holonomy cache, audit PURSUE-NEXT) was reverted at `3a5a75e` because its
bit-identity-preserving design had no amortized payoff on the GIBBS_SAMPLE
sweep — the cache was a 2.9x regression and the audit's prediction was
wrong. **Sprint A's `face_edges` hoist (commit `7d8f6e4`) is the deployed
substrate win** the probe now verifies.

## When to run

- **Immediately after deploying Sprint A to `gigi-stream.fly.dev`.** If the
  binary is still the pre-Sprint-A build, the substrate-wall assertion will
  FAIL — that is the intended signal.
- **Before sending any Halcyon reply that cites the deployed substrate.**
  The SHA assertion is the citation contract. If it passes, the one-line
  summary at the end of the probe is pasteable into the reply letter.

## What each assertion catches

| Assertion | Target | Catches |
| --- | --- | --- |
| `probe_wall < 200 ms` (warn), `< 500 ms` (fail) | Sprint A baseline is live (~20 ms substrate + public-internet RTT) | A pre-Sprint-A binary (the old ~25 ms baseline before the `face_edges` hoist), or network congestion. Defaults assume a probe run from outside the VPC; override `-WallTargetMs` / `-WallFailMs` (PS1) or `WALL_TARGET_MS` / `WALL_FAIL_MS` (sh) for loopback or VPC-internal runs (substrate-only target ~25/~50 ms). |
| `MeanPlaquette[199] == 0.535084392992716` | bit-identity at the chain endpoint scalar (chain[199] of the 200-sweep run at fixed seed). **Distinct from** the bench's `final <P> = 0.512543` — that's the post-sweep-200 buffer mean (different measurement, different array slot); the two landing near each other is coincidence of the typical SU(2) thermalization range, not the same number. | RNG drift, sweep-count off-by-one, β decode bug, or a measurement-history index shift. This is the cheaper of the two state receipts; if it passes but the SHA fails, the divergence is somewhere outside the sweep-199 scalar. |
| `snapshot_sha256 == ea7b934c…66516591` | bit-identity over the entire terminal state | Any single-bit drift anywhere in the 360-element f64 buffer (the `n_edges × 4 = 90 × 4` SU(2) quaternion buffer). The SHA is computed over the LE-encoded buffer bytes per locked decision D-V-C. This is the strongest receipt — cryptographic fingerprint of the trajectory's endpoint state, not just one scalar measurement. |

The SHA is Halcyon's value, not gigi's: it lives in the `davis-wilson-lattice`
repo (read-only from gigi) under the `test_G_LIVE_B2` family. The probe asserts
the gigi substrate reproduces it byte-for-byte.

## How to set `GIGI_API_KEY`

The production key is held in the Fly.io app environment, not in this repo.
Extract it once per session:

```powershell
# PowerShell
$env:GIGI_API_KEY = (flyctl ssh console -C 'printenv GIGI_API_KEY').Trim()
```

```bash
# bash
export GIGI_API_KEY="$(flyctl ssh console -C 'printenv GIGI_API_KEY' | tr -d '\r\n')"
```

Optional: override the base URL (defaults to `https://gigi-stream.fly.dev`):

```powershell
$env:GIGI_BASE_URL = "https://gigi-stream.fly.dev"
```

```bash
export GIGI_BASE_URL="https://gigi-stream.fly.dev"
```

## Running

```powershell
# PowerShell on Windows (Bee's default shell)
pwsh ./scripts/halcyon_post_deploy_probe.ps1
```

```bash
# bash (cross-platform; needs curl + jq + python3)
./scripts/halcyon_post_deploy_probe.sh
```

Exit codes:

- `0` — SHIPPED. All three assertions PASS. The script prints a one-line
  summary suitable for pasting into a reply letter to Halcyon.
- `1` — FAIL. At least one assertion failed; the script names which one and
  prints the actual value. No fix is recommended — the probe reports, it does
  not diagnose.
- `2` — environment error (missing `GIGI_API_KEY`, missing `curl`/`jq`/`python3`
  on the bash path, etc.).

## What the probe sends

Four statements, one per `POST /v1/gql` (the endpoint parses a single
`Statement` per body — multi-statement bundles are not supported by the wire
surface today):

```sql
LATTICE buckyball FROM TRUNCATED_ICOSAHEDRON TOPOLOGY 'S2';
GAUGE_FIELD halcyon_canonical_U ON LATTICE buckyball GROUP SU(2) INIT IDENTITY PERSIST;
GIBBS_SAMPLE halcyon_canonical_U BETA 2.5 N_SWEEPS 200 MEASURE_EVERY 1 MEASURE (MEAN(PLAQUETTE)) SEED 20260616;
SNAPSHOT GAUGE_FIELD halcyon_canonical_U PERSIST;
```

The `SNAPSHOT` statement's grammar (locked decision D-V-D — `PERSIST` is
required, bare `SNAPSHOT` rejects):

```ebnf
snapshot_stmt
  : "SNAPSHOT" "GAUGE_FIELD" ident "PERSIST" ";"
  ;
```

String literals are single-quoted (`'S2'`) per the parser convention discovered
in the V.0 probe.

## What the probe reads

From the `GIBBS_SAMPLE` response (`src/parser.rs:9188`):

- `rows[0].MeanPlaquette` — the 200-element measurement chain. Column name
  comes from `ObservableId::MeanPlaquette.label()` at
  `src/gauge/gibbs_sample.rs:117`.
- substrate wall = client-side round-trip time on the `GIBBS_SAMPLE` POST.

From the `SNAPSHOT` response (`src/parser.rs:9640-9660`):

- `rows[0].sha256` — lowercase hex of the SHA-256 over the LE-encoded buffer
  bytes. Same hash the WAL entry carries (`src/wal.rs:144`).
- `rows[0].wal_offset` — byte offset of the `OP_GAUGE_FIELD_SNAPSHOT` (0x0B)
  entry in the WAL file.
- `rows[0].n_edges` — must equal 90 for the buckyball.
- `rows[0].repr_dim` — must equal 4 for SU(2).

## DB-not-pen-pal note

This probe is a tool, not a letter. Output goes into the deploy commit body's
receipts section (or wherever post-deploy receipts live for that cycle). A
one-line PASS/FAIL goes into a reply to Halcyon **only if Bee chooses to send
one** — the probe is not a trigger for correspondence.

## References

- `theory/halcyon/HALCYON_PART_V_SNAPSHOT_GATES.md` — Part V spec (snapshot
  verb, WAL op `0x0B`, locked decisions D-V-A/B/C/D, §5 reproducibility
  contract).
- `theory/halcyon/GIGI_TO_HALCYON_REPLY_2026-06-19.md` — closes Halcyon's
  §7 open questions; §2 locks LE encoding and SHA-over-buffer.
- `tests/halcyon_part_v_snapshot.rs` — reference implementation; the same
  4-statement chain, end-to-end SHA-256 verification, byte-identity replay.
- `tests/halcyon_part_v_p1_gql_dispatch.rs` — V.0 probe; the parser convention
  for `'S2'` single-quoted literals comes from here.
