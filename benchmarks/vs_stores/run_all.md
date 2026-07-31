# vs_stores — gigi vs sqlite vs duckdb (LOCKED PROTOCOL, 2026-07-31)

Benchmark harness for the three-store comparison. Violating any fairness rule
in the locked protocol invalidates the run; the rules, and where each one is
enforced in code, are listed in the self-review at the bottom.

## What gets measured

| cell | task | metric |
|---|---|---|
| A ingest | 100k rows via each system's idiomatic bulk path | rows/sec |
| B point query | 2,000 shared txn_id lookups, warm, sequential | p50 / p95 ms |
| C aggregate | mean+stddev of amount by merchant, full 100k | wall ms |
| D anomaly | score the 20k labeled subset | wall ms + LOC + PR-AUC |

Every timed cell = **1 untimed warmup + 3 timed repetitions, median reported**
(`eval_common.warmup_plus_reps`). PR-AUC = average precision, step
interpolation, computed by the **one** shared `eval_common.average_precision`
for all three systems.

Deployment-shape asymmetry, disclosed not hidden: gigi is measured **over HTTP
at 127.0.0.1:3143** (its real shape, persistent keep-alive client);
sqlite/duckdb run **in-process** (their real shape).

## Prerequisites

- Python ≥ 3.11 (3.12.x used for the reference run) — stdlib covers gigi and
  sqlite; `pip install duckdb` for the duckdb runner.
- A release build of gigi-stream (`cargo build --release --bin gigi_stream`).
- Laptop on AC power, high-performance plan, **nothing else running** — no
  browser, no editors indexing, no other benchmarks. Runs are serial, one
  system at a time.

## Exact serial run commands

From `benchmarks/vs_stores/` (PowerShell):

```powershell
# 0. dataset (deterministic, seed 20260731 — byte-reproducible)
python gen_dataset.py

# 1. start gigi yourself in a SECOND terminal — run_gigi.py starts NOTHING.
#    Use port 3143 and put the server's data dir on local disk, NOT inside
#    the OneDrive-synced tree (repo folder is OneDrive-synced; the runner DBs
#    already live under %TEMP% for the same reason):
$env:PORT = "3143"
$env:GIGI_DATA_DIR = "$env:TEMP\gigi_vs_stores_server"   # if your build reads it; else pass your data-dir flag
cargo run --release --bin gigi_stream

# 2. back in the first terminal — one system at a time, nothing concurrent:
python run_gigi.py      # needs the server from step 1 up
python run_sqlite.py
python run_duckdb.py
```

Outputs: `results_gigi.json`, `results_sqlite.json`, `results_duckdb.json` —
identical cell schema (documented in `eval_common.py`), each carrying the
protocol block, environment (CPU model, OS, Python/sqlite/duckdb/gigi versions,
gigi git sha), per-rep raw numbers, and that system's disclosure list.

Optional override: `VS_STORES_WORKDIR=<dir>` moves the sqlite/duckdb database
files (default `%TEMP%\gigi_vs_stores`).

## Task definitions (locked)

- **A ingest** — gigi: `POST /v1/bundles/bench/insert` in 1,000-row batches
  over HTTP; sqlite: one `executemany` in one explicit transaction; duckdb:
  `INSERT INTO … SELECT FROM read_csv(...)` (its bulk path). Fresh
  bundle/database per repetition; row count verified after each rep, outside
  the clock.
- **B point query** — same 2,000 ids in the same order for every system,
  derived deterministically (seed 20260731+1, a stream independent of the
  dataset RNG). sqlite/duckdb have txn_id PRIMARY KEY (index in place before
  timing — best foot forward); gigi uses its O(1) `GET /get?txn_id=` point
  endpoint.
