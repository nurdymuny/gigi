# Deploy 2026-07-05 — residual fixes + public demo bundles

Base: main @ `0955011`. Shipped: `1806dba`, `b881eed`, `ccf154e`, `ea63e06`
(code) + this report + `examples/seed_tetmesh_demo.py`.
Image: `deployment-01KWTADE63YF3747C2X5ZE72SR` (machine version 245,
replacing v244 `deployment-01KWM7S0SKS47P0ZEBXX4GP519`).

## Fix 1 — DEFINE PATTERN consumes the AND/OR combinator chain (`1806dba`)

`parse_define_pattern` (src/parser.rs, `patterns`-gated) stopped after the
first AND-chained condition list; a trailing `OR ...` fell through to the
statement-level trailing-token refusal. The fix adds the same OR loop
COVER's statement parser uses: each `OR` after the base predicate opens a
new AND-chained group in `or_groups`, which the AST already carried (it was
always `Vec::new()` before).

Evidence: `pattern_hunt_parser::ph3_and_or_combinators_parse` fails on
`0955011` (trailing-clause refusal naming `OR field_b = 2`), passes after
the fix; suite 15/15. ph3 pins parse success for both combinators, nothing
more.

**Semantics caveat (read this before using pattern OR).** Execution was
inherited whole, not invented: HUNT desugars into COVER handing
`or_groups` through, and `bundle::matches_filter` evaluates COVER's
documented composition (GIGI_API.md §OR): a row matches when the base
predicate matches **AND** at least one OR group matches. That is COVER
surface parity — the contract ph3's file header pins — but it is **not**
SQL boolean-OR across the whole predicate. Truth-table on a 4-row fixture:
`DEFINE PATTERN q AS a = 1 OR b = 2` + HUNT matches only the
`a=1 AND b=2` row. A reader expecting disjunction gets intersection-like
results on 2 of 4 truth-table rows. `ea63e06` corrects an inline comment
that had this backwards; the commit message of `1806dba` states it
correctly.

## Fix 2 — bundle-less EMIT dispatched instead of silently OK (`b881eed` + `ccf154e`)

`gql_query`'s pre-resolve arm early-returned bare `{"status":"ok"}` for any
statement whose bundle resolves to `None` — including `Emit` wrappers whose
inner statement has no bundle (`SHOW BUNDLES EMIT CSV TO ...`). The
2026-07-03 fix (`7a23aba`) only covered single-bundle inners. The None arm
now special-cases `Statement::Emit` through `execute_gql_on_engine` →
`gigi::parser::execute`, the same executor path the with-bundle EMIT takes;
the executor owns the `GIGI_EMIT_DIR` gate. Non-Emit None-bundle statements
keep the batch's bare-ok semantics byte-identically (Notice statements
unchanged).

RED (`b881eed`): bin-internal test proves the real handler returned bare ok.
GREEN (`ccf154e`):
`tests::http_gql_emit_bundleless_inner_dispatches_through_parser_executor`
pins both phases — gate closed → the `GIGI_EMIT_DIR` gate error; gate open →
the inner `SHOW BUNDLES` really executes and EMIT answers its typed
rows-shape error. Bin EMIT tests 2/2; full bin module 67/67.

**Behavior change, intended:** Emit-wrapped bundle-less inners that the
executor cannot run now return a 500 `ExecError` instead of silent ok, and
with the gate open an EMIT of any rows-producing bundle-less inner actually
writes. Real execution replaces success theater for the whole class.

## Review follow-up (`ea63e06`)

Skeptic lens, low severity: inline comment in `parse_define_pattern`
claimed base-OR-groups; empirically false (see caveat above). Comment now
states the base-AND-(groups-ORed) composition `matches_filter` evaluates.
Comment-only.

## Gates (all green before deploy)

