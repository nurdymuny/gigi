# DEPLOY_2026-07-03 — sixteen-feature batch: review findings fixed, gates, live probes

## SHA + image

- Base: `8244907` — the 34-commit feature batch from the parallel instance, already on `origin/main`.
- Fix range merged: `8244907..152de57` (7 review-fix commits + 1 lens follow-up), fast-forward, SHAs preserved from `worktree-wf_6bd9b98c-753-1`.
- HEAD at deploy: `152de57` — pushed to `origin/main` before deploy; verified equal both directions.
- Deploy image: `registry.fly.io/gigi-stream:deployment-01KWM5JK2SDA4RRV176MEJ68ZE` (digest `sha256:86ff73ba…`, 63 MB).
- `flyctl status`: machine `683961dbe9ee38` (iad) running `gigi-stream:deployment-01KWM5JK2SDA4RRV176MEJ68ZE`, version v243, 1/1 checks passing — the authoritative tag check (no uptime heuristics, per the 7/02 lesson).

## Review findings — blocker, fixed, deferred

The parallel instance's 34-commit batch went through adversarial review before this deploy. Five confirmed findings were fixed (in locked order, TDD where behavioral); three were deferred as policy or design calls.

| # | severity | finding | disposition |
|---|---|---|---|
| 1 | BLOCKER | `patterns` feature did not compile: `parse_weight_expr` mapped `Token::human` (takes `&Token`) over a `&[String]` slice — E0631 under `--features patterns`, which also kept every `patterns`-gated test unrunnable | FIXED `7d239a0` |
| 2 | panic | `Token::human` truncated string tokens with `&s[..24]` — any parse error rendering a >24-byte literal with multibyte UTF-8 straddling byte 24 panicked the handler instead of returning the error | FIXED `dd19b37` |
| 3 | panic | `timefmt::parse_iso_ms` byte-sliced the time part (`&rest[0..2]`, `&rest[3..5]`, `&t[0..2]`) — multibyte input to a TIMESTAMP field panicked the write handler | FIXED `5e0c066` |
| 4 | success theater | `Statement::Emit` had no server dispatch arm: `get_bundle_name` fell to `_ => None` and `/v1/gql` answered `{"status":"ok"}` without executing anything — same class as the three prior route-handler bypasses (553a6c9 topology verbs, INGEST bypass, ALTER BUNDLE delegate) | FIXED `8c3b380` + `7a23aba` |
| 5 | write-path gap | TIMESTAMP coercion chokepoint covered insert paths but not `Engine::update` — a REDEFINE could store raw `Text` in a TIMESTAMP field, silently constant under every time comparison thereafter | FIXED `d6e21c8` + `3d16658` |

### Deferred (explicitly out of this deploy's scope)

| finding | why deferred |
|---|---|
| CSV/JSONL ingest reads any server-readable file with the API key | policy call (a `GIGI_INGEST_DIR`-style gate) belongs to Bee |
| `emit_target` Windows drive-prefix path escape (`/tmp/x` and `C:x` count as relative on Windows) | prod is Linux; product behavior untouched — the phase-3 test fixture was made platform-absolute so the gate tests what it means (see `152de57`) |
| `explain_kappa` sum-to-total invariant is vacuous (no `_kappa` column exists on the point read) | needs a design decision on what EXPLAIN SECTION AT should cross-check |

### Follow-ups surfaced by the review (not fixed here)

- `pattern_hunt_parser::ph3_and_or_combinators_parse` fails: DEFINE PATTERN body parser does not consume OR combinators. Pre-existing in the batch, unrunnable at base because `patterns` did not compile (finding 1 unmasked it). Needs its own fix; not in the locked gate set.
- EMIT wrapping a bundle-less inner statement (e.g. `SHOW BUNDLES ... EMIT CSV`) still hits the silent-ok early return — `get_bundle_name` recurses to `None`, same semantics as the pre-existing Explain arm. All single-bundle inners (COVER, INTEGRATE, SHOW FIELDS, ...) dispatch correctly.
- `Engine::update` KEY record is not coerced (a TIMESTAMP base field addressed by ISO string in `AT` could miss) — pre-existing matching behavior, outside the locked fix scope.

## Fix commits

| commit | subject |
|---|---|
| `7d239a0` | fix(parser): patterns-feature compile break — weight-expr trailing-token error maps Token::human over strings |
| `dd19b37` | fix(parser): Token::human truncates on char boundary — multibyte literals no longer panic the error path |
| `5e0c066` | fix(timefmt): parse_iso_ms rejects multibyte input instead of panicking on byte-slice |
| `8c3b380` | tests(route-handler): RED — EMIT over GQL returns silent ok without executing |
| `7a23aba` | impl(route-handler): GREEN — Statement::Emit dispatched through parser executor |
| `d6e21c8` | tests(engine): RED — update stores raw Text in TIMESTAMP field |
| `3d16658` | impl(engine): GREEN — TIMESTAMP coercion applied on update path |
| `152de57` | fix(review): lens follow-ups — emit_csv absolute-path fixture is platform-absolute |

