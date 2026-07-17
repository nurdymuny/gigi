# Durability snapshot-encoder hang — diagnosis + fix (worktree only)

**Date:** 2026-07-16
**Status:** ROOT-CAUSED, FIXED IN WORKTREE, GATED. **NOT merged, NOT pushed, NOT deployed.**
**Branch:** `worktree-wf_c5993b15-5ef-2` (base `main` @ `fdf1af31050b79bfabb7d43c4ebfaee36ebb2358`)
**Reproduced locally:** yes (pure-heap bundle; no mmap backing required).

This is the un-root-caused hang from `reference_gigi_durability_layer` — the boot
heap-replay snapshot that wedged on `marcella_source_embeddings_bge_v2` and
`stacks_passages` on 2026-06-26, forcing `GIGI_SKIP_BOOT_SNAPSHOT=1` and keeping the
machine on the ~13–25 GB heap path instead of the ~200 MB fast-mmap golden path.

---

## 1. Root cause

**One line:** `dhoom::encode_bundle` Phase 2 (`src/dhoom.rs`) ran computed-field
detection as an **O(F³·N)** loop over the record maps — and each "operation" is a
~570 ns `BTreeMap` probe + heap allocation, not a 1 ns array read — inside a single
`StreamEncoder::new` call that the snapshot's between-records timeout cannot interrupt.
At embedding width (F ≈ 385 numeric fields) that is **days** of compute.

### The exact code (pre-fix)

`encode_bundle` Phase 2 called `detect_computed_field` once per remaining key:

```rust
// Phase 2: detect computed fields among remaining keys
for key in remaining_keys.clone() {                       // F iterations
    if let Some(expr) = detect_computed_field(&key, records, &remaining_keys) {
        computed_fields.push((key.clone(), expr));
    }
}
```

and `detect_computed_field` (pre-fix) re-collected the field columns **inside** its
`op × a × b` triple loop:

```rust
for op in &['*', '+', '-'] {                              // 3
    for a in candidate_keys {                             // F
        let a_vals: Vec<f64> = records.iter()             // O(N) map-lookups + alloc
            .filter_map(|r| r.as_object()?.get(a.as_str())?.as_f64()).collect();
        for b in candidate_keys {                         // F
            let b_vals: Vec<f64> = records.iter()         // O(N) map-lookups + alloc — EVERY triple
                .filter_map(|r| r.as_object()?.get(b.as_str())?.as_f64()).collect();
            // compare a_vals[i] op b_vals[i] == values[i] (early-breaks for random data)
```

Per call: ~`3·F²` full `Vec<f64>` re-collections, each O(N). Phase 2 makes F calls ⇒
**O(F³·N) map-lookups**. The `b_vals` collect happens *before* the length check and
*before* the early-breaking compare, so nothing short-circuits it.

### Why the timeout can't fire

The call chain that reaches it:

```
boot heap-replay (gigi_stream.rs:16231)  engine.snapshot_with_report()
  -> engine.rs:2327  snapshot_with_chunk_size_report(50_000, budget=Some(600s))
       for rec in store.records() { if start.elapsed() > budget { timed_out } ; encoder.push_record(&rec) }   // budget checked BETWEEN records
       encoder.finish()                                                                                       // <- the flush lands HERE
  -> dhoom StreamingDhoomEncoder::finish -> flush_chunk
  -> StreamEncoder::new(&chunk)      // chunk = ALL buffered records (a <50k bundle buffers entirely)
  -> encode(&wrapped) -> encode_bundle -> Phase 2 -> detect_computed_field   // the O(F³·N) cube
```

For any bundle smaller than the 50 000 `chunk_size`, every `push_record` merely buffers;
the single `flush_chunk` runs inside `encoder.finish()`, **outside** the budget-guarded
loop. So `start.elapsed() > budget` is never re-evaluated while the cube runs. The
`Some(600)` budget is real but unreachable — exactly the "timeout fires only BETWEEN
records" hazard the durability note flagged.

### Why it fires on the embedding bundle specifically