| Gate | Result |
|---|---|
| `cargo check` Dockerfile feature combo, `--bin gigi-stream` | exit 0 (warnings only, pre-existing) |
| `pattern_hunt_parser` (`patterns`) | 15 / 0 |
| `cargo test --no-default-features --lib` | 911 / 0 |
| bin `http_gql_emit` (old + new) | 2 / 0 |
| default battery: `emit_csv` `ingest_executor` `ingest_csv_basic` `ingest_jsonl_basic` `noop_notices` `timestamp_ergonomics` `gql_reference_truth` `explain_kappa` `ingest_dir_gate` `pathguard_escapes` | 52 / 0 across 10 suites |
| halcyon INGEST family (6 suites incl. `halcyon_l24_workflow_e2e`) | 40 / 0 |
| halcyon spectral + topology (7 suites) | 67 / 0 |
| `imagine_coherence_phase2` (kahler+imagine) | 10 / 0 |
| `halcyon_part_iv_gold` + `aurora_lie_poisson_trait` | 16 / 0 (1 ignored) |
| `cubic_lattice` + `lattice_obc_basic` (lattice) | 17 / 0 |
| `davis_conjecture_lambda_brain_ridealong` (kahler) | 25 / 0 |

Co-author grep over `0955011..HEAD`: 0 matches.

## Deploy

`flyctl deploy -a gigi-stream` → image
`deployment-01KWTADE63YF3747C2X5ZE72SR`, machine `683961dbe9ee38` v245,
smoke + readiness passing. `flyctl status` Image field matches the deployed
tag. `/v1/health` ok (13,001,312 records / 5,046 bundles at first boot).

## Live probes (v245)

| Probe | Result |
|---|---|
| R1 `DEFINE PATTERN probe_q AS field_a = 1 OR field_b = 2;` | `{"status":"ok"}` — not the trailing-clause refusal |
| R2 `SHOW BUNDLES EMIT CSV TO 'x.csv';` | 500 `EMIT is disabled on this engine: set GIGI_EMIT_DIR=...` — not bare ok |
| R3 `SHOW BUNDLES;` | rows, 200 |
| R3 `VACUUM claude_substrate_v0;` | 200 Notice: "parsed and validated, but this verb performs no work on this server path yet — nothing was executed" |
| R3 `COVER claude_substrate_v0 EMIT CSV TO 'y.csv';` | the same `GIGI_EMIT_DIR` gate error (with-bundle path unchanged) |
| R3 `INGEST probe_bundle FROM 'x.csv' FORMAT CSV SOME JUNK;` | 400 "Statement parsed, but this trailing input is not a supported clause and was NOT executed: 'SOME JUNK'" |
| R3 `LATTICE l4_deploy_probe_20260705 FROM CUBIC L=4 DIM=2 OBC AXIS 0;` | `{"status":"ok"}` |
| R4 `POST /v1/bundles/imagine_deploy_probe_2026_07_05/imagine_coherence` `{"dim":4,"steps":3,...}` | 200; coherence 1.0 every step, `max_imagined_curvature` 4.0 (FS ceiling), `max_accumulated_holonomy` 0.5 |

## Substrate drill — claude_substrate_v0

Pre-deploy export: `COVER claude_substrate_v0` → 20 rows saved to
`.deploy-backups/2026-07-05/claude_substrate_v0_backup.json`. The deploy
wiped it (expected: last good on-disk snapshot predates the bundle; boot
health showed 5,049 → 5,046 bundles, −23 records). Restore: `CREATE BUNDLE`
+ `POST /v1/bundles/claude_substrate_v0/import` with `{"records":[...]}` →
`{"status":"imported","count":20,"total":20}`; COVER verifies 20/20. It was
wiped a second time by the snapshot-wedge restart below and restored the
same way; final state 20/20, matching the pre-deploy export.

## Demo seeder + public bundles

`examples/seed_demo_bundles.py --endpoint https://gigi-stream.fly.dev`
(write key): drops and recreates `stations` and `chembl` from the demos'
seeded mulberry32 PRNGs.

- `HEALTH stations` → record_count 480, curvature 0.06514836932303834
- `HEALTH chembl` → record_count 2200, curvature 0.02884644757628001

`tetmesh_demo` had never been materialized on production (deferred in the
2026-07-02 report). New: `examples/seed_tetmesh_demo.py` (this commit)
replays the tetmesh harness's deterministic corpus — 5,760 fiber records,
classifier state {C0: 3456, C1: 768, C2: 1536}, the spec receipt — under
the bundle name the site queries. `HEALTH tetmesh_demo` → record_count
5760, curvature 0.09004696936520351, 1 base + 15 fiber fields. The loads
ran three times today (initial, post-restart, committed-script proof) and
produced this receipt byte-identically each time.

