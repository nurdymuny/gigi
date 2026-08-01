# GIGI vs SQLite vs DuckDB — the vs_stores benchmark (2026-07-31)

One machine, one dataset, three stores, four tasks, locked protocol. Every cell
below is exactly as measured — losses are reported at full volume, per rule 4 of
the protocol ("disclose, do not soften"). The harness, dataset generator, and
raw per-rep results are committed in [`benchmarks/vs_stores/`](benchmarks/vs_stores/);
the run passed an independent adversarial fairness audit and an independent
reproduction spot-check before this report was written (both summarized in §3).

**The one-line honest summary:** GIGI loses ingest 7.8x to SQLite, loses
aggregates 293x to DuckDB, and loses its own thesis task — anomaly detection —
8.5x on PR-AUC to a 15-line hand-tuned SQL baseline. Its two real wins are
narrow: it beats in-process DuckDB on point lookups *while paying an HTTP round
trip per query*, and its anomaly call is zero-config (bundle name only, 5
client lines) where the SQL baseline needed a hand-chosen grouping. That's the
scoreboard. It's committed anyway, because a benchmark you'd only publish if
you won isn't a benchmark.

---

## 1. Headline table

100,000-row synthetic transaction dataset (txn_id, merchant, customer, hour,
amount), seed 20260731, byte-reproducible from
[`gen_dataset.py`](benchmarks/vs_stores/gen_dataset.py). Every cell = 1 untimed
warmup + 3 timed repetitions, **median** reported; all per-rep raws are in the
results JSONs. GIGI is measured **over HTTP** at `127.0.0.1:3143` (its real
deployment shape, persistent keep-alive client); SQLite and DuckDB run
**in-process** (their real shape). That asymmetry is disclosed here and in
every results file — see §3.

| Task | Unit | GIGI (HTTP) | SQLite (in-proc) | DuckDB (in-proc) | Winner |
|---|---|---:|---:|---:|---|
| A. Ingest 100k rows | rows/sec, higher better | 78,775 | **617,285** | 390,428 | SQLite, 7.8x over GIGI |
| B. Point query, 2,000 lookups — p50 | ms, lower better | 0.354 | **0.056** | 0.417 | SQLite, 6.4x over GIGI |
| B. Point query — p95 | ms, lower better | 0.475 | **0.098** | 0.732 | SQLite |
| C. Aggregate: mean+stddev(amount) by merchant, 100k | wall ms, lower better | 2,167.3 † | 50.6 | **7.4** | DuckDB, 293x over GIGI |
| D. Anomaly: score 20k labeled subset | wall ms, lower better | 317.4 | 33.6 | **14.2** | DuckDB, 22x over GIGI |
| D. Anomaly: PR-AUC | average precision, higher better | 0.0848 | **0.7189** | **0.7189** | SQL baseline, 8.5x over GIGI |
| D. Anomaly: client code | non-blank/non-comment lines | **5** | 15 | 15 | GIGI |