`detect_computed_field` only enters the expensive path when a candidate's `.as_f64()`
succeeds for **every** record — i.e. the field is a numeric scalar. Production
`marcella_source_embeddings_bge_v2` stores the 384-dim BGE embedding as **384 SEPARATE
scalar fibers** `v0..v383` (+ `id` + `ts`) — all numeric — so F ≈ 385 and the full cube
runs. The existing regression guard `tests/encoder_high_dim_smoke.rs::smoke_bge` stored
the embedding as **one `Value::Vector` fiber** (2 keys); `.as_f64()` on the array returns
`None`, the detector early-returns, and the cube never runs. **That shape mismatch is
why the wedge slipped past the existing test.**

`stacks_passages` (70 849 records, different schema) wedged in the same `encode()` — the
cost is shared encoder infrastructure, not a per-bundle quirk. (If it is text-heavy its
wedge may be a different cause; its schema was not available to repro here — see §5.)

---

## 2. Repro receipt

Synthetic bundle mirroring the production shape: `id` base + high-variance `ts` fiber +
`v0..v(n-1)` separate scalar float fibers, inserted then snapshotted through the **same**
boot path (`Engine::snapshot_with_report`) under a wall-clock thread-join guard. Test
file: `tests/snapshot_high_field_wedge.rs`. Phase timings gathered with temporary
`encode_bundle` instrumentation (since removed).

### Field-count bisection — the cube is in the field count

Phase-2 timing, measured directly (N = 10 000):

| Scalar fibers | remaining numeric keys | Phase 2 (pre-fix) | Phase 2 (post-fix) |
|---:|---:|---|---|
| 8 | 9 | **11.0 s** | 0.07 s |
| 384 | 385 | **~10 days** (extrapolated `(385/9)³ × 11 s`) | **200 ns** (skipped) |

The cube bites even at 8 fields — 11 s of Phase 2 — which is why a naive field-count-only
cap would have to be absurdly low (≈ 5) to be safe. The fix attacks the redundant
re-fetch instead (§3).

### Whole-snapshot bisection (thread-join guard)

Pre-fix, through `snapshot_with_report`:

| Fibers | Records | Result |
|---:|---:|---|
| 8 | 10 000 | 12.0 s (completes, but the cube already dominates) |
| 384 | 10 000 | **WEDGE** — guard fired at 30 s; `budget=600s` never fired; log showed `high-dim sort bypass active` then no `Snapshot written` |
| 384 | 2 000 | **WEDGE** |
| 384 | 150 | **WEDGE** (the O(F³) enumeration is N-independent; still hours with the re-fetch) |

The 384/10k RED log is the money shot: the snapshot took the **`should_bypass_sort`
branch** (the prior ITEM-4 / SUDOKU fix) and *still wedged* — proving that earlier fix
(engine-level `Vec<serde_json::Value>` sort) never covered the encoder-level cube.

Post-fix, through `snapshot_with_report` (measured, this machine):

| Fibers | Records | Total snapshot |
|---:|---:|---|
| 8 | 10 000 | 1.02 s |
| 8 | 2 000 | 0.18 s |
| 32 | 2 000 | 0.62 s |
| 64 | 2 000 | 1.12 s |
| 128 | 2 000 | 2.38 s |
| 256 | 2 000 | 5.17 s |
| 384 | 150 | 0.57 s |
| 384 | 1 000 | 3.66 s |
| 384 | 2 000 | 7.49 s |
| 384 | 5 000 | 18.4 s |
| **384** | **9 964 (production count)** | **37.9 s** |

Post-fix growth is ~linear in both F and N (the O(F³·N) term is gone). The 9 964 × 384
production shape now **completes in ~38 s** where it previously never returned.

### Backing: heap vs mmap

The boot Phase-2 snapshot runs against **heap** bundles right after `replay_wal()`,
before the mmap reopen. The repro wedges on a pure-heap bundle — **mmap backing is not
required**, which rules out the "lazy mmap page-fault chain" hypothesis for this hang.

---

## 3. The fix

Two changes, both in `src/dhoom.rs`, both **format-neutral**:

