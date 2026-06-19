# Halcyon forward-looking ask — SNAPSHOT_EVERY on SYMPLECTIC_FLOW

**Received** 2026-06-19 from Halcyon (follow-up to Part V, low priority,
explicitly non-blocking).

**Receipt from Halcyon side**: `--use-gigi` flag wired end-to-end against
`gigi-stream.fly.dev` — Phase F0 PASS in 30.71 s. Sprint A is on the wire
and Halcyon is consuming it from the orchestrator.

## The ask

SYMPLECTIC_FLOW currently returns scalar measurement chains plus the final
field handle. It does NOT emit per-step buffer snapshots, so anything that
wants per-frame state (e.g. Halcyon's `--sector-classifier` walking per-frame
U_i to bin by Q_surrogate) stays in the Python kernel under `--use-gigi`.

The ask: add a `SNAPSHOT_EVERY k` clause to SYMPLECTIC_FLOW with the
same parser shape as the existing `MEASURE_EVERY` arm. Every k steps the
substrate would write OP_GAUGE_FIELD_SNAPSHOT (0x0B) to the WAL and emit
the resulting SHA-256 + wal_offset into the response Rows envelope. The
orchestrator could then retrieve per-frame state via
`GET /v1/gauge_field/{name}` keyed by the returned SHAs.

End state: `--sector-classifier` runs substrate-side instead of in the
Python kernel under `--use-gigi`. Retires the last Python-kernel hot
path in the Halcyon verifier loop.

## What's mechanical

- Parser: `SNAPSHOT_EVERY` token + grammar arm modeled on `MEASURE_EVERY`
  in `src/parser.rs` Statement::SymplecticFlow. ~30 LOC of parser +
  AST field + executor plumbing.
- Executor: at every k-th KDK step, clone the dense buffer, compute
  SHA-256, append OP_GAUGE_FIELD_SNAPSHOT to the WAL, push the
  (sha256, wal_offset) pair into a `SnapshotTrajectory` Vector column
  of the Rows envelope.
- Response shape: extend the existing SYMPLECTIC_FLOW Rows envelope with
  one new column. Other columns unchanged.

## Two design questions worth pinning before any sprint

### D-VI-A — WAL growth budget

Per-step snapshot at k=1 on a 1000-step flow = 1000 × OP_GAUGE_FIELD_SNAPSHOT
records, each ~3 KB (name + group + 360 f64 buffer + 32-byte SHA). That's
~3 MB of WAL growth per flow at k=1. Real concerns:

1. Does Halcyon's classifier actually want k=1, or is k=10 / k=50
   sufficient for the binning resolution? At k=50 on a 1000-step flow,
   that's 60 KB of WAL — negligible.
2. Should SNAPSHOT_EVERY default to TRANSIENT (heap-only) and require an
   explicit PERSIST keyword like the bare SNAPSHOT verb does (D-V-D)?
   TRANSIENT would skip WAL entirely; orchestrator would have to drain
   the buffers in the same request/response cycle. That preserves the
   WAL surface area but moves the load to in-memory throughput.

### D-VI-B — retrieval surface

Halcyon's proposal: `GET /v1/gauge_field/{name}` keyed by SHA. Two
sub-shapes:

1. **Name + SHA filter**: `GET /v1/gauge_field/{name}?sha={hex}` — looks
   up the snapshot by name AND SHA, returns the buffer. Reuses existing
   route; adds query parameter. Requires the engine to index snapshots
   by (name, sha) — currently they're indexed by (name, wal_offset).
2. **SHA-only global lookup**: `GET /v1/gauge_snapshot/{hex}` — looks up
   by SHA globally. New route. Cleaner semantics (SHA is collision-free
   over the buffer; you don't actually need the name to disambiguate).

(2) is closer to the "SHA is the citation handle" framing already locked
in D-V-C. (1) is closer to existing REST shape on the gauge_field
namespace. Either works.

## Recommendation

Park this until Halcyon either (a) escalates from "low priority" to a
concrete sprint trigger (chapter copy needs it, or the verifier flow
hits a latency wall in the Python-kernel sector-classifier path), or
(b) Bee greenlights it as a slot during a future Halcyon coordination
window. The parser arm is mechanical, but the two design questions
above should be answered with Halcyon before any implementation, not
after (last time we did "after" with the audit's PURSUE-NEXT prediction
and ate the revert).

## Status

- Logged: yes (this doc)
- Bee greenlit: not yet (pending)
- Sprint slot: not assigned
- Halcyon-side blocking pressure: none (`--use-gigi` works without it;
  sector-classifier stays in Python kernel until this lands)

## References

- `theory/halcyon/HALCYON_PART_V_SNAPSHOT_GATES.md` — the OP 0x0B
  snapshot infrastructure this would extend.
- `theory/halcyon/THERMALIZATION_AUDIT_2026-06-19.md` — Sprint B revert
  lesson: do not bypass bit-identity on Halcyon-coordinated paths.
- `src/parser.rs` Statement::SymplecticFlow + Statement::GibbsSample —
  the `MEASURE_EVERY` arm SNAPSHOT_EVERY would mirror.
- Halcyon's verifier `--sector-classifier` flag — current consumer of
  per-frame U_i state.