- **C aggregate** — sqlite/duckdb: one GROUP BY with population stddev
  (`STDDEV_POP` / sqrt-form — same convention). gigi: GQL
  `INTEGRATE bench OVER merchant MEASURE avg(amount), stddev(amount)`, probed
  at runtime; if the server's parser rejects `stddev` (parser supports
  COUNT/SUM/AVG/MIN/MAX as of this writing, though GQL_REFERENCE.md §V claims
  stddev), the runner measures the honest fallback — INTEGRATE for avg+count
  plus one COVER per merchant with client-side stddev, ~100k rows pulled over
  HTTP inside the clock — and discloses it. Expected honest loss either way:
  duckdb will likely win this cell by a wide margin.
- **D anomaly** — gigi: `POST /v1/bundles/bench_anom/scan
  {budget: 0.05, limit: 0}`, zero-config; the labels file is never sent to the
  server. sqlite/duckdb: the honest expert-SQL flat baseline — per-(merchant,
  2h-bucket) |z| of amount via GROUP BY, scored per row, indexes allowed,
  `floor(hour/2)` bucketing identical across both SQL engines. If the SQL
  baseline ties or beats /scan on PR-AUC, that is reported as measured, with
  the note that the baseline needed hand-chosen grouping (merchant ×
  hour-bucket) while /scan got only the bundle name.
- **LOC metric** — gigi: the actual minimal client function
  (`run_gigi.gigi_scan`, source-counted at run time); sqlite/duckdb: the SQL
  text. One shared counting rule (`eval_common.count_loc`: non-blank,
  non-comment lines); the basis is recorded per system in the results JSON.

## Fairness self-review (every place a reviewer could cry foul)

| # | possible foul | addressed by |
|---|---|---|
| 1 | missing index on txn_id for the stores | PRIMARY KEY in both DDLs; in place before point-query timing (rule 2B) |
| 2 | stores build their key index *after* ingest, gigi pays it *during* | PK declared in DDL for both stores, so all three maintain the key index inside the ingest clock |
| 3 | cold-cache point queries | 1 full untimed warmup pass per cell; ingest immediately precedes; "warm, sequential" by construction |
| 4 | asymmetric batch sizes / ingest paths | each system's idiomatic path is locked by the protocol (gigi 1,000-row HTTP batches; sqlite one-transaction executemany; duckdb read_csv bulk) and disclosed |
| 5 | different lookup ids or order per system | one deterministic id list (seed+1), same order everywhere (`eval_common.point_query_ids`) |
| 6 | per-system PR-AUC implementations | one shared `average_precision`; a missing score for any labeled id raises instead of silently dropping |
| 7 | score-tie ordering flatters one system | ties broken by txn_id, identical policy for all; disclosed as arbitrary-but-identical |
| 8 | label leakage into gigi's scan | anomaly bundle contains only the five data columns; labels.json is never sent to the server |
| 9 | anomaly plant leaking into the lookup workload | point-query ids come from an independent RNG stream |
| 10 | GC pauses landing in one system's cell | `gc.collect()` + `gc.disable()` uniformly around every timed invocation, all systems |
| 11 | sqlite pragmas neither defended nor disclosed | defaults kept (journal=delete, sync=full), one transaction ⇒ one commit fsync; disclosed in results |
| 12 | OneDrive sync I/O contaminating file DBs | store DBs live under `%TEMP%`, outside the synced tree; the gigi server is instructed to keep its data dir there too |
| 13 | duckdb's CSV parse inside its timed ingest while others pre-parse | disclosed; the extra in-clock work is duckdb's own, on its fastest honest path |
| 14 | duckdb multithreading vs sqlite single-thread vs gigi server | each system's default/real shape; duckdb thread count recorded in results |
| 15 | HTTP vs in-process | disclosed in the shared protocol block of every results file, per rule 3 |
| 16 | sample-vs-population stddev mismatch between SQL engines | population stddev everywhere (STDDEV_POP / sqrt-form / gigi client fallback) |
| 17 | gigi stddev gap papered over | runtime probe; honest fallback measured with full HTTP cost; loud disclosure naming the parser limitation |
| 18 | zero-variance cohorts undefined z | COALESCE to 0.0, identical in both SQL baselines, disclosed |
| 19 | LOC metric apples-to-oranges | shared counting rule; basis (python function vs SQL text) recorded per system |
| 20 | gigi's timed scan includes shipping 20k scored rows | symmetric: the SQL cells' timed window includes `fetchall` materialization of all 20k scored rows |
| 21 | cherry-picked reps | all rep values are in the results JSON, cell statistic is the median, warmup discarded per rule 3 |
| 22 | schema drift between results files | `write_results` asserts the exact cell set; cells built only by shared `cell_*` builders |

