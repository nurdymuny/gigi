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

---

# Hardening trio — same-day follow-up merge (2026-07-03, second deploy)

The three deferred rows in the review table above are exactly what this follow-up ships: the `GIGI_INGEST_DIR` gate (row 1), the `emit_target` Windows path-shape escapes (row 2), and the vacuous `explain_kappa` invariant (row 3). Six commits cherry-picked from `worktree-wf_8c6cd101-1dc-2` onto `bc1d038`, zero conflicts (`src/parser.rs` auto-merged — main's morning fixes sat at `Token::human`/`parse_weight_expr`, the trio's at `emit_target`; distinct regions).

## SHAs

| original (branch) | merged (main) | subject |
|---|---|---|
| `96ba912` | `6b77fff` | tests(pathguard): RED — emit_target escape matrix (drive-prefix, rooted, UNC, .., symlink) |
| `e1c8300` | `f2c60af` | impl(pathguard): GREEN — component-screen + canonical containment; emit_target swapped |
| `bf07967` | `046e9f8` | tests(ingest-gate): RED — GIGI_INGEST_DIR gate + containment attack matrix |
| `2ec93c2` | `f37c089` | impl(ingest-gate): GREEN — fail-closed GIGI_INGEST_DIR gate via shared pathguard |
| `d59e453` | `5e72f62` | tests(explain-kappa): RED — sum-to-total invariant asserted unconditionally |
| `ff26bf6` | `2706513` | impl(explain-kappa): GREEN — record_kappa stamped on every EXPLAIN row from compute_record_k |

Co-Authored-By grep over `bc1d038..HEAD` (full `%B` bodies): 0 matches.

## pathguard design (`src/pathguard.rs` — one guard for both knobs)

`contain(root_env, user_path, must_exist)`, two layers in order:

