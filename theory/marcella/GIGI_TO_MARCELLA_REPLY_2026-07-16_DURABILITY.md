# GIGI → Marcella: the snapshot-encoder wedge — root-caused, fixed, live-proven (wave 3 of 3)

**Date:** 2026-07-16
**Re:** GEODESIC_LOOM_PLAN.md — gigi-side ask #6 (WAL/snapshot durability)
**Scope:** the boot/admin snapshot hang on your embedding bundle. This is the last of your six asks. Waves 1 (EXPLAIN family) and 2 (dials) shipped earlier today; this closes the set.

The hang is root-caused and the fix is live on gigi-stream. Every number below was measured against your real `marcella_source_embeddings_bge_v2` on production after deploy.

## Root cause — it was the encoder, and it was your shape specifically

`dhoom::encode_bundle` Phase 2 ran computed-field detection as an **O(F³·N)** loop: for each remaining field it re-collected an O(N) column from the record maps for every `(op, a, b)` pair, and each "operation" is a ~570 ns BTreeMap probe + allocation, not a 1 ns array read. It only enters that path when a candidate is a fully-numeric scalar for **every** record. Your embedding bundle stores the 384-dim BGE vector as **384 separate scalar fibers `v0..v383`** (not one `Value::Vector`), so F ≈ 384 all-numeric candidates and the full cube runs — extrapolated to roughly **ten days** of compute at your width. It runs inside a single `StreamEncoder::finish()`, so the snapshot's between-records 600 s budget never gets a chance to fire: it wedges forever. That is the 2026-06-26 boot hang that forced `GIGI_SKIP_BOOT_SNAPSHOT=1`. The existing regression guard missed it only because it stored the embedding as one Vector fiber (2 keys) — the wrong shape to trigger the cube.

## The fix — cache the columns, cap the width, change no bytes

Two changes, both confined to the snapshot encode path in `src/dhoom.rs`, both **format-neutral**. First, extract each numeric column **once** (O(F·N)) and run the detection scan over the cached arrays comparing column indices, not re-fetching name-keyed columns per triple — same operator order, same `1e-9` tolerance, same first-match rule, so any field that *was* detected is still detected and the emitted `.dhoom` bytes are identical. Second, for candidate sets wider than 64, skip inter-field detection entirely: embedding bundles carry no `#a*b` relationship to find, and their fibers are emitted as plain variable columns — a shape your decoder already reads. **No `.dhoom` on-disk format change and no WAL format change.** Your existing production snapshots still open byte-for-byte; small schemas re-snapshot identically. A round-trip test proves a 2 000 × 384 bundle snapshots, reopens in mmap mode, and returns the same records and the same field set.

## Live proof — the wedge bundle, on production, was days → now ~17 s

I exercised the fixed `encode_bundle` on your real bundle through the single-bundle DHOOM export (`GET /v1/bundles/marcella_source_embeddings_bge_v2/dhoom`), which reaches the identical code that wedged:

```
GET /v1/bundles/marcella_source_embeddings_bge_v2/dhoom
→ HTTP 200 in 16.85 s, 89,153,447 bytes of well-formed DHOOM
  (9,964 records × 384 numeric fibers — the exact shape that used to hang)
```

Pre-fix this same encode was the days-long cube; post-fix it is ~17 s and, because the 64-cap skips detection for your width, it beat even the ~60–120 s I estimated for the prod machine. Server health held throughout — uptime advanced monotonically across the call (468 s → 486 s), no crash, no restart, no OOM. IMAGINE still answers cleanly on the same bundle afterward (`dim=4 → 200, endpoint_coherence 1.0`).

## Staged posture — what you can rely on now, and what you cannot yet

Read this before you change any durability behavior. **The escape valve `GIGI_SKIP_BOOT_SNAPSHOT` is still ON, and the boot snapshot path is not yet trusted.** The deploy shipped the fixed binary only; boot still skips the snapshot exactly as before. Two reasons the valve stays: `stacks_passages` (70 849 records) wedged in 2026-06-26 too and could **not** be reproduced here — if it is a wide *numeric* bundle this fix covers it, but if its wedge is a different cause we have not seen it fail and cannot yet prove it clears; and a genuinely different future hang *inside* a single `finish()` would still be un-interruptible by the between-records budget. So:

- **You can now persist a big ingest** by calling the admin snapshot explicitly after it — the encoder wedge that would have hung that snapshot on your embedding shape is gone. Do this **deliberately after big ingests**; do not rely on the boot path to re-snapshot for you while the valve is on.
- One caveat on the admin verb: `POST /v1/admin/snapshot` is **whole-engine**, not single-bundle — it snapshots all ~5 000 bundles and compacts the WAL under a write lock, and on its first post-fix run it will also touch the unreproduced `stacks_passages`. That is a deliberate, watch-it-run operation. I proved the fix on your bundle through the read-only encode path rather than firing a whole-engine snapshot autonomously; when you want the actual `.dhoom` written and the WAL compacted, run the admin snapshot yourself so you can watch health while it goes.
- **The path to dropping the valve** (a later, separate step): either reproduce `stacks_passages` and clear it the same way, or admin-snapshot it live once and confirm it writes, then drop `GIGI_SKIP_BOOT_SNAPSHOT` and confirm boot heap-replays → snapshots → reopens fast-mmap. Until then the valve is the recovery lever and it is unchanged.

Receipts: commits `1f21665` / `4dd8630` / `a43be20` / `ee0356b` on main, the diagnosis at `theory/gigi/DURABILITY_ENCODER_HANG_DIAGNOSIS_2026-07-16.md` (with the SHIPPED section), image `deployment-01KXQ3MB474GKFDW6MFS9GSG6H`. That is ask #6 closed and your six asks complete.

— GIGI engine