1. **`detect_computed_expr_cached` (dhoom.rs:1627)** — extract each fully-numeric
   candidate column **once** (O(F·N)), then run the `op × a × b` scan over the cached
   arrays, comparing column **indices** (usize, ~1 ns) instead of field-name strings.
   Same `['*','+','-']` order, same `a`/`b` order, same `1e-9` tolerance, same
   first-match short-circuit ⇒ **the detected field and expression are unchanged**.
   This turns Phase 2 from O(F³·N) into O(F·N + F³) and dropped the 8-field control's
   Phase 2 from 11.0 s to 0.07 s.

2. **`MAX_COMPUTED_FIELD_CANDIDATES = 64` (dhoom.rs:995), guard at dhoom.rs:1095** — even
   with cached columns the residual `op × a × b` **enumeration** is O(F³) and
   N-independent (~14 s at F ≈ 385), so for wide candidate sets detection is skipped
   entirely. Embedding-style bundles (hundreds of numeric fibers) carry no inter-field
   `#a*b` relationship to find; their fields are emitted as **plain variable columns** —
   a shape the decoder already reads. 64 covers any realistic analytics/OLAP table while
   excluding vector-embedding bundles. It is far below the engine-level
   `should_bypass_sort` count (1000) because *this* work is cubic in field count, not
   linear.

The widely-used `detect_computed_field` is left intact (the profile/`FieldRole` API at
dhoom.rs still calls it) — the change is confined to the snapshot encode path.

### Why it cannot corrupt the snapshot

- **Small/normal bundles (≤ 64 numeric candidates):** cached detection is byte-identical
  to the old detection. The existing unit test `dhoom::tests::test_roundtrip_computed`
  still detects `total#qty*price` and round-trips `encode`→`decode` unchanged.
- **Wide bundles (> 64):** the skipped fields become plain variable columns. Round-trip
  test `roundtrip_wide_bundle_reopens_in_mmap_with_same_count_and_fields`: a 2 000 × 384
  bundle is snapshotted, reopened via `Engine::open_mmap`, and `COVER` returns the **same
  2 000 records** and the **same 386-field set** (`id` + `ts` + `v0..v383`), `id`
  reconstructed from its arithmetic header modifier.

### Format compatibility (mandatory)

**No WAL byte format change. No `.dhoom` on-disk format change.** The reader is untouched;
new `.dhoom` files use only shapes the reader already handles (plain variable columns are
the most basic form). Existing production `.dhoom` files (small schemas, ≤ 64 candidates)
re-snapshot **byte-identically**. The embedding bundle was never successfully snapshotted,
so there is no existing `.dhoom` for it to be incompatible with. **A format change was
NOT required** and was not made.

---

## 4. Tests (TDD, worktree only)

`tests/snapshot_high_field_wedge.rs`, all under `--no-default-features`:

| Test | Role |
|---|---|
| `high_field_count_snapshot_completes_within_budget` (2 000 × 384) | **RED→GREEN wedge pin.** Pre-fix: wedges (guard fires). Post-fix: 8.0 s. |
| `wide_bundle_encode_is_not_cubic_in_field_count` (150 × 384) | N-independent cubic discriminator; post-fix 0.57 s, asserts < 4 s. Catches cap-removal (~14 s) and re-fetch (wedge) regressions cheaply. |
| `low_field_count_snapshot_is_fast` (10 000 × 8) | Bisection control — field count is the cause. Post-fix 1.0 s. |
| `roundtrip_wide_bundle_reopens_in_mmap_with_same_count_and_fields` | **Round-trip integrity.** Same record count + field set after mmap reopen. |
| `snapshot_timeout_degrades_to_skip_not_wedge` | **Defense-in-depth.** Zero-budget snapshot degrades to timeout+skip (bundle preserved in WAL), not a wedge. |
| `bisect_measure` (`#[ignore]`) | Manual env-driven bisection harness (produced the §2 tables). |

The wedge pin uses 2 000 records rather than the production 9 964 purely so the pre-fix
run wedges unambiguously while the post-fix run stays fast for CI — the wedge is
N-independent-enough that any N ≥ a few hundred hangs for days pre-fix.

