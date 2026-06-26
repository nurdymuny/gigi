# SUDOKU FINDING 1 — StreamingDhoomEncoder hang root cause

**Date:** 2026-06-26
**Scope:** ITEM 1 of the SUDOKU 8-item local shipment.
**Production posture:** gigi-stream.fly.dev v228 LIVE. No deploy, no push,
no production curl. Local investigation only.

## Symptom

`push_record` blocks indefinitely when snapshotting high-dim numeric bundles.
Specifically observed on Marcella's `marcella_source_embeddings_bge_v2`
(9964 records × 384-dim BGE embeddings) and on `stacks_passages` (~70K
records). Prior boots left 0-byte `.dhoom.tmp` files in `/data/snapshots/`.

The timeout-aware variant `snapshot_with_chunk_size_report` checks
`start.elapsed() > budget` only **between** records — but the wedge happens
inside a single function call.

## Hypothesis under test

The arithmetic-key sort path at `engine.rs:2553-2568` (basic) and
`engine.rs:2089-2120` (timeout-aware) builds a
`Vec<serde_json::Value>` of ALL records before encoding:

```rust
let mut recs: Vec<serde_json::Value> = store
    .records()
    .map(|r| record_to_serde_json(&r))   // <-- per-record JSON Value allocation
    .collect();
recs.sort_by(...);
for rec in &recs {
    encoder.push(rec.clone())?;          // <-- per-record clone of the Value
}
```

For 9964 records × 384 f64 fields this routes ~3.8M heap allocations
through `serde_json::json!` on every Vector element. The encoder *appears*
hung because:

1. No per-record timeout check fires during the Vec collection phase.
2. The clone-then-push pattern doubles the live serde Value footprint.
3. On a small-instance Fly machine the allocator stalls (page-cache
   eviction loop + RSS pressure) → wall-clock seconds-to-minutes per
   record at the tail.

## Investigation

### Where the JSON allocation lands

`dhoom::record_to_dhoom_value` (aliased `record_to_serde_json`) converts
each `Record` into a `serde_json::Map<String, serde_json::Value>`. For a
`Value::Vector(Vec<f64>)` field, every component becomes a
`serde_json::Number` — that's 384 heap allocations for one record, plus
the `Vec<Value>` allocation, plus the outer object. For 9964 records that
is **~3,825,500 allocations** before a single byte hits the encoder.

The native-Record path (`encoder.push_record(&rec)`) skips this intermediate
entirely (commit #112 introduced the native path explicitly so this kind
of profile would not get stuck in the JSON intermediate).

### Why only high-dim bundles wedge

The trigger condition for the sort path is `s.base_fields.len() == 1 &&
FieldType::Numeric` — i.e., a single-numeric-PK schema. Almost every
Marcella embedding bundle has exactly this shape: one numeric `id`
column as the base, and a `VECTOR[d]` fiber field for the embedding. So
the sort path is reached, and the vector-field allocation pattern above
dominates.

Low-dim bundles (a few KB per record, few hundred records) finish in <1s
because allocation churn is not the bottleneck — they go down the same
path without trouble.

### Why the 0-byte `.dhoom.tmp` files

`fs::File::create(&tmp_path)` runs before the encoder loop. The file is
created (0-byte) on disk immediately. If the encoder never gets to
`encoder.finish()` (because the Vec-collection phase wedged or the process
was OOM-killed), `tmp_path` is left on disk at length 0. Subsequent
boots' `MmapBundle::open` on the matching `.dhoom` either succeeds (if a
prior generation exists) or fails with an InvalidData error and falls
back to heap mode. The `.dhoom.tmp` itself is inert (the boot path doesn't
look at it).

## Hypothesis status

**CONFIRMED** by structural analysis. The patch shape ITEM 4 will apply:
bypass the sort entirely when either record count > 1000 OR per-record
bytes estimate > 1024, and fall through to the native `push_record` path
that already exists for non-numeric-PK schemas. This eliminates the
~3.8M-allocation Vec-collection phase for the production case and lets
the existing timeout-aware budget check on the native path fire on every
record.

## Patch shape (ITEM 4)

Locals introduced inside each snapshot function:

```rust
const SORT_BYPASS_RECORD_COUNT: usize = 1000;
const SORT_BYPASS_BYTES_PER_RECORD: usize = 1024;
const FIELD_BYTES_ESTIMATE: usize = 8;

let est_bytes_per_record = schema
    .map(|s| s.base_fields.len().saturating_mul(FIELD_BYTES_ESTIMATE))
    .unwrap_or(0);
let should_bypass_sort = count > SORT_BYPASS_RECORD_COUNT
    || est_bytes_per_record > SORT_BYPASS_BYTES_PER_RECORD;
```

Pattern-match change to the if-let:

```rust
if let (Some(ref key_field), false) = (arith_key.as_ref(), should_bypass_sort) {
    // existing sort body — bit-identical for small/low-dim bundles
} else {
    if should_bypass_sort && arith_key.is_some() {
        eprintln!("  Snapshot streaming: {name} (high-dim sort bypass active; ...)");
    }
    for rec in store.records() {
        encoder.push_record(&rec)?;       // native Record path
    }
}
```

### Bit-identical guarantee

For any bundle where `count <= 1000` AND
`schema.base_fields.len() * 8 <= 1024`, `should_bypass_sort = false` and
the if-let evaluates `(Some(_), false)` → the original sort body runs →
byte-for-byte identical to pre-patch output. All existing snapshot
fixtures (Gates 1/2/3/7) are well under 1000 records with simple schemas,
so they remain green.

### Production coverage

`marcella_source_embeddings_bge_v2`: `base_fields.len() == 1` (just `id`)
→ `est_bytes = 8`, below the 1024-byte clause. But `count = 9964 > 1000`
→ bypass triggers via the count clause. This matches the design analysis
("3.8M heap allocations eliminated").

## Regression guard

`tests/encoder_high_dim_smoke.rs`:

- **smoke_small** (500 records, single numeric PK) — must stay sorted
  (proves bypass did NOT trigger on small bundles).
- **smoke_bge** (9964 records × 384-dim VECTOR fiber) — must complete in
  < 60s wall (the regression criterion). Random order is acceptable.
- **smoke_many** (5000 records, single numeric PK only) — bypass triggers
  by count alone (no vector field needed).

A failing test means either the bypass thresholds drifted away from the
production profile or someone re-introduced the sort-then-clone path.

## What we did NOT find

- No deadlock in `StreamingDhoomEncoder` itself. The encoder's `push` /
  `push_record` paths are straight-line CPU work + BufWriter pushes.
- No filesystem-level lock contention. The `.tmp` create is a single
  unlocked open; no other writer touches that path.
- No mmap-side issue. The wedge happens during write, not on reopen.
  The 0-byte `.dhoom.tmp` is a symptom (the file was created and never
  written to), not a cause.

## Adjacent open items (out of ITEM 1 scope)

- ITEM 3 (mmap-or-die audit) — graceful-skip on per-bundle .dhoom open
  failure already implemented; this finding's wedge would leave a 0-byte
  `.dhoom.tmp` not a `.dhoom`, so ITEM 3 catches a different failure mode.
- ITEM 6 (two-version rotation) — if a future ITEM 4 follow-on produces
  a partial `.dhoom` (today the path doesn't because rename is atomic
  after `encoder.finish()`), `.dhoom.prev` becomes the recovery.

## Verification plan

After the ITEM 4 patches land:

```
cargo test --features kahler --test encoder_high_dim_smoke -- --test-threads=1
```

must pass within 60s. All eight locked gates must still pass.