† **Disclosed fallback, not GIGI's intended path:** the GQL parser rejected
`stddev()` in `INTEGRATE` at runtime (it accepts COUNT/SUM/AVG/MIN/MAX only,
even though `GQL_REFERENCE.md` §V claims stddev). Per the locked protocol, the
runner probed at runtime and measured the honest fallback instead: `INTEGRATE`
for avg+count, then 12x `COVER` pulling ~100k rows over HTTP with client-side
stddev — all inside the clock. That is a real product gap (docs promise a verb
the parser doesn't have) and it cost GIGI this cell at full price.

**Surprises, as measured:**

- GIGI over HTTP **beat in-process DuckDB on point queries** — p50 0.354 ms vs
  0.417 ms, p95 0.475 ms vs 0.732 ms — despite paying a localhost HTTP round
  trip per lookup. DuckDB's per-query overhead exceeds a localhost round trip.
  Only SQLite (p50 0.056 ms) actually collected the expected in-process win.
- `/scan` on the full 20k subset was near-linear and cheap (~317 ms vs the
  ~4.5 s quadratic worst case extrapolated from the 5k smoke test), so no
  subset downsizing was needed; the SQL baselines used the same 20k files.
- DuckDB's ingest reps had a wobble (280k middle rep vs ~390k on the others);
  the median stands, and all three raw reps are in `results_duckdb.json`.

---

## 2. The anomaly thesis task — reported exactly as measured

This was the cell the benchmark exists for: 100 combination anomalies planted
in a 20,000-row labeled subset (0.5% prevalence). Each plant has a normal hour
for its own merchant and a normal amount for *some* merchant — only the
(merchant, hour, amount) **combination** is wrong. Labels never leave the
client; PR-AUC for all three systems is computed by the one shared
`eval_common.average_precision` (step interpolation, deterministic txn_id
tie-break, raises on any missing labeled id).

| | GIGI `/scan` | SQLite expert SQL | DuckDB expert SQL |
|---|---:|---:|---:|
| PR-AUC (average precision) | 0.0848 | **0.7189** | **0.7189** (identical by construction) |
| Wall (median of 3) | 317.40 ms | 33.55 ms | **14.17 ms** |
| Client code (shared counting rule) | **5 lines** | 15 SQL lines | 15 SQL lines |
| Configuration required | bundle name only (`POST /v1/bundles/bench_anom/scan {budget:0.05, limit:0}`) | hand-chosen per-(merchant, 2h-bucket) \|z\| grouping | same hand-chosen grouping |

**The honest headline: the hand-tuned SQL z-score baseline beats `/scan` on
PR-AUC by 8.5x (0.7189 vs 0.0848) on GIGI's own thesis task.** Not a tie — a
decisive loss. The planted combination anomalies are exactly what a
merchant x hour-bucket z-score is shaped to catch, and `/scan`'s zero-config
scoring ranked them poorly. For calibration: the random-ranking AP floor at
0.5% prevalence is ~0.005, so `/scan`'s 0.0848 is ~17x above chance — real
signal, far below the expert baseline.

What `/scan` actually won in this cell: **zero configuration** (it received
the bundle name and a budget; the SQL baseline embeds a human's choice of
grouping columns and bucket width, which is domain knowledge the planted
anomalies happen to reward) and **5 vs 15 lines of client code**. Those are
the entire wins. If a store's answer to "find my anomalies" requires knowing
in advance that merchant x 2h-bucket is the right cohort, the 15-line query is
cheap once you know it — and on this dataset, knowing it was worth 8.5x PR-AUC.

---

## 3. Fairness, environment, and how to reproduce

### Deployment-shape asymmetry (disclosed, not hidden)

All GIGI cells go over HTTP at `127.0.0.1:3143` with a persistent keep-alive
client — its real deployment shape. SQLite and DuckDB run in-process — their
real shape. This asymmetry is stated in the protocol block embedded in every
results file (`timing_shape`), in each system's disclosure list, and in the
`honest_summary` of `results_all.json`. Note the asymmetry's direction is
anti-GIGI in every cell, and GIGI still won one of them (point query vs
DuckDB).

### Protocol (locked before the run — full text in [`run_all.md`](benchmarks/vs_stores/run_all.md))

- **Warmup + reps:** every cell = 1 discarded warmup + 3 timed reps, median
  reported; all raw reps published in the results JSONs
  (`eval_common.warmup_plus_reps`, shared by all three runners).
- **Indexes:** SQLite declares `txn_id TEXT PRIMARY KEY` and DuckDB
  `VARCHAR PRIMARY KEY` in DDL, so both stores have their key index in place
  before any point-query timing (and pay its maintenance inside the ingest
  clock, same as GIGI). SQLite's anomaly baseline additionally got a cohort
  index + `ANALYZE` before timing — best foot forward for the competition.
- **Same data:** all three systems read the same files by path; the dataset is
  byte-reproducible from seed 20260731.
- **Same eval:** one shared `average_precision`; one shared LOC counting rule;
  one deterministic point-query id list (independent RNG stream, seed+1).
- **Serial-run integrity:** GIGI leg first with the server up; server killed
  and verified down (process gone, port 3143 refusing) before the SQLite and
  DuckDB legs. Nothing else running. `gc.collect()` + `gc.disable()` around
  every timed invocation, all systems.
- **No label leakage:** the anomaly bundle carries only the five data columns;
  `labels.json` is never sent to the server; `/scan`'s payload is
  `{budget, limit}` only.
- The 22-row fairness self-review (every place a reviewer could cry foul, and
  where each is enforced in code) is at the bottom of
  [`run_all.md`](benchmarks/vs_stores/run_all.md).

### Independent verification (both passed before this report was written)

- **Adversarial fairness audit — PASS.** Re-derived every median from the
  recorded reps (zero mismatches), hash-verified the dataset regeneration
  (SHA-256 byte-identical), re-ran the checked-in anomaly SQL in fresh
  in-memory SQLite and DuckDB (both reproduce 0.7189, zero per-row score
  divergence), restarted the recorded GIGI binary (same sha) and reproduced
  `/scan`'s 0.0848 exactly. Verdict: "no fairness violation favoring gigi was
  found, and the headline result is brutally anti-gigi and fully reproduced."
  Three non-failing notes for next run: persist raw per-row scores in the
  results JSONs (offline auditability without a live server), generate
  `results_all.json` from a script rather than assembling it, and start the
  GIGI server empty (it had 10 stale bundles resident — a memory effect that
  is anti-GIGI if anything).
- **Reproduction spot-check — PASS.** Dataset regenerated byte-identically in
  an isolated dir; SQLite point-query and DuckDB aggregate cells re-run from
  the committed code came in at 1.2–1.3x the committed medians (well inside
  the 2x noise threshold, in the expected direction — the rerun shared the
  laptop with an agent session, the attested run did not); 5/5 inspected
  planted anomalies match the spec's combination-anomaly construction
  (own-merchant hour normal, cross-merchant amount normal, cohort log-gap
  1.30–2.73 vs negative-row max 0.699).

### Environment

| | |
|---|---|
| Machine | LAPTOP-5ECOBNCR — 13th Gen Intel Core i7-13620H, 64 GB RAM, Windows 11 Pro (10.0.26200), single node |
| Python | 3.12.10 (stdlib `sqlite3`; `duckdb` from pip) |
| SQLite | 3.49.1 (default pragmas: journal=delete, sync=full — disclosed, not tuned down) |
| DuckDB | 1.5.5, 16 threads (its default/real shape) |
| GIGI | 0.1.0, release build of `gigi-stream.exe`, git sha `4f9c2a4` (`4f9c2a448a227f89689d9e22df8465a4ff72afeb`), data dir on local disk outside the OneDrive-synced tree, `GIGI_SKIP_BOOT_SNAPSHOT=1` |
| Run condition | AC power, high-performance plan, strictly serial, nothing else running; 2026-07-31 |

### Reproduce it

```powershell
cd benchmarks\vs_stores

# 0. dataset (deterministic, seed 20260731 — byte-reproducible)
python gen_dataset.py

# 1. in a SECOND terminal — run_gigi.py starts nothing itself:
$env:PORT = "3143"
$env:GIGI_DATA_DIR = "$env:TEMP\gigi_vs_stores_server"
cargo run --release --bin gigi_stream

# 2. back in the first terminal — one system at a time, nothing concurrent:
python run_gigi.py      # needs the server from step 1 up
python run_sqlite.py    # kill the gigi server first; verify port 3143 refuses
python run_duckdb.py
```

Outputs land next to the runners as `results_gigi.json` /
`results_sqlite.json` / `results_duckdb.json`, each with the protocol block,
environment capture, per-rep raws, and that system's disclosure list.
`results_all.json` is the assembled cross-system file this report is written
from.

---

## 4. Snowflake, Databricks, Cassandra — NOT benchmarked, and why

No numbers in this section, on purpose.

- **Snowflake and Databricks were excluded, not dodged.** Their terms of
  service prohibit publishing benchmark results without permission (the
  standard DeWitt clause), and both require accounts. Publishing a
  cloud-warehouse row here would be either a ToS violation or a number nobody
  can check. Neither belongs in a receipts-first benchmark.
- **Cassandra was excluded for a boring reason:** no runtime on this machine —
  it wants Docker/JVM infrastructure this laptop run didn't have. That's an
  availability gap, not a verdict.

What can be said honestly is architectural, about the anomaly task only: to
answer "score my rows for anomalies," Snowflake needs Snowpark ML or an
external pipeline bolted on; Databricks needs an MLflow model or notebook job;
Cassandra needs an external ML layer entirely — each is a second system with
its own deployment, credentials, and failure modes. GIGI answers it with one
native call (`POST .../scan`) against data already in the store. Given §2,
that call currently loses 8.5x on PR-AUC to fifteen lines of SQL — so today
the architectural point is about *shape* (one system vs two), not *quality*.
Both halves of that sentence stay in the report.

**Standing invitation:** the harness is committed and deterministic. Anyone
with a Snowflake/Databricks account willing to obtain benchmark-publication
permission, or a machine with Cassandra, can run round 2 against the exact
same committed dataset, protocol, and shared scorer. Same rules apply to us:
whatever it says, it ships.

---

## 5. What this run actually taught us (the fix list)

1. **GQL stddev is a documented lie today** — `GQL_REFERENCE.md` §V claims it,
   the parser rejects it. Either ship `stddev()` in `INTEGRATE` or fix the
   docs; the 293x aggregate loss is mostly this gap plus the ~100k-row HTTP
   fallback it forced.
2. **`/scan`'s zero-config scoring missed cohort-conditional anomalies** that
   a merchant x hour z-score catches trivially. 0.0848 vs 0.7189 is the
   sharpest, most actionable number in this file.
3. **Point-query latency is genuinely competitive** — beating in-process
   DuckDB while paying HTTP round trips is the one performance bright spot,
   and it's real (audit-reproduced).
4. Next-run hygiene from the auditors: persist per-row scores for all
   systems, script the `results_all.json` assembly, start the server empty.

---

# Round 2 — embedded lane, scaling curves, expert parity (2026-07-31)

Round 1's three sharpest design critiques came from the benchmark's own
subject, and each one became an arm of round 2. (1) *Server-vs-in-process
asymmetry:* round 1 timed GIGI over HTTP against in-process sqlite/duckdb —
disclosed, but never isolated; round 2 adds an **embedded lane** with GIGI
linked as an in-process Rust library, the true apples-to-apples shape. (2)
*Single-N cannot test complexity:* one 100k point-query cell says nothing
about O(1) claims; round 2 sweeps **10k / 100k / 1M** with shared per-N id
sets and a locked rule that any embedded growth is flagged as a regression,
not smoothed. (3) *Expert-vs-novice arm asymmetry:* round 1 pitted
hand-tuned SQL against zero-config `/scan`; round 2 grants GIGI **exactly
the SQL author's domain knowledge** (cohort = merchant × 2h-bucket, amount =
value channel, nothing more, labels never sent) and reports both candidate
plays. The benchmark got better because the thing being measured pushed
back — that is how it should work.