## Honest losses we expect (rule 4 — disclose, do not soften)

- duckdb will likely win the aggregate cell by a wide margin.
- in-process point lookups may beat gigi's HTTP round trips.
- if gigi's GQL stddev is unsupported at run time, gigi's aggregate cell pays
  a ~100k-row HTTP pull and will lose badly; reported as measured.
- if the expert z-score baseline ties or beats /scan on PR-AUC, it is reported,
  alongside what it required (hand-chosen grouping) vs /scan's zero-config.

---

# Round 2 — LOCKED PROTOCOL ADDENDUM (2026-07-31)

Round-1 protocol still binds where not amended. Round 2 adds three arms:

| arm | what | files |
|---|---|---|
| 1 embedded lane | gigi as an in-process Rust library, no HTTP: ingest rows/sec + 2,000 point queries in **microseconds** at 100k, same ids as round 1 | release-built example binary (see the embedded-lane prep) |
| 2 scaling sweep | point-query p50/p95 curve at N = 10k / 100k / 1M for gigi-embedded, gigi-HTTP, sqlite in-process; ratio latency(1M)/latency(10k) per system | `gen_scale.py`, `run_scale_sqlite.py`, `run_scale_gigi_http.py` (+ the embedded example's sweep leg) |
| 3 expert-parity anomaly | gigi granted the SAME domain knowledge the SQL author had (cohort = merchant × 2h-bucket, amount = value channel) and NOTHING more; top TWO candidate plays both measured, both PR-AUCs reported | see the expert-parity prep; round-1 zero-config cell (0.0848) stands as the zero-config row |

## Scaling-sweep rules (clause 2)

- Datasets `data_10k.csv` / `data_100k.csv` / `data_1m.csv` live in
  `%TEMP%\gigi_vs_stores_scale` — **never** in the repo or OneDrive
  (`gen_scale.py` refuses to write inside either; `VS_STORES_WORKDIR`
  relocates the parent). `data_100k.csv` is a verified byte-copy of round-1
  `data.csv`; `data_10k.csv` is its byte-exact first-10,000-row prefix
  (txn_ids T000000..T009999); `data_1m.csv` extends the same generative
  model, same seed 20260731, no anomaly plant. `scale_manifest.json` holds
  row counts + SHA-256 per file.
- Per-N id sets are shared across all three systems:
  `random.Random(20260731+1).sample(range(N), 2000)` — at N=100k this IS the
  round-1 id list (asserted in `gen_scale.py`). Id files
  `point_ids_{10k,100k,1m}.txt` are written to the scale dir as the
  cross-language determinism receipt: the Rust embedded example re-derives
  the ids in-code (bit-exact MT19937/`sample()` reimplementation) and the
  `--ids-check` output is verified equal to these files at every N.
- Only point queries are timed in the sweep; ingest is untimed setup. Warm,
  sequential, 1 warmup pass + 3 timed passes, median — round-1 rule.
  Latencies in **microseconds** (round-2 unit, matching the embedded lane).
- Ratio latency(1M)/latency(10k) per system: flat ~1.0× supports O(1); growth
  is reported as measured — for **gigi embedded** any clear growth is flagged
  as a **REGRESSION finding**, not smoothed.

## Round 2 exact serial command order

Timing purity (clause 4): strictly serial, one agent, nothing concurrent;
**all cargo builds complete before any timed phase**; the gigi server is up
ONLY for HTTP legs and **verified dead** before in-process legs
(`run_scale_sqlite.py` additionally refuses to run while port 3143 answers).

From `benchmarks/vs_stores/` (PowerShell):

```powershell
# R2-0. builds FIRST — nothing compiles after this point
cargo build --release --bin gigi_stream
cargo build --release --example vs_stores_embedded   # embedded lane (adjust if that prep named it differently)

# R2-1. datasets (untimed)
python gen_dataset.py   # only if data.csv / labels.json are missing
python gen_scale.py     # -> %TEMP%\gigi_vs_stores_scale (+ scale_manifest.json, id files)

# R2-2. HTTP legs — start gigi in a SECOND terminal, data dir on LOCAL DISK
#       (not OneDrive), wait for /v1/health:
#           $env:PORT = "3143"; cargo run --release --bin gigi_stream
python run_scale_gigi_http.py   # sweep HTTP leg (ingest untimed; the 1M ingest is the long pole)
# (expert-parity arm HTTP runs also go here, per its prep)

# R2-3. STOP the gigi server, then verify it is dead:
Test-NetConnection 127.0.0.1 -Port 3143 -InformationLevel Quiet   # must print False

# R2-4. in-process legs — server dead, nothing else running:
cargo run --release --example vs_stores_embedded   # embedded lane + its sweep leg
python run_scale_sqlite.py                         # sweep sqlite leg (aborts if 3143 answers)
```

Outputs (this prep): `results_scale_gigi_http.json`,
`results_scale_sqlite.json` — identical schema (per-N cells with p50/p95 µs
reps + medians, and a `scaling` block with `p50_ratio_1m_over_10k`), both
built by the ONE shared `gen_scale.scale_point_cell` /
`gen_scale.write_scale_results`. The embedded lane writes its own results
file per its prep.

Env knobs: `VS_STORES_WORKDIR=<dir>` relocates the scale data dir parent
(default `%TEMP%`); `VS_STORES_SCALE_FRESH=1` forces `run_scale_gigi_http.py`
to drop + re-ingest the scale bundles instead of reusing a bundle whose
record count already matches N.

## Round 2 post-audit amendments (2026-07-31, before the report was written)

The round-2 run was adversarially audited (schema equivalence, expert-arm
knowledge grant, label hygiene, arithmetic, sweep hygiene) plus an independent
O(1)-curve verification. The curve check PASSED outright. The fairness audit
FAILED the artifact as first published — every measured number reproduced
exactly, but two gigi-favoring defects survived — and the following was fixed
**before** `BENCHMARKS_VS_STORES.md` §Round 2 was written:

1. **FAIL-1 (schema parity):** `examples/vs_stores_embedded.rs` originally
   omitted the secondary `txn_id` index that round 1's HTTP `create_bundle`
   adds for every key field, so the embedded ingest clock skipped index
   maintenance the other systems paid (fairness self-review row 2). Fixed:
   `bench_schema()` now declares `.index("txn_id")` and the 100k embedded
   INGEST cell was re-run at schema parity (134,381 → 116,526 rows/s median,
   ~13%; the audit's instrumented estimate was ~11%). No verdict flips.
   Point-query cells carried unchanged — `BundleStore::point_query` never
   touches `field_index` (audit-verified), and the parity re-run reproduced
   them.
2. **FAIL-2 (pro-gigi summary error):** PLAY 1's wall deficit is **8.6x vs
   the SQL baseline** (287.9 vs 33.6 ms); "~5x" was vs gigi's own PLAY 2.
   Corrected in `honest_summary`.
3. **FINDING-3 (anti-gigi arithmetic):** "35-45x faster than sqlite at every
   N" corrected to **45-63x** (62.9x at 10k); the native-Rust-loop vs
   Python-`sqlite3`-wrapper timing shape is now disclosed next to it.
4. **MINOR (doc/code mismatch):** this file and `gen_scale.py` claimed the
   Rust example READS `point_ids_*.txt`; it re-derives the ids in-code
   (bit-exact MT19937 reimplementation, verified equal at all Ns). Docs
   corrected above; the id files are the receipt, not an input.

The amendment record, superseded no-index ingest reps, and re-run provenance
(`git diff 4f9c2a4 -- src/` empty; `Cargo.toml` delta = the `[[example]]`
registration only; port 3143 verified dead) live in
`results_round2.json.post_audit_amendments`.