Notes on shape:

- Fix 4's RED test lives in `src/bin/gigi_stream.rs` `cfg(test)` (`tests::http_gql_emit_dispatches_through_parser_executor`), not `tests/` — the fix is bin-internal (`get_bundle_name`, `needs_write`, `execute_gql_on_engine` are not lib-visible) and the lib-side EMIT executor is already covered by `tests/emit_csv.rs`, so a `tests/`-dir file could never have gone RED. Run with `cargo test --bin gigi-stream http_gql_emit`.
- `152de57` is tests-only: the emit_csv phase-3 fixture `/tmp/abs.csv` has no drive prefix, so Windows' `Path::is_absolute()` said false, the refusal never fired, and the join escaped the emit dir — the phase was vacuous on Windows dev boxes. Fixture now picks a platform-absolute path; Linux bytes unchanged; the deferred product finding above stays deferred.
- Co-Authored-By grep over `8244907..HEAD` (full `%B` bodies): 0 matches. Author on all 8 commits: nurdymuny.

## Pre-deploy gates (all green)

| gate | result |
|---|---|
| `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` — THE DOCKERFILE COMBO, new permanent gate | PASS |
| `cargo check --features patterns --lib` | PASS |
| `cargo test --no-default-features --lib` | 911 / 0 |
| `halcyon_l24_workflow_e2e` | 1 / 0 |
| INGEST family (`as_gauge_field` 18, `gauge_vertex` 8, `npz_key` 4, `npz_dtype` 4, `gql_bypass` 5) | 39 / 0 |
| spectral + topology (`spectral_gauge` 21, `spectral_gauge_where` 7, `chern_class` 6, `chern_class_bundle_target` 11, `betti_pi1` 8, `obstruction` 5, `topology_verbs_gql_integration` 9) | 67 / 0 |
| `explain_kappa` (run in the halcyon group; test has no feature cfg) | 3 / 0 |
| `halcyon_part_iv_gold` + `aurora_lie_poisson_trait` | 16 / 0 (1 pre-existing ignore) |
| `imagine_coherence_phase2` (kahler+imagine) | 10 / 0 |
| `cubic_lattice` + `lattice_obc_basic` (lattice) | 17 / 0 |
| `davis_conjecture_lambda_brain_ridealong` (kahler) | 25 / 0 |
| no-feature group (`emit_csv` 1, `noop_notices` 3, `timestamp_ergonomics` 6, `ingest_csv_basic` 11, `ingest_jsonl_basic` 3, `ingest_executor` 4, `gql_reference_truth` 6) | 34 / 0 |
| new RED-turned-GREEN suites (`parser_error_ux` 3, bin `http_gql_emit` 1) | 4 / 0 |

One transient: the first halcyon-group run failed `chern_class_bundle_target_basic` 10/11 (cargo fail-fast stopped the group). The same binary then passed 7 consecutive runs (6 isolated + 1 full group re-run, 11/11 each) with no code change in between. Filed as an environmental flake on this Windows box (tempdir teardown contention is the usual suspect); the gate result above is the clean full-group run.

## Deploy

- `flyctl deploy -a gigi-stream` from the repo root; remote build, rolling update, release v243 `complete`.
- Boot replay: mmap catalog walk, then WAL replay. Two pre-existing warnings on the gauge WALs — "replay stopped at corrupted WAL tail after 2341314 valid entries; preserving valid prefix" — graceful-skip behavior, not introduced by this deploy.
- `/v1/health` post-boot: `{"status":"ok", "bundles":5046, "total_records":13001312}`. Pre-deploy baseline was 5049 / 13,009,750 — the delta is the heap-only bundles that never survive a redeploy (including `claude_substrate_v0`, restored below).

## Live probes on the deployed image

All ten probes ran over `POST /v1/gql` (X-API-Key auth) against the new image.