Round 2 was independently verified before this section was written: an
**adversarial fairness audit** (which FAILED the artifact as first published
— every measured number reproduced exactly, but two gigi-favoring defects
survived; both were fixed and the affected cell re-run before these final
numbers — see `run_all.md` §post-audit amendments and
`results_round2.json.post_audit_amendments`) and an **O(1)-curve
verification** (PASS: all six stated ratios recomputed exactly from raw
reps; sqlite's growth independently reproduced with fresh DBs).

## R2.1 Embedded lane — the transport tax, isolated

100k rows, same dataset, same 2,000 point-query ids as round 1 (the Rust
example re-derives the CPython id stream bit-exactly — verified against the
committed id files at every N). Post-audit schema parity: the embedded
bundle declares the same secondary `txn_id` index the HTTP path creates, so
index maintenance is inside the ingest clock for every system.

| Task | GIGI embedded (in-proc Rust lib) | GIGI (HTTP, round 1) | SQLite (in-proc, round 1) | DuckDB (in-proc, round 1) |
|---|---:|---:|---:|---:|
| Ingest, rows/sec | 116,526 | 78,775 | **617,285** | 390,428 |
| Point query p50 | **1.5 µs** | 353.7 µs | 55.5 µs | 416.5 µs |
| Point query p95 | **2.4 µs** | 474.9 µs | 97.5 µs | 732.0 µs |

- **The round-1 point-query loss was the transport, not the engine.**
  Embedded p50 is 1.5 µs — a **236x** HTTP tax removed, flipping the cell
  from a 6.4x loss to sqlite into a **37x win** at matched (in-process)
  shape. Disclosed: the embedded loop is native Rust timing; the sqlite
  numbers time Python's `sqlite3` wrapper — the protocol shape carried from
  round 1.
- **The ingest loss is real either way.** At schema parity embedded ingest
  is 116,526 rows/s — 1.5x faster than GIGI's own HTTP shape, still a
  **5.3x loss to sqlite** (3.4x to duckdb). The wire was never the ingest
  bottleneck; the engine's write path (WAL + index maintenance) is.
- **Audit correction, at full volume:** the first published embedded ingest
  figure (134,381 rows/s) came from a schema missing the secondary
  `txn_id` index the other systems paid for — a gigi-favoring ~13% that the
  fairness audit caught (FAIL-1). The cell of record above is the
  schema-parity re-run; the superseded reps stay in `results_round2.json`.

## R2.2 Scaling curves — 10k / 100k / 1M

Point-query p50 (µs, median of 3 reps, 2,000 warm lookups, shared per-N id
sets, server verified dead for in-process legs):

| System (shape) | 10k | 100k | 1M | p50 ratio 1M/10k | p95 ratio 1M/10k |
|---|---:|---:|---:|---:|---:|
| GIGI embedded (in-proc) | 0.9 | 1.5 | 1.6 | 1.78x ⚑ | 2.17x |
| GIGI (HTTP) | 381.3 | 409.7 | 426.9 | 1.12x | 1.06x |
| SQLite (in-proc) | 56.6 | 68.8 | 72.5 | 1.28x | 2.25x |

⚑ = REGRESSION-FLAGGED per the locked protocol (any embedded growth is
flagged, not smoothed).

**Per-system verdicts, as the independent curve verifier stated them:**

- **GIGI embedded — not flat, but the growth is a single step, not a
  curve.** The 10k→100k step (0.9→1.5 µs, 1.67x) is outside 3-rep noise
  (rep ranges cleanly separated); the 100k→1M step (1.07x) is within noise
  over 10x more rows. "Inconclusive-at-this-N-range between
  O(1)-plus-cache-hierarchy-step and very weak log N"; the per-decade
  multipliers (1.67x then 1.07x) are inconsistent with a uniform O(log N)
  or O(N) index walk. **"No action-worthy regression signal"** — the flag
  is honest bookkeeping on a ~0.7 µs absolute delta, and 1M-row embedded
  p50 is still ~45x faster than sqlite at the same N. What would upgrade
  it: a 10M-row point, or 1M/100k exceeding ~1.3x with more reps.
- **GIGI HTTP — consistent-with-O(1) within noise.** 1.12x over two decades
  against a ~400 µs transport floor; the apparent growth is not
  distinguishable from jitter (p95 1M/100k is actually 0.998). The
  transport supports O(1) but also masks the engine — which is what the
  embedded lane is for.
- **SQLite — growing, outside noise, independently reproduced.** p50 1.28x,
  p95 **2.25x** (93→209 µs); the verifier's fresh re-run confirmed 1.274x /
  2.22x. The log-N cost lives in the tail, and it is first-decade-heavy on
  p50 but keeps growing at p95 in the second decade — the B-tree showing.
- **Statistical honesty at 3 reps:** rep-extreme bounds put the embedded
  1M/10k ratio anywhere in [1.50, 2.63]; the direction calls above all
  survive the range-separation test, fine-ratio claims would not. A 10M
  point with 5+ reps is the natural next cell.

Embedded GIGI is **45–63x faster than in-process sqlite at every N
measured** (62.9x at 10k, 45.9x at 100k, 45.3x at 1M).

## R2.3 Expert parity — the anomaly task with knowledge equalized

GIGI was granted exactly what the round-1 SQL author knew — cohort =
(merchant × 2h-bucket), amount = value channel — and nothing else. Labels
never sent; both candidate plays reported, no cherry-picking; PR-AUC via
the one shared scorer.

| Arm | PR-AUC | Wall (median) | Client LOC | Knowledge |
|---|---:|---:|---:|---|
| GIGI `/scan` zero-config (round 1, stands) | 0.0848 | 317.4 ms | 5 | bundle name only |
| GIGI PLAY 1: weighted cohort-lens `/scan` (native idiom) | 0.7003 | 287.9 ms | 14 | granted cohort |
| GIGI PLAY 2: GQL `INTEGRATE` cohort moments + client \|z\| — **best** | **0.7189** | 53.5 ms | 21 | granted cohort |
| Expert SQL (sqlite, round 1) | **0.7189** | 33.6 ms | 15 | hand-chosen cohort |

- **PLAY 2 exactly ties expert SQL: 0.7189 = 0.7189.** The tie is genuine
  and audit-reproduced to six decimals (0.718946, live and offline) — and
  it is a tie *by construction to the extent the math is the same
  statistic*: GQL can express the expert's cohort z-score at comparable
  line count (21 vs 15 LOC), in 53.5 ms vs sqlite's 33.6 ms. The honest
  reading is expressiveness parity, not discovery.
- **The round-1 gap was the knowledge, not the engine.** Zero-config 0.0848
  → knowledge-granted 0.7189 on the same engine, same data, same scorer.
  Round 1's 8.5x PR-AUC loss decomposes almost entirely into the missing
  cohort grant.
- **The native-idiom gap stands, narrowed but real.** PLAY 1 — GIGI's own
  `/scan` machinery with a-priori cohort weights, zero tuning — reached
  0.7003: 1.9 pts below expert SQL and **8.6x** its wall time (287.9 vs
  33.6 ms; 5.4x vs GIGI's own PLAY 2). The `/scan` rank-normalization step
  appears to cost the last ~2 pts. The interaction-lens roadmap item from
  round 1 survives round 2.

## R2.4 Surprises, as measured

1. The **236x transport tax** on point queries — far larger than expected;
   removing it flips the round-1 point-query loss into a 37x win over
   in-process sqlite.
2. Embedded ingest is only ~1.5x HTTP ingest — the ingest bottleneck is
   the engine's write path, not the wire; sqlite wins ingest 5.3x even
   with HTTP removed.
3. PLAY 1's cohort lens hit 0.7003 with a-priori weights and zero tuning —
   `/scan` can *nearly* express the expert statistic natively.
4. sqlite's p95 more than doubled 10k→1M while its p50 grew only 1.28x —
   the log-N cost hides in the tail.

## R2.5 Reproduce round 2

```powershell
cd benchmarks\vs_stores

# R2-0. builds FIRST — nothing compiles after this point
cargo build --release --bin gigi_stream
cargo build --release --example vs_stores_embedded

# R2-1. datasets (untimed; scale data lands OUTSIDE the repo/OneDrive)
python gen_dataset.py   # only if data.csv / labels.json are missing
python gen_scale.py     # -> %TEMP%\gigi_vs_stores_scale + manifest + id files

# R2-2. HTTP legs — gigi server in a SECOND terminal, data dir on local disk:
#   $env:PORT = "3143"; $env:GIGI_DATA_DIR = "$env:TEMP\gigi_vs_stores_server_r2"
#   $env:GIGI_SKIP_BOOT_SNAPSHOT = "1"; cargo run --release --bin gigi_stream
python run_scale_gigi_http.py
python run_gigi_expert.py

# R2-3. STOP the server, verify dead (must print False):
Test-NetConnection 127.0.0.1 -Port 3143 -InformationLevel Quiet

# R2-4. in-process legs — server dead, nothing else running:
cargo run --release --example vs_stores_embedded -- "$env:TEMP\gigi_vs_stores_scale\data_100k.csv"
python run_scale_sqlite.py   # self-enforces port-3143-dead in code
```

All raw reps, id-file SHA-256s, the superseded pre-parity ingest reps, both
verifier verdicts, and the amendment record are in
[`results_round2.json`](benchmarks/vs_stores/results_round2.json); the
per-leg raws are in `results_scale_gigi_http.json`,
`results_scale_sqlite.json`, and `results_gigi_expert.json`.

## R2.6 The updated fix list

1. **Ship the interaction lens.** PLAY 1 shows `/scan` is ~2 pts of PR-AUC
   and one rank-normalization decision away from the expert statistic with
   zero tuning — the highest-leverage item, unchanged from round 1.
2. **The write path is the ingest story now.** 5.3x behind sqlite with HTTP
   fully removed; WAL + index maintenance own the gap.
3. **Cache-step or log-N?** A 10M-row point with 5+ reps closes the
   embedded scaling question the 3-rep protocol honestly cannot.
4. GQL stddev is still missing (`INTEGRATE` moments + client |z| is the
   workaround PLAY 2 used); the round-1 docs-vs-parser gap stands.

---

# Round 3 — the fixes, measured (2026-07-31)

Rounds 1–2 ended with a fix list. Round 3 is that list turned into three fix
lanes, merged onto the main tip (`0eafad3`) as `fix/round3-integration`
(`19f58c0`), and re-run under the identical locked protocol: same dataset
(`data.csv` SHA-256 verified byte-identical to the round-2 manifest
`data_100k`), same seeds, same warmup(1)+3-reps/median rule, strictly serial,
committed harness byte-identical to main (`git diff main...HEAD --
benchmarks/` is empty — every round-3 number came from unmodified round-1/2
runner code against the fixed engine). All four test suites are green and
at-or-above main (lib 977 vs 963, bin 72 vs 70, lib+post_kahler_phase1 1277
vs 1263, bin+post_kahler_phase1 94 vs 92; 0 failures everywhere), and the
regression guard passed (ML endpoints smoke 1/1, `/scan` unit suite 24/24).

Two independent verifiers ran **before** this section was written, and both
passed: a **durability audit** on the ingest write path (§R3.2 — the
anti-cheating receipt for the ingest gain) and an **adversarial fairness +
no-regression audit** (§R3.3 — the anti-overfit receipt for the scan gain).

## R3.1 Three losses, three fixes, three deltas

| Loss (from rounds 1–2) | The fix, honestly named | Round 2 → Round 3 |
|---|---|---|
| Embedded ingest 5.3x behind sqlite — "WAL + index maintenance own the gap" | `perf(wal)`: bit-serial CRC-32C → compile-time slice-by-8 tables + zero-alloc append (reused scratch buffer, streaming CRC) — same bytes on disk, same fsync contract | **116,526 → 184,349 rows/s (+58.2%)**; sqlite gap 5.3x → 3.3x |
| Zero-config `/scan` PR-AUC 0.0848 — "missed cohort-conditional anomalies" | `fix(scan)`: generic interaction context lens (categorical side × binned axis cohort z) + magnitude-preserving normalization | **PR-AUC 0.0848 → 0.4579 (+0.3731)**; wall 317.4 → 501.4 ms (worse — §R3.4) |
| GQL stddev "a documented lie" — 2,167 ms fallback pulling ~100k rows over HTTP | `feat(gql)`: `INTEGRATE`/`SELECT` learn `stddev` + `variance` (population, single-pass); docs and parser finally agree | **2,167.3 → 167.5 ms (−1,999.8 ms, ~12.9x)** — probe accepted stddev, server-side path taken |

Cell-by-cell, round 2 → round 3, all medians of 3 reps (raws in
[`results_round3.json`](benchmarks/vs_stores/results_round3.json)):

| Cell | Metric | Round 2 | Round 3 | Delta |
|---|---|---:|---:|---|
| Embedded ingest 100k | rows/s | 116,526.3 | **184,349.5** | +58.2% |
| Zero-config `/scan` | PR-AUC | 0.0848 | **0.4579** | +0.3731 |
| Zero-config `/scan` | wall ms | 317.4 | 501.36 | **+183.96 (worse)** |
| Expert play 1 (`/scan` weighted) | PR-AUC | 0.7003 | 0.4924 | **−0.2079 (worse)** |
| Expert play 1 | wall ms | 287.86 | 266.24 | −21.62 |
| Expert play 2 (GQL cohort-z) | PR-AUC | 0.7189 | **0.7189** | exact reproduction |
| Expert play 2 | wall ms | 53.49 | 51.59 | −1.90 |
| Aggregate mean+stddev by merchant | wall ms | 2,167.34 (round-1 fallback) | **167.54** | −1,999.8 (~12.9x) |

- **The aggregate fix was cross-checked, not just timed.** The old fallback
  path (INTEGRATE avg+count + 12x COVER + client-side stddev) was re-timed
  live at 2,568.56 ms and its per-merchant numbers **numerically match** the
  server-side stddev (max rel diff: mean 7.6e-15, stddev 1.4e-13; population
  convention both sides) — the ~12.9x is the same answer, computed where it
  should be ([`aggcheck_round3.json`](benchmarks/vs_stores/aggcheck_round3.json)).
- **Zero-config `/scan` moved from ~17x above the random-AP floor to ~92x** —
  still 0.26 below the expert statistic, but round 1's "sharpest number in
  this file" (0.0848 vs 0.7189) is now 0.4579 vs 0.7189 with no knowledge
  granted.
- **Play 2 reproduced exactly** (0.7189, six-decimal match), so the round-2
  headline — GIGI ties the expert SQL baseline at knowledge parity — holds
  through the integration.
- Context, not of record: HTTP ingest ~83.9k rows/s, HTTP point query p50
  0.29 ms / p95 0.53 ms — consistent with rounds 1–2.

## R3.2 The durability receipt (why the ingest gain is real)

A +58% ingest gain from touching the WAL invites one suspicion: *the speedup
was bought with durability.* An independent durability audit read the
write-path diff line by line and ran the full crash/replay surface.
**Verdict: PASS.**

- **Byte-equivalent, fsync-identical.** Quoting the audit: "WAL append
  ordering, sync() (flush + sync_all), the batch_insert
  log-all-then-fsync-then-ack contract, and the entire read/replay/decode
  path are untouched. No durability/speed tradeoff flag exists because no
  tradeoff was made — nothing to opt into, nothing silent." The CRC-32C
  rewrite keeps the same polynomial/init/xorout, pinned by RFC 3720
  known-answer vectors, a verbatim copy of the old bit-serial loop kept as a
  test oracle over random payloads, and an on-disk-bytes layout test.
- **Tests before optimization.** The durability pins (crash-after-fsync
  replay at 250 and 300 records, all 8 value variants, engine-level point
  queries after reopen) were committed **before** the perf commit —
  `4853a3b` is a confirmed ancestor of `c67b557` — so they were written
  against the original implementation, not tuned to the new one. The
  pre-existing corrupt-CRC-tail test (torn-write recovery) also passes.
- **Full surface green.** The wal/replay/snapshot/durability/crash/
  checkpoint/recover suites pass on default features and `--features gauge`
  (including the 95 Halcyon TDD-HAL gates). One pre-existing failure was
  found and correctly attributed: `tdd_hal_ii_5_gauge_field_executor_unsupported_group`
  fails **identically on main** (stale U(1) expectation in the parser's
  gauge executor, pure line-shift) — unrelated to this branch, flagged for a
  separate fix.

## R3.3 The generic-lens receipt (why the scan gain is not overfit)

A +0.3731 PR-AUC gain on a benchmark the lens author could see invites the
other suspicion: *the lens hardcodes the benchmark.* An adversarial fairness
audit re-derived every median from raws, re-ran all three PR-AUCs live
against a fresh release build of `19f58c0` (all three exact matches:
0.4579 / 0.4924 / 0.7189), and read the lens source. **Verdict: PASS, one
caveat flagged.**

- **Genericity.** Quoting the audit: "the interaction lens in
  `src/ml/scan.rs` is fully generic — sides are ANY non-text categorical
  with 2-64 distinct values, axes are ANY time-NAMED numeric … or ANY
  small-integer numeric; cat-x-cat pairs are also built (beyond what the
  bench needs) … Zero occurrences of merchant/amount/txn_id/cohort or a
  literal 2h constant in production src." The dataset generator's ground
  truth is a **continuous** cosine modulation over hour-of-day — the 2h
  bucket exists only in the expert arm's discretization, so the lens's
  12-bin rule is not copying a generative constant.
- **The flagged caveat, at full volume.** 12 equal-width bins over a [0,24)
  hour field lands within ~2% of the expert's 2h partition, so the 0.4579
  is partly sensitive to that lucky-but-defensible constant. The constant is
  range-adaptive, not field- or width-hardcoded — but **bin-count
  sensitivity was never measured**. That sweep is on the round-4 list before
  0.4579 gets quoted as a robust zero-config number.
- **Harness integrity.** Committed harness byte-identical to main; the two
  new untracked helper scripts (fallback re-timer, results assembler) were
  reviewed line by line and are faithful; dataset SHA-256 re-verified;
  tracked round-1/2 results files are clean per git.

## R3.4 What regressed, plainly

Two cells got worse, and they are cells of record, not footnotes (both are
also recorded in `results_round3.json` under `honest_regressions`):

1. **Expert play 1 (`/scan` weighted) PR-AUC 0.7003 → 0.4924 (−0.2079).**
   Cause, precisely: the new interaction lens skips `context:cohort` because
   its 129 distinct values exceed the 64-value interaction side limit — and
   `context:cohort` was the field play 1 put all its fusion weight on, so
   the play collapses toward zero-config behavior. The round-2 claim that
   `/scan` can *nearly* express the expert statistic natively (0.7003) is
   **not true on this branch**. Headline expert parity survives only because
   play 2 (GQL cohort-z, the round-2 best play) still ties expert SQL at
   0.7189 — but the `/scan`-native expert play is genuinely worse and this
   report says so.
2. **Zero-config `/scan` wall 317.4 → 501.36 ms (+58%).** The interaction
   lens does more geometry per scan; the +0.3731 PR-AUC is the trade. On
   this dataset the trade is obviously right; the cost is still reported at
   full price.

## R3.5 Reproduce round 3

```powershell
cd benchmarks\vs_stores

# R3-0. build the integration branch (fix/round3-integration) first
cargo build --release --bin gigi_stream
cargo build --release --example vs_stores_embedded

# R3-1. dataset (only if data.csv / labels.json are missing; sha256 must
#        match the round-2 manifest data_100k)
python gen_dataset.py

# R3-2. HTTP legs — server in a SECOND terminal, fresh scratch data dir:
#   $env:PORT="3143"; $env:GIGI_DATA_DIR="$env:TEMP\gigi_vs_stores_server_r3"
#   $env:GIGI_SKIP_BOOT_SNAPSHOT="1"; cargo run --release --bin gigi_stream
python run_gigi.py                 # round-1 cells on the fixed engine; the
                                   # stddev probe now takes the server-side path
python run_round3_aggcheck.py > aggcheck_round3.json   # re-time fallback +
                                   # numeric cross-check vs server-side stddev
python run_gigi_expert.py          # plays 1 and 2

# the round-1/2 runners overwrite their committed outputs — copy the round-3
# numbers to their round-3 names, then restore the committed receipts:
copy results_gigi.json results_round3_gigi.json
copy results_gigi_expert.json results_round3_expert.json
git checkout -- results_gigi.json results_gigi_expert.json

# R3-3. STOP the server, verify dead (must print False):
Test-NetConnection 127.0.0.1 -Port 3143 -InformationLevel Quiet

# R3-4. embedded lane, then assemble the cross-cell file:
cargo run --release --example vs_stores_embedded -- data.csv > embedded_round3.json
python build_results_round3.py     # -> results_round3.json

# R3-5. gates + regression guard (all green, totals >= main required):
cargo test --release --lib
cargo test --release --bin gigi-stream
cargo test --release --bin gigi-stream ml_all_endpoints_regression_smoke
cargo test --lib scan
```

## R3.6 The updated fix list

1. **Raise or restructure the 64-value interaction side limit** (or fall
   back to honoring explicit fusion weights when a weighted field is
   skipped) — it silently gutted play 1; a user who *grants* cohort
   knowledge should never do worse than round 2's 0.7003.
2. **Sweep the bin count.** The fairness audit's caveat: 12 bins lands
   within ~2% of the expert's 2h partition on this dataset. Measure
   zero-config PR-AUC at other bin counts before quoting 0.4579 as robust.
3. **Ingest is now 3.3x behind sqlite** (was 5.3x). The CRC + allocation
   lane is spent; what remains is index maintenance and WAL entry framing.
4. Buy back the +184 ms zero-config scan wall time — lens candidate pruning
   or memoized cohort stats — without giving back the +0.3731 PR-AUC.