### Defense-in-depth: honest scope

`snapshot_timeout_degrades_to_skip_not_wedge` proves the between-records timeout degrades
a *distributed-cost* overrun to a skip. It does **not** prove interruption of a hang
*inside* a single `push_record`/`finish()` — that shape is architecturally
non-interruptible by a between-records check, and simulating it would need a poison-pill
branch in production encoder code (deliberately not added). **The real protection against
this class of hang is removing the unbounded per-record cost — which the §3 fix does.**

---

## 5. Residual risk on the production boot path

**Memory profile.** The fix *reduces* peak snapshot allocation (no more O(F³) `Vec<f64>`
churn — instead O(F) cached columns). More importantly it lets the boot snapshot
**complete**, which is what unblocks the heap→fast-mmap upgrade (~13–25 GB → ~200 MB).
So the fix **improves** the RSS posture: it does not change steady-state memory, and it
removes the reason the machine was pinned on the heap path.

**The O(F·N) tail (the honest caveat).** Post-fix, the 384-column embedding snapshot
takes **~38 s on this dev box** at 9 964 records; on the slower prod Fly machine under
memory pressure, estimate **~60–120 s**. That is a **one-time recovery-boot** cost, well
under the 600 s per-bundle budget, after which `.dhoom` is written and the boot upgrades
to fast-mmap (seconds thereafter). The tail is dominated by three O(F·N) passes the fix
deliberately did **not** touch (to stay minimal):
  - `find_modal_default` in Phase 3 — a `format!("{}", v)` + `HashMap` build per value,
    useless for high-cardinality float columns (~12.5 s of the 38 s at 384/10k);
  - Phase-1 arithmetic detection (~3.7 s);
  - the double body-encode — `StreamEncoder::new` full-encodes the whole sample only to
    keep line 1 (the header), then `flush_chunk` re-encodes every record.
**Follow-up (not in this fix):** skip `find_modal_default` for high-cardinality numeric
columns; have `StreamEncoder::new` build only the header. These would take the embedding
snapshot from ~38 s toward a few seconds. They are pure O(F·N) speedups, not correctness.