| id | query | status | body head | pass |
|---|---|---|---|---|
| P1 | `SHOW FIELDS ON claude_substrate_v0;` | 200 | `{"rows":[{"field":"thought_id","type":"Categorical","kind":"base",...],"count":6}` — 6 real field rows | PASS |
| P2 | `BUNDLE ts_probe_0703 ... at TIMESTAMP` + SECTION with `at='2026-07-01T14:30:05Z'` + SECTION with `at=NOW` + `COVER ... WHERE at <= NOW;` | 200 | `{"rows":[{"id":"e1","at":1782916205000},{"id":"e2","at":1783088737899}],"count":2}` — ISO string and NOW both stored as timestamps; time-order WHERE works | PASS |
| P3 | `COMPACT ts_probe_0703;` | 200 | `{"status":"ok","notice":"statement parsed and validated, but this verb performs no work on this server path yet — nothing was executed"}` | PASS |
| P4 | `EXPLAIN SECTION ts_probe_0703 AT id='e1';` | 200 | `{"rows":[{"kappa":0.998...,"field":"at","z":1.0,...}],"count":1}` — real decomposition rows, not "Unknown statement" | PASS |
| P5 | `INTEGRATE nonexistent MEASURE avg(x) WITH JACKKNIFE ALONG t SKIP FIRST 5;` | 404 | `{"error":"No bundle: nonexistent"}` — parser accepted, executor resolved | PASS |
| P6 | `COVER ts_probe_0703 ALL EMIT CSV TO 'probe.csv';` | 500 | `{"error":"EMIT is disabled on this engine: set GIGI_EMIT_DIR=<directory> to enable it — ..."}` — the gate error, NOT the old bare `{"status":"ok"}`; fix 4 witnessed live | PASS |
| P7 | `LATTICE l4_deploy_0703 FROM CUBIC L=4 DIM=2 OBC AXIS 0;` | 200 | `{"status":"ok"}` | PASS |
| P8 | `INGEST px FROM '/tmp/nope.npz' FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_deploy_0703;` | 500 | `{"error":"INGEST: source file not found: /tmp/nope.npz"}` — executor-level, not the pre-bypass `No bundle: px` | PASS |
| P9 | `INGEST py FROM '/tmp/a.npz' FORMAT NPZ JUNKTOKEN;` | 400 | `{"error":"Parse error: Statement parsed, but this trailing input is not a supported clause and was NOT executed: 'JUNKTOKEN'. ..."}` | PASS |
| P10 | `COVER '<x + я×20, 41 bytes>' ALL;` | 400 | `{"error":"Parse error: Expected a name here, found string 'xяяяяяяяяяяяя…' (near: ...)"}` — clean 4xx with char-boundary truncation + ellipsis, no panic/500; fix 2 witnessed live | PASS |

Probe fixtures created on prod: `ts_probe_0703`, `l4_deploy_0703`, `marcella_sanity_0703` — heap-only, will vanish on the next redeploy.

## Marcella sanity (IMAGINE Phase 2)

`POST /v1/bundles/marcella_sanity_0703/imagine_coherence` with `{"dim":4,"steps":3,"starting_from":[0,0,0,0],"along":[1,0,0,0]}` (bundle created fresh first — the endpoint 404s on a missing bundle):

```
200: dim=4, trajectory=seed+3 steps, coherence=1.0 at every step,
     defect=0.0 (float-eps at step 3), max_imagined_curvature=4.0 (FS ceiling),
     max_accumulated_holonomy=0.5
```

Identical to the 2026-07-02 reference values. Phase 2 path intact.

## claude_substrate_v0

- Already missing BEFORE this deploy (`COVER` → `No bundle` on the v242 image at 18h uptime), so the wipe predates this ship; redeploy would have wiped it regardless (known heap-only fragility).
- Restored per the documented procedure: `CREATE BUNDLE claude_substrate_v0 (thought_id TEXT BASE, ts NUMERIC FIBER, session TEXT FIBER, topic TEXT FIBER, content TEXT FIBER, refs TEXT FIBER);` then `POST /v1/bundles/claude_substrate_v0/import` from `.deploy-backups/2026-06-29-morning/claude_substrate_v0_backup.json`.
- Import result: `{"status":"imported","count":20,"total":20}` — all 20 records landed; the 2026-06-29/07-02 20→4 fingerprint collapse did NOT recur (base key `thought_id` has 20 unique values; the earlier collapse is worth a look next time someone is in `batch_insert`, but today's outcome is the better one). `COVER claude_substrate_v0 ALL;` → 20 rows.
- Still fragile until the snapshot wedge is fixed; `/v1/admin/snapshot` intentionally NOT triggered here (out of this deploy's locked scope; known encoder-hang unknowns).

## Notes

- The one gate flake (`chern_class_bundle_target_basic`, 1/11 on first group run, 7 clean re-runs) is documented in the gate section above; environmental, this Windows box.
- Post-deploy `/v1/gql` without a key still 401s; the API key path works throughout.
- No `Co-Authored-By` footer anywhere on `8244907..HEAD` — grep gate 0 before push, re-run including this doc's commit before the final push.