Known doc drift, not chased here: `site/gigi-builds/llms-full.txt` quotes
an older ad-hoc tetmesh load (7 fiber fields, curvature 0.1175) that is
reproducible from nothing in the repo. The canonical corpus now live has
15 fiber fields, curvature 0.0900. The site page copy needs a one-line
refresh.

## Public-read verification (no API key on any of these)

| Check | Result |
|---|---|
| `POST /v1/public/gql` `HEALTH tetmesh_demo;` | 200, record_count 5760 |
| `POST /v1/public/gql` `COVER tetmesh_demo ON classifier_cell = 'C1' FIRST 2;` | 200 with fiber rows |
| `POST /v1/public/gql` `HEALTH stations;` / `HEALTH chembl;` | 200, 480 / 2200 |
| `POST /v1/public/gql` `HEALTH sensors;` | 200, record_count 0 (allowlisted, intentionally empty) |
| `POST /v1/public/gql` `COVER claude_substrate_v0;` | 403 "bundle 'claude_substrate_v0' is not exposed on the public read endpoint" |
| `POST /v1/public/gql` INSERT into stations | 403 "verb not allowed on the public read endpoint" |
| `POST /v1/gql` without key | 401 |
| `GET /v1/bundles/claude_substrate_v0/records` without key | 401 (key wall) |

One correction to the drill text: `GET /v1/bundles/{name}/records` is not
a route this engine has ever registered (404 even with a key). The public
read surface `b295238` shipped — and what the verification above proves
out, rows included — is `POST /v1/public/gql` against the
`GIGI_PUBLIC_BUNDLES` allowlist (`stations,sensors,chembl,tetmesh_demo`).

## Incident during post-deploy hardening: admin snapshot wedge, now pinned

After seeding, one `POST /v1/admin/snapshot` was fired to persist the
restored substrate + demo bundles. The streaming path wrote per-bundle
snapshots (chunk_size=50000, budget=600s per log line) through ~5,000
bundles in ~2 minutes, then hung at 23:48:17Z on
**`marcella_source_embeddings_bge_v2` (9,964 embedding records)** — zero
log lines for the following 24+ minutes, engine lock held, `/v1/health`
degraded to `{"bundles":0,"loading":true}`, data plane blocked. The
per-bundle 600s budget did not fire. This is the first time the
long-suspected encoder hang has a concrete bundle name: large-vector
embedding payloads on the admin snapshot path.

Recovery: `flyctl machine restart` → boot loaded the last good snapshot
(orphan partial snapshot skipped per the TDD-HAL-V.3 amendment policy,
`d592313`) → substrate + seeder + tetmesh loads re-run, byte-identical
receipts, all verification re-run green. One anomaly for the durability
worklist: post-restart `total_records` came up 13,008,385 vs 13,001,312 at
first v245 boot (+7,073) with the same bundle count — the orphan-skip
policy's interaction with per-bundle snapshot files deserves a look.

Consequence, stated plainly: **no admin snapshot has succeeded on this
dataset**, so `claude_substrate_v0`, `stations`, `chembl`, and
`tetmesh_demo` will not survive the next process restart. Re-materialize
after any restart:

```
python examples/seed_demo_bundles.py  --endpoint https://gigi-stream.fly.dev   # stations + chembl
python examples/seed_tetmesh_demo.py  --endpoint https://gigi-stream.fly.dev   # tetmesh_demo
# claude_substrate_v0: CREATE BUNDLE + POST .deploy-backups/2026-07-05/… to /import
```

(with `GIGI_API_KEY` set). Do not fire `/v1/admin/snapshot` on production
until the embeddings-bundle hang is fixed; the wedge takes the whole data
plane down with it.

## Final state

`/v1/health`: ok, 5,046+ bundles, ~13.0M records, v245.
Public demo bundles live and readable without a key; write surface and
non-allowlisted bundles gated exactly as before.