1. **Component-level lexical screen, BEFORE joining.** Any `Prefix` (drive/UNC), `RootDir`, or `ParentDir` component rejects outright; `CurDir` is stripped as noise; a path with no remaining components is rejected. This kills `C:file` and `\rooted` — the two Windows shapes `Path::is_absolute()` misses and `Path::join` silently promotes to a full path replacement — plus `/rooted`, `\\server\share`, and `../x`, uniformly on every platform.
2. **Canonical verification, canonical-to-canonical only.** After joining onto the root, `fs::canonicalize` both sides and require `starts_with` — symlinks and junctions cannot tunnel out, and Windows `\\?\` verbatim prefixes are a non-issue because canonical compares against canonical. Read mode (`must_exist=true`, INGEST) canonicalizes the candidate itself; write mode (EMIT) creates parent dirs, checks the canonical PARENT, and returns the uncanonicalized join so receipts keep the operator's spelling.

`emit_target` now delegates to `contain("GIGI_EMIT_DIR", …)` with the error-message contract unchanged (unset names the knob; every lexical rejection keeps the "must be relative" wording) — `tests/emit_csv.rs` needed zero edits, and the `152de57` platform-absolute fixture is retained (a drive-prefixed path is exactly what the Prefix screen rejects). **The pre-merge Windows failure of emit_csv phase 3 is gone: `emit_csv` 1/1 on this Windows box** — the deferred-row-2 workaround is now enforced product behavior.

## GIGI_INGEST_DIR contract (`src/ingest.rs::resolve_ingest_source`)

Fail-closed, same posture as Postgres `pg_read_server_files` / MySQL `secure_file_priv`: unset ⇒ INGEST from server-side files is disabled engine-wide; set ⇒ source paths are RELATIVE to the root, component-screened and canonically verified through the shared pathguard. Single chokepoint before ANY source-path open — both entry points (`execute_ingest` AUTO_GENERIC and `execute_ingest_as_gauge_field`) resolve through it, and the gauge path resolves BEFORE lattice/bundle work. `fly.toml` now carries `GIGI_INGEST_DIR = "/data/ingest"` under `[env]` — the December harvest pipeline keeps its capability, bounded to the volume directory it already writes into. Probe Q2 below proves the env landed behaviorally: the live containment error names `/data/ingest`.

**INGEST error-shape contract change (intentional):** an absolute source like `/tmp/nonexistent.npz` now returns the containment error BEFORE any filesystem access — the morning probe P8's `INGEST: source file not found: /tmp/nope.npz` shape is superseded. `file not found` is now reserved for paths legal under the root but absent (and carries the resolved candidate). Halcyon notified with exact error strings: `GIGI_TO_HALCYON_REPLY_2026-07-03_INGEST_DIR.md`.

## explain_kappa invariant

`explain_record_k` (src/bundle.rs) rebuilds the record's fiber values in `fiber_fields` order and calls `compute_record_k` — the total path insert-time pricing runs — stamping the result as a constant `record_kappa` field on every decomposition row. The response certifies its own invariant: mean(kappa column) == record_kappa, cross-checking the decomposition loop against the total loop on every EXPLAIN; `tests/explain_kappa.rs` asserts it unconditionally (tolerance 1e-9, deferred-row-3 closed). Single chokepoint in bundle.rs — the embedded executor and the server arm inherit it with zero dispatch edits.

## Attack matrix

- `tests/pathguard_escapes.rs`: 16 test fns — 14 runnable on Windows (junction case live), 12 on Linux (symlink case live); unset/empty root, `..`, rooted, drive-prefix, UNC, backslash-rooted, CurDir stripping, missing-file IO shape, unresolvable root, symlink/junction tunnel, error-names-path-and-root.
- `tests/ingest_dir_gate.rs`: 1 test × 6 phases (gate closed; absolute-outside; relative-inside succeeds; `..` traversal; missing-under-root keeps file-not-found; link-out-of-root refused) — process-global env, single-test pattern like emit_csv.

## Gates (all green before deploy, this Windows box)

| gate | result |
|---|---|
| `cargo check --features "kahler imagine sharded transactions patterns causal_states wish halcyon" --bin gigi-stream` | PASS |
| `cargo test --no-default-features --lib` | 911 / 0 |
| `pathguard_escapes` | 14 / 0 (Windows; junction live) |
| no-feature group (`emit_csv` 1, `ingest_executor` 11, `ingest_csv_basic` 6, `ingest_jsonl_basic` 3, `noop_notices` 4, `timestamp_ergonomics` 6, `gql_reference_truth` 3) | 34 / 0 |
| `ingest_dir_gate` | 1 / 0 (6 phases) |
| `explain_kappa` (no feature cfg; runs in the default group) | 3 / 0 |
| halcyon group (`ingest_as_gauge_field` 18, `ingest_gauge_vertex` 8, `ingest_npz_key` 4, `ingest_npz_dtype` 4, `ingest_gql_bypass` 5, `halcyon_l24_workflow_e2e` 1, `spectral_gauge` 21, `spectral_gauge_where` 7, `chern_class` 6, `chern_class_bundle_target` 11, `betti_pi1` 8, `obstruction` 5, `topology_verbs_gql_integration` 9, `halcyon_part_iv_gold` 4 + 1 pre-existing ignore, `aurora_lie_poisson_trait` 12) | 123 / 0 |
| `imagine_coherence_phase2` (kahler+imagine) | 10 / 0 |
| `cubic_lattice` + `lattice_obc_basic` (lattice) | 17 / 0 |
| `davis_conjecture_lambda_brain_ridealong` (kahler) | 25 / 0 |
| bin `http_gql_emit` | 1 / 0 |

No gate-fix commits needed; no flakes this round (`chern_class_bundle_target_basic` 11/11 first try). KNOWN NON-GATE, unchanged: `pattern_hunt_parser::ph3_and_or_combinators_parse` under `--features patterns` — pre-existing DEFINE PATTERN OR combinator bug, queued separately, not touched here.

## Deploy

- Image: `registry.fly.io/gigi-stream:deployment-01KWM7S0SKS47P0ZEBXX4GP519` (digest `sha256:392996d8…`, 63 MB), release v244.
- `flyctl status`: machine `683961dbe9ee38` (iad) running `gigi-stream:deployment-01KWM7S0SKS47P0ZEBXX4GP519`, version 244, started, 1/1 checks passing — authoritative tag check.
- `/v1/health` post-boot: `{"status":"ok","bundles":5046,"total_records":13001312}` at uptime 135 s — identical to the morning post-boot baseline (same heap-only delta: the morning's probe fixtures + `claude_substrate_v0`, restored below).

## Live probes on the deployed image

| id | query | status | body head | pass |
|---|---|---|---|---|
| Q1 | `LATTICE l4_hard_0703 FROM CUBIC L=4 DIM=2 OBC AXIS 0;` | 200 | `{"status":"ok"}` | PASS |
| Q2 | `INGEST qx FROM '/tmp/nonexistent.npz' FORMAT NPZ AS GAUGE_FIELD GROUP SU(2) ON LATTICE l4_hard_0703;` | 500 | `{"error":"INGEST: path '/tmp/nonexistent.npz' escapes containment root '/data/ingest': absolute paths are not allowed; use a path relative to the root"}` — containment BEFORE file access, names the prod root; not file-not-found, not `No bundle: qx` | PASS |
| Q3 | `INGEST qy FROM '../escape.csv' FORMAT CSV;` | 500 | `{"error":"INGEST: path '../escape.csv' escapes containment root '/data/ingest': '..' components are not allowed"}` — lexical ParentDir screen, live | PASS |
| Q4a | `COVER claude_substrate_v0 ALL EMIT CSV TO '../pwn.csv';` | 500 | `{"error":"EMIT is disabled on this engine: set GIGI_EMIT_DIR=<directory> …"}` — RootUnset precedes the lexical screen in `contain()`, so with the knob unset on prod BOTH emit probes return the fail-closed gate error; the escape path is pinned by `pathguard_escapes`/`emit_csv` locally and by Q3 live through the same shared guard | PASS (fail-closed dominates) |
| Q4b | `COVER claude_substrate_v0 ALL EMIT CSV TO 'ok.csv';` | 500 | same gate error — GIGI_EMIT_DIR intentionally unset on prod | PASS |
| Q5 | `EXPLAIN SECTION kappa_probe_0703 AT id='moon';` (fresh 2-fiber bundle, 4 sections, one outlier) | 200 | `{"rows":[{…"field":"temp","kappa":0.561,"record_kappa":0.281,…},{…"field":"wind","kappa":0.000999…,"record_kappa":0.281,…}]}` — `record_kappa` on EVERY row, constant, and mean(0.561, 0.000999…)/2 = 0.281 exactly: the invariant surfaces live | PASS |
| Q6a | `SHOW FIELDS ON claude_substrate_v0;` | 200 | 6 real field rows | PASS |
| Q6b | `INGEST pz FROM 'a.npz' FORMAT NPZ JUNKTOKEN;` | 400 | `Parse error: … trailing input is not a supported clause and was NOT executed: 'JUNKTOKEN'` | PASS |
| Q6c | `INTEGRATE nonexistent MEASURE avg(x) WITH JACKKNIFE ALONG t SKIP FIRST 5;` | 404 | `{"error":"No bundle: nonexistent"}` — parser accepted, executor resolved | PASS |

Probe fixtures created on prod: `l4_hard_0703`, `kappa_probe_0703`, `marcella_sanity_0703` — heap-only, will vanish on the next redeploy.

## Marcella sanity (IMAGINE Phase 2)

`POST /v1/bundles/marcella_sanity_0703/imagine_coherence` with `{"dim":4,"steps":3,"starting_from":[0,0,0,0],"along":[1,0,0,0]}` (bundle created fresh first):

```
200: dim=4, trajectory=seed+3 steps, coherence=1.0 at every step,
     defect=0.0 (float-eps 5.55e-17 at step 3), max_imagined_curvature=4.0 (FS ceiling),
     max_accumulated_holonomy=0.5, refused=false