**What could still go wrong.**
  1. A genuinely different future hang *inside* one `push_record`/`finish()` would still
     wedge (the between-records timeout can't reach it). Mitigation for that would be
     thread-level cancellation of the per-bundle snapshot (the admin path already uses
     `spawn_blocking`); out of scope here.
  2. `stacks_passages` (70 849 records) could not be reproduced — its schema was not
     available. If it is a wide *numeric* bundle this fix covers it; if it is text-heavy
     its wedge is a different cause (e.g. the pre-#104 interning scan, already fixed).
  3. Bundles with 33–64 numeric columns still run cached detection. It is fast
     (≤ ~1 s at N = 10k), but note the compare loop is only cheap because random data
     early-breaks at i=0; a pathological all-near-match column set could make it O(N) per
     triple (still bounded at 64³·N ≈ ~1 s at N = 10k).

**Pre-existing unrelated failure discovered during gating.**
`tests/halcyon_part_v_snapshot.rs::tdd_hal_v_3_snapshot_orphan_rejection` **fails on the
base commit** (proven by reverting `dhoom.rs` to base and re-running — it still fails).
It expects a gauge-field orphan snapshot to be *rejected*, but the shipped d592313
amendment (`engine.rs:1221`) *skips* orphans gracefully (`Engine::open` returns `Ok`).
It is a stale test, orthogonal to this fix — flagged as a separate task, not touched here.

### Deploy + verification plan (for when Bee greenlights — NOT done here)

1. **Nothing is deployed or merged.** Ship from branch `worktree-wf_c5993b15-5ef-2` after
   review.
2. **First live probe (smallest blast radius):** admin-snapshot `claude_substrate_v0` via
   `POST /v1/admin/snapshot` (timeout-aware, `spawn_blocking`). Confirms the encoder
   change is sound on a live bundle without touching boot.
3. **Then the actual wedge bundle:** admin-snapshot `marcella_source_embeddings_bge_v2`
   the same way. Confirm it now completes (~60–120 s est.), writes `.dhoom`, and watch
   RSS stay bounded.
4. **Keep `GIGI_SKIP_BOOT_SNAPSHOT=1` for the first post-deploy boot** (belt-and-braces).
   Deploy, confirm the machine boots healthy on heap mode with the new binary.
5. **Drop `GIGI_SKIP_BOOT_SNAPSHOT` and restart.** Boot heap-replays, Phase-2 snapshot now
   completes for all bundles, writes `.dhoom`, reopens fast-mmap. Verify in
   `flyctl logs -a gigi-stream`: `Snapshot written: marcella_source_embeddings_bge_v2`
   followed by `Mmap opened`.
6. **If anything wedges, re-set `GIGI_SKIP_BOOT_SNAPSHOT=1` and restart** — the escape
   valve is unchanged and still works.

Recommendation: keep `GIGI_SKIP_BOOT_SNAPSHOT` set even after a successful upgrade — per
the durability note it becomes a near-no-op once fast-mmap holds, and it is the recovery
lever if a future different hang appears.

---

## 6. Gates (worktree correctness gates — NOT a deploy gate)

| Gate | Result |
|---|---|
| `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` | pass (warnings only, pre-existing) |
| `cargo test --no-default-features --lib` | **916 passed, 0 failed** |
| `cargo test --no-default-features --test snapshot_high_field_wedge` | 5 passed, 1 ignored |
| `cargo test --features halcyon --test halcyon_l24_workflow_e2e` | 1 passed |
| `snapshot_rotation` / `encoder_high_dim_smoke` / `engine_open_mmap_orphan` / `explain_mmap` | 9 / 3 / 2 / 4 passed |
| `halcyon_part_v_snapshot` | 2 passed, 1 **pre-existing** fail (orphan-rejection; base-preexisting, unrelated — §5) |

The admin-path snapshot suites (`snapshot_rotation`, `halcyon_part_v_snapshot`'s other
gates) stay green — the fix does not regress the admin snapshot path.

---

## 7. Commits (worktree branch, no `Co-Authored-By` footer)

- `cbba084` — `test(durability): RED — pin the wide-numeric-bundle snapshot-encoder wedge`
- `5eda245` — `fix(durability): GREEN — cache + guard the O(F^3*N) computed-field detection`
- `966da6d` — `test(durability): defense — timeout degrades to skip; bisection harness`
- (this doc) — report

**NOT merged. NOT pushed. NOT deployed.** Follow-up ship reads this diagnosis first.

---

## 8. SHIPPED — 2026-07-16

The header's "NOT merged / NOT pushed / NOT deployed / GATED" is now **superseded**:
merged, pushed, deployed, and live-proven on the real wedge bundle. Staged posture —
escape valve still ON, boot path still not trusted.

### Merge + gates
Cherry-picked onto `main` (base `fdf1af3`, **zero drift** — the branch was already based
on current main; the cherry-picked tree is byte-identical to `131efa0`):

- `1f21665` test(durability): RED
- `4dd8630` fix(durability): GREEN
- `a43be20` test(durability): defense
- `ee0356b` docs(durability): diagnosis (this file, pre-SHIPPED)

- **GREP GATE:** 0 `Co-Authored-By` footers on `fdf1af3..HEAD`.
- **FULL GATES:** all 14 LOCKED suites green, **first attempt, no DLL flakes, no
  assertion failures.** `--no-default-features --lib` 916 passed; `snapshot_high_field_wedge`
  5 passed / 1 ignored; spectral suite green (incl. `spectral_magnetic_basic` 285.65 s);
  halcyon-l24 / ingest-gauge / topology / imagine-coherence / halcyon-iv+aurora / lattice /
  davis-conj-ridealong / pattern-hunt / default-verbs-batch all green.
- **Push:** `origin/main == HEAD == ee0356b`, verified both directions.

### Deploy
- `flyctl deploy -a gigi-stream`. **NO secret changes** (no `flyctl secrets set/unset`).
  Depot build, release binary with the production feature set, image size 63 MB.
- **Image:** `registry.fly.io/gigi-stream:deployment-01KXQ3MB474GKFDW6MFS9GSG6H`
  (was `deployment-01KXPW3B3QVBF2X1PMKEGTHK3C`). Machine `683961dbe9ee38` version 250,
  confirmed on the new tag via `flyctl status`, 1/1 health check passing.
- Post-deploy `/v1/health`: `status ok`. Restart confirmed (uptime reset to 134 s). Boot
  skipped the snapshot as expected.

### Escape valve — UNTOUCHED
`GIGI_SKIP_BOOT_SNAPSHOT` was not read, set, or removed. Boot behavior this deploy is
unchanged: boot still skips the snapshot. The fix was validated via the **encode path**,
not the boot path.

### Controlled live proof — single-bundle, via the fixed encode path
The admin verb `POST /v1/admin/snapshot` is **WHOLE-ENGINE**, not single-bundle — it
snapshots all ~5 000 bundles and compacts the WAL under `engine_write()`, and its first
post-fix run would also touch the never-reproduced `stacks_passages`. Auth: the workflow's
`X-API-Key` is owner = admin (**no separate admin token needed**) — but a whole-engine
snapshot was **NOT fired autonomously** (LOCKED: prefer to recommend Bee run it when
uncertain; the `stacks_passages` unknown makes it uncertain, and it holds the global write
lock on 13 M records).

Instead the fix was proven on the wedge bundle through `GET /v1/bundles/{name}/dhoom`,
which reaches the **identical** fixed `encode_bundle` Phase 2 under a shared **read** lock:

| Bundle | Result |
|---|---|
| `claude_substrate_v0` (smoke, 20 rec) | HTTP 200, **0.27 s**, 44,337 B — fixed encode path works on the new binary |
| `marcella_source_embeddings_bge_v2` (THE wedge, 9,964 × 384 numeric fibers) | HTTP 200, **16.85 s**, 89,153,447 B of well-formed DHOOM |

**Was days (O(F³·N) cube) → now ~17 s.** It beat the ~60–120 s prod estimate because the
64-cap skips detection entirely at this width. Health during the proof: uptime **monotonic
468 s → 486 s** across the call (no crash, no restart); the server stayed responsive
(`health` uses `try_read`, concurrent reads unaffected by the shared read lock).

Marcella IMAGINE sanity post-proof: `POST .../imagine_coherence` `dim=4` → HTTP 200,
`endpoint_coherence 1.0`, `refused false`.

### RSS observation
RSS is **not exposed** by `/v1/metrics` (no memory field) or `flyctl status`. Indirect
bound: uptime advanced monotonically (134 → 425 → 468 → 486 → 530 s) with no restart across
the ~17 s / 85 MB encode, so the fixed encode **did not OOM** the machine. RSS-unobservable
via the app API; absence of a restart = absence of an OOM kill.

### Substrate drill
`claude_substrate_v0` exported pre-deploy (20 records, backed up to
`.deploy-backups/2026-07-16-durability/`). Wiped by the restart (boot skips snapshot), as
predicted. Restored via `CREATE BUNDLE (keys=[thought_id])` + import: **20 records, 20
distinct thought_ids, WAL-logged.** Post-count 20 == pre-count 20. (First restore attempt
with an empty key set collapsed all records onto a Null identity → 1 record; recreated with
`thought_id` as the base key, which the export shows is unique 20/20.)

### Remaining path to boot-path trust
Escape valve **stays ON**; boot path **still not trusted**. To drop
`GIGI_SKIP_BOOT_SNAPSHOT` later: (1) reproduce `stacks_passages` (70 849 records) and clear
it, or admin-snapshot it live once and confirm it writes; (2) run the whole-engine admin
snapshot and confirm every bundle writes `.dhoom` + the WAL compacts (watch health); (3)
drop the valve and confirm one boot heap-replays → snapshots → reopens fast-mmap. Until all
three hold, the valve is the recovery lever, unchanged.

Marcella handoff: `theory/marcella/GIGI_TO_MARCELLA_REPLY_2026-07-16_DURABILITY.md` (ask #6
closed — her six asks complete).