```

Identical to the 2026-07-02 and 2026-07-03-morning reference values. Phase 2 path intact.

## claude_substrate_v0

- Wiped by this redeploy as expected (heap-only fragility, unchanged).
- Restored: `CREATE BUNDLE claude_substrate_v0 (…documented schema…)` then `POST /v1/bundles/claude_substrate_v0/import`. Note for the next restore: the import endpoint takes `{"records":[…]}` — the backup file `.deploy-backups/2026-06-29-morning/claude_substrate_v0_backup.json` is a COVER response (`{"rows":[…]}`), so wrap `rows` as `records` before posting (the raw file 422s with "missing field `records`").
- Import result: `{"status":"imported","count":20,"total":20}`; `COVER claude_substrate_v0 ALL;` → 20 rows. No fingerprint collapse. A live pre-deploy export (45,647 bytes, byte-identical size to the 06-29 backup) was taken before the deploy as insurance; live state had not drifted from the backup.
- Still fragile until the snapshot wedge is fixed; `/v1/admin/snapshot` intentionally not triggered (unchanged posture).

## Notes (hardening trio)

- Q4's expectation in the ship order said "escape rejection" for `'../pwn.csv'`; the deployed code's actual (and correct) precedence is RootUnset-first — fail-closed dominates every other answer when the knob is unset. Recorded here so the next probe round expects the gate error until GIGI_EMIT_DIR is set on prod.
- `gql_reference_truth` counts 3 tests in this round's run (the morning table said 6 — suite composition changed upstream of this merge; 3/3 green is the current truth).
- No `Co-Authored-By` footer anywhere on `bc1d038..HEAD`; grep re-run including this doc's commit before the final push.
